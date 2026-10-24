use serde::{Deserialize, Serialize};

use crate::TypeName;

pub const STORAGE_TYPE_MARKER: &str = "__ncc_storage";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageDuration {
    Static,
    ThreadLocal,
    PerCpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageSection {
    Automatic,
    Data,
    Bss,
    ReadOnlyData,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageLinkage {
    Default,
    Internal,
    External,
    Weak,
    Common,
    LinkOnceOdr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageContract {
    pub duration: StorageDuration,
    pub section: StorageSection,
    pub custom_section: Option<String>,
    pub align_bytes: Option<usize>,
    pub linkage: StorageLinkage,
    pub comdat: Option<String>,
}

impl Default for StorageContract {
    fn default() -> Self {
        Self {
            duration: StorageDuration::Static,
            section: StorageSection::Automatic,
            custom_section: None,
            align_bytes: None,
            linkage: StorageLinkage::Default,
            comdat: None,
        }
    }
}

impl StorageContract {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn validate(&self, container_local: bool, initialized: bool) -> Result<(), String> {
        if let Some(align) = self.align_bytes {
            if align == 0 || !align.is_power_of_two() {
                return Err(format!(
                    "E_STORAGE_ALIGN_001: storage alignment must be a non-zero power of two, got {align}"
                ));
            }
        }
        if let Some(section) = &self.custom_section {
            if section.is_empty() || section.as_bytes().contains(&0) {
                return Err(
                    "E_STORAGE_SECTION_001: custom section name must be non-empty and contain no NUL byte"
                        .to_string(),
                );
            }
        }
        if self.section == StorageSection::Bss && initialized {
            return Err(
                "E_STORAGE_BSS_INIT_001: bss storage is zero-fill and cannot have a source initializer"
                    .to_string(),
            );
        }
        if self.linkage == StorageLinkage::Common && initialized {
            return Err(
                "E_STORAGE_COMMON_INIT_001: common storage is tentative zero-fill and cannot have a source initializer"
                    .to_string(),
            );
        }
        if self.linkage == StorageLinkage::Common
            && (self.duration != StorageDuration::Static
                || self.section == StorageSection::ReadOnlyData
                || self.custom_section.is_some()
                || self.comdat.is_some())
        {
            return Err(
                "E_STORAGE_COMMON_CONTRACT_001: common storage must be ordinary static zero-fill storage without TLS, per-CPU, rodata, custom-section or COMDAT qualifiers"
                    .to_string(),
            );
        }
        if container_local
            && matches!(
                self.linkage,
                StorageLinkage::External
                    | StorageLinkage::Weak
                    | StorageLinkage::Common
                    | StorageLinkage::LinkOnceOdr
            )
        {
            return Err(
                "E_STORAGE_LOCAL_LINKAGE_001: local storage cannot request external, weak, common or COMDAT linkage"
                    .to_string(),
            );
        }
        if self.comdat.as_deref().is_some_and(str::is_empty) {
            return Err("E_STORAGE_COMDAT_001: COMDAT group name cannot be empty".to_string());
        }
        if self.comdat.is_some() && self.linkage != StorageLinkage::LinkOnceOdr {
            return Err(
                "E_STORAGE_COMDAT_LINKAGE_001: COMDAT storage must use linkonce_odr linkage"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn duration_name(&self) -> &'static str {
        match self.duration {
            StorageDuration::Static => "static",
            StorageDuration::ThreadLocal => "tls",
            StorageDuration::PerCpu => "percpu",
        }
    }

    fn section_name(&self) -> &'static str {
        match self.section {
            StorageSection::Automatic => "auto",
            StorageSection::Data => "data",
            StorageSection::Bss => "bss",
            StorageSection::ReadOnlyData => "rodata",
        }
    }

    fn linkage_name(&self) -> &'static str {
        match self.linkage {
            StorageLinkage::Default => "default",
            StorageLinkage::Internal => "internal",
            StorageLinkage::External => "external",
            StorageLinkage::Weak => "weak",
            StorageLinkage::Common => "common",
            StorageLinkage::LinkOnceOdr => "linkonce_odr",
        }
    }
}

pub fn attach_storage_contract(ty: TypeName, contract: StorageContract) -> TypeName {
    if contract.is_default() {
        return ty;
    }
    attach_explicit_storage_contract(ty, contract)
}

pub fn attach_explicit_storage_contract(ty: TypeName, contract: StorageContract) -> TypeName {
    TypeName::Generic {
        base: STORAGE_TYPE_MARKER.to_string(),
        args: vec![
            TypeName::Simple(contract.duration_name().to_string()),
            TypeName::Simple(contract.section_name().to_string()),
            TypeName::Simple(contract.custom_section.clone().unwrap_or_default()),
            TypeName::Simple(
                contract
                    .align_bytes
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            TypeName::Simple(contract.linkage_name().to_string()),
            TypeName::Simple(contract.comdat.clone().unwrap_or_default()),
            ty,
        ],
    }
}

pub fn split_storage_contract(ty: &TypeName) -> Option<(StorageContract, &TypeName)> {
    let TypeName::Generic { base, args } = ty else {
        return None;
    };
    if base != STORAGE_TYPE_MARKER || args.len() != 7 {
        return None;
    }
    let simple = |index: usize| match &args[index] {
        TypeName::Simple(value) => Some(value.as_str()),
        _ => None,
    };
    let duration = match simple(0)? {
        "static" => StorageDuration::Static,
        "tls" => StorageDuration::ThreadLocal,
        "percpu" => StorageDuration::PerCpu,
        _ => return None,
    };
    let section = match simple(1)? {
        "auto" => StorageSection::Automatic,
        "data" => StorageSection::Data,
        "bss" => StorageSection::Bss,
        "rodata" => StorageSection::ReadOnlyData,
        _ => return None,
    };
    let custom_section = match simple(2)? {
        "" => None,
        value => Some(value.to_string()),
    };
    let align_bytes = match simple(3)? {
        "" => None,
        value => Some(value.parse().ok()?),
    };
    let linkage = match simple(4)? {
        "default" => StorageLinkage::Default,
        "internal" => StorageLinkage::Internal,
        "external" => StorageLinkage::External,
        "weak" => StorageLinkage::Weak,
        "common" => StorageLinkage::Common,
        "linkonce_odr" => StorageLinkage::LinkOnceOdr,
        _ => return None,
    };
    let comdat = match simple(5)? {
        "" => None,
        value => Some(value.to_string()),
    };
    Some((
        StorageContract {
            duration,
            section,
            custom_section,
            align_bytes,
            linkage,
            comdat,
        },
        &args[6],
    ))
}

pub fn storage_value_type(ty: &TypeName) -> &TypeName {
    split_storage_contract(ty)
        .map(|(_, value_type)| value_type)
        .unwrap_or(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_contract_roundtrips_without_changing_value_type() {
        let contract = StorageContract {
            duration: StorageDuration::ThreadLocal,
            section: StorageSection::Data,
            custom_section: Some(".tdata.fast".to_string()),
            align_bytes: Some(64),
            linkage: StorageLinkage::LinkOnceOdr,
            comdat: Some("fast_errno".to_string()),
        };
        let ty = attach_storage_contract(TypeName::Simple("int".to_string()), contract.clone());
        let (decoded, value_type) = split_storage_contract(&ty).expect("storage metadata");
        assert_eq!(decoded, contract);
        assert_eq!(value_type, &TypeName::Simple("int".to_string()));
    }

    #[test]
    fn storage_contract_rejects_non_power_of_two_alignment() {
        let contract = StorageContract {
            align_bytes: Some(24),
            ..StorageContract::default()
        };
        assert!(contract.validate(false, false).is_err());
    }
}
