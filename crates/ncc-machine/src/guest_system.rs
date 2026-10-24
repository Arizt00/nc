use crate::{
    adapt_opensbi_handoff, build_opensbi_payload, build_uefi_gpt_fat32_disk_named_with_initrd,
    crc32, parse_fdt, GuestBootProtocol, GuestCpuMode, GuestLoadPlan, GuestSegmentFlags,
};
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

pub const LINUX_BOOT_PARAMS_ADDRESS: u64 = 0x7000;
pub const LINUX_BOOT_PARAMS_BYTES: usize = 4096;
pub const LINUX_COMMAND_LINE_ADDRESS: u64 = 0x20_000;
pub const LINUX_SETUP_ADDRESS: u64 = 0x90_000;
pub const LINUX_GDT_ADDRESS: u64 = 0x500;
pub const LINUX_STACK_TOP: u64 = 0x8f_000;
pub const LINUX_E820_TABLE_OFFSET: usize = 0x2d0;
pub const LINUX_E820_COUNT_OFFSET: usize = 0x1e8;
pub const LINUX_E820_MAX_ENTRIES: usize = 128;
pub const PC_ACPI_BASE: u64 = 0x000e_0000;
pub const PC_ACPI_PM1_EVENT_PORT: u16 = 0x0600;
pub const PC_ACPI_PM1_CONTROL_PORT: u16 = 0x0604;
pub const PC_ACPI_PM_TIMER_PORT: u16 = 0x0608;
pub const PC_VIRTIO_MMIO_BASE: u64 = 0xd000_0000;
pub const PC_VIRTIO_MMIO_STRIDE: u64 = 0x1000;
// Keep paravirtual devices away from the legacy PIC assignments used during x86 boot.
pub const PC_VIRTIO_IRQ_BASE: u32 = 10;
pub const PC_VIRTIO_MAX_DEVICES: usize = 4096;
pub const OPENSBI_DYNAMIC_INFO_MAGIC: u64 = 0x4942_534f;
pub const OPENSBI_DYNAMIC_INFO_VERSION: u64 = 2;
pub const OPENSBI_NEXT_MODE_S: u64 = 1;
pub const RISCV_VIRT_MEMORY_BASE: u64 = 0x8000_0000;
pub const RISCV_VIRT_UART_BASE: u64 = 0x1000_0000;
pub const RISCV_VIRT_VIRTIO_BASE: u64 = 0x1000_1000;
pub const RISCV_VIRT_VIRTIO_STRIDE: u64 = 0x1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestSystemError {
    InvalidArchitecture,
    InvalidBootMethod,
    InvalidKernel(String),
    InvalidFirmware(String),
    InvalidFdt(String),
    InvalidCommandLine,
    InvalidMemory,
    InvalidVcpuCount,
    InvalidDisk(String),
    AddressOverflow,
    RegionOutsideMemory(String),
    RegionOverlap { left: String, right: String },
    InitrdPlacement,
    TooManyE820Entries,
    DuplicateFirmwareTable,
}

impl fmt::Display for GuestSystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArchitecture => {
                formatter.write_str("boot method and guest architecture disagree")
            }
            Self::InvalidBootMethod => formatter.write_str("guest boot method is invalid"),
            Self::InvalidKernel(reason) => write!(formatter, "invalid guest kernel: {reason}"),
            Self::InvalidFirmware(reason) => write!(formatter, "invalid guest firmware: {reason}"),
            Self::InvalidFdt(reason) => write!(formatter, "invalid guest FDT: {reason}"),
            Self::InvalidCommandLine => formatter.write_str("invalid guest kernel command line"),
            Self::InvalidMemory => formatter.write_str("invalid guest physical memory layout"),
            Self::InvalidVcpuCount => formatter.write_str("guest must contain at least one vCPU"),
            Self::InvalidDisk(id) => write!(formatter, "invalid VirtIO disk '{id}'"),
            Self::AddressOverflow => {
                formatter.write_str("guest physical address arithmetic overflow")
            }
            Self::RegionOutsideMemory(name) => {
                write!(formatter, "guest region '{name}' lies outside RAM")
            }
            Self::RegionOverlap { left, right } => {
                write!(formatter, "guest regions '{left}' and '{right}' overlap")
            }
            Self::InitrdPlacement => {
                formatter.write_str("initrd cannot be placed below its address limit")
            }
            Self::TooManyE820Entries => formatter.write_str("Linux E820 map exceeds 128 entries"),
            Self::DuplicateFirmwareTable => {
                formatter.write_str("duplicate guest firmware table kind")
            }
        }
    }
}

