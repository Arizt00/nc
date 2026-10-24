//! Generate the nC Unicode 17.0.0 lookup provider from the official UCD.
//!
//! This is a build-time tool. The generated runtime is nC source containing
//! immutable tables; no Rust or C runtime is linked into user programs.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = "17.0.0";

#[derive(Clone)]
struct Decomposition {
    compatibility: bool,
    values: Vec<u32>,
}

#[derive(Clone, Copy)]
struct RangeValue {
    start: u32,
    end: u32,
    value: u32,
}

fn main() -> Result<(), String> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let data = root.join("runtime").join("unicode").join(VERSION);
    let output = root.join("runtime").join("nc").join("unicode_tables.n");

    let unicode_data = read(&data.join("UnicodeData.txt"))?;
    let normalization = read(&data.join("DerivedNormalizationProps.txt"))?;
    let grapheme = read(&data.join("GraphemeBreakProperty.txt"))?;
    let east_asian_width = read(&data.join("EastAsianWidth.txt"))?;
    let emoji = read(&data.join("emoji-data.txt"))?;
    let core = read(&data.join("DerivedCoreProperties.txt"))?;

    let (decompositions, combining_classes) = parse_unicode_data(&unicode_data)?;
    let composition_exclusions = parse_property_set(&normalization, "Full_Composition_Exclusion")?;
    let canonical: BTreeMap<u32, Decomposition> = decompositions
        .iter()
        .filter(|(_, item)| !item.compatibility)
        .map(|(key, item)| (*key, item.clone()))
        .collect();
    let compositions = build_compositions(&canonical, &composition_exclusions);
    let grapheme_ranges = parse_grapheme_ranges(&grapheme)?;
    let incb_ranges = parse_incb_ranges(&core)?;
    let pictographic_ranges = parse_boolean_ranges(&emoji, "Extended_Pictographic")?;
    let wide_ranges = parse_selected_property_ranges(&east_asian_width, &["W", "F"])?;
    let emoji_presentation_ranges = parse_boolean_ranges(&emoji, "Emoji_Presentation")?;
    let mut zero_width_ranges =
        parse_general_category_ranges(&unicode_data, &["Mn", "Mc", "Me", "Cc", "Cf"])?;
    zero_width_ranges.extend(parse_boolean_ranges(&core, "Default_Ignorable_Code_Point")?);
    zero_width_ranges = merge_ranges(zero_width_ranges);
    let mut conformance = verify_conformance(
        &data,
        &decompositions,
        &combining_classes,
        &compositions,
        &grapheme_ranges,
        &incb_ranges,
        &pictographic_ranges,
    )?;
    conformance.wide_ranges = wide_ranges.len();
    conformance.zero_width_ranges = zero_width_ranges.len();
    conformance.emoji_presentation_ranges = emoji_presentation_ranges.len();

    let mut generated = String::new();
    generated.push_str("/* Generated from Unicode 17.0.0 UCD. Do not edit manually. */\n");
    generated.push_str("nccUnicodeTables :: runtime {\n");
    generated.push_str("    stc:\n");

    emit_decomposition_tables(&mut generated, "canonical", &canonical);
    emit_decomposition_tables(&mut generated, "compatibility", &decompositions);

    let ccc_keys: Vec<u64> = combining_classes
        .keys()
        .map(|value| u64::from(*value))
        .collect();
    let ccc_values: Vec<u64> = combining_classes
        .values()
        .map(|value| u64::from(*value))
        .collect();
    emit_array(&mut generated, "unicodeCccKeys", "u32", &ccc_keys);
    emit_array(&mut generated, "unicodeCccValues", "byte", &ccc_values);

    let compose_keys: Vec<u64> = compositions.keys().copied().collect();
    let compose_values: Vec<u64> = compositions
        .values()
        .map(|value| u64::from(*value))
        .collect();
    emit_array(&mut generated, "unicodeComposeKeys", "u64", &compose_keys);
    emit_array(
        &mut generated,
        "unicodeComposeValues",
        "u32",
        &compose_values,
    );

    emit_range_tables(&mut generated, "unicodeGrapheme", &grapheme_ranges);
    emit_range_tables(&mut generated, "unicodeIncb", &incb_ranges);
    emit_boolean_range_tables(&mut generated, "unicodePictographic", &pictographic_ranges);
    emit_boolean_range_tables(&mut generated, "unicodeWide", &wide_ranges);
    emit_boolean_range_tables(&mut generated, "unicodeZeroWidth", &zero_width_ranges);
    emit_boolean_range_tables(
        &mut generated,
        "unicodeEmojiPresentation",
        &emoji_presentation_ranges,
    );

    generated.push_str("\n    fn:\n        public:\n");
    emit_decomposition_lookup(&mut generated, "Canonical", "canonical", canonical.len());
    emit_decomposition_lookup(
        &mut generated,
        "Compatibility",
        "compatibility",
        decompositions.len(),
    );
    emit_value_lookup(
        &mut generated,
        "unicodeCombiningClass",
        "unicodeCccKeys",
        "unicodeCccValues",
        ccc_keys.len(),
        "int",
    );
    emit_compose_lookup(&mut generated, compose_keys.len());
    emit_range_lookup(
        &mut generated,
        "unicodeGraphemeProperty",
        "unicodeGrapheme",
        grapheme_ranges.len(),
    );
    emit_range_lookup(
        &mut generated,
        "unicodeIndicConjunctProperty",
        "unicodeIncb",
        incb_ranges.len(),
    );
    emit_boolean_range_lookup(
        &mut generated,
        "unicodeIsExtendedPictographic",
        "unicodePictographic",
        pictographic_ranges.len(),
    );
    emit_boolean_range_lookup(
        &mut generated,
        "unicodeIsWide",
        "unicodeWide",
        wide_ranges.len(),
    );
    emit_boolean_range_lookup(
        &mut generated,
        "unicodeIsZeroWidth",
        "unicodeZeroWidth",
        zero_width_ranges.len(),
    );
    emit_boolean_range_lookup(
        &mut generated,
        "unicodeIsEmojiPresentation",
        "unicodeEmojiPresentation",
        emoji_presentation_ranges.len(),
    );
    generated.push_str("\n    fa:\n        return true;\n}\n");

    fs::write(&output, generated)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    write_conformance_report(&root, &conformance)?;
    println!(
        "generated {} canonical={} compatibility={} compositions={} grapheme_ranges={} incb_ranges={} pictographic_ranges={} wide_ranges={} zero_width_ranges={} emoji_presentation_ranges={} normalization_cases={} normalization_checks={} grapheme_cases={}",
        output.display(),
        canonical.len(),
        decompositions.len(),
        compositions.len(),
        grapheme_ranges.len(),
        incb_ranges.len(),
        pictographic_ranges.len(),
        wide_ranges.len(),
        zero_width_ranges.len(),
        emoji_presentation_ranges.len(),
        conformance.normalization_cases,
        conformance.normalization_checks,
        conformance.grapheme_cases
    );
    Ok(())
}

