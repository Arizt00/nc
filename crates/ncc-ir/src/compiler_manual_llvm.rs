pub(crate) struct CompilerManualRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const COMPILER_MANUAL_CALLS: &[CompilerManualRuntimeCall] = &[
    call(
        "nC.runtime.compilerManual.compilerUnitStride",
        "__ncc_compiler_unit_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerTokenStride",
        "__ncc_compiler_token_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerNodeStride",
        "__ncc_compiler_node_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerIrStride",
        "__ncc_compiler_ir_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerDiagnosticStride",
        "__ncc_compiler_diagnostic_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerManifestStride",
        "__ncc_compiler_manifest_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerUnitInit",
        "__ncc_compiler_unit_init",
        &[
            "ptr", "ptr", "i64", "ptr", "i64", "ptr", "i64", "ptr", "i64", "ptr", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerParse",
        "__ncc_compiler_parse",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerAst",
        "__ncc_compiler_ast",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerSema",
        "__ncc_compiler_sema",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerLower",
        "__ncc_compiler_lower",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerDiagnosticWrite",
        "__ncc_compiler_diagnostic_write",
        &["ptr", "i64", "i64", "i64", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerDiagnosticCount",
        "__ncc_compiler_diagnostic_count",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerReport",
        "__ncc_compiler_report",
        &["ptr", "ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerManifest",
        "__ncc_compiler_manifest",
        &["ptr", "ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerTokenCount",
        "__ncc_compiler_token_count",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerNodeCount",
        "__ncc_compiler_node_count",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerIrCount",
        "__ncc_compiler_ir_count",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerState",
        "__ncc_compiler_state",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.compilerManual.compilerReset",
        "__ncc_compiler_reset",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.compilerManual.compilerCopy",
        "__ncc_compiler_copy",
        &["ptr", "ptr"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> CompilerManualRuntimeCall {
    CompilerManualRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static CompilerManualRuntimeCall> {
    COMPILER_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    COMPILER_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_compiler_manual_runtime() -> &'static str {
    r#"
define i64 @__ncc_compiler_unit_stride() { ret i64 128 }
define i64 @__ncc_compiler_token_stride() { ret i64 24 }
define i64 @__ncc_compiler_node_stride() { ret i64 32 }
define i64 @__ncc_compiler_ir_stride() { ret i64 32 }
define i64 @__ncc_compiler_diagnostic_stride() { ret i64 40 }
define i64 @__ncc_compiler_manifest_stride() { ret i64 48 }

define internal ptr @__ncc_compiler_slot(ptr %unit, i64 %offset) {
entry:
  %slot = getelementptr i8, ptr %unit, i64 %offset
  ret ptr %slot
}

define internal i64 @__ncc_compiler_load_i64(ptr %unit, i64 %offset) {
entry:
  %slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 %offset)
  %value = load i64, ptr %slot, align 1
  ret i64 %value
}

define internal ptr @__ncc_compiler_load_ptr(ptr %unit, i64 %offset) {
entry:
  %slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 %offset)
  %value = load ptr, ptr %slot, align 1
  ret ptr %value
}

define internal void @__ncc_compiler_store_i64(ptr %unit, i64 %offset, i64 %value) {
entry:
  %slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 %offset)
  store i64 %value, ptr %slot, align 1
  ret void
}

define i1 @__ncc_compiler_unit_init(ptr %unit, ptr %source, i64 %source_bytes, ptr %tokens, i64 %token_capacity, ptr %nodes, i64 %node_capacity, ptr %ir, i64 %ir_capacity, ptr %diagnostics, i64 %diagnostic_capacity, i64 %flags) {
entry:
  %source_slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 0)
  %tokens_slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 16)
  %nodes_slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 40)
  %ir_slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 64)
  %diagnostics_slot = call ptr @__ncc_compiler_slot(ptr %unit, i64 88)
  store ptr %source, ptr %source_slot, align 1
  call void @__ncc_compiler_store_i64(ptr %unit, i64 8, i64 %source_bytes)
  store ptr %tokens, ptr %tokens_slot, align 1
  call void @__ncc_compiler_store_i64(ptr %unit, i64 24, i64 %token_capacity)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 32, i64 0)
  store ptr %nodes, ptr %nodes_slot, align 1
  call void @__ncc_compiler_store_i64(ptr %unit, i64 48, i64 %node_capacity)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 56, i64 0)
  store ptr %ir, ptr %ir_slot, align 1
  call void @__ncc_compiler_store_i64(ptr %unit, i64 72, i64 %ir_capacity)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 80, i64 0)
  store ptr %diagnostics, ptr %diagnostics_slot, align 1
  call void @__ncc_compiler_store_i64(ptr %unit, i64 96, i64 %diagnostic_capacity)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 104, i64 0)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 112, i64 %flags)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 0)
  ret i1 true
}

