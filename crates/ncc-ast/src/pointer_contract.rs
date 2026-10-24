use crate::TypeName;
use serde::{Deserialize, Serialize};

pub const POINTER_ADDRESS_SPACE_COUNT: u32 = 6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PointerAddressSpace {
    Cpu = 0,
    Mmio = 1,
    Physical = 2,
    Dma = 3,
    Gpu = 4,
    Firmware = 5,
}

impl PointerAddressSpace {
    pub const ALL: [Self; POINTER_ADDRESS_SPACE_COUNT as usize] = [
        Self::Cpu,
        Self::Mmio,
        Self::Physical,
        Self::Dma,
        Self::Gpu,
        Self::Firmware,
    ];

    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Mmio => "mmio",
            Self::Physical => "physical",
            Self::Dma => "dma",
            Self::Gpu => "gpu",
            Self::Firmware => "firmware",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "cpu" | "default" | "generic" => Some(Self::Cpu),
            "mmio" | "io" => Some(Self::Mmio),
            "physical" | "phys" => Some(Self::Physical),
            "dma" | "iommu" | "iova" => Some(Self::Dma),
            "gpu" | "device" => Some(Self::Gpu),
            "firmware" | "fw" => Some(Self::Firmware),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PointerWidth {
    Target,
    Bits16,
    Bits32,
    Bits64,
    Bits128,
}

impl PointerWidth {
    pub const EXPLICIT: [Self; 4] = [Self::Bits16, Self::Bits32, Self::Bits64, Self::Bits128];