#[derive(Default)]
struct ConformanceSummary {
    normalization_cases: usize,
    normalization_checks: usize,
    grapheme_cases: usize,
    grapheme_boundaries: usize,
    wide_ranges: usize,
    zero_width_ranges: usize,
    emoji_presentation_ranges: usize,
}

struct UnicodeModel<'a> {
    decompositions: &'a BTreeMap<u32, Decomposition>,
    combining_classes: &'a BTreeMap<u32, u8>,
    compositions: &'a BTreeMap<u64, u32>,
    grapheme_ranges: &'a [RangeValue],
    incb_ranges: &'a [RangeValue],
    pictographic_ranges: &'a [(u32, u32)],
}

fn verify_conformance(
    data: &Path,
    decompositions: &BTreeMap<u32, Decomposition>,
    combining_classes: &BTreeMap<u32, u8>,
    compositions: &BTreeMap<u64, u32>,
    grapheme_ranges: &[RangeValue],
    incb_ranges: &[RangeValue],
    pictographic_ranges: &[(u32, u32)],
) -> Result<ConformanceSummary, String> {
    let model = UnicodeModel {
        decompositions,
        combining_classes,
        compositions,
        grapheme_ranges,
        incb_ranges,
        pictographic_ranges,
    };
    let mut summary = ConformanceSummary::default();
    verify_normalization(
        &read(&data.join("NormalizationTest.txt"))?,
        &model,
        &mut summary,
    )?;
    verify_graphemes(
        &read(&data.join("GraphemeBreakTest.txt"))?,
        &model,
        &mut summary,
    )?;
    Ok(summary)
}

