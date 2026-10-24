pub(crate) struct AuthPathRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const AUTH_PATH_MANUAL_CALLS: &[AuthPathRuntimeCall] = &[
    call(
        "nC.runtime.authPath.authPathStride",
        "__ncc_auth_path_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathWrite",
        "__ncc_auth_path_write",
        &[
            "ptr", "i64", "i64", "i64", "ptr", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authPath.authPathKey",
        "__ncc_auth_path_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathBaseKey",
        "__ncc_auth_path_base_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathData",
        "__ncc_auth_path_data",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authPath.authPathBytes",
        "__ncc_auth_path_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathRights",
        "__ncc_auth_path_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathFlags",
        "__ncc_auth_path_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathOwner",
        "__ncc_auth_path_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathState",
        "__ncc_auth_path_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPath.authPathAllows",
        "__ncc_auth_path_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authPath.authPathCopy",
        "__ncc_auth_path_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authPath.authPathClear",
        "__ncc_auth_path_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> AuthPathRuntimeCall {
    AuthPathRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(target: &str, arity: usize) -> Option<&'static AuthPathRuntimeCall> {
    AUTH_PATH_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    AUTH_PATH_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_auth_path_manual_runtime() -> &'static str {
    r#"
define internal ptr @__ncc_auth_path_record(ptr %table, i64 %index) {
  %offset = mul i64 %index, 64
  %record = getelementptr i8, ptr %table, i64 %offset
  ret ptr %record
}

define i64 @__ncc_auth_path_stride() { ret i64 64 }

define i1 @__ncc_auth_path_write(ptr %table, i64 %index, i64 %path_key, i64 %base_key, ptr %data, i64 %bytes, i64 %rights, i64 %flags, i64 %owner, i64 %state) {
  %record = call ptr @__ncc_auth_path_record(ptr %table, i64 %index)
  %base_slot = getelementptr i8, ptr %record, i64 8
  %data_slot = getelementptr i8, ptr %record, i64 16
  %bytes_slot = getelementptr i8, ptr %record, i64 24
  %rights_slot = getelementptr i8, ptr %record, i64 32
  %flags_slot = getelementptr i8, ptr %record, i64 40
  %owner_slot = getelementptr i8, ptr %record, i64 48
  %state_slot = getelementptr i8, ptr %record, i64 56
  store i64 %path_key, ptr %record, align 1
  store i64 %base_key, ptr %base_slot, align 1
  store ptr %data, ptr %data_slot, align 1
  store i64 %bytes, ptr %bytes_slot, align 1
  store i64 %rights, ptr %rights_slot, align 1
  store i64 %flags, ptr %flags_slot, align 1
  store i64 %owner, ptr %owner_slot, align 1
  store i64 %state, ptr %state_slot, align 1
  ret i1 true
}

define i64 @__ncc_auth_path_key(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_auth_path_base_key(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define ptr @__ncc_auth_path_data(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 16 %v = load ptr, ptr %s, align 1 ret ptr %v }
define i64 @__ncc_auth_path_bytes(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_auth_path_rights(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 32 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_auth_path_flags(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 40 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_auth_path_owner(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 48 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_auth_path_state(ptr %table, i64 %index) { %r = call ptr @__ncc_auth_path_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 56 %v = load i64, ptr %s, align 1 ret i64 %v }

define i1 @__ncc_auth_path_allows(ptr %table, i64 %index, i64 %required) {
  %rights = call i64 @__ncc_auth_path_rights(ptr %table, i64 %index)
  %state = call i64 @__ncc_auth_path_state(ptr %table, i64 %index)
  %selected = and i64 %rights, %required
  %has = icmp eq i64 %selected, %required
  %active = icmp ne i64 %state, 0
  %ok = and i1 %has, %active
  ret i1 %ok
}

define i1 @__ncc_auth_path_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {
  %destination_record = call ptr @__ncc_auth_path_record(ptr %destination, i64 %destination_index)
  %source_record = call ptr @__ncc_auth_path_record(ptr %source, i64 %source_index)
  br label %loop
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %index, 64
  br i1 %done, label %exit, label %write
write:
  %source_byte = getelementptr i8, ptr %source_record, i64 %index
  %destination_byte = getelementptr i8, ptr %destination_record, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i1 true
}

define i1 @__ncc_auth_path_clear(ptr %table, i64 %index) {
  %record = call ptr @__ncc_auth_path_record(ptr %table, i64 %index)
  br label %loop
loop:
  %offset = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %offset, 64
  br i1 %done, label %exit, label %write
write:
  %byte = getelementptr i8, ptr %record, i64 %offset
  store i8 0, ptr %byte, align 1
  %next = add i64 %offset, 1
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
    fn auth_path_runtime_is_manual_raw_and_has_no_c_provider() {
        let llvm = render_auth_path_manual_runtime();
        assert!(llvm.contains("store ptr %data"));
        assert!(llvm.contains("@__ncc_auth_path_allows"));
        assert!(!llvm.contains("ncc_rt_"));
        assert!(!llvm.contains("malloc"));
        assert!(!llvm.contains("getelementptr inbounds"));
    }
}
