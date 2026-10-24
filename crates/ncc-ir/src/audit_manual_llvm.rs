pub(crate) struct AuditRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const AUDIT_MANUAL_CALLS: &[AuditRuntimeCall] = &[
    call("nC.runtime.log.logStride", "__ncc_log_stride", &[], "i64"),
    call(
        "nC.runtime.log.logWrite",
        "__ncc_log_write",
        &["ptr", "i64", "i64", "i64", "ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.log.logLevel",
        "__ncc_log_level",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.log.logCategory",
        "__ncc_log_category",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.log.logMessage",
        "__ncc_log_message",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.log.logMessageBytes",
        "__ncc_log_message_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.log.logValue",
        "__ncc_log_value",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.log.logFlags",
        "__ncc_log_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.log.logCopy",
        "__ncc_log_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.log.logClear",
        "__ncc_log_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.profile.profileStride",
        "__ncc_profile_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileWrite",
        "__ncc_profile_write",
        &["ptr", "i64", "i64", "i64", "i64", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.profile.profileMode",
        "__ncc_profile_mode",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileFlags",
        "__ncc_profile_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileCpuFeatures",
        "__ncc_profile_cpu_features",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileOptimization",
        "__ncc_profile_optimization",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileTarget",
        "__ncc_profile_target",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.profile.profileTargetBytes",
        "__ncc_profile_target_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.profile.profileHas",
        "__ncc_profile_has",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.profile.profileRequire",
        "__ncc_profile_require",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.profile.profileCopy",
        "__ncc_profile_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.profile.profileClear",
        "__ncc_profile_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.status.statusStride",
        "__ncc_status_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.status.statusWrite",
        "__ncc_status_write",
        &["ptr", "i64", "i64", "i64", "ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.status.statusKind",
        "__ncc_status_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.status.statusCode",
        "__ncc_status_code",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.status.statusMessage",
        "__ncc_status_message",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.status.statusMessageBytes",
        "__ncc_status_message_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.status.statusDetail",
        "__ncc_status_detail",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.status.statusCopy",
        "__ncc_status_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.status.statusClear",
        "__ncc_status_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.trace.traceStride",
        "__ncc_trace_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceWrite",
        "__ncc_trace_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "ptr", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.trace.traceKind",
        "__ncc_trace_kind",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceId",
        "__ncc_trace_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceParent",
        "__ncc_trace_parent",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceTick",
        "__ncc_trace_tick",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.tracePayload",
        "__ncc_trace_payload",
        &["ptr", "i64"],
        "ptr",
    ),
    call(
        "nC.runtime.trace.tracePayloadBytes",
        "__ncc_trace_payload_bytes",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceValue",
        "__ncc_trace_value",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.trace.traceCopy",
        "__ncc_trace_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.trace.traceClear",
        "__ncc_trace_clear",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateNotNull",
        "__ncc_validate_not_null",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateRange",
        "__ncc_validate_range",
        &["i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateLength",
        "__ncc_validate_length",
        &["i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateEqual",
        "__ncc_validate_equal",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateAscii",
        "__ncc_validate_ascii",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateUtf8",
        "__ncc_validate_utf8",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateAligned",
        "__ncc_validate_aligned",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validatePowerOfTwo",
        "__ncc_validate_power_of_two",
        &["i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateZero",
        "__ncc_validate_zero",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.validate.validateMask",
        "__ncc_validate_mask",
        &["i64", "i64", "i64"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> AuditRuntimeCall {
    AuditRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(target: &str, arity: usize) -> Option<&'static AuditRuntimeCall> {
    AUDIT_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical == target && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    AUDIT_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical == target)
        .map(|call| call.material)
}

pub(crate) fn render_audit_manual_runtime() -> String {
    AUDIT_MANUAL_RUNTIME.to_string()
}

const AUDIT_MANUAL_RUNTIME: &str = r#"
define internal ptr @__ncc_audit_record(ptr %table, i64 %index, i64 %stride) {
entry:
  %offset = mul i64 %index, %stride
  %record = getelementptr i8, ptr %table, i64 %offset
  ret ptr %record
}

define internal i1 @__ncc_audit_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index, i64 %stride) {
entry:
  %d = call ptr @__ncc_audit_record(ptr %destination, i64 %destination_index, i64 %stride)
  %s = call ptr @__ncc_audit_record(ptr %source, i64 %source_index, i64 %stride)
  br label %copy
copy:
  %i = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp slt i64 %i, %stride
  br i1 %within, label %write, label %done
write:
  %sb = getelementptr i8, ptr %s, i64 %i
  %db = getelementptr i8, ptr %d, i64 %i
  %value = load i8, ptr %sb, align 1
  store i8 %value, ptr %db, align 1
  %next = add i64 %i, 1
  br label %copy
done:
  ret i1 true
}

define internal i1 @__ncc_audit_clear(ptr %table, i64 %index, i64 %stride) {
entry:
  %record = call ptr @__ncc_audit_record(ptr %table, i64 %index, i64 %stride)
  br label %clear
clear:
  %i = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp slt i64 %i, %stride
  br i1 %within, label %write, label %done
write:
  %slot = getelementptr i8, ptr %record, i64 %i
  store i8 0, ptr %slot, align 1
  %next = add i64 %i, 1
  br label %clear
done:
  ret i1 true
}

define i64 @__ncc_log_stride() { ret i64 48 }
define i1 @__ncc_log_write(ptr %t, i64 %i, i64 %level, i64 %category, ptr %message, i64 %bytes, i64 %value, i64 %flags) {
entry:
  %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48)
  %s1 = getelementptr i8, ptr %r, i64 8
  %s2 = getelementptr i8, ptr %r, i64 16
  %s3 = getelementptr i8, ptr %r, i64 24
  %s4 = getelementptr i8, ptr %r, i64 32
  %s5 = getelementptr i8, ptr %r, i64 40
  store i64 %level, ptr %r, align 1
  store i64 %category, ptr %s1, align 1
  store ptr %message, ptr %s2, align 1
  store i64 %bytes, ptr %s3, align 1
  store i64 %value, ptr %s4, align 1
  store i64 %flags, ptr %s5, align 1
  ret i1 true
}
define i64 @__ncc_log_level(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_log_category(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define ptr @__ncc_log_message(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 16 %v = load ptr, ptr %s, align 1 ret ptr %v }
define i64 @__ncc_log_message_bytes(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_log_value(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 32 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_log_flags(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 40 %v = load i64, ptr %s, align 1 ret i64 %v }
define i1 @__ncc_log_copy(ptr %d, i64 %di, ptr %s, i64 %si) { %v = call i1 @__ncc_audit_copy(ptr %d, i64 %di, ptr %s, i64 %si, i64 48) ret i1 %v }
define i1 @__ncc_log_clear(ptr %t, i64 %i) { %v = call i1 @__ncc_audit_clear(ptr %t, i64 %i, i64 48) ret i1 %v }

define i64 @__ncc_profile_stride() { ret i64 48 }
define i1 @__ncc_profile_write(ptr %t, i64 %i, i64 %mode, i64 %flags, i64 %cpu, i64 %optimization, ptr %target, i64 %target_bytes) {
entry:
  %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48)
  %s1 = getelementptr i8, ptr %r, i64 8
  %s2 = getelementptr i8, ptr %r, i64 16
  %s3 = getelementptr i8, ptr %r, i64 24
  %s4 = getelementptr i8, ptr %r, i64 32
  %s5 = getelementptr i8, ptr %r, i64 40
  store i64 %mode, ptr %r, align 1
  store i64 %flags, ptr %s1, align 1
  store i64 %cpu, ptr %s2, align 1
  store i64 %optimization, ptr %s3, align 1
  store ptr %target, ptr %s4, align 1
  store i64 %target_bytes, ptr %s5, align 1
  ret i1 true
}
define i64 @__ncc_profile_mode(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_profile_flags(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_profile_cpu_features(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 16 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_profile_optimization(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define ptr @__ncc_profile_target(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 32 %v = load ptr, ptr %s, align 1 ret ptr %v }
define i64 @__ncc_profile_target_bytes(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 48) %s = getelementptr i8, ptr %r, i64 40 %v = load i64, ptr %s, align 1 ret i64 %v }
define i1 @__ncc_profile_has(ptr %t, i64 %i, i64 %mask) { %flags = call i64 @__ncc_profile_flags(ptr %t, i64 %i) %selected = and i64 %flags, %mask %ok = icmp eq i64 %selected, %mask ret i1 %ok }
define i1 @__ncc_profile_require(ptr %t, i64 %i, i64 %mask) {
entry:
  %ok = call i1 @__ncc_profile_has(ptr %t, i64 %i, i64 %mask)
  br i1 %ok, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}
define i1 @__ncc_profile_copy(ptr %d, i64 %di, ptr %s, i64 %si) { %v = call i1 @__ncc_audit_copy(ptr %d, i64 %di, ptr %s, i64 %si, i64 48) ret i1 %v }
define i1 @__ncc_profile_clear(ptr %t, i64 %i) { %v = call i1 @__ncc_audit_clear(ptr %t, i64 %i, i64 48) ret i1 %v }

define i64 @__ncc_status_stride() { ret i64 40 }
define i1 @__ncc_status_write(ptr %t, i64 %i, i64 %kind, i64 %code, ptr %message, i64 %bytes, i64 %detail) {
entry:
  %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40)
  %s1 = getelementptr i8, ptr %r, i64 8
  %s2 = getelementptr i8, ptr %r, i64 16
  %s3 = getelementptr i8, ptr %r, i64 24
  %s4 = getelementptr i8, ptr %r, i64 32
  store i64 %kind, ptr %r, align 1
  store i64 %code, ptr %s1, align 1
  store ptr %message, ptr %s2, align 1
  store i64 %bytes, ptr %s3, align 1
  store i64 %detail, ptr %s4, align 1
  ret i1 true
}
define i64 @__ncc_status_kind(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_status_code(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define ptr @__ncc_status_message(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40) %s = getelementptr i8, ptr %r, i64 16 %v = load ptr, ptr %s, align 1 ret ptr %v }
define i64 @__ncc_status_message_bytes(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_status_detail(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 40) %s = getelementptr i8, ptr %r, i64 32 %v = load i64, ptr %s, align 1 ret i64 %v }
define i1 @__ncc_status_copy(ptr %d, i64 %di, ptr %s, i64 %si) { %v = call i1 @__ncc_audit_copy(ptr %d, i64 %di, ptr %s, i64 %si, i64 40) ret i1 %v }
define i1 @__ncc_status_clear(ptr %t, i64 %i) { %v = call i1 @__ncc_audit_clear(ptr %t, i64 %i, i64 40) ret i1 %v }

define i64 @__ncc_trace_stride() { ret i64 56 }
define i1 @__ncc_trace_write(ptr %t, i64 %i, i64 %kind, i64 %id, i64 %parent, i64 %tick, ptr %payload, i64 %bytes, i64 %value) {
entry:
  %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56)
  %s1 = getelementptr i8, ptr %r, i64 8
  %s2 = getelementptr i8, ptr %r, i64 16
  %s3 = getelementptr i8, ptr %r, i64 24
  %s4 = getelementptr i8, ptr %r, i64 32
  %s5 = getelementptr i8, ptr %r, i64 40
  %s6 = getelementptr i8, ptr %r, i64 48
  store i64 %kind, ptr %r, align 1
  store i64 %id, ptr %s1, align 1
  store i64 %parent, ptr %s2, align 1
  store i64 %tick, ptr %s3, align 1
  store ptr %payload, ptr %s4, align 1
  store i64 %bytes, ptr %s5, align 1
  store i64 %value, ptr %s6, align 1
  ret i1 true
}
define i64 @__ncc_trace_kind(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %v = load i64, ptr %r, align 1 ret i64 %v }
define i64 @__ncc_trace_id(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_trace_parent(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 16 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_trace_tick(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }
define ptr @__ncc_trace_payload(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 32 %v = load ptr, ptr %s, align 1 ret ptr %v }
define i64 @__ncc_trace_payload_bytes(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 40 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_trace_value(ptr %t, i64 %i) { %r = call ptr @__ncc_audit_record(ptr %t, i64 %i, i64 56) %s = getelementptr i8, ptr %r, i64 48 %v = load i64, ptr %s, align 1 ret i64 %v }
define i1 @__ncc_trace_copy(ptr %d, i64 %di, ptr %s, i64 %si) { %v = call i1 @__ncc_audit_copy(ptr %d, i64 %di, ptr %s, i64 %si, i64 56) ret i1 %v }
define i1 @__ncc_trace_clear(ptr %t, i64 %i) { %v = call i1 @__ncc_audit_clear(ptr %t, i64 %i, i64 56) ret i1 %v }

define i1 @__ncc_validate_not_null(ptr %value) { %ok = icmp ne ptr %value, null ret i1 %ok }
define i1 @__ncc_validate_range(i64 %value, i64 %minimum, i64 %maximum) { %lo = icmp sge i64 %value, %minimum %hi = icmp sle i64 %value, %maximum %ok = and i1 %lo, %hi ret i1 %ok }
define i1 @__ncc_validate_length(i64 %length, i64 %minimum, i64 %maximum) { %ok = call i1 @__ncc_validate_range(i64 %length, i64 %minimum, i64 %maximum) ret i1 %ok }
define i1 @__ncc_validate_equal(ptr %left, i64 %left_bytes, ptr %right, i64 %right_bytes) {
entry:
  %same = icmp eq i64 %left_bytes, %right_bytes
  br i1 %same, label %scan, label %different
scan:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %within = icmp slt i64 %i, %left_bytes
  br i1 %within, label %compare, label %equal
compare:
  %ls = getelementptr i8, ptr %left, i64 %i
  %rs = getelementptr i8, ptr %right, i64 %i
  %lv = load i8, ptr %ls, align 1
  %rv = load i8, ptr %rs, align 1
  %match = icmp eq i8 %lv, %rv
  br i1 %match, label %advance, label %different
advance:
  %next = add i64 %i, 1
  br label %scan
equal:
  ret i1 true
different:
  ret i1 false
}
define i1 @__ncc_validate_ascii(ptr %data, i64 %bytes) {
entry:
  br label %scan
scan:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %within = icmp slt i64 %i, %bytes
  br i1 %within, label %check, label %valid
check:
  %s = getelementptr i8, ptr %data, i64 %i
  %v = load i8, ptr %s, align 1
  %ascii = icmp sge i8 %v, 0
  br i1 %ascii, label %advance, label %invalid
advance:
  %next = add i64 %i, 1
  br label %scan
valid:
  ret i1 true
invalid:
  ret i1 false
}
define internal i1 @__ncc_utf8_cont(ptr %data, i64 %index) {
entry:
  %s = getelementptr i8, ptr %data, i64 %index
  %b = load i8, ptr %s, align 1
  %v = zext i8 %b to i64
  %lo = icmp sge i64 %v, 128
  %hi = icmp sle i64 %v, 191
  %ok = and i1 %lo, %hi
  ret i1 %ok
}
define i1 @__ncc_validate_utf8(ptr %data, i64 %bytes) {
entry:
  br label %scan
scan:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %within = icmp slt i64 %i, %bytes
  br i1 %within, label %classify, label %valid
classify:
  %slot = getelementptr i8, ptr %data, i64 %i
  %raw = load i8, ptr %slot, align 1
  %b0 = zext i8 %raw to i64
  %ascii = icmp ult i64 %b0, 128
  br i1 %ascii, label %one, label %two_class
one:
  %n1 = add i64 %i, 1
  br label %advance
two_class:
  %two_lo = icmp sge i64 %b0, 194
  %two_hi = icmp sle i64 %b0, 223
  %two = and i1 %two_lo, %two_hi
  br i1 %two, label %two_bounds, label %three_class
two_bounds:
  %i1 = add i64 %i, 1
  %enough2 = icmp slt i64 %i1, %bytes
  br i1 %enough2, label %two_bytes, label %invalid
two_bytes:
  %c1_2 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i1)
  br i1 %c1_2, label %two_valid, label %invalid
two_valid:
  %n2 = add i64 %i, 2
  br label %advance
three_class:
  %three_lo = icmp sge i64 %b0, 224
  %three_hi = icmp sle i64 %b0, 239
  %three = and i1 %three_lo, %three_hi
  br i1 %three, label %three_bounds, label %four_class
three_bounds:
  %i2_3 = add i64 %i, 2
  %enough3 = icmp slt i64 %i2_3, %bytes
  br i1 %enough3, label %three_bytes, label %invalid
three_bytes:
  %i1_3 = add i64 %i, 1
  %s1_3 = getelementptr i8, ptr %data, i64 %i1_3
  %r1_3 = load i8, ptr %s1_3, align 1
  %b1_3 = zext i8 %r1_3 to i64
  %c2_3 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i2_3)
  %is_e0 = icmp eq i64 %b0, 224
  %e0_lo = icmp sge i64 %b1_3, 160
  %e0_hi = icmp sle i64 %b1_3, 191
  %e0_ok = and i1 %e0_lo, %e0_hi
  %is_ed = icmp eq i64 %b0, 237
  %ed_lo = icmp sge i64 %b1_3, 128
  %ed_hi = icmp sle i64 %b1_3, 159
  %ed_ok = and i1 %ed_lo, %ed_hi
  %other_a = icmp sge i64 %b0, 225
  %other_b = icmp sle i64 %b0, 236
  %other_c = and i1 %other_a, %other_b
  %other_d = icmp sge i64 %b0, 238
  %other = or i1 %other_c, %other_d
  %cont1_3 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i1_3)
  %e0_case = and i1 %is_e0, %e0_ok
  %ed_case = and i1 %is_ed, %ed_ok
  %other_case = and i1 %other, %cont1_3
  %first_pair = or i1 %e0_case, %ed_case
  %first_ok = or i1 %first_pair, %other_case
  %three_ok = and i1 %first_ok, %c2_3
  br i1 %three_ok, label %three_valid, label %invalid
three_valid:
  %n3 = add i64 %i, 3
  br label %advance
four_class:
  %four_lo = icmp sge i64 %b0, 240
  %four_hi = icmp sle i64 %b0, 244
  %four = and i1 %four_lo, %four_hi
  br i1 %four, label %four_bounds, label %invalid
four_bounds:
  %i3_4 = add i64 %i, 3
  %enough4 = icmp slt i64 %i3_4, %bytes
  br i1 %enough4, label %four_bytes, label %invalid
four_bytes:
  %i1_4 = add i64 %i, 1
  %i2_4 = add i64 %i, 2
  %s1_4 = getelementptr i8, ptr %data, i64 %i1_4
  %r1_4 = load i8, ptr %s1_4, align 1
  %b1_4 = zext i8 %r1_4 to i64
  %is_f0 = icmp eq i64 %b0, 240
  %f0_lo = icmp sge i64 %b1_4, 144
  %f0_hi = icmp sle i64 %b1_4, 191
  %f0_ok = and i1 %f0_lo, %f0_hi
  %is_f4 = icmp eq i64 %b0, 244
  %f4_lo = icmp sge i64 %b1_4, 128
  %f4_hi = icmp sle i64 %b1_4, 143
  %f4_ok = and i1 %f4_lo, %f4_hi
  %mid_lo = icmp sge i64 %b0, 241
  %mid_hi = icmp sle i64 %b0, 243
  %mid = and i1 %mid_lo, %mid_hi
  %cont1_4 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i1_4)
  %f0_case = and i1 %is_f0, %f0_ok
  %f4_case = and i1 %is_f4, %f4_ok
  %mid_case = and i1 %mid, %cont1_4
  %first_pair4 = or i1 %f0_case, %f4_case
  %first_ok4 = or i1 %first_pair4, %mid_case
  %c2_4 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i2_4)
  %c3_4 = call i1 @__ncc_utf8_cont(ptr %data, i64 %i3_4)
  %rest4 = and i1 %c2_4, %c3_4
  %four_ok = and i1 %first_ok4, %rest4
  br i1 %four_ok, label %four_valid, label %invalid
four_valid:
  %n4 = add i64 %i, 4
  br label %advance
advance:
  %next = phi i64 [ %n1, %one ], [ %n2, %two_valid ], [ %n3, %three_valid ], [ %n4, %four_valid ]
  br label %scan
valid:
  ret i1 true
invalid:
  ret i1 false
}
define i1 @__ncc_validate_aligned(ptr %value, i64 %alignment) {
entry:
  %positive = icmp sgt i64 %alignment, 0
  br i1 %positive, label %check, label %invalid
check:
  %address = ptrtoint ptr %value to i64
  %remainder = urem i64 %address, %alignment
  %ok = icmp eq i64 %remainder, 0
  ret i1 %ok
invalid:
  ret i1 false
}
define i1 @__ncc_validate_power_of_two(i64 %value) { %positive = icmp sgt i64 %value, 0 %minus = sub i64 %value, 1 %masked = and i64 %value, %minus %single = icmp eq i64 %masked, 0 %ok = and i1 %positive, %single ret i1 %ok }
define i1 @__ncc_validate_zero(ptr %data, i64 %bytes) {
entry:
  br label %scan
scan:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %within = icmp slt i64 %i, %bytes
  br i1 %within, label %check, label %valid
check:
  %s = getelementptr i8, ptr %data, i64 %i
  %v = load i8, ptr %s, align 1
  %zero = icmp eq i8 %v, 0
  br i1 %zero, label %advance, label %invalid
advance:
  %next = add i64 %i, 1
  br label %scan
valid:
  ret i1 true
invalid:
  ret i1 false
}
define i1 @__ncc_validate_mask(i64 %value, i64 %required, i64 %forbidden) { %selected = and i64 %value, %required %has = icmp eq i64 %selected, %required %blocked = and i64 %value, %forbidden %clean = icmp eq i64 %blocked, 0 %ok = and i1 %has, %clean ret i1 %ok }
"#;

#[cfg(test)]
mod tests {
    use super::{material_symbol, render_audit_manual_runtime, AUDIT_MANUAL_CALLS};

    #[test]
    fn all_manual_audit_calls_have_material_symbols_and_no_c_provider() {
        assert_eq!(AUDIT_MANUAL_CALLS.len(), 52);
        let ir = render_audit_manual_runtime();
        for call in AUDIT_MANUAL_CALLS {
            assert_eq!(material_symbol(call.canonical), Some(call.material));
            assert!(
                ir.contains(&format!("@{}(", call.material)),
                "missing {}",
                call.material
            );
        }
        for forbidden in [
            "@malloc(", "@free(", "@memcpy(", "@printf(", "libc", "msvcrt",
        ] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