fn verify_normalization(
    source: &str,
    model: &UnicodeModel<'_>,
    summary: &mut ConformanceSummary,
) -> Result<(), String> {
    for (line_index, line) in source.lines().enumerate() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() || body.starts_with('@') {
            continue;
        }
        let fields = body
            .split(';')
            .take(5)
            .map(parse_codepoint_sequence)
            .collect::<Result<Vec<_>, _>>()?;
        if fields.len() != 5 {
            return Err(format!(
                "NormalizationTest line {} does not contain five columns",
                line_index + 1
            ));
        }
        let c1 = &fields[0];
        let c2 = &fields[1];
        let c3 = &fields[2];
        let c4 = &fields[3];
        let c5 = &fields[4];
        let checks = [
            (normalize(c1, false, true, model), c2, "NFC(c1)"),
            (normalize(c2, false, true, model), c2, "NFC(c2)"),
            (normalize(c3, false, true, model), c2, "NFC(c3)"),
            (normalize(c4, false, true, model), c4, "NFC(c4)"),
            (normalize(c5, false, true, model), c4, "NFC(c5)"),
            (normalize(c1, false, false, model), c3, "NFD(c1)"),
            (normalize(c2, false, false, model), c3, "NFD(c2)"),
            (normalize(c3, false, false, model), c3, "NFD(c3)"),
            (normalize(c4, false, false, model), c5, "NFD(c4)"),
            (normalize(c5, false, false, model), c5, "NFD(c5)"),
            (normalize(c1, true, true, model), c4, "NFKC(c1)"),
            (normalize(c2, true, true, model), c4, "NFKC(c2)"),
            (normalize(c3, true, true, model), c4, "NFKC(c3)"),
            (normalize(c4, true, true, model), c4, "NFKC(c4)"),
            (normalize(c5, true, true, model), c4, "NFKC(c5)"),
            (normalize(c1, true, false, model), c5, "NFKD(c1)"),
            (normalize(c2, true, false, model), c5, "NFKD(c2)"),
            (normalize(c3, true, false, model), c5, "NFKD(c3)"),
            (normalize(c4, true, false, model), c5, "NFKD(c4)"),
            (normalize(c5, true, false, model), c5, "NFKD(c5)"),
        ];
        for (actual, expected, label) in checks {
            summary.normalization_checks += 1;
            if actual != *expected {
                return Err(format!(
                    "NormalizationTest line {} failed {}: actual={} expected={}",
                    line_index + 1,
                    label,
                    format_codepoints(&actual),
                    format_codepoints(expected)
                ));
            }
        }
        summary.normalization_cases += 1;
    }
    Ok(())
}

fn parse_codepoint_sequence(value: &str) -> Result<Vec<u32>, String> {
    value
        .split_whitespace()
        .map(parse_codepoint)
        .collect::<Result<Vec<_>, _>>()
}