define internal i1 @__ncc_compiler_is_space(i8 %byte) {
entry:
  %space = icmp eq i8 %byte, 32
  %tab = icmp eq i8 %byte, 9
  %lf = icmp eq i8 %byte, 10
  %cr = icmp eq i8 %byte, 13
  %a = or i1 %space, %tab
  %b = or i1 %lf, %cr
  %result = or i1 %a, %b
  ret i1 %result
}

define internal i64 @__ncc_compiler_token_kind(i8 %byte) {
entry:
  %digit_low = icmp uge i8 %byte, 48
  %digit_high = icmp ule i8 %byte, 57
  %digit = and i1 %digit_low, %digit_high
  %quote = icmp eq i8 %byte, 34
  %punct_a = icmp eq i8 %byte, 40
  %punct_b = icmp eq i8 %byte, 41
  %punct_c = icmp eq i8 %byte, 123
  %punct_d = icmp eq i8 %byte, 125
  %punct_e = icmp eq i8 %byte, 59
  %punct_ab = or i1 %punct_a, %punct_b
  %punct_cd = or i1 %punct_c, %punct_d
  %punct_abcd = or i1 %punct_ab, %punct_cd
  %punct = or i1 %punct_abcd, %punct_e
  %digit_kind = select i1 %digit, i64 2, i64 1
  %quote_kind = select i1 %quote, i64 3, i64 %digit_kind
  %kind = select i1 %punct, i64 4, i64 %quote_kind
  ret i64 %kind
}

define i1 @__ncc_compiler_parse(ptr %unit) {
entry:
  %source = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 0)
  %source_bytes = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 8)
  %tokens = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 16)
  br label %seek
seek:
  %cursor = phi i64 [ 0, %entry ], [ %skip_next, %skip ], [ %token_end, %emit ]
  %count = phi i64 [ 0, %entry ], [ %count, %skip ], [ %count_next, %emit ]
  %finished = icmp uge i64 %cursor, %source_bytes
  br i1 %finished, label %done, label %classify
classify:
  %byte_ptr = getelementptr i8, ptr %source, i64 %cursor
  %byte = load i8, ptr %byte_ptr, align 1
  %space = call i1 @__ncc_compiler_is_space(i8 %byte)
  br i1 %space, label %skip, label %scan
skip:
  %skip_next = add i64 %cursor, 1
  br label %seek
scan:
  %kind = call i64 @__ncc_compiler_token_kind(i8 %byte)
  %single = icmp eq i64 %kind, 4
  %single_end = add i64 %cursor, 1
  br i1 %single, label %emit_single, label %scan_loop
scan_loop:
  %scan_cursor = phi i64 [ %single_end, %scan ], [ %scan_next, %scan_more ]
  %scan_finished = icmp uge i64 %scan_cursor, %source_bytes
  br i1 %scan_finished, label %emit_scanned, label %scan_byte
scan_byte:
  %scan_ptr = getelementptr i8, ptr %source, i64 %scan_cursor
  %scan_value = load i8, ptr %scan_ptr, align 1
  %scan_space = call i1 @__ncc_compiler_is_space(i8 %scan_value)
  %scan_kind = call i64 @__ncc_compiler_token_kind(i8 %scan_value)
  %scan_punct = icmp eq i64 %scan_kind, 4
  %scan_stop = or i1 %scan_space, %scan_punct
  br i1 %scan_stop, label %emit_scanned, label %scan_more
scan_more:
  %scan_next = add i64 %scan_cursor, 1
  br label %scan_loop
emit_single:
  br label %emit
emit_scanned:
  br label %emit