    pub const fn bits(self, target_bits: u16) -> u16 {
        match self {
            Self::Target => target_bits,
            Self::Bits16 => 16,
            Self::Bits32 => 32,
            Self::Bits64 => 64,
            Self::Bits128 => 128,
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name
            .trim()
            .to_ascii_lowercase()
            .trim_start_matches("bits")
            .trim_start_matches('w')
        {
            "16" => Some(Self::Bits16),
            "32" => Some(Self::Bits32),
            "64" => Some(Self::Bits64),
            "128" => Some(Self::Bits128),
            "target" | "native" => Some(Self::Target),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PointerPointeeKind {
    Data,
    Function,
    Member,
    FixedArray,
    FlexibleArray,
    Aggregate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PointerTypeContract {
    pub pointee: String,
    pub address_space: PointerAddressSpace,
    pub width: PointerWidth,
    pub pointee_kind: PointerPointeeKind,
    pub capability: bool,
}

impl PointerTypeContract {
    pub fn resolved_width_bits(&self, target_bits: u16) -> u16 {
        self.width.bits(target_bits)
    }

    pub fn llvm_address_space(&self, target_bits: u16) -> u32 {
        let semantic = self.address_space as u32;
        let width = self.resolved_width_bits(target_bits);
        if width == target_bits {
            semantic
        } else {
            u32::from(width) + semantic
        }
    }

    pub fn llvm_pointer_type(&self, target_bits: u16) -> String {
        match self.llvm_address_space(target_bits) {
            0 => "ptr".to_string(),
            address_space => format!("ptr addrspace({address_space})"),
        }
    }

    pub fn pointer_integer_type(&self, target_bits: u16) -> String {
        format!("i{}", self.resolved_width_bits(target_bits))
    }

    pub fn is_native_width(&self, target_bits: u16) -> bool {
        self.resolved_width_bits(target_bits) == target_bits
    }
}

pub fn pointer_type_contract(ty: &TypeName) -> Result<Option<PointerTypeContract>, String> {
    match ty {
        TypeName::Generic { base, args } => {
            let base_lower = base.trim().to_ascii_lowercase();
            if base_lower == "fn" {
                return Ok(Some(PointerTypeContract {
                    pointee: render_type_name(ty),
                    address_space: PointerAddressSpace::Cpu,
                    width: PointerWidth::Target,
                    pointee_kind: PointerPointeeKind::Function,
                    capability: false,
                }));
            }
            if matches!(base_lower.as_str(), "member" | "pm") {
                if args.len() != 2 {
                    return Err(
                        "E_PTR_MEMBER_ARITY_001: member<Owner, Field> requires two types"
                            .to_string(),
                    );
                }
                return Ok(Some(PointerTypeContract {
                    pointee: render_type_name(&args[1]),
                    address_space: PointerAddressSpace::Cpu,
                    width: PointerWidth::Target,
                    pointee_kind: PointerPointeeKind::Member,
                    capability: false,
                }));
            }
            if !matches!(
                base_lower.as_str(),
                "ptr" | "ptr mut" | "ptrmut" | "ref" | "ref mut" | "refmut" | "site" | "cap"
            ) {
                return Ok(None);
            }
            if args.is_empty() || args.len() > 3 {
                return Err(
                    "E_PTR_TYPE_ARITY_001: p<T[, address-space][, 16|32|64|128]> expects one to three arguments"
                        .to_string(),
                );
            }

            let mut address_space = PointerAddressSpace::Cpu;
            let mut width = if base_lower == "cap" {
                PointerWidth::Bits128
            } else {
                PointerWidth::Target
            };
            let mut saw_space = false;
            let mut saw_width = base_lower == "cap";
            for modifier in &args[1..] {
                let TypeName::Simple(modifier) = modifier else {
                    return Err(
                        "E_PTR_MODIFIER_001: pointer address-space and width modifiers must be simple names"
                            .to_string(),
                    );
                };
                if let Some(parsed) = PointerAddressSpace::parse(modifier) {
                    if saw_space {
                        return Err(
                            "E_PTR_ADDRESS_SPACE_DUP_001: pointer address space declared more than once"
                                .to_string(),
                        );
                    }
                    address_space = parsed;
                    saw_space = true;
                    continue;
                }
                if let Some(parsed) = PointerWidth::parse(modifier) {
                    if saw_width {
                        return Err("E_PTR_WIDTH_DUP_001: pointer width declared more than once"
                            .to_string());
                    }
                    width = parsed;
                    saw_width = true;
                    continue;
                }
                return Err(format!(
                    "E_PTR_MODIFIER_001: unknown pointer modifier '{modifier}'"
                ));
            }
            let resolved_128 = width == PointerWidth::Bits128;
            Ok(Some(PointerTypeContract {
                pointee: render_type_name(&args[0]),
                address_space,
                width,
                pointee_kind: pointee_kind(&args[0]),
                capability: base_lower == "cap" || resolved_128,
            }))
        }
        TypeName::Simple(name) if matches!(name.as_str(), "addr" | "ptr" | "skk*" | "skk**") => {
            Ok(Some(PointerTypeContract {
                pointee: "byte".to_string(),
                address_space: PointerAddressSpace::Cpu,
                width: PointerWidth::Target,
                pointee_kind: PointerPointeeKind::Data,
                capability: false,
            }))
        }
        _ => Ok(None),
    }
}

pub fn pointer_type_contract_from_surface(
    surface: &str,
) -> Result<Option<PointerTypeContract>, String> {
    let ty = parse_surface_type(surface.trim())?;
    pointer_type_contract(&ty)
}

fn pointee_kind(ty: &TypeName) -> PointerPointeeKind {
    match ty {
        TypeName::Array(_) => PointerPointeeKind::FlexibleArray,
        TypeName::Generic { base, .. } if base.eq_ignore_ascii_case("array") => {
            PointerPointeeKind::FixedArray
        }
        TypeName::Generic { base, .. } if base.eq_ignore_ascii_case("fn") => {
            PointerPointeeKind::Function
        }
        TypeName::Simple(name) if scalar_pointee_name(name) => PointerPointeeKind::Data,
        TypeName::Simple(_)
        | TypeName::Generic { .. }
        | TypeName::Vector(_)
        | TypeName::Union(_) => PointerPointeeKind::Aggregate,
    }
}

fn scalar_pointee_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "bool"
            | "byte"
            | "char"
            | "i8"
            | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "int"
            | "uint"
            | "word"
            | "dword"
            | "qword"
            | "ft32"
            | "ft64"
            | "void"
    )
}

fn render_type_name(ty: &TypeName) -> String {
    match ty {
        TypeName::Simple(name) => name.clone(),
        TypeName::Vector(inner) => format!("vector<{}>", render_type_name(inner)),
        TypeName::Array(inner) => format!("{}[]", render_type_name(inner)),
        TypeName::Union(items) => items
            .iter()
            .map(render_type_name)
            .collect::<Vec<_>>()
            .join("|"),
        TypeName::Generic { base, args } => format!(
            "{}<{}>",
            base,
            args.iter()
                .map(render_type_name)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn parse_surface_type(surface: &str) -> Result<TypeName, String> {
    let surface = surface.trim();
    if let Some(inner) = surface.strip_suffix("[]") {
        return Ok(TypeName::Array(Box::new(parse_surface_type(inner)?)));
    }
    let Some(open) = surface.find('<') else {
        return Ok(TypeName::Simple(surface.to_string()));
    };
    if !surface.ends_with('>') {
        return Err("E_PTR_TYPE_PARSE_001: unterminated pointer type".to_string());
    }
    let base = surface[..open].trim();
    let base = if base.eq_ignore_ascii_case("p") {
        "ptr"
    } else {
        base
    };
    let body = &surface[open + 1..surface.len() - 1];
    let args = split_top_level_args(body)
        .into_iter()
        .map(parse_surface_type)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeName::Generic {
        base: base.to_string(),
        args,
    })
}

fn split_top_level_args(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in body.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(body[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(body[start..].trim());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_raw_address_spaces_widths_and_pointee_classes() {
        let mmio = pointer_type_contract_from_surface("p<u32, mmio>")
            .unwrap()
            .unwrap();
        assert_eq!(mmio.address_space, PointerAddressSpace::Mmio);
        assert_eq!(mmio.resolved_width_bits(64), 64);
        assert_eq!(mmio.llvm_pointer_type(64), "ptr addrspace(1)");

        let dma32 = pointer_type_contract_from_surface("p<Packet, dma, 32>")
            .unwrap()
            .unwrap();
        assert_eq!(dma32.address_space, PointerAddressSpace::Dma);
        assert_eq!(dma32.resolved_width_bits(64), 32);
        assert_eq!(dma32.llvm_pointer_type(64), "ptr addrspace(35)");
        assert_eq!(dma32.pointee_kind, PointerPointeeKind::Aggregate);

        let capability = pointer_type_contract_from_surface("p<byte, gpu, 128>")
            .unwrap()
            .unwrap();
        assert!(capability.capability);
        assert_eq!(capability.pointer_integer_type(64), "i128");

        let flexible = pointer_type_contract_from_surface("p<byte[]>")
            .unwrap()
            .unwrap();
        assert_eq!(flexible.pointee_kind, PointerPointeeKind::FlexibleArray);

        let fixed = pointer_type_contract_from_surface("p<array<Packet,32>>")
            .unwrap()
            .unwrap();
        assert_eq!(fixed.pointee_kind, PointerPointeeKind::FixedArray);

        let function = pointer_type_contract_from_surface("fn<int,bool>")
            .unwrap()
            .unwrap();
        assert_eq!(function.pointee_kind, PointerPointeeKind::Function);

        let member = pointer_type_contract_from_surface("member<Packet,u32>")
            .unwrap()
            .unwrap();
        assert_eq!(member.pointee, "u32");
        assert_eq!(member.pointee_kind, PointerPointeeKind::Member);
    }

    #[test]
    fn rejects_unknown_or_duplicate_modifiers() {
        assert!(pointer_type_contract_from_surface("p<int, mystery>").is_err());
        assert!(pointer_type_contract_from_surface("p<int, dma, mmio>").is_err());
        assert!(pointer_type_contract_from_surface("p<int, 32, 64>").is_err());
    }
}
