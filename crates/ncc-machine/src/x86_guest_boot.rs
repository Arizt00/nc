use core::fmt;

pub const BIOS_BOOT_SECTOR_BYTES: usize = 512;
pub const BIOS_BOOT_SIGNATURE_OFFSET: usize = 510;
pub const BIOS_BOOT_LOAD_ADDRESS: u64 = 0x7c00;
pub const BIOS_RESET_VECTOR: u64 = 0xffff0;
pub const UEFI_FIRMWARE_TOP: u64 = 0x1_0000_0000;
pub const LINUX_SETUP_LOAD_ADDRESS: u64 = 0x90000;
pub const LINUX_PROTECTED_LOAD_ADDRESS: u64 = 0x10_0000;
pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe852_50d6;
pub const MULTIBOOT2_BOOT_MAGIC: u32 = 0x36d7_6289;
pub const FAT12_DISK_BYTES: usize = 1_474_560;
pub const UEFI_GPT_MIN_DISK_BYTES: usize = 40 * 1024 * 1024;
pub const MULTIBOOT2_X86_64_LOADER_BASE: u64 = 0x0010_0000;
pub const MULTIBOOT2_X86_64_DATA_BASE: u64 = 0x0011_0000;
pub const MULTIBOOT2_X86_64_STACK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestBootError {
    ImageTooSmall { required: usize, actual: usize },
    UnsupportedImage,
    UnsupportedArchitecture(u16),
    UnsupportedEndianness,
    InvalidDosHeader,
    InvalidPeHeader,
    InvalidPeOptionalHeader,
    InvalidPeSubsystem(u16),
    InvalidElfHeader,
    InvalidElfProgramHeader,
    InvalidLinuxHeader,
    InvalidMultiboot2Header,
    InvalidMultiboot2Entry { expected: u64, actual: u64 },
    InvalidMultiboot2SegmentCount { expected: usize, actual: usize },
    InvalidFirmwareVolume,
    InvalidBiosImage,
    InvalidUefiImage,
    InvalidOpenSbiPayload,
    InvalidFat12Name,
    Fat12ImageTooLarge,
    InvalidGptImage,
    InvalidFat32Image,
    Fat32ImageTooLarge,
    SegmentOutsideImage,
    SegmentOutsideGuestMemory,
    SegmentFileLargerThanMemory,
    OverlappingLoadSegments,
    ArithmeticOverflow,
}

impl fmt::Display for GuestBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageTooSmall { required, actual } => {
                write!(f, "image needs at least {required} bytes, got {actual}")
            }
            Self::UnsupportedImage => write!(f, "unsupported guest image format"),
            Self::UnsupportedArchitecture(machine) => {
                write!(f, "unsupported guest machine {machine:#x}")
            }
            Self::UnsupportedEndianness => {
                write!(f, "only little-endian guest images are supported")
            }
            Self::InvalidDosHeader => write!(f, "invalid DOS MZ header"),
            Self::InvalidPeHeader => write!(f, "invalid PE/COFF header"),
            Self::InvalidPeOptionalHeader => write!(f, "invalid PE optional header"),
            Self::InvalidPeSubsystem(subsystem) => {
                write!(f, "PE subsystem {subsystem} is not an EFI image")
            }
            Self::InvalidElfHeader => write!(f, "invalid ELF header"),
            Self::InvalidElfProgramHeader => write!(f, "invalid ELF load program header"),
            Self::InvalidLinuxHeader => write!(f, "invalid Linux x86 boot protocol header"),
            Self::InvalidMultiboot2Header => write!(f, "invalid Multiboot2 header"),
            Self::InvalidMultiboot2Entry { expected, actual } => write!(
                f,
                "invalid Multiboot2 entry: expected {expected:#x}, got {actual:#x}"
            ),
            Self::InvalidMultiboot2SegmentCount { expected, actual } => write!(
                f,
                "invalid Multiboot2 segment count: expected {expected}, got {actual}"
            ),
            Self::InvalidFirmwareVolume => write!(f, "invalid UEFI firmware volume"),
            Self::InvalidBiosImage => write!(f, "invalid legacy BIOS image"),
            Self::InvalidUefiImage => write!(f, "invalid UEFI application image"),
            Self::InvalidOpenSbiPayload => write!(f, "invalid OpenSBI next-stage payload"),
            Self::InvalidFat12Name => write!(f, "invalid FAT12 8.3 name"),
            Self::Fat12ImageTooLarge => write!(f, "EFI application does not fit in FAT12 image"),
            Self::InvalidGptImage => write!(f, "invalid GPT disk image"),
            Self::InvalidFat32Image => write!(f, "invalid FAT32 EFI system partition"),
            Self::Fat32ImageTooLarge => {
                write!(f, "EFI application exceeds the FAT32/GPT image limits")
            }
            Self::SegmentOutsideImage => write!(f, "load segment points outside the source image"),
            Self::SegmentOutsideGuestMemory => {
                write!(f, "load segment points outside guest physical memory")
            }
            Self::SegmentFileLargerThanMemory => {
                write!(f, "load segment file size exceeds memory size")
            }
            Self::OverlappingLoadSegments => write!(f, "guest load segments overlap"),
            Self::ArithmeticOverflow => write!(f, "guest boot address arithmetic overflow"),
        }
    }
}

