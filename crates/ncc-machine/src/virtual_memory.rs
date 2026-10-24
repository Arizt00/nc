use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const VM_PAGE_4K: u64 = 4 * 1024;
pub const VM_PAGE_2M: u64 = 2 * 1024 * 1024;
pub const VM_PAGE_1G: u64 = 1024 * 1024 * 1024;
pub const VM_TABLE_ENTRIES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmArchitecture {
    X86_64,
    Aarch64_4K,
    RiscvSv39,
    RiscvSv48,
}

impl VmArchitecture {
    pub const fn levels(self) -> usize {
        match self {
            Self::RiscvSv39 => 3,
            Self::X86_64 | Self::Aarch64_4K | Self::RiscvSv48 => 4,
        }
    }

    pub const fn virtual_address_bits(self) -> u8 {
        match self {
            Self::RiscvSv39 => 39,
            Self::X86_64 | Self::Aarch64_4K | Self::RiscvSv48 => 48,
        }
    }

    pub const fn activation_register(self) -> &'static str {
        match self {
            Self::X86_64 => "cr3",
            Self::Aarch64_4K => "ttbr0_el1",
            Self::RiscvSv39 | Self::RiscvSv48 => "satp",
        }
    }

    pub const fn profile(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64-pml4-4k-v1",
            Self::Aarch64_4K => "aarch64-stage1-4k-v1",
            Self::RiscvSv39 => "riscv64-sv39-v1",
            Self::RiscvSv48 => "riscv64-sv48-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmPageSize {
    Size4K,
    Size2M,
    Size1G,
}