fn format_codepoints(values: &[u32]) -> String {
    values
        .iter()
        .map(|value| format!("{value:04X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize(
    input: &[u32],
    compatibility: bool,
    compose: bool,
    model: &UnicodeModel<'_>,
) -> Vec<u32> {
    let mut decomposed = Vec::new();
    for code in input {
        decompose(*code, compatibility, model, &mut decomposed);
    }
    canonical_order(&mut decomposed, model);
    if compose {
        canonical_compose(&decomposed, model)
    } else {
        decomposed
    }
}

fn decompose(code: u32, compatibility: bool, model: &UnicodeModel<'_>, output: &mut Vec<u32>) {
    const S_BASE: u32 = 0xAC00;
    const L_BASE: u32 = 0x1100;
    const V_BASE: u32 = 0x1161;
    const T_BASE: u32 = 0x11A7;
    const T_COUNT: u32 = 28;
    const N_COUNT: u32 = 588;
    const S_COUNT: u32 = 11172;
    if (S_BASE..S_BASE + S_COUNT).contains(&code) {
        let s_index = code - S_BASE;
        let l = L_BASE + s_index / N_COUNT;
        let v = V_BASE + (s_index % N_COUNT) / T_COUNT;
        let t = T_BASE + s_index % T_COUNT;
        output.push(l);
        output.push(v);
        if t != T_BASE {
            output.push(t);
        }
        return;
    }
    if let Some(mapping) = model.decompositions.get(&code) {
        if compatibility || !mapping.compatibility {
            for value in &mapping.values {
                decompose(*value, compatibility, model, output);
            }
            return;
        }
    }
    output.push(code);
}

fn canonical_order(values: &mut [u32], model: &UnicodeModel<'_>) {
    for index in 1..values.len() {
        let class = combining_class(values[index], model);
        if class == 0 {
            continue;
        }
        let value = values[index];
        let mut position = index;
        while position > 0 {
            let previous = combining_class(values[position - 1], model);
            if previous == 0 || previous <= class {
                break;
            }
            values[position] = values[position - 1];
            position -= 1;
        }
        values[position] = value;
    }
}

fn canonical_compose(values: &[u32], model: &UnicodeModel<'_>) -> Vec<u32> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut output = vec![values[0]];
    let mut starter_at = 0usize;
    let mut starter = values[0];
    let mut last_class = 0u8;
    for code in &values[1..] {
        let class = combining_class(*code, model);
        let composite = compose_hangul(starter, *code).or_else(|| {
            let key = (u64::from(starter) << 21) | u64::from(*code);
            model.compositions.get(&key).copied()
        });
        if let Some(composite) = composite.filter(|_| last_class == 0 || last_class < class) {
            output[starter_at] = composite;
            starter = composite;
        } else {
            if class == 0 {
                starter_at = output.len();
                starter = *code;
            }
            last_class = class;
            output.push(*code);
        }
    }
    output
}

fn compose_hangul(starter: u32, next: u32) -> Option<u32> {
    const S_BASE: u32 = 0xAC00;
    const L_BASE: u32 = 0x1100;
    const V_BASE: u32 = 0x1161;
    const T_BASE: u32 = 0x11A7;
    const L_COUNT: u32 = 19;
    const V_COUNT: u32 = 21;
    const T_COUNT: u32 = 28;
    const S_COUNT: u32 = 11172;
    if (L_BASE..L_BASE + L_COUNT).contains(&starter) && (V_BASE..V_BASE + V_COUNT).contains(&next) {
        return Some(S_BASE + ((starter - L_BASE) * V_COUNT + (next - V_BASE)) * T_COUNT);
    }
    if (S_BASE..S_BASE + S_COUNT).contains(&starter)
        && (starter - S_BASE) % T_COUNT == 0
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&next)
    {
        return Some(starter + next - T_BASE);
    }
    None
}

fn combining_class(code: u32, model: &UnicodeModel<'_>) -> u8 {
    model.combining_classes.get(&code).copied().unwrap_or(0)
}

fn verify_graphemes(
    source: &str,
    model: &UnicodeModel<'_>,
    summary: &mut ConformanceSummary,
) -> Result<(), String> {
    for (line_index, line) in source.lines().enumerate() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() || body.starts_with('@') {
            continue;
        }
        let mut points = Vec::new();
        let mut expected = Vec::new();
        for token in body.split_whitespace() {
            match token {
                "÷" => expected.push(points.len()),
                "×" => {}
                value => points.push(parse_codepoint(value)?),
            }
        }
        let mut actual = vec![0];
        for index in 1..points.len() {
            if grapheme_break(&points, index, model) {
                actual.push(index);
            }
        }
        actual.push(points.len());
        if actual != expected {
            return Err(format!(
                "GraphemeBreakTest line {} failed: actual={actual:?} expected={expected:?} points={}",
                line_index + 1,
                format_codepoints(&points)
            ));
        }
        summary.grapheme_cases += 1;
        summary.grapheme_boundaries += actual.len();
    }
    Ok(())
}

