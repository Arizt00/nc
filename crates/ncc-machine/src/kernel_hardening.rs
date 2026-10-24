use std::error::Error;
use std::fmt;

pub const KERNEL_HARDENING_SCHEMA: &str = "ncc.machine.kernel_hardening.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardeningArchitecture {
    X86_64,
    I386Pae,
    Aarch64,
    Armv7Lpae,
    Riscv32,
    Riscv64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HardeningPolicy {
    pub aslr: bool,
    pub kaslr: bool,
    pub enforce_w_x: bool,
    pub nx: bool,
    pub stack_canary: bool,
    pub sanitizer: bool,
    pub permit_explicit_rwx: bool,
}

impl HardeningPolicy {
    pub const RAW: Self = Self {
        aslr: false,
        kaslr: false,
        enforce_w_x: false,
        nx: false,
        stack_canary: false,
        sanitizer: false,
        permit_explicit_rwx: false,
    };

    pub const fn enabled_count(self) -> usize {
        self.aslr as usize
            + self.kaslr as usize
            + self.enforce_w_x as usize
            + self.nx as usize
            + self.stack_canary as usize
            + self.sanitizer as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardeningError {
    InvalidConfiguration(&'static str),
    AddressOverflow,
    Misaligned { value: u64, alignment: u64 },
    NoKaslrSlot,
    WriteExecuteDenied,
    ExecuteDenied,
    CanaryMismatch { expected: u64, observed: u64 },
    ShadowOutsideRange { address: u64, bytes: usize },
    PoisonedAccess { address: u64, bytes: usize },
    RelocationOutsideImage { offset: usize, width: usize },
    RelocationOverflow,
}

impl fmt::Display for HardeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => f.write_str(message),
            Self::AddressOverflow => f.write_str("hardening address arithmetic overflow"),
            Self::Misaligned { value, alignment } => {
                write!(f, "value {value:#x} is not aligned to {alignment:#x}")
            }
            Self::NoKaslrSlot => f.write_str("KASLR range has no aligned image slot"),
            Self::WriteExecuteDenied => f.write_str("W^X rejected writable executable memory"),
            Self::ExecuteDenied => f.write_str("NX policy rejected executable transition"),
            Self::CanaryMismatch { expected, observed } => write!(
                f,
                "kernel stack canary mismatch: expected {expected:#x}, observed {observed:#x}"
            ),
            Self::ShadowOutsideRange { address, bytes } => {
                write!(f, "sanitizer shadow does not cover {address:#x}+{bytes:#x}")
            }
            Self::PoisonedAccess { address, bytes } => {
                write!(
                    f,
                    "sanitizer rejected poisoned access {address:#x}+{bytes:#x}"
                )
            }
            Self::RelocationOutsideImage { offset, width } => write!(
                f,
                "relocation {offset:#x}+{width:#x} is outside the kernel image"
            ),
            Self::RelocationOverflow => f.write_str("kernel relocation value overflow"),
        }
    }
}

impl Error for HardeningError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PagePermissions(u8);

impl PagePermissions {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);
    pub const GLOBAL: Self = Self(1 << 4);
    pub const DEVICE: Self = Self(1 << 5);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn bits(self) -> u8 {
        self.0
    }
}

pub fn validate_permission_transition(
    policy: HardeningPolicy,
    previous: PagePermissions,
    next: PagePermissions,
) -> Result<(), HardeningError> {
    let next_wx = next.contains(PagePermissions::WRITE) && next.contains(PagePermissions::EXECUTE);
    if policy.enforce_w_x && next_wx && !policy.permit_explicit_rwx {
        return Err(HardeningError::WriteExecuteDenied);
    }
    if policy.nx
        && !previous.contains(PagePermissions::EXECUTE)
        && next.contains(PagePermissions::EXECUTE)
        && next.contains(PagePermissions::WRITE)
        && !policy.permit_explicit_rwx
    {
        return Err(HardeningError::ExecuteDenied);
    }
    Ok(())
}

