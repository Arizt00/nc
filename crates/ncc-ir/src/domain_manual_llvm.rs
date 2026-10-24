pub(crate) struct DomainManualRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> DomainManualRuntimeCall {
    DomainManualRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) const DOMAIN_MANUAL_CALLS: &[DomainManualRuntimeCall] = &[
    call(
        "nC.runtime.aiManual.aiStride",
        "__ncc_ai_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiWrite",
        "__ncc_ai_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.aiManual.aiId",
        "__ncc_ai_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiKind",
        "__ncc_ai_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiInput",
        "__ncc_ai_input",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiOutput",
        "__ncc_ai_output",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiWeight",
        "__ncc_ai_weight",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiBias",
        "__ncc_ai_bias",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiFlags",
        "__ncc_ai_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiState",
        "__ncc_ai_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiScore",
        "__ncc_ai_score",
        &["ptr", "i64", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiSelect",
        "__ncc_ai_select",
        &["ptr", "i64", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.aiManual.aiCopy",
        "__ncc_ai_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.aiManual.aiClear",
        "__ncc_ai_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.highManual.highStride",
        "__ncc_high_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highWrite",
        "__ncc_high_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "ptr", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.highManual.highKind",
        "__ncc_high_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highKey",
        "__ncc_high_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highValue",
        "__ncc_high_value",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highAux",
        "__ncc_high_aux",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highFlags",
        "__ncc_high_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highSource",
        "__ncc_high_source",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.highManual.highSourceBytes",
        "__ncc_high_source_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highState",
        "__ncc_high_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highEval",
        "__ncc_high_eval",
        &["i64", "i64", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highCopyBytes",
        "__ncc_high_copy_bytes",
        &["ptr", "ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highHashBytes",
        "__ncc_high_hash_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.highManual.highCopy",
        "__ncc_high_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.highManual.highClear",
        "__ncc_high_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.inspectManual.inspectStride",
        "__ncc_inspect_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectWrite",
        "__ncc_inspect_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.inspectManual.inspectEvent",
        "__ncc_inspect_event",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectSite",
        "__ncc_inspect_site",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectValue",
        "__ncc_inspect_value",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectFlags",
        "__ncc_inspect_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectTick",
        "__ncc_inspect_tick",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectDetail",
        "__ncc_inspect_detail",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectSequence",
        "__ncc_inspect_sequence",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectState",
        "__ncc_inspect_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectChecksum",
        "__ncc_inspect_checksum",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectFind",
        "__ncc_inspect_find",
        &["ptr", "i64", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.inspectManual.inspectCopy",
        "__ncc_inspect_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.inspectManual.inspectClear",
        "__ncc_inspect_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.toolingManual.toolingStride",
        "__ncc_tooling_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingWrite",
        "__ncc_tooling_write",
        &[
            "ptr", "i64", "i64", "ptr", "ptr", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.toolingManual.toolingKind",
        "__ncc_tooling_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingPath",
        "__ncc_tooling_path",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.toolingManual.toolingData",
        "__ncc_tooling_data",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.toolingManual.toolingBytes",
        "__ncc_tooling_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingFlags",
        "__ncc_tooling_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingStatus",
        "__ncc_tooling_status",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingHash",
        "__ncc_tooling_hash",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingState",
        "__ncc_tooling_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingHashBytes",
        "__ncc_tooling_hash_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingCountLines",
        "__ncc_tooling_count_lines",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingCopyBytes",
        "__ncc_tooling_copy_bytes",
        &["ptr", "ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingFindByte",
        "__ncc_tooling_find_byte",
        &["ptr", "i64", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.toolingManual.toolingCopy",
        "__ncc_tooling_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.toolingManual.toolingClear",
        "__ncc_tooling_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static DomainManualRuntimeCall> {
    DOMAIN_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    DOMAIN_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

struct RecordFamily<'a> {
    canonical_prefix: &'a str,
    material_prefix: &'a str,
    fields: &'a [(&'a str, &'a str)],
}

fn render_record_family(family: &RecordFamily<'_>) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "define i64 @{}stride() {{ ret i64 64 }}\n",
        family.material_prefix
    ));

    let write_params = family
        .fields
        .iter()
        .map(|(name, ty)| format!("{ty} %{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "define i1 @{}write(ptr %table, i64 %index, {write_params}) {{\nentry:\n",
        family.material_prefix
    ));
    let mut values = Vec::new();
    for (name, ty) in family.fields {
        if *ty == "ptr" {
            out.push_str(&format!("  %{name}_int = ptrtoint ptr %{name} to i64\n"));
            values.push(format!("i64 %{name}_int"));
        } else {
            values.push(format!("i64 %{name}"));
        }
    }
    out.push_str(&format!(
        "  %ok = call i1 @__ncc_dom_record_write(ptr %table, i64 %index, {})\n  ret i1 %ok\n}}\n",
        values.join(", ")
    ));

    for (index, (name, ty)) in family.fields.iter().enumerate() {
        let material_name = format!("{}{}", family.material_prefix, camel_to_snake(name));
        out.push_str(&format!(
            "define {ty} @{material_name}(ptr %table, i64 %index) {{\nentry:\n  %value = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 {})\n",
            index * 8
        ));
        if *ty == "ptr" {
            out.push_str("  %pointer = inttoptr i64 %value to ptr\n  ret ptr %pointer\n}\n");
        } else {
            out.push_str("  ret i64 %value\n}\n");
        }
    }

    out.push_str(&format!(
        "define i1 @{}copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {{\nentry:\n  %ok = call i1 @__ncc_dom_record_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index)\n  ret i1 %ok\n}}\n",
        family.material_prefix
    ));
    out.push_str(&format!(
        "define i1 @{}clear(ptr %table, i64 %index) {{\nentry:\n  %ok = call i1 @__ncc_dom_record_clear(ptr %table, i64 %index)\n  ret i1 %ok\n}}\n",
        family.material_prefix
    ));
    out
}

fn camel_to_snake(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() && index != 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

pub(crate) fn render_domain_manual_runtime() -> String {
    let mut out = String::from(
        r#"
define internal ptr @__ncc_dom_record_slot(ptr %table, i64 %index) {
entry:
  %offset = mul i64 %index, 64
  %slot = getelementptr i8, ptr %table, i64 %offset
  ret ptr %slot
}

define internal i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 %field_offset) {
entry:
  %slot = call ptr @__ncc_dom_record_slot(ptr %table, i64 %index)
  %field = getelementptr i8, ptr %slot, i64 %field_offset
  %value = load i64, ptr %field, align 1
  ret i64 %value
}

define internal i1 @__ncc_dom_record_write(ptr %table, i64 %index, i64 %f0, i64 %f1, i64 %f2, i64 %f3, i64 %f4, i64 %f5, i64 %f6, i64 %f7) {
entry:
  %slot = call ptr @__ncc_dom_record_slot(ptr %table, i64 %index)
  %p0 = getelementptr i8, ptr %slot, i64 0
  %p1 = getelementptr i8, ptr %slot, i64 8
  %p2 = getelementptr i8, ptr %slot, i64 16
  %p3 = getelementptr i8, ptr %slot, i64 24
  %p4 = getelementptr i8, ptr %slot, i64 32
  %p5 = getelementptr i8, ptr %slot, i64 40
  %p6 = getelementptr i8, ptr %slot, i64 48
  %p7 = getelementptr i8, ptr %slot, i64 56
  store i64 %f0, ptr %p0, align 1
  store i64 %f1, ptr %p1, align 1
  store i64 %f2, ptr %p2, align 1
  store i64 %f3, ptr %p3, align 1
  store i64 %f4, ptr %p4, align 1
  store i64 %f5, ptr %p5, align 1
  store i64 %f6, ptr %p6, align 1
  store i64 %f7, ptr %p7, align 1
  ret i1 true
}

define internal i1 @__ncc_dom_record_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {
entry:
  %destination_slot = call ptr @__ncc_dom_record_slot(ptr %destination, i64 %destination_index)
  %source_slot = call ptr @__ncc_dom_record_slot(ptr %source, i64 %source_index)
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %done = icmp uge i64 %index, 64
  br i1 %done, label %exit, label %copy
copy:
  %source_byte = getelementptr i8, ptr %source_slot, i64 %index
  %destination_byte = getelementptr i8, ptr %destination_slot, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i1 true
}

define internal i1 @__ncc_dom_record_clear(ptr %table, i64 %record_index) {
entry:
  %slot = call ptr @__ncc_dom_record_slot(ptr %table, i64 %record_index)
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %clear ]
  %done = icmp uge i64 %index, 64
  br i1 %done, label %exit, label %clear
clear:
  %byte = getelementptr i8, ptr %slot, i64 %index
  store i8 0, ptr %byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i1 true
}

define internal i64 @__ncc_dom_copy_bytes(ptr %destination, ptr %source, i64 %count) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %exit, label %copy
copy:
  %source_byte = getelementptr i8, ptr %source, i64 %index
  %destination_byte = getelementptr i8, ptr %destination, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %index
}

define internal i64 @__ncc_dom_hash_bytes(ptr %data, i64 %count) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %step ]
  %hash = phi i64 [ 1469598103934665603, %entry ], [ %mixed, %step ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %exit, label %step
step:
  %byte_ptr = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %byte_ptr, align 1
  %wide = zext i8 %byte to i64
  %xor = xor i64 %hash, %wide
  %mixed = mul i64 %xor, 1099511628211
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %hash
}
"#,
    );

    for family in [
        RecordFamily {
            canonical_prefix: "nC.runtime.aiManual",
            material_prefix: "__ncc_ai_",
            fields: &[
                ("id", "i64"),
                ("kind", "i64"),
                ("input", "i64"),
                ("output", "i64"),
                ("weight", "i64"),
                ("bias", "i64"),
                ("flags", "i64"),
                ("state", "i64"),
            ],
        },
        RecordFamily {
            canonical_prefix: "nC.runtime.highManual",
            material_prefix: "__ncc_high_",
            fields: &[
                ("kind", "i64"),
                ("key", "i64"),
                ("value", "i64"),
                ("aux", "i64"),
                ("flags", "i64"),
                ("source", "ptr"),
                ("sourceBytes", "i64"),
                ("state", "i64"),
            ],
        },
        RecordFamily {
            canonical_prefix: "nC.runtime.inspectManual",
            material_prefix: "__ncc_inspect_",
            fields: &[
                ("event", "i64"),
                ("site", "i64"),
                ("value", "i64"),
                ("flags", "i64"),
                ("tick", "i64"),
                ("detail", "i64"),
                ("sequence", "i64"),
                ("state", "i64"),
            ],
        },
        RecordFamily {
            canonical_prefix: "nC.runtime.toolingManual",
            material_prefix: "__ncc_tooling_",
            fields: &[
                ("kind", "i64"),
                ("path", "ptr"),
                ("data", "ptr"),
                ("bytes", "i64"),
                ("flags", "i64"),
                ("status", "i64"),
                ("hash", "i64"),
                ("state", "i64"),
            ],
        },
    ] {
        debug_assert!(DOMAIN_MANUAL_CALLS
            .iter()
            .any(|call| call.canonical.starts_with(family.canonical_prefix)));
        out.push_str(&render_record_family(&family));
    }

    out.push_str(
        r#"
define i64 @__ncc_ai_score(ptr %table, i64 %index, i64 %value) {
entry:
  %weight = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 32)
  %bias = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 40)
  %weighted = mul i64 %value, %weight
  %score = add i64 %weighted, %bias
  ret i64 %score
}

define i64 @__ncc_ai_select(ptr %table, i64 %count, i64 %value) {
entry:
  %empty = icmp sle i64 %count, 0
  br i1 %empty, label %empty_exit, label %seed
empty_exit:
  ret i64 -1
seed:
  %seed_score = call i64 @__ncc_ai_score(ptr %table, i64 0, i64 %value)
  br label %loop
loop:
  %index = phi i64 [ 1, %seed ], [ %next, %step ]
  %best_index = phi i64 [ 0, %seed ], [ %chosen_index, %step ]
  %best_score = phi i64 [ %seed_score, %seed ], [ %chosen_score, %step ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %exit, label %step
step:
  %candidate = call i64 @__ncc_ai_score(ptr %table, i64 %index, i64 %value)
  %better = icmp sgt i64 %candidate, %best_score
  %chosen_index = select i1 %better, i64 %index, i64 %best_index
  %chosen_score = select i1 %better, i64 %candidate, i64 %best_score
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %best_index
}

define i64 @__ncc_high_eval(i64 %operation, i64 %left, i64 %right) {
entry:
  switch i64 %operation, label %add [
    i64 1, label %sub
    i64 2, label %mul
    i64 3, label %div
    i64 4, label %rem
    i64 5, label %and
    i64 6, label %or
    i64 7, label %xor
    i64 8, label %shl
    i64 9, label %shr
  ]
add:
  %add_value = add i64 %left, %right
  ret i64 %add_value
sub:
  %sub_value = sub i64 %left, %right
  ret i64 %sub_value
mul:
  %mul_value = mul i64 %left, %right
  ret i64 %mul_value
div:
  %div_value = sdiv i64 %left, %right
  ret i64 %div_value
rem:
  %rem_value = srem i64 %left, %right
  ret i64 %rem_value
and:
  %and_value = and i64 %left, %right
  ret i64 %and_value
or:
  %or_value = or i64 %left, %right
  ret i64 %or_value
xor:
  %xor_value = xor i64 %left, %right
  ret i64 %xor_value
shl:
  %shl_value = shl i64 %left, %right
  ret i64 %shl_value
shr:
  %shr_value = ashr i64 %left, %right
  ret i64 %shr_value
}

define i64 @__ncc_high_copy_bytes(ptr %destination, ptr %source, i64 %count) {
entry:
  %written = call i64 @__ncc_dom_copy_bytes(ptr %destination, ptr %source, i64 %count)
  ret i64 %written
}

define i64 @__ncc_high_hash_bytes(ptr %data, i64 %count) {
entry:
  %hash = call i64 @__ncc_dom_hash_bytes(ptr %data, i64 %count)
  ret i64 %hash
}

define i64 @__ncc_inspect_checksum(ptr %table, i64 %count) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %step ]
  %sum = phi i64 [ 0, %entry ], [ %sum3, %step ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %exit, label %step
step:
  %event = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 0)
  %site = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 8)
  %value = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 16)
  %sum1 = add i64 %sum, %event
  %sum2 = xor i64 %sum1, %site
  %sum3 = add i64 %sum2, %value
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %sum
}

define i64 @__ncc_inspect_find(ptr %table, i64 %count, i64 %wanted) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %miss ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %not_found, label %probe
probe:
  %event = call i64 @__ncc_dom_record_get(ptr %table, i64 %index, i64 0)
  %found = icmp eq i64 %event, %wanted
  br i1 %found, label %hit, label %miss
hit:
  ret i64 %index
miss:
  %next = add i64 %index, 1
  br label %loop
not_found:
  ret i64 -1
}

define i64 @__ncc_tooling_hash_bytes(ptr %data, i64 %count) {
entry:
  %hash = call i64 @__ncc_dom_hash_bytes(ptr %data, i64 %count)
  ret i64 %hash
}

define i64 @__ncc_tooling_count_lines(ptr %data, i64 %count) {
entry:
  %empty = icmp eq i64 %count, 0
  br i1 %empty, label %empty_exit, label %loop
empty_exit:
  ret i64 0
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %step ]
  %lines = phi i64 [ 1, %entry ], [ %updated, %step ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %exit, label %step
step:
  %byte_ptr = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %byte_ptr, align 1
  %newline = icmp eq i8 %byte, 10
  %increment = zext i1 %newline to i64
  %updated = add i64 %lines, %increment
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %lines
}

define i64 @__ncc_tooling_copy_bytes(ptr %destination, ptr %source, i64 %count) {
entry:
  %written = call i64 @__ncc_dom_copy_bytes(ptr %destination, ptr %source, i64 %count)
  ret i64 %written
}

define i64 @__ncc_tooling_find_byte(ptr %data, i64 %count, i64 %wanted) {
entry:
  %wanted_byte = trunc i64 %wanted to i8
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %miss ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %not_found, label %probe
probe:
  %byte_ptr = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %byte_ptr, align 1
  %found = icmp eq i8 %byte, %wanted_byte
  br i1 %found, label %hit, label %miss
hit:
  ret i64 %index
miss:
  %next = add i64 %index, 1
  br label %loop
not_found:
  ret i64 -1
}
"#,
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_runtime_has_material_work_and_no_c_provider() {
        let llvm = render_domain_manual_runtime();
        for material in [
            "@__ncc_ai_score",
            "@__ncc_high_eval",
            "@__ncc_inspect_checksum",
            "@__ncc_tooling_count_lines",
        ] {
            assert!(llvm.contains(material), "missing {material}");
        }
        assert!(!llvm.contains("ncc_rt_"));
        assert!(!llvm.contains("malloc"));
        assert!(!llvm.contains("memcpy"));
        assert!(!llvm.contains("getelementptr inbounds"));
    }
}