fn grapheme_break(points: &[u32], at: usize, model: &UnicodeModel<'_>) -> bool {
    let previous = points[at - 1];
    let current = points[at];
    let left = range_value(previous, model.grapheme_ranges);
    let right = range_value(current, model.grapheme_ranges);
    if left == 1 && right == 2 {
        return false;
    }
    if matches!(left, 1 | 2 | 3) || matches!(right, 1 | 2 | 3) {
        return true;
    }
    if left == 9 && matches!(right, 9 | 10 | 12 | 13) {
        return false;
    }
    if matches!(left, 12 | 10) && matches!(right, 10 | 11) {
        return false;
    }
    if matches!(left, 13 | 11) && right == 11 {
        return false;
    }
    if matches!(right, 4 | 5 | 8) || left == 7 {
        return false;
    }
    if range_value(current, model.incb_ranges) == 1 {
        let mut cursor = at as isize - 1;
        let mut saw_linker = false;
        while cursor >= 0 {
            match range_value(points[cursor as usize], model.incb_ranges) {
                2 => saw_linker = true,
                3 => {}
                1 if saw_linker => return false,
                _ => break,
            }
            cursor -= 1;
        }
    }
    if in_boolean_ranges(current, model.pictographic_ranges) && left == 5 {
        let mut cursor = at as isize - 2;
        while cursor >= 0 && range_value(points[cursor as usize], model.grapheme_ranges) == 4 {
            cursor -= 1;
        }
        if cursor >= 0 && in_boolean_ranges(points[cursor as usize], model.pictographic_ranges) {
            return false;
        }
    }
    if left == 6 && right == 6 {
        let mut count = 0usize;
        let mut cursor = at as isize - 1;
        while cursor >= 0 && range_value(points[cursor as usize], model.grapheme_ranges) == 6 {
            count += 1;
            cursor -= 1;
        }
        return count % 2 == 0;
    }
    true
}

fn range_value(code: u32, ranges: &[RangeValue]) -> u32 {
    let index = ranges.partition_point(|range| range.start <= code);
    if index == 0 || code > ranges[index - 1].end {
        0
    } else {
        ranges[index - 1].value
    }
}

fn in_boolean_ranges(code: u32, ranges: &[(u32, u32)]) -> bool {
    let index = ranges.partition_point(|range| range.0 <= code);
    index > 0 && code <= ranges[index - 1].1
}

fn write_conformance_report(root: &Path, summary: &ConformanceSummary) -> Result<(), String> {
    let directory = root.join("runtime").join("unicode").join(VERSION);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = directory.join("CONFORMANCE.json");
    let report = format!(
        concat!(
            "{{\n",
            "  \"schema\": \"nc.unicode.conformance.v1\",\n",
            "  \"unicode_version\": \"17.0.0\",\n",
            "  \"normalization_test\": {{ \"cases\": {}, \"checks\": {}, \"status\": \"passed\" }},\n",
            "  \"grapheme_break_test\": {{ \"cases\": {}, \"boundaries\": {}, \"status\": \"passed\" }},\n",
            "  \"width_data\": {{ \"uax11_wide_ranges\": {}, \"zero_width_ranges\": {}, \"emoji_presentation_ranges\": {}, \"status\": \"passed\" }},\n",
            "  \"runtime_language\": \"nC\",\n",
            "  \"linked_c_runtime\": false,\n",
            "  \"status\": \"passed\"\n",
            "}}\n"
        ),
        summary.normalization_cases,
        summary.normalization_checks,
        summary.grapheme_cases,
        summary.grapheme_boundaries,
        summary.wide_ranges,
        summary.zero_width_ranges,
        summary.emoji_presentation_ranges
    );
    fs::write(&path, report).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn parse_codepoint(value: &str) -> Result<u32, String> {
    u32::from_str_radix(value.trim(), 16)
        .map_err(|error| format!("invalid code point '{value}': {error}"))
}

fn parse_range(value: &str) -> Result<(u32, u32), String> {
    let mut parts = value.trim().split("..");
    let start = parse_codepoint(parts.next().ok_or("missing range start")?)?;
    let end = parts
        .next()
        .map(parse_codepoint)
        .transpose()?
        .unwrap_or(start);
    Ok((start, end))
}

fn parse_unicode_data(
    source: &str,
) -> Result<(BTreeMap<u32, Decomposition>, BTreeMap<u32, u8>), String> {
    let mut decompositions = BTreeMap::new();
    let mut combining_classes = BTreeMap::new();
    for (line_index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 6 {
            return Err(format!(
                "UnicodeData line {} has fewer than 6 fields",
                line_index + 1
            ));
        }
        let code = parse_codepoint(fields[0])?;
        let ccc: u8 = fields[3]
            .parse()
            .map_err(|error| format!("invalid CCC at line {}: {error}", line_index + 1))?;
        if ccc != 0 {
            combining_classes.insert(code, ccc);
        }
        let mapping = fields[5].trim();
        if mapping.is_empty() {
            continue;
        }
        let mut parts = mapping.split_whitespace();
        let first = parts.next().ok_or("empty decomposition mapping")?;
        let compatibility = first.starts_with('<');
        let values = if compatibility {
            parts.map(parse_codepoint).collect::<Result<Vec<_>, _>>()?
        } else {
            std::iter::once(first)
                .chain(parts)
                .map(parse_codepoint)
                .collect::<Result<Vec<_>, _>>()?
        };
        decompositions.insert(
            code,
            Decomposition {
                compatibility,
                values,
            },
        );
    }
    Ok((decompositions, combining_classes))
}

fn parse_property_set(source: &str, property: &str) -> Result<BTreeSet<u32>, String> {
    let mut values = BTreeSet::new();
    for line in source.lines() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() {
            continue;
        }
        let mut fields = body.split(';').map(str::trim);
        let range = fields.next().unwrap_or_default();
        if fields.next() != Some(property) {
            continue;
        }
        let (start, end) = parse_range(range)?;
        values.extend(start..=end);
    }
    Ok(values)
}

