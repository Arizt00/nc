use core::fmt;

use std::collections::BTreeMap;

use crate::smp::{KernelSmpPlatformV1, SmpArchitecture, SmpInterruptModel, SmpStartMethod};

pub const KERNEL_BOOT_INFO_MAGIC: u64 = u64::from_le_bytes(*b"NCCBOO1\0");
pub const KERNEL_SERIAL_MAGIC: u64 = u64::from_le_bytes(*b"NCCSER1\0");
pub const KERNEL_PLATFORM_MAGIC: u64 = u64::from_le_bytes(*b"NCCPLT1\0");
pub const KERNEL_CPU_TOPOLOGY_MAGIC: u64 = u64::from_le_bytes(*b"NCCCPU1\0");
pub const FDT_MAGIC: u32 = 0xd00d_feed;
pub const UEFI_PAGE_SIZE: u64 = 4096;

pub const BOOT_FLAG_MEMORY_MAP: u64 = 1 << 0;
pub const BOOT_FLAG_SERIAL: u64 = 1 << 1;
pub const BOOT_FLAG_PLATFORM_TABLE: u64 = 1 << 2;
pub const BOOT_FLAG_FRAMEBUFFER: u64 = 1 << 3;
pub const BOOT_FLAG_CPU_TOPOLOGY: u64 = 1 << 4;

pub const FRAMEBUFFER_FORMAT_RGB_RESERVED_8: u32 = 0;
pub const FRAMEBUFFER_FORMAT_BGR_RESERVED_8: u32 = 1;
pub const FRAMEBUFFER_FORMAT_BIT_MASK: u32 = 2;
pub const FRAMEBUFFER_FLAG_LINEAR: u64 = 1 << 0;
pub const FRAMEBUFFER_FLAG_FIRMWARE_OWNED: u64 = 1 << 1;

pub const PLATFORM_KIND_NCC_BOOT: u32 = 1;
pub const PLATFORM_KIND_ACPI_RSDP: u32 = 2;
pub const PLATFORM_KIND_UEFI_SYSTEM_TABLE: u32 = 3;
pub const PLATFORM_KIND_MULTIBOOT2_TAGS: u32 = 4;
pub const PLATFORM_KIND_BIOS_E820: u32 = 5;
pub const PLATFORM_KIND_FDT: u32 = 6;
pub const PLATFORM_KIND_OPENSBI_HART: u32 = 7;
pub const PLATFORM_KIND_CPU_TOPOLOGY: u32 = 10;
pub const PLATFORM_KIND_SMBIOS32: u32 = 11;
pub const PLATFORM_KIND_SMBIOS64: u32 = 12;
pub const PLATFORM_KIND_SMBIOS_TABLES: u32 = 13;
pub const PLATFORM_KIND_MULTIBOOT2_BOOT_DEVICE: u32 = 14;
pub const PLATFORM_KIND_UEFI_MEMORY_MAP: u32 = 15;
pub const PLATFORM_KIND_UEFI_IMAGE_HANDLE: u32 = 16;
pub const PLATFORM_KIND_MULTIBOOT2_LOAD_BASE: u32 = 17;
pub const PLATFORM_KIND_MULTIBOOT2_VBE: u32 = 18;
pub const PLATFORM_KIND_MULTIBOOT2_ELF_SECTIONS: u32 = 19;
pub const PLATFORM_KIND_MULTIBOOT2_APM: u32 = 20;
pub const PLATFORM_KIND_MULTIBOOT2_NETWORK: u32 = 21;
pub const PLATFORM_KIND_UEFI_BOOT_SERVICES: u32 = 22;
pub const PLATFORM_KIND_MULTIBOOT2_FRAMEBUFFER: u32 = 23;
pub const PLATFORM_KIND_MULTIBOOT2_BOOT_LOADER_NAME: u32 = 24;
pub const PLATFORM_KIND_SMP_PLATFORM: u32 = 25;

pub const PLATFORM_FLAG_PRESENT: u32 = 1 << 0;
pub const PLATFORM_FLAG_REQUIRED: u32 = 1 << 1;
pub const PLATFORM_FLAG_OWN_CONTRACT: u32 = 1 << 2;
pub const PLATFORM_FLAG_EXTERNAL: u32 = 1 << 3;
pub const PLATFORM_FLAG_ZERO_IF_ABSENT: u32 = 1 << 4;
pub const PLATFORM_FLAG_RUNTIME_PROBE: u32 = 1 << 5;
pub const PLATFORM_FLAG_DYNAMIC_LENGTH: u32 = 1 << 6;

pub const MEMORY_KIND_RESERVED: u32 = 0;
pub const MEMORY_KIND_LOADER: u32 = 1;
pub const MEMORY_KIND_KERNEL_IMAGE: u32 = 2;
pub const MEMORY_KIND_KERNEL_SEGMENT: u32 = 3;
pub const MEMORY_KIND_INITIAL_STACK: u32 = 4;
pub const MEMORY_KIND_USABLE: u32 = 5;
pub const MEMORY_KIND_ACPI_RECLAIM: u32 = 6;
pub const MEMORY_KIND_ACPI_NVS: u32 = 7;
pub const MEMORY_KIND_BAD: u32 = 8;
pub const MEMORY_KIND_RUNTIME: u32 = 9;
pub const MEMORY_KIND_MMIO: u32 = 10;
pub const MEMORY_KIND_PERSISTENT: u32 = 11;

