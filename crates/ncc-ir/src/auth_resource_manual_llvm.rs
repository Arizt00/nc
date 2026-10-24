use std::fmt::Write as _;

pub(crate) struct AuthResourceRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const AUTH_RESOURCE_MANUAL_CALLS: &[AuthResourceRuntimeCall] = &[
    call(
        "nC.runtime.authHandle.handleStride",
        "__ncc_auth_handle_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authHandle.handleWrite",
        "__ncc_auth_handle_write",
        &["ptr", "i64", "ptr", "i64", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authHandle.handleValue",
        "__ncc_auth_handle_value",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authHandle.handleKind",
        "__ncc_auth_handle_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authHandle.handleRights",
        "__ncc_auth_handle_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authHandle.handleFlags",
        "__ncc_auth_handle_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authHandle.handleGeneration",
        "__ncc_auth_handle_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authHandle.handleAllows",
        "__ncc_auth_handle_allows",
        &["ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authHandle.handleCopy",
        "__ncc_auth_handle_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authHandle.handleClear",
        "__ncc_auth_handle_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authResource.resourceStride",
        "__ncc_auth_resource_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceWrite",
        "__ncc_auth_resource_write",
        &[
            "ptr", "i64", "i64", "ptr", "ptr", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authResource.resourceId",
        "__ncc_auth_resource_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceHandle",
        "__ncc_auth_resource_handle",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authResource.resourceBase",
        "__ncc_auth_resource_base",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authResource.resourceBytes",
        "__ncc_auth_resource_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceKind",
        "__ncc_auth_resource_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceRights",
        "__ncc_auth_resource_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceFlags",
        "__ncc_auth_resource_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceOwner",
        "__ncc_auth_resource_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authResource.resourceContains",
        "__ncc_auth_resource_contains",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authResource.resourceAllows",
        "__ncc_auth_resource_allows",
        &["ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authResource.resourceCopy",
        "__ncc_auth_resource_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authResource.resourceClear",
        "__ncc_auth_resource_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSession.sessionStride",
        "__ncc_auth_session_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionWrite",
        "__ncc_auth_session_write",
        &[
            "ptr", "i64", "i64", "i64", "ptr", "ptr", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authSession.sessionId",
        "__ncc_auth_session_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionOwner",
        "__ncc_auth_session_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionToken",
        "__ncc_auth_session_token",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authSession.sessionResources",
        "__ncc_auth_session_resources",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authSession.sessionResourceCount",
        "__ncc_auth_session_resource_count",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionRights",
        "__ncc_auth_session_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionState",
        "__ncc_auth_session_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSession.sessionHasRight",
        "__ncc_auth_session_has_right",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSession.sessionResourceAt",
        "__ncc_auth_session_resource_at",
        &["ptr", "i64", "i64", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.authSession.sessionCopy",
        "__ncc_auth_session_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSession.sessionClear",
        "__ncc_auth_session_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> AuthResourceRuntimeCall {
    AuthResourceRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static AuthResourceRuntimeCall> {
    AUTH_RESOURCE_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical == target && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    AUTH_RESOURCE_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical == target)
        .map(|call| call.material)
}

fn emit_stride(out: &mut String, symbol: &str, stride: i64) {
    writeln!(
        out,
        "define i64 @{symbol}() {{\nentry:\n  ret i64 {stride}\n}}\n"
    )
    .unwrap();
}

fn emit_reader(out: &mut String, symbol: &str, stride: i64, offset: i64, ty: &str) {
    writeln!(
        out,
        "define {ty} @{symbol}(ptr %table, i64 %index) {{\nentry:\n  %record = call ptr @__ncc_auth_record(ptr %table, i64 %index, i64 {stride})\n  %field = getelementptr i8, ptr %record, i64 {offset}\n  %value = load {ty}, ptr %field, align 8\n  ret {ty} %value\n}}\n"
    )
    .unwrap();
}

fn emit_copy_clear(out: &mut String, prefix: &str, stride: i64) {
    writeln!(
        out,
        "define i1 @__ncc_auth_{prefix}_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {{\nentry:\n  %ok = call i1 @__ncc_auth_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index, i64 {stride})\n  ret i1 %ok\n}}\n\ndefine i1 @__ncc_auth_{prefix}_clear(ptr %table, i64 %index) {{\nentry:\n  %ok = call i1 @__ncc_auth_clear(ptr %table, i64 %index, i64 {stride})\n  ret i1 %ok\n}}\n"
    )
    .unwrap();
}

pub(crate) fn render_auth_resource_manual_runtime() -> String {
    let mut out = String::from(
        r#"
define internal ptr @__ncc_auth_record(ptr %table, i64 %index, i64 %stride) {
entry:
  %offset = mul i64 %index, %stride
  %record = getelementptr i8, ptr %table, i64 %offset
  ret ptr %record
}

define internal i1 @__ncc_auth_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index, i64 %stride) {
entry:
  %destination_record = call ptr @__ncc_auth_record(ptr %destination, i64 %destination_index, i64 %stride)
  %source_record = call ptr @__ncc_auth_record(ptr %source, i64 %source_index, i64 %stride)
  br label %copy
copy:
  %index = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp ult i64 %index, %stride
  br i1 %within, label %write, label %done
write:
  %source_byte = getelementptr i8, ptr %source_record, i64 %index
  %destination_byte = getelementptr i8, ptr %destination_record, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %copy
done:
  ret i1 true
}

define internal i1 @__ncc_auth_clear(ptr %table, i64 %record_index, i64 %stride) {
entry:
  %record = call ptr @__ncc_auth_record(ptr %table, i64 %record_index, i64 %stride)
  br label %clear
clear:
  %index = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp ult i64 %index, %stride
  br i1 %within, label %write, label %done
write:
  %byte = getelementptr i8, ptr %record, i64 %index
  store i8 0, ptr %byte, align 1
  %next = add i64 %index, 1
  br label %clear
done:
  ret i1 true
}

"#,
    );

    emit_stride(&mut out, "__ncc_auth_handle_stride", 40);
    out.push_str(
        r#"define i1 @__ncc_auth_handle_write(ptr %table, i64 %index, ptr %value, i64 %kind, i64 %rights, i64 %flags, i64 %generation) {
entry:
  %record = call ptr @__ncc_auth_record(ptr %table, i64 %index, i64 40)
  %f0 = getelementptr i8, ptr %record, i64 0
  %f1 = getelementptr i8, ptr %record, i64 8
  %f2 = getelementptr i8, ptr %record, i64 16
  %f3 = getelementptr i8, ptr %record, i64 24
  %f4 = getelementptr i8, ptr %record, i64 32
  store ptr %value, ptr %f0, align 8
  store i64 %kind, ptr %f1, align 8
  store i64 %rights, ptr %f2, align 8
  store i64 %flags, ptr %f3, align 8
  store i64 %generation, ptr %f4, align 8
  ret i1 true
}

"#,
    );
    emit_reader(&mut out, "__ncc_auth_handle_value", 40, 0, "ptr");
    emit_reader(&mut out, "__ncc_auth_handle_kind", 40, 8, "i64");
    emit_reader(&mut out, "__ncc_auth_handle_rights", 40, 16, "i64");
    emit_reader(&mut out, "__ncc_auth_handle_flags", 40, 24, "i64");
    emit_reader(&mut out, "__ncc_auth_handle_generation", 40, 32, "i64");
    out.push_str(
        r#"define i1 @__ncc_auth_handle_allows(ptr %table, i64 %index, i64 %required, i64 %forbidden) {
entry:
  %rights = call i64 @__ncc_auth_handle_rights(ptr %table, i64 %index)
  %required_bits = and i64 %rights, %required
  %has_required = icmp eq i64 %required_bits, %required
  %forbidden_bits = and i64 %rights, %forbidden
  %has_no_forbidden = icmp eq i64 %forbidden_bits, 0
  %allowed = and i1 %has_required, %has_no_forbidden
  ret i1 %allowed
}

"#,
    );
    emit_copy_clear(&mut out, "handle", 40);

    emit_stride(&mut out, "__ncc_auth_resource_stride", 64);
    out.push_str(
        r#"define i1 @__ncc_auth_resource_write(ptr %table, i64 %index, i64 %id, ptr %handle, ptr %base, i64 %bytes, i64 %kind, i64 %rights, i64 %flags, i64 %owner) {
entry:
  %record = call ptr @__ncc_auth_record(ptr %table, i64 %index, i64 64)
  %f0 = getelementptr i8, ptr %record, i64 0
  %f1 = getelementptr i8, ptr %record, i64 8
  %f2 = getelementptr i8, ptr %record, i64 16
  %f3 = getelementptr i8, ptr %record, i64 24
  %f4 = getelementptr i8, ptr %record, i64 32
  %f5 = getelementptr i8, ptr %record, i64 40
  %f6 = getelementptr i8, ptr %record, i64 48
  %f7 = getelementptr i8, ptr %record, i64 56
  store i64 %id, ptr %f0, align 8
  store ptr %handle, ptr %f1, align 8
  store ptr %base, ptr %f2, align 8
  store i64 %bytes, ptr %f3, align 8
  store i64 %kind, ptr %f4, align 8
  store i64 %rights, ptr %f5, align 8
  store i64 %flags, ptr %f6, align 8
  store i64 %owner, ptr %f7, align 8
  ret i1 true
}

"#,
    );
    emit_reader(&mut out, "__ncc_auth_resource_id", 64, 0, "i64");
    emit_reader(&mut out, "__ncc_auth_resource_handle", 64, 8, "ptr");
    emit_reader(&mut out, "__ncc_auth_resource_base", 64, 16, "ptr");
    emit_reader(&mut out, "__ncc_auth_resource_bytes", 64, 24, "i64");
    emit_reader(&mut out, "__ncc_auth_resource_kind", 64, 32, "i64");
    emit_reader(&mut out, "__ncc_auth_resource_rights", 64, 40, "i64");
    emit_reader(&mut out, "__ncc_auth_resource_flags", 64, 48, "i64");
    emit_reader(&mut out, "__ncc_auth_resource_owner", 64, 56, "i64");
    out.push_str(
        r#"define i1 @__ncc_auth_resource_contains(ptr %table, i64 %index, ptr %address, i64 %requested_bytes) {
entry:
  %raw_base = call ptr @__ncc_auth_resource_base(ptr %table, i64 %index)
  %available = call i64 @__ncc_auth_resource_bytes(ptr %table, i64 %index)
  %base = ptrtoint ptr %raw_base to i64
  %candidate = ptrtoint ptr %address to i64
  %not_before = icmp uge i64 %candidate, %base
  %offset = sub i64 %candidate, %base
  %offset_fits = icmp ule i64 %offset, %available
  %remaining = sub i64 %available, %offset
  %length_fits = icmp ule i64 %requested_bytes, %remaining
  %base_and_offset = and i1 %not_before, %offset_fits
  %contained = and i1 %base_and_offset, %length_fits
  ret i1 %contained
}

define i1 @__ncc_auth_resource_allows(ptr %table, i64 %index, i64 %required, i64 %forbidden) {
entry:
  %rights = call i64 @__ncc_auth_resource_rights(ptr %table, i64 %index)
  %required_bits = and i64 %rights, %required
  %has_required = icmp eq i64 %required_bits, %required
  %forbidden_bits = and i64 %rights, %forbidden
  %has_no_forbidden = icmp eq i64 %forbidden_bits, 0
  %allowed = and i1 %has_required, %has_no_forbidden
  ret i1 %allowed
}

"#,
    );
    emit_copy_clear(&mut out, "resource", 64);

    emit_stride(&mut out, "__ncc_auth_session_stride", 56);
    out.push_str(
        r#"define i1 @__ncc_auth_session_write(ptr %table, i64 %index, i64 %id, i64 %owner, ptr %token, ptr %resources, i64 %resource_count, i64 %rights, i64 %state) {
entry:
  %record = call ptr @__ncc_auth_record(ptr %table, i64 %index, i64 56)
  %f0 = getelementptr i8, ptr %record, i64 0
  %f1 = getelementptr i8, ptr %record, i64 8
  %f2 = getelementptr i8, ptr %record, i64 16
  %f3 = getelementptr i8, ptr %record, i64 24
  %f4 = getelementptr i8, ptr %record, i64 32
  %f5 = getelementptr i8, ptr %record, i64 40
  %f6 = getelementptr i8, ptr %record, i64 48
  store i64 %id, ptr %f0, align 8
  store i64 %owner, ptr %f1, align 8
  store ptr %token, ptr %f2, align 8
  store ptr %resources, ptr %f3, align 8
  store i64 %resource_count, ptr %f4, align 8
  store i64 %rights, ptr %f5, align 8
  store i64 %state, ptr %f6, align 8
  ret i1 true
}

"#,
    );
    emit_reader(&mut out, "__ncc_auth_session_id", 56, 0, "i64");
    emit_reader(&mut out, "__ncc_auth_session_owner", 56, 8, "i64");
    emit_reader(&mut out, "__ncc_auth_session_token", 56, 16, "ptr");
    emit_reader(&mut out, "__ncc_auth_session_resources", 56, 24, "ptr");
    emit_reader(&mut out, "__ncc_auth_session_resource_count", 56, 32, "i64");
    emit_reader(&mut out, "__ncc_auth_session_rights", 56, 40, "i64");
    emit_reader(&mut out, "__ncc_auth_session_state", 56, 48, "i64");
    out.push_str(
        r#"define i1 @__ncc_auth_session_has_right(ptr %table, i64 %index, i64 %required) {
entry:
  %rights = call i64 @__ncc_auth_session_rights(ptr %table, i64 %index)
  %bits = and i64 %rights, %required
  %allowed = icmp eq i64 %bits, %required
  ret i1 %allowed
}

define ptr @__ncc_auth_session_resource_at(ptr %table, i64 %index, i64 %resource_index, i64 %resource_stride) {
entry:
  %resources = call ptr @__ncc_auth_session_resources(ptr %table, i64 %index)
  %offset = mul i64 %resource_index, %resource_stride
  %resource = getelementptr i8, ptr %resources, i64 %offset
  ret ptr %resource
}

"#,
    );
    emit_copy_clear(&mut out, "session", 56);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_resource_runtime_is_manual_raw_and_has_no_c_provider() {
        assert_eq!(AUTH_RESOURCE_MANUAL_CALLS.len(), 37);
        let llvm = render_auth_resource_manual_runtime();
        for call in AUTH_RESOURCE_MANUAL_CALLS {
            assert!(
                llvm.contains(&format!("@{}", call.material)),
                "{}",
                call.material
            );
        }
        for forbidden in [
            "malloc",
            "free(",
            "memcpy",
            "printf",
            "libc",
            "msvcrt",
            " inbounds ",
        ] {
            assert!(
                !llvm.contains(forbidden),
                "unexpected provider/assumption: {forbidden}"
            );
        }
    }
}