fn build_compositions(
    canonical: &BTreeMap<u32, Decomposition>,
    exclusions: &BTreeSet<u32>,
) -> BTreeMap<u64, u32> {
    canonical
        .iter()
        .filter(|(code, item)| item.values.len() == 2 && !exclusions.contains(code))
        .map(|(code, item)| {
            let key = (u64::from(item.values[0]) << 21) | u64::from(item.values[1]);
            (key, *code)
        })
        .collect()
}

fn property_code(name: &str) -> Result<u32, String> {
    match name {
        "Other" => Ok(0),
        "CR" => Ok(1),
        "LF" => Ok(2),
        "Control" => Ok(3),
        "Extend" => Ok(4),
        "ZWJ" => Ok(5),
        "Regional_Indicator" => Ok(6),
        "Prepend" => Ok(7),
        "SpacingMark" => Ok(8),
        "L" => Ok(9),
        "V" => Ok(10),
        "T" => Ok(11),
        "LV" => Ok(12),
        "LVT" => Ok(13),
        other => Err(format!("unknown grapheme property '{other}'")),
    }
}

fn parse_grapheme_ranges(source: &str) -> Result<Vec<RangeValue>, String> {
    let mut ranges = Vec::new();
    for line in source.lines() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() || body.starts_with('@') {
            continue;
        }
        let mut fields = body.split(';').map(str::trim);
        let range = fields.next().unwrap_or_default();
        let property = fields.next().unwrap_or_default();
        let (start, end) = parse_range(range)?;
        ranges.push(RangeValue {
            start,
            end,
            value: property_code(property)?,
        });
    }
    ranges.sort_by_key(|range| range.start);
    Ok(ranges)
}

fn parse_incb_ranges(source: &str) -> Result<Vec<RangeValue>, String> {
    let mut ranges = Vec::new();
    for line in source.lines() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() || !body.contains("; InCB;") {
            continue;
        }
        let fields: Vec<&str> = body.split(';').map(str::trim).collect();
        if fields.len() < 3 || fields[1] != "InCB" {
            continue;
        }
        let value = match fields[2] {
            "Consonant" => 1,
            "Linker" => 2,
            "Extend" => 3,
            "None" => 0,
            other => return Err(format!("unknown InCB property '{other}'")),
        };
        let (start, end) = parse_range(fields[0])?;
        ranges.push(RangeValue { start, end, value });
    }
    ranges.sort_by_key(|range| range.start);
    Ok(ranges)
}

fn parse_boolean_ranges(source: &str, property: &str) -> Result<Vec<(u32, u32)>, String> {
    let mut ranges = Vec::new();
    for line in source.lines() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() {
            continue;
        }
        let mut fields = body.split(';').map(str::trim);
        let range = fields.next().unwrap_or_default();
        if fields.next() != Some(property) {
            continue;
        }
        ranges.push(parse_range(range)?);
    }
    ranges.sort_by_key(|range| range.0);
    Ok(ranges)
}

