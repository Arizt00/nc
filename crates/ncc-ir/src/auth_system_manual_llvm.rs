use std::fmt::Write as _;

pub(crate) struct AuthSystemRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const AUTH_SERVICE_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authService.authServiceStride",
        "__ncc_auth_service_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceWrite",
        "__ncc_auth_service_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authService.authServiceKey",
        "__ncc_auth_service_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceHandle",
        "__ncc_auth_service_handle",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceProcess",
        "__ncc_auth_service_process",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceRights",
        "__ncc_auth_service_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceFlags",
        "__ncc_auth_service_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceOwner",
        "__ncc_auth_service_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceGeneration",
        "__ncc_auth_service_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceState",
        "__ncc_auth_service_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authService.authServiceAllows",
        "__ncc_auth_service_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authService.authServiceCopy",
        "__ncc_auth_service_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authService.authServiceClear",
        "__ncc_auth_service_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) const AUTH_SIGNAL_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authSignal.authSignalStride",
        "__ncc_auth_signal_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalWrite",
        "__ncc_auth_signal_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authSignal.authSignalKey",
        "__ncc_auth_signal_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalTarget",
        "__ncc_auth_signal_target",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalCode",
        "__ncc_auth_signal_code",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalRights",
        "__ncc_auth_signal_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalFlags",
        "__ncc_auth_signal_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalOwner",
        "__ncc_auth_signal_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalGeneration",
        "__ncc_auth_signal_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalState",
        "__ncc_auth_signal_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSignal.authSignalAllows",
        "__ncc_auth_signal_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSignal.authSignalCopy",
        "__ncc_auth_signal_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSignal.authSignalClear",
        "__ncc_auth_signal_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) const AUTH_SYSTEM_SESSION_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authSystemSession.authSystemSessionStride",
        "__ncc_auth_system_session_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionWrite",
        "__ncc_auth_system_session_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionKey",
        "__ncc_auth_system_session_key",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionPrincipal",
        "__ncc_auth_system_session_principal",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionHandle",
        "__ncc_auth_system_session_handle",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionRights",
        "__ncc_auth_system_session_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionFlags",
        "__ncc_auth_system_session_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionOwner",
        "__ncc_auth_system_session_owner",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionGeneration",
        "__ncc_auth_system_session_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionState",
        "__ncc_auth_system_session_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionAllows",
        "__ncc_auth_system_session_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionCopy",
        "__ncc_auth_system_session_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSystemSession.authSystemSessionClear",
        "__ncc_auth_system_session_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) const AUTH_POLICY_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authPolicy.authPolicyStride",
        "__ncc_auth_policy_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyWrite",
        "__ncc_auth_policy_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyId",
        "__ncc_auth_policy_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicySubject",
        "__ncc_auth_policy_subject",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyResource",
        "__ncc_auth_policy_resource",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyRights",
        "__ncc_auth_policy_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyDenied",
        "__ncc_auth_policy_denied",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyFlags",
        "__ncc_auth_policy_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyPriority",
        "__ncc_auth_policy_priority",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyState",
        "__ncc_auth_policy_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyAllows",
        "__ncc_auth_policy_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyCopy",
        "__ncc_auth_policy_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authPolicy.authPolicyClear",
        "__ncc_auth_policy_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) const AUTH_CAPABILITY_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authCapability.authCapabilityStride",
        "__ncc_auth_capability_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityWrite",
        "__ncc_auth_capability_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityId",
        "__ncc_auth_capability_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilitySubject",
        "__ncc_auth_capability_subject",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityResource",
        "__ncc_auth_capability_resource",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityRights",
        "__ncc_auth_capability_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityScope",
        "__ncc_auth_capability_scope",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityFlags",
        "__ncc_auth_capability_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityGeneration",
        "__ncc_auth_capability_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityState",
        "__ncc_auth_capability_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityAllows",
        "__ncc_auth_capability_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityCopy",
        "__ncc_auth_capability_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authCapability.authCapabilityClear",
        "__ncc_auth_capability_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

