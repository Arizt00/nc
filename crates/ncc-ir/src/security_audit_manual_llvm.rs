pub(crate) struct SecurityAuditRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const SECURITY_AUDIT_MANUAL_CALLS: &[SecurityAuditRuntimeCall] = &[
    call(
        "nC.runtime.securityAudit.securityAuditStride",
        "__ncc_security_audit_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditWrite",
        "__ncc_security_audit_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditEvent",
        "__ncc_security_audit_event",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditSubject",
        "__ncc_security_audit_subject",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditResource",
        "__ncc_security_audit_resource",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditRights",
        "__ncc_security_audit_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditDecision",
        "__ncc_security_audit_decision",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditTick",
        "__ncc_security_audit_tick",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditDetail",
        "__ncc_security_audit_detail",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditAllows",
        "__ncc_security_audit_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditCopy",
        "__ncc_security_audit_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.securityAudit.securityAuditClear",
        "__ncc_security_audit_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> SecurityAuditRuntimeCall {
    SecurityAuditRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static SecurityAuditRuntimeCall> {
    SECURITY_AUDIT_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    SECURITY_AUDIT_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_security_audit_manual_runtime() -> &'static str {
    r#"
define internal ptr @__ncc_security_audit_record(ptr %table, i64 %index) {
  %offset = mul i64 %index, 56
  %record = getelementptr i8, ptr %table, i64 %offset
  ret ptr %record
}

define i64 @__ncc_security_audit_stride() { ret i64 56 }

define i1 @__ncc_security_audit_write(ptr %table, i64 %index, i64 %event, i64 %subject, i64 %resource, i64 %rights, i64 %decision, i64 %tick, i64 %detail) {
  %record = call ptr @__ncc_security_audit_record(ptr %table, i64 %index)
  %subject_slot = getelementptr i8, ptr %record, i64 8
  %resource_slot = getelementptr i8, ptr %record, i64 16
  %rights_slot = getelementptr i8, ptr %record, i64 24
  %decision_slot = getelementptr i8, ptr %record, i64 32
  %tick_slot = getelementptr i8, ptr %record, i64 40
  %detail_slot = getelementptr i8, ptr %record, i64 48
  store i64 %event, ptr %record, align 1
  store i64 %subject, ptr %subject_slot, align 1
  store i64 %resource, ptr %resource_slot, align 1
  store i64 %rights, ptr %rights_slot, align 1
  store i64 %decision, ptr %decision_slot, align 1
  store i64 %tick, ptr %tick_slot, align 1
  store i64 %detail, ptr %detail_slot, align 1
  ret i1 true
}

define i64 @__ncc_security_audit_event(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_security_audit_subject(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_security_audit_resource(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 16 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_security_audit_rights(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_security_audit_decision(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 32 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_security_audit_tick(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 40 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_security_audit_detail(ptr %table, i64 %index) { %r = call ptr @__ncc_security_audit_record(ptr %table, i64 %index) %s = getelementptr i8, ptr %r, i64 48 %v = load i64, ptr %s, align 1 ret i64 %v }

define i1 @__ncc_security_audit_allows(ptr %table, i64 %index, i64 %required) {
  %rights = call i64 @__ncc_security_audit_rights(ptr %table, i64 %index)
  %decision = call i64 @__ncc_security_audit_decision(ptr %table, i64 %index)
  %selected = and i64 %rights, %required
  %has = icmp eq i64 %selected, %required
  %accepted = icmp ne i64 %decision, 0
  %ok = and i1 %has, %accepted
  ret i1 %ok
}

define i1 @__ncc_security_audit_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {
  %destination_record = call ptr @__ncc_security_audit_record(ptr %destination, i64 %destination_index)
  %source_record = call ptr @__ncc_security_audit_record(ptr %source, i64 %source_index)
  br label %loop
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %index, 56
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

define i1 @__ncc_security_audit_clear(ptr %table, i64 %index) {
  %record = call ptr @__ncc_security_audit_record(ptr %table, i64 %index)
  br label %loop
loop:
  %offset = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %offset, 56
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