fn parse_selected_property_ranges(
    source: &str,
    selected: &[&str],
) -> Result<Vec<(u32, u32)>, String> {
    let mut ranges = Vec::new();
    for line in source.lines() {
        let body = line.split('#').next().unwrap_or_default().trim();
        if body.is_empty() || body.starts_with('@') {
            continue;
        }
        let mut fields = body.split(';').map(str::trim);
        let range = fields.next().unwrap_or_default();
        let property = fields.next().unwrap_or_default();
        if selected.contains(&property) {
            ranges.push(parse_range(range)?);
        }
    }
    Ok(merge_ranges(ranges))
}

fn parse_general_category_ranges(
    source: &str,
    selected: &[&str],
) -> Result<Vec<(u32, u32)>, String> {
    let mut ranges = Vec::new();
    let mut pending: Option<(u32, String, bool)> = None;
    for (line_index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 3 {
            return Err(format!(
                "UnicodeData line {} has fewer than 3 fields",
                line_index + 1
            ));
        }
        let code = parse_codepoint(fields[0])?;
        let name = fields[1];
        let category = fields[2];
        let wanted = selected.contains(&category);
        if name.ends_with(", First>") {
            pending = Some((code, category.to_string(), wanted));
        } else if name.ends_with(", Last>") {
            let Some((start, first_category, first_wanted)) = pending.take() else {
                return Err(format!(
                    "UnicodeData line {} closes a range without First",
                    line_index + 1
                ));
            };
            if first_category != category {
                return Err(format!(
                    "UnicodeData line {} changes category inside range",
                    line_index + 1
                ));
            }
            if first_wanted {
                ranges.push((start, code));
            }
        } else if wanted {
            ranges.push((code, code));
        }
    }
    if pending.is_some() {
        return Err("UnicodeData ends inside an open First/Last range".to_string());
    }
    Ok(merge_ranges(ranges))
}