emit:
  %token_end = phi i64 [ %single_end, %emit_single ], [ %scan_cursor, %emit_scanned ]
  %length = sub i64 %token_end, %cursor
  %token_offset = mul i64 %count, 24
  %token = getelementptr i8, ptr %tokens, i64 %token_offset
  %length_slot = getelementptr i8, ptr %token, i64 8
  %kind_slot = getelementptr i8, ptr %token, i64 16
  store i64 %cursor, ptr %token, align 1
  store i64 %length, ptr %length_slot, align 1
  store i64 %kind, ptr %kind_slot, align 1
  %count_next = add i64 %count, 1
  br label %seek
done:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 32, i64 %count)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 1)
  ret i1 true
}

define i1 @__ncc_compiler_ast(ptr %unit) {
entry:
  %tokens = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 16)
  %token_count = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 32)
  %nodes = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 40)
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %done = icmp uge i64 %index, %token_count
  br i1 %done, label %exit, label %copy
copy:
  %token_offset = mul i64 %index, 24
  %token = getelementptr i8, ptr %tokens, i64 %token_offset
  %token_length_slot = getelementptr i8, ptr %token, i64 8
  %token_kind_slot = getelementptr i8, ptr %token, i64 16
  %start = load i64, ptr %token, align 1
  %length = load i64, ptr %token_length_slot, align 1
  %kind = load i64, ptr %token_kind_slot, align 1
  %node_offset = mul i64 %index, 32
  %node = getelementptr i8, ptr %nodes, i64 %node_offset
  %node_start_slot = getelementptr i8, ptr %node, i64 8
  %node_length_slot = getelementptr i8, ptr %node, i64 16
  %node_parent_slot = getelementptr i8, ptr %node, i64 24
  store i64 %kind, ptr %node, align 1
  store i64 %start, ptr %node_start_slot, align 1
  store i64 %length, ptr %node_length_slot, align 1
  store i64 -1, ptr %node_parent_slot, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 56, i64 %token_count)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 2)
  ret i1 true
}

define i1 @__ncc_compiler_sema(ptr %unit) {
entry:
  %nodes = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 40)
  %node_count = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 56)
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %valid ]
  %done = icmp uge i64 %index, %node_count
  br i1 %done, label %success, label %check
check:
  %offset = mul i64 %index, 32
  %node = getelementptr i8, ptr %nodes, i64 %offset
  %kind = load i64, ptr %node, align 1
  %invalid = icmp eq i64 %kind, 0
  br i1 %invalid, label %failure, label %valid
valid:
  %next = add i64 %index, 1
  br label %loop
failure:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 -1)
  ret i1 false
success:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 3)
  ret i1 true
}

define i1 @__ncc_compiler_lower(ptr %unit) {
entry:
  %nodes = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 40)
  %node_count = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 56)
  %ir = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 64)
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %done = icmp uge i64 %index, %node_count
  br i1 %done, label %exit, label %copy
copy:
  %node_offset = mul i64 %index, 32
  %node = getelementptr i8, ptr %nodes, i64 %node_offset
  %node_start_slot = getelementptr i8, ptr %node, i64 8
  %node_length_slot = getelementptr i8, ptr %node, i64 16
  %kind = load i64, ptr %node, align 1
  %start = load i64, ptr %node_start_slot, align 1
  %length = load i64, ptr %node_length_slot, align 1
  %ir_offset = mul i64 %index, 32
  %instruction = getelementptr i8, ptr %ir, i64 %ir_offset
  %source_slot = getelementptr i8, ptr %instruction, i64 8
  %length_slot = getelementptr i8, ptr %instruction, i64 16
  %flags_slot = getelementptr i8, ptr %instruction, i64 24
  store i64 %kind, ptr %instruction, align 1
  store i64 %start, ptr %source_slot, align 1
  store i64 %length, ptr %length_slot, align 1
  store i64 0, ptr %flags_slot, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 80, i64 %node_count)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 4)
  ret i1 true
}