pub(crate) const AUTH_SANDBOX_MANUAL_CALLS: &[AuthSystemRuntimeCall] = &[
    call(
        "nC.runtime.authSandbox.authSandboxStride",
        "__ncc_auth_sandbox_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxWrite",
        "__ncc_auth_sandbox_write",
        &[
            "ptr", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64", "i64",
        ],
        "i1",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxId",
        "__ncc_auth_sandbox_id",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxPrincipal",
        "__ncc_auth_sandbox_principal",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxRoot",
        "__ncc_auth_sandbox_root",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxRights",
        "__ncc_auth_sandbox_rights",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxRestrictions",
        "__ncc_auth_sandbox_restrictions",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxFlags",
        "__ncc_auth_sandbox_flags",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxGeneration",
        "__ncc_auth_sandbox_generation",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxState",
        "__ncc_auth_sandbox_state",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxAllows",
        "__ncc_auth_sandbox_allows",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxCopy",
        "__ncc_auth_sandbox_copy",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.authSandbox.authSandboxClear",
        "__ncc_auth_sandbox_clear",
        &["ptr", "i64"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> AuthSystemRuntimeCall {
    AuthSystemRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

fn families() -> [&'static [AuthSystemRuntimeCall]; 6] {
    [
        AUTH_SERVICE_MANUAL_CALLS,
        AUTH_SIGNAL_MANUAL_CALLS,
        AUTH_SYSTEM_SESSION_MANUAL_CALLS,
        AUTH_POLICY_MANUAL_CALLS,
        AUTH_CAPABILITY_MANUAL_CALLS,
        AUTH_SANDBOX_MANUAL_CALLS,
    ]
}

pub(crate) fn call_signature(target: &str, arity: usize) -> Option<&'static AuthSystemRuntimeCall> {
    families()
        .into_iter()
        .flatten()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    families()
        .into_iter()
        .flatten()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

fn render_family(prefix: &str, fields: &[&str], blocked_field: Option<&str>) -> String {
    let mut out = String::new();
    writeln!(out, "define internal ptr @__ncc_{prefix}_record(ptr %table, i64 %index) {{\nentry:\n  %offset = mul i64 %index, 64\n  %record = getelementptr i8, ptr %table, i64 %offset\n  ret ptr %record\n}}\n").unwrap();
    writeln!(
        out,
        "define i64 @__ncc_{prefix}_stride() {{ ret i64 64 }}\n"
    )
    .unwrap();

    let params = fields
        .iter()
        .map(|field| format!("i64 %{field}"))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "define i1 @__ncc_{prefix}_write(ptr %table, i64 %index, {params}) {{\nentry:\n  %record = call ptr @__ncc_{prefix}_record(ptr %table, i64 %index)").unwrap();
    for (index, field) in fields.iter().enumerate() {
        writeln!(
            out,
            "  %slot_{field} = getelementptr i8, ptr %record, i64 {}",
            index * 8
        )
        .unwrap();
        writeln!(out, "  store i64 %{field}, ptr %slot_{field}, align 1").unwrap();
    }
    out.push_str("  ret i1 true\n}\n\n");

    for (index, field) in fields.iter().enumerate() {
        writeln!(out, "define i64 @__ncc_{prefix}_{field}(ptr %table, i64 %index) {{\nentry:\n  %record = call ptr @__ncc_{prefix}_record(ptr %table, i64 %index)\n  %slot = getelementptr i8, ptr %record, i64 {}\n  %value = load i64, ptr %slot, align 1\n  ret i64 %value\n}}\n", index * 8).unwrap();
    }

    writeln!(out, "define i1 @__ncc_{prefix}_allows(ptr %table, i64 %index, i64 %required) {{\nentry:\n  %rights = call i64 @__ncc_{prefix}_rights(ptr %table, i64 %index)\n  %state = call i64 @__ncc_{prefix}_state(ptr %table, i64 %index)\n  %selected = and i64 %rights, %required\n  %has = icmp eq i64 %selected, %required\n  %active = icmp ne i64 %state, 0").unwrap();
    if let Some(blocked_field) = blocked_field {
        writeln!(out, "  %blocked = call i64 @__ncc_{prefix}_{blocked_field}(ptr %table, i64 %index)\n  %blocked_bits = and i64 %blocked, %required\n  %unblocked = icmp eq i64 %blocked_bits, 0\n  %granted = and i1 %has, %unblocked\n  %ok = and i1 %granted, %active").unwrap();
    } else {
        out.push_str("\n  %ok = and i1 %has, %active\n");
    }
    out.push_str("  ret i1 %ok\n}\n\n");

    writeln!(out, "define i1 @__ncc_{prefix}_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {{\nentry:\n  %destination_record = call ptr @__ncc_{prefix}_record(ptr %destination, i64 %destination_index)\n  %source_record = call ptr @__ncc_{prefix}_record(ptr %source, i64 %source_index)\n  br label %loop\nloop:\n  %index = phi i64 [ 0, %entry ], [ %next, %write ]\n  %done = icmp uge i64 %index, 64\n  br i1 %done, label %exit, label %write\nwrite:\n  %source_byte = getelementptr i8, ptr %source_record, i64 %index\n  %destination_byte = getelementptr i8, ptr %destination_record, i64 %index\n  %value = load i8, ptr %source_byte, align 1\n  store i8 %value, ptr %destination_byte, align 1\n  %next = add i64 %index, 1\n  br label %loop\nexit:\n  ret i1 true\n}}\n").unwrap();

    writeln!(out, "define i1 @__ncc_{prefix}_clear(ptr %table, i64 %index) {{\nentry:\n  %record = call ptr @__ncc_{prefix}_record(ptr %table, i64 %index)\n  br label %loop\nloop:\n  %offset = phi i64 [ 0, %entry ], [ %next, %write ]\n  %done = icmp uge i64 %offset, 64\n  br i1 %done, label %exit, label %write\nwrite:\n  %byte = getelementptr i8, ptr %record, i64 %offset\n  store i8 0, ptr %byte, align 1\n  %next = add i64 %offset, 1\n  br label %loop\nexit:\n  ret i1 true\n}}\n").unwrap();
    out
}

pub(crate) fn render_auth_service_manual_runtime() -> String {
    render_family(
        "auth_service",
        &[
            "key",
            "handle",
            "process",
            "rights",
            "flags",
            "owner",
            "generation",
            "state",
        ],
        None,
    )
}

pub(crate) fn render_auth_signal_manual_runtime() -> String {
    render_family(
        "auth_signal",
        &[
            "key",
            "target",
            "code",
            "rights",
            "flags",
            "owner",
            "generation",
            "state",
        ],
        None,
    )
}

pub(crate) fn render_auth_system_session_manual_runtime() -> String {
    render_family(
        "auth_system_session",
        &[
            "key",
            "principal",
            "handle",
            "rights",
            "flags",
            "owner",
            "generation",
            "state",
        ],
        None,
    )
}

pub(crate) fn render_auth_policy_manual_runtime() -> String {
    render_family(
        "auth_policy",
        &[
            "id", "subject", "resource", "rights", "denied", "flags", "priority", "state",
        ],
        Some("denied"),
    )
}

pub(crate) fn render_auth_capability_manual_runtime() -> String {
    render_family(
        "auth_capability",
        &[
            "id",
            "subject",
            "resource",
            "rights",
            "scope",
            "flags",
            "generation",
            "state",
        ],
        None,
    )
}

pub(crate) fn render_auth_sandbox_manual_runtime() -> String {
    render_family(
        "auth_sandbox",
        &[
            "id",
            "principal",
            "root",
            "rights",
            "restrictions",
            "flags",
            "generation",
            "state",
        ],
        Some("restrictions"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_authority_runtimes_are_manual_raw_and_have_no_c_provider() {
        for llvm in [
            render_auth_service_manual_runtime(),
            render_auth_signal_manual_runtime(),
            render_auth_system_session_manual_runtime(),
            render_auth_policy_manual_runtime(),
            render_auth_capability_manual_runtime(),
            render_auth_sandbox_manual_runtime(),
        ] {
            assert!(llvm.contains("mul i64 %index, 64"));
            assert!(llvm.contains("_allows"));
            assert!(!llvm.contains("ncc_rt_"));
            assert!(!llvm.contains("malloc"));
            assert!(!llvm.contains("getelementptr inbounds"));
        }
    }
}