pub fn encode_leaf_page_entry(
    architecture: HardeningArchitecture,
    physical_address: u64,
    permissions: PagePermissions,
    policy: HardeningPolicy,
) -> Result<u64, HardeningError> {
    if physical_address & 0xfff != 0 {
        return Err(HardeningError::Misaligned {
            value: physical_address,
            alignment: 4096,
        });
    }
    validate_permission_transition(policy, PagePermissions::default(), permissions)?;
    let executable = permissions.contains(PagePermissions::EXECUTE);
    let writable = permissions.contains(PagePermissions::WRITE);
    let user = permissions.contains(PagePermissions::USER);
    let global = permissions.contains(PagePermissions::GLOBAL);
    Ok(match architecture {
        HardeningArchitecture::X86_64 | HardeningArchitecture::I386Pae => {
            let mut entry = physical_address | 1;
            entry |= (writable as u64) << 1;
            entry |= (user as u64) << 2;
            entry |= (global as u64) << 8;
            if policy.nx && !executable {
                entry |= 1u64 << 63;
            }
            entry
        }
        HardeningArchitecture::Aarch64 => {
            let mut entry = physical_address | 0b11 | (1 << 10);
            if !writable {
                entry |= 1 << 7;
            }
            if user {
                entry |= 1 << 6;
            }
            if policy.nx && !executable {
                entry |= (1u64 << 53) | (1u64 << 54);
            }
            entry
        }
        HardeningArchitecture::Armv7Lpae => {
            let mut entry = physical_address | 0b11 | (1 << 10);
            if !writable {
                entry |= 1 << 7;
            }
            if policy.nx && !executable {
                entry |= 1u64 << 54;
            }
            entry
        }
        HardeningArchitecture::Riscv32 | HardeningArchitecture::Riscv64 => {
            let mut entry = (physical_address >> 2) | 1;
            entry |= (permissions.contains(PagePermissions::READ) as u64) << 1;
            entry |= (writable as u64) << 2;
            entry |= ((!policy.nx || executable) as u64) << 3;
            entry |= (user as u64) << 4;
            entry |= (global as u64) << 5;
            entry |= 1 << 6;
            if writable {
                entry |= 1 << 7;
            }
            entry
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KaslrLayout {
    pub minimum: u64,
    pub maximum: u64,
    pub alignment: u64,
    pub image_size: u64,
    pub slide: u64,
    pub load_address: u64,
}

pub fn select_kaslr_layout(
    minimum: u64,
    maximum: u64,
    image_size: u64,
    alignment: u64,
    entropy: u64,
) -> Result<KaslrLayout, HardeningError> {
    if image_size == 0 || alignment == 0 || !alignment.is_power_of_two() || minimum > maximum {
        return Err(HardeningError::InvalidConfiguration(
            "KASLR requires a non-empty image, ordered range and power-of-two alignment",
        ));
    }
    let aligned_minimum = minimum
        .checked_add(alignment - 1)
        .ok_or(HardeningError::AddressOverflow)?
        & !(alignment - 1);
    let last = maximum
        .checked_sub(image_size)
        .ok_or(HardeningError::NoKaslrSlot)?
        & !(alignment - 1);
    if aligned_minimum > last {
        return Err(HardeningError::NoKaslrSlot);
    }
    let slots = ((last - aligned_minimum) / alignment) + 1;
    let mixed = splitmix64(entropy ^ minimum ^ maximum ^ image_size);
    let load_address = aligned_minimum + (mixed % slots) * alignment;
    Ok(KaslrLayout {
        minimum,
        maximum,
        alignment,
        image_size,
        slide: load_address - aligned_minimum,
        load_address,
    })
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelRelocationKind {
    Absolute32,
    Absolute64,
    Relative32,
    Relative64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelRelocation {
    pub offset: usize,
    pub kind: KernelRelocationKind,
    pub addend: i64,
}

pub fn apply_kernel_relocations(
    image: &mut [u8],
    link_address: u64,
    load_address: u64,
    relocations: &[KernelRelocation],
) -> Result<(), HardeningError> {
    let slide = load_address.wrapping_sub(link_address);
    for relocation in relocations {
        let width = match relocation.kind {
            KernelRelocationKind::Absolute32 | KernelRelocationKind::Relative32 => 4,
            KernelRelocationKind::Absolute64 | KernelRelocationKind::Relative64 => 8,
        };
        let end = relocation
            .offset
            .checked_add(width)
            .filter(|end| *end <= image.len())
            .ok_or(HardeningError::RelocationOutsideImage {
                offset: relocation.offset,
                width,
            })?;
        let value = match width {
            4 => u32::from_le_bytes(image[relocation.offset..end].try_into().expect("four")) as u64,
            8 => u64::from_le_bytes(image[relocation.offset..end].try_into().expect("eight")),
            _ => unreachable!(),
        };
        let adjusted = match relocation.kind {
            KernelRelocationKind::Absolute32 | KernelRelocationKind::Absolute64 => value
                .checked_add(slide)
                .and_then(|value| value.checked_add_signed(relocation.addend))
                .ok_or(HardeningError::RelocationOverflow)?,
            KernelRelocationKind::Relative32 | KernelRelocationKind::Relative64 => value
                .checked_add_signed(relocation.addend)
                .ok_or(HardeningError::RelocationOverflow)?,
        };
        if width == 4 {
            let adjusted =
                u32::try_from(adjusted).map_err(|_| HardeningError::RelocationOverflow)?;
            image[relocation.offset..end].copy_from_slice(&adjusted.to_le_bytes());
        } else {
            image[relocation.offset..end].copy_from_slice(&adjusted.to_le_bytes());
        }
    }
    Ok(())
}

pub fn derive_stack_canary(entropy: u64, stack_base: u64, cpu_id: u32) -> u64 {
    let value = splitmix64(entropy ^ stack_base.rotate_left(17) ^ u64::from(cpu_id));
    (value & !0xff) | 0
}

pub fn verify_stack_canary(expected: u64, observed: u64) -> Result<(), HardeningError> {
    if expected == observed {
        Ok(())
    } else {
        Err(HardeningError::CanaryMismatch { expected, observed })
    }
}

#[derive(Debug)]
pub struct KernelShadow<'a> {
    base: u64,
    granule: usize,
    shadow: &'a mut [u8],
}

impl<'a> KernelShadow<'a> {
    pub fn new(base: u64, granule: usize, shadow: &'a mut [u8]) -> Result<Self, HardeningError> {
        if granule == 0 || !granule.is_power_of_two() || shadow.is_empty() {
            return Err(HardeningError::InvalidConfiguration(
                "kernel shadow requires power-of-two granules and caller-owned storage",
            ));
        }
        Ok(Self {
            base,
            granule,
            shadow,
        })
    }

    fn range(
        &self,
        address: u64,
        len: usize,
    ) -> Result<std::ops::RangeInclusive<usize>, HardeningError> {
        if len == 0 {
            return Err(HardeningError::InvalidConfiguration(
                "sanitizer access length must be non-zero",
            ));
        }
        let start = address
            .checked_sub(self.base)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(HardeningError::ShadowOutsideRange {
                address,
                bytes: len,
            })?;
        let end = start
            .checked_add(len - 1)
            .ok_or(HardeningError::AddressOverflow)?;
        let first = start / self.granule;
        let last = end / self.granule;
        if last >= self.shadow.len() {
            return Err(HardeningError::ShadowOutsideRange {
                address,
                bytes: len,
            });
        }
        Ok(first..=last)
    }

    pub fn poison(&mut self, address: u64, len: usize, tag: u8) -> Result<(), HardeningError> {
        let range = self.range(address, len)?;
        for slot in range {
            self.shadow[slot] = tag.max(1);
        }
        Ok(())
    }

    pub fn unpoison(&mut self, address: u64, len: usize) -> Result<(), HardeningError> {
        let range = self.range(address, len)?;
        for slot in range {
            self.shadow[slot] = 0;
        }
        Ok(())
    }

    pub fn check(&self, address: u64, len: usize) -> Result<(), HardeningError> {
        if self.range(address, len)?.any(|slot| self.shadow[slot] != 0) {
            Err(HardeningError::PoisonedAccess {
                address,
                bytes: len,
            })
        } else {
            Ok(())
        }
    }

    pub fn shadow(&self) -> &[u8] {
        self.shadow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_is_the_only_implicit_policy_and_wx_is_opt_in() {
        assert_eq!(HardeningPolicy::default(), HardeningPolicy::RAW);
        let rwx = PagePermissions::READ
            .union(PagePermissions::WRITE)
            .union(PagePermissions::EXECUTE);
        assert!(validate_permission_transition(
            HardeningPolicy::RAW,
            PagePermissions::default(),
            rwx
        )
        .is_ok());
        let policy = HardeningPolicy {
            enforce_w_x: true,
            ..HardeningPolicy::RAW
        };
        assert_eq!(
            validate_permission_transition(policy, PagePermissions::default(), rwx),
            Err(HardeningError::WriteExecuteDenied)
        );
    }

    #[test]
    fn kaslr_relocations_nx_and_canary_are_material() {
        let layout = select_kaslr_layout(0x20_0000, 0x80_0000, 0x10_0000, 0x20_0000, 7).unwrap();
        assert_eq!(layout.load_address % 0x20_0000, 0);
        let policy = HardeningPolicy {
            nx: true,
            ..HardeningPolicy::RAW
        };
        let entry = encode_leaf_page_entry(
            HardeningArchitecture::X86_64,
            0x4000,
            PagePermissions::READ.union(PagePermissions::WRITE),
            policy,
        )
        .unwrap();
        assert_ne!(entry & (1u64 << 63), 0);
        let mut image = vec![0; 16];
        image[..8].copy_from_slice(&0x20_0100u64.to_le_bytes());
        apply_kernel_relocations(
            &mut image,
            0x20_0000,
            0x40_0000,
            &[KernelRelocation {
                offset: 0,
                kind: KernelRelocationKind::Absolute64,
                addend: 0,
            }],
        )
        .unwrap();
        assert_eq!(
            u64::from_le_bytes(image[..8].try_into().unwrap()),
            0x40_0100
        );
        let canary = derive_stack_canary(99, 0xffff_8000_0000_0000, 2);
        verify_stack_canary(canary, canary).unwrap();
        assert!(verify_stack_canary(canary, canary ^ 1).is_err());
    }

    #[test]
    fn sanitizer_uses_only_caller_owned_shadow_and_can_be_disabled() {
        let mut bytes = [0u8; 64];
        let mut shadow = KernelShadow::new(0x1000, 8, &mut bytes).unwrap();
        shadow.poison(0x1040, 16, 0xf1).unwrap();
        assert!(shadow.check(0x1048, 1).is_err());
        shadow.unpoison(0x1040, 16).unwrap();
        shadow.check(0x1048, 1).unwrap();
    }
}
