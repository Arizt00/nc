use crate::TypeContractKind;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BinaryRepr {
    #[default]
    Native,
    C,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BinaryEndian {
    #[default]
    Native,
    Little,
    Big,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryBitFieldContract {
    pub field: String,
    pub storage: String,
    pub bit_offset: u16,
    pub bit_width: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BinaryLayoutContract {
    pub explicit: bool,
    pub repr: BinaryRepr,
    pub packed: bool,
    pub align_bytes: Option<usize>,
    pub size_bytes: Option<usize>,
    pub endian: BinaryEndian,
    pub flexible_field: Option<String>,
    pub bitfields: Vec<BinaryBitFieldContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryLayoutFieldInput {
    pub name: String,
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub integer_bits: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryLayoutField {
    pub name: String,
    pub offset_bytes: usize,
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub bit_offset: Option<u16>,
    pub bit_width: Option<u16>,
    pub storage: Option<String>,
    pub flexible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputedBinaryLayout {
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub fields: Vec<BinaryLayoutField>,
    pub repr: BinaryRepr,
    pub packed: bool,
    pub endian: BinaryEndian,
}

fn compact_layout_item(item: &str) -> String {
    item.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn layout_call_args(item: &str, key: &str) -> Option<Vec<String>> {
    let compact = compact_layout_item(item);
    let lower = compact.to_ascii_lowercase();
    let prefix = format!("{}(", key.to_ascii_lowercase());
    if !lower.starts_with(&prefix) || !compact.ends_with(')') {
        return None;
    }
    let body = &compact[prefix.len()..compact.len() - 1];
    Some(
        body.split(',')
            .filter(|arg| !arg.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn layout_key_value<'a>(item: &'a str, key: &str) -> Option<&'a str> {
    let compact = compact_layout_item(item);
    let lower = compact.to_ascii_lowercase();
    let prefix = format!("{}:", key.to_ascii_lowercase());
    lower
        .starts_with(&prefix)
        .then(|| &item[item.find(':').unwrap_or(0) + 1..])
        .map(|value| value.trim().trim_end_matches(';').trim())
}

fn parse_layout_usize(raw: &str, diagnostic: &str) -> Result<usize, String> {
    raw.trim()
        .parse::<usize>()
        .map_err(|_| diagnostic.to_string())
}

pub fn parse_binary_layout_contract(items: &[String]) -> Result<BinaryLayoutContract, String> {
    let mut layout = BinaryLayoutContract::default();
    let mut seen = HashSet::new();

    for item in items {
        let compact = compact_layout_item(item);
        let lower = compact.to_ascii_lowercase();
        if lower == "binary" || lower == "kind:binary" || lower == "kind:stable" {
            layout.explicit = true;
            continue;
        }
        if lower == "unbinary" || lower == "kind:unbinary" || lower == "kind:runtime" {
            continue;
        }
        if let Some(kind) = layout_key_value(item, "kind") {
            return Err(format!(
                "E_TYPE_LAYOUT_001: layout kind must be binary, stable, unbinary or runtime, got '{}'",
                kind.trim()
            ));
        }
        if lower == "packed" {
            if !seen.insert("packed") {
                return Err("E_LAYOUT_PACKED_001: duplicate packed layout item".into());
            }
            layout.explicit = true;
            layout.packed = true;
            continue;
        }

        let repr = layout_call_args(item, "repr")
            .and_then(|args| (args.len() == 1).then(|| args[0].clone()))
            .or_else(|| layout_key_value(item, "repr").map(str::to_string));
        if let Some(repr) = repr {
            if !seen.insert("repr") {
                return Err("E_LAYOUT_REPR_001: duplicate representation".into());
            }
            layout.explicit = true;
            layout.repr = match repr.trim().to_ascii_lowercase().as_str() {
                "c" => BinaryRepr::C,
                "native" => BinaryRepr::Native,
                _ => return Err("E_LAYOUT_REPR_001: repr must be C or native".into()),
            };
            continue;
        }

        let align = layout_call_args(item, "align")
            .and_then(|args| (args.len() == 1).then(|| args[0].clone()))
            .or_else(|| layout_key_value(item, "align").map(str::to_string));
        if let Some(align) = align {
            if !seen.insert("align") {
                return Err("E_LAYOUT_ALIGN_001: duplicate aggregate alignment".into());
            }
            let value = parse_layout_usize(
                &align,
                "E_LAYOUT_ALIGN_001: alignment must be a non-zero power of two",
            )?;
            if value == 0 || !value.is_power_of_two() {
                return Err("E_LAYOUT_ALIGN_001: alignment must be a non-zero power of two".into());
            }
            layout.explicit = true;
            layout.align_bytes = Some(value);
            continue;
        }

        let size = layout_call_args(item, "size")
            .and_then(|args| (args.len() == 1).then(|| args[0].clone()))
            .or_else(|| layout_key_value(item, "size").map(str::to_string));
        if let Some(size) = size {
            if !seen.insert("size") {
                return Err("E_LAYOUT_SIZE_001: duplicate aggregate size".into());
            }
            let value =
                parse_layout_usize(&size, "E_LAYOUT_SIZE_001: size must be a positive integer")?;
            if value == 0 {
                return Err("E_LAYOUT_SIZE_001: size must be a positive integer".into());
            }
            layout.explicit = true;
            layout.size_bytes = Some(value);
            continue;
        }

        let endian = layout_call_args(item, "endian")
            .and_then(|args| (args.len() == 1).then(|| args[0].clone()))
            .or_else(|| layout_key_value(item, "endian").map(str::to_string));
        if let Some(endian) = endian {
            if !seen.insert("endian") {
                return Err("E_LAYOUT_ENDIAN_001: duplicate endian contract".into());
            }
            layout.explicit = true;
            layout.endian = match endian.trim().to_ascii_lowercase().as_str() {
                "native" => BinaryEndian::Native,
                "little" | "le" => BinaryEndian::Little,
                "big" | "be" => BinaryEndian::Big,
                _ => return Err("E_LAYOUT_ENDIAN_001: endian must be native, little or big".into()),
            };
            continue;
        }

        let flexible = layout_call_args(item, "flexible")
            .and_then(|args| (args.len() == 1).then(|| args[0].clone()))
            .or_else(|| layout_key_value(item, "flexible").map(str::to_string));
        if let Some(flexible) = flexible {
            if layout.flexible_field.is_some() {
                return Err(
                    "E_LAYOUT_FLEXIBLE_ARRAY_001: an aggregate may have only one flexible member"
                        .into(),
                );
            }
            layout.explicit = true;
            layout.flexible_field = Some(flexible.trim().to_string());
            continue;
        }

        if let Some(args) = layout_call_args(item, "bitfield") {
            if args.len() != 4 {
                return Err("E_LAYOUT_BITFIELD_001: bitfield(field, storage, offset, width) expects four arguments".into());
            }
            let bit_offset = args[2]
                .parse::<u16>()
                .map_err(|_| "E_LAYOUT_BITFIELD_001: bit offset must be an integer")?;
            let bit_width = args[3]
                .parse::<u16>()
                .map_err(|_| "E_LAYOUT_BITFIELD_001: bit width must be an integer")?;
            if bit_width == 0 {
                return Err("E_LAYOUT_BITFIELD_001: bit width must be non-zero".into());
            }
            layout.explicit = true;
            layout.bitfields.push(BinaryBitFieldContract {
                field: args[0].to_string(),
                storage: args[1].to_string(),
                bit_offset,
                bit_width,
            });
            continue;
        }

        return Err(format!(
            "E_LAYOUT_ITEM_001: unknown binary layout item '{}'",
            item.trim()
        ));
    }
    Ok(layout)
}

fn align_up(value: usize, align: usize) -> usize {
    let align = align.max(1);
    value.div_ceil(align) * align
}

pub fn compute_binary_layout(
    kind: TypeContractKind,
    contract: &BinaryLayoutContract,
    fields: &[BinaryLayoutFieldInput],
) -> Result<ComputedBinaryLayout, String> {
    if fields.is_empty() && matches!(kind, TypeContractKind::Struct | TypeContractKind::Union) {
        return Err("E_TYPE_FIELD_001: binary aggregates require at least one field".into());
    }
    if matches!(kind, TypeContractKind::Union) && contract.flexible_field.is_some() {
        return Err(
            "E_LAYOUT_FLEXIBLE_ARRAY_001: a union cannot contain a flexible array member".into(),
        );
    }
    if let Some(flexible) = &contract.flexible_field {
        if fields.last().map(|field| &field.name) != Some(flexible) {
            return Err(
                "E_LAYOUT_FLEXIBLE_ARRAY_001: a flexible array member must be the final field"
                    .into(),
            );
        }
    }

    let by_name = fields
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect::<HashMap<_, _>>();
    let mut bitfield_by_name = HashMap::new();
    let mut occupied_bits: HashMap<String, HashSet<u16>> = HashMap::new();
    for bitfield in &contract.bitfields {
        let Some(field) = by_name.get(bitfield.field.as_str()) else {
            return Err(format!(
                "E_LAYOUT_BITFIELD_001: unknown bitfield member '{}'",
                bitfield.field
            ));
        };
        let Some(storage) = by_name.get(bitfield.storage.as_str()) else {
            return Err(format!(
                "E_LAYOUT_BITFIELD_001: unknown bitfield storage '{}'",
                bitfield.storage
            ));
        };
        let storage_bits = storage.integer_bits.ok_or_else(|| {
            format!(
                "E_LAYOUT_BITFIELD_001: storage '{}' must use an integer type",
                storage.name
            )
        })?;
        if field.size_bytes != storage.size_bytes
            || bitfield.bit_offset.saturating_add(bitfield.bit_width) > storage_bits
        {
            return Err(format!(
                "E_LAYOUT_BITFIELD_001: '{}' must match and fit {}-bit storage '{}'",
                bitfield.field, storage_bits, bitfield.storage
            ));
        }
        let occupied = occupied_bits.entry(bitfield.storage.clone()).or_default();
        for bit in bitfield.bit_offset..bitfield.bit_offset + bitfield.bit_width {
            if !occupied.insert(bit) {
                return Err(format!(
                    "E_LAYOUT_BITFIELD_001: bit {} overlaps in storage '{}'",
                    bit, bitfield.storage
                ));
            }
        }
        if bitfield_by_name
            .insert(bitfield.field.as_str(), bitfield)
            .is_some()
        {
            return Err(format!(
                "E_LAYOUT_BITFIELD_001: duplicate bitfield member '{}'",
                bitfield.field
            ));
        }
    }

    let field_alignment = |field: &BinaryLayoutFieldInput| {
        if contract.packed {
            1
        } else {
            field.align_bytes.max(1)
        }
    };
    let mut lowered = Vec::with_capacity(fields.len());
    let mut storage_offsets: HashMap<String, usize> = HashMap::new();
    let mut offset = 0usize;
    let mut max_align = 1usize;
    let mut union_size = 0usize;

    for field in fields {
        let flexible = contract.flexible_field.as_deref() == Some(field.name.as_str());
        let bitfield = bitfield_by_name.get(field.name.as_str()).copied();
        let storage_name = bitfield.map(|bitfield| bitfield.storage.as_str());
        let storage_field = storage_name.and_then(|name| by_name.get(name)).copied();
        let effective = storage_field.unwrap_or(field);
        let align = field_alignment(effective);
        max_align = max_align.max(align);

        let field_offset = if matches!(kind, TypeContractKind::Union) {
            0
        } else if let Some(storage) = storage_name {
            if let Some(existing) = storage_offsets.get(storage) {
                *existing
            } else {
                offset = align_up(offset, align);
                let current = offset;
                storage_offsets.insert(storage.to_string(), current);
                if !flexible {
                    offset = offset.saturating_add(effective.size_bytes);
                }
                current
            }
        } else {
            offset = align_up(offset, align);
            let current = offset;
            storage_offsets.entry(field.name.clone()).or_insert(current);
            if !flexible {
                offset = offset.saturating_add(field.size_bytes);
            }
            current
        };
        union_size = union_size.max(if flexible { 0 } else { effective.size_bytes });
        lowered.push(BinaryLayoutField {
            name: field.name.clone(),
            offset_bytes: field_offset,
            size_bytes: if flexible { 0 } else { field.size_bytes },
            align_bytes: align,
            bit_offset: bitfield.map(|value| value.bit_offset),
            bit_width: bitfield.map(|value| value.bit_width),
            storage: bitfield.map(|value| value.storage.clone()),
            flexible,
        });
    }

    let aggregate_align = contract.align_bytes.unwrap_or(max_align).max(max_align);
    if !aggregate_align.is_power_of_two() {
        return Err("E_LAYOUT_ALIGN_001: aggregate alignment must be a power of two".into());
    }
    let natural_size = align_up(
        if matches!(kind, TypeContractKind::Union) {
            union_size
        } else {
            offset
        },
        aggregate_align,
    );
    let size_bytes = if let Some(requested) = contract.size_bytes {
        if requested < natural_size || requested % aggregate_align != 0 {
            return Err(format!(
                "E_LAYOUT_SIZE_001: explicit size {} is smaller than/alignment-incompatible with natural size {} align {}",
                requested, natural_size, aggregate_align
            ));
        }
        requested
    } else {
        natural_size
    };

    Ok(ComputedBinaryLayout {
        size_bytes,
        align_bytes: aggregate_align,
        fields: lowered,
        repr: contract.repr,
        packed: contract.packed,
        endian: contract.endian,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_c_packed_endian_bitfield_and_flexible_contract() {
        let layout = parse_binary_layout_contract(&[
            "repr ( C )".into(),
            "packed".into(),
            "align ( 8 )".into(),
            "endian ( little )".into(),
            "bitfield ( mode , flags , 1 , 3 )".into(),
            "flexible ( tail )".into(),
        ])
        .expect("layout must parse");
        assert_eq!(layout.repr, BinaryRepr::C);
        assert!(layout.packed);
        assert_eq!(layout.align_bytes, Some(8));
        assert_eq!(layout.endian, BinaryEndian::Little);
        assert_eq!(layout.bitfields.len(), 1);
        assert_eq!(layout.flexible_field.as_deref(), Some("tail"));
    }

    #[test]
    fn distinguishes_invalid_kind_from_unknown_layout_item() {
        let invalid_kind = parse_binary_layout_contract(&["kind: impossible".into()])
            .expect_err("known kind key with an invalid value must fail as a type layout");
        assert!(invalid_kind.contains("E_TYPE_LAYOUT_001"), "{invalid_kind}");

        let unknown = parse_binary_layout_contract(&["mystery: value".into()])
            .expect_err("unknown layout key must retain the catalog diagnostic");
        assert!(unknown.contains("E_LAYOUT_ITEM_001"), "{unknown}");
    }

    #[test]
    fn accepts_parser_and_programmatic_layout_items_with_statement_terminators() {
        let layout = parse_binary_layout_contract(&[
            "kind: binary;".into(),
            "repr(C);".into(),
            "align: 8;".into(),
            "endian(little);".into(),
        ])
        .expect("layout items retain their meaning when terminated as statements");

        assert!(layout.explicit);
        assert_eq!(layout.repr, BinaryRepr::C);
        assert_eq!(layout.align_bytes, Some(8));
        assert_eq!(layout.endian, BinaryEndian::Little);
    }

    #[test]
    fn computes_union_and_shared_bitfield_storage() {
        let fields = vec![
            BinaryLayoutFieldInput {
                name: "flags".into(),
                size_bytes: 4,
                align_bytes: 4,
                integer_bits: Some(32),
            },
            BinaryLayoutFieldInput {
                name: "mode".into(),
                size_bytes: 4,
                align_bytes: 4,
                integer_bits: Some(32),
            },
        ];
        let contract = BinaryLayoutContract {
            explicit: true,
            bitfields: vec![BinaryBitFieldContract {
                field: "mode".into(),
                storage: "flags".into(),
                bit_offset: 4,
                bit_width: 3,
            }],
            ..BinaryLayoutContract::default()
        };
        let layout = compute_binary_layout(TypeContractKind::Struct, &contract, &fields)
            .expect("bitfield layout must compute");
        assert_eq!(layout.size_bytes, 4);
        assert_eq!(layout.fields[0].offset_bytes, layout.fields[1].offset_bytes);

        let union = compute_binary_layout(TypeContractKind::Union, &contract, &fields)
            .expect("union layout must compute");
        assert!(union.fields.iter().all(|field| field.offset_bytes == 0));
    }
}