pub const MEMORY_SOURCE_NCC_LOADER: u32 = 1;
pub const MEMORY_SOURCE_ELF: u32 = 2;
pub const MEMORY_SOURCE_BIOS_E820: u32 = 3;
pub const MEMORY_SOURCE_UEFI: u32 = 4;
pub const MEMORY_SOURCE_MULTIBOOT2: u32 = 5;
pub const MEMORY_SOURCE_FDT: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareError {
    Truncated(&'static str),
    InvalidMagic(&'static str),
    InvalidVersion(&'static str),
    InvalidChecksum(&'static str),
    InvalidLength(&'static str),
    InvalidAlignment(&'static str),
    InvalidRange(&'static str),
    InvalidTag(u32),
    InvalidProperty(String),
    MissingRequired(&'static str),
    DuplicateCpu(u64),
    ArithmeticOverflow,
}

impl fmt::Display for FirmwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated(item) => write!(f, "truncated firmware {item}"),
            Self::InvalidMagic(item) => write!(f, "invalid {item} magic"),
            Self::InvalidVersion(item) => write!(f, "unsupported {item} version"),
            Self::InvalidChecksum(item) => write!(f, "invalid {item} checksum"),
            Self::InvalidLength(item) => write!(f, "invalid {item} length"),
            Self::InvalidAlignment(item) => write!(f, "invalid {item} alignment"),
            Self::InvalidRange(item) => write!(f, "{item} points outside retained bytes"),
            Self::InvalidTag(tag) => write!(f, "invalid firmware tag {tag}"),
            Self::InvalidProperty(property) => write!(f, "invalid FDT property '{property}'"),
            Self::MissingRequired(item) => write!(f, "missing required firmware item {item}"),
            Self::DuplicateCpu(id) => write!(f, "duplicate firmware CPU id {id:#x}"),
            Self::ArithmeticOverflow => write!(f, "firmware arithmetic overflow"),
        }
    }
}

impl std::error::Error for FirmwareError {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelBootInfoV1 {
    pub magic: u64,
    pub version: u64,
    pub flags: u64,
    pub memory_map_ptr: u64,
    pub memory_map_len: u64,
    pub framebuffer_ptr: u64,
    pub serial_ptr: u64,
    pub platform_table_ptr: u64,
    pub command_line_ptr: u64,
    pub kernel_payload_ptr: u64,
    pub kernel_payload_size: u64,
}

impl KernelBootInfoV1 {
    pub const BYTE_SIZE: usize = 88;

    pub fn validate(self) -> Result<Self, FirmwareError> {
        if self.magic != KERNEL_BOOT_INFO_MAGIC {
            return Err(FirmwareError::InvalidMagic("ncc_kernel_boot_info_v1"));
        }
        if self.version != 1 {
            return Err(FirmwareError::InvalidVersion("ncc_kernel_boot_info_v1"));
        }
        if self.flags & BOOT_FLAG_MEMORY_MAP != 0
            && (self.memory_map_ptr == 0 || self.memory_map_len == 0)
        {
            return Err(FirmwareError::MissingRequired("memory map"));
        }
        if self.flags & BOOT_FLAG_PLATFORM_TABLE != 0 && self.platform_table_ptr == 0 {
            return Err(FirmwareError::MissingRequired("platform table"));
        }
        if self.flags & BOOT_FLAG_FRAMEBUFFER != 0 && self.framebuffer_ptr == 0 {
            return Err(FirmwareError::MissingRequired("framebuffer"));
        }
        if self.flags & BOOT_FLAG_SERIAL != 0 && self.serial_ptr == 0 {
            return Err(FirmwareError::MissingRequired("serial"));
        }
        Ok(self)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelMemoryRegionV1 {
    pub base: u64,
    pub length: u64,
    pub kind: u32,
    pub flags: u32,
    pub source: u32,
    pub reserved: u32,
}

impl KernelMemoryRegionV1 {
    pub const BYTE_SIZE: usize = 32;

    pub fn end(self) -> Result<u64, FirmwareError> {
        self.base
            .checked_add(self.length)
            .ok_or(FirmwareError::ArithmeticOverflow)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelFramebufferV1 {
    pub base: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub format: u32,
    pub flags: u64,
}

impl KernelFramebufferV1 {
    pub const BYTE_SIZE: usize = 32;

    pub fn validate(self) -> Result<Self, FirmwareError> {
        if self.base == 0 || self.width == 0 || self.height == 0 || self.pitch == 0 {
            return Err(FirmwareError::InvalidRange("framebuffer"));
        }
        let minimum_pitch = u64::from(self.width)
            .checked_mul(4)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        if u64::from(self.pitch) < minimum_pitch {
            return Err(FirmwareError::InvalidLength("framebuffer pitch"));
        }
        Ok(self)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelSerialV1 {
    pub magic: u64,
    pub version: u32,
    pub kind: u32,
    pub base: u64,
    pub irq: u32,
    pub baud: u32,
    pub flags: u32,
    pub register_shift: u32,
}

impl KernelSerialV1 {
    pub const BYTE_SIZE: usize = 40;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelPlatformEntryV1 {
    pub kind: u32,
    pub flags: u32,
    pub ptr: u64,
    pub length: u64,
    pub checksum_or_revision: u64,
}

impl KernelPlatformEntryV1 {
    pub const BYTE_SIZE: usize = 32;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelCpuTopologyHeaderV1 {
    pub magic: u64,
    pub version: u32,
    pub entry_count: u32,
    pub flags: u64,
    pub bootstrap_hardware_id: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KernelCpuTopologyEntryV1 {
    pub logical_id: u32,
    pub flags: u32,
    pub hardware_id: u64,
    pub firmware_id: u64,
    pub numa_node: u32,
    pub package_id: u16,
    pub core_id: u16,
    pub thread_id: u16,
    pub reserved: u16,
    pub stack_top: u64,
}

impl KernelCpuTopologyEntryV1 {
    pub const BYTE_SIZE: usize = 48;
    pub const FLAG_ENABLED: u32 = 1 << 0;
    pub const FLAG_BOOTSTRAP: u32 = 1 << 1;
    pub const FLAG_ONLINE_CAPABLE: u32 = 1 << 2;
}

const _: [(); KernelBootInfoV1::BYTE_SIZE] = [(); core::mem::size_of::<KernelBootInfoV1>()];
const _: [(); KernelMemoryRegionV1::BYTE_SIZE] = [(); core::mem::size_of::<KernelMemoryRegionV1>()];
const _: [(); KernelFramebufferV1::BYTE_SIZE] = [(); core::mem::size_of::<KernelFramebufferV1>()];
const _: [(); KernelSerialV1::BYTE_SIZE] = [(); core::mem::size_of::<KernelSerialV1>()];
const _: [(); KernelPlatformEntryV1::BYTE_SIZE] =
    [(); core::mem::size_of::<KernelPlatformEntryV1>()];
const _: [(); KernelCpuTopologyEntryV1::BYTE_SIZE] =
    [(); core::mem::size_of::<KernelCpuTopologyEntryV1>()];
const _: [(); KernelCpuTopologyHeaderV1::BYTE_SIZE] =
    [(); core::mem::size_of::<KernelCpuTopologyHeaderV1>()];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareArchitecture {
    X86,
    Aarch64,
    RiscV,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirmwareCpu {
    pub hardware_id: u64,
    pub firmware_id: u64,
    pub enabled: bool,
    pub bootstrap: bool,
    pub numa_node: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirmwareTopology {
    pub cpus: Vec<FirmwareCpu>,
    pub hierarchy: BTreeMap<u64, FirmwareCpuHierarchy>,
    pub interrupt_controller: Option<String>,
    pub interrupt_controller_base: Option<u64>,
    pub timer_frequency_hz: Option<u64>,
    pub smp_platform: Option<KernelSmpPlatformV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FirmwareCpuHierarchy {
    pub package_id: u16,
    pub core_id: u16,
    pub thread_id: u16,
}

impl FirmwareTopology {
    pub fn normalize(&mut self, bootstrap_hardware_id: Option<u64>) -> Result<(), FirmwareError> {
        self.cpus.sort_by_key(|cpu| cpu.hardware_id);
        let mut previous = None;
        for cpu in &self.cpus {
            if previous == Some(cpu.hardware_id) {
                return Err(FirmwareError::DuplicateCpu(cpu.hardware_id));
            }
            previous = Some(cpu.hardware_id);
        }
        let bootstrap = bootstrap_hardware_id
            .or_else(|| {
                self.cpus
                    .iter()
                    .find(|cpu| cpu.bootstrap)
                    .map(|cpu| cpu.hardware_id)
            })
            .or_else(|| {
                self.cpus
                    .iter()
                    .find(|cpu| cpu.enabled)
                    .map(|cpu| cpu.hardware_id)
            });
        for cpu in &mut self.cpus {
            cpu.bootstrap = Some(cpu.hardware_id) == bootstrap;
        }
        Ok(())
    }

    pub fn entries(&self) -> Vec<KernelCpuTopologyEntryV1> {
        self.cpus
            .iter()
            .enumerate()
            .map(|(logical_id, cpu)| {
                let hierarchy =
                    self.hierarchy
                        .get(&cpu.hardware_id)
                        .copied()
                        .unwrap_or(FirmwareCpuHierarchy {
                            package_id: 0,
                            core_id: logical_id as u16,
                            thread_id: 0,
                        });
                KernelCpuTopologyEntryV1 {
                    logical_id: logical_id as u32,
                    flags: (if cpu.enabled {
                        KernelCpuTopologyEntryV1::FLAG_ENABLED
                            | KernelCpuTopologyEntryV1::FLAG_ONLINE_CAPABLE
                    } else {
                        0
                    }) | if cpu.bootstrap {
                        KernelCpuTopologyEntryV1::FLAG_BOOTSTRAP
                    } else {
                        0
                    },
                    hardware_id: cpu.hardware_id,
                    firmware_id: cpu.firmware_id,
                    numa_node: cpu.numa_node,
                    package_id: hierarchy.package_id,
                    core_id: hierarchy.core_id,
                    thread_id: hierarchy.thread_id,
                    ..KernelCpuTopologyEntryV1::default()
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirmwareHandoff {
    pub architecture: Option<FirmwareArchitecture>,
    pub memory_map: Vec<KernelMemoryRegionV1>,
    pub framebuffer: Option<KernelFramebufferV1>,
    pub serial: Option<KernelSerialV1>,
    pub platform_entries: Vec<KernelPlatformEntryV1>,
    pub topology: FirmwareTopology,
    pub command_line: Option<String>,
    pub bootloader_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandoffImage {
    pub load_address: u64,
    pub boot_info_address: u64,
    pub memory_map_address: u64,
    pub framebuffer_address: Option<u64>,
    pub serial_address: Option<u64>,
    pub platform_table_address: u64,
    pub topology_address: Option<u64>,
    pub smp_platform_address: Option<u64>,
    pub command_line_address: Option<u64>,
    pub bytes: Vec<u8>,
}

impl KernelHandoffImage {
    pub fn end_address(&self) -> Result<u64, FirmwareError> {
        self.load_address
            .checked_add(self.bytes.len() as u64)
            .ok_or(FirmwareError::ArithmeticOverflow)
    }
}

impl FirmwareHandoff {
    pub fn normalize(&mut self) -> Result<(), FirmwareError> {
        self.memory_map.retain(|entry| entry.length != 0);
        self.memory_map.sort_by_key(|entry| entry.base);
        for entry in &self.memory_map {
            entry.end()?;
        }
        let mut merged: Vec<KernelMemoryRegionV1> = Vec::with_capacity(self.memory_map.len());
        for entry in self.memory_map.drain(..) {
            if let Some(previous) = merged.last_mut() {
                if previous.end()? == entry.base
                    && previous.kind == entry.kind
                    && previous.flags == entry.flags
                    && previous.source == entry.source
                {
                    previous.length = previous
                        .length
                        .checked_add(entry.length)
                        .ok_or(FirmwareError::ArithmeticOverflow)?;
                    continue;
                }
            }
            merged.push(entry);
        }
        self.memory_map = merged;
        self.platform_entries
            .sort_by_key(|entry| (entry.kind, entry.ptr));
        self.topology.normalize(None)?;
        if let Some(framebuffer) = self.framebuffer {
            framebuffer.validate()?;
        }
        Ok(())
    }

    pub fn materialize(
        &self,
        load_address: u64,
        kernel_payload_ptr: u64,
        kernel_payload_size: u64,
    ) -> Result<KernelHandoffImage, FirmwareError> {
        if load_address & 7 != 0 {
            return Err(FirmwareError::InvalidAlignment("kernel handoff image"));
        }
        let mut handoff = self.clone();
        handoff.normalize()?;
        let mut bytes = vec![0_u8; KernelBootInfoV1::BYTE_SIZE];
        align_bytes(&mut bytes, 8)?;

        let memory_map_offset = bytes.len();
        for entry in &handoff.memory_map {
            push_u64_le(&mut bytes, entry.base);
            push_u64_le(&mut bytes, entry.length);
            push_u32_le(&mut bytes, entry.kind);
            push_u32_le(&mut bytes, entry.flags);
            push_u32_le(&mut bytes, entry.source);
            push_u32_le(&mut bytes, entry.reserved);
        }

        align_bytes(&mut bytes, 8)?;
        let framebuffer_offset = handoff.framebuffer.map(|framebuffer| {
            let offset = bytes.len();
            push_u64_le(&mut bytes, framebuffer.base);
            push_u32_le(&mut bytes, framebuffer.width);
            push_u32_le(&mut bytes, framebuffer.height);
            push_u32_le(&mut bytes, framebuffer.pitch);
            push_u32_le(&mut bytes, framebuffer.format);
            push_u64_le(&mut bytes, framebuffer.flags);
            offset
        });

        align_bytes(&mut bytes, 8)?;
        let serial_offset = handoff.serial.map(|serial| {
            let offset = bytes.len();
            push_u64_le(&mut bytes, serial.magic);
            push_u32_le(&mut bytes, serial.version);
            push_u32_le(&mut bytes, serial.kind);
            push_u64_le(&mut bytes, serial.base);
            push_u32_le(&mut bytes, serial.irq);
            push_u32_le(&mut bytes, serial.baud);
            push_u32_le(&mut bytes, serial.flags);
            push_u32_le(&mut bytes, serial.register_shift);
            offset
        });

        let topology_entries = handoff.topology.entries();
        let topology_present = !topology_entries.is_empty();
        let smp_platform = handoff
            .topology
            .smp_platform
            .map(KernelSmpPlatformV1::validate)
            .transpose()
            .map_err(|_| FirmwareError::InvalidProperty("SMP platform descriptor".into()))?;
        let mut platform_entries = handoff
            .platform_entries
            .into_iter()
            .filter(|entry| {
                entry.kind != PLATFORM_KIND_NCC_BOOT
                    && entry.kind != PLATFORM_KIND_CPU_TOPOLOGY
                    && entry.kind != PLATFORM_KIND_SMP_PLATFORM
            })
            .collect::<Vec<_>>();
        platform_entries.push(KernelPlatformEntryV1 {
            kind: PLATFORM_KIND_NCC_BOOT,
            flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_REQUIRED | PLATFORM_FLAG_OWN_CONTRACT,
            ptr: load_address,
            length: KernelBootInfoV1::BYTE_SIZE as u64,
            checksum_or_revision: 1,
        });
        if topology_present {
            platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_CPU_TOPOLOGY,
                flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_REQUIRED | PLATFORM_FLAG_OWN_CONTRACT,
                ..KernelPlatformEntryV1::default()
            });
        }
        if smp_platform.is_some() {
            platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_SMP_PLATFORM,
                flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_REQUIRED | PLATFORM_FLAG_OWN_CONTRACT,
                ..KernelPlatformEntryV1::default()
            });
        }
        platform_entries.sort_by_key(|entry| (entry.kind, entry.ptr));

        align_bytes(&mut bytes, 8)?;
        let platform_offset = bytes.len();
        bytes.extend_from_slice(b"NCCPLT1\0");
        push_u32_le(&mut bytes, 1);
        push_u32_le(
            &mut bytes,
            u32::try_from(platform_entries.len())
                .map_err(|_| FirmwareError::InvalidLength("platform table"))?,
        );
        let platform_entries_offset = bytes.len();
        bytes.resize(
            bytes
                .len()
                .checked_add(
                    platform_entries
                        .len()
                        .checked_mul(KernelPlatformEntryV1::BYTE_SIZE)
                        .ok_or(FirmwareError::ArithmeticOverflow)?,
                )
                .ok_or(FirmwareError::ArithmeticOverflow)?,
            0,
        );

        align_bytes(&mut bytes, 8)?;
        let topology_offset = if topology_present {
            let offset = bytes.len();
            bytes.extend_from_slice(b"NCCCPU1\0");
            push_u32_le(&mut bytes, 1);
            push_u32_le(
                &mut bytes,
                u32::try_from(topology_entries.len())
                    .map_err(|_| FirmwareError::InvalidLength("CPU topology"))?,
            );
            push_u64_le(&mut bytes, 0);
            let bootstrap_id = topology_entries
                .iter()
                .find(|entry| entry.flags & KernelCpuTopologyEntryV1::FLAG_BOOTSTRAP != 0)
                .map(|entry| entry.hardware_id)
                .ok_or(FirmwareError::MissingRequired("bootstrap CPU"))?;
            push_u64_le(&mut bytes, bootstrap_id);
            for entry in &topology_entries {
                push_u32_le(&mut bytes, entry.logical_id);
                push_u32_le(&mut bytes, entry.flags);
                push_u64_le(&mut bytes, entry.hardware_id);
                push_u64_le(&mut bytes, entry.firmware_id);
                push_u32_le(&mut bytes, entry.numa_node);
                push_u16_le(&mut bytes, entry.package_id);
                push_u16_le(&mut bytes, entry.core_id);
                push_u16_le(&mut bytes, entry.thread_id);
                push_u16_le(&mut bytes, entry.reserved);
                push_u64_le(&mut bytes, entry.stack_top);
            }
            Some(offset)
        } else {
            None
        };

        align_bytes(&mut bytes, 8)?;
        let smp_platform_offset = smp_platform.map(|platform| {
            let offset = bytes.len();
            push_u64_le(&mut bytes, platform.magic);
            push_u32_le(&mut bytes, platform.version);
            push_u32_le(&mut bytes, platform.size);
            push_u32_le(&mut bytes, platform.architecture);
            push_u32_le(&mut bytes, platform.flags);
            push_u32_le(&mut bytes, platform.start_method);
            push_u32_le(&mut bytes, platform.interrupt_model);
            push_u64_le(&mut bytes, platform.primary_controller_base);
            push_u64_le(&mut bytes, platform.secondary_controller_base);
            push_u64_le(&mut bytes, platform.tertiary_controller_base);
            push_u64_le(&mut bytes, platform.timer_frequency_hz);
            push_u64_le(&mut bytes, platform.redistributor_stride);
            push_u64_le(&mut bytes, platform.context_stride);
            push_u32_le(&mut bytes, platform.source_count);
            push_u32_le(&mut bytes, platform.context_count);
            push_u64_le(&mut bytes, platform.feature_flags);
            push_u64_le(&mut bytes, platform.reserved);
            offset
        });

        let command_line_offset = handoff.command_line.as_ref().map(|command_line| {
            let offset = bytes.len();
            bytes.extend_from_slice(command_line.as_bytes());
            bytes.push(0);
            offset
        });
        align_bytes(&mut bytes, 8)?;

        let address = |offset: usize| -> Result<u64, FirmwareError> {
            load_address
                .checked_add(offset as u64)
                .ok_or(FirmwareError::ArithmeticOverflow)
        };
        let topology_address = topology_offset.map(address).transpose()?;
        let smp_platform_address = smp_platform_offset.map(address).transpose()?;
        if let Some(topology_address) = topology_address {
            let entry = platform_entries
                .iter_mut()
                .find(|entry| entry.kind == PLATFORM_KIND_CPU_TOPOLOGY)
                .ok_or(FirmwareError::MissingRequired(
                    "CPU topology platform entry",
                ))?;
            entry.ptr = topology_address;
            entry.length = (KernelCpuTopologyHeaderV1::BYTE_SIZE
                + topology_entries.len() * KernelCpuTopologyEntryV1::BYTE_SIZE)
                as u64;
            entry.checksum_or_revision = 1;
        }
        if let Some(smp_platform_address) = smp_platform_address {
            let entry = platform_entries
                .iter_mut()
                .find(|entry| entry.kind == PLATFORM_KIND_SMP_PLATFORM)
                .ok_or(FirmwareError::MissingRequired("SMP platform entry"))?;
            entry.ptr = smp_platform_address;
            entry.length = KernelSmpPlatformV1::BYTE_SIZE as u64;
            entry.checksum_or_revision = 1;
        }
        for (index, entry) in platform_entries.iter().enumerate() {
            let offset = platform_entries_offset + index * KernelPlatformEntryV1::BYTE_SIZE;
            patch_u32_le(&mut bytes, offset, entry.kind)?;
            patch_u32_le(&mut bytes, offset + 4, entry.flags)?;
            patch_u64_le(&mut bytes, offset + 8, entry.ptr)?;
            patch_u64_le(&mut bytes, offset + 16, entry.length)?;
            patch_u64_le(&mut bytes, offset + 24, entry.checksum_or_revision)?;
        }

        let memory_map_address = address(memory_map_offset)?;
        let framebuffer_address = framebuffer_offset.map(address).transpose()?;
        let serial_address = serial_offset.map(address).transpose()?;
        let platform_table_address = address(platform_offset)?;
        let command_line_address = command_line_offset.map(address).transpose()?;
        let flags = (if handoff.memory_map.is_empty() {
            0
        } else {
            BOOT_FLAG_MEMORY_MAP
        }) | (if serial_address.is_some() {
            BOOT_FLAG_SERIAL
        } else {
            0
        }) | BOOT_FLAG_PLATFORM_TABLE
            | (if framebuffer_address.is_some() {
                BOOT_FLAG_FRAMEBUFFER
            } else {
                0
            })
            | (if topology_address.is_some() {
                BOOT_FLAG_CPU_TOPOLOGY
            } else {
                0
            });
        patch_u64_le(&mut bytes, 0, KERNEL_BOOT_INFO_MAGIC)?;
        patch_u64_le(&mut bytes, 8, 1)?;
        patch_u64_le(&mut bytes, 16, flags)?;
        patch_u64_le(
            &mut bytes,
            24,
            if handoff.memory_map.is_empty() {
                0
            } else {
                memory_map_address
            },
        )?;
        patch_u64_le(&mut bytes, 32, handoff.memory_map.len() as u64)?;
        patch_u64_le(&mut bytes, 40, framebuffer_address.unwrap_or(0))?;
        patch_u64_le(&mut bytes, 48, serial_address.unwrap_or(0))?;
        patch_u64_le(&mut bytes, 56, platform_table_address)?;
        patch_u64_le(&mut bytes, 64, command_line_address.unwrap_or(0))?;
        patch_u64_le(&mut bytes, 72, kernel_payload_ptr)?;
        patch_u64_le(&mut bytes, 80, kernel_payload_size)?;

        Ok(KernelHandoffImage {
            load_address,
            boot_info_address: load_address,
            memory_map_address,
            framebuffer_address,
            serial_address,
            platform_table_address,
            topology_address,
            smp_platform_address,
            command_line_address,
            bytes,
        })
    }
}

impl KernelCpuTopologyHeaderV1 {
    pub const BYTE_SIZE: usize = 32;
}

fn align_bytes(bytes: &mut Vec<u8>, alignment: usize) -> Result<(), FirmwareError> {
    let aligned = bytes
        .len()
        .checked_add(alignment - 1)
        .ok_or(FirmwareError::ArithmeticOverflow)?
        & !(alignment - 1);
    bytes.resize(aligned, 0);
    Ok(())
}

fn push_u16_le(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64_le(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn patch_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), FirmwareError> {
    let destination = bytes
        .get_mut(offset..offset + 4)
        .ok_or(FirmwareError::Truncated("kernel handoff image"))?;
    destination.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn patch_u64_le(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), FirmwareError> {
    let destination = bytes
        .get_mut(offset..offset + 8)
        .ok_or(FirmwareError::Truncated("kernel handoff image"))?;
    destination.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UefiGuid(pub [u8; 16]);

impl UefiGuid {
    pub const ACPI_20: Self = Self([
        0x71, 0xe8, 0x68, 0x88, 0xf1, 0xe4, 0xd3, 0x11, 0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88,
        0x81,
    ]);
    pub const ACPI_10: Self = Self([
        0x30, 0x2d, 0x9d, 0xeb, 0x88, 0x2d, 0xd3, 0x11, 0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1,
        0x4d,
    ]);
    pub const SMBIOS: Self = Self([
        0x31, 0x31, 0x4d, 0xeb, 0x2d, 0x1a, 0xd3, 0x11, 0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1,
        0x4d,
    ]);
    pub const SMBIOS3: Self = Self([
        0x44, 0x15, 0x85, 0xf2, 0x9e, 0x58, 0x0d, 0x4a, 0xa7, 0x0b, 0xa2, 0x6b, 0x7c, 0xaf, 0xe5,
        0xe8,
    ]);
    pub const FDT: Self = Self([
        0xd5, 0x21, 0xb6, 0xb1, 0x9c, 0xf1, 0xa5, 0x41, 0x83, 0x0b, 0xd9, 0x15, 0x2c, 0x69, 0xaa,
        0xe0,
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UefiConfigurationTable {
    pub guid: UefiGuid,
    pub address: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UefiMemoryDescriptor {
    pub memory_type: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub page_count: u64,
    pub attributes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UefiGraphicsOutput {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub pixels_per_scanline: u32,
    pub pixel_format: u32,
}

fn uefi_memory_kind(memory_type: u32) -> u32 {
    match memory_type {
        1..=4 => MEMORY_KIND_LOADER,
        5 | 6 => MEMORY_KIND_RUNTIME,
        7 => MEMORY_KIND_USABLE,
        8 => MEMORY_KIND_BAD,
        9 => MEMORY_KIND_ACPI_RECLAIM,
        10 => MEMORY_KIND_ACPI_NVS,
        11 | 12 => MEMORY_KIND_MMIO,
        14 => MEMORY_KIND_PERSISTENT,
        _ => MEMORY_KIND_RESERVED,
    }
}

pub fn adapt_uefi_handoff(
    architecture: FirmwareArchitecture,
    system_table: u64,
    tables: &[UefiConfigurationTable],
    descriptors: &[UefiMemoryDescriptor],
    graphics: Option<UefiGraphicsOutput>,
) -> Result<FirmwareHandoff, FirmwareError> {
    if system_table == 0 {
        return Err(FirmwareError::MissingRequired("UEFI system table"));
    }
    let mut handoff = FirmwareHandoff {
        architecture: Some(architecture),
        ..FirmwareHandoff::default()
    };
    handoff.platform_entries.push(KernelPlatformEntryV1 {
        kind: PLATFORM_KIND_UEFI_SYSTEM_TABLE,
        flags: PLATFORM_FLAG_PRESENT
            | PLATFORM_FLAG_REQUIRED
            | PLATFORM_FLAG_EXTERNAL
            | PLATFORM_FLAG_DYNAMIC_LENGTH,
        ptr: system_table,
        length: 24,
        checksum_or_revision: 2,
        ..KernelPlatformEntryV1::default()
    });
    let mut acpi = None;
    for table in tables {
        if table.address == 0 {
            continue;
        }
        let (kind, prefer) = if table.guid == UefiGuid::ACPI_20 {
            (Some(PLATFORM_KIND_ACPI_RSDP), true)
        } else if table.guid == UefiGuid::ACPI_10 {
            (Some(PLATFORM_KIND_ACPI_RSDP), false)
        } else if table.guid == UefiGuid::SMBIOS3 {
            (Some(PLATFORM_KIND_SMBIOS64), true)
        } else if table.guid == UefiGuid::SMBIOS {
            (Some(PLATFORM_KIND_SMBIOS32), false)
        } else if table.guid == UefiGuid::FDT {
            (Some(PLATFORM_KIND_FDT), true)
        } else {
            (None, false)
        };
        let Some(kind) = kind else { continue };
        if kind == PLATFORM_KIND_ACPI_RSDP {
            if prefer || acpi.is_none() {
                acpi = Some(KernelPlatformEntryV1 {
                    kind,
                    flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                    ptr: table.address,
                    checksum_or_revision: if prefer { 2 } else { 1 },
                    ..KernelPlatformEntryV1::default()
                });
            }
        } else {
            handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind,
                flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                ptr: table.address,
                checksum_or_revision: if kind == PLATFORM_KIND_FDT {
                    17
                } else if prefer {
                    3
                } else {
                    2
                },
                ..KernelPlatformEntryV1::default()
            });
        }
    }
    if let Some(acpi) = acpi {
        handoff.platform_entries.push(acpi);
    }
    for descriptor in descriptors {
        let length = descriptor
            .page_count
            .checked_mul(UEFI_PAGE_SIZE)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        let kind = uefi_memory_kind(descriptor.memory_type);
        handoff.memory_map.push(KernelMemoryRegionV1 {
            base: descriptor.physical_start,
            length,
            kind,
            flags: (descriptor.attributes & u64::from(u32::MAX)) as u32,
            source: MEMORY_SOURCE_UEFI,
            reserved: 0,
        });
    }
    if let Some(graphics) = graphics {
        if graphics.size == 0
            || graphics.pixels_per_scanline < graphics.width
            || graphics.pixel_format > FRAMEBUFFER_FORMAT_BIT_MASK
        {
            return Err(FirmwareError::InvalidRange("UEFI GOP framebuffer"));
        }
        let pitch = graphics
            .pixels_per_scanline
            .checked_mul(4)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        let required = u64::from(pitch)
            .checked_mul(u64::from(graphics.height))
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        if required > graphics.size {
            return Err(FirmwareError::InvalidLength("UEFI GOP framebuffer"));
        }
        handoff.framebuffer = Some(KernelFramebufferV1 {
            base: graphics.base,
            width: graphics.width,
            height: graphics.height,
            pitch,
            format: graphics.pixel_format,
            flags: FRAMEBUFFER_FLAG_LINEAR | FRAMEBUFFER_FLAG_FIRMWARE_OWNED,
        });
    }
    handoff.normalize()?;
    Ok(handoff)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Multiboot2Module {
    pub start: u32,
    pub end: u32,
}

fn multiboot2_physical_address(physical_base: u64, offset: usize) -> Result<u64, FirmwareError> {
    physical_base
        .checked_add(u64::try_from(offset).map_err(|_| FirmwareError::ArithmeticOverflow)?)
        .ok_or(FirmwareError::ArithmeticOverflow)
}

pub fn parse_multiboot2_information(
    bytes: &[u8],
    physical_base: u64,
) -> Result<FirmwareHandoff, FirmwareError> {
    if bytes.len() < 16 {
        return Err(FirmwareError::Truncated("Multiboot2 information"));
    }
    let total_size = read_le_u32(bytes, 0)? as usize;
    if total_size < 16 || total_size > bytes.len() || total_size & 7 != 0 {
        return Err(FirmwareError::InvalidLength("Multiboot2 information"));
    }
    let mut handoff = FirmwareHandoff {
        architecture: Some(FirmwareArchitecture::X86),
        ..FirmwareHandoff::default()
    };
    handoff.platform_entries.push(KernelPlatformEntryV1 {
        kind: PLATFORM_KIND_MULTIBOOT2_TAGS,
        flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_REQUIRED | PLATFORM_FLAG_EXTERNAL,
        ptr: physical_base,
        length: total_size as u64,
        ..KernelPlatformEntryV1::default()
    });
    let mut detailed_memory = Vec::new();
    let mut efi_memory = Vec::new();
    let mut basic_memory = None;
    let mut offset = 8_usize;
    let mut saw_end = false;
    while offset < total_size {
        let tag_type = read_le_u32(bytes, offset)?;
        let tag_size = read_le_u32(bytes, offset + 4)? as usize;
        if tag_size < 8
            || offset
                .checked_add(tag_size)
                .is_none_or(|end| end > total_size)
        {
            return Err(FirmwareError::InvalidTag(tag_type));
        }
        let payload = &bytes[offset + 8..offset + tag_size];
        match tag_type {
            0 => {
                if tag_size != 8 {
                    return Err(FirmwareError::InvalidTag(tag_type));
                }
                saw_end = true;
                break;
            }
            1 => handoff.command_line = Some(c_string(payload)?),
            2 => {
                handoff.bootloader_name = Some(c_string(payload)?);
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_MULTIBOOT2_BOOT_LOADER_NAME,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                    length: payload.len() as u64,
                    checksum_or_revision: 2,
                });
            }
            3 => {
                if payload.len() < 8 {
                    return Err(FirmwareError::Truncated("Multiboot2 module"));
                }
                let start = read_le_u32(payload, 0)? as u64;
                let end = read_le_u32(payload, 4)? as u64;
                if end < start {
                    return Err(FirmwareError::InvalidRange("Multiboot2 module"));
                }
                handoff.memory_map.push(KernelMemoryRegionV1 {
                    base: start,
                    length: end - start,
                    kind: MEMORY_KIND_KERNEL_IMAGE,
                    source: MEMORY_SOURCE_MULTIBOOT2,
                    ..KernelMemoryRegionV1::default()
                });
            }
            4 => {
                if payload.len() < 8 {
                    return Err(FirmwareError::Truncated("Multiboot2 basic memory"));
                }
                basic_memory = Some((read_le_u32(payload, 0)?, read_le_u32(payload, 4)?));
            }
            5 => {
                if payload.len() < 12 {
                    return Err(FirmwareError::Truncated("Multiboot2 boot device"));
                }
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_MULTIBOOT2_BOOT_DEVICE,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                    length: 12,
                    checksum_or_revision: 1,
                });
            }
            6 => parse_multiboot2_memory_map(payload, &mut detailed_memory)?,
            7 => handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_MULTIBOOT2_VBE,
                flags: PLATFORM_FLAG_PRESENT
                    | PLATFORM_FLAG_EXTERNAL
                    | PLATFORM_FLAG_DYNAMIC_LENGTH,
                ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                length: payload.len() as u64,
                checksum_or_revision: 2,
            }),
            8 => {
                handoff.framebuffer = Some(parse_multiboot2_framebuffer(payload)?);
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_MULTIBOOT2_FRAMEBUFFER,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                    length: payload.len() as u64,
                    checksum_or_revision: 2,
                });
            }
            9 => handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_MULTIBOOT2_ELF_SECTIONS,
                flags: PLATFORM_FLAG_PRESENT
                    | PLATFORM_FLAG_EXTERNAL
                    | PLATFORM_FLAG_DYNAMIC_LENGTH,
                ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                length: payload.len() as u64,
                checksum_or_revision: 2,
            }),
            10 => handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_MULTIBOOT2_APM,
                flags: PLATFORM_FLAG_PRESENT
                    | PLATFORM_FLAG_EXTERNAL
                    | PLATFORM_FLAG_DYNAMIC_LENGTH,
                ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                length: payload.len() as u64,
                checksum_or_revision: 2,
            }),
            11 => {
                if payload.len() < 4 {
                    return Err(FirmwareError::Truncated("Multiboot2 EFI32 tag"));
                }
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_UEFI_SYSTEM_TABLE,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: read_le_u32(payload, 0)? as u64,
                    length: 24,
                    checksum_or_revision: 1,
                });
            }
            12 => {
                if payload.len() < 8 {
                    return Err(FirmwareError::Truncated("Multiboot2 EFI64 tag"));
                }
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_UEFI_SYSTEM_TABLE,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: read_le_u64(payload, 0)?,
                    length: 24,
                    checksum_or_revision: 2,
                });
            }
            13 => {
                if payload.len() < 8 {
                    return Err(FirmwareError::Truncated("Multiboot2 SMBIOS tag"));
                }
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_SMBIOS_TABLES,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: multiboot2_physical_address(physical_base, offset + 16)?,
                    length: payload.len().saturating_sub(8) as u64,
                    checksum_or_revision: (u64::from(payload[0]) << 32) | u64::from(payload[1]),
                });
            }
            14 | 15 => {
                let acpi = parse_acpi_rsdp(payload)?;
                handoff
                    .platform_entries
                    .retain(|entry| entry.kind != PLATFORM_KIND_ACPI_RSDP || tag_type != 15);
                if tag_type == 14
                    && handoff
                        .platform_entries
                        .iter()
                        .any(|entry| entry.kind == PLATFORM_KIND_ACPI_RSDP)
                {
                    // The newer ACPI tag has precedence.
                } else {
                    handoff.platform_entries.push(KernelPlatformEntryV1 {
                        kind: PLATFORM_KIND_ACPI_RSDP,
                        flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                        ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                        length: u64::from(acpi.length),
                        checksum_or_revision: u64::from(acpi.revision),
                    });
                }
            }
            16 => handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_MULTIBOOT2_NETWORK,
                flags: PLATFORM_FLAG_PRESENT
                    | PLATFORM_FLAG_EXTERNAL
                    | PLATFORM_FLAG_DYNAMIC_LENGTH,
                ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                length: payload.len() as u64,
                checksum_or_revision: 2,
            }),
            17 => {
                parse_multiboot2_efi_memory_map(payload, &mut efi_memory)?;
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_UEFI_MEMORY_MAP,
                    flags: PLATFORM_FLAG_PRESENT
                        | PLATFORM_FLAG_EXTERNAL
                        | PLATFORM_FLAG_DYNAMIC_LENGTH,
                    ptr: multiboot2_physical_address(physical_base, offset + 8)?,
                    length: payload.len() as u64,
                    checksum_or_revision: u64::from(read_le_u32(payload, 4)?),
                });
            }
            18 => handoff.platform_entries.push(KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_UEFI_BOOT_SERVICES,
                flags: PLATFORM_FLAG_PRESENT
                    | PLATFORM_FLAG_EXTERNAL
                    | PLATFORM_FLAG_DYNAMIC_LENGTH,
                ptr: multiboot2_physical_address(physical_base, offset)?,
                length: tag_size as u64,
                checksum_or_revision: 1,
            }),
            19 | 20 => {
                let (pointer, revision) = if tag_type == 19 {
                    if payload.len() < 4 {
                        return Err(FirmwareError::Truncated("Multiboot2 EFI32 image handle"));
                    }
                    (read_le_u32(payload, 0)? as u64, 1)
                } else {
                    if payload.len() < 8 {
                        return Err(FirmwareError::Truncated("Multiboot2 EFI64 image handle"));
                    }
                    (read_le_u64(payload, 0)?, 2)
                };
                if tag_type == 20 {
                    handoff
                        .platform_entries
                        .retain(|entry| entry.kind != PLATFORM_KIND_UEFI_IMAGE_HANDLE);
                }
                if tag_type == 20
                    || !handoff
                        .platform_entries
                        .iter()
                        .any(|entry| entry.kind == PLATFORM_KIND_UEFI_IMAGE_HANDLE)
                {
                    handoff.platform_entries.push(KernelPlatformEntryV1 {
                        kind: PLATFORM_KIND_UEFI_IMAGE_HANDLE,
                        flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                        ptr: pointer,
                        checksum_or_revision: revision,
                        ..KernelPlatformEntryV1::default()
                    });
                }
            }
            21 => {
                if payload.len() < 4 {
                    return Err(FirmwareError::Truncated("Multiboot2 load base"));
                }
                handoff.platform_entries.push(KernelPlatformEntryV1 {
                    kind: PLATFORM_KIND_MULTIBOOT2_LOAD_BASE,
                    flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                    ptr: read_le_u32(payload, 0)? as u64,
                    checksum_or_revision: 1,
                    ..KernelPlatformEntryV1::default()
                });
            }
            _ => {}
        }
        offset = align_up(offset + tag_size, 8)?;
    }
    if !saw_end {
        return Err(FirmwareError::MissingRequired("Multiboot2 end tag"));
    }
    if !detailed_memory.is_empty() {
        handoff.memory_map.extend(detailed_memory);
    } else if !efi_memory.is_empty() {
        handoff.memory_map.extend(efi_memory);
    } else if let Some((lower_kib, upper_kib)) = basic_memory {
        if lower_kib != 0 {
            handoff.memory_map.push(KernelMemoryRegionV1 {
                base: 0,
                length: u64::from(lower_kib) * 1024,
                kind: MEMORY_KIND_USABLE,
                flags: 1,
                source: MEMORY_SOURCE_MULTIBOOT2,
                reserved: 0,
            });
        }
        if upper_kib != 0 {
            handoff.memory_map.push(KernelMemoryRegionV1 {
                base: 0x10_0000,
                length: u64::from(upper_kib) * 1024,
                kind: MEMORY_KIND_USABLE,
                flags: 1,
                source: MEMORY_SOURCE_MULTIBOOT2,
                reserved: 0,
            });
        }
    }
    handoff.normalize()?;
    Ok(handoff)
}

fn parse_multiboot2_efi_memory_map(
    payload: &[u8],
    output: &mut Vec<KernelMemoryRegionV1>,
) -> Result<(), FirmwareError> {
    if payload.len() < 8 {
        return Err(FirmwareError::Truncated("Multiboot2 EFI memory map"));
    }
    let descriptor_size = read_le_u32(payload, 0)? as usize;
    if descriptor_size < 40 || (payload.len() - 8) % descriptor_size != 0 {
        return Err(FirmwareError::InvalidLength("Multiboot2 EFI memory map"));
    }
    for descriptor in payload[8..].chunks_exact(descriptor_size) {
        let memory_type = read_le_u32(descriptor, 0)?;
        let page_count = read_le_u64(descriptor, 24)?;
        let length = page_count
            .checked_mul(UEFI_PAGE_SIZE)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        let base = read_le_u64(descriptor, 8)?;
        base.checked_add(length)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        output.push(KernelMemoryRegionV1 {
            base,
            length,
            kind: uefi_memory_kind(memory_type),
            flags: (read_le_u64(descriptor, 32)? & u64::from(u32::MAX)) as u32,
            source: MEMORY_SOURCE_UEFI,
            reserved: 0,
        });
    }
    Ok(())
}

fn parse_multiboot2_memory_map(
    payload: &[u8],
    output: &mut Vec<KernelMemoryRegionV1>,
) -> Result<(), FirmwareError> {
    if payload.len() < 8 {
        return Err(FirmwareError::Truncated("Multiboot2 memory map"));
    }
    let entry_size = read_le_u32(payload, 0)? as usize;
    if entry_size < 24 || (payload.len() - 8) % entry_size != 0 {
        return Err(FirmwareError::InvalidLength("Multiboot2 memory map"));
    }
    for entry in payload[8..].chunks_exact(entry_size) {
        let raw_kind = read_le_u32(entry, 16)?;
        output.push(KernelMemoryRegionV1 {
            base: read_le_u64(entry, 0)?,
            length: read_le_u64(entry, 8)?,
            kind: match raw_kind {
                1 => MEMORY_KIND_USABLE,
                3 => MEMORY_KIND_ACPI_RECLAIM,
                4 => MEMORY_KIND_ACPI_NVS,
                5 => MEMORY_KIND_BAD,
                _ => MEMORY_KIND_RESERVED,
            },
            flags: raw_kind,
            source: MEMORY_SOURCE_MULTIBOOT2,
            reserved: 0,
        });
    }
    Ok(())
}

fn parse_multiboot2_framebuffer(payload: &[u8]) -> Result<KernelFramebufferV1, FirmwareError> {
    if payload.len() < 28 {
        return Err(FirmwareError::Truncated("Multiboot2 framebuffer"));
    }
    let bpp = payload[20];
    let framebuffer_type = payload[21];
    if bpp != 32 || framebuffer_type != 1 {
        return Err(FirmwareError::InvalidProperty(format!(
            "Multiboot2 framebuffer bpp={bpp} type={framebuffer_type}"
        )));
    }
    let direct_color = &payload[22..28];
    let format = match direct_color {
        [0, 8, 8, 8, 16, 8] => FRAMEBUFFER_FORMAT_RGB_RESERVED_8,
        [16, 8, 8, 8, 0, 8] => FRAMEBUFFER_FORMAT_BGR_RESERVED_8,
        _ => FRAMEBUFFER_FORMAT_BIT_MASK,
    };
    KernelFramebufferV1 {
        base: read_le_u64(payload, 0)?,
        pitch: read_le_u32(payload, 8)?,
        width: read_le_u32(payload, 12)?,
        height: read_le_u32(payload, 16)?,
        format,
        flags: FRAMEBUFFER_FLAG_LINEAR | FRAMEBUFFER_FLAG_FIRMWARE_OWNED,
    }
    .validate()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiRsdp {
    pub revision: u8,
    pub rsdt_address: u32,
    pub xsdt_address: u64,
    pub length: u32,
}

pub fn parse_acpi_rsdp(bytes: &[u8]) -> Result<AcpiRsdp, FirmwareError> {
    if bytes.len() < 20 {
        return Err(FirmwareError::Truncated("ACPI RSDP"));
    }
    if &bytes[..8] != b"RSD PTR " {
        return Err(FirmwareError::InvalidMagic("ACPI RSDP"));
    }
    if checksum(&bytes[..20]) != 0 {
        return Err(FirmwareError::InvalidChecksum("ACPI RSDP v1"));
    }
    let revision = bytes[15];
    let (length, xsdt_address) = if revision >= 2 {
        if bytes.len() < 36 {
            return Err(FirmwareError::Truncated("ACPI RSDP v2"));
        }
        let length = read_le_u32(bytes, 20)?;
        if length < 36 || length as usize > bytes.len() {
            return Err(FirmwareError::InvalidLength("ACPI RSDP v2"));
        }
        if checksum(&bytes[..length as usize]) != 0 {
            return Err(FirmwareError::InvalidChecksum("ACPI RSDP v2"));
        }
        (length, read_le_u64(bytes, 24)?)
    } else {
        (20, 0)
    };
    Ok(AcpiRsdp {
        revision,
        rsdt_address: read_le_u32(bytes, 16)?,
        xsdt_address,
        length,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpiSdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiIoApic {
    pub id: u8,
    pub address: u64,
    pub global_system_interrupt_base: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiGicDistributor {
    pub id: u32,
    pub address: u64,
    pub version: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiGicRedistributorRange {
    pub address: u64,
    pub length: u32,
    pub non_coherent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiGicIts {
    pub id: u32,
    pub address: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpiMadtInfo {
    pub topology: FirmwareTopology,
    pub flags: u32,
    pub local_apic_address: u64,
    pub io_apics: Vec<AcpiIoApic>,
    pub gic_distributor: Option<AcpiGicDistributor>,
    pub gic_redistributors: Vec<AcpiGicRedistributorRange>,
    pub gic_its: Vec<AcpiGicIts>,
}

pub fn parse_acpi_sdt(bytes: &[u8]) -> Result<AcpiSdtHeader, FirmwareError> {
    if bytes.len() < 36 {
        return Err(FirmwareError::Truncated("ACPI SDT"));
    }
    let length = read_le_u32(bytes, 4)?;
    if length < 36 || length as usize > bytes.len() {
        return Err(FirmwareError::InvalidLength("ACPI SDT"));
    }
    if checksum(&bytes[..length as usize]) != 0 {
        return Err(FirmwareError::InvalidChecksum("ACPI SDT"));
    }
    Ok(AcpiSdtHeader {
        signature: bytes[0..4].try_into().expect("four-byte signature"),
        length,
        revision: bytes[8],
        oem_id: bytes[10..16].try_into().expect("six-byte OEM id"),
        oem_table_id: bytes[16..24].try_into().expect("eight-byte OEM table id"),
    })
}

pub fn parse_acpi_madt_complete(
    bytes: &[u8],
    bootstrap_id: u64,
) -> Result<AcpiMadtInfo, FirmwareError> {
    let header = parse_acpi_sdt(bytes)?;
    if &header.signature != b"APIC" || header.length < 44 {
        return Err(FirmwareError::InvalidMagic("ACPI MADT"));
    }
    let mut local_apic_address = read_le_u32(bytes, 36)? as u64;
    let mut topology = FirmwareTopology {
        interrupt_controller: Some("ACPI MADT".into()),
        interrupt_controller_base: Some(local_apic_address),
        ..FirmwareTopology::default()
    };
    let flags = read_le_u32(bytes, 40)?;
    let mut io_apics = Vec::new();
    let mut gic_distributor = None;
    let mut gic_redistributors = Vec::new();
    let mut gic_its = Vec::new();
    let mut gicc_redistributor_base = None;
    let mut saw_x2apic = false;
    let mut offset = 44_usize;
    while offset < header.length as usize {
        if offset + 2 > header.length as usize {
            return Err(FirmwareError::Truncated("ACPI MADT entry"));
        }
        let kind = bytes[offset];
        let length = bytes[offset + 1] as usize;
        if length < 2 || offset + length > header.length as usize {
            return Err(FirmwareError::InvalidLength("ACPI MADT entry"));
        }
        let entry = &bytes[offset..offset + length];
        match kind {
            0 if length >= 8 => {
                let flags = read_le_u32(entry, 4)?;
                let hardware_id = entry[3] as u64;
                topology.cpus.push(FirmwareCpu {
                    hardware_id,
                    firmware_id: entry[2] as u64,
                    enabled: flags & 3 != 0,
                    bootstrap: hardware_id == bootstrap_id,
                    numa_node: 0,
                });
                topology.hierarchy.insert(
                    hardware_id,
                    FirmwareCpuHierarchy {
                        core_id: hardware_id as u16,
                        ..FirmwareCpuHierarchy::default()
                    },
                );
            }
            1 if length >= 12 => {
                io_apics.push(AcpiIoApic {
                    id: entry[2],
                    address: read_le_u32(entry, 4)? as u64,
                    global_system_interrupt_base: read_le_u32(entry, 8)?,
                });
            }
            5 if length >= 12 => {
                local_apic_address = read_le_u64(entry, 4)?;
            }
            9 if length >= 16 => {
                saw_x2apic = true;
                let hardware_id = read_le_u32(entry, 4)? as u64;
                let flags = read_le_u32(entry, 8)?;
                topology.cpus.push(FirmwareCpu {
                    hardware_id,
                    firmware_id: read_le_u32(entry, 12)? as u64,
                    enabled: flags & 3 != 0,
                    bootstrap: hardware_id == bootstrap_id,
                    numa_node: 0,
                });
                topology.hierarchy.insert(
                    hardware_id,
                    FirmwareCpuHierarchy {
                        core_id: hardware_id as u16,
                        ..FirmwareCpuHierarchy::default()
                    },
                );
            }
            11 if length >= 76 => {
                let hardware_id = read_le_u64(entry, 68)?;
                let flags = read_le_u32(entry, 12)?;
                let redistributor = read_le_u64(entry, 60)?;
                if redistributor != 0 && gicc_redistributor_base.is_none() {
                    gicc_redistributor_base = Some(redistributor);
                }
                topology.cpus.push(FirmwareCpu {
                    hardware_id,
                    firmware_id: read_le_u32(entry, 8)? as u64,
                    enabled: flags & 9 != 0,
                    bootstrap: hardware_id == bootstrap_id,
                    numa_node: 0,
                });
                let affinity0 = (hardware_id & 0xff) as u16;
                let affinity1 = ((hardware_id >> 8) & 0xff) as u16;
                let affinity2 = ((hardware_id >> 16) & 0xff) as u16;
                let affinity3 = ((hardware_id >> 32) & 0xff) as u16;
                topology.hierarchy.insert(
                    hardware_id,
                    FirmwareCpuHierarchy {
                        package_id: (affinity3 << 8) | affinity2,
                        core_id: affinity1,
                        thread_id: affinity0,
                    },
                );
            }
            12 if length >= 24 => {
                topology.interrupt_controller = Some("GICv3".into());
                let distributor = AcpiGicDistributor {
                    id: read_le_u32(entry, 4)?,
                    address: read_le_u64(entry, 8)?,
                    version: entry[20],
                };
                topology.interrupt_controller_base = Some(distributor.address);
                gic_distributor = Some(distributor);
            }
            14 if length >= 16 => {
                gic_redistributors.push(AcpiGicRedistributorRange {
                    address: read_le_u64(entry, 4)?,
                    length: read_le_u32(entry, 12)?,
                    non_coherent: entry[2] & 1 != 0,
                });
            }
            15 if length >= 20 => {
                gic_its.push(AcpiGicIts {
                    id: read_le_u32(entry, 4)?,
                    address: read_le_u64(entry, 8)?,
                });
            }
            _ => {}
        }
        offset += length;
    }
    if gic_distributor.is_none() {
        topology.interrupt_controller_base = Some(local_apic_address);
    }
    topology.smp_platform = if let Some(distributor) = gic_distributor {
        let mut platform = KernelSmpPlatformV1::new(
            SmpArchitecture::Aarch64,
            SmpStartMethod::PsciSmc,
            SmpInterruptModel::GicV3,
        );
        platform.flags = KernelSmpPlatformV1::FLAG_DISCOVERED_FROM_ACPI;
        platform.primary_controller_base = distributor.address;
        platform.secondary_controller_base = gic_redistributors
            .first()
            .map(|range| range.address)
            .or(gicc_redistributor_base)
            .unwrap_or(0);
        platform.tertiary_controller_base = gic_its.first().map(|its| its.address).unwrap_or(0);
        platform.redistributor_stride = 0x20_000;
        platform.feature_flags = KernelSmpPlatformV1::FEATURE_IPI
            | KernelSmpPlatformV1::FEATURE_TLB_RANGE
            | KernelSmpPlatformV1::FEATURE_TLB_ASID
            | KernelSmpPlatformV1::FEATURE_PER_CPU_TIMER
            | if gic_its.is_empty() {
                0
            } else {
                KernelSmpPlatformV1::FEATURE_MSI
            };
        Some(platform)
    } else {
        let mut platform = KernelSmpPlatformV1::new(
            SmpArchitecture::X86,
            SmpStartMethod::X86InitSipi,
            if saw_x2apic {
                SmpInterruptModel::X2Apic
            } else {
                SmpInterruptModel::XApic
            },
        );
        platform.flags = KernelSmpPlatformV1::FLAG_DISCOVERED_FROM_ACPI;
        platform.primary_controller_base = local_apic_address;
        platform.secondary_controller_base = io_apics.first().map(|io| io.address).unwrap_or(0);
        platform.feature_flags = KernelSmpPlatformV1::FEATURE_IPI
            | KernelSmpPlatformV1::FEATURE_TLB_RANGE
            | KernelSmpPlatformV1::FEATURE_TLB_ASID
            | KernelSmpPlatformV1::FEATURE_PER_CPU_TIMER;
        Some(platform)
    };
    topology.normalize(Some(bootstrap_id))?;
    Ok(AcpiMadtInfo {
        topology,
        flags,
        local_apic_address,
        io_apics,
        gic_distributor,
        gic_redistributors,
        gic_its,
    })
}

pub fn parse_acpi_madt(bytes: &[u8], bootstrap_id: u64) -> Result<FirmwareTopology, FirmwareError> {
    Ok(parse_acpi_madt_complete(bytes, bootstrap_id)?.topology)
}

pub fn apply_acpi_srat(bytes: &[u8], topology: &mut FirmwareTopology) -> Result<(), FirmwareError> {
    let header = parse_acpi_sdt(bytes)?;
    if &header.signature != b"SRAT" || header.length < 48 {
        return Err(FirmwareError::InvalidMagic("ACPI SRAT"));
    }
    let mut offset = 48_usize;
    while offset < header.length as usize {
        if offset + 2 > header.length as usize {
            return Err(FirmwareError::Truncated("ACPI SRAT entry"));
        }
        let length = bytes[offset + 1] as usize;
        if length < 2 || offset + length > header.length as usize {
            return Err(FirmwareError::InvalidLength("ACPI SRAT entry"));
        }
        let entry = &bytes[offset..offset + length];
        let affinity = match entry[0] {
            0 if length >= 16 && read_le_u32(entry, 4)? & 1 != 0 => {
                let proximity = u32::from(entry[2])
                    | (u32::from(entry[9]) << 8)
                    | (u32::from(entry[10]) << 16)
                    | (u32::from(entry[11]) << 24);
                Some((true, u64::from(entry[3]), proximity))
            }
            2 if length >= 24 && read_le_u32(entry, 12)? & 1 != 0 => Some((
                true,
                u64::from(read_le_u32(entry, 8)?),
                read_le_u32(entry, 4)?,
            )),
            3 if length >= 18 && read_le_u32(entry, 10)? & 1 != 0 => Some((
                false,
                u64::from(read_le_u32(entry, 6)?),
                read_le_u32(entry, 2)?,
            )),
            _ => None,
        };
        if let Some((hardware_id, id, proximity)) = affinity {
            let cpu = if hardware_id {
                topology.cpus.iter_mut().find(|cpu| cpu.hardware_id == id)
            } else {
                topology.cpus.iter_mut().find(|cpu| cpu.firmware_id == id)
            };
            if let Some(cpu) = cpu {
                cpu.numa_node = proximity;
            }
        }
        offset += length;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpiTableSet {
    pub rsdp: AcpiRsdp,
    pub root_address: u64,
    pub root: AcpiSdtHeader,
    pub tables: BTreeMap<[u8; 4], Vec<u64>>,
    pub madt: AcpiMadtInfo,
}

pub fn parse_acpi_fadt_psci_method(bytes: &[u8]) -> Result<Option<SmpStartMethod>, FirmwareError> {
    let header = parse_acpi_sdt(bytes)?;
    if &header.signature != b"FACP" {
        return Err(FirmwareError::InvalidMagic("ACPI FADT"));
    }
    if header.length < 131 {
        return Ok(None);
    }
    let flags = read_le_u16(bytes, 129)?;
    if flags & 1 == 0 {
        Ok(None)
    } else if flags & 2 != 0 {
        Ok(Some(SmpStartMethod::PsciHvc))
    } else {
        Ok(Some(SmpStartMethod::PsciSmc))
    }
}

pub fn parse_acpi_table_set(
    rsdp_bytes: &[u8],
    physical_tables: &BTreeMap<u64, Vec<u8>>,
    bootstrap_id: u64,
) -> Result<AcpiTableSet, FirmwareError> {
    let rsdp = parse_acpi_rsdp(rsdp_bytes)?;
    let (root_address, entry_size, expected_signature) = if rsdp.xsdt_address != 0 {
        (rsdp.xsdt_address, 8_usize, *b"XSDT")
    } else {
        (u64::from(rsdp.rsdt_address), 4_usize, *b"RSDT")
    };
    let root_bytes = physical_tables
        .get(&root_address)
        .ok_or(FirmwareError::MissingRequired("ACPI root table"))?;
    let root = parse_acpi_sdt(root_bytes)?;
    if root.signature != expected_signature || (root.length as usize - 36) % entry_size != 0 {
        return Err(FirmwareError::InvalidMagic("ACPI root table"));
    }

    let mut tables = BTreeMap::<[u8; 4], Vec<u64>>::new();
    let mut madt = None;
    let mut srat = None;
    let mut psci_method = None;
    for offset in (36..root.length as usize).step_by(entry_size) {
        let address = if entry_size == 8 {
            read_le_u64(root_bytes, offset)?
        } else {
            u64::from(read_le_u32(root_bytes, offset)?)
        };
        if address == 0 {
            continue;
        }
        let table_bytes = physical_tables
            .get(&address)
            .ok_or(FirmwareError::MissingRequired("ACPI child table"))?;
        let header = parse_acpi_sdt(table_bytes)?;
        tables.entry(header.signature).or_default().push(address);
        match &header.signature {
            b"APIC" if madt.is_none() => {
                madt = Some(parse_acpi_madt_complete(table_bytes, bootstrap_id)?);
            }
            b"SRAT" if srat.is_none() => srat = Some(table_bytes.as_slice()),
            b"FACP" if psci_method.is_none() => {
                psci_method = parse_acpi_fadt_psci_method(table_bytes)?;
            }
            _ => {}
        }
    }
    let mut madt = madt.ok_or(FirmwareError::MissingRequired("ACPI MADT"))?;
    if let Some(srat) = srat {
        apply_acpi_srat(srat, &mut madt.topology)?;
    }
    if let (Some(method), Some(platform)) = (psci_method, madt.topology.smp_platform.as_mut()) {
        if platform.architecture == SmpArchitecture::Aarch64 as u32 {
            platform.start_method = method as u32;
        }
    }
    Ok(AcpiTableSet {
        rsdp,
        root_address,
        root,
        tables,
        madt,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmbiosEntryPoint {
    pub major: u8,
    pub minor: u8,
    pub table_address: u64,
    pub table_length: u32,
    pub structure_count: Option<u16>,
}

pub fn parse_smbios_entry(bytes: &[u8]) -> Result<SmbiosEntryPoint, FirmwareError> {
    if bytes.starts_with(b"_SM3_") {
        if bytes.len() < 24 || bytes[6] < 24 || bytes[6] as usize > bytes.len() {
            return Err(FirmwareError::Truncated("SMBIOS3 entry"));
        }
        if checksum(&bytes[..bytes[6] as usize]) != 0 {
            return Err(FirmwareError::InvalidChecksum("SMBIOS3 entry"));
        }
        return Ok(SmbiosEntryPoint {
            major: bytes[7],
            minor: bytes[8],
            table_length: read_le_u32(bytes, 12)?,
            table_address: read_le_u64(bytes, 16)?,
            structure_count: None,
        });
    }
    if !bytes.starts_with(b"_SM_") || bytes.len() < 31 {
        return Err(FirmwareError::InvalidMagic("SMBIOS entry"));
    }
    let length = bytes[5] as usize;
    if length < 31 || length > bytes.len() {
        return Err(FirmwareError::InvalidLength("SMBIOS entry"));
    }
    if checksum(&bytes[..length]) != 0 || &bytes[16..21] != b"_DMI_" {
        return Err(FirmwareError::InvalidChecksum("SMBIOS entry"));
    }
    Ok(SmbiosEntryPoint {
        major: bytes[6],
        minor: bytes[7],
        table_length: read_le_u16(bytes, 22)? as u32,
        table_address: read_le_u32(bytes, 24)? as u64,
        structure_count: Some(read_le_u16(bytes, 28)?),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FdtInfo {
    pub total_size: u32,
    pub boot_cpu_id: u32,
    pub memory: Vec<KernelMemoryRegionV1>,
    pub reserved_memory: Vec<(u64, u64)>,
    pub topology: FirmwareTopology,
    pub bootargs: Option<String>,
    pub stdout_path: Option<String>,
    pub psci_method: Option<String>,
    pub framebuffer: Option<KernelFramebufferV1>,
}

#[derive(Debug, Clone)]
struct FdtNode {
    path: String,
    parent_address_cells: u32,
    parent_size_cells: u32,
    child_address_cells: u32,
    child_size_cells: u32,
    properties: BTreeMap<String, Vec<u8>>,
}

pub fn parse_fdt(bytes: &[u8]) -> Result<FdtInfo, FirmwareError> {
    if bytes.len() < 40 || read_be_u32(bytes, 0)? != FDT_MAGIC {
        return Err(FirmwareError::InvalidMagic("FDT"));
    }
    let total_size = read_be_u32(bytes, 4)? as usize;
    let struct_offset = read_be_u32(bytes, 8)? as usize;
    let strings_offset = read_be_u32(bytes, 12)? as usize;
    let reserve_offset = read_be_u32(bytes, 16)? as usize;
    let version = read_be_u32(bytes, 20)?;
    let boot_cpu_id = read_be_u32(bytes, 28)?;
    let strings_size = read_be_u32(bytes, 32)? as usize;
    let struct_size = read_be_u32(bytes, 36)? as usize;
    if version < 16 {
        return Err(FirmwareError::InvalidVersion("FDT"));
    }
    if total_size > bytes.len()
        || !range_valid(total_size, struct_offset, struct_size)
        || !range_valid(total_size, strings_offset, strings_size)
        || reserve_offset >= total_size
    {
        return Err(FirmwareError::InvalidRange("FDT block"));
    }

    let mut info = FdtInfo {
        total_size: total_size as u32,
        boot_cpu_id,
        ..FdtInfo::default()
    };
    let mut reserve = reserve_offset;
    loop {
        if reserve + 16 > total_size {
            return Err(FirmwareError::Truncated("FDT reserve map"));
        }
        let address = read_be_u64(bytes, reserve)?;
        let size = read_be_u64(bytes, reserve + 8)?;
        reserve += 16;
        if address == 0 && size == 0 {
            break;
        }
        address
            .checked_add(size)
            .ok_or(FirmwareError::ArithmeticOverflow)?;
        info.reserved_memory.push((address, size));
    }

    let strings = &bytes[strings_offset..strings_offset + strings_size];
    let structure = &bytes[struct_offset..struct_offset + struct_size];
    let mut cursor = 0_usize;
    let mut stack: Vec<FdtNode> = Vec::new();
    let mut saw_end = false;
    while cursor + 4 <= structure.len() {
        let token = read_be_u32(structure, cursor)?;
        cursor += 4;
        match token {
            1 => {
                let end = structure[cursor..]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|relative| cursor + relative)
                    .ok_or(FirmwareError::Truncated("FDT node name"))?;
                let name = String::from_utf8_lossy(&structure[cursor..end]).into_owned();
                cursor = align_up(end + 1, 4)?;
                let (parent_path, parent_address_cells, parent_size_cells) = stack
                    .last()
                    .map(|parent| {
                        (
                            parent.path.clone(),
                            parent.child_address_cells,
                            parent.child_size_cells,
                        )
                    })
                    .unwrap_or_else(|| (String::new(), 2, 1));
                let path = if parent_path.is_empty() || parent_path == "/" {
                    format!("/{name}")
                } else {
                    format!("{parent_path}/{name}")
                };
                stack.push(FdtNode {
                    path: if path == "/" { "/".into() } else { path },
                    parent_address_cells,
                    parent_size_cells,
                    child_address_cells: parent_address_cells,
                    child_size_cells: parent_size_cells,
                    properties: BTreeMap::new(),
                });
            }
            2 => {
                let node = stack.pop().ok_or(FirmwareError::InvalidTag(token))?;
                consume_fdt_node(&node, &mut info)?;
            }
            3 => {
                if cursor + 8 > structure.len() {
                    return Err(FirmwareError::Truncated("FDT property"));
                }
                let length = read_be_u32(structure, cursor)? as usize;
                let name_offset = read_be_u32(structure, cursor + 4)? as usize;
                cursor += 8;
                if cursor
                    .checked_add(length)
                    .is_none_or(|end| end > structure.len())
                {
                    return Err(FirmwareError::Truncated("FDT property value"));
                }
                let name = fdt_string(strings, name_offset)?;
                let value = structure[cursor..cursor + length].to_vec();
                cursor = align_up(cursor + length, 4)?;
                let node = stack.last_mut().ok_or(FirmwareError::InvalidTag(token))?;
                if name == "#address-cells" && value.len() == 4 {
                    node.child_address_cells = read_be_u32(&value, 0)?;
                } else if name == "#size-cells" && value.len() == 4 {
                    node.child_size_cells = read_be_u32(&value, 0)?;
                }
                node.properties.insert(name, value);
            }
            4 => {}
            9 => {
                if !stack.is_empty() {
                    return Err(FirmwareError::InvalidTag(token));
                }
                saw_end = true;
                break;
            }
            _ => return Err(FirmwareError::InvalidTag(token)),
        }
    }
    if !saw_end {
        return Err(FirmwareError::MissingRequired("FDT_END"));
    }
    for (address, size) in &info.reserved_memory {
        info.memory.push(KernelMemoryRegionV1 {
            base: *address,
            length: *size,
            kind: MEMORY_KIND_RESERVED,
            source: MEMORY_SOURCE_FDT,
            ..KernelMemoryRegionV1::default()
        });
    }
    if let Some(platform) = info.topology.smp_platform.as_mut() {
        platform.timer_frequency_hz = info.topology.timer_frequency_hz.unwrap_or(0);
        platform.context_count = info.topology.cpus.len().saturating_mul(2) as u32;
        if platform.architecture == SmpArchitecture::Aarch64 as u32 {
            platform.start_method = if info.psci_method.as_deref() == Some("hvc") {
                SmpStartMethod::PsciHvc as u32
            } else {
                SmpStartMethod::PsciSmc as u32
            };
        }
    }
    let mut handoff = FirmwareHandoff {
        architecture: Some(FirmwareArchitecture::RiscV),
        memory_map: core::mem::take(&mut info.memory),
        framebuffer: info.framebuffer,
        topology: core::mem::take(&mut info.topology),
        command_line: info.bootargs.clone(),
        ..FirmwareHandoff::default()
    };
    handoff.normalize()?;
    info.memory = handoff.memory_map;
    info.topology = handoff.topology;
    info.framebuffer = handoff.framebuffer;
    Ok(info)
}

fn consume_fdt_node(node: &FdtNode, info: &mut FdtInfo) -> Result<(), FirmwareError> {
    let compatible = property_strings(node.properties.get("compatible"));
    let device_type = node
        .properties
        .get("device_type")
        .map(|value| c_string(value))
        .transpose()?;
    let enabled = node
        .properties
        .get("status")
        .map(|value| c_string(value))
        .transpose()?
        .is_none_or(|status| status != "disabled" && status != "fail");
    if node.path == "/chosen" {
        info.bootargs = node
            .properties
            .get("bootargs")
            .map(|value| c_string(value))
            .transpose()?;
        info.stdout_path = node
            .properties
            .get("stdout-path")
            .or_else(|| node.properties.get("linux,stdout-path"))
            .map(|value| c_string(value))
            .transpose()?;
    }
    if node.path == "/psci" || compatible.iter().any(|item| item.starts_with("arm,psci")) {
        info.psci_method = node
            .properties
            .get("method")
            .map(|value| c_string(value))
            .transpose()?;
    }
    if device_type.as_deref() == Some("memory") || node.path.starts_with("/memory@") {
        for (address, size) in decode_reg(node)? {
            info.memory.push(KernelMemoryRegionV1 {
                base: address,
                length: size,
                kind: MEMORY_KIND_USABLE,
                source: MEMORY_SOURCE_FDT,
                ..KernelMemoryRegionV1::default()
            });
        }
    }
    if device_type.as_deref() == Some("cpu") && node.path.starts_with("/cpus/") {
        let hardware_id = decode_reg(node)?
            .first()
            .map(|(address, _)| *address)
            .or_else(|| {
                node.properties
                    .get("reg")
                    .and_then(|value| decode_cells(value, node.parent_address_cells).ok())
            })
            .ok_or_else(|| FirmwareError::InvalidProperty(format!("{}:reg", node.path)))?;
        info.topology.cpus.push(FirmwareCpu {
            hardware_id,
            firmware_id: hardware_id,
            enabled,
            bootstrap: hardware_id == u64::from(info.boot_cpu_id),
            numa_node: node
                .properties
                .get("numa-node-id")
                .filter(|value| value.len() >= 4)
                .map(|value| read_be_u32(value, 0))
                .transpose()?
                .unwrap_or(0),
        });
    }
    let gic_v3 = compatible.iter().any(|item| item.contains("gic-v3"));
    let plic = compatible.iter().any(|item| item.contains("riscv,plic"));
    let aplic = compatible.iter().any(|item| item.contains("riscv,aplic"));
    let imsic = compatible.iter().any(|item| item.contains("riscv,imsics"));
    if gic_v3 || plic || aplic || imsic {
        let registers = decode_reg(node)?;
        info.topology.interrupt_controller = Some(compatible.join(","));
        info.topology.interrupt_controller_base = registers.first().map(|entry| entry.0);
        if gic_v3 {
            let platform = info.topology.smp_platform.get_or_insert_with(|| {
                KernelSmpPlatformV1::new(
                    SmpArchitecture::Aarch64,
                    SmpStartMethod::PsciSmc,
                    SmpInterruptModel::GicV3,
                )
            });
            platform.flags |= KernelSmpPlatformV1::FLAG_DISCOVERED_FROM_FDT;
            platform.primary_controller_base = registers.first().map(|entry| entry.0).unwrap_or(0);
            platform.secondary_controller_base = registers.get(1).map(|entry| entry.0).unwrap_or(0);
            platform.redistributor_stride = node
                .properties
                .get("redistributor-stride")
                .map(|value| decode_cells(value, (value.len() / 4) as u32))
                .transpose()?
                .unwrap_or(0x20_000);
            platform.feature_flags |= KernelSmpPlatformV1::FEATURE_IPI
                | KernelSmpPlatformV1::FEATURE_TLB_RANGE
                | KernelSmpPlatformV1::FEATURE_TLB_ASID
                | KernelSmpPlatformV1::FEATURE_PER_CPU_TIMER;
        } else {
            let platform = info.topology.smp_platform.get_or_insert_with(|| {
                KernelSmpPlatformV1::new(
                    SmpArchitecture::RiscV,
                    SmpStartMethod::SbiHsm,
                    SmpInterruptModel::Plic,
                )
            });
            platform.flags |= KernelSmpPlatformV1::FLAG_DISCOVERED_FROM_FDT;
            if plic {
                platform.primary_controller_base =
                    registers.first().map(|entry| entry.0).unwrap_or(0);
                platform.interrupt_model = if platform.secondary_controller_base == 0 {
                    SmpInterruptModel::Plic as u32
                } else {
                    SmpInterruptModel::PlicImsic as u32
                };
                platform.source_count = node
                    .properties
                    .get("riscv,ndev")
                    .filter(|value| value.len() >= 4)
                    .map(|value| read_be_u32(value, 0))
                    .transpose()?
                    .unwrap_or(0);
            }
            if aplic {
                platform.primary_controller_base =
                    registers.first().map(|entry| entry.0).unwrap_or(0);
                platform.interrupt_model = if platform.secondary_controller_base == 0 {
                    SmpInterruptModel::Aplic as u32
                } else {
                    SmpInterruptModel::AplicImsic as u32
                };
            }
            if imsic {
                platform.secondary_controller_base =
                    registers.first().map(|entry| entry.0).unwrap_or(0);
                platform.interrupt_model = if platform.primary_controller_base == 0 {
                    SmpInterruptModel::Imsic as u32
                } else if platform.interrupt_model == SmpInterruptModel::Plic as u32 {
                    SmpInterruptModel::PlicImsic as u32
                } else {
                    SmpInterruptModel::AplicImsic as u32
                };
                platform.context_stride = 0x1000;
                platform.feature_flags |= KernelSmpPlatformV1::FEATURE_MSI;
            }
            platform.feature_flags |= KernelSmpPlatformV1::FEATURE_IPI
                | KernelSmpPlatformV1::FEATURE_TLB_RANGE
                | KernelSmpPlatformV1::FEATURE_TLB_ASID
                | KernelSmpPlatformV1::FEATURE_PER_CPU_TIMER;
        }
    }
    if compatible.iter().any(|item| item == "simple-framebuffer") && enabled {
        let (base, size) = decode_reg(node)?
            .first()
            .copied()
            .ok_or_else(|| FirmwareError::InvalidProperty(format!("{}:reg", node.path)))?;
        let width = property_u32(node, "width")?;
        let height = property_u32(node, "height")?;
        let pitch = property_u32(node, "stride")?;
        let format = node
            .properties
            .get("format")
            .map(|value| c_string(value))
            .transpose()?
            .map(|value| match value.as_str() {
                "a8r8g8b8" => 1,
                "a8b8g8r8" => 2,
                "r8g8b8a8" => 3,
                "b8g8r8a8" => 4,
                _ => 0,
            })
            .unwrap_or(0);
        let framebuffer = KernelFramebufferV1 {
            base,
            width,
            height,
            pitch,
            format,
            flags: size,
        }
        .validate()?;
        info.framebuffer = Some(framebuffer);
    }
    if let Some(frequency) = node
        .properties
        .get("timebase-frequency")
        .or_else(|| node.properties.get("clock-frequency"))
        .filter(|value| value.len() == 4)
        .map(|value| read_be_u32(value, 0))
        .transpose()?
    {
        if node.path.contains("timer")
            || node.path == "/cpus"
            || device_type.as_deref() == Some("cpu")
        {
            info.topology.timer_frequency_hz = Some(u64::from(frequency));
        }
    }
    Ok(())
}

fn property_u32(node: &FdtNode, name: &str) -> Result<u32, FirmwareError> {
    let value = node
        .properties
        .get(name)
        .ok_or_else(|| FirmwareError::InvalidProperty(format!("{}:{name}", node.path)))?;
    if value.len() != 4 {
        return Err(FirmwareError::InvalidProperty(format!(
            "{}:{name}",
            node.path
        )));
    }
    read_be_u32(value, 0)
}

fn decode_reg(node: &FdtNode) -> Result<Vec<(u64, u64)>, FirmwareError> {
    let Some(value) = node.properties.get("reg") else {
        return Ok(Vec::new());
    };
    let address_cells = node.parent_address_cells as usize;
    let size_cells = node.parent_size_cells as usize;
    if address_cells == 0 || address_cells > 2 || size_cells > 2 {
        return Err(FirmwareError::InvalidProperty(format!(
            "{}:reg cells",
            node.path
        )));
    }
    let tuple_cells = address_cells + size_cells;
    let tuple_bytes = tuple_cells
        .checked_mul(4)
        .ok_or(FirmwareError::ArithmeticOverflow)?;
    if tuple_bytes == 0 || value.len() % tuple_bytes != 0 {
        return Err(FirmwareError::InvalidProperty(format!("{}:reg", node.path)));
    }
    let mut result = Vec::new();
    for tuple in value.chunks_exact(tuple_bytes) {
        let address = decode_cells(&tuple[..address_cells * 4], address_cells as u32)?;
        let size = decode_cells(&tuple[address_cells * 4..], size_cells as u32)?;
        result.push((address, size));
    }
    Ok(result)
}

fn decode_cells(bytes: &[u8], cells: u32) -> Result<u64, FirmwareError> {
    if cells > 2 || bytes.len() != cells as usize * 4 {
        return Err(FirmwareError::InvalidLength("FDT cells"));
    }
    let mut value = 0_u64;
    for cell in bytes.chunks_exact(4) {
        value = (value << 32) | u64::from(read_be_u32(cell, 0)?);
    }
    Ok(value)
}

fn property_strings(value: Option<&Vec<u8>>) -> Vec<String> {
    value
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|item| !item.is_empty())
                .map(|item| String::from_utf8_lossy(item).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn fdt_string(strings: &[u8], offset: usize) -> Result<String, FirmwareError> {
    if offset >= strings.len() {
        return Err(FirmwareError::InvalidRange("FDT string"));
    }
    let end = strings[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|relative| offset + relative)
        .ok_or(FirmwareError::Truncated("FDT string"))?;
    Ok(String::from_utf8_lossy(&strings[offset..end]).into_owned())
}

pub fn adapt_opensbi_handoff(
    hart_id: u64,
    fdt_address: u64,
    fdt_bytes: &[u8],
) -> Result<FirmwareHandoff, FirmwareError> {
    if fdt_address == 0 {
        return Err(FirmwareError::MissingRequired("OpenSBI FDT"));
    }
    let fdt = parse_fdt(fdt_bytes)?;
    let mut handoff = FirmwareHandoff {
        architecture: Some(FirmwareArchitecture::RiscV),
        memory_map: fdt.memory,
        framebuffer: fdt.framebuffer,
        topology: fdt.topology,
        command_line: fdt.bootargs,
        ..FirmwareHandoff::default()
    };
    handoff.platform_entries.push(KernelPlatformEntryV1 {
        kind: PLATFORM_KIND_FDT,
        flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_REQUIRED | PLATFORM_FLAG_EXTERNAL,
        ptr: fdt_address,
        length: fdt.total_size as u64,
        ..KernelPlatformEntryV1::default()
    });
    handoff.platform_entries.push(KernelPlatformEntryV1 {
        kind: PLATFORM_KIND_OPENSBI_HART,
        flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
        ptr: hart_id,
        checksum_or_revision: 1,
        ..KernelPlatformEntryV1::default()
    });
    handoff.topology.normalize(Some(hart_id))?;
    handoff.normalize()?;
    Ok(handoff)
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
}

fn c_string(bytes: &[u8]) -> Result<String, FirmwareError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if bytes[..end].contains(&0) {
        return Err(FirmwareError::InvalidProperty("embedded NUL".into()));
    }
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn align_up(value: usize, alignment: usize) -> Result<usize, FirmwareError> {
    if !alignment.is_power_of_two() {
        return Err(FirmwareError::InvalidAlignment("firmware record"));
    }
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(FirmwareError::ArithmeticOverflow)
}

fn range_valid(total: usize, offset: usize, length: usize) -> bool {
    offset
        .checked_add(length)
        .is_some_and(|end| offset <= total && end <= total)
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16, FirmwareError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(FirmwareError::Truncated("u16"))?;
    Ok(u16::from_le_bytes(raw.try_into().expect("two bytes")))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, FirmwareError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(FirmwareError::Truncated("u32"))?;
    Ok(u32::from_le_bytes(raw.try_into().expect("four bytes")))
}

fn read_le_u64(bytes: &[u8], offset: usize) -> Result<u64, FirmwareError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(FirmwareError::Truncated("u64"))?;
    Ok(u64::from_le_bytes(raw.try_into().expect("eight bytes")))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, FirmwareError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(FirmwareError::Truncated("be32"))?;
    Ok(u32::from_be_bytes(raw.try_into().expect("four bytes")))
}

fn read_be_u64(bytes: &[u8], offset: usize) -> Result<u64, FirmwareError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(FirmwareError::Truncated("be64"))?;
    Ok(u64::from_be_bytes(raw.try_into().expect("eight bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_external_fdt_fixture_obeys_physical_handoff_contract() {
        let Ok(path) = std::env::var("NCC_FDT_FIXTURE") else {
            return;
        };
        let bytes = std::fs::read(path).expect("read NCC_FDT_FIXTURE");
        let info = parse_fdt(&bytes).expect("parse external FDT fixture");
        assert!(info.total_size as usize <= bytes.len());
        assert!(!info.memory.is_empty(), "external FDT has no memory nodes");
        assert!(
            !info.topology.cpus.is_empty(),
            "external FDT has no CPU nodes"
        );
    }

    fn finish_checksum(bytes: &mut [u8], checksum_offset: usize) {
        bytes[checksum_offset] = 0;
        bytes[checksum_offset] = 0_u8.wrapping_sub(checksum(bytes));
    }

    fn acpi_sdt(signature: &[u8; 4], revision: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0_u8; 36];
        bytes[..4].copy_from_slice(signature);
        bytes[4..8].copy_from_slice(&((36 + payload.len()) as u32).to_le_bytes());
        bytes[8] = revision;
        bytes[10..16].copy_from_slice(b"NCCFW ");
        bytes[16..24].copy_from_slice(b"PHYSMP  ");
        bytes.extend_from_slice(payload);
        finish_checksum(&mut bytes, 9);
        bytes
    }

    fn acpi_rsdp_for_xsdt(xsdt: u64) -> Vec<u8> {
        let mut rsdp = vec![0_u8; 36];
        rsdp[..8].copy_from_slice(b"RSD PTR ");
        rsdp[9..15].copy_from_slice(b"NCCFW ");
        rsdp[15] = 2;
        rsdp[20..24].copy_from_slice(&36_u32.to_le_bytes());
        rsdp[24..32].copy_from_slice(&xsdt.to_le_bytes());
        finish_checksum(&mut rsdp[..20], 8);
        finish_checksum(&mut rsdp, 32);
        rsdp
    }

    #[test]
    fn layouts_are_stable_and_boot_flags_are_validated() {
        assert_eq!(core::mem::size_of::<KernelBootInfoV1>(), 88);
        assert_eq!(core::mem::size_of::<KernelMemoryRegionV1>(), 32);
        assert_eq!(core::mem::size_of::<KernelCpuTopologyEntryV1>(), 48);
        let boot = KernelBootInfoV1 {
            magic: KERNEL_BOOT_INFO_MAGIC,
            version: 1,
            flags: BOOT_FLAG_MEMORY_MAP | BOOT_FLAG_PLATFORM_TABLE,
            memory_map_ptr: 0x1000,
            memory_map_len: 2,
            platform_table_ptr: 0x2000,
            ..KernelBootInfoV1::default()
        };
        assert_eq!(boot.validate(), Ok(boot));
    }

    #[test]
    fn uefi_memory_and_configuration_tables_become_boot_contract() {
        let handoff = adapt_uefi_handoff(
            FirmwareArchitecture::X86,
            0x1000,
            &[
                UefiConfigurationTable {
                    guid: UefiGuid::ACPI_20,
                    address: 0x2000,
                },
                UefiConfigurationTable {
                    guid: UefiGuid::SMBIOS3,
                    address: 0x3000,
                },
                UefiConfigurationTable {
                    guid: UefiGuid::FDT,
                    address: 0x4000,
                },
            ],
            &[
                UefiMemoryDescriptor {
                    memory_type: 7,
                    physical_start: 0x10_0000,
                    page_count: 4,
                    ..UefiMemoryDescriptor::default()
                },
                UefiMemoryDescriptor {
                    memory_type: 9,
                    physical_start: 0x20_0000,
                    page_count: 1,
                    ..UefiMemoryDescriptor::default()
                },
            ],
            Some(UefiGraphicsOutput {
                base: 0x8000_0000,
                size: 1024 * 768 * 4,
                width: 1024,
                height: 768,
                pixels_per_scanline: 1024,
                pixel_format: 1,
            }),
        )
        .unwrap();
        assert_eq!(handoff.memory_map[0].kind, MEMORY_KIND_USABLE);
        assert!(handoff
            .platform_entries
            .iter()
            .any(|entry| entry.kind == PLATFORM_KIND_ACPI_RSDP));
        assert!(handoff
            .platform_entries
            .iter()
            .any(|entry| entry.kind == PLATFORM_KIND_FDT && entry.ptr == 0x4000));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_UEFI_SYSTEM_TABLE
                && entry.ptr == 0x1000
                && entry.length == 24
                && entry.flags & PLATFORM_FLAG_DYNAMIC_LENGTH != 0
        }));
        assert_eq!(
            handoff.framebuffer.unwrap().flags,
            FRAMEBUFFER_FLAG_LINEAR | FRAMEBUFFER_FLAG_FIRMWARE_OWNED
        );
    }

    #[test]
    fn uefi_gop_rejects_blt_only_and_truncated_linear_buffers() {
        for graphics in [
            UefiGraphicsOutput {
                base: 0x8000_0000,
                size: 1024 * 768 * 4,
                width: 1024,
                height: 768,
                pixels_per_scanline: 1024,
                pixel_format: 3,
            },
            UefiGraphicsOutput {
                base: 0x8000_0000,
                size: 4096,
                width: 1024,
                height: 768,
                pixels_per_scanline: 1024,
                pixel_format: FRAMEBUFFER_FORMAT_BGR_RESERVED_8,
            },
        ] {
            assert!(adapt_uefi_handoff(
                FirmwareArchitecture::X86,
                0x1000,
                &[],
                &[UefiMemoryDescriptor {
                    memory_type: 7,
                    physical_start: 0x10_0000,
                    page_count: 4,
                    ..UefiMemoryDescriptor::default()
                }],
                Some(graphics),
            )
            .is_err());
        }
    }

    #[test]
    fn multiboot2_parser_consumes_command_map_framebuffer_and_acpi() {
        let mut bytes = vec![0_u8; 8];
        let mut command = b"root=/dev/vda\0".to_vec();
        push_mb2_test_tag(&mut bytes, 1, &mut command);
        let mut map = Vec::new();
        map.extend_from_slice(&24_u32.to_le_bytes());
        map.extend_from_slice(&0_u32.to_le_bytes());
        map.extend_from_slice(&0x10_0000_u64.to_le_bytes());
        map.extend_from_slice(&0x20_0000_u64.to_le_bytes());
        map.extend_from_slice(&1_u32.to_le_bytes());
        map.extend_from_slice(&0_u32.to_le_bytes());
        push_mb2_test_tag(&mut bytes, 6, &mut map);
        let mut framebuffer = Vec::new();
        framebuffer.extend_from_slice(&0xe000_0000_u64.to_le_bytes());
        framebuffer.extend_from_slice(&4096_u32.to_le_bytes());
        framebuffer.extend_from_slice(&1024_u32.to_le_bytes());
        framebuffer.extend_from_slice(&768_u32.to_le_bytes());
        framebuffer.extend_from_slice(&[32, 1, 16, 8, 8, 8, 0, 8]);
        push_mb2_test_tag(&mut bytes, 8, &mut framebuffer);
        let mut rsdp = vec![0_u8; 36];
        rsdp[..8].copy_from_slice(b"RSD PTR ");
        rsdp[15] = 2;
        rsdp[20..24].copy_from_slice(&36_u32.to_le_bytes());
        rsdp[24..32].copy_from_slice(&0x1234_5000_u64.to_le_bytes());
        finish_checksum(&mut rsdp[..20], 8);
        finish_checksum(&mut rsdp, 32);
        push_mb2_test_tag(&mut bytes, 15, &mut rsdp);
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        let total = bytes.len() as u32;
        bytes[..4].copy_from_slice(&total.to_le_bytes());
        let handoff = parse_multiboot2_information(&bytes, 0x5000).unwrap();
        assert_eq!(handoff.command_line.as_deref(), Some("root=/dev/vda"));
        assert_eq!(handoff.memory_map[0].kind, MEMORY_KIND_USABLE);
        assert!(handoff.framebuffer.is_some());
        assert!(handoff
            .platform_entries
            .iter()
            .any(|entry| entry.kind == PLATFORM_KIND_ACPI_RSDP));
        assert_eq!(
            handoff.framebuffer.unwrap().format,
            FRAMEBUFFER_FORMAT_BGR_RESERVED_8
        );
    }

    #[test]
    fn multiboot2_parser_preserves_extended_firmware_and_efi_map_fallback() {
        let mut bytes = vec![0_u8; 8];
        let mut bootloader = b"GRUB 2.12\0".to_vec();
        push_mb2_test_tag(&mut bytes, 2, &mut bootloader);
        let mut basic = Vec::new();
        basic.extend_from_slice(&640_u32.to_le_bytes());
        basic.extend_from_slice(&1024_u32.to_le_bytes());
        push_mb2_test_tag(&mut bytes, 4, &mut basic);
        let mut boot_device = Vec::new();
        boot_device.extend_from_slice(&0x80_u32.to_le_bytes());
        boot_device.extend_from_slice(&1_u32.to_le_bytes());
        boot_device.extend_from_slice(&2_u32.to_le_bytes());
        push_mb2_test_tag(&mut bytes, 5, &mut boot_device);
        let mut efi_map = Vec::new();
        efi_map.extend_from_slice(&40_u32.to_le_bytes());
        efi_map.extend_from_slice(&1_u32.to_le_bytes());
        efi_map.extend_from_slice(&7_u32.to_le_bytes());
        efi_map.extend_from_slice(&0_u32.to_le_bytes());
        efi_map.extend_from_slice(&0x20_0000_u64.to_le_bytes());
        efi_map.extend_from_slice(&0_u64.to_le_bytes());
        efi_map.extend_from_slice(&16_u64.to_le_bytes());
        efi_map.extend_from_slice(&8_u64.to_le_bytes());
        push_mb2_test_tag(&mut bytes, 17, &mut efi_map);
        let mut efi32_handle = 0x1234_u32.to_le_bytes().to_vec();
        push_mb2_test_tag(&mut bytes, 19, &mut efi32_handle);
        let mut efi64_handle = 0x1234_5678_9abc_def0_u64.to_le_bytes().to_vec();
        push_mb2_test_tag(&mut bytes, 20, &mut efi64_handle);
        let mut efi64_system_table = 0x1_1234_5000_u64.to_le_bytes().to_vec();
        push_mb2_test_tag(&mut bytes, 12, &mut efi64_system_table);
        let mut efi_boot_services = Vec::new();
        push_mb2_test_tag(&mut bytes, 18, &mut efi_boot_services);
        let mut load_base = 0x10_0000_u32.to_le_bytes().to_vec();
        push_mb2_test_tag(&mut bytes, 21, &mut load_base);
        let mut smbios = vec![3, 7, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3];
        push_mb2_test_tag(&mut bytes, 13, &mut smbios);
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        let total = bytes.len() as u32;
        bytes[..4].copy_from_slice(&total.to_le_bytes());

        let handoff = parse_multiboot2_information(&bytes, 0x8000).unwrap();
        assert_eq!(handoff.bootloader_name.as_deref(), Some("GRUB 2.12"));
        assert!(handoff.memory_map.iter().any(|entry| {
            entry.base == 0x20_0000
                && entry.length == 16 * UEFI_PAGE_SIZE
                && entry.kind == MEMORY_KIND_USABLE
                && entry.source == MEMORY_SOURCE_UEFI
        }));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_UEFI_MEMORY_MAP && entry.checksum_or_revision == 1
        }));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_UEFI_IMAGE_HANDLE
                && entry.ptr == 0x1234_5678_9abc_def0
                && entry.checksum_or_revision == 2
        }));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_UEFI_SYSTEM_TABLE
                && entry.ptr == 0x1_1234_5000
                && entry.length == 24
                && entry.flags & PLATFORM_FLAG_DYNAMIC_LENGTH != 0
        }));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_UEFI_BOOT_SERVICES && entry.ptr != 0 && entry.length == 8
        }));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_MULTIBOOT2_BOOT_LOADER_NAME
                && entry.ptr != 0
                && entry.length == 10
        }));
        assert!(handoff
            .platform_entries
            .iter()
            .any(|entry| entry.kind == PLATFORM_KIND_MULTIBOOT2_LOAD_BASE));
        assert!(handoff.platform_entries.iter().any(|entry| {
            entry.kind == PLATFORM_KIND_SMBIOS_TABLES
                && entry.length == 4
                && entry.checksum_or_revision == (3_u64 << 32 | 7)
        }));
    }

    fn push_mb2_test_tag(bytes: &mut Vec<u8>, kind: u32, payload: &mut Vec<u8>) {
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32 + 8).to_le_bytes());
        bytes.append(payload);
        while bytes.len() & 7 != 0 {
            bytes.push(0);
        }
    }

    #[test]
    fn acpi_and_smbios_checksums_are_materially_checked() {
        let mut rsdp = vec![0_u8; 36];
        rsdp[..8].copy_from_slice(b"RSD PTR ");
        rsdp[15] = 2;
        rsdp[20..24].copy_from_slice(&36_u32.to_le_bytes());
        rsdp[24..32].copy_from_slice(&0x1234_5000_u64.to_le_bytes());
        finish_checksum(&mut rsdp[..20], 8);
        finish_checksum(&mut rsdp, 32);
        assert_eq!(parse_acpi_rsdp(&rsdp).unwrap().xsdt_address, 0x1234_5000);

        let mut smbios = vec![0_u8; 24];
        smbios[..5].copy_from_slice(b"_SM3_");
        smbios[6] = 24;
        smbios[7] = 3;
        smbios[8] = 6;
        smbios[12..16].copy_from_slice(&4096_u32.to_le_bytes());
        smbios[16..24].copy_from_slice(&0x8000_u64.to_le_bytes());
        finish_checksum(&mut smbios, 5);
        assert_eq!(parse_smbios_entry(&smbios).unwrap().table_address, 0x8000);
    }

    #[test]
    fn acpi_xsdt_madt_srat_materialize_x2apic_numa_and_smp_descriptor() {
        let xsdt_address = 0x1000_u64;
        let madt_address = 0x2000_u64;
        let srat_address = 0x3000_u64;

        let mut madt_payload = Vec::new();
        madt_payload.extend_from_slice(&0xfee0_0000_u32.to_le_bytes());
        madt_payload.extend_from_slice(&1_u32.to_le_bytes());
        for (apic, uid) in [(2_u32, 20_u32), (4, 40)] {
            madt_payload.extend_from_slice(&[9, 16, 0, 0]);
            madt_payload.extend_from_slice(&apic.to_le_bytes());
            madt_payload.extend_from_slice(&1_u32.to_le_bytes());
            madt_payload.extend_from_slice(&uid.to_le_bytes());
        }
        madt_payload.extend_from_slice(&[1, 12, 7, 0]);
        madt_payload.extend_from_slice(&0xfec0_0000_u32.to_le_bytes());
        madt_payload.extend_from_slice(&0_u32.to_le_bytes());
        madt_payload.extend_from_slice(&[5, 12, 0, 0]);
        madt_payload.extend_from_slice(&0xfee0_0000_u64.to_le_bytes());
        let madt = acpi_sdt(b"APIC", 7, &madt_payload);

        let mut srat_payload = vec![0_u8; 12];
        for (domain, apic) in [(3_u32, 2_u32), (9, 4)] {
            srat_payload.extend_from_slice(&[2, 24, 0, 0]);
            srat_payload.extend_from_slice(&domain.to_le_bytes());
            srat_payload.extend_from_slice(&apic.to_le_bytes());
            srat_payload.extend_from_slice(&1_u32.to_le_bytes());
            srat_payload.extend_from_slice(&0_u32.to_le_bytes());
            srat_payload.extend_from_slice(&0_u32.to_le_bytes());
        }
        let srat = acpi_sdt(b"SRAT", 3, &srat_payload);
        let mut xsdt_payload = Vec::new();
        xsdt_payload.extend_from_slice(&madt_address.to_le_bytes());
        xsdt_payload.extend_from_slice(&srat_address.to_le_bytes());
        let xsdt = acpi_sdt(b"XSDT", 1, &xsdt_payload);
        let rsdp = acpi_rsdp_for_xsdt(xsdt_address);
        let tables = BTreeMap::from([
            (xsdt_address, xsdt),
            (madt_address, madt),
            (srat_address, srat),
        ]);

        let parsed = parse_acpi_table_set(&rsdp, &tables, 2).unwrap();
        assert_eq!(parsed.madt.topology.cpus.len(), 2);
        assert_eq!(parsed.madt.topology.cpus[0].numa_node, 3);
        assert_eq!(parsed.madt.topology.cpus[1].numa_node, 9);
        assert_eq!(parsed.madt.io_apics[0].address, 0xfec0_0000);
        let platform = parsed.madt.topology.smp_platform.unwrap();
        assert_eq!(platform.interrupt_model, SmpInterruptModel::X2Apic as u32);
        assert_eq!(platform.secondary_controller_base, 0xfec0_0000);
    }

    #[test]
    fn acpi_arm_gicc_gicr_its_and_fadt_select_hvc_psci() {
        let xsdt_address = 0x10_0000_u64;
        let madt_address = 0x11_0000_u64;
        let fadt_address = 0x12_0000_u64;
        let mut payload = vec![0_u8; 8];
        for (uid, mpidr) in [(0_u32, 0_u64), (1, 0x100)] {
            let mut gicc = vec![0_u8; 82];
            gicc[0] = 11;
            gicc[1] = 82;
            gicc[8..12].copy_from_slice(&uid.to_le_bytes());
            gicc[12..16].copy_from_slice(&1_u32.to_le_bytes());
            gicc[68..76].copy_from_slice(&mpidr.to_le_bytes());
            payload.extend_from_slice(&gicc);
        }
        let mut gicd = vec![0_u8; 24];
        gicd[0] = 12;
        gicd[1] = 24;
        gicd[4..8].copy_from_slice(&1_u32.to_le_bytes());
        gicd[8..16].copy_from_slice(&0x0800_0000_u64.to_le_bytes());
        gicd[20] = 3;
        payload.extend_from_slice(&gicd);
        let mut gicr = vec![0_u8; 16];
        gicr[0] = 14;
        gicr[1] = 16;
        gicr[4..12].copy_from_slice(&0x080a_0000_u64.to_le_bytes());
        gicr[12..16].copy_from_slice(&0x40_000_u32.to_le_bytes());
        payload.extend_from_slice(&gicr);
        let mut its = vec![0_u8; 20];
        its[0] = 15;
        its[1] = 20;
        its[4..8].copy_from_slice(&2_u32.to_le_bytes());
        its[8..16].copy_from_slice(&0x0808_0000_u64.to_le_bytes());
        payload.extend_from_slice(&its);
        let madt = acpi_sdt(b"APIC", 7, &payload);

        let mut fadt = acpi_sdt(b"FACP", 6, &vec![0_u8; 95]);
        fadt[129..131].copy_from_slice(&3_u16.to_le_bytes());
        finish_checksum(&mut fadt, 9);
        let mut xsdt_payload = Vec::new();
        xsdt_payload.extend_from_slice(&madt_address.to_le_bytes());
        xsdt_payload.extend_from_slice(&fadt_address.to_le_bytes());
        let xsdt = acpi_sdt(b"XSDT", 1, &xsdt_payload);
        let rsdp = acpi_rsdp_for_xsdt(xsdt_address);
        let tables = BTreeMap::from([
            (xsdt_address, xsdt),
            (madt_address, madt),
            (fadt_address, fadt),
        ]);

        let parsed = parse_acpi_table_set(&rsdp, &tables, 0).unwrap();
        let platform = parsed.madt.topology.smp_platform.unwrap();
        assert_eq!(platform.start_method, SmpStartMethod::PsciHvc as u32);
        assert_eq!(platform.primary_controller_base, 0x0800_0000);
        assert_eq!(platform.secondary_controller_base, 0x080a_0000);
        assert_eq!(platform.tertiary_controller_base, 0x0808_0000);
        assert_eq!(parsed.madt.topology.entries()[1].core_id, 1);
    }

    #[test]
    fn normalized_handoff_materializes_relocatable_boot_bytes() {
        let handoff = FirmwareHandoff {
            architecture: Some(FirmwareArchitecture::Aarch64),
            memory_map: vec![KernelMemoryRegionV1 {
                base: 0x4000_0000,
                length: 0x1000_0000,
                kind: MEMORY_KIND_USABLE,
                source: MEMORY_SOURCE_UEFI,
                ..KernelMemoryRegionV1::default()
            }],
            framebuffer: Some(KernelFramebufferV1 {
                base: 0x9000_0000,
                width: 800,
                height: 600,
                pitch: 3200,
                format: 1,
                flags: 1,
            }),
            serial: Some(KernelSerialV1 {
                magic: KERNEL_SERIAL_MAGIC,
                version: 1,
                kind: 2,
                base: 0x0900_0000,
                irq: 33,
                baud: 115_200,
                flags: 1,
                register_shift: 2,
            }),
            platform_entries: vec![KernelPlatformEntryV1 {
                kind: PLATFORM_KIND_ACPI_RSDP,
                flags: PLATFORM_FLAG_PRESENT | PLATFORM_FLAG_EXTERNAL,
                ptr: 0x7ff0_0000,
                length: 36,
                checksum_or_revision: 2,
            }],
            topology: FirmwareTopology {
                cpus: vec![
                    FirmwareCpu {
                        hardware_id: 0,
                        firmware_id: 0,
                        enabled: true,
                        bootstrap: true,
                        numa_node: 0,
                    },
                    FirmwareCpu {
                        hardware_id: 0x100,
                        firmware_id: 1,
                        enabled: true,
                        bootstrap: false,
                        numa_node: 0,
                    },
                ],
                interrupt_controller: Some("arm,gic-v3".into()),
                interrupt_controller_base: Some(0x0800_0000),
                timer_frequency_hz: Some(62_500_000),
                smp_platform: Some({
                    let mut platform = KernelSmpPlatformV1::new(
                        SmpArchitecture::Aarch64,
                        SmpStartMethod::PsciSmc,
                        SmpInterruptModel::GicV3,
                    );
                    platform.primary_controller_base = 0x0800_0000;
                    platform.secondary_controller_base = 0x080a_0000;
                    platform.timer_frequency_hz = 62_500_000;
                    platform.redistributor_stride = 0x20_000;
                    platform
                }),
                ..FirmwareTopology::default()
            },
            command_line: Some("console=ttyAMA0".into()),
            bootloader_name: Some("ncc-uefi".into()),
        };
        let image = handoff
            .materialize(0x8100_0000, 0x4008_0000, 0x20_0000)
            .unwrap();
        assert_eq!(
            read_le_u64(&image.bytes, 0).unwrap(),
            KERNEL_BOOT_INFO_MAGIC
        );
        let flags = read_le_u64(&image.bytes, 16).unwrap();
        assert_eq!(
            flags
                & (BOOT_FLAG_MEMORY_MAP
                    | BOOT_FLAG_SERIAL
                    | BOOT_FLAG_PLATFORM_TABLE
                    | BOOT_FLAG_FRAMEBUFFER
                    | BOOT_FLAG_CPU_TOPOLOGY),
            BOOT_FLAG_MEMORY_MAP
                | BOOT_FLAG_SERIAL
                | BOOT_FLAG_PLATFORM_TABLE
                | BOOT_FLAG_FRAMEBUFFER
                | BOOT_FLAG_CPU_TOPOLOGY
        );
        assert_eq!(
            read_le_u64(&image.bytes, 24).unwrap(),
            image.memory_map_address
        );
        assert!(image.smp_platform_address.is_some());
        assert_eq!(
            read_le_u64(&image.bytes, 56).unwrap(),
            image.platform_table_address
        );
        let topology = image.topology_address.unwrap();
        let topology_offset = (topology - image.load_address) as usize;
        assert_eq!(
            read_le_u64(&image.bytes, topology_offset).unwrap(),
            KERNEL_CPU_TOPOLOGY_MAGIC
        );
        assert_eq!(read_le_u32(&image.bytes, topology_offset + 12).unwrap(), 2);
        assert!(image.end_address().unwrap() > image.boot_info_address);
    }
}