impl std::error::Error for GuestBootError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestFirmwareKind {
    LegacyBios,
    UefiPi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestImageKind {
    BiosBootDisk,
    UefiApplication,
    Elf32,
    Elf64,
    Multiboot2,
    LinuxBzImage,
    RawBinary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestBootProtocol {
    BiosInt19,
    UefiLoadImage,
    ElfDirect,
    Multiboot2,
    LinuxBootProtocol,
    RawDirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestCpuMode {
    Real16,
    Protected32,
    Long64,
    Aarch64,
    RiscV32,
    RiscV64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestSegmentFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl GuestSegmentFlags {
    pub const READ_EXECUTE: Self = Self {
        read: true,
        write: false,
        execute: true,
    };

    pub const READ_WRITE: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestLoadSegment {
    pub source_offset: usize,
    pub file_size: usize,
    pub memory_size: usize,
    pub guest_physical_address: u64,
    pub flags: GuestSegmentFlags,
}

impl GuestLoadSegment {
    pub fn end_address(&self) -> Result<u64, GuestBootError> {
        self.guest_physical_address
            .checked_add(self.memory_size as u64)
            .ok_or(GuestBootError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestLoadPlan {
    pub image_kind: GuestImageKind,
    pub protocol: GuestBootProtocol,
    pub cpu_mode: GuestCpuMode,
    pub entry_point: u64,
    pub segments: Vec<GuestLoadSegment>,
    pub linux_protocol_version: Option<u16>,
    pub multiboot2_header_offset: Option<usize>,
}

impl GuestLoadPlan {
    pub fn parse(image: &[u8]) -> Result<Self, GuestBootError> {
        match detect_guest_image(image)? {
            GuestImageKind::UefiApplication => parse_uefi_application(image),
            GuestImageKind::Elf32 | GuestImageKind::Elf64 => parse_elf(image, None),
            GuestImageKind::Multiboot2 => parse_multiboot2(image),
            GuestImageKind::LinuxBzImage => parse_linux_bzimage(image),
            GuestImageKind::BiosBootDisk => Ok(Self {
                image_kind: GuestImageKind::BiosBootDisk,
                protocol: GuestBootProtocol::BiosInt19,
                cpu_mode: GuestCpuMode::Real16,
                entry_point: BIOS_BOOT_LOAD_ADDRESS,
                segments: vec![GuestLoadSegment {
                    source_offset: 0,
                    file_size: BIOS_BOOT_SECTOR_BYTES,
                    memory_size: BIOS_BOOT_SECTOR_BYTES,
                    guest_physical_address: BIOS_BOOT_LOAD_ADDRESS,
                    flags: GuestSegmentFlags::READ_EXECUTE,
                }],
                linux_protocol_version: None,
                multiboot2_header_offset: None,
            }),
            GuestImageKind::RawBinary => Ok(Self {
                image_kind: GuestImageKind::RawBinary,
                protocol: GuestBootProtocol::RawDirect,
                cpu_mode: GuestCpuMode::Real16,
                entry_point: BIOS_BOOT_LOAD_ADDRESS,
                segments: vec![GuestLoadSegment {
                    source_offset: 0,
                    file_size: image.len(),
                    memory_size: image.len(),
                    guest_physical_address: BIOS_BOOT_LOAD_ADDRESS,
                    flags: GuestSegmentFlags::READ_EXECUTE,
                }],
                linux_protocol_version: None,
                multiboot2_header_offset: None,
            }),
        }
    }

    pub fn validate(&self, image_len: usize) -> Result<(), GuestBootError> {
        for segment in &self.segments {
            if segment.file_size > segment.memory_size {
                return Err(GuestBootError::SegmentFileLargerThanMemory);
            }
            let source_end = segment
                .source_offset
                .checked_add(segment.file_size)
                .ok_or(GuestBootError::ArithmeticOverflow)?;
            if source_end > image_len {
                return Err(GuestBootError::SegmentOutsideImage);
            }
            segment.end_address()?;
        }
        for left in 0..self.segments.len() {
            for right in left + 1..self.segments.len() {
                let left_start = self.segments[left].guest_physical_address;
                let left_end = self.segments[left].end_address()?;
                let right_start = self.segments[right].guest_physical_address;
                let right_end = self.segments[right].end_address()?;
                if left_start < right_end && right_start < left_end {
                    return Err(GuestBootError::OverlappingLoadSegments);
                }
            }
        }
        Ok(())
    }

    pub fn materialize(&self, image: &[u8], guest_memory: &mut [u8]) -> Result<(), GuestBootError> {
        self.validate(image.len())?;
        for segment in &self.segments {
            let start = usize::try_from(segment.guest_physical_address)
                .map_err(|_| GuestBootError::SegmentOutsideGuestMemory)?;
            let end = start
                .checked_add(segment.memory_size)
                .ok_or(GuestBootError::ArithmeticOverflow)?;
            let target = guest_memory
                .get_mut(start..end)
                .ok_or(GuestBootError::SegmentOutsideGuestMemory)?;
            target.fill(0);
            let source_end = segment.source_offset + segment.file_size;
            target[..segment.file_size].copy_from_slice(&image[segment.source_offset..source_end]);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSbiPayload {
    pub load_address: u64,
    pub entry_point: u64,
    pub entry_trampoline: Option<u32>,
    pub bytes: Vec<u8>,
}

pub fn build_opensbi_payload(kernel_elf: &[u8]) -> Result<OpenSbiPayload, GuestBootError> {
    let plan = GuestLoadPlan::parse(kernel_elf)?;
    if plan.protocol != GuestBootProtocol::ElfDirect
        || !matches!(plan.cpu_mode, GuestCpuMode::RiscV32 | GuestCpuMode::RiscV64)
    {
        return Err(GuestBootError::InvalidOpenSbiPayload);
    }
    let load_address = plan
        .segments
        .iter()
        .map(|segment| segment.guest_physical_address)
        .min()
        .ok_or(GuestBootError::InvalidOpenSbiPayload)?;
    let end_address = plan
        .segments
        .iter()
        .map(GuestLoadSegment::end_address)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or(GuestBootError::InvalidOpenSbiPayload)?;
    if plan.entry_point < load_address || plan.entry_point >= end_address {
        return Err(GuestBootError::InvalidOpenSbiPayload);
    }
    let payload_size = usize::try_from(
        end_address
            .checked_sub(load_address)
            .ok_or(GuestBootError::ArithmeticOverflow)?,
    )
    .map_err(|_| GuestBootError::Fat32ImageTooLarge)?;
    let mut bytes = vec![0_u8; payload_size];
    for segment in &plan.segments {
        let destination = usize::try_from(segment.guest_physical_address - load_address)
            .map_err(|_| GuestBootError::InvalidOpenSbiPayload)?;
        let destination_end = destination
            .checked_add(segment.file_size)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let source_end = segment
            .source_offset
            .checked_add(segment.file_size)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        bytes
            .get_mut(destination..destination_end)
            .ok_or(GuestBootError::InvalidOpenSbiPayload)?
            .copy_from_slice(
                kernel_elf
                    .get(segment.source_offset..source_end)
                    .ok_or(GuestBootError::SegmentOutsideImage)?,
            );
    }
    let entry_delta = i64::try_from(plan.entry_point - load_address)
        .map_err(|_| GuestBootError::InvalidOpenSbiPayload)?;
    let entry_trampoline = if entry_delta == 0 {
        None
    } else {
        if bytes.len() < 4
            || entry_delta & 1 != 0
            || !(-1_048_576..=1_048_574).contains(&entry_delta)
        {
            return Err(GuestBootError::InvalidOpenSbiPayload);
        }
        let immediate = entry_delta as i32 as u32;
        let instruction = ((immediate >> 20) & 0x1) << 31
            | ((immediate >> 1) & 0x3ff) << 21
            | ((immediate >> 11) & 0x1) << 20
            | ((immediate >> 12) & 0xff) << 12
            | 0x6f;
        bytes[..4].copy_from_slice(&instruction.to_le_bytes());
        Some(instruction)
    };
    Ok(OpenSbiPayload {
        load_address,
        entry_point: plan.entry_point,
        entry_trampoline,
        bytes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestFirmwareDescriptor {
    pub kind: GuestFirmwareKind,
    pub image_bytes: usize,
    pub guest_physical_base: u64,
    pub reset_vector: u64,
    pub firmware_volume_offset: Option<usize>,
    pub firmware_volume_bytes: Option<u64>,
}

impl GuestFirmwareDescriptor {
    pub fn parse_bios(image: &[u8]) -> Result<Self, GuestBootError> {
        if image.len() < 0x1_0000 || image.len() > 0x10_0000 || image.len() % 0x1000 != 0 {
            return Err(GuestBootError::InvalidBiosImage);
        }
        let base = 0x10_0000_u64
            .checked_sub(image.len() as u64)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        Ok(Self {
            kind: GuestFirmwareKind::LegacyBios,
            image_bytes: image.len(),
            guest_physical_base: base,
            reset_vector: BIOS_RESET_VECTOR,
            firmware_volume_offset: None,
            firmware_volume_bytes: None,
        })
    }

    pub fn parse_uefi(image: &[u8]) -> Result<Self, GuestBootError> {
        if image.len() < 0x1000 || image.len() % 0x1000 != 0 {
            return Err(GuestBootError::InvalidUefiImage);
        }
        let mut volume = None;
        for offset in (0..image.len().saturating_sub(0x38)).step_by(8) {
            if image.get(offset + 0x28..offset + 0x2c) == Some(b"_FVH") {
                let length = read_u64(image, offset + 0x20)?;
                if length < 0x38 || offset as u64 + length > image.len() as u64 {
                    return Err(GuestBootError::InvalidFirmwareVolume);
                }
                volume = Some((offset, length));
                break;
            }
        }
        let (volume_offset, volume_bytes) = volume.ok_or(GuestBootError::InvalidFirmwareVolume)?;
        let image_bytes = align_up_usize(image.len(), 0x1000)?;
        let base = UEFI_FIRMWARE_TOP
            .checked_sub(image_bytes as u64)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        Ok(Self {
            kind: GuestFirmwareKind::UefiPi,
            image_bytes: image.len(),
            guest_physical_base: base,
            reset_vector: BIOS_RESET_VECTOR,
            firmware_volume_offset: Some(volume_offset),
            firmware_volume_bytes: Some(volume_bytes),
        })
    }
}

pub fn detect_guest_image(image: &[u8]) -> Result<GuestImageKind, GuestBootError> {
    if image.len() >= 0x206 && image.get(0x202..0x206) == Some(b"HdrS") {
        return Ok(GuestImageKind::LinuxBzImage);
    }
    if find_multiboot2_header(image).is_some() {
        return Ok(GuestImageKind::Multiboot2);
    }
    if image.get(0..4) == Some(b"\x7fELF") {
        return match image.get(4).copied() {
            Some(1) => Ok(GuestImageKind::Elf32),
            Some(2) => Ok(GuestImageKind::Elf64),
            _ => Err(GuestBootError::InvalidElfHeader),
        };
    }
    if image.get(0..2) == Some(b"MZ") {
        parse_uefi_application(image)?;
        return Ok(GuestImageKind::UefiApplication);
    }
    if image.len() >= BIOS_BOOT_SECTOR_BYTES
        && image.get(BIOS_BOOT_SIGNATURE_OFFSET..BIOS_BOOT_SECTOR_BYTES) == Some(&[0x55, 0xaa])
    {
        return Ok(GuestImageKind::BiosBootDisk);
    }
    if image.is_empty() {
        Err(GuestBootError::UnsupportedImage)
    } else {
        Ok(GuestImageKind::RawBinary)
    }
}

fn parse_elf(
    image: &[u8],
    multiboot2_header_offset: Option<usize>,
) -> Result<GuestLoadPlan, GuestBootError> {
    if image.len() < 52 || image.get(0..4) != Some(b"\x7fELF") {
        return Err(GuestBootError::InvalidElfHeader);
    }
    if image[5] != 1 {
        return Err(GuestBootError::UnsupportedEndianness);
    }
    let class = image[4];
    let (machine, entry, phoff, phentsize, phnum, kind) = match class {
        1 => (
            read_u16(image, 18)?,
            read_u32(image, 24)? as u64,
            read_u32(image, 28)? as u64,
            read_u16(image, 42)? as usize,
            read_u16(image, 44)? as usize,
            GuestImageKind::Elf32,
        ),
        2 if image.len() >= 64 => (
            read_u16(image, 18)?,
            read_u64(image, 24)?,
            read_u64(image, 32)?,
            read_u16(image, 54)? as usize,
            read_u16(image, 56)? as usize,
            GuestImageKind::Elf64,
        ),
        _ => return Err(GuestBootError::InvalidElfHeader),
    };
    let mode = match (class, machine) {
        (1, 3) => GuestCpuMode::Protected32,
        (2, 62) => GuestCpuMode::Long64,
        (2, 183) => GuestCpuMode::Aarch64,
        (1, 243) => GuestCpuMode::RiscV32,
        (2, 243) => GuestCpuMode::RiscV64,
        _ => return Err(GuestBootError::UnsupportedArchitecture(machine)),
    };
    let expected_size = if class == 1 { 32 } else { 56 };
    if phentsize < expected_size {
        return Err(GuestBootError::InvalidElfProgramHeader);
    }
    let mut segments = Vec::new();
    for index in 0..phnum {
        let offset = usize::try_from(phoff)
            .ok()
            .and_then(|base| base.checked_add(index.checked_mul(phentsize)?))
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        if read_u32(image, offset)? != 1 {
            continue;
        }
        let (flags, source_offset, virtual_address, physical_address, file_size, memory_size) =
            if class == 1 {
                (
                    read_u32(image, offset + 24)?,
                    read_u32(image, offset + 4)? as u64,
                    read_u32(image, offset + 8)? as u64,
                    read_u32(image, offset + 12)? as u64,
                    read_u32(image, offset + 16)? as u64,
                    read_u32(image, offset + 20)? as u64,
                )
            } else {
                (
                    read_u32(image, offset + 4)?,
                    read_u64(image, offset + 8)?,
                    read_u64(image, offset + 16)?,
                    read_u64(image, offset + 24)?,
                    read_u64(image, offset + 32)?,
                    read_u64(image, offset + 40)?,
                )
            };
        let source_offset =
            usize::try_from(source_offset).map_err(|_| GuestBootError::InvalidElfProgramHeader)?;
        let file_size =
            usize::try_from(file_size).map_err(|_| GuestBootError::InvalidElfProgramHeader)?;
        let memory_size =
            usize::try_from(memory_size).map_err(|_| GuestBootError::InvalidElfProgramHeader)?;
        segments.push(GuestLoadSegment {
            source_offset,
            file_size,
            memory_size,
            guest_physical_address: if physical_address == 0 {
                virtual_address
            } else {
                physical_address
            },
            flags: GuestSegmentFlags {
                read: flags & 4 != 0,
                write: flags & 2 != 0,
                execute: flags & 1 != 0,
            },
        });
    }
    if segments.is_empty() {
        return Err(GuestBootError::InvalidElfProgramHeader);
    }
    let multiboot = multiboot2_header_offset.is_some();
    let plan = GuestLoadPlan {
        image_kind: if multiboot {
            GuestImageKind::Multiboot2
        } else {
            kind
        },
        protocol: if multiboot {
            GuestBootProtocol::Multiboot2
        } else {
            GuestBootProtocol::ElfDirect
        },
        cpu_mode: mode,
        entry_point: entry,
        segments,
        linux_protocol_version: None,
        multiboot2_header_offset,
    };
    plan.validate(image.len())?;
    Ok(plan)
}

fn parse_uefi_application(image: &[u8]) -> Result<GuestLoadPlan, GuestBootError> {
    if image.len() < 0x40 || image.get(0..2) != Some(b"MZ") {
        return Err(GuestBootError::InvalidDosHeader);
    }
    let pe_offset = read_u32(image, 0x3c)? as usize;
    if image.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0") {
        return Err(GuestBootError::InvalidPeHeader);
    }
    let machine = read_u16(image, pe_offset + 4)?;
    let sections = read_u16(image, pe_offset + 6)? as usize;
    let optional_size = read_u16(image, pe_offset + 20)? as usize;
    let optional = pe_offset + 24;
    let magic = read_u16(image, optional)?;
    let (mode, image_base, subsystem_offset) = match (machine, magic) {
        (0x8664, 0x20b) => (GuestCpuMode::Long64, read_u64(image, optional + 24)?, 68),
        (0xaa64, 0x20b) => (GuestCpuMode::Aarch64, read_u64(image, optional + 24)?, 68),
        (0x014c, 0x10b) => (
            GuestCpuMode::Protected32,
            read_u32(image, optional + 28)? as u64,
            68,
        ),
        _ => return Err(GuestBootError::UnsupportedArchitecture(machine)),
    };
    if optional_size < subsystem_offset + 2 {
        return Err(GuestBootError::InvalidPeOptionalHeader);
    }
    let subsystem = read_u16(image, optional + subsystem_offset)?;
    if !matches!(subsystem, 10..=12) {
        return Err(GuestBootError::InvalidPeSubsystem(subsystem));
    }
    let entry = image_base
        .checked_add(read_u32(image, optional + 16)? as u64)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let section_table = optional
        .checked_add(optional_size)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let mut load_segments = Vec::new();
    for index in 0..sections {
        let header = section_table
            .checked_add(
                index
                    .checked_mul(40)
                    .ok_or(GuestBootError::ArithmeticOverflow)?,
            )
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let virtual_size = read_u32(image, header + 8)? as usize;
        let virtual_address = read_u32(image, header + 12)? as u64;
        let raw_size = read_u32(image, header + 16)? as usize;
        let raw_offset = read_u32(image, header + 20)? as usize;
        let characteristics = read_u32(image, header + 36)?;
        if virtual_size == 0 && raw_size == 0 {
            continue;
        }
        load_segments.push(GuestLoadSegment {
            source_offset: raw_offset,
            file_size: raw_size,
            memory_size: virtual_size.max(raw_size),
            guest_physical_address: image_base
                .checked_add(virtual_address)
                .ok_or(GuestBootError::ArithmeticOverflow)?,
            flags: GuestSegmentFlags {
                read: characteristics & 0x4000_0000 != 0,
                write: characteristics & 0x8000_0000 != 0,
                execute: characteristics & 0x2000_0000 != 0,
            },
        });
    }
    if load_segments.is_empty() {
        return Err(GuestBootError::InvalidPeHeader);
    }
    let plan = GuestLoadPlan {
        image_kind: GuestImageKind::UefiApplication,
        protocol: GuestBootProtocol::UefiLoadImage,
        cpu_mode: mode,
        entry_point: entry,
        segments: load_segments,
        linux_protocol_version: None,
        multiboot2_header_offset: None,
    };
    plan.validate(image.len())?;
    Ok(plan)
}

fn parse_linux_bzimage(image: &[u8]) -> Result<GuestLoadPlan, GuestBootError> {
    if image.len() < 0x230 || image.get(0x202..0x206) != Some(b"HdrS") {
        return Err(GuestBootError::InvalidLinuxHeader);
    }
    let protocol = read_u16(image, 0x206)?;
    if protocol < 0x0200 {
        return Err(GuestBootError::InvalidLinuxHeader);
    }
    let setup_sectors = match image[0x1f1] {
        0 => 4,
        value => value as usize,
    };
    let setup_bytes = (setup_sectors + 1)
        .checked_mul(512)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    if setup_bytes >= image.len() {
        return Err(GuestBootError::InvalidLinuxHeader);
    }
    let protected_address = if protocol >= 0x020a {
        let pref_address = read_u64(image, 0x258).unwrap_or(0);
        if pref_address == 0 {
            LINUX_PROTECTED_LOAD_ADDRESS
        } else {
            pref_address
        }
    } else {
        LINUX_PROTECTED_LOAD_ADDRESS
    };
    let plan = GuestLoadPlan {
        image_kind: GuestImageKind::LinuxBzImage,
        protocol: GuestBootProtocol::LinuxBootProtocol,
        cpu_mode: GuestCpuMode::Real16,
        entry_point: LINUX_SETUP_LOAD_ADDRESS + 0x200,
        segments: vec![
            GuestLoadSegment {
                source_offset: 0,
                file_size: setup_bytes,
                memory_size: setup_bytes,
                guest_physical_address: LINUX_SETUP_LOAD_ADDRESS,
                flags: GuestSegmentFlags::READ_WRITE,
            },
            GuestLoadSegment {
                source_offset: setup_bytes,
                file_size: image.len() - setup_bytes,
                memory_size: image.len() - setup_bytes,
                guest_physical_address: protected_address,
                flags: GuestSegmentFlags::READ_EXECUTE,
            },
        ],
        linux_protocol_version: Some(protocol),
        multiboot2_header_offset: None,
    };
    plan.validate(image.len())?;
    Ok(plan)
}

fn parse_multiboot2(image: &[u8]) -> Result<GuestLoadPlan, GuestBootError> {
    let offset = find_multiboot2_header(image).ok_or(GuestBootError::InvalidMultiboot2Header)?;
    let length = read_u32(image, offset + 8)? as usize;
    let end = offset
        .checked_add(length)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    if length < 24 || end > image.len() {
        return Err(GuestBootError::InvalidMultiboot2Header);
    }
    let mut cursor = offset + 16;
    let mut address_tag = None;
    let mut entry_tag = None;
    let mut end_seen = false;
    while cursor + 8 <= end {
        let tag_type = read_u16(image, cursor)?;
        let size = read_u32(image, cursor + 4)? as usize;
        if size < 8 || cursor + size > end {
            return Err(GuestBootError::InvalidMultiboot2Header);
        }
        match tag_type {
            0 => {
                if size != 8 {
                    return Err(GuestBootError::InvalidMultiboot2Header);
                }
                end_seen = true;
                break;
            }
            2 if size >= 24 => {
                address_tag = Some((
                    read_u32(image, cursor + 8)? as u64,
                    read_u32(image, cursor + 12)? as u64,
                    read_u32(image, cursor + 16)? as u64,
                    read_u32(image, cursor + 20)? as u64,
                ));
            }
            3 if size >= 12 => entry_tag = Some(read_u32(image, cursor + 8)? as u64),
            _ => {}
        }
        cursor = align_up_usize(cursor + size, 8)?;
    }
    if !end_seen {
        return Err(GuestBootError::InvalidMultiboot2Header);
    }
    if image.get(0..4) == Some(b"\x7fELF") {
        let mut plan = parse_elf(image, Some(offset))?;
        plan.cpu_mode = GuestCpuMode::Protected32;
        if let Some(entry) = entry_tag {
            plan.entry_point = entry;
        }
        plan.validate(image.len())?;
        return Ok(plan);
    }
    let (header_address, load_address, load_end, bss_end) =
        address_tag.ok_or(GuestBootError::InvalidMultiboot2Header)?;
    if header_address < load_address || (offset as u64) < header_address - load_address {
        return Err(GuestBootError::InvalidMultiboot2Header);
    }
    let source_offset = offset - (header_address - load_address) as usize;
    let file_end = if load_end == 0 {
        image.len()
    } else {
        source_offset
            .checked_add((load_end - load_address) as usize)
            .ok_or(GuestBootError::ArithmeticOverflow)?
    };
    let memory_end = if bss_end == 0 { load_end } else { bss_end };
    let file_size = file_end
        .checked_sub(source_offset)
        .ok_or(GuestBootError::InvalidMultiboot2Header)?;
    let memory_size = if memory_end == 0 {
        file_size
    } else {
        usize::try_from(memory_end - load_address)
            .map_err(|_| GuestBootError::InvalidMultiboot2Header)?
    };
    let plan = GuestLoadPlan {
        image_kind: GuestImageKind::Multiboot2,
        protocol: GuestBootProtocol::Multiboot2,
        cpu_mode: GuestCpuMode::Protected32,
        entry_point: entry_tag.unwrap_or(load_address),
        segments: vec![GuestLoadSegment {
            source_offset,
            file_size,
            memory_size,
            guest_physical_address: load_address,
            flags: GuestSegmentFlags {
                read: true,
                write: true,
                execute: true,
            },
        }],
        linux_protocol_version: None,
        multiboot2_header_offset: Some(offset),
    };
    plan.validate(image.len())?;
    Ok(plan)
}

pub fn find_multiboot2_header(image: &[u8]) -> Option<usize> {
    let limit = image.len().min(32 * 1024);
    for offset in (0..limit.saturating_sub(16)).step_by(8) {
        if read_u32(image, offset).ok()? != MULTIBOOT2_HEADER_MAGIC {
            continue;
        }
        let architecture = read_u32(image, offset + 4).ok()?;
        let length = read_u32(image, offset + 8).ok()?;
        let checksum = read_u32(image, offset + 12).ok()?;
        if architecture == 0
            && length >= 24
            && offset.checked_add(length as usize)? <= image.len()
            && MULTIBOOT2_HEADER_MAGIC
                .wrapping_add(architecture)
                .wrapping_add(length)
                .wrapping_add(checksum)
                == 0
        {
            return Some(offset);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBootParameters {
    pub zero_page: Vec<u8>,
    pub command_line: Vec<u8>,
    pub command_line_address: u32,
    pub initrd_address: Option<u32>,
    pub initrd_size: u32,
}

impl LinuxBootParameters {
    pub fn build(
        setup_header: &[u8],
        command_line: &str,
        command_line_address: u32,
        initrd: Option<(u32, usize)>,
    ) -> Result<Self, GuestBootError> {
        if setup_header.len() < 0x230 || setup_header.get(0x202..0x206) != Some(b"HdrS") {
            return Err(GuestBootError::InvalidLinuxHeader);
        }
        let mut zero_page = vec![0_u8; 4096];
        let copy = setup_header.len().min(zero_page.len());
        zero_page[..copy].copy_from_slice(&setup_header[..copy]);
        zero_page[0x210] = 0xff;
        zero_page[0x211] |= 0x80;
        zero_page[0x228..0x22c].copy_from_slice(&command_line_address.to_le_bytes());
        let (initrd_address, initrd_size) = if let Some((address, size)) = initrd {
            let size = u32::try_from(size).map_err(|_| GuestBootError::ArithmeticOverflow)?;
            zero_page[0x218..0x21c].copy_from_slice(&address.to_le_bytes());
            zero_page[0x21c..0x220].copy_from_slice(&size.to_le_bytes());
            (Some(address), size)
        } else {
            (None, 0)
        };
        let mut command_line = command_line.as_bytes().to_vec();
        command_line.push(0);
        Ok(Self {
            zero_page,
            command_line,
            command_line_address,
            initrd_address,
            initrd_size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Multiboot2MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub kind: u32,
}

pub fn build_multiboot2_information(
    command_line: Option<&str>,
    memory_map: &[Multiboot2MemoryMapEntry],
) -> Result<Vec<u8>, GuestBootError> {
    let mut bytes = vec![0_u8; 8];
    if let Some(command_line) = command_line {
        let mut payload = command_line.as_bytes().to_vec();
        payload.push(0);
        push_multiboot2_tag(&mut bytes, 1, &payload)?;
    }
    let mut map = Vec::with_capacity(8 + memory_map.len() * 24);
    map.extend_from_slice(&24_u32.to_le_bytes());
    map.extend_from_slice(&0_u32.to_le_bytes());
    for entry in memory_map {
        map.extend_from_slice(&entry.base.to_le_bytes());
        map.extend_from_slice(&entry.length.to_le_bytes());
        map.extend_from_slice(&entry.kind.to_le_bytes());
        map.extend_from_slice(&0_u32.to_le_bytes());
    }
    push_multiboot2_tag(&mut bytes, 6, &map)?;
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    let total = bytes.len() as u32;
    bytes[0..4].copy_from_slice(&total.to_le_bytes());
    Ok(bytes)
}

fn push_multiboot2_tag(
    bytes: &mut Vec<u8>,
    tag_type: u32,
    payload: &[u8],
) -> Result<(), GuestBootError> {
    let size = 8_usize
        .checked_add(payload.len())
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    bytes.extend_from_slice(&tag_type.to_le_bytes());
    bytes.extend_from_slice(&(size as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    while bytes.len() & 7 != 0 {
        bytes.push(0);
    }
    Ok(())
}

pub fn build_bios_boot_sector(code: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    if code.len() > BIOS_BOOT_SIGNATURE_OFFSET {
        return Err(GuestBootError::InvalidBiosImage);
    }
    let mut sector = vec![0_u8; BIOS_BOOT_SECTOR_BYTES];
    sector[..code.len()].copy_from_slice(code);
    sector[BIOS_BOOT_SIGNATURE_OFFSET] = 0x55;
    sector[BIOS_BOOT_SIGNATURE_OFFSET + 1] = 0xaa;
    Ok(sector)
}

pub fn build_bios_disk_image(boot_sector: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    if boot_sector.len() != BIOS_BOOT_SECTOR_BYTES
        || boot_sector[BIOS_BOOT_SIGNATURE_OFFSET..] != [0x55, 0xaa]
    {
        return Err(GuestBootError::InvalidBiosImage);
    }
    let mut disk = vec![0_u8; FAT12_DISK_BYTES];
    disk[..BIOS_BOOT_SECTOR_BYTES].copy_from_slice(boot_sector);
    Ok(disk)
}

pub fn build_x86_64_uefi_application(code: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    if code.is_empty() || code.len() > 0x1000 {
        return Err(GuestBootError::InvalidUefiImage);
    }
    let headers = 0x200_usize;
    let raw_size = align_up_usize(code.len(), 0x200)?.max(0x200);
    let mut image = vec![0_u8; headers + raw_size];
    image[0..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
    let pe = 0x80;
    image[pe..pe + 4].copy_from_slice(b"PE\0\0");
    write_u16(&mut image, pe + 4, 0x8664)?;
    write_u16(&mut image, pe + 6, 1)?;
    write_u16(&mut image, pe + 20, 0xf0)?;
    write_u16(&mut image, pe + 22, 0x0022)?;
    let optional = pe + 24;
    write_u16(&mut image, optional, 0x20b)?;
    image[optional + 2] = 1;
    write_u32(&mut image, optional + 4, raw_size as u32)?;
    write_u32(&mut image, optional + 16, 0x1000)?;
    write_u32(&mut image, optional + 20, 0x1000)?;
    write_u64(&mut image, optional + 24, 0x0040_0000)?;
    write_u32(&mut image, optional + 32, 0x1000)?;
    write_u32(&mut image, optional + 36, 0x200)?;
    write_u16(&mut image, optional + 40, 2)?;
    write_u16(&mut image, optional + 48, 2)?;
    write_u32(&mut image, optional + 56, 0x2000)?;
    write_u32(&mut image, optional + 60, headers as u32)?;
    write_u16(&mut image, optional + 68, 10)?;
    write_u64(&mut image, optional + 72, 0x10_0000)?;
    write_u64(&mut image, optional + 80, 0x1000)?;
    write_u64(&mut image, optional + 88, 0x10_0000)?;
    write_u64(&mut image, optional + 96, 0x1000)?;
    write_u32(&mut image, optional + 108, 16)?;
    let section = optional + 0xf0;
    image[section..section + 8].copy_from_slice(b".text\0\0\0");
    write_u32(&mut image, section + 8, code.len() as u32)?;
    write_u32(&mut image, section + 12, 0x1000)?;
    write_u32(&mut image, section + 16, raw_size as u32)?;
    write_u32(&mut image, section + 20, headers as u32)?;
    write_u32(&mut image, section + 36, 0x6000_0020)?;
    image[headers..headers + code.len()].copy_from_slice(code);
    parse_uefi_application(&image)?;
    Ok(image)
}

pub fn build_aarch64_uefi_application(code: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    let mut image = build_x86_64_uefi_application(code)?;
    let pe = read_u32(&image, 0x3c)? as usize;
    write_u16(&mut image, pe + 4, 0xaa64)?;
    parse_uefi_application(&image)?;
    Ok(image)
}

pub fn build_ia32_uefi_application(code: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    if code.is_empty() || code.len() > 0x1000 {
        return Err(GuestBootError::InvalidUefiImage);
    }
    let headers = 0x200_usize;
    let raw_size = align_up_usize(code.len(), 0x200)?.max(0x200);
    let mut image = vec![0_u8; headers + raw_size];
    image[0..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
    let pe = 0x80;
    image[pe..pe + 4].copy_from_slice(b"PE\0\0");
    write_u16(&mut image, pe + 4, 0x014c)?;
    write_u16(&mut image, pe + 6, 1)?;
    write_u16(&mut image, pe + 20, 0xe0)?;
    write_u16(&mut image, pe + 22, 0x0102)?;
    let optional = pe + 24;
    write_u16(&mut image, optional, 0x10b)?;
    image[optional + 2] = 1;
    write_u32(&mut image, optional + 4, raw_size as u32)?;
    write_u32(&mut image, optional + 16, 0x1000)?;
    write_u32(&mut image, optional + 20, 0x1000)?;
    write_u32(&mut image, optional + 28, 0x0040_0000)?;
    write_u32(&mut image, optional + 32, 0x1000)?;
    write_u32(&mut image, optional + 36, 0x200)?;
    write_u16(&mut image, optional + 40, 2)?;
    write_u16(&mut image, optional + 48, 2)?;
    write_u32(&mut image, optional + 56, 0x2000)?;
    write_u32(&mut image, optional + 60, headers as u32)?;
    write_u16(&mut image, optional + 68, 10)?;
    write_u32(&mut image, optional + 72, 0x10_0000)?;
    write_u32(&mut image, optional + 76, 0x1000)?;
    write_u32(&mut image, optional + 80, 0x10_0000)?;
    write_u32(&mut image, optional + 84, 0x1000)?;
    write_u32(&mut image, optional + 92, 16)?;
    let section = optional + 0xe0;
    image[section..section + 8].copy_from_slice(b".text\0\0\0");
    write_u32(&mut image, section + 8, code.len() as u32)?;
    write_u32(&mut image, section + 12, 0x1000)?;
    write_u32(&mut image, section + 16, raw_size as u32)?;
    write_u32(&mut image, section + 20, headers as u32)?;
    write_u32(&mut image, section + 36, 0x6000_0020)?;
    image[headers..headers + code.len()].copy_from_slice(code);
    parse_uefi_application(&image)?;
    Ok(image)
}

pub fn build_elf64_guest_image(
    code: &[u8],
    guest_physical_address: u64,
    entry_offset: usize,
) -> Result<Vec<u8>, GuestBootError> {
    if code.is_empty() || entry_offset >= code.len() {
        return Err(GuestBootError::InvalidElfHeader);
    }
    let file_offset = 0x1000_usize;
    let mut image = vec![0_u8; file_offset + code.len()];
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    write_u16(&mut image, 16, 2)?;
    write_u16(&mut image, 18, 62)?;
    write_u32(&mut image, 20, 1)?;
    write_u64(&mut image, 24, guest_physical_address + entry_offset as u64)?;
    write_u64(&mut image, 32, 64)?;
    write_u16(&mut image, 52, 64)?;
    write_u16(&mut image, 54, 56)?;
    write_u16(&mut image, 56, 1)?;
    let program = 64;
    write_u32(&mut image, program, 1)?;
    write_u32(&mut image, program + 4, 5)?;
    write_u64(&mut image, program + 8, file_offset as u64)?;
    write_u64(&mut image, program + 16, guest_physical_address)?;
    write_u64(&mut image, program + 24, guest_physical_address)?;
    write_u64(&mut image, program + 32, code.len() as u64)?;
    write_u64(&mut image, program + 40, code.len() as u64)?;
    write_u64(&mut image, program + 48, 0x1000)?;
    image[file_offset..].copy_from_slice(code);
    parse_elf(&image, None)?;
    Ok(image)
}

pub fn build_multiboot2_elf64_guest(
    code: &[u8],
    guest_physical_address: u64,
) -> Result<Vec<u8>, GuestBootError> {
    let header_length = 24_u32;
    let checksum = 0_u32
        .wrapping_sub(MULTIBOOT2_HEADER_MAGIC)
        .wrapping_sub(header_length);
    let mut payload = Vec::with_capacity(header_length as usize + code.len());
    payload.extend_from_slice(&MULTIBOOT2_HEADER_MAGIC.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&header_length.to_le_bytes());
    payload.extend_from_slice(&checksum.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&8_u32.to_le_bytes());
    payload.extend_from_slice(code);
    build_elf64_guest_image(&payload, guest_physical_address, header_length as usize)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Multiboot2X86_64KernelImage {
    pub bytes: Vec<u8>,
    pub protected_entry: u64,
    pub efi64_entry: u64,
    pub kernel_entry: u64,
    pub stack_top: u64,
}

fn push_multiboot2_header_tag(bytes: &mut Vec<u8>, kind: u16, flags: u16, payload: &[u8]) {
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&((payload.len() + 8) as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    while bytes.len() & 7 != 0 {
        bytes.push(0);
    }
}

fn build_multiboot2_x86_64_header(protected_entry: u32, efi64_entry: u32) -> Vec<u8> {
    let mut bytes = vec![0_u8; 16];
    let mut information = Vec::new();
    for kind in [
        1_u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
    ] {
        information.extend_from_slice(&kind.to_le_bytes());
    }
    push_multiboot2_header_tag(&mut bytes, 1, 1, &information);
    push_multiboot2_header_tag(&mut bytes, 3, 0, &protected_entry.to_le_bytes());
    push_multiboot2_header_tag(&mut bytes, 4, 1, &1_u32.to_le_bytes());
    let mut framebuffer = Vec::with_capacity(12);
    framebuffer.extend_from_slice(&1024_u32.to_le_bytes());
    framebuffer.extend_from_slice(&768_u32.to_le_bytes());
    framebuffer.extend_from_slice(&32_u32.to_le_bytes());
    push_multiboot2_header_tag(&mut bytes, 5, 1, &framebuffer);
    push_multiboot2_header_tag(&mut bytes, 6, 1, &[]);
    push_multiboot2_header_tag(&mut bytes, 9, 1, &efi64_entry.to_le_bytes());
    push_multiboot2_header_tag(&mut bytes, 0, 0, &[]);
    let length = bytes.len() as u32;
    let checksum = 0_u32
        .wrapping_sub(MULTIBOOT2_HEADER_MAGIC)
        .wrapping_sub(length);
    bytes[0..4].copy_from_slice(&MULTIBOOT2_HEADER_MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&length.to_le_bytes());
    bytes[12..16].copy_from_slice(&checksum.to_le_bytes());
    bytes
}

fn patch_relative_i32(
    bytes: &mut [u8],
    displacement_offset: usize,
    target_offset: usize,
) -> Result<(), GuestBootError> {
    let origin = displacement_offset
        .checked_add(4)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let displacement = i64::try_from(target_offset)
        .map_err(|_| GuestBootError::ArithmeticOverflow)?
        - i64::try_from(origin).map_err(|_| GuestBootError::ArithmeticOverflow)?;
    let displacement =
        i32::try_from(displacement).map_err(|_| GuestBootError::ArithmeticOverflow)?;
    bytes[displacement_offset..displacement_offset + 4]
        .copy_from_slice(&displacement.to_le_bytes());
    Ok(())
}

fn append_x86_serial_message(bytes: &mut Vec<u8>, message: &[u8]) {
    bytes.extend_from_slice(&[0x66, 0xba, 0xf8, 0x03]); // mov dx, 0x3f8
    for byte in message {
        bytes.extend_from_slice(&[0xb0, *byte, 0xee]); // mov al, byte; out dx, al
    }
}

fn build_multiboot2_x86_64_code(
    kernel_entry: u64,
) -> Result<(Vec<u8>, u64, u64, u64), GuestBootError> {
    let header_probe = build_multiboot2_x86_64_header(0, 0);
    let protected_offset = align_up_usize(header_probe.len(), 16)?;
    let mut bytes = header_probe;
    bytes.resize(protected_offset, 0x90);

    let protected_entry = MULTIBOOT2_X86_64_LOADER_BASE + protected_offset as u64;
    bytes.push(0xfa); // cli
    bytes.extend_from_slice(&[0x3d]); // cmp eax, MULTIBOOT2_BOOT_MAGIC
    bytes.extend_from_slice(&MULTIBOOT2_BOOT_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x85]); // jne halt32
    let protected_bad = bytes.len();
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x20, 0xe0]); // mov eax, cr4
    bytes.push(0x0d); // or eax, CR4.PAE | OSFXSR | OSXMMEXCPT
    bytes.extend_from_slice(&0x0000_0620_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x22, 0xe0]); // mov cr4, eax
    bytes.push(0xb8); // mov eax, pml4
    bytes.extend_from_slice(&(MULTIBOOT2_X86_64_DATA_BASE as u32).to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x22, 0xd8]); // mov cr3, eax
    bytes.push(0xb9); // mov ecx, EFER
    bytes.extend_from_slice(&0xc000_0080_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x32]); // rdmsr
    bytes.push(0x0d); // or eax, EFER.LME
    bytes.extend_from_slice(&0x0000_0100_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x30]); // wrmsr
    bytes.extend_from_slice(&[0x0f, 0x01, 0x15]); // lgdt [absolute32]
    let protected_gdtr = bytes.len();
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x20, 0xc0]); // mov eax, cr0
    bytes.push(0x25); // and eax, ~(CR0.EM | CR0.TS)
    bytes.extend_from_slice(&0xffff_fff3_u32.to_le_bytes());
    bytes.push(0x0d); // or eax, CR0.PG | WP | NE | MP
    bytes.extend_from_slice(&0x8001_0022_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x22, 0xc0]); // mov cr0, eax
    bytes.push(0xea); // far jump selector:offset into 64-bit code
    let protected_long = bytes.len();
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    let halt32 = bytes.len();
    bytes.extend_from_slice(&[0xf4, 0xeb, 0xfd]);

    let efi64_offset = align_up_usize(bytes.len(), 16)?;
    bytes.resize(efi64_offset, 0x90);
    let efi64_entry = MULTIBOOT2_X86_64_LOADER_BASE + efi64_offset as u64;
    bytes.push(0xfa); // cli
    bytes.extend_from_slice(&[0x0f, 0x01, 0x15]); // lgdt [rip + disp32]
    let efi64_gdtr = bytes.len();
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.push(0xe9); // jmp long_entry
    let efi64_long = bytes.len();
    bytes.extend_from_slice(&0_i32.to_le_bytes());

    let long_offset = align_up_usize(bytes.len(), 16)?;
    bytes.resize(long_offset, 0x90);
    let long_entry = MULTIBOOT2_X86_64_LOADER_BASE + long_offset as u64;
    bytes.extend_from_slice(&[0x66, 0xb8, 0x10, 0x00]); // mov ax, data selector
    bytes.extend_from_slice(&[0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0]); // ds/es/ss
    let stack_top = MULTIBOOT2_X86_64_DATA_BASE + 6 * 4096 + MULTIBOOT2_X86_64_STACK_BYTES as u64;
    bytes.extend_from_slice(&[0x48, 0xbc]); // mov rsp, stack_top
    bytes.extend_from_slice(&stack_top.to_le_bytes());
    bytes.extend_from_slice(&[0x31, 0xed]); // xor ebp, ebp
    bytes.extend_from_slice(&[0xdb, 0xe3]); // fninit
    bytes.extend_from_slice(&[0x48, 0x89, 0xdf]); // mov rdi, rbx
    bytes.push(0xbe); // mov esi, MULTIBOOT2_BOOT_MAGIC
    bytes.extend_from_slice(&MULTIBOOT2_BOOT_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&[0x48, 0xb8]); // mov rax, kernel_entry
    bytes.extend_from_slice(&kernel_entry.to_le_bytes());
    bytes.extend_from_slice(&[0xff, 0xd0]); // call rax
    bytes.push(0x3d); // cmp eax, 'OK'
    bytes.extend_from_slice(&20_299_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x0f, 0x85]); // jne failure
    let long_bad = bytes.len();
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    append_x86_serial_message(&mut bytes, b"nC kernel Multiboot2 kmain\r\n");
    bytes.push(0xe9); // jmp park
    let long_park = bytes.len();
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    let failure = bytes.len();
    append_x86_serial_message(&mut bytes, b"nC kernel Multiboot2 kmain bad\r\n");
    let park = bytes.len();
    bytes.extend_from_slice(&[0xfa, 0xf4, 0xeb, 0xfd]);

    while bytes.len() & 7 != 0 {
        bytes.push(0);
    }
    let gdt_offset = bytes.len();
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&0x00af_9a00_0000_ffff_u64.to_le_bytes());
    bytes.extend_from_slice(&0x00cf_9200_0000_ffff_u64.to_le_bytes());
    let gdtr_offset = bytes.len();
    bytes.extend_from_slice(&23_u16.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(MULTIBOOT2_X86_64_LOADER_BASE + gdt_offset as u64)
            .map_err(|_| GuestBootError::ArithmeticOverflow)?
            .to_le_bytes(),
    );

    patch_relative_i32(&mut bytes, protected_bad, halt32)?;
    bytes[protected_gdtr..protected_gdtr + 4].copy_from_slice(
        &u32::try_from(MULTIBOOT2_X86_64_LOADER_BASE + gdtr_offset as u64)
            .map_err(|_| GuestBootError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    bytes[protected_long..protected_long + 4].copy_from_slice(
        &u32::try_from(long_entry)
            .map_err(|_| GuestBootError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    patch_relative_i32(&mut bytes, efi64_gdtr, gdtr_offset)?;
    patch_relative_i32(&mut bytes, efi64_long, long_offset)?;
    patch_relative_i32(&mut bytes, long_bad, failure)?;
    patch_relative_i32(&mut bytes, long_park, park)?;

    let header = build_multiboot2_x86_64_header(
        u32::try_from(protected_entry).map_err(|_| GuestBootError::ArithmeticOverflow)?,
        u32::try_from(efi64_entry).map_err(|_| GuestBootError::ArithmeticOverflow)?,
    );
    if header.len() > protected_offset {
        return Err(GuestBootError::InvalidMultiboot2Header);
    }
    bytes[..header.len()].copy_from_slice(&header);
    Ok((bytes, protected_entry, efi64_entry, stack_top))
}

fn build_multiboot2_identity_tables() -> Vec<u8> {
    let mut tables = vec![0_u8; 6 * 4096];
    tables[0..8].copy_from_slice(&(MULTIBOOT2_X86_64_DATA_BASE + 0x1000 | 3).to_le_bytes());
    for directory in 0..4_u64 {
        let pointer = MULTIBOOT2_X86_64_DATA_BASE + (2 + directory) * 4096 | 3;
        let offset = 4096 + directory as usize * 8;
        tables[offset..offset + 8].copy_from_slice(&pointer.to_le_bytes());
        for entry in 0..512_u64 {
            let physical = (directory * 512 + entry) * 0x20_0000;
            let value = physical | 0x83;
            let offset = (2 + directory as usize) * 4096 + entry as usize * 8;
            tables[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    tables
}

fn write_elf64_program_header(
    image: &mut [u8],
    index: usize,
    flags: u32,
    file_offset: u64,
    address: u64,
    file_size: u64,
    memory_size: u64,
) -> Result<(), GuestBootError> {
    let offset = 64_usize
        .checked_add(
            index
                .checked_mul(56)
                .ok_or(GuestBootError::ArithmeticOverflow)?,
        )
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    write_u32(image, offset, 1)?;
    write_u32(image, offset + 4, flags)?;
    write_u64(image, offset + 8, file_offset)?;
    write_u64(image, offset + 16, address)?;
    write_u64(image, offset + 24, address)?;
    write_u64(image, offset + 32, file_size)?;
    write_u64(image, offset + 40, memory_size)?;
    write_u64(image, offset + 48, 4096)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct Elf64SectionDescriptor {
    name: String,
    kind: u32,
    flags: u64,
    address: u64,
    file_offset: u64,
    size: u64,
    alignment: u64,
}

fn append_elf64_section_header(
    image: &mut Vec<u8>,
    name_offset: u32,
    section: &Elf64SectionDescriptor,
) {
    image.extend_from_slice(&name_offset.to_le_bytes());
    image.extend_from_slice(&section.kind.to_le_bytes());
    image.extend_from_slice(&section.flags.to_le_bytes());
    image.extend_from_slice(&section.address.to_le_bytes());
    image.extend_from_slice(&section.file_offset.to_le_bytes());
    image.extend_from_slice(&section.size.to_le_bytes());
    image.extend_from_slice(&0_u32.to_le_bytes());
    image.extend_from_slice(&0_u32.to_le_bytes());
    image.extend_from_slice(&section.alignment.to_le_bytes());
    image.extend_from_slice(&0_u64.to_le_bytes());
}

pub fn build_multiboot2_x86_64_kernel(
    kernel_elf: &[u8],
) -> Result<Multiboot2X86_64KernelImage, GuestBootError> {
    let kernel = parse_elf(kernel_elf, None)?;
    if kernel.image_kind != GuestImageKind::Elf64 || kernel.cpu_mode != GuestCpuMode::Long64 {
        return Err(GuestBootError::InvalidElfHeader);
    }
    let (code, protected_entry, efi64_entry, stack_top) =
        build_multiboot2_x86_64_code(kernel.entry_point)?;
    let data = build_multiboot2_identity_tables();
    let loader_end = stack_top;
    for segment in &kernel.segments {
        let end = segment
            .guest_physical_address
            .checked_add(segment.memory_size as u64)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        if end > UEFI_FIRMWARE_TOP
            || (segment.guest_physical_address < loader_end && end > MULTIBOOT2_X86_64_LOADER_BASE)
        {
            return Err(GuestBootError::OverlappingLoadSegments);
        }
    }

    let program_count = 2_usize
        .checked_add(kernel.segments.len())
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let headers_end = 64_usize
        .checked_add(
            program_count
                .checked_mul(56)
                .ok_or(GuestBootError::ArithmeticOverflow)?,
        )
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    if headers_end > 4096 {
        return Err(GuestBootError::InvalidElfProgramHeader);
    }
    let mut image = vec![0_u8; 4096];
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    write_u16(&mut image, 16, 2)?;
    write_u16(&mut image, 18, 62)?;
    write_u32(&mut image, 20, 1)?;
    write_u64(&mut image, 24, protected_entry)?;
    write_u64(&mut image, 32, 64)?;
    write_u16(&mut image, 52, 64)?;
    write_u16(&mut image, 54, 56)?;
    write_u16(
        &mut image,
        56,
        u16::try_from(program_count).map_err(|_| GuestBootError::ArithmeticOverflow)?,
    )?;

    let code_offset = image.len();
    image.extend_from_slice(&code);
    while image.len() & 4095 != 0 {
        image.push(0);
    }
    let data_offset = image.len();
    image.extend_from_slice(&data);
    let mut sections = vec![
        Elf64SectionDescriptor {
            name: ".boot.text".to_string(),
            kind: 1,
            flags: 0x2 | 0x4,
            address: MULTIBOOT2_X86_64_LOADER_BASE,
            file_offset: code_offset as u64,
            size: code.len() as u64,
            alignment: 16,
        },
        Elf64SectionDescriptor {
            name: ".boot.data".to_string(),
            kind: 1,
            flags: 0x1 | 0x2,
            address: MULTIBOOT2_X86_64_DATA_BASE,
            file_offset: data_offset as u64,
            size: data.len() as u64,
            alignment: 4096,
        },
        Elf64SectionDescriptor {
            name: ".boot.stack".to_string(),
            kind: 8,
            flags: 0x1 | 0x2,
            address: MULTIBOOT2_X86_64_DATA_BASE + data.len() as u64,
            file_offset: (data_offset + data.len()) as u64,
            size: MULTIBOOT2_X86_64_STACK_BYTES as u64,
            alignment: 16,
        },
    ];
    write_elf64_program_header(
        &mut image,
        0,
        5,
        code_offset as u64,
        MULTIBOOT2_X86_64_LOADER_BASE,
        code.len() as u64,
        code.len() as u64,
    )?;
    write_elf64_program_header(
        &mut image,
        1,
        6,
        data_offset as u64,
        MULTIBOOT2_X86_64_DATA_BASE,
        data.len() as u64,
        data.len() as u64 + MULTIBOOT2_X86_64_STACK_BYTES as u64,
    )?;

    for (index, segment) in kernel.segments.iter().enumerate() {
        while image.len() & 4095 != 0 {
            image.push(0);
        }
        let destination = image.len();
        let end = segment
            .source_offset
            .checked_add(segment.file_size)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let source = kernel_elf
            .get(segment.source_offset..end)
            .ok_or(GuestBootError::SegmentOutsideImage)?;
        image.extend_from_slice(source);
        let section_flags = 0x2
            | (if segment.flags.write { 0x1 } else { 0 })
            | (if segment.flags.execute { 0x4 } else { 0 });
        if segment.file_size != 0 {
            sections.push(Elf64SectionDescriptor {
                name: format!(".kernel.{index}"),
                kind: 1,
                flags: section_flags,
                address: segment.guest_physical_address,
                file_offset: destination as u64,
                size: segment.file_size as u64,
                alignment: 4096,
            });
        }
        if segment.memory_size > segment.file_size {
            sections.push(Elf64SectionDescriptor {
                name: format!(".kernel.{index}.bss"),
                kind: 8,
                flags: section_flags,
                address: segment.guest_physical_address + segment.file_size as u64,
                file_offset: (destination + segment.file_size) as u64,
                size: (segment.memory_size - segment.file_size) as u64,
                alignment: 16,
            });
        }
        let flags = (if segment.flags.read { 4 } else { 0 })
            | (if segment.flags.write { 2 } else { 0 })
            | (if segment.flags.execute { 1 } else { 0 });
        write_elf64_program_header(
            &mut image,
            index + 2,
            flags,
            destination as u64,
            segment.guest_physical_address,
            segment.file_size as u64,
            segment.memory_size as u64,
        )?;
    }

    let mut section_names = vec![0_u8];
    let mut section_name_offsets = Vec::with_capacity(sections.len());
    for section in &sections {
        section_name_offsets.push(
            u32::try_from(section_names.len()).map_err(|_| GuestBootError::ArithmeticOverflow)?,
        );
        section_names.extend_from_slice(section.name.as_bytes());
        section_names.push(0);
    }
    let shstrtab_name_offset =
        u32::try_from(section_names.len()).map_err(|_| GuestBootError::ArithmeticOverflow)?;
    section_names.extend_from_slice(b".shstrtab\0");
    let shstrtab_offset = image.len();
    image.extend_from_slice(&section_names);
    while image.len() & 7 != 0 {
        image.push(0);
    }
    let section_table_offset = image.len();
    image.extend_from_slice(&[0_u8; 64]);
    for (section, name_offset) in sections.iter().zip(section_name_offsets) {
        append_elf64_section_header(&mut image, name_offset, section);
    }
    append_elf64_section_header(
        &mut image,
        shstrtab_name_offset,
        &Elf64SectionDescriptor {
            name: ".shstrtab".to_string(),
            kind: 3,
            flags: 0,
            address: 0,
            file_offset: shstrtab_offset as u64,
            size: section_names.len() as u64,
            alignment: 1,
        },
    );
    let section_count = sections
        .len()
        .checked_add(2)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    write_u64(&mut image, 40, section_table_offset as u64)?;
    write_u16(&mut image, 58, 64)?;
    write_u16(
        &mut image,
        60,
        u16::try_from(section_count).map_err(|_| GuestBootError::ArithmeticOverflow)?,
    )?;
    write_u16(
        &mut image,
        62,
        u16::try_from(section_count - 1).map_err(|_| GuestBootError::ArithmeticOverflow)?,
    )?;

    let parsed = parse_multiboot2(&image)?;
    if parsed.protocol != GuestBootProtocol::Multiboot2 {
        return Err(GuestBootError::InvalidMultiboot2Header);
    }
    if parsed.entry_point != protected_entry {
        return Err(GuestBootError::InvalidMultiboot2Entry {
            expected: protected_entry,
            actual: parsed.entry_point,
        });
    }
    if parsed.segments.len() != program_count {
        return Err(GuestBootError::InvalidMultiboot2SegmentCount {
            expected: program_count,
            actual: parsed.segments.len(),
        });
    }
    Ok(Multiboot2X86_64KernelImage {
        bytes: image,
        protected_entry,
        efi64_entry,
        kernel_entry: kernel.entry_point,
        stack_top,
    })
}

pub fn build_uefi_fat12_disk(efi_application: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    build_uefi_fat12_disk_named(efi_application, b"BOOTX64 EFI")
}

pub fn build_uefi_fat12_disk_named(
    efi_application: &[u8],
    boot_file_name: &[u8; 11],
) -> Result<Vec<u8>, GuestBootError> {
    parse_uefi_application(efi_application)?;
    if !matches!(
        boot_file_name,
        b"BOOTX64 EFI" | b"BOOTAA64EFI" | b"BOOTIA32EFI"
    ) {
        return Err(GuestBootError::InvalidFat12Name);
    }
    const BYTES_PER_SECTOR: usize = 512;
    const RESERVED_SECTORS: usize = 1;
    const FAT_COUNT: usize = 2;
    const SECTORS_PER_FAT: usize = 9;
    const ROOT_ENTRIES: usize = 224;
    const ROOT_SECTORS: usize = (ROOT_ENTRIES * 32).div_ceil(BYTES_PER_SECTOR);
    const DATA_FIRST_SECTOR: usize = RESERVED_SECTORS + FAT_COUNT * SECTORS_PER_FAT + ROOT_SECTORS;
    const MAX_CLUSTERS: usize = 2880 - DATA_FIRST_SECTOR;

    let file_clusters = efi_application.len().div_ceil(BYTES_PER_SECTOR);
    if file_clusters == 0 || file_clusters + 2 > MAX_CLUSTERS {
        return Err(GuestBootError::Fat12ImageTooLarge);
    }
    let mut disk = vec![0_u8; FAT12_DISK_BYTES];
    disk[0..3].copy_from_slice(&[0xeb, 0x3c, 0x90]);
    disk[3..11].copy_from_slice(b"NCCUEFI ");
    write_u16(&mut disk, 11, BYTES_PER_SECTOR as u16)?;
    disk[13] = 1;
    write_u16(&mut disk, 14, RESERVED_SECTORS as u16)?;
    disk[16] = FAT_COUNT as u8;
    write_u16(&mut disk, 17, ROOT_ENTRIES as u16)?;
    write_u16(&mut disk, 19, 2880)?;
    disk[21] = 0xf0;
    write_u16(&mut disk, 22, SECTORS_PER_FAT as u16)?;
    write_u16(&mut disk, 24, 18)?;
    write_u16(&mut disk, 26, 2)?;
    disk[36] = 0;
    disk[38] = 0x29;
    write_u32(&mut disk, 39, 0x4e43_4355)?;
    disk[43..54].copy_from_slice(b"NCC UEFI   ");
    disk[54..62].copy_from_slice(b"FAT12   ");
    disk[510..512].copy_from_slice(&[0x55, 0xaa]);

    let fat_offset = BYTES_PER_SECTOR * RESERVED_SECTORS;
    let fat_bytes = BYTES_PER_SECTOR * SECTORS_PER_FAT;
    let mut fat = vec![0_u8; fat_bytes];
    fat[0..3].copy_from_slice(&[0xf0, 0xff, 0xff]);
    set_fat12_entry(&mut fat, 2, 0xfff)?;
    set_fat12_entry(&mut fat, 3, 0xfff)?;
    for index in 0..file_clusters {
        let cluster = 4 + index;
        let next = if index + 1 == file_clusters {
            0xfff
        } else {
            (cluster + 1) as u16
        };
        set_fat12_entry(&mut fat, cluster, next)?;
    }
    for copy in 0..FAT_COUNT {
        let start = fat_offset + copy * fat_bytes;
        disk[start..start + fat_bytes].copy_from_slice(&fat);
    }

    let root_offset = (RESERVED_SECTORS + FAT_COUNT * SECTORS_PER_FAT) * BYTES_PER_SECTOR;
    write_fat_directory_entry(
        &mut disk[root_offset..root_offset + 32],
        b"EFI        ",
        0x10,
        2,
        0,
    )?;
    let data_offset = DATA_FIRST_SECTOR * BYTES_PER_SECTOR;
    let efi_directory = data_offset;
    write_fat_directory_entry(
        &mut disk[efi_directory..efi_directory + 32],
        b".          ",
        0x10,
        2,
        0,
    )?;
    write_fat_directory_entry(
        &mut disk[efi_directory + 32..efi_directory + 64],
        b"..         ",
        0x10,
        0,
        0,
    )?;
    write_fat_directory_entry(
        &mut disk[efi_directory + 64..efi_directory + 96],
        b"BOOT       ",
        0x10,
        3,
        0,
    )?;
    let boot_directory = data_offset + BYTES_PER_SECTOR;
    write_fat_directory_entry(
        &mut disk[boot_directory..boot_directory + 32],
        b".          ",
        0x10,
        3,
        0,
    )?;
    write_fat_directory_entry(
        &mut disk[boot_directory + 32..boot_directory + 64],
        b"..         ",
        0x10,
        2,
        0,
    )?;
    write_fat_directory_entry(
        &mut disk[boot_directory + 64..boot_directory + 96],
        boot_file_name,
        0x20,
        4,
        efi_application.len() as u32,
    )?;
    let file_offset = data_offset + 2 * BYTES_PER_SECTOR;
    disk[file_offset..file_offset + efi_application.len()].copy_from_slice(efi_application);
    Ok(disk)
}

pub fn build_uefi_gpt_fat32_disk(efi_application: &[u8]) -> Result<Vec<u8>, GuestBootError> {
    build_uefi_gpt_fat32_disk_named(efi_application, b"BOOTX64 EFI")
}

pub fn build_uefi_gpt_fat32_disk_named(
    efi_application: &[u8],
    boot_file_name: &[u8; 11],
) -> Result<Vec<u8>, GuestBootError> {
    build_uefi_gpt_fat32_disk_named_with_initrd(efi_application, boot_file_name, &[])
}

pub fn build_uefi_gpt_fat32_disk_named_with_initrd(
    efi_application: &[u8],
    boot_file_name: &[u8; 11],
    initrd: &[u8],
) -> Result<Vec<u8>, GuestBootError> {
    parse_uefi_application(efi_application)?;
    if !matches!(
        boot_file_name,
        b"BOOTX64 EFI" | b"BOOTAA64EFI" | b"BOOTIA32EFI"
    ) {
        return Err(GuestBootError::InvalidFat12Name);
    }
    if efi_application.is_empty()
        || efi_application.len() > u32::MAX as usize
        || initrd.len() > u32::MAX as usize
    {
        return Err(GuestBootError::Fat32ImageTooLarge);
    }

    const SECTOR: usize = 512;
    const ALIGNMENT_SECTORS: usize = 2048;
    const PARTITION_FIRST_LBA: usize = 2048;
    const GPT_ENTRY_COUNT: usize = 128;
    const GPT_ENTRY_SIZE: usize = 128;
    const GPT_ENTRY_SECTORS: usize = (GPT_ENTRY_COUNT * GPT_ENTRY_SIZE) / SECTOR;
    const RESERVED_SECTORS: usize = 32;
    const FAT_COUNT: usize = 2;
    const SECTORS_PER_CLUSTER: usize = 1;
    const FAT32_MIN_CLUSTERS: usize = 65_525;

    let file_clusters = efi_application.len().div_ceil(SECTOR * SECTORS_PER_CLUSTER);
    let initrd_clusters = initrd.len().div_ceil(SECTOR * SECTORS_PER_CLUSTER);
    let payload_clusters = file_clusters
        .checked_add(initrd_clusters)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let required_clusters = FAT32_MIN_CLUSTERS.max(
        payload_clusters
            .checked_add(3)
            .ok_or(GuestBootError::ArithmeticOverflow)?,
    );
    let estimated_fat_sectors = required_clusters
        .checked_add(2)
        .and_then(|entries| entries.checked_mul(4))
        .ok_or(GuestBootError::ArithmeticOverflow)?
        .div_ceil(SECTOR);
    let minimum_partition_sectors = RESERVED_SECTORS
        .checked_add(
            FAT_COUNT
                .checked_mul(estimated_fat_sectors)
                .ok_or(GuestBootError::ArithmeticOverflow)?,
        )
        .and_then(|value| value.checked_add(required_clusters * SECTORS_PER_CLUSTER))
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let minimum_disk_sectors = PARTITION_FIRST_LBA
        .checked_add(minimum_partition_sectors)
        .and_then(|value| value.checked_add(GPT_ENTRY_SECTORS + 1))
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let disk_sectors = align_up_usize(
        minimum_disk_sectors.max(UEFI_GPT_MIN_DISK_BYTES / SECTOR),
        ALIGNMENT_SECTORS,
    )?;
    let disk_bytes = disk_sectors
        .checked_mul(SECTOR)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let mut disk = vec![0_u8; disk_bytes];

    let last_lba = disk_sectors - 1;
    let backup_entries_lba = last_lba - GPT_ENTRY_SECTORS;
    let partition_last_lba = backup_entries_lba - 1;
    let partition_sectors = partition_last_lba
        .checked_sub(PARTITION_FIRST_LBA)
        .and_then(|value| value.checked_add(1))
        .ok_or(GuestBootError::ArithmeticOverflow)?;

    let mut fat_sectors = 1_usize;
    loop {
        let overhead = RESERVED_SECTORS
            .checked_add(
                FAT_COUNT
                    .checked_mul(fat_sectors)
                    .ok_or(GuestBootError::ArithmeticOverflow)?,
            )
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let data_sectors = partition_sectors
            .checked_sub(overhead)
            .ok_or(GuestBootError::Fat32ImageTooLarge)?;
        let clusters = data_sectors / SECTORS_PER_CLUSTER;
        let required_fat = clusters
            .checked_add(2)
            .and_then(|entries| entries.checked_mul(4))
            .ok_or(GuestBootError::ArithmeticOverflow)?
            .div_ceil(SECTOR);
        if required_fat == fat_sectors {
            break;
        }
        fat_sectors = required_fat;
    }
    let data_first_sector = RESERVED_SECTORS
        .checked_add(
            FAT_COUNT
                .checked_mul(fat_sectors)
                .ok_or(GuestBootError::ArithmeticOverflow)?,
        )
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let cluster_count = (partition_sectors - data_first_sector) / SECTORS_PER_CLUSTER;
    if cluster_count < FAT32_MIN_CLUSTERS || payload_clusters + 3 > cluster_count {
        return Err(GuestBootError::Fat32ImageTooLarge);
    }

    let mut partition_entries = vec![0_u8; GPT_ENTRY_COUNT * GPT_ENTRY_SIZE];
    const ESP_GUID: [u8; 16] = [
        0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9,
        0x3b,
    ];
    const PARTITION_GUID: [u8; 16] = [
        0x4e, 0x43, 0x43, 0x45, 0x46, 0x49, 0x10, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ];
    const DISK_GUID: [u8; 16] = [
        0x4e, 0x43, 0x43, 0x44, 0x49, 0x53, 0x10, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ];
    partition_entries[0..16].copy_from_slice(&ESP_GUID);
    partition_entries[16..32].copy_from_slice(&PARTITION_GUID);
    partition_entries[32..40].copy_from_slice(&(PARTITION_FIRST_LBA as u64).to_le_bytes());
    partition_entries[40..48].copy_from_slice(&(partition_last_lba as u64).to_le_bytes());
    for (index, unit) in "nC EFI System".encode_utf16().enumerate() {
        partition_entries[56 + index * 2..58 + index * 2].copy_from_slice(&unit.to_le_bytes());
    }
    let entries_crc = crc32_ieee(&partition_entries);

    disk[446 + 1..446 + 4].copy_from_slice(&[0x00, 0x02, 0x00]);
    disk[446 + 4] = 0xee;
    disk[446 + 5..446 + 8].copy_from_slice(&[0xff, 0xff, 0xff]);
    disk[446 + 8..446 + 12].copy_from_slice(&1_u32.to_le_bytes());
    disk[446 + 12..446 + 16]
        .copy_from_slice(&(u32::try_from(last_lba).unwrap_or(u32::MAX)).to_le_bytes());
    disk[510..512].copy_from_slice(&[0x55, 0xaa]);

    let primary_entries_offset = 2 * SECTOR;
    disk[primary_entries_offset..primary_entries_offset + partition_entries.len()]
        .copy_from_slice(&partition_entries);
    let backup_entries_offset = backup_entries_lba * SECTOR;
    disk[backup_entries_offset..backup_entries_offset + partition_entries.len()]
        .copy_from_slice(&partition_entries);
    write_gpt_header(
        &mut disk[SECTOR..2 * SECTOR],
        1,
        last_lba as u64,
        34,
        (backup_entries_lba - 1) as u64,
        &DISK_GUID,
        2,
        GPT_ENTRY_COUNT as u32,
        GPT_ENTRY_SIZE as u32,
        entries_crc,
    )?;
    write_gpt_header(
        &mut disk[last_lba * SECTOR..(last_lba + 1) * SECTOR],
        last_lba as u64,
        1,
        34,
        (backup_entries_lba - 1) as u64,
        &DISK_GUID,
        backup_entries_lba as u64,
        GPT_ENTRY_COUNT as u32,
        GPT_ENTRY_SIZE as u32,
        entries_crc,
    )?;

    let partition_offset = PARTITION_FIRST_LBA * SECTOR;
    let boot = &mut disk[partition_offset..partition_offset + SECTOR];
    boot[0..3].copy_from_slice(&[0xeb, 0x58, 0x90]);
    boot[3..11].copy_from_slice(b"NCCUEFI ");
    boot[11..13].copy_from_slice(&(SECTOR as u16).to_le_bytes());
    boot[13] = SECTORS_PER_CLUSTER as u8;
    boot[14..16].copy_from_slice(&(RESERVED_SECTORS as u16).to_le_bytes());
    boot[16] = FAT_COUNT as u8;
    boot[21] = 0xf8;
    boot[24..26].copy_from_slice(&63_u16.to_le_bytes());
    boot[26..28].copy_from_slice(&255_u16.to_le_bytes());
    boot[28..32].copy_from_slice(&(PARTITION_FIRST_LBA as u32).to_le_bytes());
    boot[32..36].copy_from_slice(&(partition_sectors as u32).to_le_bytes());
    boot[36..40].copy_from_slice(&(fat_sectors as u32).to_le_bytes());
    boot[44..48].copy_from_slice(&2_u32.to_le_bytes());
    boot[48..50].copy_from_slice(&1_u16.to_le_bytes());
    boot[50..52].copy_from_slice(&6_u16.to_le_bytes());
    boot[64] = 0x80;
    boot[66] = 0x29;
    boot[67..71].copy_from_slice(&crc32_ieee(efi_application).to_le_bytes());
    boot[71..82].copy_from_slice(b"NCC UEFI   ");
    boot[82..90].copy_from_slice(b"FAT32   ");
    boot[510..512].copy_from_slice(&[0x55, 0xaa]);
    let backup_boot_offset = (PARTITION_FIRST_LBA + 6) * SECTOR;
    let primary_boot = disk[partition_offset..partition_offset + SECTOR].to_vec();
    disk[backup_boot_offset..backup_boot_offset + SECTOR].copy_from_slice(&primary_boot);

    let allocated_clusters = payload_clusters + 3;
    let free_clusters = cluster_count.saturating_sub(allocated_clusters) as u32;
    let next_free = u32::try_from(5 + payload_clusters).unwrap_or(u32::MAX);
    for relative_lba in [1_usize, 7] {
        let offset = (PARTITION_FIRST_LBA + relative_lba) * SECTOR;
        let fsinfo = &mut disk[offset..offset + SECTOR];
        fsinfo[0..4].copy_from_slice(&0x4161_5252_u32.to_le_bytes());
        fsinfo[484..488].copy_from_slice(&0x6141_7272_u32.to_le_bytes());
        fsinfo[488..492].copy_from_slice(&free_clusters.to_le_bytes());
        fsinfo[492..496].copy_from_slice(&next_free.to_le_bytes());
        fsinfo[508..512].copy_from_slice(&0xaa55_0000_u32.to_le_bytes());
    }

    let mut fat = vec![0_u8; fat_sectors * SECTOR];
    set_fat32_entry(&mut fat, 0, 0x0fff_fff8)?;
    set_fat32_entry(&mut fat, 1, 0xffff_ffff)?;
    for cluster in 2..=4 {
        set_fat32_entry(&mut fat, cluster, 0x0fff_ffff)?;
    }
    for index in 0..file_clusters {
        let cluster = 5 + index;
        let next = if index + 1 == file_clusters {
            0x0fff_ffff
        } else {
            (cluster + 1) as u32
        };
        set_fat32_entry(&mut fat, cluster, next)?;
    }
    let initrd_first_cluster = 5 + file_clusters;
    for index in 0..initrd_clusters {
        let cluster = initrd_first_cluster + index;
        let next = if index + 1 == initrd_clusters {
            0x0fff_ffff
        } else {
            (cluster + 1) as u32
        };
        set_fat32_entry(&mut fat, cluster, next)?;
    }
    for copy in 0..FAT_COUNT {
        let offset = (PARTITION_FIRST_LBA + RESERVED_SECTORS + copy * fat_sectors) * SECTOR;
        disk[offset..offset + fat.len()].copy_from_slice(&fat);
    }

    let data_offset = (PARTITION_FIRST_LBA + data_first_sector) * SECTOR;
    write_fat32_directory_entry(
        &mut disk[data_offset..data_offset + 32],
        b"EFI        ",
        0x10,
        3,
        0,
    )?;
    let efi_dir = data_offset + SECTOR;
    write_fat32_directory_entry(&mut disk[efi_dir..efi_dir + 32], b".          ", 0x10, 3, 0)?;
    write_fat32_directory_entry(
        &mut disk[efi_dir + 32..efi_dir + 64],
        b"..         ",
        0x10,
        2,
        0,
    )?;
    write_fat32_directory_entry(
        &mut disk[efi_dir + 64..efi_dir + 96],
        b"BOOT       ",
        0x10,
        4,
        0,
    )?;
    let boot_dir = data_offset + 2 * SECTOR;
    write_fat32_directory_entry(
        &mut disk[boot_dir..boot_dir + 32],
        b".          ",
        0x10,
        4,
        0,
    )?;
    write_fat32_directory_entry(
        &mut disk[boot_dir + 32..boot_dir + 64],
        b"..         ",
        0x10,
        3,
        0,
    )?;
    write_fat32_directory_entry(
        &mut disk[boot_dir + 64..boot_dir + 96],
        boot_file_name,
        0x20,
        5,
        efi_application.len() as u32,
    )?;
    if !initrd.is_empty() {
        write_fat32_directory_entry(
            &mut disk[boot_dir + 96..boot_dir + 128],
            b"INITRD     ",
            0x20,
            initrd_first_cluster as u32,
            initrd.len() as u32,
        )?;
    }
    let file_offset = data_offset + 3 * SECTOR;
    disk[file_offset..file_offset + efi_application.len()].copy_from_slice(efi_application);
    if !initrd.is_empty() {
        let initrd_offset = data_offset
            .checked_add((3 + file_clusters) * SECTOR)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        disk[initrd_offset..initrd_offset + initrd.len()].copy_from_slice(initrd);
    }

    let extracted = validate_uefi_gpt_fat32_disk(&disk, boot_file_name)?;
    if extracted != efi_application {
        return Err(GuestBootError::InvalidFat32Image);
    }
    Ok(disk)
}

pub fn validate_uefi_gpt_fat32_disk(
    disk: &[u8],
    boot_file_name: &[u8; 11],
) -> Result<Vec<u8>, GuestBootError> {
    const SECTOR: usize = 512;
    if disk.len() < UEFI_GPT_MIN_DISK_BYTES
        || disk.len() % SECTOR != 0
        || disk.get(510..512) != Some(&[0x55, 0xaa])
        || disk.get(450) != Some(&0xee)
    {
        return Err(GuestBootError::InvalidGptImage);
    }
    let sectors = disk.len() / SECTOR;
    let validate_header = |header: &[u8], current: u64, backup: u64| {
        if header.get(0..8) != Some(b"EFI PART")
            || read_u32(header, 8)? != 0x0001_0000
            || read_u32(header, 12)? != 92
            || read_u64(header, 24)? != current
            || read_u64(header, 32)? != backup
        {
            return Err(GuestBootError::InvalidGptImage);
        }
        let expected = read_u32(header, 16)?;
        let mut body = header[..92].to_vec();
        body[16..20].fill(0);
        if crc32_ieee(&body) != expected {
            return Err(GuestBootError::InvalidGptImage);
        }
        Ok(())
    };
    let last_lba = sectors - 1;
    let primary = &disk[SECTOR..2 * SECTOR];
    let backup = &disk[last_lba * SECTOR..(last_lba + 1) * SECTOR];
    validate_header(primary, 1, last_lba as u64)?;
    validate_header(backup, last_lba as u64, 1)?;
    let entries_lba =
        usize::try_from(read_u64(primary, 72)?).map_err(|_| GuestBootError::InvalidGptImage)?;
    let entry_count = read_u32(primary, 80)? as usize;
    let entry_size = read_u32(primary, 84)? as usize;
    let entries_bytes = entry_count
        .checked_mul(entry_size)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let entries_offset = entries_lba
        .checked_mul(SECTOR)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let entries = disk
        .get(entries_offset..entries_offset + entries_bytes)
        .ok_or(GuestBootError::InvalidGptImage)?;
    if crc32_ieee(entries) != read_u32(primary, 88)? || entry_size < 128 {
        return Err(GuestBootError::InvalidGptImage);
    }
    let partition_first =
        usize::try_from(read_u64(entries, 32)?).map_err(|_| GuestBootError::InvalidGptImage)?;
    let partition_last =
        usize::try_from(read_u64(entries, 40)?).map_err(|_| GuestBootError::InvalidGptImage)?;
    if partition_first < 34 || partition_first > partition_last || partition_last >= last_lba {
        return Err(GuestBootError::InvalidGptImage);
    }
    let partition_offset = partition_first * SECTOR;
    let boot = disk
        .get(partition_offset..partition_offset + SECTOR)
        .ok_or(GuestBootError::InvalidFat32Image)?;
    if read_u16(boot, 11)? != SECTOR as u16
        || boot[13] == 0
        || boot[16] == 0
        || read_u16(boot, 17)? != 0
        || read_u16(boot, 22)? != 0
        || boot.get(82..90) != Some(b"FAT32   ")
        || boot.get(510..512) != Some(&[0x55, 0xaa])
    {
        return Err(GuestBootError::InvalidFat32Image);
    }
    let sectors_per_cluster = boot[13] as usize;
    let reserved = read_u16(boot, 14)? as usize;
    let fats = boot[16] as usize;
    let fat_sectors = read_u32(boot, 36)? as usize;
    let root_cluster = read_u32(boot, 44)?;
    let data_sector = partition_first
        .checked_add(reserved)
        .and_then(|value| value.checked_add(fats * fat_sectors))
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    let fat_offset = (partition_first + reserved) * SECTOR;
    let fat = disk
        .get(fat_offset..fat_offset + fat_sectors * SECTOR)
        .ok_or(GuestBootError::InvalidFat32Image)?;
    let cluster_bytes = sectors_per_cluster * SECTOR;
    let cluster = |number: u32| -> Result<&[u8], GuestBootError> {
        if number < 2 {
            return Err(GuestBootError::InvalidFat32Image);
        }
        let relative = usize::try_from(number - 2)
            .map_err(|_| GuestBootError::InvalidFat32Image)?
            .checked_mul(sectors_per_cluster)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let offset = data_sector
            .checked_add(relative)
            .and_then(|sector| sector.checked_mul(SECTOR))
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        disk.get(offset..offset + cluster_bytes)
            .ok_or(GuestBootError::InvalidFat32Image)
    };
    let find = |directory: &[u8], name: &[u8; 11]| -> Result<(u32, u32, u8), GuestBootError> {
        for entry in directory.chunks_exact(32) {
            if entry[0] == 0 {
                break;
            }
            if entry[0] != 0xe5 && entry[11] != 0x0f && &entry[..11] == name {
                let high = u32::from(u16::from_le_bytes([entry[20], entry[21]]));
                let low = u32::from(u16::from_le_bytes([entry[26], entry[27]]));
                let size = u32::from_le_bytes(entry[28..32].try_into().unwrap());
                return Ok(((high << 16) | low, size, entry[11]));
            }
        }
        Err(GuestBootError::InvalidFat32Image)
    };
    let (efi_cluster, _, efi_attr) = find(cluster(root_cluster)?, b"EFI        ")?;
    let (boot_cluster, _, boot_attr) = find(cluster(efi_cluster)?, b"BOOT       ")?;
    let (file_cluster, file_size, file_attr) = find(cluster(boot_cluster)?, boot_file_name)?;
    if efi_attr & 0x10 == 0 || boot_attr & 0x10 == 0 || file_attr & 0x10 != 0 || file_size == 0 {
        return Err(GuestBootError::InvalidFat32Image);
    }
    let mut output = Vec::with_capacity(file_size as usize);
    let mut current = file_cluster;
    let mut remaining = file_size as usize;
    let max_steps = disk.len() / cluster_bytes;
    for _ in 0..max_steps {
        let bytes = cluster(current)?;
        let take = remaining.min(bytes.len());
        output.extend_from_slice(&bytes[..take]);
        remaining -= take;
        if remaining == 0 {
            parse_uefi_application(&output)?;
            return Ok(output);
        }
        let fat_entry = usize::try_from(current)
            .map_err(|_| GuestBootError::InvalidFat32Image)?
            .checked_mul(4)
            .ok_or(GuestBootError::ArithmeticOverflow)?;
        let next = read_u32(fat, fat_entry)? & 0x0fff_ffff;
        if next < 2 || next >= 0x0fff_fff8 || next == current {
            return Err(GuestBootError::InvalidFat32Image);
        }
        current = next;
    }
    Err(GuestBootError::InvalidFat32Image)
}

fn write_gpt_header(
    sector: &mut [u8],
    current_lba: u64,
    backup_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: &[u8; 16],
    entries_lba: u64,
    entry_count: u32,
    entry_size: u32,
    entries_crc: u32,
) -> Result<(), GuestBootError> {
    if sector.len() < 512 || first_usable_lba > last_usable_lba {
        return Err(GuestBootError::InvalidGptImage);
    }
    sector.fill(0);
    sector[0..8].copy_from_slice(b"EFI PART");
    sector[8..12].copy_from_slice(&0x0001_0000_u32.to_le_bytes());
    sector[12..16].copy_from_slice(&92_u32.to_le_bytes());
    sector[24..32].copy_from_slice(&current_lba.to_le_bytes());
    sector[32..40].copy_from_slice(&backup_lba.to_le_bytes());
    sector[40..48].copy_from_slice(&first_usable_lba.to_le_bytes());
    sector[48..56].copy_from_slice(&last_usable_lba.to_le_bytes());
    sector[56..72].copy_from_slice(disk_guid);
    sector[72..80].copy_from_slice(&entries_lba.to_le_bytes());
    sector[80..84].copy_from_slice(&entry_count.to_le_bytes());
    sector[84..88].copy_from_slice(&entry_size.to_le_bytes());
    sector[88..92].copy_from_slice(&entries_crc.to_le_bytes());
    let crc = crc32_ieee(&sector[..92]);
    sector[16..20].copy_from_slice(&crc.to_le_bytes());
    Ok(())
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn set_fat32_entry(fat: &mut [u8], cluster: usize, value: u32) -> Result<(), GuestBootError> {
    let offset = cluster
        .checked_mul(4)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    fat.get_mut(offset..offset + 4)
        .ok_or(GuestBootError::Fat32ImageTooLarge)?
        .copy_from_slice(&(value & 0x0fff_ffff).to_le_bytes());
    Ok(())
}

fn write_fat32_directory_entry(
    entry: &mut [u8],
    name: &[u8; 11],
    attributes: u8,
    first_cluster: u32,
    size: u32,
) -> Result<(), GuestBootError> {
    if entry.len() < 32 || name.iter().any(|byte| *byte < 0x20 || *byte > 0x7e) {
        return Err(GuestBootError::InvalidFat12Name);
    }
    entry.fill(0);
    entry[..11].copy_from_slice(name);
    entry[11] = attributes;
    entry[20..22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
    entry[26..28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
    Ok(())
}

fn set_fat12_entry(fat: &mut [u8], cluster: usize, value: u16) -> Result<(), GuestBootError> {
    let offset = cluster
        .checked_add(cluster / 2)
        .ok_or(GuestBootError::ArithmeticOverflow)?;
    if offset + 1 >= fat.len() {
        return Err(GuestBootError::Fat12ImageTooLarge);
    }
    let value = value & 0x0fff;
    if cluster & 1 == 0 {
        fat[offset] = value as u8;
        fat[offset + 1] = (fat[offset + 1] & 0xf0) | ((value >> 8) as u8 & 0x0f);
    } else {
        fat[offset] = (fat[offset] & 0x0f) | ((value << 4) as u8 & 0xf0);
        fat[offset + 1] = (value >> 4) as u8;
    }
    Ok(())
}

fn write_fat_directory_entry(
    entry: &mut [u8],
    name: &[u8; 11],
    attributes: u8,
    first_cluster: u16,
    size: u32,
) -> Result<(), GuestBootError> {
    if entry.len() < 32 || name.iter().any(|byte| *byte < 0x20 || *byte > 0x7e) {
        return Err(GuestBootError::InvalidFat12Name);
    }
    entry.fill(0);
    entry[..11].copy_from_slice(name);
    entry[11] = attributes;
    entry[26..28].copy_from_slice(&first_cluster.to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, GuestBootError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 2,
            actual: bytes.len(),
        })?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, GuestBootError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 4,
            actual: bytes.len(),
        })?;
    Ok(u32::from_le_bytes(raw.try_into().expect("four bytes")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, GuestBootError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 8,
            actual: bytes.len(),
        })?;
    Ok(u64::from_le_bytes(raw.try_into().expect("eight bytes")))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), GuestBootError> {
    let length = bytes.len();
    bytes
        .get_mut(offset..offset + 2)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 2,
            actual: length,
        })?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), GuestBootError> {
    let length = bytes.len();
    bytes
        .get_mut(offset..offset + 4)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 4,
            actual: length,
        })?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), GuestBootError> {
    let length = bytes.len();
    bytes
        .get_mut(offset..offset + 8)
        .ok_or(GuestBootError::ImageTooSmall {
            required: offset + 8,
            actual: length,
        })?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn align_up_usize(value: usize, alignment: usize) -> Result<usize, GuestBootError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(GuestBootError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALT_CODE: &[u8] = &[0xf4, 0xeb, 0xfd];

    #[test]
    fn bios_uefi_elf_multiboot_and_linux_formats_build_parse_and_materialize() {
        let bios = build_bios_boot_sector(HALT_CODE).unwrap();
        assert_eq!(
            detect_guest_image(&bios).unwrap(),
            GuestImageKind::BiosBootDisk
        );
        assert_eq!(GuestLoadPlan::parse(&bios).unwrap().entry_point, 0x7c00);

        let efi = build_x86_64_uefi_application(HALT_CODE).unwrap();
        let efi_plan = GuestLoadPlan::parse(&efi).unwrap();
        assert_eq!(efi_plan.protocol, GuestBootProtocol::UefiLoadImage);
        assert_eq!(efi_plan.cpu_mode, GuestCpuMode::Long64);

        let elf = build_elf64_guest_image(HALT_CODE, 0x20_0000, 0).unwrap();
        let elf_plan = GuestLoadPlan::parse(&elf).unwrap();
        assert_eq!(elf_plan.entry_point, 0x20_0000);
        let mut memory = vec![0xcc; 0x21_0000];
        elf_plan.materialize(&elf, &mut memory).unwrap();
        assert_eq!(&memory[0x20_0000..0x20_0003], HALT_CODE);

        let multiboot = build_multiboot2_elf64_guest(HALT_CODE, 0x30_0000).unwrap();
        let multiboot_plan = GuestLoadPlan::parse(&multiboot).unwrap();
        assert_eq!(multiboot_plan.protocol, GuestBootProtocol::Multiboot2);
        assert_eq!(multiboot_plan.entry_point, 0x30_0018);

        let mut linux = vec![0_u8; 4096];
        linux[0x1f1] = 4;
        linux[0x202..0x206].copy_from_slice(b"HdrS");
        linux[0x206..0x208].copy_from_slice(&0x020a_u16.to_le_bytes());
        linux[0x258..0x260].copy_from_slice(&0x10_0000_u64.to_le_bytes());
        let linux_plan = GuestLoadPlan::parse(&linux).unwrap();
        assert_eq!(linux_plan.protocol, GuestBootProtocol::LinuxBootProtocol);
        assert_eq!(linux_plan.segments.len(), 2);
        assert_eq!(linux_plan.linux_protocol_version, Some(0x020a));
    }

    #[test]
    fn multiboot2_x86_64_kernel_has_bios_efi64_entries_page_tables_and_exact_kernel_segments() {
        let kernel_code = [0xb8, 0x4b, 0x4f, 0x00, 0x00, 0xc3];
        let kernel = build_elf64_guest_image(&kernel_code, 0x40_0000, 0).unwrap();
        let wrapped = build_multiboot2_x86_64_kernel(&kernel).unwrap();
        let header_offset = find_multiboot2_header(&wrapped.bytes).unwrap();
        assert!(header_offset < 32 * 1024);
        let header_length = read_u32(&wrapped.bytes, header_offset + 8).unwrap() as usize;
        let header = &wrapped.bytes[header_offset..header_offset + header_length];
        assert_eq!(
            header
                .chunks_exact(4)
                .take(4)
                .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
                .fold(0_u32, u32::wrapping_add),
            0
        );
        assert_eq!(wrapped.protected_entry & 15, 0);
        assert!(wrapped.protected_entry >= MULTIBOOT2_X86_64_LOADER_BASE + header_length as u64);
        assert!(wrapped.efi64_entry > wrapped.protected_entry);
        assert_eq!(wrapped.kernel_entry, 0x40_0000);
        assert_eq!(wrapped.stack_top & 15, 0);
        let section_table_offset = read_u64(&wrapped.bytes, 40).unwrap() as usize;
        let section_count = read_u16(&wrapped.bytes, 60).unwrap() as usize;
        assert_eq!(read_u16(&wrapped.bytes, 58).unwrap(), 64);
        assert!(section_count >= 6);
        assert!(section_table_offset + section_count * 64 <= wrapped.bytes.len());
        assert!(wrapped
            .bytes
            .windows(b".boot.text\0".len())
            .any(|window| window == b".boot.text\0"));
        assert!(wrapped
            .bytes
            .windows(b".shstrtab\0".len())
            .any(|window| window == b".shstrtab\0"));
        assert!(wrapped
            .bytes
            .windows(5)
            .any(|window| window == [0x0d, 0x20, 0x06, 0x00, 0x00]));
        assert!(wrapped
            .bytes
            .windows(5)
            .any(|window| window == [0x0d, 0x22, 0x00, 0x01, 0x80]));

        let plan = GuestLoadPlan::parse(&wrapped.bytes).unwrap();
        assert_eq!(plan.protocol, GuestBootProtocol::Multiboot2);
        assert_eq!(plan.cpu_mode, GuestCpuMode::Protected32);
        assert_eq!(plan.entry_point, wrapped.protected_entry);
        assert_eq!(plan.segments.len(), 3);
        let kernel_segment = plan
            .segments
            .iter()
            .find(|segment| segment.guest_physical_address == 0x40_0000)
            .unwrap();
        assert_eq!(
            &wrapped.bytes[kernel_segment.source_offset
                ..kernel_segment.source_offset + kernel_segment.file_size],
            kernel_code
        );
        let page_tables = plan
            .segments
            .iter()
            .find(|segment| segment.guest_physical_address == MULTIBOOT2_X86_64_DATA_BASE)
            .unwrap();
        let pml4 = read_u64(&wrapped.bytes, page_tables.source_offset).unwrap();
        assert_eq!(pml4, MULTIBOOT2_X86_64_DATA_BASE + 0x1000 | 3);
    }

    #[test]
    fn opensbi_payload_preserves_riscv_load_address_entry_and_segment_bytes() {
        let mut elf = build_elf64_guest_image(HALT_CODE, 0x8020_0000, 0).unwrap();
        elf[18..20].copy_from_slice(&243_u16.to_le_bytes());
        let payload = build_opensbi_payload(&elf).unwrap();
        assert_eq!(payload.load_address, 0x8020_0000);
        assert_eq!(payload.entry_point, 0x8020_0000);
        assert_eq!(payload.entry_trampoline, None);
        assert_eq!(payload.bytes, HALT_CODE);
        assert_eq!(
            GuestLoadPlan::parse(&elf).unwrap().cpu_mode,
            GuestCpuMode::RiscV64
        );

        let mut offset_elf = build_elf64_guest_image(&[0_u8; 8], 0x8020_0000, 4).unwrap();
        offset_elf[18..20].copy_from_slice(&243_u16.to_le_bytes());
        let offset_payload = build_opensbi_payload(&offset_elf).unwrap();
        assert_eq!(offset_payload.entry_point, 0x8020_0004);
        assert_eq!(offset_payload.entry_trampoline, Some(0x0040_006f));
        assert_eq!(&offset_payload.bytes[..4], &0x0040_006f_u32.to_le_bytes());
    }

    #[test]
    fn fat12_contains_canonical_efi_boot_path_and_complete_cluster_chain() {
        let efi = build_x86_64_uefi_application(&vec![0x90; 1400]).unwrap();
        let disk = build_uefi_fat12_disk(&efi).unwrap();
        assert_eq!(disk.len(), FAT12_DISK_BYTES);
        assert_eq!(&disk[510..512], &[0x55, 0xaa]);
        assert!(disk.windows(11).any(|window| window == b"EFI        "));
        assert!(disk.windows(11).any(|window| window == b"BOOTX64 EFI"));
        assert!(disk.windows(efi.len()).any(|window| window == efi));
    }

    #[test]
    fn gpt_fat32_esp_has_redundant_headers_crc_and_exact_boot_payload() {
        let efi = build_x86_64_uefi_application(&vec![0x90; 1400]).unwrap();
        let disk = build_uefi_gpt_fat32_disk_named(&efi, b"BOOTX64 EFI").unwrap();
        assert!(disk.len() >= UEFI_GPT_MIN_DISK_BYTES);
        assert_eq!(&disk[510..512], &[0x55, 0xaa]);
        assert_eq!(&disk[512..520], b"EFI PART");
        assert_eq!(&disk[disk.len() - 512..disk.len() - 504], b"EFI PART");
        assert_eq!(
            validate_uefi_gpt_fat32_disk(&disk, b"BOOTX64 EFI").unwrap(),
            efi
        );

        let mut corrupt_primary = disk.clone();
        corrupt_primary[512 + 40] ^= 1;
        assert_eq!(
            validate_uefi_gpt_fat32_disk(&corrupt_primary, b"BOOTX64 EFI").unwrap_err(),
            GuestBootError::InvalidGptImage
        );
    }

    #[test]
    fn gpt_fat32_esp_supports_every_uefi_fallback_name_and_embeds_initrd() {
        let cases = [
            (
                build_x86_64_uefi_application(HALT_CODE).unwrap(),
                b"BOOTX64 EFI",
            ),
            (
                build_aarch64_uefi_application(&[0x00, 0x00, 0x20, 0xd4]).unwrap(),
                b"BOOTAA64EFI",
            ),
            (
                build_ia32_uefi_application(HALT_CODE).unwrap(),
                b"BOOTIA32EFI",
            ),
        ];
        let initrd = (0..1500)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        for (application, name) in cases {
            let disk =
                build_uefi_gpt_fat32_disk_named_with_initrd(&application, name, &initrd).unwrap();
            assert_eq!(
                validate_uefi_gpt_fat32_disk(&disk, name).unwrap(),
                application
            );
            assert!(disk.windows(11).any(|window| window == b"INITRD     "));
            assert!(disk.windows(initrd.len()).any(|window| window == initrd));
        }
    }

    #[test]
    fn firmware_descriptors_cover_bios_reset_and_uefi_firmware_volumes() {
        let bios = vec![0xff; 0x20_000];
        let descriptor = GuestFirmwareDescriptor::parse_bios(&bios).unwrap();
        assert_eq!(descriptor.guest_physical_base, 0xe_0000);
        assert_eq!(descriptor.reset_vector, 0xf_fff0);

        let mut uefi = vec![0xff; 0x40_000];
        let volume = 0x1000;
        uefi[volume + 0x20..volume + 0x28].copy_from_slice(&(0x20_000_u64).to_le_bytes());
        uefi[volume + 0x28..volume + 0x2c].copy_from_slice(b"_FVH");
        let descriptor = GuestFirmwareDescriptor::parse_uefi(&uefi).unwrap();
        assert_eq!(descriptor.firmware_volume_offset, Some(volume));
        assert_eq!(descriptor.firmware_volume_bytes, Some(0x20_000));
        assert_eq!(descriptor.guest_physical_base, UEFI_FIRMWARE_TOP - 0x40_000);
    }

    #[test]
    fn linux_zero_page_and_multiboot_information_are_protocol_exact() {
        let mut setup = vec![0_u8; 0x300];
        setup[0x202..0x206].copy_from_slice(b"HdrS");
        setup[0x206..0x208].copy_from_slice(&0x020a_u16.to_le_bytes());
        let params = LinuxBootParameters::build(
            &setup,
            "console=ttyS0 panic=-1",
            0x20_000,
            Some((0x40_0000, 0x1000)),
        )
        .unwrap();
        assert_eq!(&params.zero_page[0x228..0x22c], &0x20_000_u32.to_le_bytes());
        assert_eq!(params.command_line.last(), Some(&0));

        let info = build_multiboot2_information(
            Some("serial"),
            &[
                Multiboot2MemoryMapEntry {
                    base: 0,
                    length: 0x9f000,
                    kind: 1,
                },
                Multiboot2MemoryMapEntry {
                    base: 0x10_0000,
                    length: 0x3f0_0000,
                    kind: 1,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            u32::from_le_bytes(info[0..4].try_into().unwrap()) as usize,
            info.len()
        );
        assert_eq!(info.len() & 7, 0);
    }
}
