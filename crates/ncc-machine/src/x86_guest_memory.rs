use core::fmt;
use core::marker::PhantomData;

pub const X86_GUEST_PAGE_SIZE_4K: u64 = 1 << 12;
pub const X86_GUEST_PAGE_SIZE_2M: u64 = 1 << 21;
pub const X86_GUEST_PAGE_SIZE_1G: u64 = 1 << 30;
pub const X86_GUEST_PAGE_TABLE_ENTRIES: usize = 512;
pub const X86_PHYSICAL_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
pub const X86_PHYSICAL_ADDRESS_VALUE_MASK: u64 = 0x000f_ffff_ffff_ffff;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GuestPhysicalAddress(u64);

impl GuestPhysicalAddress {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_aligned(self, alignment: u64) -> bool {
        alignment != 0 && self.0 & (alignment - 1) == 0
    }

    pub const fn page_offset(self, size: GuestPageSize) -> u64 {
        self.0 & (size.bytes() - 1)
    }

    pub const fn table_index(self, level: u8) -> usize {
        let shift = match level {
            4 => 39,
            3 => 30,
            2 => 21,
            _ => 12,
        };
        ((self.0 >> shift) & 0x1ff) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HostPhysicalAddress(u64);

impl HostPhysicalAddress {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_aligned(self, alignment: u64) -> bool {
        alignment != 0 && self.0 & (alignment - 1) == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GuestPageSize {
    Size4K = 1,
    Size2M = 2,
    Size1G = 3,
}

impl GuestPageSize {
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Size4K => X86_GUEST_PAGE_SIZE_4K,
            Self::Size2M => X86_GUEST_PAGE_SIZE_2M,
            Self::Size1G => X86_GUEST_PAGE_SIZE_1G,
        }
    }

    pub const fn leaf_level(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct GuestPagePermissions(u8);

impl GuestPagePermissions {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER_EXECUTE: Self = Self(1 << 3);
    pub const READ_WRITE: Self = Self(Self::READ.0 | Self::WRITE.0);
    pub const READ_EXECUTE: Self = Self(Self::READ.0 | Self::EXECUTE.0);
    pub const READ_WRITE_EXECUTE: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::EXECUTE.0);

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0x0f)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EptMemoryType {
    Uncacheable = 0,
    WriteCombining = 1,
    WriteThrough = 4,
    WriteProtected = 5,
    WriteBack = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EptMapAttributes {
    pub permissions: GuestPagePermissions,
    pub memory_type: EptMemoryType,
    pub ignore_guest_pat: bool,
    pub track_accessed_dirty: bool,
    pub suppress_virtualization_exception: bool,
}

impl EptMapAttributes {
    pub const RAM: Self = Self {
        permissions: GuestPagePermissions::READ_WRITE_EXECUTE,
        memory_type: EptMemoryType::WriteBack,
        ignore_guest_pat: false,
        track_accessed_dirty: true,
        suppress_virtualization_exception: false,
    };

    pub const MMIO: Self = Self {
        permissions: GuestPagePermissions::READ_WRITE,
        memory_type: EptMemoryType::Uncacheable,
        ignore_guest_pat: true,
        track_accessed_dirty: true,
        suppress_virtualization_exception: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NptMapAttributes {
    pub permissions: GuestPagePermissions,
    pub user: bool,
    pub global: bool,
    pub write_through: bool,
    pub cache_disable: bool,
    pub pat: bool,
}

impl NptMapAttributes {
    pub const RAM: Self = Self {
        permissions: GuestPagePermissions::READ_WRITE_EXECUTE,
        user: true,
        global: false,
        write_through: false,
        cache_disable: false,
        pat: false,
    };

    pub const MMIO: Self = Self {
        permissions: GuestPagePermissions::READ_WRITE,
        user: true,
        global: false,
        write_through: false,
        cache_disable: true,
        pat: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemoryError {
    InvalidPhysicalAddress(u64),
    MisalignedAddress {
        address: u64,
        alignment: u64,
    },
    EmptyPermissions,
    LengthNotPageMultiple(u64),
    AddressOverflow,
    TablePoolExhausted,
    TableNotOwned(u64),
    MappingAlreadyPresent(u64),
    MappingNotPresent(u64),
    ReplacementPageSizeMismatch {
        guest_physical: u64,
        existing: GuestPageSize,
        requested: GuestPageSize,
    },
    LeafConflictsWithTable {
        guest_physical: u64,
        level: u8,
    },
    TableConflictsWithLeaf {
        guest_physical: u64,
        level: u8,
    },
    RegionCapacityExhausted,
    RegionOverlap {
        start: u64,
        end: u64,
    },
}

impl fmt::Display for GuestMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPhysicalAddress(address) => {
                write!(f, "physical address 0x{address:x} exceeds x86-64 width")
            }
            Self::MisalignedAddress { address, alignment } => write!(
                f,
                "physical address 0x{address:x} is not aligned to 0x{alignment:x}"
            ),
            Self::EmptyPermissions => write!(f, "a second-level mapping needs permissions"),
            Self::LengthNotPageMultiple(length) => write!(
                f,
                "guest memory length 0x{length:x} is not a multiple of 4 KiB"
            ),
            Self::AddressOverflow => write!(f, "guest physical range overflows u64"),
            Self::TablePoolExhausted => write!(f, "second-level page-table pool is exhausted"),
            Self::TableNotOwned(address) => {
                write!(
                    f,
                    "page-table physical address 0x{address:x} is not in the pool"
                )
            }
            Self::MappingAlreadyPresent(address) => {
                write!(f, "guest physical address 0x{address:x} is already mapped")
            }
            Self::MappingNotPresent(address) => {
                write!(f, "guest physical address 0x{address:x} is not mapped")
            }
            Self::ReplacementPageSizeMismatch {
                guest_physical,
                existing,
                requested,
            } => write!(
                f,
                "guest physical address 0x{guest_physical:x} is mapped as {existing:?}, not {requested:?}"
            ),
            Self::LeafConflictsWithTable {
                guest_physical,
                level,
            } => write!(
                f,
                "guest address 0x{guest_physical:x} needs a level-{level} leaf but a table exists"
            ),
            Self::TableConflictsWithLeaf {
                guest_physical,
                level,
            } => write!(
                f,
                "guest address 0x{guest_physical:x} needs a level-{level} table but a leaf exists"
            ),
            Self::RegionCapacityExhausted => write!(f, "guest physical region map is full"),
            Self::RegionOverlap { start, end } => write!(
                f,
                "guest physical region 0x{start:x}..0x{end:x} overlaps an existing region"
            ),
        }
    }
}

impl std::error::Error for GuestMemoryError {}

fn validate_physical(address: u64) -> Result<(), GuestMemoryError> {
    if address & !X86_PHYSICAL_ADDRESS_VALUE_MASK != 0 {
        return Err(GuestMemoryError::InvalidPhysicalAddress(address));
    }
    Ok(())
}

fn validate_mapping(
    guest: GuestPhysicalAddress,
    host: HostPhysicalAddress,
    size: GuestPageSize,
    permissions: GuestPagePermissions,
) -> Result<(), GuestMemoryError> {
    validate_physical(guest.raw())?;
    validate_physical(host.raw())?;
    if !guest.is_aligned(size.bytes()) {
        return Err(GuestMemoryError::MisalignedAddress {
            address: guest.raw(),
            alignment: size.bytes(),
        });
    }
    if !host.is_aligned(size.bytes()) {
        return Err(GuestMemoryError::MisalignedAddress {
            address: host.raw(),
            alignment: size.bytes(),
        });
    }
    if permissions.is_empty() {
        return Err(GuestMemoryError::EmptyPermissions);
    }
    Ok(())
}

#[repr(C, align(4096))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct X86SecondLevelTable {
    entries: [u64; X86_GUEST_PAGE_TABLE_ENTRIES],
}

impl X86SecondLevelTable {
    pub const ZEROED: Self = Self {
        entries: [0; X86_GUEST_PAGE_TABLE_ENTRIES],
    };

    pub const fn entries(&self) -> &[u64; X86_GUEST_PAGE_TABLE_ENTRIES] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [u64; X86_GUEST_PAGE_TABLE_ENTRIES] {
        &mut self.entries
    }

    pub fn clear(&mut self) {
        self.entries.fill(0);
    }
}

#[derive(Clone, Copy)]
struct TableSlot {
    physical: HostPhysicalAddress,
    used: bool,
    table: X86SecondLevelTable,
}

impl TableSlot {
    const EMPTY: Self = Self {
        physical: HostPhysicalAddress::new(0),
        used: false,
        table: X86SecondLevelTable::ZEROED,
    };
}

pub trait SecondLevelFormat {
    type Attributes: Copy;

    fn permissions(attributes: Self::Attributes) -> GuestPagePermissions;
    fn table_entry(address: HostPhysicalAddress) -> u64;
    fn leaf_entry(
        address: HostPhysicalAddress,
        size: GuestPageSize,
        attributes: Self::Attributes,
    ) -> u64;
    fn is_present(entry: u64) -> bool;
    fn is_leaf(entry: u64, level: u8) -> bool;
    fn address(entry: u64) -> HostPhysicalAddress;
    fn entry_permissions(entry: u64) -> GuestPagePermissions;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntelEpt;

impl SecondLevelFormat for IntelEpt {
    type Attributes = EptMapAttributes;

    fn permissions(attributes: Self::Attributes) -> GuestPagePermissions {
        attributes.permissions
    }

    fn table_entry(address: HostPhysicalAddress) -> u64 {
        address.raw() | 0x7
    }

    fn leaf_entry(
        address: HostPhysicalAddress,
        size: GuestPageSize,
        attributes: Self::Attributes,
    ) -> u64 {
        let permissions = attributes.permissions.bits() as u64 & 0x7;
        let user_execute = if attributes
            .permissions
            .contains(GuestPagePermissions::USER_EXECUTE)
        {
            1 << 10
        } else {
            0
        };
        address.raw()
            | permissions
            | ((attributes.memory_type as u64) << 3)
            | (u64::from(attributes.ignore_guest_pat) << 6)
            | (u64::from(size != GuestPageSize::Size4K) << 7)
            | user_execute
            | (u64::from(attributes.suppress_virtualization_exception) << 63)
    }

    fn is_present(entry: u64) -> bool {
        entry & 0x7 != 0
    }

    fn is_leaf(entry: u64, level: u8) -> bool {
        level == 1 || entry & (1 << 7) != 0
    }

    fn address(entry: u64) -> HostPhysicalAddress {
        HostPhysicalAddress::new(entry & X86_PHYSICAL_ADDRESS_MASK)
    }

    fn entry_permissions(entry: u64) -> GuestPagePermissions {
        let mut bits = (entry & 0x7) as u8;
        if entry & (1 << 10) != 0 {
            bits |= GuestPagePermissions::USER_EXECUTE.bits();
        }
        GuestPagePermissions::from_bits(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmdNpt;

impl SecondLevelFormat for AmdNpt {
    type Attributes = NptMapAttributes;

    fn permissions(attributes: Self::Attributes) -> GuestPagePermissions {
        attributes.permissions
    }

    fn table_entry(address: HostPhysicalAddress) -> u64 {
        address.raw() | 0x7
    }

    fn leaf_entry(
        address: HostPhysicalAddress,
        size: GuestPageSize,
        attributes: Self::Attributes,
    ) -> u64 {
        let mut entry = address.raw() | 1;
        if attributes.permissions.contains(GuestPagePermissions::WRITE) {
            entry |= 1 << 1;
        }
        if attributes.user {
            entry |= 1 << 2;
        }
        if attributes.write_through {
            entry |= 1 << 3;
        }
        if attributes.cache_disable {
            entry |= 1 << 4;
        }
        if size != GuestPageSize::Size4K {
            entry |= 1 << 7;
        }
        if attributes.global {
            entry |= 1 << 8;
        }
        if attributes.pat {
            entry |= if size == GuestPageSize::Size4K {
                1 << 7
            } else {
                1 << 12
            };
        }
        if !attributes
            .permissions
            .contains(GuestPagePermissions::EXECUTE)
        {
            entry |= 1 << 63;
        }
        entry
    }

    fn is_present(entry: u64) -> bool {
        entry & 1 != 0
    }

    fn is_leaf(entry: u64, level: u8) -> bool {
        level == 1 || entry & (1 << 7) != 0
    }

    fn address(entry: u64) -> HostPhysicalAddress {
        HostPhysicalAddress::new(entry & X86_PHYSICAL_ADDRESS_MASK)
    }

    fn entry_permissions(entry: u64) -> GuestPagePermissions {
        if entry & 1 == 0 {
            return GuestPagePermissions::NONE;
        }
        let mut permissions = GuestPagePermissions::READ;
        if entry & (1 << 1) != 0 {
            permissions = permissions.union(GuestPagePermissions::WRITE);
        }
        if entry & (1 << 63) == 0 {
            permissions = permissions.union(GuestPagePermissions::EXECUTE);
        }
        permissions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTranslation {
    pub guest_physical: GuestPhysicalAddress,
    pub host_physical: HostPhysicalAddress,
    pub page_size: GuestPageSize,
    pub permissions: GuestPagePermissions,
}

pub struct X86SecondLevelPageTables<F: SecondLevelFormat, const TABLES: usize> {
    slots: [TableSlot; TABLES],
    root: HostPhysicalAddress,
    marker: PhantomData<F>,
}

pub type IntelEptPageTables<const TABLES: usize> = X86SecondLevelPageTables<IntelEpt, TABLES>;
pub type AmdNptPageTables<const TABLES: usize> = X86SecondLevelPageTables<AmdNpt, TABLES>;

impl<F: SecondLevelFormat, const TABLES: usize> X86SecondLevelPageTables<F, TABLES> {
    pub fn new(table_pool_physical_base: HostPhysicalAddress) -> Result<Self, GuestMemoryError> {
        if TABLES == 0 {
            return Err(GuestMemoryError::TablePoolExhausted);
        }
        validate_physical(table_pool_physical_base.raw())?;
        if !table_pool_physical_base.is_aligned(X86_GUEST_PAGE_SIZE_4K) {
            return Err(GuestMemoryError::MisalignedAddress {
                address: table_pool_physical_base.raw(),
                alignment: X86_GUEST_PAGE_SIZE_4K,
            });
        }
        let mut slots = [TableSlot::EMPTY; TABLES];
        for (index, slot) in slots.iter_mut().enumerate() {
            let physical = table_pool_physical_base
                .raw()
                .checked_add(index as u64 * X86_GUEST_PAGE_SIZE_4K)
                .ok_or(GuestMemoryError::AddressOverflow)?;
            validate_physical(physical)?;
            slot.physical = HostPhysicalAddress::new(physical);
        }
        slots[0].used = true;
        Ok(Self {
            slots,
            root: table_pool_physical_base,
            marker: PhantomData,
        })
    }

    pub const fn root_physical(&self) -> HostPhysicalAddress {
        self.root
    }

    pub fn used_table_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.used).count()
    }

    pub fn table(&self, physical: HostPhysicalAddress) -> Option<&X86SecondLevelTable> {
        self.slots
            .iter()
            .find(|slot| slot.used && slot.physical == physical)
            .map(|slot| &slot.table)
    }

    pub fn table_mut(&mut self, physical: HostPhysicalAddress) -> Option<&mut X86SecondLevelTable> {
        self.slots
            .iter_mut()
            .find(|slot| slot.used && slot.physical == physical)
            .map(|slot| &mut slot.table)
    }

    fn allocate_table(&mut self) -> Result<HostPhysicalAddress, GuestMemoryError> {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| !slot.used)
            .ok_or(GuestMemoryError::TablePoolExhausted)?;
        slot.used = true;
        slot.table.clear();
        Ok(slot.physical)
    }

    fn release_table(&mut self, physical: HostPhysicalAddress) -> Result<(), GuestMemoryError> {
        if physical == self.root {
            return Ok(());
        }
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.used && slot.physical == physical)
            .ok_or(GuestMemoryError::TableNotOwned(physical.raw()))?;
        slot.table.clear();
        slot.used = false;
        Ok(())
    }

    fn entry(&self, table: HostPhysicalAddress, index: usize) -> Result<u64, GuestMemoryError> {
        self.table(table)
            .map(|page| page.entries[index])
            .ok_or(GuestMemoryError::TableNotOwned(table.raw()))
    }

    fn set_entry(
        &mut self,
        table: HostPhysicalAddress,
        index: usize,
        value: u64,
    ) -> Result<(), GuestMemoryError> {
        self.table_mut(table)
            .map(|page| page.entries[index] = value)
            .ok_or(GuestMemoryError::TableNotOwned(table.raw()))
    }

    fn rollback_allocated_tables(
        &mut self,
        allocations: &[(HostPhysicalAddress, usize, HostPhysicalAddress); 3],
        allocation_count: usize,
    ) {
        for &(parent, index, child) in allocations[..allocation_count].iter().rev() {
            let _ = self.set_entry(parent, index, 0);
            let _ = self.release_table(child);
        }
    }

    pub fn map(
        &mut self,
        guest: GuestPhysicalAddress,
        host: HostPhysicalAddress,
        size: GuestPageSize,
        attributes: F::Attributes,
    ) -> Result<(), GuestMemoryError> {
        validate_mapping(guest, host, size, F::permissions(attributes))?;
        let mut current = self.root;
        let mut allocations = [(HostPhysicalAddress::new(0), 0, HostPhysicalAddress::new(0)); 3];
        let mut allocation_count = 0;
        for level in (size.leaf_level() + 1..=4).rev() {
            let index = guest.table_index(level);
            let entry = match self.entry(current, index) {
                Ok(entry) => entry,
                Err(error) => {
                    self.rollback_allocated_tables(&allocations, allocation_count);
                    return Err(error);
                }
            };
            if F::is_present(entry) {
                if F::is_leaf(entry, level) {
                    self.rollback_allocated_tables(&allocations, allocation_count);
                    return Err(GuestMemoryError::TableConflictsWithLeaf {
                        guest_physical: guest.raw(),
                        level,
                    });
                }
                current = F::address(entry);
                if self.table(current).is_none() {
                    self.rollback_allocated_tables(&allocations, allocation_count);
                    return Err(GuestMemoryError::TableNotOwned(current.raw()));
                }
            } else {
                let child = match self.allocate_table() {
                    Ok(child) => child,
                    Err(error) => {
                        self.rollback_allocated_tables(&allocations, allocation_count);
                        return Err(error);
                    }
                };
                if let Err(error) = self.set_entry(current, index, F::table_entry(child)) {
                    let _ = self.release_table(child);
                    self.rollback_allocated_tables(&allocations, allocation_count);
                    return Err(error);
                }
                allocations[allocation_count] = (current, index, child);
                allocation_count += 1;
                current = child;
            }
        }

        let leaf_index = guest.table_index(size.leaf_level());
        let existing = match self.entry(current, leaf_index) {
            Ok(entry) => entry,
            Err(error) => {
                self.rollback_allocated_tables(&allocations, allocation_count);
                return Err(error);
            }
        };
        if F::is_present(existing) {
            self.rollback_allocated_tables(&allocations, allocation_count);
            if !F::is_leaf(existing, size.leaf_level()) {
                return Err(GuestMemoryError::LeafConflictsWithTable {
                    guest_physical: guest.raw(),
                    level: size.leaf_level(),
                });
            }
            return Err(GuestMemoryError::MappingAlreadyPresent(guest.raw()));
        }
        if let Err(error) =
            self.set_entry(current, leaf_index, F::leaf_entry(host, size, attributes))
        {
            self.rollback_allocated_tables(&allocations, allocation_count);
            return Err(error);
        }
        Ok(())
    }

    pub fn replace(
        &mut self,
        guest: GuestPhysicalAddress,
        host: HostPhysicalAddress,
        size: GuestPageSize,
        attributes: F::Attributes,
    ) -> Result<(), GuestMemoryError> {
        validate_mapping(guest, host, size, F::permissions(attributes))?;
        let translation = self.translate(guest)?;
        if translation.page_size != size {
            return Err(GuestMemoryError::ReplacementPageSizeMismatch {
                guest_physical: guest.raw(),
                existing: translation.page_size,
                requested: size,
            });
        }
        let mut current = self.root;
        for level in (size.leaf_level() + 1..=4).rev() {
            let entry = self.entry(current, guest.table_index(level))?;
            if !F::is_present(entry) || F::is_leaf(entry, level) {
                return Err(GuestMemoryError::MappingNotPresent(guest.raw()));
            }
            current = F::address(entry);
        }
        let leaf_index = guest.table_index(size.leaf_level());
        let existing = self.entry(current, leaf_index)?;
        if !F::is_present(existing) || !F::is_leaf(existing, size.leaf_level()) {
            return Err(GuestMemoryError::MappingNotPresent(guest.raw()));
        }
        self.set_entry(current, leaf_index, F::leaf_entry(host, size, attributes))
    }

    pub fn map_range(
        &mut self,
        guest_start: GuestPhysicalAddress,
        host_start: HostPhysicalAddress,
        length: u64,
        attributes: F::Attributes,
        allow_huge_pages: bool,
    ) -> Result<(), GuestMemoryError> {
        if length == 0 || length & (X86_GUEST_PAGE_SIZE_4K - 1) != 0 {
            return Err(GuestMemoryError::LengthNotPageMultiple(length));
        }
        let guest_end = guest_start
            .raw()
            .checked_add(length)
            .ok_or(GuestMemoryError::AddressOverflow)?;
        host_start
            .raw()
            .checked_add(length)
            .ok_or(GuestMemoryError::AddressOverflow)?;

        let mut guest = guest_start.raw();
        let mut host = host_start.raw();
        while guest < guest_end {
            let remaining = guest_end - guest;
            let size = if allow_huge_pages
                && remaining >= X86_GUEST_PAGE_SIZE_1G
                && guest & (X86_GUEST_PAGE_SIZE_1G - 1) == 0
                && host & (X86_GUEST_PAGE_SIZE_1G - 1) == 0
            {
                GuestPageSize::Size1G
            } else if allow_huge_pages
                && remaining >= X86_GUEST_PAGE_SIZE_2M
                && guest & (X86_GUEST_PAGE_SIZE_2M - 1) == 0
                && host & (X86_GUEST_PAGE_SIZE_2M - 1) == 0
            {
                GuestPageSize::Size2M
            } else {
                GuestPageSize::Size4K
            };
            if let Err(error) = self.map(
                GuestPhysicalAddress::new(guest),
                HostPhysicalAddress::new(host),
                size,
                attributes,
            ) {
                let mut rollback = guest_start.raw();
                while rollback < guest {
                    let step = self
                        .translate(GuestPhysicalAddress::new(rollback))
                        .map(|translation| translation.page_size.bytes())
                        .unwrap_or(X86_GUEST_PAGE_SIZE_4K);
                    let _ = self.unmap(GuestPhysicalAddress::new(rollback));
                    rollback += step;
                }
                return Err(error);
            }
            guest += size.bytes();
            host += size.bytes();
        }
        Ok(())
    }

    pub fn translate(
        &self,
        guest: GuestPhysicalAddress,
    ) -> Result<GuestTranslation, GuestMemoryError> {
        validate_physical(guest.raw())?;
        let mut current = self.root;
        let mut effective =
            GuestPagePermissions::READ_WRITE_EXECUTE.union(GuestPagePermissions::USER_EXECUTE);
        for level in (1..=4).rev() {
            let entry = self.entry(current, guest.table_index(level))?;
            if !F::is_present(entry) {
                return Err(GuestMemoryError::MappingNotPresent(guest.raw()));
            }
            effective = effective.intersect(F::entry_permissions(entry));
            if F::is_leaf(entry, level) {
                let page_size = match level {
                    3 => GuestPageSize::Size1G,
                    2 => GuestPageSize::Size2M,
                    _ => GuestPageSize::Size4K,
                };
                let offset = guest.page_offset(page_size);
                let host = F::address(entry)
                    .raw()
                    .checked_add(offset)
                    .ok_or(GuestMemoryError::AddressOverflow)?;
                return Ok(GuestTranslation {
                    guest_physical: guest,
                    host_physical: HostPhysicalAddress::new(host),
                    page_size,
                    permissions: effective,
                });
            }
            current = F::address(entry);
            if self.table(current).is_none() {
                return Err(GuestMemoryError::TableNotOwned(current.raw()));
            }
        }
        Err(GuestMemoryError::MappingNotPresent(guest.raw()))
    }

    pub fn unmap(
        &mut self,
        guest: GuestPhysicalAddress,
    ) -> Result<GuestTranslation, GuestMemoryError> {
        let translation = self.translate(guest)?;
        let guest_base =
            GuestPhysicalAddress::new(guest.raw() & !(translation.page_size.bytes() - 1));
        let leaf_level = translation.page_size.leaf_level();
        let mut tables = [HostPhysicalAddress::new(0); 4];
        let mut indexes = [0usize; 4];
        let mut current = self.root;
        tables[3] = current;
        for level in (leaf_level + 1..=4).rev() {
            let slot = (level - 1) as usize;
            let index = guest_base.table_index(level);
            indexes[slot] = index;
            let entry = self.entry(current, index)?;
            current = F::address(entry);
            tables[(level - 2) as usize] = current;
        }
        let leaf_table = tables[(leaf_level - 1) as usize];
        self.set_entry(leaf_table, guest_base.table_index(leaf_level), 0)?;

        for level in leaf_level..4 {
            let table = tables[(level - 1) as usize];
            let empty = self
                .table(table)
                .ok_or(GuestMemoryError::TableNotOwned(table.raw()))?
                .entries
                .iter()
                .all(|entry| *entry == 0);
            if !empty || table == self.root {
                break;
            }
            self.release_table(table)?;
            let parent = tables[level as usize];
            self.set_entry(parent, indexes[level as usize], 0)?;
        }
        Ok(translation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct EptPointer(u64);

impl EptPointer {
    pub fn new(
        root: HostPhysicalAddress,
        memory_type: EptMemoryType,
        accessed_dirty: bool,
    ) -> Result<Self, GuestMemoryError> {
        validate_physical(root.raw())?;
        if !root.is_aligned(X86_GUEST_PAGE_SIZE_4K) {
            return Err(GuestMemoryError::MisalignedAddress {
                address: root.raw(),
                alignment: X86_GUEST_PAGE_SIZE_4K,
            });
        }
        Ok(Self(
            root.raw() | memory_type as u64 | (3 << 3) | (u64::from(accessed_dirty) << 6),
        ))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn root(self) -> HostPhysicalAddress {
        HostPhysicalAddress::new(self.0 & X86_PHYSICAL_ADDRESS_MASK)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemoryRegionKind {
    Ram,
    ReadOnlyMemory,
    Mmio,
    Shared,
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestMemoryRegion {
    pub guest_start: GuestPhysicalAddress,
    pub host_start: HostPhysicalAddress,
    pub length: u64,
    pub kind: GuestMemoryRegionKind,
    pub permissions: GuestPagePermissions,
}

impl GuestMemoryRegion {
    pub fn end(self) -> Result<u64, GuestMemoryError> {
        self.guest_start
            .raw()
            .checked_add(self.length)
            .ok_or(GuestMemoryError::AddressOverflow)
    }

    pub fn contains(self, address: GuestPhysicalAddress) -> bool {
        self.end()
            .is_ok_and(|end| address.raw() >= self.guest_start.raw() && address.raw() < end)
    }

    pub fn resolve(
        self,
        address: GuestPhysicalAddress,
    ) -> Result<HostPhysicalAddress, GuestMemoryError> {
        if !self.contains(address) {
            return Err(GuestMemoryError::MappingNotPresent(address.raw()));
        }
        let offset = address.raw() - self.guest_start.raw();
        Ok(HostPhysicalAddress::new(
            self.host_start
                .raw()
                .checked_add(offset)
                .ok_or(GuestMemoryError::AddressOverflow)?,
        ))
    }
}

pub struct GuestPhysicalMemoryMap<const REGIONS: usize> {
    regions: [Option<GuestMemoryRegion>; REGIONS],
}

impl<const REGIONS: usize> Default for GuestPhysicalMemoryMap<REGIONS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const REGIONS: usize> GuestPhysicalMemoryMap<REGIONS> {
    pub const fn new() -> Self {
        Self {
            regions: [None; REGIONS],
        }
    }

    pub fn region_count(&self) -> usize {
        self.regions.iter().flatten().count()
    }

    pub fn insert(&mut self, region: GuestMemoryRegion) -> Result<(), GuestMemoryError> {
        if region.length == 0 {
            return Err(GuestMemoryError::LengthNotPageMultiple(0));
        }
        validate_physical(region.guest_start.raw())?;
        validate_physical(region.host_start.raw())?;
        let end = region.end()?;
        for existing in self.regions.iter().flatten() {
            let existing_end = existing.end()?;
            if region.guest_start.raw() < existing_end && end > existing.guest_start.raw() {
                return Err(GuestMemoryError::RegionOverlap {
                    start: region.guest_start.raw(),
                    end,
                });
            }
        }
        let slot = self
            .regions
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(GuestMemoryError::RegionCapacityExhausted)?;
        *slot = Some(region);
        Ok(())
    }

    pub fn remove(&mut self, guest_start: GuestPhysicalAddress) -> Option<GuestMemoryRegion> {
        let slot = self
            .regions
            .iter_mut()
            .find(|slot| slot.is_some_and(|region| region.guest_start == guest_start))?;
        slot.take()
    }

    pub fn resolve(
        &self,
        address: GuestPhysicalAddress,
    ) -> Result<(HostPhysicalAddress, GuestMemoryRegion), GuestMemoryError> {
        let region = self
            .regions
            .iter()
            .flatten()
            .copied()
            .find(|region| region.contains(address))
            .ok_or(GuestMemoryError::MappingNotPresent(address.raw()))?;
        Ok((region.resolve(address)?, region))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_table_and_pointer_layouts_are_exact() {
        assert_eq!(core::mem::size_of::<X86SecondLevelTable>(), 4096);
        assert_eq!(core::mem::align_of::<X86SecondLevelTable>(), 4096);
        let eptp = EptPointer::new(
            HostPhysicalAddress::new(0x20_0000),
            EptMemoryType::WriteBack,
            true,
        )
        .unwrap();
        assert_eq!(eptp.raw(), 0x20_0000 | 6 | (3 << 3) | (1 << 6));
        assert_eq!(eptp.root(), HostPhysicalAddress::new(0x20_0000));
    }

    #[test]
    fn ept_maps_translates_replaces_and_reclaims_all_page_sizes() {
        let mut ept = IntelEptPageTables::<16>::new(HostPhysicalAddress::new(0x10_0000)).unwrap();
        ept.map(
            GuestPhysicalAddress::new(0),
            HostPhysicalAddress::new(0x4000_0000),
            GuestPageSize::Size1G,
            EptMapAttributes::RAM,
        )
        .unwrap();
        let root_entry = ept.table(ept.root_physical()).expect("EPT root").entries()[0];
        let pdpt = <IntelEpt as SecondLevelFormat>::address(root_entry);
        let one_gib_leaf = ept.table(pdpt).expect("EPT PDPT").entries()[0];
        assert_eq!(one_gib_leaf & ((1 << 8) | (1 << 9)), 0);
        ept.map(
            GuestPhysicalAddress::new(0x4000_0000),
            HostPhysicalAddress::new(0x8000_0000),
            GuestPageSize::Size2M,
            EptMapAttributes::RAM,
        )
        .unwrap();
        ept.map(
            GuestPhysicalAddress::new(0x4020_0000),
            HostPhysicalAddress::new(0xc000_0000),
            GuestPageSize::Size4K,
            EptMapAttributes::MMIO,
        )
        .unwrap();

        let one_gib = ept
            .translate(GuestPhysicalAddress::new(0x1234_5000))
            .unwrap();
        assert_eq!(one_gib.host_physical.raw(), 0x5234_5000);
        assert_eq!(one_gib.page_size, GuestPageSize::Size1G);
        let mmio = ept
            .translate(GuestPhysicalAddress::new(0x4020_0123))
            .unwrap();
        assert_eq!(mmio.host_physical.raw(), 0xc000_0123);
        assert_eq!(mmio.page_size, GuestPageSize::Size4K);
        assert_eq!(mmio.permissions, GuestPagePermissions::READ_WRITE);

        ept.replace(
            GuestPhysicalAddress::new(0x4020_0000),
            HostPhysicalAddress::new(0xd000_0000),
            GuestPageSize::Size4K,
            EptMapAttributes::RAM,
        )
        .unwrap();
        assert_eq!(
            ept.translate(GuestPhysicalAddress::new(0x4020_0042))
                .unwrap()
                .host_physical
                .raw(),
            0xd000_0042
        );
        ept.unmap(GuestPhysicalAddress::new(0x4020_0000)).unwrap();
        assert!(matches!(
            ept.translate(GuestPhysicalAddress::new(0x4020_0000)),
            Err(GuestMemoryError::MappingNotPresent(_))
        ));
    }

    #[test]
    fn npt_encodes_nx_and_translates_a_transactional_range() {
        let mut npt = AmdNptPageTables::<16>::new(HostPhysicalAddress::new(0x30_0000)).unwrap();
        let read_write = NptMapAttributes {
            permissions: GuestPagePermissions::READ_WRITE,
            ..NptMapAttributes::RAM
        };
        npt.map_range(
            GuestPhysicalAddress::new(0x20_0000),
            HostPhysicalAddress::new(0x1000_0000),
            2 * X86_GUEST_PAGE_SIZE_2M,
            read_write,
            true,
        )
        .unwrap();
        let translated = npt.translate(GuestPhysicalAddress::new(0x5ff000)).unwrap();
        assert_eq!(translated.host_physical.raw(), 0x103f_f000);
        assert_eq!(translated.page_size, GuestPageSize::Size2M);
        assert!(!translated
            .permissions
            .contains(GuestPagePermissions::EXECUTE));
    }

    #[test]
    fn overlapping_regions_and_failed_ranges_do_not_corrupt_the_map() {
        let mut regions = GuestPhysicalMemoryMap::<4>::new();
        regions
            .insert(GuestMemoryRegion {
                guest_start: GuestPhysicalAddress::new(0),
                host_start: HostPhysicalAddress::new(0x1000_0000),
                length: 0x20_0000,
                kind: GuestMemoryRegionKind::Ram,
                permissions: GuestPagePermissions::READ_WRITE_EXECUTE,
            })
            .unwrap();
        assert!(matches!(
            regions.insert(GuestMemoryRegion {
                guest_start: GuestPhysicalAddress::new(0x10_0000),
                host_start: HostPhysicalAddress::new(0x2000_0000),
                length: 0x20_0000,
                kind: GuestMemoryRegionKind::Mmio,
                permissions: GuestPagePermissions::READ_WRITE,
            }),
            Err(GuestMemoryError::RegionOverlap { .. })
        ));
        let (host, region) = regions.resolve(GuestPhysicalAddress::new(0x1234)).unwrap();
        assert_eq!(host.raw(), 0x1000_1234);
        assert_eq!(region.kind, GuestMemoryRegionKind::Ram);
    }

    #[test]
    fn failed_map_and_page_size_change_leave_existing_tables_unchanged() {
        let mut exhausted =
            IntelEptPageTables::<2>::new(HostPhysicalAddress::new(0x10_0000)).unwrap();
        assert_eq!(
            exhausted.map(
                GuestPhysicalAddress::new(0),
                HostPhysicalAddress::new(0x20_0000),
                GuestPageSize::Size4K,
                EptMapAttributes::RAM,
            ),
            Err(GuestMemoryError::TablePoolExhausted)
        );
        assert_eq!(exhausted.used_table_count(), 1);
        assert!(exhausted
            .table(exhausted.root_physical())
            .unwrap()
            .entries()
            .iter()
            .all(|entry| *entry == 0));

        let mut ept = IntelEptPageTables::<8>::new(HostPhysicalAddress::new(0x30_0000)).unwrap();
        ept.map(
            GuestPhysicalAddress::new(0x40_0000),
            HostPhysicalAddress::new(0x80_0000),
            GuestPageSize::Size2M,
            EptMapAttributes::RAM,
        )
        .unwrap();
        let before = ept.translate(GuestPhysicalAddress::new(0x40_1234)).unwrap();
        assert!(matches!(
            ept.replace(
                GuestPhysicalAddress::new(0x40_0000),
                HostPhysicalAddress::new(0x90_0000),
                GuestPageSize::Size4K,
                EptMapAttributes::RAM,
            ),
            Err(GuestMemoryError::ReplacementPageSizeMismatch { .. })
        ));
        assert_eq!(
            ept.translate(GuestPhysicalAddress::new(0x40_1234)).unwrap(),
            before
        );
    }
}
