use crate::kernel_core::{
    AddressSpace, KernelCoreError, PageFlags, PhysicalMemory, KERNEL_PAGE_SIZE,
};
use std::collections::{BTreeMap, BTreeSet};

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PT_PHDR: u32 = 6;
const PT_TLS: u32 = 7;
const PT_GNU_STACK: u32 = 0x6474_e551;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_PLTRELSZ: i64 = 2;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_STRSZ: i64 = 10;
const DT_SYMENT: i64 = 11;
const DT_REL: i64 = 17;
const DT_RELSZ: i64 = 18;
const DT_RELENT: i64 = 19;
const DT_PLTREL: i64 = 20;
const DT_JMPREL: i64 = 23;

const AT_NULL: u64 = 0;
const AT_PHDR: u64 = 3;
const AT_PHENT: u64 = 4;
const AT_PHNUM: u64 = 5;
const AT_PAGESZ: u64 = 6;
const AT_BASE: u64 = 7;
const AT_ENTRY: u64 = 9;
const AT_RANDOM: u64 = 25;
const AT_EXECFN: u64 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserElfClass {
    Elf32,
    Elf64,
}

impl UserElfClass {
    pub const fn pointer_width(self) -> usize {
        match self {
            Self::Elf32 => 4,
            Self::Elf64 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserElfSegment {
    pub kind: u32,
    pub flags: u32,
    pub file_offset: u64,
    pub virtual_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub alignment: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserTlsTemplate {
    pub file_offset: usize,
    pub file_size: usize,
    pub memory_size: usize,
    pub alignment: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUserElf {
    pub class: UserElfClass,
    pub elf_type: u16,
    pub machine: u16,
    pub entry: u64,
    pub program_header_offset: u64,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
    pub segments: Vec<UserElfSegment>,
    pub tls: Option<UserTlsTemplate>,
    pub interpreter: Option<String>,
    pub executable_stack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLoadOptions {
    pub load_bias: u64,
    pub stack_top: u64,
    pub stack_pages: usize,
    pub tls_address: u64,
    pub argv: Vec<String>,
    pub envp: Vec<String>,
    pub symbols: BTreeMap<String, u64>,
    pub random: [u8; 16],
}

impl UserLoadOptions {
    pub fn for_class(class: UserElfClass) -> Self {
        match class {
            UserElfClass::Elf32 => Self {
                load_bias: 0x4000_0000,
                stack_top: 0x7fff_f000,
                stack_pages: 16,
                tls_address: 0x7000_0000,
                argv: vec!["program".into()],
                envp: Vec::new(),
                symbols: BTreeMap::new(),
                random: *b"nC-ELF-STACK-v1!",
            },
            UserElfClass::Elf64 => Self {
                load_bias: 0x0000_5555_0000_0000,
                stack_top: 0x0000_7fff_ffff_f000,
                stack_pages: 16,
                tls_address: 0x0000_7000_0000_0000,
                argv: vec!["program".into()],
                envp: Vec::new(),
                symbols: BTreeMap::new(),
                random: *b"nC-ELF-STACK-v1!",
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedUserElf {
    pub class: UserElfClass,
    pub machine: u16,
    pub entry: u64,
    pub load_bias: u64,
    pub stack_pointer: u64,
    pub stack_low: u64,
    pub thread_pointer: u64,
    pub tls_size: usize,
    pub interpreter: Option<String>,
    pub needed_libraries: Vec<String>,
    pub relocations_applied: usize,
    pub mapped_pages: usize,
    pub argv_addresses: Vec<u64>,
    pub envp_addresses: Vec<u64>,
    pub auxv: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelocationEncoding {
    Rel,
    Rela,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DynamicRelocationTable {
    address: u64,
    size: u64,
    entry_size: u64,
    encoding: RelocationEncoding,
}

#[derive(Debug, Clone, Default)]
struct DynamicInfo {
    strtab: Option<u64>,
    strsz: u64,
    symtab: Option<u64>,
    syment: u64,
    needed_offsets: Vec<u64>,
    tables: Vec<DynamicRelocationTable>,
    jmprel: Option<u64>,
    pltrelsz: u64,
    pltrel: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DynamicSymbol {
    name: String,
    value: u64,
    section_index: u16,
}

fn invalid(message: &'static str) -> KernelCoreError {
    KernelCoreError::InvalidElf(message)
}

fn checked_slice(image: &[u8], offset: u64, size: u64) -> Result<&[u8], KernelCoreError> {
    let start = usize::try_from(offset).map_err(|_| KernelCoreError::AddressOverflow)?;
    let size = usize::try_from(size).map_err(|_| KernelCoreError::AddressOverflow)?;
    let end = start
        .checked_add(size)
        .ok_or(KernelCoreError::AddressOverflow)?;
    image
        .get(start..end)
        .ok_or(invalid("ELF range is truncated"))
}

fn read_u16(image: &[u8], offset: usize) -> Result<u16, KernelCoreError> {
    let bytes = image
        .get(offset..offset + 2)
        .ok_or(invalid("truncated ELF u16"))?;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

fn read_u32(image: &[u8], offset: usize) -> Result<u32, KernelCoreError> {
    let bytes = image
        .get(offset..offset + 4)
        .ok_or(invalid("truncated ELF u32"))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn read_i32(image: &[u8], offset: usize) -> Result<i32, KernelCoreError> {
    Ok(read_u32(image, offset)? as i32)
}

fn read_u64(image: &[u8], offset: usize) -> Result<u64, KernelCoreError> {
    let bytes = image
        .get(offset..offset + 8)
        .ok_or(invalid("truncated ELF u64"))?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

fn read_i64(image: &[u8], offset: usize) -> Result<i64, KernelCoreError> {
    Ok(read_u64(image, offset)? as i64)
}

fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

fn align_up(value: u64, alignment: u64) -> Result<u64, KernelCoreError> {
    value
        .checked_add(alignment - 1)
        .map(|value| align_down(value, alignment))
        .ok_or(KernelCoreError::AddressOverflow)
}

fn runtime_address(parsed: &ParsedUserElf, load_bias: u64, address: u64) -> u64 {
    if parsed.elf_type == 3 {
        load_bias.wrapping_add(address)
    } else {
        address
    }
}

pub fn parse_user_elf(
    image: &[u8],
    expected_machine: Option<u16>,
) -> Result<ParsedUserElf, KernelCoreError> {
    if image.len() < 52 || !image.starts_with(b"\x7fELF") {
        return Err(invalid("missing ELF magic or header"));
    }
    if image[5] != 1 || image[6] != 1 {
        return Err(invalid("ELF must be little-endian version 1"));
    }
    let class = match image[4] {
        1 => UserElfClass::Elf32,
        2 => UserElfClass::Elf64,
        _ => return Err(invalid("unsupported ELF class")),
    };
    let elf_type = read_u16(image, 16)?;
    if !matches!(elf_type, 2 | 3) {
        return Err(invalid("user ELF must be ET_EXEC or ET_DYN"));
    }
    let machine = read_u16(image, 18)?;
    if expected_machine.is_some_and(|expected| expected != machine) {
        return Err(KernelCoreError::UnsupportedElf {
            class: image[4],
            machine,
        });
    }
    let (entry, phoff, phentsize, phnum) = match class {
        UserElfClass::Elf32 => (
            read_u32(image, 24)? as u64,
            read_u32(image, 28)? as u64,
            read_u16(image, 42)?,
            read_u16(image, 44)?,
        ),
        UserElfClass::Elf64 => (
            read_u64(image, 24)?,
            read_u64(image, 32)?,
            read_u16(image, 54)?,
            read_u16(image, 56)?,
        ),
    };
    let minimum = match class {
        UserElfClass::Elf32 => 32,
        UserElfClass::Elf64 => 56,
    };
    if usize::from(phentsize) < minimum || phnum == 0 {
        return Err(invalid("invalid ELF program header table"));
    }
    checked_slice(
        image,
        phoff,
        u64::from(phentsize)
            .checked_mul(u64::from(phnum))
            .ok_or(KernelCoreError::AddressOverflow)?,
    )?;

    let mut segments = Vec::new();
    let mut tls = None;
    let mut interpreter = None;
    let mut executable_stack = false;
    for index in 0..usize::from(phnum) {
        let offset = usize::try_from(phoff)
            .map_err(|_| KernelCoreError::AddressOverflow)?
            .checked_add(index * usize::from(phentsize))
            .ok_or(KernelCoreError::AddressOverflow)?;
        let kind = read_u32(image, offset)?;
        let (flags, file_offset, virtual_address, file_size, memory_size, alignment) = match class {
            UserElfClass::Elf32 => (
                read_u32(image, offset + 24)?,
                read_u32(image, offset + 4)? as u64,
                read_u32(image, offset + 8)? as u64,
                read_u32(image, offset + 16)? as u64,
                read_u32(image, offset + 20)? as u64,
                read_u32(image, offset + 28)? as u64,
            ),
            UserElfClass::Elf64 => (
                read_u32(image, offset + 4)?,
                read_u64(image, offset + 8)?,
                read_u64(image, offset + 16)?,
                read_u64(image, offset + 32)?,
                read_u64(image, offset + 40)?,
                read_u64(image, offset + 48)?,
            ),
        };
        if file_size > memory_size && matches!(kind, PT_LOAD | PT_TLS) {
            return Err(invalid("ELF filesz exceeds memsz"));
        }
        if file_size != 0 {
            checked_slice(image, file_offset, file_size)?;
        }
        if kind == PT_LOAD && memory_size != 0 {
            if alignment > 1
                && (!alignment.is_power_of_two()
                    || virtual_address % alignment != file_offset % alignment)
            {
                return Err(invalid("ELF PT_LOAD alignment contract is invalid"));
            }
        }
        let segment = UserElfSegment {
            kind,
            flags,
            file_offset,
            virtual_address,
            file_size,
            memory_size,
            alignment,
        };
        match kind {
            PT_TLS => {
                if tls.is_some() {
                    return Err(invalid("ELF contains multiple PT_TLS segments"));
                }
                tls = Some(UserTlsTemplate {
                    file_offset: usize::try_from(file_offset)
                        .map_err(|_| KernelCoreError::AddressOverflow)?,
                    file_size: usize::try_from(file_size)
                        .map_err(|_| KernelCoreError::AddressOverflow)?,
                    memory_size: usize::try_from(memory_size)
                        .map_err(|_| KernelCoreError::AddressOverflow)?,
                    alignment: alignment.max(1),
                });
            }
            PT_INTERP => {
                let raw = checked_slice(image, file_offset, file_size)?;
                let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
                interpreter = Some(
                    std::str::from_utf8(&raw[..end])
                        .map_err(|_| invalid("PT_INTERP is not UTF-8"))?
                        .to_owned(),
                );
            }
            PT_GNU_STACK => executable_stack = flags & 1 != 0,
            _ => {}
        }
        segments.push(segment);
    }
    let loads = segments
        .iter()
        .filter(|segment| segment.kind == PT_LOAD)
        .collect::<Vec<_>>();
    if loads.is_empty() {
        return Err(invalid("ELF has no PT_LOAD segment"));
    }
    if !loads.iter().any(|segment| {
        entry >= segment.virtual_address
            && entry < segment.virtual_address.saturating_add(segment.memory_size)
            && segment.flags & 1 != 0
    }) {
        return Err(invalid("ELF entry is outside executable PT_LOAD"));
    }
    Ok(ParsedUserElf {
        class,
        elf_type,
        machine,
        entry,
        program_header_offset: phoff,
        program_header_entry_size: phentsize,
        program_header_count: phnum,
        segments,
        tls,
        interpreter,
        executable_stack,
    })
}

fn segment_page_flags(flags: u32) -> PageFlags {
    let mut result = PageFlags::USER;
    if flags & 4 != 0 {
        result = result.union(PageFlags::READ);
    }
    if flags & 2 != 0 {
        result = result.union(PageFlags::WRITE);
    }
    if flags & 1 != 0 {
        result = result.union(PageFlags::EXECUTE);
    }
    result
}

fn virtual_to_file_offset(
    parsed: &ParsedUserElf,
    address: u64,
    size: u64,
) -> Result<usize, KernelCoreError> {
    for segment in parsed
        .segments
        .iter()
        .filter(|segment| segment.kind == PT_LOAD)
    {
        let end = address
            .checked_add(size)
            .ok_or(KernelCoreError::AddressOverflow)?;
        let segment_file_end = segment
            .virtual_address
            .checked_add(segment.file_size)
            .ok_or(KernelCoreError::AddressOverflow)?;
        if address >= segment.virtual_address && end <= segment_file_end {
            let delta = address - segment.virtual_address;
            return usize::try_from(segment.file_offset + delta)
                .map_err(|_| KernelCoreError::AddressOverflow);
        }
    }
    Err(invalid("dynamic address is outside file-backed PT_LOAD"))
}

fn parse_dynamic(image: &[u8], parsed: &ParsedUserElf) -> Result<DynamicInfo, KernelCoreError> {
    let Some(segment) = parsed
        .segments
        .iter()
        .find(|segment| segment.kind == PT_DYNAMIC)
    else {
        return Ok(DynamicInfo::default());
    };
    let entry_size = match parsed.class {
        UserElfClass::Elf32 => 8usize,
        UserElfClass::Elf64 => 16usize,
    };
    let bytes = checked_slice(image, segment.file_offset, segment.file_size)?;
    if bytes.len() % entry_size != 0 {
        return Err(invalid("PT_DYNAMIC size is not entry aligned"));
    }
    let mut values = BTreeMap::<i64, Vec<u64>>::new();
    for offset in (0..bytes.len()).step_by(entry_size) {
        let (tag, value) = match parsed.class {
            UserElfClass::Elf32 => (
                read_i32(bytes, offset)? as i64,
                read_u32(bytes, offset + 4)? as u64,
            ),
            UserElfClass::Elf64 => (read_i64(bytes, offset)?, read_u64(bytes, offset + 8)?),
        };
        if tag == DT_NULL {
            break;
        }
        values.entry(tag).or_default().push(value);
    }
    let first = |tag: i64| values.get(&tag).and_then(|values| values.first()).copied();
    let mut info = DynamicInfo {
        strtab: first(DT_STRTAB),
        strsz: first(DT_STRSZ).unwrap_or(0),
        symtab: first(DT_SYMTAB),
        syment: first(DT_SYMENT).unwrap_or(match parsed.class {
            UserElfClass::Elf32 => 16,
            UserElfClass::Elf64 => 24,
        }),
        needed_offsets: values.get(&DT_NEEDED).cloned().unwrap_or_default(),
        jmprel: first(DT_JMPREL),
        pltrelsz: first(DT_PLTRELSZ).unwrap_or(0),
        pltrel: first(DT_PLTREL).map(|value| value as i64),
        ..DynamicInfo::default()
    };
    if let Some(address) = first(DT_RELA) {
        info.tables.push(DynamicRelocationTable {
            address,
            size: first(DT_RELASZ).unwrap_or(0),
            entry_size: first(DT_RELAENT).unwrap_or(match parsed.class {
                UserElfClass::Elf32 => 12,
                UserElfClass::Elf64 => 24,
            }),
            encoding: RelocationEncoding::Rela,
        });
    }
    if let Some(address) = first(DT_REL) {
        info.tables.push(DynamicRelocationTable {
            address,
            size: first(DT_RELSZ).unwrap_or(0),
            entry_size: first(DT_RELENT).unwrap_or(match parsed.class {
                UserElfClass::Elf32 => 8,
                UserElfClass::Elf64 => 16,
            }),
            encoding: RelocationEncoding::Rel,
        });
    }
    if let Some(address) = info.jmprel {
        info.tables.push(DynamicRelocationTable {
            address,
            size: info.pltrelsz,
            entry_size: match (parsed.class, info.pltrel) {
                (UserElfClass::Elf32, Some(DT_RELA)) => 12,
                (UserElfClass::Elf32, _) => 8,
                (UserElfClass::Elf64, Some(DT_REL)) => 16,
                (UserElfClass::Elf64, _) => 24,
            },
            encoding: if info.pltrel == Some(DT_REL) {
                RelocationEncoding::Rel
            } else {
                RelocationEncoding::Rela
            },
        });
    }
    Ok(info)
}

fn dynamic_string(
    image: &[u8],
    parsed: &ParsedUserElf,
    dynamic: &DynamicInfo,
    offset: u64,
) -> Result<String, KernelCoreError> {
    let strtab = dynamic
        .strtab
        .ok_or(invalid("dynamic string table is missing"))?;
    if offset >= dynamic.strsz {
        return Err(invalid("dynamic string offset is out of range"));
    }
    let available = dynamic.strsz - offset;
    let file_offset = virtual_to_file_offset(parsed, strtab + offset, available)?;
    let bytes = image
        .get(file_offset..file_offset + available as usize)
        .ok_or(invalid("dynamic string is truncated"))?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(invalid("dynamic string is not terminated"))?;
    Ok(std::str::from_utf8(&bytes[..end])
        .map_err(|_| invalid("dynamic string is not UTF-8"))?
        .to_owned())
}

fn dynamic_symbol(
    image: &[u8],
    parsed: &ParsedUserElf,
    dynamic: &DynamicInfo,
    index: u32,
) -> Result<DynamicSymbol, KernelCoreError> {
    if index == 0 {
        return Ok(DynamicSymbol {
            name: String::new(),
            value: 0,
            section_index: 0,
        });
    }
    let symtab = dynamic.symtab.ok_or(invalid("DT_SYMTAB is missing"))?;
    let minimum = match parsed.class {
        UserElfClass::Elf32 => 16,
        UserElfClass::Elf64 => 24,
    };
    if dynamic.syment < minimum {
        return Err(invalid("DT_SYMENT is too small"));
    }
    let address = symtab
        .checked_add(u64::from(index) * dynamic.syment)
        .ok_or(KernelCoreError::AddressOverflow)?;
    let offset = virtual_to_file_offset(parsed, address, dynamic.syment)?;
    let (name_offset, value, section_index) = match parsed.class {
        UserElfClass::Elf32 => (
            read_u32(image, offset)? as u64,
            read_u32(image, offset + 4)? as u64,
            read_u16(image, offset + 14)?,
        ),
        UserElfClass::Elf64 => (
            read_u32(image, offset)? as u64,
            read_u64(image, offset + 8)?,
            read_u16(image, offset + 6)?,
        ),
    };
    Ok(DynamicSymbol {
        name: dynamic_string(image, parsed, dynamic, name_offset)?,
        value,
        section_index,
    })
}

fn read_pointer(
    address_space: &AddressSpace,
    memory: &PhysicalMemory,
    address: u64,
    width: usize,
) -> Result<u64, KernelCoreError> {
    let mut bytes = [0u8; 8];
    address_space.read(memory, address, &mut bytes[..width])?;
    Ok(match width {
        4 => u32::from_le_bytes(bytes[..4].try_into().unwrap()) as u64,
        8 => u64::from_le_bytes(bytes),
        _ => return Err(invalid("unsupported ELF pointer width")),
    })
}

fn write_integer(
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
    address: u64,
    value: u64,
    width: usize,
) -> Result<(), KernelCoreError> {
    match width {
        4 => address_space.write(memory, address, &(value as u32).to_le_bytes()),
        8 => address_space.write(memory, address, &value.to_le_bytes()),
        _ => Err(invalid("unsupported ELF relocation width")),
    }
}

fn relocation_shape(machine: u16, kind: u32) -> Result<(usize, bool, bool), KernelCoreError> {
    let shape = match machine {
        62 => match kind {
            1 => (8, false, false),
            2 => (4, false, true),
            6 | 7 => (8, false, false),
            8 => (8, true, false),
            10 | 11 => (4, false, false),
            _ => return Err(invalid("unsupported x86_64 dynamic relocation")),
        },
        3 => match kind {
            1 | 6 | 7 => (4, false, false),
            2 => (4, false, true),
            8 => (4, true, false),
            _ => return Err(invalid("unsupported i386 dynamic relocation")),
        },
        183 => match kind {
            257 | 1025 | 1026 => (8, false, false),
            1027 => (8, true, false),
            _ => return Err(invalid("unsupported AArch64 dynamic relocation")),
        },
        243 => match kind {
            1 => (4, false, false),
            2 | 5 => (8, false, false),
            3 => (8, true, false),
            _ => return Err(invalid("unsupported RISC-V dynamic relocation")),
        },
        _ => return Err(invalid("dynamic relocation machine is unsupported")),
    };
    Ok(shape)
}

fn apply_relocations(
    image: &[u8],
    parsed: &ParsedUserElf,
    dynamic: &DynamicInfo,
    load_bias: u64,
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
    symbols: &BTreeMap<String, u64>,
) -> Result<usize, KernelCoreError> {
    let mut applied = 0usize;
    let mut seen = BTreeSet::new();
    for table in &dynamic.tables {
        if table.size == 0 || table.entry_size == 0 || table.size % table.entry_size != 0 {
            return Err(invalid("dynamic relocation table has invalid geometry"));
        }
        let minimum = match (parsed.class, table.encoding) {
            (UserElfClass::Elf32, RelocationEncoding::Rel) => 8,
            (UserElfClass::Elf32, RelocationEncoding::Rela) => 12,
            (UserElfClass::Elf64, RelocationEncoding::Rel) => 16,
            (UserElfClass::Elf64, RelocationEncoding::Rela) => 24,
        };
        if table.entry_size < minimum {
            return Err(invalid("dynamic relocation entry is too small"));
        }
        let table_offset = virtual_to_file_offset(parsed, table.address, table.size)?;
        for index in 0..(table.size / table.entry_size) {
            let offset = table_offset
                .checked_add((index * table.entry_size) as usize)
                .ok_or(KernelCoreError::AddressOverflow)?;
            let (relocation_offset, info, explicit_addend) = match parsed.class {
                UserElfClass::Elf32 => (
                    read_u32(image, offset)? as u64,
                    read_u32(image, offset + 4)? as u64,
                    if table.encoding == RelocationEncoding::Rela {
                        Some(read_i32(image, offset + 8)? as i64)
                    } else {
                        None
                    },
                ),
                UserElfClass::Elf64 => (
                    read_u64(image, offset)?,
                    read_u64(image, offset + 8)?,
                    if table.encoding == RelocationEncoding::Rela {
                        Some(read_i64(image, offset + 16)?)
                    } else {
                        None
                    },
                ),
            };
            let (symbol_index, kind) = match parsed.class {
                UserElfClass::Elf32 => ((info >> 8) as u32, (info & 0xff) as u32),
                UserElfClass::Elf64 => ((info >> 32) as u32, info as u32),
            };
            if kind == 0 {
                continue;
            }
            let target = runtime_address(parsed, load_bias, relocation_offset);
            if !seen.insert((target, kind, symbol_index)) {
                continue;
            }
            let (width, relative, pc_relative) = relocation_shape(parsed.machine, kind)?;
            let addend = match explicit_addend {
                Some(value) => value,
                None => read_pointer(address_space, memory, target, width)? as i64,
            };
            let symbol_value = if relative {
                load_bias
            } else if symbol_index == 0 {
                0
            } else {
                let symbol = dynamic_symbol(image, parsed, dynamic, symbol_index)?;
                if symbol.section_index != 0 {
                    runtime_address(parsed, load_bias, symbol.value)
                } else {
                    *symbols
                        .get(&symbol.name)
                        .ok_or(invalid("unresolved dynamic ELF symbol"))?
                }
            };
            let value = if pc_relative {
                (symbol_value as i128 + addend as i128 - target as i128) as u64
            } else {
                (symbol_value as i128 + addend as i128) as u64
            };
            write_integer(address_space, memory, target, value, width)?;
            applied += 1;
        }
    }
    Ok(applied)
}

fn map_page_range(
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
    start: u64,
    end: u64,
    flags: PageFlags,
) -> Result<(), KernelCoreError> {
    let mut page = start;
    while page < end {
        address_space.map_zeroed(memory, page, flags)?;
        page = page
            .checked_add(KERNEL_PAGE_SIZE as u64)
            .ok_or(KernelCoreError::AddressOverflow)?;
    }
    Ok(())
}

fn push_blob(
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
    cursor: &mut u64,
    low: u64,
    bytes: &[u8],
) -> Result<u64, KernelCoreError> {
    *cursor = cursor
        .checked_sub(bytes.len() as u64)
        .filter(|value| *value >= low)
        .ok_or(invalid("initial user stack is too small"))?;
    address_space.write(memory, *cursor, bytes)?;
    Ok(*cursor)
}

fn build_initial_stack(
    parsed: &ParsedUserElf,
    options: &UserLoadOptions,
    load_bias: u64,
    entry: u64,
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
) -> Result<(u64, u64, Vec<u64>, Vec<u64>, Vec<(u64, u64)>), KernelCoreError> {
    if options.stack_pages < 2 || options.stack_top % KERNEL_PAGE_SIZE as u64 != 0 {
        return Err(invalid(
            "user stack requires aligned top and at least two pages",
        ));
    }
    let stack_bytes = (options.stack_pages as u64)
        .checked_mul(KERNEL_PAGE_SIZE as u64)
        .ok_or(KernelCoreError::AddressOverflow)?;
    let stack_low = options
        .stack_top
        .checked_sub(stack_bytes)
        .ok_or(KernelCoreError::AddressOverflow)?;
    let stack_flags = PageFlags::READ
        .union(PageFlags::WRITE)
        .union(PageFlags::USER)
        .union(if parsed.executable_stack {
            PageFlags::EXECUTE
        } else {
            PageFlags::NONE
        });
    map_page_range(
        address_space,
        memory,
        stack_low,
        options.stack_top,
        stack_flags,
    )?;
    let mut cursor = options.stack_top;
    let random_address = push_blob(
        address_space,
        memory,
        &mut cursor,
        stack_low,
        &options.random,
    )?;
    let mut argv_addresses = Vec::with_capacity(options.argv.len());
    for value in options.argv.iter().rev() {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        argv_addresses.push(push_blob(
            address_space,
            memory,
            &mut cursor,
            stack_low,
            &bytes,
        )?);
    }
    argv_addresses.reverse();
    let mut envp_addresses = Vec::with_capacity(options.envp.len());
    for value in options.envp.iter().rev() {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        envp_addresses.push(push_blob(
            address_space,
            memory,
            &mut cursor,
            stack_low,
            &bytes,
        )?);
    }
    envp_addresses.reverse();
    let execfn = argv_addresses.first().copied().unwrap_or(0);
    let phdr_address = parsed
        .segments
        .iter()
        .find(|segment| segment.kind == PT_PHDR)
        .map(|segment| runtime_address(parsed, load_bias, segment.virtual_address))
        .unwrap_or_else(|| runtime_address(parsed, load_bias, parsed.program_header_offset));
    let auxv = vec![
        (AT_PHDR, phdr_address),
        (AT_PHENT, u64::from(parsed.program_header_entry_size)),
        (AT_PHNUM, u64::from(parsed.program_header_count)),
        (AT_PAGESZ, KERNEL_PAGE_SIZE as u64),
        (AT_BASE, load_bias),
        (AT_ENTRY, entry),
        (AT_RANDOM, random_address),
        (AT_EXECFN, execfn),
        (AT_NULL, 0),
    ];
    let mut words = Vec::new();
    words.push(options.argv.len() as u64);
    words.extend(argv_addresses.iter().copied());
    words.push(0);
    words.extend(envp_addresses.iter().copied());
    words.push(0);
    for (tag, value) in &auxv {
        words.push(*tag);
        words.push(*value);
    }
    let width = parsed.class.pointer_width();
    let vector_bytes = words
        .len()
        .checked_mul(width)
        .ok_or(KernelCoreError::AddressOverflow)? as u64;
    let stack_pointer = align_down(
        cursor
            .checked_sub(vector_bytes)
            .filter(|value| *value >= stack_low)
            .ok_or(invalid("initial user stack is too small"))?,
        16,
    );
    let mut address = stack_pointer;
    for word in words {
        write_integer(address_space, memory, address, word, width)?;
        address += width as u64;
    }
    Ok((
        stack_pointer,
        stack_low,
        argv_addresses,
        envp_addresses,
        auxv,
    ))
}

pub fn load_user_elf(
    image: &[u8],
    expected_machine: Option<u16>,
    options: &UserLoadOptions,
    address_space: &mut AddressSpace,
    memory: &mut PhysicalMemory,
) -> Result<LoadedUserElf, KernelCoreError> {
    let parsed = parse_user_elf(image, expected_machine)?;
    if options.load_bias % KERNEL_PAGE_SIZE as u64 != 0
        || options.tls_address % KERNEL_PAGE_SIZE as u64 != 0
    {
        return Err(invalid(
            "ELF load bias and TLS address must be page aligned",
        ));
    }
    let load_bias = if parsed.elf_type == 3 {
        options.load_bias
    } else {
        0
    };
    let mut page_flags = BTreeMap::<u64, PageFlags>::new();
    for segment in parsed
        .segments
        .iter()
        .filter(|segment| segment.kind == PT_LOAD)
    {
        let start = align_down(
            runtime_address(&parsed, load_bias, segment.virtual_address),
            KERNEL_PAGE_SIZE as u64,
        );
        let end = align_up(
            runtime_address(
                &parsed,
                load_bias,
                segment
                    .virtual_address
                    .checked_add(segment.memory_size)
                    .ok_or(KernelCoreError::AddressOverflow)?,
            ),
            KERNEL_PAGE_SIZE as u64,
        )?;
        let flags = segment_page_flags(segment.flags);
        let mut page = start;
        while page < end {
            page_flags
                .entry(page)
                .and_modify(|current| *current = current.union(flags))
                .or_insert(flags);
            page += KERNEL_PAGE_SIZE as u64;
        }
    }
    for page in page_flags.keys().copied() {
        address_space.map_zeroed(
            memory,
            page,
            PageFlags::READ
                .union(PageFlags::WRITE)
                .union(PageFlags::USER),
        )?;
    }
    for segment in parsed
        .segments
        .iter()
        .filter(|segment| segment.kind == PT_LOAD && segment.file_size != 0)
    {
        address_space.write(
            memory,
            runtime_address(&parsed, load_bias, segment.virtual_address),
            checked_slice(image, segment.file_offset, segment.file_size)?,
        )?;
    }
    let dynamic = parse_dynamic(image, &parsed)?;
    let needed_libraries = dynamic
        .needed_offsets
        .iter()
        .map(|offset| dynamic_string(image, &parsed, &dynamic, *offset))
        .collect::<Result<Vec<_>, _>>()?;
    let relocations_applied = apply_relocations(
        image,
        &parsed,
        &dynamic,
        load_bias,
        address_space,
        memory,
        &options.symbols,
    )?;
    for (page, flags) in &page_flags {
        address_space.protect(*page, *flags)?;
    }

    let (thread_pointer, tls_size) = if let Some(tls) = &parsed.tls {
        let mapped_size = align_up(tls.memory_size as u64, KERNEL_PAGE_SIZE as u64)?;
        map_page_range(
            address_space,
            memory,
            options.tls_address,
            options.tls_address + mapped_size,
            PageFlags::READ
                .union(PageFlags::WRITE)
                .union(PageFlags::USER),
        )?;
        if tls.file_size != 0 {
            address_space.write(
                memory,
                options.tls_address,
                &image[tls.file_offset..tls.file_offset + tls.file_size],
            )?;
        }
        (options.tls_address, tls.memory_size)
    } else {
        (0, 0)
    };
    let entry = runtime_address(&parsed, load_bias, parsed.entry);
    let (stack_pointer, stack_low, argv_addresses, envp_addresses, auxv) =
        build_initial_stack(&parsed, options, load_bias, entry, address_space, memory)?;
    Ok(LoadedUserElf {
        class: parsed.class,
        machine: parsed.machine,
        entry,
        load_bias,
        stack_pointer,
        stack_low,
        thread_pointer,
        tls_size,
        interpreter: parsed.interpreter,
        needed_libraries,
        relocations_applied,
        mapped_pages: address_space.mapped_pages(),
        argv_addresses,
        envp_addresses,
        auxv,
    })
}

pub fn build_test_elf64_dynamic_tls(machine: u16) -> Vec<u8> {
    let image_size = 0x500usize;
    let mut image = vec![0u8; image_size];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&3u16.to_le_bytes());
    image[18..20].copy_from_slice(&machine.to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..32].copy_from_slice(&0x180u64.to_le_bytes());
    image[32..40].copy_from_slice(&64u64.to_le_bytes());
    image[52..54].copy_from_slice(&64u16.to_le_bytes());
    image[54..56].copy_from_slice(&56u16.to_le_bytes());
    image[56..58].copy_from_slice(&3u16.to_le_bytes());

    let ph = |image: &mut [u8],
              index: usize,
              kind: u32,
              flags: u32,
              offset: u64,
              vaddr: u64,
              filesz: u64,
              memsz: u64,
              align: u64| {
        let base = 64 + index * 56;
        image[base..base + 4].copy_from_slice(&kind.to_le_bytes());
        image[base + 4..base + 8].copy_from_slice(&flags.to_le_bytes());
        image[base + 8..base + 16].copy_from_slice(&offset.to_le_bytes());
        image[base + 16..base + 24].copy_from_slice(&vaddr.to_le_bytes());
        image[base + 24..base + 32].copy_from_slice(&vaddr.to_le_bytes());
        image[base + 32..base + 40].copy_from_slice(&filesz.to_le_bytes());
        image[base + 40..base + 48].copy_from_slice(&memsz.to_le_bytes());
        image[base + 48..base + 56].copy_from_slice(&align.to_le_bytes());
    };
    ph(
        &mut image,
        0,
        PT_LOAD,
        7,
        0,
        0,
        image_size as u64,
        0x500,
        0x1000,
    );
    ph(
        &mut image,
        1,
        PT_DYNAMIC,
        6,
        0x200,
        0x200,
        9 * 16,
        9 * 16,
        8,
    );
    ph(&mut image, 2, PT_TLS, 4, 0x3a0, 0x3a0, 4, 16, 8);
    image[0x180..0x184].copy_from_slice(&[0x31, 0xc0, 0xc3, 0x90]);
    let dynamic = [
        (DT_RELA, 0x300u64),
        (DT_RELASZ, 48),
        (DT_RELAENT, 24),
        (DT_STRTAB, 0x340),
        (DT_STRSZ, 23),
        (DT_SYMTAB, 0x360),
        (DT_SYMENT, 24),
        (DT_NEEDED, 1),
        (DT_NULL, 0),
    ];
    for (index, (tag, value)) in dynamic.into_iter().enumerate() {
        let offset = 0x200 + index * 16;
        image[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
        image[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
    let (relative_kind, symbol_kind) = match machine {
        62 => (8u64, 6u64),
        183 => (1027u64, 1025u64),
        243 => (3u64, 5u64),
        _ => (8u64, 6u64),
    };
    image[0x300..0x308].copy_from_slice(&0x3e0u64.to_le_bytes());
    image[0x308..0x310].copy_from_slice(&relative_kind.to_le_bytes());
    image[0x310..0x318].copy_from_slice(&0x180i64.to_le_bytes());
    image[0x318..0x320].copy_from_slice(&0x3e8u64.to_le_bytes());
    image[0x320..0x328].copy_from_slice(&((1u64 << 32) | symbol_kind).to_le_bytes());
    image[0x328..0x330].copy_from_slice(&4i64.to_le_bytes());
    image[0x340..0x357].copy_from_slice(b"\0libdemo.so\0host_write\0");
    image[0x378..0x37c].copy_from_slice(&12u32.to_le_bytes());
    image[0x3a0..0x3a4].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    image
}

pub fn build_test_elf32_dynamic_tls() -> Vec<u8> {
    let image_size = 0x340usize;
    let mut image = vec![0u8; image_size];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 1;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&3u16.to_le_bytes());
    image[18..20].copy_from_slice(&3u16.to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..28].copy_from_slice(&0x120u32.to_le_bytes());
    image[28..32].copy_from_slice(&52u32.to_le_bytes());
    image[40..42].copy_from_slice(&52u16.to_le_bytes());
    image[42..44].copy_from_slice(&32u16.to_le_bytes());
    image[44..46].copy_from_slice(&3u16.to_le_bytes());

    let ph = |image: &mut [u8],
              index: usize,
              kind: u32,
              flags: u32,
              offset: u32,
              vaddr: u32,
              filesz: u32,
              memsz: u32,
              align: u32| {
        let base = 52 + index * 32;
        image[base..base + 4].copy_from_slice(&kind.to_le_bytes());
        image[base + 4..base + 8].copy_from_slice(&offset.to_le_bytes());
        image[base + 8..base + 12].copy_from_slice(&vaddr.to_le_bytes());
        image[base + 12..base + 16].copy_from_slice(&vaddr.to_le_bytes());
        image[base + 16..base + 20].copy_from_slice(&filesz.to_le_bytes());
        image[base + 20..base + 24].copy_from_slice(&memsz.to_le_bytes());
        image[base + 24..base + 28].copy_from_slice(&flags.to_le_bytes());
        image[base + 28..base + 32].copy_from_slice(&align.to_le_bytes());
    };
    ph(
        &mut image,
        0,
        PT_LOAD,
        7,
        0,
        0,
        image_size as u32,
        0x400,
        0x1000,
    );
    ph(&mut image, 1, PT_DYNAMIC, 6, 0x180, 0x180, 72, 72, 4);
    ph(&mut image, 2, PT_TLS, 4, 0x2a0, 0x2a0, 4, 16, 4);
    image[0x120..0x124].copy_from_slice(&[0x31, 0xc0, 0xc3, 0x90]);
    let dynamic = [
        (DT_REL as i32, 0x220u32),
        (DT_RELSZ as i32, 16),
        (DT_RELENT as i32, 8),
        (DT_STRTAB as i32, 0x240),
        (DT_STRSZ as i32, 23),
        (DT_SYMTAB as i32, 0x260),
        (DT_SYMENT as i32, 16),
        (DT_NEEDED as i32, 1),
        (DT_NULL as i32, 0),
    ];
    for (index, (tag, value)) in dynamic.into_iter().enumerate() {
        let offset = 0x180 + index * 8;
        image[offset..offset + 4].copy_from_slice(&tag.to_le_bytes());
        image[offset + 4..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    image[0x220..0x224].copy_from_slice(&0x2c0u32.to_le_bytes());
    image[0x224..0x228].copy_from_slice(&8u32.to_le_bytes());
    image[0x228..0x22c].copy_from_slice(&0x2c4u32.to_le_bytes());
    image[0x22c..0x230].copy_from_slice(&((1u32 << 8) | 6).to_le_bytes());
    image[0x240..0x257].copy_from_slice(b"\0libdemo.so\0host_write\0");
    image[0x270..0x274].copy_from_slice(&12u32.to_le_bytes());
    image[0x2a0..0x2a4].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
    image[0x2c0..0x2c4].copy_from_slice(&0x120u32.to_le_bytes());
    image[0x2c4..0x2c8].copy_from_slice(&4u32.to_le_bytes());
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_elf_materializes_relative_relocation_tls_and_sysv_stack() {
        let image = build_test_elf64_dynamic_tls(62);
        let mut memory = PhysicalMemory::new(128).unwrap();
        let mut address_space = AddressSpace::default();
        let mut options = UserLoadOptions::for_class(UserElfClass::Elf64);
        options.argv = vec!["demo".into(), "--fast".into()];
        options.envp = vec!["MODE=test".into()];
        options.symbols.insert("host_write".into(), 0x7fff_1000);
        let loaded =
            load_user_elf(&image, Some(62), &options, &mut address_space, &mut memory).unwrap();
        assert_eq!(loaded.entry, options.load_bias + 0x180);
        assert_eq!(loaded.relocations_applied, 2);
        assert_eq!(loaded.needed_libraries, ["libdemo.so"]);
        assert_eq!(loaded.thread_pointer, options.tls_address);
        assert_eq!(loaded.tls_size, 16);
        let mut tls = [0u8; 16];
        address_space
            .read(&memory, loaded.thread_pointer, &mut tls)
            .unwrap();
        assert_eq!(&tls[..4], &[0x11, 0x22, 0x33, 0x44]);
        assert!(tls[4..].iter().all(|byte| *byte == 0));
        let relocated =
            read_pointer(&address_space, &memory, options.load_bias + 0x3e0, 8).unwrap();
        assert_eq!(relocated, options.load_bias + 0x180);
        assert_eq!(
            read_pointer(&address_space, &memory, options.load_bias + 0x3e8, 8).unwrap(),
            0x7fff_1004
        );
        assert_eq!(
            read_pointer(&address_space, &memory, loaded.stack_pointer, 8).unwrap(),
            2
        );
        assert_eq!(loaded.argv_addresses.len(), 2);
        assert_eq!(loaded.envp_addresses.len(), 1);
        assert_eq!(loaded.stack_pointer & 15, 0);
    }

    #[test]
    fn unsupported_relocation_is_rejected_before_claiming_loaded() {
        let mut image = build_test_elf64_dynamic_tls(62);
        image[0x308..0x310].copy_from_slice(&0xffffu64.to_le_bytes());
        let mut memory = PhysicalMemory::new(128).unwrap();
        let mut address_space = AddressSpace::default();
        let mut options = UserLoadOptions::for_class(UserElfClass::Elf64);
        options.symbols.insert("host_write".into(), 0x7fff_1000);
        assert!(matches!(
            load_user_elf(&image, Some(62), &options, &mut address_space, &mut memory),
            Err(KernelCoreError::InvalidElf(_))
        ));
    }

    #[test]
    fn elf64_relative_and_symbol_relocations_cover_aarch64_and_riscv64() {
        for machine in [183u16, 243u16] {
            let image = build_test_elf64_dynamic_tls(machine);
            let mut memory = PhysicalMemory::new(128).unwrap();
            let mut address_space = AddressSpace::default();
            let mut options = UserLoadOptions::for_class(UserElfClass::Elf64);
            options.symbols.insert("host_write".into(), 0x6000_0000);
            let loaded = load_user_elf(
                &image,
                Some(machine),
                &options,
                &mut address_space,
                &mut memory,
            )
            .unwrap();
            assert_eq!(loaded.relocations_applied, 2);
            assert_eq!(
                read_pointer(&address_space, &memory, options.load_bias + 0x3e0, 8).unwrap(),
                options.load_bias + 0x180
            );
            assert_eq!(
                read_pointer(&address_space, &memory, options.load_bias + 0x3e8, 8).unwrap(),
                0x6000_0004
            );
        }
    }

    #[test]
    fn elf32_i386_rel_tls_and_four_byte_initial_stack_are_material() {
        let image = build_test_elf32_dynamic_tls();
        let mut memory = PhysicalMemory::new(128).unwrap();
        let mut address_space = AddressSpace::default();
        let mut options = UserLoadOptions::for_class(UserElfClass::Elf32);
        options.argv = vec!["demo32".into(), "arg".into()];
        options.envp = vec!["ABI=i386".into()];
        options.symbols.insert("host_write".into(), 0x1234_0000);
        let loaded =
            load_user_elf(&image, Some(3), &options, &mut address_space, &mut memory).unwrap();
        assert_eq!(loaded.entry, options.load_bias + 0x120);
        assert_eq!(loaded.relocations_applied, 2);
        assert_eq!(
            read_pointer(&address_space, &memory, options.load_bias + 0x2c0, 4).unwrap(),
            options.load_bias + 0x120
        );
        assert_eq!(
            read_pointer(&address_space, &memory, options.load_bias + 0x2c4, 4).unwrap(),
            0x1234_0004
        );
        assert_eq!(
            read_pointer(&address_space, &memory, loaded.stack_pointer, 4).unwrap(),
            2
        );
        assert_eq!(loaded.thread_pointer, options.tls_address);
    }
}