define i1 @__ncc_compiler_diagnostic_write(ptr %unit, i64 %index, i64 %stage, i64 %code, i64 %offset, i64 %length, i64 %detail) {
entry:
  %diagnostics = call ptr @__ncc_compiler_load_ptr(ptr %unit, i64 88)
  %record_offset = mul i64 %index, 40
  %record = getelementptr i8, ptr %diagnostics, i64 %record_offset
  %code_slot = getelementptr i8, ptr %record, i64 8
  %offset_slot = getelementptr i8, ptr %record, i64 16
  %length_slot = getelementptr i8, ptr %record, i64 24
  %detail_slot = getelementptr i8, ptr %record, i64 32
  store i64 %stage, ptr %record, align 1
  store i64 %code, ptr %code_slot, align 1
  store i64 %offset, ptr %offset_slot, align 1
  store i64 %length, ptr %length_slot, align 1
  store i64 %detail, ptr %detail_slot, align 1
  %count = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 104)
  %next = add i64 %index, 1
  %grows = icmp ugt i64 %next, %count
  %updated = select i1 %grows, i64 %next, i64 %count
  call void @__ncc_compiler_store_i64(ptr %unit, i64 104, i64 %updated)
  ret i1 true
}

define i64 @__ncc_compiler_diagnostic_count(ptr %unit) { %v = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 104) ret i64 %v }
define i64 @__ncc_compiler_token_count(ptr %unit) { %v = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 32) ret i64 %v }
define i64 @__ncc_compiler_node_count(ptr %unit) { %v = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 56) ret i64 %v }
define i64 @__ncc_compiler_ir_count(ptr %unit) { %v = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 80) ret i64 %v }
define i64 @__ncc_compiler_state(ptr %unit) { %v = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 120) ret i64 %v }

define i64 @__ncc_compiler_report(ptr %unit, ptr %out) {
entry:
  %tokens = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 32)
  %nodes = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 56)
  %ir = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 80)
  %diagnostics = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 104)
  %state = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 120)
  %f1 = getelementptr i8, ptr %out, i64 8
  %f2 = getelementptr i8, ptr %out, i64 16
  %f3 = getelementptr i8, ptr %out, i64 24
  %f4 = getelementptr i8, ptr %out, i64 32
  store i64 %tokens, ptr %out, align 1
  store i64 %nodes, ptr %f1, align 1
  store i64 %ir, ptr %f2, align 1
  store i64 %diagnostics, ptr %f3, align 1
  store i64 %state, ptr %f4, align 1
  ret i64 40
}

define i64 @__ncc_compiler_manifest(ptr %unit, ptr %out) {
entry:
  %source_bytes = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 8)
  %tokens = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 32)
  %nodes = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 56)
  %ir = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 80)
  %diagnostics = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 104)
  %state = call i64 @__ncc_compiler_load_i64(ptr %unit, i64 120)
  %f1 = getelementptr i8, ptr %out, i64 8
  %f2 = getelementptr i8, ptr %out, i64 16
  %f3 = getelementptr i8, ptr %out, i64 24
  %f4 = getelementptr i8, ptr %out, i64 32
  %f5 = getelementptr i8, ptr %out, i64 40
  store i64 %source_bytes, ptr %out, align 1
  store i64 %tokens, ptr %f1, align 1
  store i64 %nodes, ptr %f2, align 1
  store i64 %ir, ptr %f3, align 1
  store i64 %diagnostics, ptr %f4, align 1
  store i64 %state, ptr %f5, align 1
  ret i64 48
}

define i1 @__ncc_compiler_reset(ptr %unit) {
entry:
  call void @__ncc_compiler_store_i64(ptr %unit, i64 32, i64 0)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 56, i64 0)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 80, i64 0)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 104, i64 0)
  call void @__ncc_compiler_store_i64(ptr %unit, i64 120, i64 0)
  ret i1 true
}

define i1 @__ncc_compiler_copy(ptr %destination, ptr %source) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %done = icmp uge i64 %index, 128
  br i1 %done, label %exit, label %copy
copy:
  %source_byte = getelementptr i8, ptr %source, i64 %index
  %destination_byte = getelementptr i8, ptr %destination, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i1 true
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_runtime_is_manual_raw_and_has_no_c_provider() {
        let llvm = render_compiler_manual_runtime();
        assert!(llvm.contains("@__ncc_compiler_parse"));
        assert!(llvm.contains("@__ncc_compiler_lower"));
        assert!(llvm.contains("mul i64 %count, 24"));
        assert!(!llvm.contains("ncc_rt_"));
        assert!(!llvm.contains("malloc"));
        assert!(!llvm.contains("getelementptr inbounds"));
    }
}