impl std::error::Error for GuestSystemError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestSystemArchitecture {
    X86_64,
    I386,
    Aarch64,
    RiscV64,
    RiscV32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestSystemBootMethod {
    LinuxBzImage,
    Efi,
    OpenSbi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuestFirmwareTableKind {
    AcpiRsdp,
    Smbios2,
    Smbios3,
    EfiSystemTable,
    Fdt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFirmwareTable {
    pub kind: GuestFirmwareTableKind,
    pub guest_physical_address: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxE820Entry {
    pub address: u64,
    pub size: u64,
    pub kind: u32,
}

impl LinuxE820Entry {
    pub const RAM: u32 = 1;
    pub const RESERVED: u32 = 2;
    pub const ACPI_RECLAIM: u32 = 3;
    pub const ACPI_NVS: u32 = 4;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioDiskAttachment {
    pub id: String,
    pub block_size: u32,
    pub read_only: bool,
    pub bytes: Vec<u8>,
    pub checksum: u32,
}

impl VirtioDiskAttachment {
    pub fn new(
        id: impl Into<String>,
        mut bytes: Vec<u8>,
        block_size: u32,
        read_only: bool,
    ) -> Result<Self, GuestSystemError> {
        let id = id.into();
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || !(512..=65_536).contains(&block_size)
            || !block_size.is_power_of_two()
        {
            return Err(GuestSystemError::InvalidDisk(id));
        }
        let block_size_usize = block_size as usize;
        if bytes.is_empty() {
            bytes.resize(block_size_usize, 0);
        } else {
            let padded = align_up(bytes.len() as u64, block_size as u64)? as usize;
            bytes.resize(padded, 0);
        }
        let checksum = crc32(&bytes);
        Ok(Self {
            id,
            block_size,
            read_only,
            bytes,
            checksum,
        })
    }

    pub fn verify(&self) -> Result<(), GuestSystemError> {
        if self.bytes.is_empty()
            || self.bytes.len() % self.block_size as usize != 0
            || crc32(&self.bytes) != self.checksum
        {
            return Err(GuestSystemError::InvalidDisk(self.id.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestSystemConfig {
    pub architecture: GuestSystemArchitecture,
    pub method: GuestSystemBootMethod,
    pub memory_base: u64,
    pub memory_size: u64,
    pub vcpu_count: u32,
    pub kernel: Vec<u8>,
    pub firmware: Vec<u8>,
    pub initrd: Vec<u8>,
    pub command_line: String,
    pub fdt: Vec<u8>,
    pub firmware_address: Option<u64>,
    pub initrd_address: Option<u64>,
    pub fdt_address: Option<u64>,
    pub firmware_tables: Vec<GuestFirmwareTable>,
    pub e820: Vec<LinuxE820Entry>,
    pub disks: Vec<VirtioDiskAttachment>,
}

impl GuestSystemConfig {
    pub fn linux_bzimage(kernel: Vec<u8>, memory_size: u64) -> Self {
        Self {
            architecture: GuestSystemArchitecture::X86_64,
            method: GuestSystemBootMethod::LinuxBzImage,
            memory_base: 0,
            memory_size,
            vcpu_count: 1,
            kernel,
            firmware: Vec::new(),
            initrd: Vec::new(),
            command_line: "console=ttyS0 reboot=k panic=-1".into(),
            fdt: Vec::new(),
            firmware_address: None,
            initrd_address: None,
            fdt_address: None,
            firmware_tables: Vec::new(),
            e820: Vec::new(),
            disks: Vec::new(),
        }
    }

    pub fn efi(
        architecture: GuestSystemArchitecture,
        application: Vec<u8>,
        firmware: Vec<u8>,
        memory_size: u64,
    ) -> Self {
        Self {
            architecture,
            method: GuestSystemBootMethod::Efi,
            memory_base: 0,
            memory_size,
            vcpu_count: 1,
            kernel: application,
            firmware,
            initrd: Vec::new(),
            command_line: String::new(),
            fdt: Vec::new(),
            firmware_address: None,
            initrd_address: None,
            fdt_address: None,
            firmware_tables: Vec::new(),
            e820: Vec::new(),
            disks: Vec::new(),
        }
    }

    pub fn opensbi(
        architecture: GuestSystemArchitecture,
        kernel_elf: Vec<u8>,
        firmware: Vec<u8>,
        memory_size: u64,
    ) -> Self {
        Self {
            architecture,
            method: GuestSystemBootMethod::OpenSbi,
            memory_base: RISCV_VIRT_MEMORY_BASE,
            memory_size,
            vcpu_count: 1,
            kernel: kernel_elf,
            firmware,
            initrd: Vec::new(),
            command_line: "console=ttyS0 earlycon=sbi reboot=t panic=-1".into(),
            fdt: Vec::new(),
            firmware_address: None,
            initrd_address: None,
            fdt_address: None,
            firmware_tables: Vec::new(),
            e820: Vec::new(),
            disks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestRegionKind {
    Firmware,
    KernelSetup,
    KernelCode,
    BootParameters,
    CommandLine,
    Initrd,
    FirmwareTable,
    Fdt,
    OpenSbiDynamicInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPhysicalRegion {
    pub name: String,
    pub guest_physical_address: u64,
    pub bytes: Vec<u8>,
    pub flags: GuestSegmentFlags,
    pub kind: GuestRegionKind,
}

impl GuestPhysicalRegion {
    pub fn end(&self) -> Result<u64, GuestSystemError> {
        self.guest_physical_address
            .checked_add(self.bytes.len() as u64)
            .ok_or(GuestSystemError::AddressOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestInitialCpuState {
    pub mode: GuestCpuMode,
    pub entry_point: u64,
    pub registers: BTreeMap<String, u128>,
    pub interrupts_enabled: bool,
    pub paging_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBootHandoff {
    pub protocol_version: u16,
    pub setup_bytes: usize,
    pub code32_start: u64,
    pub kernel_alignment: u32,
    pub init_size: u32,
    pub command_line_limit: usize,
    pub initrd_address_limit: u64,
    pub boot_params_address: u64,
    pub command_line_address: u64,
    pub initrd_address: Option<u64>,
    pub e820: Vec<LinuxE820Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfiBootHandoff {
    pub application_path: String,
    pub load_options_utf16: Vec<u16>,
    pub initrd_media_guid: [u8; 16],
    pub initrd_address: Option<u64>,
    pub initrd_size: u64,
    pub image_plan: GuestLoadPlan,
    pub esp_disk: VirtioDiskAttachment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSbiBootHandoff {
    pub hart_id: u64,
    pub fdt_address: u64,
    pub dynamic_info_address: u64,
    pub firmware_entry: u64,
    pub kernel_entry: u64,
    pub kernel_load_address: u64,
    pub next_mode: u64,
    pub xlen: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestBootPlan {
    pub architecture: GuestSystemArchitecture,
    pub method: GuestSystemBootMethod,
    pub memory_base: u64,
    pub memory_size: u64,
    pub vcpu_count: u32,
    pub initial_cpu: GuestInitialCpuState,
    pub regions: Vec<GuestPhysicalRegion>,
    pub firmware_tables: Vec<GuestFirmwareTable>,
    pub disks: Vec<VirtioDiskAttachment>,
    pub linux: Option<LinuxBootHandoff>,
    pub efi: Option<EfiBootHandoff>,
    pub opensbi: Option<OpenSbiBootHandoff>,
    pub power_contract: GuestPowerContract,
}

impl GuestBootPlan {
    pub fn build(config: &GuestSystemConfig) -> Result<Self, GuestSystemError> {
        if config.vcpu_count == 0 {
            return Err(GuestSystemError::InvalidVcpuCount);
        }
        if config.memory_size < 2 * 1024 * 1024
            || config.memory_base.checked_add(config.memory_size).is_none()
        {
            return Err(GuestSystemError::InvalidMemory);
        }
        validate_firmware_tables(&config.firmware_tables)?;
        for disk in &config.disks {
            disk.verify()?;
        }
        let mut plan = match config.method {
            GuestSystemBootMethod::LinuxBzImage => build_linux_bzimage_plan(config)?,
            GuestSystemBootMethod::Efi => build_efi_plan(config)?,
            GuestSystemBootMethod::OpenSbi => build_opensbi_plan(config)?,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&mut self) -> Result<(), GuestSystemError> {
        let memory_end = self
            .memory_base
            .checked_add(self.memory_size)
            .ok_or(GuestSystemError::AddressOverflow)?;
        self.regions
            .sort_by_key(|region| region.guest_physical_address);
        for region in &self.regions {
            let end = region.end()?;
            if region.bytes.is_empty()
                || region.guest_physical_address < self.memory_base
                || end > memory_end
            {
                return Err(GuestSystemError::RegionOutsideMemory(region.name.clone()));
            }
        }
        for pair in self.regions.windows(2) {
            if pair[0].end()? > pair[1].guest_physical_address {
                return Err(GuestSystemError::RegionOverlap {
                    left: pair[0].name.clone(),
                    right: pair[1].name.clone(),
                });
            }
        }
        for disk in &self.disks {
            disk.verify()?;
        }
        Ok(())
    }

    pub fn materialize(&self, memory: &mut [u8]) -> Result<(), GuestSystemError> {
        if memory.len() as u64 != self.memory_size {
            return Err(GuestSystemError::InvalidMemory);
        }
        memory.fill(0);
        for region in &self.regions {
            let start = usize::try_from(region.guest_physical_address - self.memory_base)
                .map_err(|_| GuestSystemError::AddressOverflow)?;
            let end = start
                .checked_add(region.bytes.len())
                .ok_or(GuestSystemError::AddressOverflow)?;
            memory
                .get_mut(start..end)
                .ok_or_else(|| GuestSystemError::RegionOutsideMemory(region.name.clone()))?
                .copy_from_slice(&region.bytes);
        }
        Ok(())
    }
}

fn build_linux_bzimage_plan(config: &GuestSystemConfig) -> Result<GuestBootPlan, GuestSystemError> {
    if !matches!(
        config.architecture,
        GuestSystemArchitecture::X86_64 | GuestSystemArchitecture::I386
    ) || config.memory_base != 0
        || config.kernel.len() < 0x264
        || config.kernel.get(0x202..0x206) != Some(b"HdrS")
    {
        return Err(GuestSystemError::InvalidArchitecture);
    }
    let protocol = le_u16(&config.kernel, 0x206)?;
    let loadflags = config.kernel[0x211];
    if protocol < 0x0200 || loadflags & 1 == 0 {
        return Err(GuestSystemError::InvalidKernel(
            "image does not implement the bzImage boot protocol".into(),
        ));
    }
    let setup_sectors = match config.kernel[0x1f1] {
        0 => 4,
        value => value as usize,
    };
    let setup_bytes = (setup_sectors + 1)
        .checked_mul(512)
        .ok_or(GuestSystemError::AddressOverflow)?;
    if setup_bytes >= config.kernel.len() {
        return Err(GuestSystemError::InvalidKernel(
            "truncated setup code".into(),
        ));
    }
    let code32_start = u64::from(le_u32(&config.kernel, 0x214)?).max(0x10_0000);
    let kernel_alignment = if protocol >= 0x0205 {
        le_u32(&config.kernel, 0x230)?.max(4096)
    } else {
        4096
    };
    if !kernel_alignment.is_power_of_two() || code32_start % 4096 != 0 {
        return Err(GuestSystemError::InvalidKernel(
            "code32_start is not page aligned".into(),
        ));
    }
    let init_size = if protocol >= 0x020a {
        le_u32(&config.kernel, 0x260)?.max((config.kernel.len() - setup_bytes) as u32)
    } else {
        (config.kernel.len() - setup_bytes) as u32
    };
    let command_line_limit = if protocol >= 0x0206 {
        le_u32(&config.kernel, 0x238)? as usize
    } else {
        255
    };
    if config.command_line.as_bytes().contains(&0)
        || config.command_line.len().saturating_add(1) > command_line_limit.max(1)
    {
        return Err(GuestSystemError::InvalidCommandLine);
    }
    let initrd_limit = if protocol >= 0x0203 {
        u64::from(le_u32(&config.kernel, 0x22c)?).max(0x37ff_ffff)
    } else {
        0x37ff_ffff
    };
    let mut e820 = if config.e820.is_empty() {
        default_x86_e820(config.memory_size)
    } else {
        config.e820.clone()
    };
    normalize_e820(&mut e820, config.memory_size)?;

    let mut firmware_tables = config.firmware_tables.clone();
    if !firmware_tables
        .iter()
        .any(|table| table.kind == GuestFirmwareTableKind::AcpiRsdp)
    {
        firmware_tables.push(build_pc_acpi_firmware_with_virtio(
            config.vcpu_count,
            config.disks.len(),
        )?);
    }
    let mut boot_params = vec![0_u8; LINUX_BOOT_PARAMS_BYTES];
    let header_copy = config.kernel.len().min(LINUX_BOOT_PARAMS_BYTES);
    boot_params[..header_copy].copy_from_slice(&config.kernel[..header_copy]);
    boot_params[0x210] = 0xff;
    boot_params[0x211] |= 0x80;
    put_le_u16(&mut boot_params, 0x224, 0xfe00)?;
    put_le_u32(
        &mut boot_params,
        0x228,
        u32::try_from(LINUX_COMMAND_LINE_ADDRESS).map_err(|_| GuestSystemError::AddressOverflow)?,
    )?;
    boot_params[LINUX_E820_COUNT_OFFSET] = e820.len() as u8;
    for (index, entry) in e820.iter().enumerate() {
        let offset = LINUX_E820_TABLE_OFFSET + index * 20;
        put_le_u64(&mut boot_params, offset, entry.address)?;
        put_le_u64(&mut boot_params, offset + 8, entry.size)?;
        put_le_u32(&mut boot_params, offset + 16, entry.kind)?;
    }
    if let Some(rsdp) = firmware_tables
        .iter()
        .find(|table| table.kind == GuestFirmwareTableKind::AcpiRsdp)
    {
        put_le_u64(&mut boot_params, 0x70, rsdp.guest_physical_address)?;
    }

    let command_line = nul_terminated(&config.command_line)?;
    let kernel_bytes = config.kernel[setup_bytes..].to_vec();
    let kernel_end = code32_start
        .checked_add(init_size.max(kernel_bytes.len() as u32) as u64)
        .ok_or(GuestSystemError::AddressOverflow)?;
    let initrd_address = place_initrd(
        config,
        initrd_limit,
        kernel_end,
        LINUX_SETUP_ADDRESS + setup_bytes as u64,
    )?;
    if let Some(address) = initrd_address {
        put_le_u32(
            &mut boot_params,
            0x218,
            u32::try_from(address).map_err(|_| GuestSystemError::InitrdPlacement)?,
        )?;
        put_le_u32(
            &mut boot_params,
            0x21c,
            u32::try_from(config.initrd.len()).map_err(|_| GuestSystemError::InitrdPlacement)?,
        )?;
    }

    let mut regions = vec![
        region(
            "linux-gdt",
            LINUX_GDT_ADDRESS,
            linux_boot_gdt(),
            GuestRegionKind::BootParameters,
            GuestSegmentFlags::READ_WRITE,
        ),
        region(
            "linux-boot-params",
            LINUX_BOOT_PARAMS_ADDRESS,
            boot_params,
            GuestRegionKind::BootParameters,
            GuestSegmentFlags::READ_WRITE,
        ),
        region(
            "linux-cmdline",
            LINUX_COMMAND_LINE_ADDRESS,
            command_line,
            GuestRegionKind::CommandLine,
            GuestSegmentFlags::READ_WRITE,
        ),
        region(
            "linux-setup",
            LINUX_SETUP_ADDRESS,
            config.kernel[..setup_bytes].to_vec(),
            GuestRegionKind::KernelSetup,
            GuestSegmentFlags::READ_WRITE,
        ),
        region(
            "linux-protected-kernel",
            code32_start,
            kernel_bytes,
            GuestRegionKind::KernelCode,
            GuestSegmentFlags::READ_EXECUTE,
        ),
    ];
    if let Some(address) = initrd_address {
        regions.push(region(
            "linux-initrd",
            address,
            config.initrd.clone(),
            GuestRegionKind::Initrd,
            GuestSegmentFlags::READ_WRITE,
        ));
    }
    append_firmware_tables(&mut regions, &firmware_tables);
    let registers = BTreeMap::from([
        ("esi".into(), LINUX_BOOT_PARAMS_ADDRESS as u128),
        ("esp".into(), LINUX_STACK_TOP as u128),
        ("ebp".into(), 0),
        ("edi".into(), 0),
        ("ebx".into(), 0),
        ("cs".into(), 0x10),
        ("ds".into(), 0x18),
        ("es".into(), 0x18),
        ("ss".into(), 0x18),
        ("gdt_base".into(), LINUX_GDT_ADDRESS as u128),
        ("gdt_limit".into(), 23),
        ("cr0".into(), 1),
        ("cr4".into(), 0),
    ]);
    Ok(GuestBootPlan {
        architecture: config.architecture,
        method: config.method,
        memory_base: config.memory_base,
        memory_size: config.memory_size,
        vcpu_count: config.vcpu_count,
        initial_cpu: GuestInitialCpuState {
            mode: GuestCpuMode::Protected32,
            entry_point: code32_start,
            registers,
            interrupts_enabled: false,
            paging_enabled: false,
        },
        regions,
        firmware_tables,
        disks: config.disks.clone(),
        linux: Some(LinuxBootHandoff {
            protocol_version: protocol,
            setup_bytes,
            code32_start,
            kernel_alignment,
            init_size,
            command_line_limit,
            initrd_address_limit: initrd_limit,
            boot_params_address: LINUX_BOOT_PARAMS_ADDRESS,
            command_line_address: LINUX_COMMAND_LINE_ADDRESS,
            initrd_address,
            e820,
        }),
        efi: None,
        opensbi: None,
        power_contract: GuestPowerContract::pc(),
    })
}

fn build_efi_plan(config: &GuestSystemConfig) -> Result<GuestBootPlan, GuestSystemError> {
    if !matches!(
        config.architecture,
        GuestSystemArchitecture::X86_64
            | GuestSystemArchitecture::I386
            | GuestSystemArchitecture::Aarch64
    ) {
        return Err(GuestSystemError::InvalidArchitecture);
    }
    let image_plan = GuestLoadPlan::parse(&config.kernel)
        .map_err(|error| GuestSystemError::InvalidKernel(error.to_string()))?;
    if image_plan.protocol != GuestBootProtocol::UefiLoadImage
        || !efi_mode_matches(config.architecture, image_plan.cpu_mode)
    {
        return Err(GuestSystemError::InvalidArchitecture);
    }
    let file_name = match config.architecture {
        GuestSystemArchitecture::X86_64 => b"BOOTX64 EFI",
        GuestSystemArchitecture::Aarch64 => b"BOOTAA64EFI",
        GuestSystemArchitecture::I386 => b"BOOTIA32EFI",
        _ => return Err(GuestSystemError::InvalidArchitecture),
    };
    let esp_bytes =
        build_uefi_gpt_fat32_disk_named_with_initrd(&config.kernel, file_name, &config.initrd)
            .map_err(|error| GuestSystemError::InvalidFirmware(error.to_string()))?;
    let mut disks = vec![VirtioDiskAttachment::new(
        "efi-system",
        esp_bytes,
        512,
        false,
    )?];
    disks.extend(config.disks.clone());
    let esp_disk = disks[0].clone();
    let app_path = match config.architecture {
        GuestSystemArchitecture::Aarch64 => "\\EFI\\BOOT\\BOOTAA64.EFI",
        GuestSystemArchitecture::I386 => "\\EFI\\BOOT\\BOOTIA32.EFI",
        GuestSystemArchitecture::X86_64 => "\\EFI\\BOOT\\BOOTX64.EFI",
        _ => return Err(GuestSystemError::InvalidArchitecture),
    };
    let mut options = config.command_line.clone();
    if !config.initrd.is_empty() {
        if !options.is_empty() {
            options.push(' ');
        }
        options.push_str("initrd=\\EFI\\BOOT\\INITRD");
    }
    if options.as_bytes().contains(&0) {
        return Err(GuestSystemError::InvalidCommandLine);
    }
    let mut load_options_utf16: Vec<u16> = options.encode_utf16().collect();
    load_options_utf16.push(0);
    let memory_end = config
        .memory_base
        .checked_add(config.memory_size)
        .ok_or(GuestSystemError::AddressOverflow)?;
    let firmware_address = if config.firmware.is_empty() {
        None
    } else {
        Some(place_high_region_below(
            config,
            config.firmware.len() as u64,
            config.firmware_address,
            4096,
            config.memory_base + 0x20_0000,
            memory_end,
        )?)
    };
    let initrd_address = if config.initrd.is_empty() {
        None
    } else {
        Some(place_high_region_below(
            config,
            config.initrd.len() as u64,
            config.initrd_address,
            4096,
            config.memory_base + 0x20_0000,
            firmware_address.unwrap_or(memory_end),
        )?)
    };
    let mut regions = Vec::new();
    if let Some(address) = firmware_address {
        regions.push(region(
            "uefi-firmware",
            address,
            config.firmware.clone(),
            GuestRegionKind::Firmware,
            GuestSegmentFlags::READ_EXECUTE,
        ));
    }
    for (index, segment) in image_plan.segments.iter().enumerate() {
        let end = segment
            .source_offset
            .checked_add(segment.file_size)
            .ok_or(GuestSystemError::AddressOverflow)?;
        let mut bytes = vec![0; segment.memory_size];
        bytes[..segment.file_size].copy_from_slice(
            config
                .kernel
                .get(segment.source_offset..end)
                .ok_or_else(|| {
                    GuestSystemError::InvalidKernel("PE section outside image".into())
                })?,
        );
        regions.push(region(
            &format!("efi-image-section-{index}"),
            segment.guest_physical_address,
            bytes,
            GuestRegionKind::KernelCode,
            segment.flags,
        ));
    }
    if let Some(address) = initrd_address {
        regions.push(region(
            "efi-initrd-media",
            address,
            config.initrd.clone(),
            GuestRegionKind::Initrd,
            GuestSegmentFlags::READ_WRITE,
        ));
    }
    append_firmware_tables(&mut regions, &config.firmware_tables);
    let entry = if config.firmware.is_empty() {
        image_plan.entry_point
    } else if matches!(
        config.architecture,
        GuestSystemArchitecture::X86_64 | GuestSystemArchitecture::I386
    ) {
        0xffff0
    } else {
        firmware_address.unwrap_or(config.memory_base)
    };
    Ok(GuestBootPlan {
        architecture: config.architecture,
        method: config.method,
        memory_base: config.memory_base,
        memory_size: config.memory_size,
        vcpu_count: config.vcpu_count,
        initial_cpu: GuestInitialCpuState {
            mode: image_plan.cpu_mode,
            entry_point: entry,
            registers: BTreeMap::new(),
            interrupts_enabled: false,
            paging_enabled: false,
        },
        regions,
        firmware_tables: config.firmware_tables.clone(),
        disks,
        linux: None,
        efi: Some(EfiBootHandoff {
            application_path: app_path.into(),
            load_options_utf16,
            initrd_media_guid: [
                0x32, 0x04, 0x7d, 0x55, 0x98, 0x6d, 0x2c, 0x4e, 0xb6, 0x16, 0xe2, 0xd1, 0xbf, 0x5d,
                0x9f, 0x4f,
            ],
            initrd_address,
            initrd_size: config.initrd.len() as u64,
            image_plan,
            esp_disk,
        }),
        opensbi: None,
        power_contract: GuestPowerContract::uefi(),
    })
}

fn build_opensbi_plan(config: &GuestSystemConfig) -> Result<GuestBootPlan, GuestSystemError> {
    let xlen = match config.architecture {
        GuestSystemArchitecture::RiscV64 => 64,
        GuestSystemArchitecture::RiscV32 => 32,
        _ => return Err(GuestSystemError::InvalidArchitecture),
    };
    if config.firmware.is_empty() {
        return Err(GuestSystemError::InvalidFirmware(
            "OpenSBI FW_DYNAMIC/FW_JUMP image is required".into(),
        ));
    }
    let payload = build_opensbi_payload(&config.kernel)
        .map_err(|error| GuestSystemError::InvalidKernel(error.to_string()))?;
    let required_alignment = if xlen == 64 {
        2 * 1024 * 1024
    } else {
        4 * 1024 * 1024
    };
    if payload.load_address % required_alignment != 0 {
        return Err(GuestSystemError::InvalidKernel(format!(
            "RISC-V Linux payload must be aligned to {required_alignment:#x}"
        )));
    }
    let firmware_address = config.firmware_address.unwrap_or(config.memory_base);
    let dynamic_info_address = align_up(
        firmware_address
            .checked_add(config.firmware.len() as u64)
            .ok_or(GuestSystemError::AddressOverflow)?,
        if xlen == 64 { 8 } else { 4 },
    )?;
    let initrd_address = if config.initrd.is_empty() {
        None
    } else {
        Some(place_high_region(
            config,
            config.initrd.len() as u64,
            config.initrd_address,
            4096,
            payload
                .load_address
                .checked_add(payload.bytes.len() as u64)
                .ok_or(GuestSystemError::AddressOverflow)?,
        )?)
    };
    let fdt_address = config.fdt_address.unwrap_or_else(|| {
        align_down(
            config.memory_base + config.memory_size - 2 * 1024 * 1024,
            4096,
        )
    });
    let fdt = if config.fdt.is_empty() {
        build_riscv_virt_fdt(
            xlen,
            config.memory_base,
            config.memory_size,
            config.vcpu_count,
            &config.command_line,
            initrd_address.map(|address| (address, config.initrd.len() as u64)),
            config.disks.len(),
        )?
    } else {
        parse_fdt(&config.fdt).map_err(|error| GuestSystemError::InvalidFdt(error.to_string()))?;
        config.fdt.clone()
    };
    adapt_opensbi_handoff(0, fdt_address, &fdt)
        .map_err(|error| GuestSystemError::InvalidFdt(error.to_string()))?;
    let dynamic_info =
        encode_opensbi_dynamic_info(xlen, payload.entry_point, OPENSBI_NEXT_MODE_S, 0)?;
    let mut regions = vec![
        region(
            "opensbi-firmware",
            firmware_address,
            config.firmware.clone(),
            GuestRegionKind::Firmware,
            GuestSegmentFlags::READ_EXECUTE,
        ),
        region(
            "opensbi-dynamic-info",
            dynamic_info_address,
            dynamic_info,
            GuestRegionKind::OpenSbiDynamicInfo,
            GuestSegmentFlags::READ_WRITE,
        ),
        region(
            "riscv-linux-kernel",
            payload.load_address,
            payload.bytes,
            GuestRegionKind::KernelCode,
            GuestSegmentFlags::READ_EXECUTE,
        ),
        region(
            "riscv-fdt",
            fdt_address,
            fdt,
            GuestRegionKind::Fdt,
            GuestSegmentFlags::READ_WRITE,
        ),
    ];
    if let Some(address) = initrd_address {
        regions.push(region(
            "riscv-initrd",
            address,
            config.initrd.clone(),
            GuestRegionKind::Initrd,
            GuestSegmentFlags::READ_WRITE,
        ));
    }
    append_firmware_tables(&mut regions, &config.firmware_tables);
    let registers = BTreeMap::from([
        ("a0".into(), 0),
        ("a1".into(), fdt_address as u128),
        ("a2".into(), dynamic_info_address as u128),
        ("satp".into(), 0),
    ]);
    Ok(GuestBootPlan {
        architecture: config.architecture,
        method: config.method,
        memory_base: config.memory_base,
        memory_size: config.memory_size,
        vcpu_count: config.vcpu_count,
        initial_cpu: GuestInitialCpuState {
            mode: if xlen == 64 {
                GuestCpuMode::RiscV64
            } else {
                GuestCpuMode::RiscV32
            },
            entry_point: firmware_address,
            registers,
            interrupts_enabled: false,
            paging_enabled: false,
        },
        regions,
        firmware_tables: config.firmware_tables.clone(),
        disks: config.disks.clone(),
        linux: None,
        efi: None,
        opensbi: Some(OpenSbiBootHandoff {
            hart_id: 0,
            fdt_address,
            dynamic_info_address,
            firmware_entry: firmware_address,
            kernel_entry: payload.entry_point,
            kernel_load_address: payload.load_address,
            next_mode: OPENSBI_NEXT_MODE_S,
            xlen,
        }),
        power_contract: GuestPowerContract::sbi(),
    })
}

pub fn build_pc_acpi_firmware(vcpu_count: u32) -> Result<GuestFirmwareTable, GuestSystemError> {
    build_pc_acpi_firmware_with_virtio(vcpu_count, 0)
}

pub fn build_pc_acpi_firmware_with_virtio(
    vcpu_count: u32,
    virtio_devices: usize,
) -> Result<GuestFirmwareTable, GuestSystemError> {
    if vcpu_count == 0 || vcpu_count > 255 {
        return Err(GuestSystemError::InvalidVcpuCount);
    }
    if virtio_devices > PC_VIRTIO_MAX_DEVICES {
        return Err(GuestSystemError::InvalidDisk(format!(
            "PC firmware supports at most {PC_VIRTIO_MAX_DEVICES} VirtIO-MMIO devices"
        )));
    }
    const RSDT_OFFSET: usize = 0x0100;
    const XSDT_OFFSET: usize = 0x0200;
    const FADT_OFFSET: usize = 0x1000;
    const MADT_OFFSET: usize = 0x2000;
    const DSDT_OFFSET: usize = 0x3000;
    const FADT_LENGTH: usize = 244;
    let madt_length = 44_usize
        .checked_add(vcpu_count as usize * 8)
        .and_then(|length| length.checked_add(12 + 10))
        .ok_or(GuestSystemError::AddressOverflow)?;
    let aml = build_pc_dsdt_aml(virtio_devices)?;
    let dsdt_length = 36 + aml.len();
    let arena_length = DSDT_OFFSET
        .checked_add(dsdt_length)
        .ok_or(GuestSystemError::AddressOverflow)?;
    let mut arena = vec![0_u8; arena_length];

    let fadt_address = PC_ACPI_BASE + FADT_OFFSET as u64;
    let madt_address = PC_ACPI_BASE + MADT_OFFSET as u64;
    let dsdt_address = PC_ACPI_BASE + DSDT_OFFSET as u64;
    let rsdt_address = PC_ACPI_BASE + RSDT_OFFSET as u64;
    let xsdt_address = PC_ACPI_BASE + XSDT_OFFSET as u64;

    {
        let table = &mut arena[RSDT_OFFSET..RSDT_OFFSET + 44];
        write_acpi_header(table, b"RSDT", 1, b"NCCROOT ")?;
        put_le_u32(table, 36, fadt_address as u32)?;
        put_le_u32(table, 40, madt_address as u32)?;
        finalize_acpi_table(table);
    }
    {
        let table = &mut arena[XSDT_OFFSET..XSDT_OFFSET + 52];
        write_acpi_header(table, b"XSDT", 1, b"NCCROOT ")?;
        put_le_u64(table, 36, fadt_address)?;
        put_le_u64(table, 44, madt_address)?;
        finalize_acpi_table(table);
    }
    {
        let table = &mut arena[FADT_OFFSET..FADT_OFFSET + FADT_LENGTH];
        write_acpi_header(table, b"FACP", 6, b"NCCFADT ")?;
        put_le_u32(table, 40, dsdt_address as u32)?;
        put_le_u16(table, 46, 9)?;
        put_le_u32(table, 56, u32::from(PC_ACPI_PM1_EVENT_PORT))?;
        put_le_u32(table, 64, u32::from(PC_ACPI_PM1_CONTROL_PORT))?;
        put_le_u32(table, 76, u32::from(PC_ACPI_PM_TIMER_PORT))?;
        table[88] = 4;
        table[89] = 2;
        table[91] = 4;
        put_le_u16(table, 109, 3)?;
        put_le_u32(table, 112, 1 | (1 << 8) | (1 << 10))?;
        write_acpi_gas(table, 116, 1, 8, 0, 1, 0x0cf9)?;
        table[128] = 0x06;
        put_le_u64(table, 140, dsdt_address)?;
        write_acpi_gas(table, 148, 1, 32, 0, 3, u64::from(PC_ACPI_PM1_EVENT_PORT))?;
        write_acpi_gas(table, 172, 1, 16, 0, 2, u64::from(PC_ACPI_PM1_CONTROL_PORT))?;
        write_acpi_gas(table, 208, 1, 32, 0, 3, u64::from(PC_ACPI_PM_TIMER_PORT))?;
        finalize_acpi_table(table);
    }
    {
        let table = &mut arena[MADT_OFFSET..MADT_OFFSET + madt_length];
        write_acpi_header(table, b"APIC", 5, b"NCCMADT ")?;
        put_le_u32(table, 36, 0xfee0_0000)?;
        put_le_u32(table, 40, 1)?;
        let mut offset = 44;
        for id in 0..vcpu_count {
            table[offset] = 0;
            table[offset + 1] = 8;
            table[offset + 2] = id as u8;
            table[offset + 3] = id as u8;
            put_le_u32(table, offset + 4, 1)?;
            offset += 8;
        }
        table[offset] = 1;
        table[offset + 1] = 12;
        table[offset + 2] = 0;
        table[offset + 3] = 0;
        put_le_u32(table, offset + 4, 0xfec0_0000)?;
        put_le_u32(table, offset + 8, 0)?;
        offset += 12;
        table[offset] = 2;
        table[offset + 1] = 10;
        table[offset + 2] = 0;
        table[offset + 3] = 0;
        put_le_u32(table, offset + 4, 2)?;
        put_le_u16(table, offset + 8, 0)?;
        finalize_acpi_table(table);
    }
    {
        let table = &mut arena[DSDT_OFFSET..DSDT_OFFSET + dsdt_length];
        write_acpi_header(table, b"DSDT", 2, b"NCCDSDT ")?;
        table[36..].copy_from_slice(&aml);
        finalize_acpi_table(table);
    }

    arena[0..8].copy_from_slice(b"RSD PTR ");
    arena[9..15].copy_from_slice(b"NCC   ");
    arena[15] = 2;
    put_le_u32(&mut arena, 16, rsdt_address as u32)?;
    put_le_u32(&mut arena, 20, 36)?;
    put_le_u64(&mut arena, 24, xsdt_address)?;
    arena[8] = wrapping_checksum(&arena[..20]);
    arena[32] = wrapping_checksum(&arena[..36]);

    Ok(GuestFirmwareTable {
        kind: GuestFirmwareTableKind::AcpiRsdp,
        guest_physical_address: PC_ACPI_BASE,
        bytes: arena,
    })
}

fn build_pc_dsdt_aml(virtio_devices: usize) -> Result<Vec<u8>, GuestSystemError> {
    let mut aml = vec![
        0x08, b'_', b'S', b'5', b'_', 0x12, 0x06, 0x02, 0x0a, 0x05, 0x0a, 0x05,
    ];
    for index in 0..virtio_devices {
        let address = PC_VIRTIO_MMIO_BASE
            .checked_add(index as u64 * PC_VIRTIO_MMIO_STRIDE)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(GuestSystemError::AddressOverflow)?;
        let interrupt = PC_VIRTIO_IRQ_BASE
            .checked_add(index as u32)
            .ok_or(GuestSystemError::AddressOverflow)?;
        let name = format!("V{index:03X}");
        if name.len() != 4 {
            return Err(GuestSystemError::InvalidDisk(
                "VirtIO ACPI device name exceeds one NameSeg".into(),
            ));
        }

        let mut resource = vec![0x86, 0x09, 0x00, 0x01];
        resource.extend_from_slice(&address.to_le_bytes());
        resource.extend_from_slice(&(PC_VIRTIO_MMIO_STRIDE as u32).to_le_bytes());
        resource.extend_from_slice(&[0x89, 0x06, 0x00, 0x01, 0x01]);
        resource.extend_from_slice(&interrupt.to_le_bytes());
        resource.extend_from_slice(&[0x79, 0x00]);

        let mut body = Vec::new();
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(&[
            0x08, b'_', b'H', b'I', b'D', 0x0d, b'L', b'N', b'R', b'O', b'0', b'0', b'0', b'5',
            0x00,
        ]);
        body.extend_from_slice(&[0x08, b'_', b'U', b'I', b'D']);
        write_aml_integer(&mut body, index as u32);
        body.extend_from_slice(&[0x08, b'_', b'C', b'R', b'S', 0x11]);
        let mut buffer = Vec::new();
        write_aml_integer(&mut buffer, resource.len() as u32);
        buffer.extend_from_slice(&resource);
        write_aml_package_length(&mut body, buffer.len())?;
        body.extend_from_slice(&buffer);

        aml.extend_from_slice(&[0x5b, 0x82]);
        write_aml_package_length(&mut aml, body.len())?;
        aml.extend_from_slice(&body);
    }
    Ok(aml)
}

fn write_aml_integer(output: &mut Vec<u8>, value: u32) {
    match value {
        0 => output.push(0x00),
        1 => output.push(0x01),
        2..=0xff => output.extend_from_slice(&[0x0a, value as u8]),
        0x100..=0xffff => {
            output.push(0x0b);
            output.extend_from_slice(&(value as u16).to_le_bytes());
        }
        _ => {
            output.push(0x0c);
            output.extend_from_slice(&value.to_le_bytes());
        }
    }
}

fn write_aml_package_length(
    output: &mut Vec<u8>,
    body_length: usize,
) -> Result<(), GuestSystemError> {
    for width in 1..=4_usize {
        let total = body_length
            .checked_add(width)
            .ok_or(GuestSystemError::AddressOverflow)?;
        let maximum = if width == 1 {
            0x3f_usize
        } else {
            (1_usize << (4 + 8 * (width - 1))) - 1
        };
        if total > maximum {
            continue;
        }
        if width == 1 {
            output.push(total as u8);
        } else {
            output.push((((width - 1) as u8) << 6) | (total as u8 & 0x0f));
            for byte in 0..width - 1 {
                output.push((total >> (4 + byte * 8)) as u8);
            }
        }
        return Ok(());
    }
    Err(GuestSystemError::InvalidFirmware(
        "AML package exceeds the ACPI package-length encoding".into(),
    ))
}

fn write_acpi_header(
    table: &mut [u8],
    signature: &[u8; 4],
    revision: u8,
    oem_table_id: &[u8; 8],
) -> Result<(), GuestSystemError> {
    if table.len() < 36 {
        return Err(GuestSystemError::InvalidFirmware(
            "ACPI table is shorter than its header".into(),
        ));
    }
    let length = table.len() as u32;
    table[0..4].copy_from_slice(signature);
    put_le_u32(table, 4, length)?;
    table[8] = revision;
    table[10..16].copy_from_slice(b"NCC   ");
    table[16..24].copy_from_slice(oem_table_id);
    put_le_u32(table, 24, 1)?;
    table[28..32].copy_from_slice(b"NCC ");
    put_le_u32(table, 32, 1)?;
    Ok(())
}

fn write_acpi_gas(
    table: &mut [u8],
    offset: usize,
    address_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_size: u8,
    address: u64,
) -> Result<(), GuestSystemError> {
    let gas = table
        .get_mut(offset..offset + 12)
        .ok_or_else(|| GuestSystemError::InvalidFirmware("truncated ACPI GAS".into()))?;
    gas[0] = address_space;
    gas[1] = bit_width;
    gas[2] = bit_offset;
    gas[3] = access_size;
    put_le_u64(gas, 4, address)
}

fn finalize_acpi_table(table: &mut [u8]) {
    table[9] = 0;
    table[9] = wrapping_checksum(table);
}

fn wrapping_checksum(bytes: &[u8]) -> u8 {
    0_u8.wrapping_sub(bytes.iter().copied().fold(0_u8, u8::wrapping_add))
}

fn validate_firmware_tables(tables: &[GuestFirmwareTable]) -> Result<(), GuestSystemError> {
    let mut kinds = BTreeSet::new();
    for table in tables {
        if table.bytes.is_empty() || !kinds.insert(table.kind) {
            return Err(GuestSystemError::DuplicateFirmwareTable);
        }
    }
    Ok(())
}

fn append_firmware_tables(regions: &mut Vec<GuestPhysicalRegion>, tables: &[GuestFirmwareTable]) {
    for table in tables {
        regions.push(region(
            &format!("firmware-table-{:?}", table.kind),
            table.guest_physical_address,
            table.bytes.clone(),
            GuestRegionKind::FirmwareTable,
            GuestSegmentFlags::READ_WRITE,
        ));
    }
}

fn default_x86_e820(memory_size: u64) -> Vec<LinuxE820Entry> {
    let mut entries = vec![
        LinuxE820Entry {
            address: 0,
            size: 0x9fc00,
            kind: LinuxE820Entry::RAM,
        },
        LinuxE820Entry {
            address: 0x9fc00,
            size: 0x60400,
            kind: LinuxE820Entry::RESERVED,
        },
    ];
    if memory_size > 0x10_0000 {
        entries.push(LinuxE820Entry {
            address: 0x10_0000,
            size: memory_size - 0x10_0000,
            kind: LinuxE820Entry::RAM,
        });
    }
    entries
}

fn normalize_e820(
    entries: &mut Vec<LinuxE820Entry>,
    memory_size: u64,
) -> Result<(), GuestSystemError> {
    entries.retain(|entry| entry.size != 0);
    entries.sort_by_key(|entry| entry.address);
    if entries.len() > LINUX_E820_MAX_ENTRIES {
        return Err(GuestSystemError::TooManyE820Entries);
    }
    let mut previous_end = 0;
    for entry in entries {
        let end = entry
            .address
            .checked_add(entry.size)
            .ok_or(GuestSystemError::AddressOverflow)?;
        if entry.address < previous_end || end > memory_size || !(1..=5).contains(&entry.kind) {
            return Err(GuestSystemError::InvalidMemory);
        }
        previous_end = end;
    }
    Ok(())
}

fn place_initrd(
    config: &GuestSystemConfig,
    address_limit: u64,
    kernel_end: u64,
    setup_end: u64,
) -> Result<Option<u64>, GuestSystemError> {
    if config.initrd.is_empty() {
        return Ok(None);
    }
    let memory_end = config.memory_base + config.memory_size;
    let upper_exclusive = memory_end.min(address_limit.saturating_add(1));
    let address = if let Some(address) = config.initrd_address {
        address
    } else {
        align_down(
            upper_exclusive
                .checked_sub(config.initrd.len() as u64)
                .ok_or(GuestSystemError::InitrdPlacement)?,
            4096,
        )
    };
    let end = address
        .checked_add(config.initrd.len() as u64)
        .ok_or(GuestSystemError::AddressOverflow)?;
    if address < kernel_end.max(setup_end) || end > upper_exclusive || end > u32::MAX as u64 + 1 {
        return Err(GuestSystemError::InitrdPlacement);
    }
    Ok(Some(address))
}

fn place_high_region(
    config: &GuestSystemConfig,
    size: u64,
    requested: Option<u64>,
    alignment: u64,
    minimum: u64,
) -> Result<u64, GuestSystemError> {
    let memory_end = config
        .memory_base
        .checked_add(config.memory_size)
        .ok_or(GuestSystemError::AddressOverflow)?;
    let address = requested.unwrap_or(align_down(
        memory_end
            .checked_sub(size)
            .ok_or(GuestSystemError::InvalidMemory)?,
        alignment,
    ));
    let end = address
        .checked_add(size)
        .ok_or(GuestSystemError::AddressOverflow)?;
    if address < minimum
        || address < config.memory_base
        || end > memory_end
        || address % alignment != 0
    {
        return Err(GuestSystemError::InvalidMemory);
    }
    Ok(address)
}

fn place_high_region_below(
    config: &GuestSystemConfig,
    size: u64,
    requested: Option<u64>,
    alignment: u64,
    minimum: u64,
    upper_limit: u64,
) -> Result<u64, GuestSystemError> {
    let memory_end = config
        .memory_base
        .checked_add(config.memory_size)
        .ok_or(GuestSystemError::AddressOverflow)?;
    if upper_limit > memory_end || upper_limit < config.memory_base {
        return Err(GuestSystemError::InvalidMemory);
    }
    let address = requested.unwrap_or(align_down(
        upper_limit
            .checked_sub(size)
            .ok_or(GuestSystemError::InvalidMemory)?,
        alignment,
    ));
    let end = address
        .checked_add(size)
        .ok_or(GuestSystemError::AddressOverflow)?;
    if address < minimum
        || address < config.memory_base
        || end > upper_limit
        || address % alignment != 0
    {
        return Err(GuestSystemError::InvalidMemory);
    }
    Ok(address)
}

fn efi_mode_matches(architecture: GuestSystemArchitecture, mode: GuestCpuMode) -> bool {
    matches!(
        (architecture, mode),
        (GuestSystemArchitecture::X86_64, GuestCpuMode::Long64)
            | (GuestSystemArchitecture::I386, GuestCpuMode::Protected32)
            | (GuestSystemArchitecture::Aarch64, GuestCpuMode::Aarch64)
    )
}

fn linux_boot_gdt() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&0x00cf_9a00_0000_ffff_u64.to_le_bytes());
    bytes.extend_from_slice(&0x00cf_9200_0000_ffff_u64.to_le_bytes());
    bytes
}

fn encode_opensbi_dynamic_info(
    xlen: u8,
    next_address: u64,
    next_mode: u64,
    boot_hart: u64,
) -> Result<Vec<u8>, GuestSystemError> {
    let values = [
        OPENSBI_DYNAMIC_INFO_MAGIC,
        OPENSBI_DYNAMIC_INFO_VERSION,
        next_address,
        next_mode,
        0,
        boot_hart,
    ];
    let mut bytes = Vec::new();
    match xlen {
        64 => {
            for value in values {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        32 => {
            for value in values {
                bytes.extend_from_slice(
                    &u32::try_from(value)
                        .map_err(|_| GuestSystemError::AddressOverflow)?
                        .to_le_bytes(),
                );
            }
        }
        _ => return Err(GuestSystemError::InvalidArchitecture),
    }
    Ok(bytes)
}

fn build_riscv_virt_fdt(
    xlen: u8,
    memory_base: u64,
    memory_size: u64,
    vcpus: u32,
    command_line: &str,
    initrd: Option<(u64, u64)>,
    disks: usize,
) -> Result<Vec<u8>, GuestSystemError> {
    if command_line.as_bytes().contains(&0) || vcpus == 0 || disks > 32 {
        return Err(GuestSystemError::InvalidFdt(
            "invalid platform description".into(),
        ));
    }
    let mut fdt = FdtWriter::default();
    fdt.begin_node("")?;
    fdt.property_u32("#address-cells", 2)?;
    fdt.property_u32("#size-cells", 2)?;
    fdt.property_string("compatible", "ncc,riscv-virt")?;
    fdt.property_string("model", "nC portable RISC-V guest")?;

    fdt.begin_node("chosen")?;
    fdt.property_string("bootargs", command_line)?;
    if let Some((start, size)) = initrd {
        fdt.property_u64_cells("linux,initrd-start", start)?;
        fdt.property_u64_cells(
            "linux,initrd-end",
            start
                .checked_add(size)
                .ok_or(GuestSystemError::AddressOverflow)?,
        )?;
    }
    fdt.end_node()?;

    fdt.begin_node(&format!("memory@{memory_base:x}"))?;
    fdt.property_string("device_type", "memory")?;
    fdt.property_reg64("reg", memory_base, memory_size)?;
    fdt.end_node()?;

    fdt.begin_node("cpus")?;
    fdt.property_u32("#address-cells", 1)?;
    fdt.property_u32("#size-cells", 0)?;
    fdt.property_u32("timebase-frequency", 10_000_000)?;
    for cpu in 0..vcpus {
        fdt.begin_node(&format!("cpu@{cpu}"))?;
        fdt.property_string("device_type", "cpu")?;
        fdt.property_u32("reg", cpu)?;
        fdt.property_string("status", "okay")?;
        fdt.property_string("compatible", "riscv")?;
        fdt.property_string(
            "riscv,isa",
            if xlen == 64 {
                "rv64imafdc_zicsr_zifencei"
            } else {
                "rv32imafdc_zicsr_zifencei"
            },
        )?;
        fdt.property_string(
            "mmu-type",
            if xlen == 64 {
                "riscv,sv48"
            } else {
                "riscv,sv32"
            },
        )?;
        fdt.begin_node("interrupt-controller")?;
        fdt.property_empty("interrupt-controller")?;
        fdt.property_u32("#interrupt-cells", 1)?;
        fdt.property_string("compatible", "riscv,cpu-intc")?;
        fdt.property_u32("phandle", 0x100 + cpu)?;
        fdt.end_node()?;
        fdt.end_node()?;
    }
    fdt.end_node()?;

    fdt.begin_node("soc")?;
    fdt.property_string_list("compatible", &["ncc,riscv-virt-soc", "simple-bus"])?;
    fdt.property_u32("#address-cells", 2)?;
    fdt.property_u32("#size-cells", 2)?;
    fdt.property_empty("ranges")?;
    fdt.begin_node("interrupt-controller@c000000")?;
    fdt.property_string_list("compatible", &["sifive,plic-1.0.0", "riscv,plic0"])?;
    fdt.property_empty("interrupt-controller")?;
    fdt.property_u32("#interrupt-cells", 1)?;
    fdt.property_u32("phandle", 1)?;
    fdt.property_u32("riscv,ndev", 127)?;
    fdt.property_reg64("reg", 0x0c00_0000, 0x0400_0000)?;
    let mut extended = Vec::new();
    for cpu in 0..vcpus {
        extended.extend_from_slice(&(0x100 + cpu).to_be_bytes());
        extended.extend_from_slice(&9_u32.to_be_bytes());
        extended.extend_from_slice(&(0x100 + cpu).to_be_bytes());
        extended.extend_from_slice(&11_u32.to_be_bytes());
    }
    fdt.property("interrupts-extended", &extended)?;
    fdt.end_node()?;
    fdt.begin_node("clint@2000000")?;
    fdt.property_string("compatible", "riscv,clint0")?;
    fdt.property_reg64("reg", 0x0200_0000, 0x1_0000)?;
    let mut clint_interrupts = Vec::new();
    for cpu in 0..vcpus {
        clint_interrupts.extend_from_slice(&(0x100 + cpu).to_be_bytes());
        clint_interrupts.extend_from_slice(&3_u32.to_be_bytes());
        clint_interrupts.extend_from_slice(&(0x100 + cpu).to_be_bytes());
        clint_interrupts.extend_from_slice(&7_u32.to_be_bytes());
    }
    fdt.property("interrupts-extended", &clint_interrupts)?;
    fdt.end_node()?;
    fdt.begin_node("uart@10000000")?;
    fdt.property_string("compatible", "ns16550a")?;
    fdt.property_reg64("reg", RISCV_VIRT_UART_BASE, 0x100)?;
    fdt.property_u32("clock-frequency", 3_686_400)?;
    fdt.property_u32("current-speed", 115_200)?;
    fdt.property_u32("interrupt-parent", 1)?;
    fdt.property_u32("interrupts", 10)?;
    fdt.end_node()?;
    for index in 0..disks {
        let address = RISCV_VIRT_VIRTIO_BASE + index as u64 * RISCV_VIRT_VIRTIO_STRIDE;
        fdt.begin_node(&format!("virtio_mmio@{address:x}"))?;
        fdt.property_string("compatible", "virtio,mmio")?;
        fdt.property_reg64("reg", address, RISCV_VIRT_VIRTIO_STRIDE)?;
        fdt.property_u32("interrupt-parent", 1)?;
        fdt.property_u32("interrupts", 1 + index as u32)?;
        fdt.end_node()?;
    }
    fdt.end_node()?;
    fdt.end_node()?;
    fdt.finish(0)
}

#[derive(Default)]
struct FdtWriter {
    structure: Vec<u8>,
    strings: Vec<u8>,
    string_offsets: BTreeMap<String, u32>,
    depth: usize,
}

impl FdtWriter {
    fn token(&mut self, token: u32) {
        self.structure.extend_from_slice(&token.to_be_bytes());
    }

    fn pad(&mut self) {
        while self.structure.len() % 4 != 0 {
            self.structure.push(0);
        }
    }

    fn begin_node(&mut self, name: &str) -> Result<(), GuestSystemError> {
        if name.as_bytes().contains(&0) {
            return Err(GuestSystemError::InvalidFdt(
                "node name contains NUL".into(),
            ));
        }
        self.token(1);
        self.structure.extend_from_slice(name.as_bytes());
        self.structure.push(0);
        self.pad();
        self.depth += 1;
        Ok(())
    }

    fn end_node(&mut self) -> Result<(), GuestSystemError> {
        if self.depth == 0 {
            return Err(GuestSystemError::InvalidFdt("unbalanced node".into()));
        }
        self.token(2);
        self.depth -= 1;
        Ok(())
    }

    fn property(&mut self, name: &str, value: &[u8]) -> Result<(), GuestSystemError> {
        if self.depth == 0 || name.is_empty() || name.as_bytes().contains(&0) {
            return Err(GuestSystemError::InvalidFdt("invalid property".into()));
        }
        let name_offset = if let Some(offset) = self.string_offsets.get(name) {
            *offset
        } else {
            let offset =
                u32::try_from(self.strings.len()).map_err(|_| GuestSystemError::AddressOverflow)?;
            self.strings.extend_from_slice(name.as_bytes());
            self.strings.push(0);
            self.string_offsets.insert(name.into(), offset);
            offset
        };
        self.token(3);
        self.structure.extend_from_slice(
            &u32::try_from(value.len())
                .map_err(|_| GuestSystemError::AddressOverflow)?
                .to_be_bytes(),
        );
        self.structure.extend_from_slice(&name_offset.to_be_bytes());
        self.structure.extend_from_slice(value);
        self.pad();
        Ok(())
    }

    fn property_empty(&mut self, name: &str) -> Result<(), GuestSystemError> {
        self.property(name, &[])
    }

    fn property_u32(&mut self, name: &str, value: u32) -> Result<(), GuestSystemError> {
        self.property(name, &value.to_be_bytes())
    }

    fn property_u64_cells(&mut self, name: &str, value: u64) -> Result<(), GuestSystemError> {
        self.property(name, &value.to_be_bytes())
    }

    fn property_reg64(
        &mut self,
        name: &str,
        address: u64,
        size: u64,
    ) -> Result<(), GuestSystemError> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&address.to_be_bytes());
        bytes.extend_from_slice(&size.to_be_bytes());
        self.property(name, &bytes)
    }

    fn property_string(&mut self, name: &str, value: &str) -> Result<(), GuestSystemError> {
        if value.as_bytes().contains(&0) {
            return Err(GuestSystemError::InvalidFdt("string contains NUL".into()));
        }
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        self.property(name, &bytes)
    }

    fn property_string_list(
        &mut self,
        name: &str,
        values: &[&str],
    ) -> Result<(), GuestSystemError> {
        let mut bytes = Vec::new();
        for value in values {
            if value.as_bytes().contains(&0) {
                return Err(GuestSystemError::InvalidFdt("string contains NUL".into()));
            }
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
        self.property(name, &bytes)
    }

    fn finish(mut self, boot_cpu: u32) -> Result<Vec<u8>, GuestSystemError> {
        if self.depth != 0 {
            return Err(GuestSystemError::InvalidFdt("unclosed node".into()));
        }
        self.token(9);
        let reserve_bytes = 16_usize;
        let structure_offset = 40 + reserve_bytes;
        let strings_offset = structure_offset
            .checked_add(self.structure.len())
            .ok_or(GuestSystemError::AddressOverflow)?;
        let total_size = strings_offset
            .checked_add(self.strings.len())
            .ok_or(GuestSystemError::AddressOverflow)?;
        let mut bytes = vec![0_u8; 40];
        put_be_u32(&mut bytes, 0, 0xd00d_feed)?;
        put_be_u32(&mut bytes, 4, total_size as u32)?;
        put_be_u32(&mut bytes, 8, structure_offset as u32)?;
        put_be_u32(&mut bytes, 12, strings_offset as u32)?;
        put_be_u32(&mut bytes, 16, 40)?;
        put_be_u32(&mut bytes, 20, 17)?;
        put_be_u32(&mut bytes, 24, 16)?;
        put_be_u32(&mut bytes, 28, boot_cpu)?;
        put_be_u32(&mut bytes, 32, self.strings.len() as u32)?;
        put_be_u32(&mut bytes, 36, self.structure.len() as u32)?;
        bytes.resize(structure_offset, 0);
        bytes.extend_from_slice(&self.structure);
        bytes.extend_from_slice(&self.strings);
        parse_fdt(&bytes).map_err(|error| GuestSystemError::InvalidFdt(error.to_string()))?;
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestPowerAction {
    None,
    Shutdown,
    WarmReboot,
    ColdReboot,
    Crash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPowerContract {
    pub acpi_pm1_control_ports: Vec<u16>,
    pub reset_control_port: Option<u16>,
    pub keyboard_control_port: Option<u16>,
    pub uefi_reset_system: bool,
    pub sbi_system_reset: bool,
}

impl GuestPowerContract {
    pub fn pc() -> Self {
        Self {
            acpi_pm1_control_ports: vec![0x604, 0xb004],
            reset_control_port: Some(0xcf9),
            keyboard_control_port: Some(0x64),
            uefi_reset_system: false,
            sbi_system_reset: false,
        }
    }

    pub fn uefi() -> Self {
        let mut contract = Self::pc();
        contract.uefi_reset_system = true;
        contract
    }

    pub fn sbi() -> Self {
        Self {
            acpi_pm1_control_ports: Vec::new(),
            reset_control_port: None,
            keyboard_control_port: None,
            uefi_reset_system: false,
            sbi_system_reset: true,
        }
    }

    pub fn decode_port_write(&self, port: u16, value: u32) -> GuestPowerAction {
        if self.acpi_pm1_control_ports.contains(&port) && value & 0x3c00 == 0x2000 {
            return GuestPowerAction::Shutdown;
        }
        if self.reset_control_port == Some(port) && value & 0x04 != 0 {
            return if value & 0x02 != 0 {
                GuestPowerAction::ColdReboot
            } else {
                GuestPowerAction::WarmReboot
            };
        }
        if self.keyboard_control_port == Some(port) && value as u8 == 0xfe {
            return GuestPowerAction::WarmReboot;
        }
        GuestPowerAction::None
    }

    pub fn decode_uefi_reset(&self, reset_type: u32) -> GuestPowerAction {
        if !self.uefi_reset_system {
            return GuestPowerAction::None;
        }
        match reset_type {
            0 => GuestPowerAction::ColdReboot,
            1 => GuestPowerAction::WarmReboot,
            2 => GuestPowerAction::Shutdown,
            _ => GuestPowerAction::ColdReboot,
        }
    }

    pub fn decode_sbi_reset(
        &self,
        extension: u64,
        function: u64,
        reset_type: u64,
    ) -> GuestPowerAction {
        if !self.sbi_system_reset || extension != 0x5352_5354 || function != 0 {
            return GuestPowerAction::None;
        }
        match reset_type {
            0 => GuestPowerAction::Shutdown,
            1 => GuestPowerAction::ColdReboot,
            2 => GuestPowerAction::WarmReboot,
            _ => GuestPowerAction::Crash,
        }
    }
}

fn region(
    name: &str,
    guest_physical_address: u64,
    bytes: Vec<u8>,
    kind: GuestRegionKind,
    flags: GuestSegmentFlags,
) -> GuestPhysicalRegion {
    GuestPhysicalRegion {
        name: name.into(),
        guest_physical_address,
        bytes,
        flags,
        kind,
    }
}

fn nul_terminated(value: &str) -> Result<Vec<u8>, GuestSystemError> {
    if value.as_bytes().contains(&0) {
        return Err(GuestSystemError::InvalidCommandLine);
    }
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    Ok(bytes)
}

fn align_up(value: u64, alignment: u64) -> Result<u64, GuestSystemError> {
    if !alignment.is_power_of_two() {
        return Err(GuestSystemError::InvalidMemory);
    }
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(GuestSystemError::AddressOverflow)
}

const fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

fn le_u16(bytes: &[u8], offset: usize) -> Result<u16, GuestSystemError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or_else(|| GuestSystemError::InvalidKernel("truncated field".into()))?
            .try_into()
            .unwrap(),
    ))
}

fn le_u32(bytes: &[u8], offset: usize) -> Result<u32, GuestSystemError> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(|| GuestSystemError::InvalidKernel("truncated field".into()))?
            .try_into()
            .unwrap(),
    ))
}

fn put_le_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), GuestSystemError> {
    bytes
        .get_mut(offset..offset + 2)
        .ok_or(GuestSystemError::AddressOverflow)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_le_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), GuestSystemError> {
    bytes
        .get_mut(offset..offset + 4)
        .ok_or(GuestSystemError::AddressOverflow)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_le_u64(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), GuestSystemError> {
    bytes
        .get_mut(offset..offset + 8)
        .ok_or(GuestSystemError::AddressOverflow)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_be_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), GuestSystemError> {
    bytes
        .get_mut(offset..offset + 4)
        .ok_or(GuestSystemError::AddressOverflow)?
        .copy_from_slice(&value.to_be_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bzimage_fixture() -> Vec<u8> {
        let mut image = vec![0x90; 32 * 1024];
        image[0x1f1] = 4;
        image[0x202..0x206].copy_from_slice(b"HdrS");
        image[0x206..0x208].copy_from_slice(&0x020c_u16.to_le_bytes());
        image[0x211] = 1;
        image[0x214..0x218].copy_from_slice(&0x10_0000_u32.to_le_bytes());
        image[0x22c..0x230].copy_from_slice(&0x37ff_ffff_u32.to_le_bytes());
        image[0x230..0x234].copy_from_slice(&0x20_0000_u32.to_le_bytes());
        image[0x234] = 1;
        image[0x238..0x23c].copy_from_slice(&2048_u32.to_le_bytes());
        let image_len = image.len() as u32;
        image[0x260..0x264].copy_from_slice(&image_len.to_le_bytes());
        image
    }

    fn riscv_elf64_fixture(load: u64) -> Vec<u8> {
        let mut elf = vec![0u8; 0x140];
        elf[0..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&2_u16.to_le_bytes());
        elf[18..20].copy_from_slice(&243_u16.to_le_bytes());
        elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
        elf[24..32].copy_from_slice(&load.to_le_bytes());
        elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
        elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
        elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
        elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
        elf[68..72].copy_from_slice(&5_u32.to_le_bytes());
        elf[72..80].copy_from_slice(&0x100_u64.to_le_bytes());
        elf[80..88].copy_from_slice(&load.to_le_bytes());
        elf[88..96].copy_from_slice(&load.to_le_bytes());
        elf[96..104].copy_from_slice(&64_u64.to_le_bytes());
        elf[104..112].copy_from_slice(&64_u64.to_le_bytes());
        elf[112..120].copy_from_slice(&0x1000_u64.to_le_bytes());
        elf[0x100..0x140].fill(0x13);
        elf
    }

    #[test]
    fn bzimage_plan_materializes_zero_page_cmdline_initrd_and_e820() {
        let mut config = GuestSystemConfig::linux_bzimage(bzimage_fixture(), 128 * 1024 * 1024);
        config.initrd = vec![0xa5; 8192];
        config.command_line = "console=ttyS0 root=/dev/vda ro".into();
        config
            .disks
            .push(VirtioDiskAttachment::new("root", vec![7; 4096], 512, true).unwrap());
        let mut plan = GuestBootPlan::build(&config).unwrap();
        plan.validate().unwrap();
        let linux = plan.linux.as_ref().unwrap();
        assert_eq!(linux.protocol_version, 0x020c);
        assert_eq!(linux.e820.len(), 3);
        assert!(linux.initrd_address.unwrap() < linux.initrd_address_limit);
        assert_eq!(
            plan.initial_cpu.registers["esi"],
            LINUX_BOOT_PARAMS_ADDRESS as u128
        );
        let mut memory = vec![0; config.memory_size as usize];
        plan.materialize(&mut memory).unwrap();
        assert_eq!(
            &memory[LINUX_BOOT_PARAMS_ADDRESS as usize + 0x202..][..4],
            b"HdrS"
        );
        assert_eq!(
            &memory[LINUX_COMMAND_LINE_ADDRESS as usize..][..13],
            b"console=ttyS0"
        );
        assert_eq!(
            memory[LINUX_BOOT_PARAMS_ADDRESS as usize + LINUX_E820_COUNT_OFFSET],
            3
        );
        let acpi = plan
            .firmware_tables
            .iter()
            .find(|table| table.kind == GuestFirmwareTableKind::AcpiRsdp)
            .unwrap();
        assert_eq!(acpi.guest_physical_address, PC_ACPI_BASE);
        let rsdp = crate::parse_acpi_rsdp(&acpi.bytes[..36]).unwrap();
        assert_eq!(u64::from(rsdp.rsdt_address), PC_ACPI_BASE + 0x100);
        assert_eq!(rsdp.xsdt_address, PC_ACPI_BASE + 0x200);
        assert_eq!(
            u64::from_le_bytes(
                memory[LINUX_BOOT_PARAMS_ADDRESS as usize + 0x70..][..8]
                    .try_into()
                    .unwrap()
            ),
            PC_ACPI_BASE
        );
        let dsdt_length =
            u32::from_le_bytes(acpi.bytes[0x3004..0x3008].try_into().unwrap()) as usize;
        for (offset, length, signature) in [
            (0x100, 44, b"RSDT".as_slice()),
            (0x200, 52, b"XSDT".as_slice()),
            (0x1000, 244, b"FACP".as_slice()),
            (0x2000, 74, b"APIC".as_slice()),
            (0x3000, dsdt_length, b"DSDT".as_slice()),
        ] {
            let table = &acpi.bytes[offset..offset + length];
            assert_eq!(&table[..4], signature);
            assert_eq!(table.iter().copied().fold(0_u8, u8::wrapping_add), 0);
            crate::parse_acpi_sdt(table).unwrap();
        }
        let dsdt = &acpi.bytes[0x3000..0x3000 + dsdt_length];
        assert!(dsdt.windows(8).any(|bytes| bytes == b"LNRO0005"));
        assert!(dsdt
            .windows(4)
            .any(|bytes| bytes == (PC_VIRTIO_MMIO_BASE as u32).to_le_bytes()));
        assert!(dsdt
            .windows(4)
            .any(|bytes| bytes == PC_VIRTIO_IRQ_BASE.to_le_bytes()));
    }

    #[test]
    fn pc_acpi_virtio_aml_matches_the_lnro0005_platform_contract() {
        let table = build_pc_acpi_firmware_with_virtio(2, 2).unwrap();
        let length = u32::from_le_bytes(table.bytes[0x3004..0x3008].try_into().unwrap()) as usize;
        let dsdt = &table.bytes[0x3000..0x3000 + length];
        crate::parse_acpi_sdt(dsdt).unwrap();
        assert_eq!(
            dsdt.windows(8)
                .filter(|bytes| *bytes == b"LNRO0005")
                .count(),
            2
        );
        for index in 0..2_u32 {
            let address = (PC_VIRTIO_MMIO_BASE + u64::from(index) * PC_VIRTIO_MMIO_STRIDE) as u32;
            let irq = PC_VIRTIO_IRQ_BASE + index;
            assert!(dsdt.windows(4).any(|bytes| bytes == address.to_le_bytes()));
            assert!(dsdt.windows(4).any(|bytes| bytes == irq.to_le_bytes()));
        }
        assert_eq!(dsdt.iter().copied().fold(0_u8, u8::wrapping_add), 0);
    }

    #[test]
    fn opensbi_plan_builds_valid_fdt_dynamic_info_and_initrd() {
        let mut config = GuestSystemConfig::opensbi(
            GuestSystemArchitecture::RiscV64,
            riscv_elf64_fixture(0x8020_0000),
            vec![0x13; 0x20_000],
            256 * 1024 * 1024,
        );
        config.initrd = vec![0x44; 4096];
        config.vcpu_count = 4;
        config
            .disks
            .push(VirtioDiskAttachment::new("root", vec![0; 4096], 512, false).unwrap());
        let mut plan = GuestBootPlan::build(&config).unwrap();
        plan.validate().unwrap();
        let handoff = plan.opensbi.as_ref().unwrap();
        assert_eq!(handoff.xlen, 64);
        assert_eq!(
            plan.initial_cpu.registers["a1"],
            handoff.fdt_address as u128
        );
        assert_eq!(
            plan.initial_cpu.registers["a2"],
            handoff.dynamic_info_address as u128
        );
        let fdt = plan
            .regions
            .iter()
            .find(|region| region.kind == GuestRegionKind::Fdt)
            .unwrap();
        let parsed = parse_fdt(&fdt.bytes).unwrap();
        assert_eq!(
            parsed.bootargs.as_deref(),
            Some(config.command_line.as_str())
        );
        assert_eq!(parsed.topology.cpus.len(), 4);
    }

    #[test]
    fn power_contract_decodes_pc_uefi_and_sbi_shutdown_reboot() {
        let pc = GuestPowerContract::pc();
        assert_eq!(
            pc.decode_port_write(0x604, 0x2000),
            GuestPowerAction::Shutdown
        );
        assert_eq!(
            pc.decode_port_write(0xcf9, 0x06),
            GuestPowerAction::ColdReboot
        );
        assert_eq!(
            pc.decode_port_write(0x64, 0xfe),
            GuestPowerAction::WarmReboot
        );
        let uefi = GuestPowerContract::uefi();
        assert_eq!(uefi.decode_uefi_reset(2), GuestPowerAction::Shutdown);
        let sbi = GuestPowerContract::sbi();
        assert_eq!(
            sbi.decode_sbi_reset(0x5352_5354, 0, 0),
            GuestPowerAction::Shutdown
        );
        assert_eq!(
            sbi.decode_sbi_reset(0x5352_5354, 0, 2),
            GuestPowerAction::WarmReboot
        );
    }

    #[test]
    fn overlapping_regions_are_rejected_before_materialization() {
        let mut config = GuestSystemConfig::linux_bzimage(bzimage_fixture(), 64 * 1024 * 1024);
        config.firmware_tables.push(GuestFirmwareTable {
            kind: GuestFirmwareTableKind::AcpiRsdp,
            guest_physical_address: LINUX_COMMAND_LINE_ADDRESS,
            bytes: vec![1; 64],
        });
        assert!(matches!(
            GuestBootPlan::build(&config),
            Err(GuestSystemError::RegionOverlap { .. })
        ));
    }
}