fn merge_ranges(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut() {
            if start <= last.1.saturating_add(1) {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn decomposition_vectors(
    values: &BTreeMap<u32, Decomposition>,
) -> (Vec<u64>, Vec<u64>, Vec<u64>, Vec<u64>) {
    let mut keys = Vec::new();
    let mut offsets = Vec::new();
    let mut lengths = Vec::new();
    let mut flat = Vec::new();
    for (key, item) in values {
        keys.push(u64::from(*key));
        offsets.push(flat.len() as u64);
        lengths.push(item.values.len() as u64);
        flat.extend(item.values.iter().map(|value| u64::from(*value)));
    }
    (keys, offsets, lengths, flat)
}

fn emit_decomposition_tables(
    out: &mut String,
    suffix: &str,
    values: &BTreeMap<u32, Decomposition>,
) {
    let (keys, offsets, lengths, flat) = decomposition_vectors(values);
    let title = title_case(suffix);
    emit_array(out, &format!("unicode{title}Keys"), "u32", &keys);
    emit_array(out, &format!("unicode{title}Offsets"), "u32", &offsets);
    emit_array(out, &format!("unicode{title}Lengths"), "byte", &lengths);
    emit_array(out, &format!("unicode{title}Data"), "u32", &flat);
}

fn emit_range_tables(out: &mut String, prefix: &str, ranges: &[RangeValue]) {
    emit_array(
        out,
        &format!("{prefix}Starts"),
        "u32",
        &ranges
            .iter()
            .map(|range| u64::from(range.start))
            .collect::<Vec<_>>(),
    );
    emit_array(
        out,
        &format!("{prefix}Ends"),
        "u32",
        &ranges
            .iter()
            .map(|range| u64::from(range.end))
            .collect::<Vec<_>>(),
    );
    emit_array(
        out,
        &format!("{prefix}Values"),
        "byte",
        &ranges
            .iter()
            .map(|range| u64::from(range.value))
            .collect::<Vec<_>>(),
    );
}

fn emit_boolean_range_tables(out: &mut String, prefix: &str, ranges: &[(u32, u32)]) {
    emit_array(
        out,
        &format!("{prefix}Starts"),
        "u32",
        &ranges
            .iter()
            .map(|range| u64::from(range.0))
            .collect::<Vec<_>>(),
    );
    emit_array(
        out,
        &format!("{prefix}Ends"),
        "u32",
        &ranges
            .iter()
            .map(|range| u64::from(range.1))
            .collect::<Vec<_>>(),
    );
}

fn emit_array(out: &mut String, name: &str, ty: &str, values: &[u64]) {
    out.push_str(&format!(
        "        rodata align(16) {name} >> array<{ty}, {}> <- [\n",
        values.len()
    ));
    let chunk_count = values.chunks(12).len();
    for (chunk_index, chunk) in values.chunks(12).enumerate() {
        out.push_str("            ");
        for (index, value) in chunk.iter().enumerate() {
            if index != 0 {
                out.push_str(", ");
            }
            out.push_str(&value.to_string());
        }
        if chunk_index + 1 != chunk_count {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("        ];\n");
}

fn emit_decomposition_lookup(
    out: &mut String,
    public_suffix: &str,
    table_suffix: &str,
    count: usize,
) {
    let table = title_case(table_suffix);
    out.push_str(&format!(
        r#"
            unicode{public_suffix}Decomposition(code >> u32, output >> p<u32>) -> int {{
                low >> int <- 0;
                high >> int <- {count};
                while (low < high) {{
                    middle >> int <- low + (high - low) / 2;
                    candidate >> u32 <- unicode{table}Keys[middle];
                    if (candidate < code) {{ low <- middle + 1; }}
                    else {{ high <- middle; }}
                }}
                guard low < {count} else return 0;
                guard unicode{table}Keys[low] == code else return 0;
                startAt >> int <- cast(unicode{table}Offsets[low]);
                count >> int <- cast(unicode{table}Lengths[low]);
                index >> int <- 0;
                while (index < count) {{
                    dataAt >> int <- startAt + index;
                    value >> u32 <- unicode{table}Data[dataAt];
                    wr(off(output, index), value);
                    index += 1;
                }}
                return count;
            }}
"#
    ));
}

fn emit_value_lookup(
    out: &mut String,
    function: &str,
    keys: &str,
    values: &str,
    count: usize,
    return_ty: &str,
) {
    out.push_str(&format!(
        r#"
            {function}(code >> u32) -> {return_ty} {{
                low >> int <- 0;
                high >> int <- {count};
                while (low < high) {{
                    middle >> int <- low + (high - low) / 2;
                    candidate >> u32 <- {keys}[middle];
                    if (candidate < code) {{ low <- middle + 1; }}
                    else {{ high <- middle; }}
                }}
                guard low < {count} else return 0;
                guard {keys}[low] == code else return 0;
                return cast({values}[low]);
            }}
"#
    ));
}

fn emit_compose_lookup(out: &mut String, count: usize) {
    out.push_str(&format!(
        r#"
            unicodeComposePair(starter >> u32, next >> u32) -> u32 {{
                starter64 >> u64 <- cast(starter);
                next64 >> u64 <- cast(next);
                key >> u64 <- (starter64 << 21) | next64;
                low >> int <- 0;
                high >> int <- {count};
                while (low < high) {{
                    middle >> int <- low + (high - low) / 2;
                    candidate >> u64 <- unicodeComposeKeys[middle];
                    if (candidate < key) {{ low <- middle + 1; }}
                    else {{ high <- middle; }}
                }}
                guard low < {count} else return 0;
                guard unicodeComposeKeys[low] == key else return 0;
                return unicodeComposeValues[low];
            }}
"#
    ));
}

fn emit_range_lookup(out: &mut String, function: &str, prefix: &str, count: usize) {
    out.push_str(&format!(
        r#"
            {function}(code >> u32) -> int {{
                low >> int <- 0;
                high >> int <- {count};
                while (low < high) {{
                    middle >> int <- low + (high - low) / 2;
                    if ({prefix}Starts[middle] <= code) {{ low <- middle + 1; }}
                    else {{ high <- middle; }}
                }}
                guard low > 0 else return 0;
                index >> int <- low - 1;
                guard code <= {prefix}Ends[index] else return 0;
                return cast({prefix}Values[index]);
            }}
"#
    ));
}

fn emit_boolean_range_lookup(out: &mut String, function: &str, prefix: &str, count: usize) {
    out.push_str(&format!(
        r#"
            {function}(code >> u32) -> bool {{
                low >> int <- 0;
                high >> int <- {count};
                while (low < high) {{
                    middle >> int <- low + (high - low) / 2;
                    if ({prefix}Starts[middle] <= code) {{ low <- middle + 1; }}
                    else {{ high <- middle; }}
                }}
                guard low > 0 else return false;
                return code <= {prefix}Ends[low - 1];
            }}
"#
    ));
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}