impl VmPageSize {
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Size4K => VM_PAGE_4K,
            Self::Size2M => VM_PAGE_2M,
            Self::Size1G => VM_PAGE_1G,
        }
    }

    const fn leaf_level_from_bottom(self) -> usize {
        match self {
            Self::Size4K => 0,
            Self::Size2M => 1,
            Self::Size1G => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VmPageFlags(u16);

impl VmPageFlags {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);
    pub const GLOBAL: Self = Self(1 << 4);
    pub const DEVICE: Self = Self(1 << 5);
    pub const COW: Self = Self(1 << 6);
    pub const GUARD: Self = Self(1 << 7);
    pub const ACCESSED: Self = Self(1 << 8);
    pub const DIRTY: Self = Self(1 << 9);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn bits(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    InvalidConfiguration(&'static str),
    InvalidVirtualAddress(u64),
    InvalidPhysicalAddress(u64),
    Misaligned { address: u64, alignment: u64 },
    AddressOverflow,
    OutOfTablePages,
    MappingConflict(u64),
    MappingMissing(u64),
    GuardOverlap(u64),
    InvalidPermissions(&'static str),
    InvalidShootdownCpu(u32),
    DuplicateShootdownAck(u32),
    ShootdownGeneration { expected: u64, observed: u64 },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) | Self::InvalidPermissions(message) => {
                f.write_str(message)
            }
            Self::InvalidVirtualAddress(address) => {
                write!(f, "virtual address {address:#x} is not canonical")
            }
            Self::InvalidPhysicalAddress(address) => {
                write!(
                    f,
                    "physical address {address:#x} is outside the material profile"
                )
            }
            Self::Misaligned { address, alignment } => {
                write!(f, "address {address:#x} is not aligned to {alignment:#x}")
            }
            Self::AddressOverflow => f.write_str("virtual-memory address arithmetic overflow"),
            Self::OutOfTablePages => f.write_str("page-table backing range is exhausted"),
            Self::MappingConflict(address) => {
                write!(f, "virtual mapping conflicts at {address:#x}")
            }
            Self::MappingMissing(address) => write!(f, "virtual mapping is absent at {address:#x}"),
            Self::GuardOverlap(address) => write!(f, "guard range overlaps at {address:#x}"),
            Self::InvalidShootdownCpu(cpu) => write!(f, "CPU {cpu} is not in the shootdown set"),
            Self::DuplicateShootdownAck(cpu) => {
                write!(f, "CPU {cpu} acknowledged the shootdown twice")
            }
            Self::ShootdownGeneration { expected, observed } => write!(
                f,
                "TLB shootdown generation mismatch: expected {expected}, observed {observed}"
            ),
        }
    }
}

impl Error for VmError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmMapping {
    pub virtual_address: u64,
    pub physical_address: u64,
    pub page_size: VmPageSize,
    pub flags: VmPageFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VmTablePage {
    physical_address: u64,
    entries: Box<[u64; VM_TABLE_ENTRIES]>,
}

impl VmTablePage {
    fn new(physical_address: u64) -> Self {
        Self {
            physical_address,
            entries: Box::new([0; VM_TABLE_ENTRIES]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HardwarePageTables {
    architecture: VmArchitecture,
    asid: u16,
    root_physical_address: u64,
    next_table_physical_address: u64,
    table_limit_physical_address: u64,
    tables: BTreeMap<u64, VmTablePage>,
    mappings: BTreeMap<u64, VmMapping>,
    guards: BTreeMap<u64, u64>,
    generation: u64,
}

impl HardwarePageTables {
    pub fn new(
        architecture: VmArchitecture,
        root_physical_address: u64,
        table_page_capacity: usize,
        asid: u16,
    ) -> Result<Self, VmError> {
        if table_page_capacity == 0 {
            return Err(VmError::InvalidConfiguration(
                "page tables require at least one table page",
            ));
        }
        require_alignment(root_physical_address, VM_PAGE_4K)?;
        validate_physical_address(root_physical_address)?;
        let byte_capacity = (table_page_capacity as u64)
            .checked_mul(VM_PAGE_4K)
            .ok_or(VmError::AddressOverflow)?;
        let limit = root_physical_address
            .checked_add(byte_capacity)
            .ok_or(VmError::AddressOverflow)?;
        let mut tables = BTreeMap::new();
        tables.insert(
            root_physical_address,
            VmTablePage::new(root_physical_address),
        );
        Ok(Self {
            architecture,
            asid,
            root_physical_address,
            next_table_physical_address: root_physical_address + VM_PAGE_4K,
            table_limit_physical_address: limit,
            tables,
            mappings: BTreeMap::new(),
            guards: BTreeMap::new(),
            generation: 0,
        })
    }

    pub const fn architecture(&self) -> VmArchitecture {
        self.architecture
    }

    pub const fn root_physical_address(&self) -> u64 {
        self.root_physical_address
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn table_page_count(&self) -> usize {
        self.tables.len()
    }

    pub fn mappings(&self) -> impl Iterator<Item = &VmMapping> {
        self.mappings.values()
    }

    pub fn map(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        page_size: VmPageSize,
        flags: VmPageFlags,
    ) -> Result<(), VmError> {
        self.validate_mapping(virtual_address, physical_address, page_size, flags)?;
        if self.mapping_covering(virtual_address).is_some()
            || self.range_overlaps_existing(virtual_address, page_size.bytes())
        {
            return Err(VmError::MappingConflict(virtual_address));
        }
        let indices = page_indices(self.architecture, virtual_address);
        let leaf_depth = self
            .architecture
            .levels()
            .checked_sub(1 + page_size.leaf_level_from_bottom())
            .ok_or(VmError::InvalidConfiguration(
                "page size is unsupported by the selected translation depth",
            ))?;
        let mut table_address = self.root_physical_address;
        for (depth, index) in indices.iter().copied().enumerate() {
            if depth == leaf_depth {
                let leaf = encode_leaf(self.architecture, physical_address, page_size, flags)?;
                let table = self
                    .tables
                    .get_mut(&table_address)
                    .ok_or(VmError::InvalidConfiguration("missing material table page"))?;
                if table.entries[index] != 0 {
                    return Err(VmError::MappingConflict(virtual_address));
                }
                table.entries[index] = leaf;
                break;
            }
            let current = self
                .tables
                .get(&table_address)
                .ok_or(VmError::InvalidConfiguration("missing material table page"))?
                .entries[index];
            table_address = if current == 0 {
                let child = self.allocate_table_page()?;
                let pointer = encode_table_pointer(self.architecture, child);
                self.tables
                    .get_mut(&table_address)
                    .ok_or(VmError::InvalidConfiguration("missing parent table page"))?
                    .entries[index] = pointer;
                child
            } else if entry_is_table(self.architecture, current, depth) {
                decode_table_address(self.architecture, current)
            } else {
                return Err(VmError::MappingConflict(virtual_address));
            };
        }
        self.mappings.insert(
            virtual_address,
            VmMapping {
                virtual_address,
                physical_address,
                page_size,
                flags,
            },
        );
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn map_range(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        length: u64,
        page_size: VmPageSize,
        flags: VmPageFlags,
    ) -> Result<(), VmError> {
        if length == 0 || length % page_size.bytes() != 0 {
            return Err(VmError::InvalidConfiguration(
                "mapped range length must be a non-zero page-size multiple",
            ));
        }
        let mut offset = 0;
        while offset < length {
            let va = virtual_address
                .checked_add(offset)
                .ok_or(VmError::AddressOverflow)?;
            let pa = physical_address
                .checked_add(offset)
                .ok_or(VmError::AddressOverflow)?;
            if let Err(error) = self.map(va, pa, page_size, flags) {
                let _ = self.unmap_range(virtual_address, offset);
                return Err(error);
            }
            offset += page_size.bytes();
        }
        Ok(())
    }

    pub fn mmap_kernel(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        length: u64,
        page_size: VmPageSize,
        flags: VmPageFlags,
    ) -> Result<(), VmError> {
        self.map_range(
            virtual_address,
            physical_address,
            length,
            page_size,
            flags.without(VmPageFlags::USER),
        )
    }

    pub fn mmap_user(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        length: u64,
        page_size: VmPageSize,
        flags: VmPageFlags,
    ) -> Result<(), VmError> {
        self.map_range(
            virtual_address,
            physical_address,
            length,
            page_size,
            flags.union(VmPageFlags::USER),
        )
    }

    pub fn reserve_guard(&mut self, virtual_address: u64, length: u64) -> Result<(), VmError> {
        require_alignment(virtual_address, VM_PAGE_4K)?;
        if length == 0 || length % VM_PAGE_4K != 0 {
            return Err(VmError::InvalidConfiguration(
                "guard range length must be a non-zero 4 KiB multiple",
            ));
        }
        validate_virtual_address(self.architecture, virtual_address)?;
        if self.range_overlaps_existing(virtual_address, length)
            || self.guard_overlaps(virtual_address, length)
        {
            return Err(VmError::GuardOverlap(virtual_address));
        }
        self.guards.insert(virtual_address, length);
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn unmap(&mut self, virtual_address: u64) -> Result<VmMapping, VmError> {
        let mapping = self
            .mappings
            .remove(&virtual_address)
            .ok_or(VmError::MappingMissing(virtual_address))?;
        let (table_address, index) = self.leaf_slot(mapping)?;
        self.tables
            .get_mut(&table_address)
            .ok_or(VmError::InvalidConfiguration("missing material leaf table"))?
            .entries[index] = 0;
        self.generation = self.generation.wrapping_add(1);
        Ok(mapping)
    }

    pub fn unmap_range(&mut self, virtual_address: u64, length: u64) -> Result<(), VmError> {
        if length == 0 {
            return Ok(());
        }
        let end = virtual_address
            .checked_add(length)
            .ok_or(VmError::AddressOverflow)?;
        let bases = self
            .mappings
            .range(virtual_address..end)
            .map(|(base, _)| *base)
            .collect::<Vec<_>>();
        for base in bases {
            self.unmap(base)?;
        }
        Ok(())
    }

    pub fn protect(&mut self, virtual_address: u64, flags: VmPageFlags) -> Result<(), VmError> {
        let mapping = self
            .mappings
            .get(&virtual_address)
            .copied()
            .ok_or(VmError::MappingMissing(virtual_address))?;
        validate_flags(self.architecture, flags)?;
        let (table_address, index) = self.leaf_slot(mapping)?;
        let leaf = encode_leaf(
            self.architecture,
            mapping.physical_address,
            mapping.page_size,
            flags,
        )?;
        self.tables
            .get_mut(&table_address)
            .ok_or(VmError::InvalidConfiguration("missing material leaf table"))?
            .entries[index] = leaf;
        self.mappings
            .get_mut(&virtual_address)
            .expect("mapping checked above")
            .flags = flags;
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn translate(&self, virtual_address: u64) -> Result<u64, VmError> {
        validate_virtual_address(self.architecture, virtual_address)?;
        let mapping = self
            .mapping_covering(virtual_address)
            .ok_or(VmError::MappingMissing(virtual_address))?;
        mapping
            .physical_address
            .checked_add(virtual_address - mapping.virtual_address)
            .ok_or(VmError::AddressOverflow)
    }

    pub fn fault(&self, virtual_address: u64, access: VmFaultAccess) -> VmFaultResolution {
        if self.guard_covering(virtual_address).is_some() {
            return VmFaultResolution::GuardPage;
        }
        let Some(mapping) = self.mapping_covering(virtual_address) else {
            return VmFaultResolution::NotPresent;
        };
        match access {
            VmFaultAccess::Read if !mapping.flags.contains(VmPageFlags::READ) => {
                VmFaultResolution::Protection
            }
            VmFaultAccess::Execute if !mapping.flags.contains(VmPageFlags::EXECUTE) => {
                VmFaultResolution::Protection
            }
            VmFaultAccess::Write if mapping.flags.contains(VmPageFlags::COW) => {
                VmFaultResolution::CopyOnWriteRequired {
                    mapping_base: mapping.virtual_address,
                    source_physical_address: mapping.physical_address,
                    page_size: mapping.page_size,
                }
            }
            VmFaultAccess::Write if !mapping.flags.contains(VmPageFlags::WRITE) => {
                VmFaultResolution::Protection
            }
            _ => VmFaultResolution::Spurious,
        }
    }

    pub fn resolve_cow(
        &mut self,
        mapping_base: u64,
        replacement_physical_address: u64,
    ) -> Result<(), VmError> {
        let mapping = self
            .mappings
            .get(&mapping_base)
            .copied()
            .ok_or(VmError::MappingMissing(mapping_base))?;
        if !mapping.flags.contains(VmPageFlags::COW) {
            return Err(VmError::InvalidPermissions(
                "COW resolution requires a copy-on-write mapping",
            ));
        }
        require_alignment(replacement_physical_address, mapping.page_size.bytes())?;
        validate_physical_address(replacement_physical_address)?;
        let flags = mapping
            .flags
            .without(VmPageFlags::COW)
            .union(VmPageFlags::WRITE)
            .union(VmPageFlags::DIRTY);
        let (table_address, index) = self.leaf_slot(mapping)?;
        self.tables
            .get_mut(&table_address)
            .ok_or(VmError::InvalidConfiguration("missing material leaf table"))?
            .entries[index] = encode_leaf(
            self.architecture,
            replacement_physical_address,
            mapping.page_size,
            flags,
        )?;
        let updated = self
            .mappings
            .get_mut(&mapping_base)
            .expect("mapping checked above");
        updated.physical_address = replacement_physical_address;
        updated.flags = flags;
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn fork_cow(
        &mut self,
        child_root_physical_address: u64,
        child_table_page_capacity: usize,
        child_asid: u16,
    ) -> Result<Self, VmError> {
        let mappings = self.mappings.values().copied().collect::<Vec<_>>();
        let guards = self.guards.clone();
        let mut child = Self::new(
            self.architecture,
            child_root_physical_address,
            child_table_page_capacity,
            child_asid,
        )?;
        for mapping in mappings {
            let flags = if mapping.flags.contains(VmPageFlags::WRITE) {
                mapping
                    .flags
                    .without(VmPageFlags::WRITE)
                    .union(VmPageFlags::COW)
            } else {
                mapping.flags
            };
            self.protect(mapping.virtual_address, flags)?;
            child.map(
                mapping.virtual_address,
                mapping.physical_address,
                mapping.page_size,
                flags,
            )?;
        }
        for (base, length) in guards {
            child.reserve_guard(base, length)?;
        }
        Ok(child)
    }

    pub fn activation_value(&self) -> u64 {
        match self.architecture {
            VmArchitecture::X86_64 => self.root_physical_address,
            VmArchitecture::Aarch64_4K => self.root_physical_address | ((self.asid as u64) << 48),
            VmArchitecture::RiscvSv39 => {
                (8u64 << 60) | ((self.asid as u64) << 44) | (self.root_physical_address >> 12)
            }
            VmArchitecture::RiscvSv48 => {
                (9u64 << 60) | ((self.asid as u64) << 44) | (self.root_physical_address >> 12)
            }
        }
    }

    pub fn material_image(&self) -> Vec<u8> {
        let page_count =
            ((self.next_table_physical_address - self.root_physical_address) / VM_PAGE_4K) as usize;
        let mut bytes = vec![0u8; page_count * VM_PAGE_4K as usize];
        for table in self.tables.values() {
            let page_offset = (table.physical_address - self.root_physical_address) as usize;
            for (index, entry) in table.entries.iter().enumerate() {
                let offset = page_offset + index * 8;
                bytes[offset..offset + 8].copy_from_slice(&entry.to_le_bytes());
            }
        }
        bytes
    }

    pub fn root_entry(&self, index: usize) -> Option<u64> {
        self.tables
            .get(&self.root_physical_address)
            .and_then(|table| table.entries.get(index))
            .copied()
    }

    fn allocate_table_page(&mut self) -> Result<u64, VmError> {
        if self.next_table_physical_address >= self.table_limit_physical_address {
            return Err(VmError::OutOfTablePages);
        }
        let address = self.next_table_physical_address;
        self.next_table_physical_address = address
            .checked_add(VM_PAGE_4K)
            .ok_or(VmError::AddressOverflow)?;
        self.tables.insert(address, VmTablePage::new(address));
        Ok(address)
    }

    fn validate_mapping(
        &self,
        virtual_address: u64,
        physical_address: u64,
        page_size: VmPageSize,
        flags: VmPageFlags,
    ) -> Result<(), VmError> {
        validate_virtual_address(self.architecture, virtual_address)?;
        validate_physical_address(physical_address)?;
        require_alignment(virtual_address, page_size.bytes())?;
        require_alignment(physical_address, page_size.bytes())?;
        validate_flags(self.architecture, flags)?;
        if flags.contains(VmPageFlags::GUARD) {
            return Err(VmError::InvalidPermissions(
                "guard pages are reserved without a physical leaf",
            ));
        }
        Ok(())
    }

    fn leaf_slot(&self, mapping: VmMapping) -> Result<(u64, usize), VmError> {
        let indices = page_indices(self.architecture, mapping.virtual_address);
        let leaf_depth =
            self.architecture.levels() - 1 - mapping.page_size.leaf_level_from_bottom();
        let mut table_address = self.root_physical_address;
        for (depth, index) in indices.into_iter().enumerate() {
            if depth == leaf_depth {
                return Ok((table_address, index));
            }
            let entry = self
                .tables
                .get(&table_address)
                .ok_or(VmError::MappingMissing(mapping.virtual_address))?
                .entries[index];
            if !entry_is_table(self.architecture, entry, depth) {
                return Err(VmError::MappingMissing(mapping.virtual_address));
            }
            table_address = decode_table_address(self.architecture, entry);
        }
        Err(VmError::MappingMissing(mapping.virtual_address))
    }

    fn mapping_covering(&self, address: u64) -> Option<VmMapping> {
        let (_, mapping) = self.mappings.range(..=address).next_back()?;
        (address < mapping.virtual_address + mapping.page_size.bytes()).then_some(*mapping)
    }

    fn range_overlaps_existing(&self, base: u64, length: u64) -> bool {
        self.mappings.values().any(|mapping| {
            ranges_overlap(
                base,
                length,
                mapping.virtual_address,
                mapping.page_size.bytes(),
            )
        }) || self.guard_overlaps(base, length)
    }

    fn guard_overlaps(&self, base: u64, length: u64) -> bool {
        self.guards.iter().any(|(guard_base, guard_length)| {
            ranges_overlap(base, length, *guard_base, *guard_length)
        })
    }

    fn guard_covering(&self, address: u64) -> Option<(u64, u64)> {
        let (base, length) = self.guards.range(..=address).next_back()?;
        (address < base.saturating_add(*length)).then_some((*base, *length))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmFaultAccess {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmFaultResolution {
    NotPresent,
    GuardPage,
    Protection,
    CopyOnWriteRequired {
        mapping_base: u64,
        source_physical_address: u64,
        page_size: VmPageSize,
    },
    Spurious,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmIsaInstruction {
    pub mnemonic: &'static str,
    pub operands: &'static str,
    pub bytes: &'static [u8],
}

pub fn vm_activation_sequence(architecture: VmArchitecture) -> Vec<VmIsaInstruction> {
    match architecture {
        VmArchitecture::X86_64 => vec![VmIsaInstruction {
            mnemonic: "mov",
            operands: "cr3, rax",
            bytes: &[0x0f, 0x22, 0xd8],
        }],
        VmArchitecture::Aarch64_4K => vec![
            VmIsaInstruction {
                mnemonic: "msr",
                operands: "ttbr0_el1, x0",
                bytes: &[0x00, 0x20, 0x18, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "dsb",
                operands: "ish",
                bytes: &[0x9f, 0x3b, 0x03, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "isb",
                operands: "sy",
                bytes: &[0xdf, 0x3f, 0x03, 0xd5],
            },
        ],
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => vec![
            VmIsaInstruction {
                mnemonic: "csrw",
                operands: "satp, a0",
                bytes: &[0x73, 0x10, 0x05, 0x18],
            },
            VmIsaInstruction {
                mnemonic: "sfence.vma",
                operands: "x0, x0",
                bytes: &[0x73, 0x00, 0x00, 0x12],
            },
        ],
    }
}

pub fn vm_local_invalidation_sequence(architecture: VmArchitecture) -> Vec<VmIsaInstruction> {
    match architecture {
        VmArchitecture::X86_64 => vec![VmIsaInstruction {
            mnemonic: "invlpg",
            operands: "[rax]",
            bytes: &[0x0f, 0x01, 0x38],
        }],
        VmArchitecture::Aarch64_4K => vec![
            VmIsaInstruction {
                mnemonic: "dsb",
                operands: "ishst",
                bytes: &[0x9f, 0x3a, 0x03, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "tlbi",
                operands: "vae1is, x0",
                bytes: &[0x20, 0x83, 0x08, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "dsb",
                operands: "ish",
                bytes: &[0x9f, 0x3b, 0x03, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "isb",
                operands: "sy",
                bytes: &[0xdf, 0x3f, 0x03, 0xd5],
            },
        ],
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => vec![VmIsaInstruction {
            mnemonic: "sfence.vma",
            operands: "a0, a1",
            bytes: &[0x73, 0x00, 0xb5, 0x12],
        }],
    }
}

pub fn vm_context_invalidation_sequence(architecture: VmArchitecture) -> Vec<VmIsaInstruction> {
    match architecture {
        VmArchitecture::X86_64 => vec![VmIsaInstruction {
            mnemonic: "invpcid",
            operands: "rax, [rbx]",
            bytes: &[0x66, 0x0f, 0x38, 0x82, 0x03],
        }],
        VmArchitecture::Aarch64_4K => vec![
            VmIsaInstruction {
                mnemonic: "dsb",
                operands: "ishst",
                bytes: &[0x9f, 0x3a, 0x03, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "tlbi",
                operands: "aside1is, x0",
                bytes: &[0x40, 0x83, 0x08, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "dsb",
                operands: "ish",
                bytes: &[0x9f, 0x3b, 0x03, 0xd5],
            },
            VmIsaInstruction {
                mnemonic: "isb",
                operands: "sy",
                bytes: &[0xdf, 0x3f, 0x03, 0xd5],
            },
        ],
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => vec![VmIsaInstruction {
            mnemonic: "sfence.vma",
            operands: "x0, a1",
            bytes: &[0x73, 0x00, 0xb0, 0x12],
        }],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmTlbShootdown {
    pub generation: u64,
    pub virtual_address: u64,
    pub length: u64,
    pub asid: u16,
    pending: BTreeSet<u32>,
    acknowledged: BTreeSet<u32>,
}

impl VmTlbShootdown {
    pub fn new(
        generation: u64,
        virtual_address: u64,
        length: u64,
        asid: u16,
        target_cpus: impl IntoIterator<Item = u32>,
    ) -> Result<Self, VmError> {
        if length == 0 {
            return Err(VmError::InvalidConfiguration(
                "TLB shootdown length must be non-zero",
            ));
        }
        let pending = target_cpus.into_iter().collect::<BTreeSet<_>>();
        if pending.is_empty() {
            return Err(VmError::InvalidConfiguration(
                "TLB shootdown requires at least one remote CPU",
            ));
        }
        Ok(Self {
            generation,
            virtual_address,
            length,
            asid,
            pending,
            acknowledged: BTreeSet::new(),
        })
    }

    pub fn acknowledge(&mut self, cpu: u32, generation: u64) -> Result<bool, VmError> {
        if generation != self.generation {
            return Err(VmError::ShootdownGeneration {
                expected: self.generation,
                observed: generation,
            });
        }
        if self.acknowledged.contains(&cpu) {
            return Err(VmError::DuplicateShootdownAck(cpu));
        }
        if !self.pending.remove(&cpu) {
            return Err(VmError::InvalidShootdownCpu(cpu));
        }
        self.acknowledged.insert(cpu);
        Ok(self.pending.is_empty())
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    pub fn complete(&self) -> bool {
        self.pending.is_empty()
    }
}

fn require_alignment(address: u64, alignment: u64) -> Result<(), VmError> {
    if address & (alignment - 1) != 0 {
        Err(VmError::Misaligned { address, alignment })
    } else {
        Ok(())
    }
}

fn validate_physical_address(address: u64) -> Result<(), VmError> {
    if address & !0x000f_ffff_ffff_ffff != 0 {
        Err(VmError::InvalidPhysicalAddress(address))
    } else {
        Ok(())
    }
}

fn validate_virtual_address(architecture: VmArchitecture, address: u64) -> Result<(), VmError> {
    let bits = architecture.virtual_address_bits();
    let low_mask = (1u64 << bits) - 1;
    let upper = address & !low_mask;
    let sign = (address >> (bits - 1)) & 1;
    if (sign == 0 && upper == 0) || (sign == 1 && upper == !low_mask) {
        Ok(())
    } else {
        Err(VmError::InvalidVirtualAddress(address))
    }
}

fn validate_flags(architecture: VmArchitecture, flags: VmPageFlags) -> Result<(), VmError> {
    if !flags.contains(VmPageFlags::READ)
        && (flags.contains(VmPageFlags::WRITE) || flags.contains(VmPageFlags::EXECUTE))
        && matches!(
            architecture,
            VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48
        )
    {
        return Err(VmError::InvalidPermissions(
            "RISC-V leaf PTEs require read when write or execute is present",
        ));
    }
    if flags.contains(VmPageFlags::COW) && flags.contains(VmPageFlags::WRITE) {
        return Err(VmError::InvalidPermissions(
            "copy-on-write mappings must be hardware read-only",
        ));
    }
    if !flags.contains(VmPageFlags::READ) {
        return Err(VmError::InvalidPermissions(
            "material mappings require explicit read permission",
        ));
    }
    Ok(())
}

fn page_indices(architecture: VmArchitecture, virtual_address: u64) -> Vec<usize> {
    let levels = architecture.levels();
    (0..levels)
        .map(|depth| {
            let shift = 12 + 9 * (levels - depth - 1);
            ((virtual_address >> shift) & 0x1ff) as usize
        })
        .collect()
}

fn encode_table_pointer(architecture: VmArchitecture, physical_address: u64) -> u64 {
    match architecture {
        VmArchitecture::X86_64 => physical_address | 0x7,
        VmArchitecture::Aarch64_4K => physical_address | 0x3,
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => {
            (physical_address >> 12) << 10 | 0x1
        }
    }
}

fn decode_table_address(architecture: VmArchitecture, entry: u64) -> u64 {
    match architecture {
        VmArchitecture::X86_64 | VmArchitecture::Aarch64_4K => entry & 0x000f_ffff_ffff_f000,
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => {
            ((entry >> 10) & 0x0000_0fff_ffff_ffff) << 12
        }
    }
}

fn entry_is_table(architecture: VmArchitecture, entry: u64, _depth: usize) -> bool {
    match architecture {
        VmArchitecture::X86_64 => entry & 1 != 0 && entry & (1 << 7) == 0,
        VmArchitecture::Aarch64_4K => entry & 0x3 == 0x3,
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => {
            entry & 1 != 0 && entry & 0x0e == 0
        }
    }
}

fn encode_leaf(
    architecture: VmArchitecture,
    physical_address: u64,
    page_size: VmPageSize,
    flags: VmPageFlags,
) -> Result<u64, VmError> {
    validate_flags(architecture, flags)?;
    let entry = match architecture {
        VmArchitecture::X86_64 => {
            let mut entry = physical_address | 1;
            if flags.contains(VmPageFlags::WRITE) {
                entry |= 1 << 1;
            }
            if flags.contains(VmPageFlags::USER) {
                entry |= 1 << 2;
            }
            if flags.contains(VmPageFlags::DEVICE) {
                entry |= (1 << 3) | (1 << 4);
            }
            if flags.contains(VmPageFlags::ACCESSED) {
                entry |= 1 << 5;
            }
            if flags.contains(VmPageFlags::DIRTY) {
                entry |= 1 << 6;
            }
            if page_size != VmPageSize::Size4K {
                entry |= 1 << 7;
            }
            if flags.contains(VmPageFlags::GLOBAL) {
                entry |= 1 << 8;
            }
            if !flags.contains(VmPageFlags::EXECUTE) {
                entry |= 1 << 63;
            }
            entry
        }
        VmArchitecture::Aarch64_4K => {
            let descriptor = if page_size == VmPageSize::Size4K {
                0b11
            } else {
                0b01
            };
            let mut entry = physical_address | descriptor | (0b11 << 8) | (1 << 10);
            if flags.contains(VmPageFlags::DEVICE) {
                entry |= 1 << 2;
            }
            let ap = match (
                flags.contains(VmPageFlags::USER),
                flags.contains(VmPageFlags::WRITE),
            ) {
                (false, true) => 0b00,
                (true, true) => 0b01,
                (false, false) => 0b10,
                (true, false) => 0b11,
            };
            entry |= (ap as u64) << 6;
            if !flags.contains(VmPageFlags::GLOBAL) {
                entry |= 1 << 11;
            }
            if !flags.contains(VmPageFlags::EXECUTE) {
                entry |= (1 << 53) | (1 << 54);
            } else if flags.contains(VmPageFlags::USER) {
                entry |= 1 << 53;
            } else {
                entry |= 1 << 54;
            }
            entry
        }
        VmArchitecture::RiscvSv39 | VmArchitecture::RiscvSv48 => {
            let mut entry = (physical_address >> 12) << 10 | 1;
            if flags.contains(VmPageFlags::READ) {
                entry |= 1 << 1;
            }
            if flags.contains(VmPageFlags::WRITE) {
                entry |= 1 << 2;
            }
            if flags.contains(VmPageFlags::EXECUTE) {
                entry |= 1 << 3;
            }
            if flags.contains(VmPageFlags::USER) {
                entry |= 1 << 4;
            }
            if flags.contains(VmPageFlags::GLOBAL) {
                entry |= 1 << 5;
            }
            if flags.contains(VmPageFlags::ACCESSED) {
                entry |= 1 << 6;
            }
            if flags.contains(VmPageFlags::DIRTY) {
                entry |= 1 << 7;
            }
            entry
        }
    };
    Ok(entry)
}

fn ranges_overlap(a_base: u64, a_length: u64, b_base: u64, b_length: u64) -> bool {
    let a_end = a_base.saturating_add(a_length);
    let b_end = b_base.saturating_add(b_length);
    a_base < b_end && b_base < a_end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rw_user() -> VmPageFlags {
        VmPageFlags::READ
            .union(VmPageFlags::WRITE)
            .union(VmPageFlags::USER)
    }

    #[test]
    fn x86_64_materializes_pml4_and_huge_leaves() {
        let mut tables = HardwarePageTables::new(VmArchitecture::X86_64, 0x100000, 16, 7).unwrap();
        tables
            .map(0x400000, 0x200000, VmPageSize::Size4K, rw_user())
            .unwrap();
        tables
            .map(
                0x40000000,
                0x80000000,
                VmPageSize::Size2M,
                VmPageFlags::READ.union(VmPageFlags::EXECUTE),
            )
            .unwrap();
        assert_eq!(tables.root_entry(0).unwrap(), 0x101007);
        assert_eq!(tables.translate(0x400123).unwrap(), 0x200123);
        assert_eq!(tables.translate(0x40001234).unwrap(), 0x80001234);
        assert_eq!(tables.activation_value(), 0x100000);
        assert_eq!(
            tables.material_image().len(),
            tables.table_page_count() * 4096
        );
    }

    #[test]
    fn aarch64_materializes_ttbr_descriptors_and_xn_permissions() {
        let mut tables =
            HardwarePageTables::new(VmArchitecture::Aarch64_4K, 0x200000, 16, 0x41).unwrap();
        tables
            .map(
                0xffff800000000000,
                0x40000000,
                VmPageSize::Size1G,
                VmPageFlags::READ
                    .union(VmPageFlags::EXECUTE)
                    .union(VmPageFlags::GLOBAL),
            )
            .unwrap();
        assert_eq!(tables.root_entry(0x100).unwrap(), 0x201003);
        assert_eq!(tables.activation_value(), 0x0041_0000_0020_0000);
        assert_eq!(tables.translate(0xffff800000001234).unwrap(), 0x40001234);
    }

    #[test]
    fn riscv_sv39_materializes_satp_and_exact_pte_permissions() {
        let mut tables =
            HardwarePageTables::new(VmArchitecture::RiscvSv39, 0x300000, 16, 9).unwrap();
        let flags = VmPageFlags::READ
            .union(VmPageFlags::WRITE)
            .union(VmPageFlags::USER)
            .union(VmPageFlags::ACCESSED)
            .union(VmPageFlags::DIRTY);
        tables
            .map(0x2000, 0x9000, VmPageSize::Size4K, flags)
            .unwrap();
        assert_eq!(
            tables.activation_value(),
            (8u64 << 60) | (9u64 << 44) | 0x300
        );
        assert_eq!(tables.translate(0x2345).unwrap(), 0x9345);
        assert_eq!(tables.root_entry(0).unwrap(), (0x301u64 << 10) | 1);
    }

    #[test]
    fn cow_fault_is_explicit_and_rewrites_the_material_leaf() {
        let mut parent = HardwarePageTables::new(VmArchitecture::X86_64, 0x100000, 32, 1).unwrap();
        parent
            .map(0x8000, 0x500000, VmPageSize::Size4K, rw_user())
            .unwrap();
        let mut child = parent.fork_cow(0x200000, 32, 2).unwrap();
        assert!(matches!(
            child.fault(0x8001, VmFaultAccess::Write),
            VmFaultResolution::CopyOnWriteRequired { .. }
        ));
        child.resolve_cow(0x8000, 0x600000).unwrap();
        assert_eq!(child.translate(0x8123).unwrap(), 0x600123);
        assert_eq!(parent.translate(0x8123).unwrap(), 0x500123);
        assert_eq!(
            child.fault(0x8001, VmFaultAccess::Write),
            VmFaultResolution::Spurious
        );
    }

    #[test]
    fn guard_pages_and_shootdowns_fail_closed() {
        let mut tables =
            HardwarePageTables::new(VmArchitecture::RiscvSv48, 0x400000, 8, 3).unwrap();
        tables.reserve_guard(0x10000, 0x2000).unwrap();
        assert_eq!(
            tables.fault(0x11000, VmFaultAccess::Read),
            VmFaultResolution::GuardPage
        );
        let mut shootdown = VmTlbShootdown::new(7, 0x10000, 0x2000, 3, [1, 2]).unwrap();
        assert!(!shootdown.acknowledge(1, 7).unwrap());
        assert!(shootdown.acknowledge(2, 7).unwrap());
        assert!(shootdown.complete());
        assert_eq!(
            vm_local_invalidation_sequence(VmArchitecture::RiscvSv48)[0].bytes,
            [0x73, 0x00, 0xb5, 0x12]
        );
    }

    #[test]
    fn every_architecture_emits_real_table_bytes_and_invalidation_isa() {
        for (index, architecture) in [
            VmArchitecture::X86_64,
            VmArchitecture::Aarch64_4K,
            VmArchitecture::RiscvSv39,
            VmArchitecture::RiscvSv48,
        ]
        .into_iter()
        .enumerate()
        {
            let root = 0x1000000 + index as u64 * 0x100000;
            let mut tables = HardwarePageTables::new(architecture, root, 16, 1).unwrap();
            tables
                .map(0, 0, VmPageSize::Size4K, VmPageFlags::READ)
                .unwrap();
            assert!(tables.material_image().iter().any(|byte| *byte != 0));
            assert!(!vm_local_invalidation_sequence(architecture).is_empty());
            assert!(!vm_activation_sequence(architecture).is_empty());
            assert!(!vm_context_invalidation_sequence(architecture).is_empty());
        }
        assert_eq!(
            vm_activation_sequence(VmArchitecture::X86_64)[0].bytes,
            [0x0f, 0x22, 0xd8]
        );
        assert_eq!(
            vm_activation_sequence(VmArchitecture::Aarch64_4K)[0].bytes,
            [0x00, 0x20, 0x18, 0xd5]
        );
        assert_eq!(
            vm_activation_sequence(VmArchitecture::RiscvSv39)[0].bytes,
            [0x73, 0x10, 0x05, 0x18]
        );
        assert_eq!(
            vm_context_invalidation_sequence(VmArchitecture::X86_64)[0].bytes,
            [0x66, 0x0f, 0x38, 0x82, 0x03]
        );
        assert_eq!(
            vm_context_invalidation_sequence(VmArchitecture::Aarch64_4K)[1].bytes,
            [0x40, 0x83, 0x08, 0xd5]
        );
        assert_eq!(
            vm_context_invalidation_sequence(VmArchitecture::RiscvSv39)[0].bytes,
            [0x73, 0x00, 0xb0, 0x12]
        );
    }
}
