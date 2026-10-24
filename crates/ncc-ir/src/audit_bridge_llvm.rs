pub(crate) fn render_audit_bridge_runtime() -> String {
    AUDIT_BRIDGE_RUNTIME.to_string()
}

const AUDIT_BRIDGE_RUNTIME: &str = r#"
define i64 @__ncc_bridge_stride() {
entry:
  ret i64 32
}

define i1 @__ncc_bridge_write(ptr %table, i64 %index, i64 %key, ptr %target, i64 %mode, i64 %authority) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %target_slot = getelementptr i8, ptr %record, i64 8
  %mode_slot = getelementptr i8, ptr %record, i64 16
  %authority_slot = getelementptr i8, ptr %record, i64 24
  store i64 %key, ptr %record, align 1
  store ptr %target, ptr %target_slot, align 1
  store i64 %mode, ptr %mode_slot, align 1
  store i64 %authority, ptr %authority_slot, align 1
  ret i1 true
}

define i64 @__ncc_bridge_find(ptr %table, i64 %count, i64 %key) {
entry:
  br label %scan
scan:
  %index = phi i64 [ 0, %entry ], [ %next_index, %next ]
  %within = icmp slt i64 %index, %count
  br i1 %within, label %probe, label %missing
probe:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %stored_key = load i64, ptr %record, align 1
  %matched = icmp eq i64 %stored_key, %key
  br i1 %matched, label %found, label %next
next:
  %next_index = add i64 %index, 1
  br label %scan
found:
  ret i64 %index
missing:
  ret i64 -1
}

define i1 @__ncc_bridge_active(ptr %table, i64 %count, i64 %key) {
entry:
  %index = call i64 @__ncc_bridge_find(ptr %table, i64 %count, i64 %key)
  %present = icmp sge i64 %index, 0
  ret i1 %present
}

define i64 @__ncc_bridge_require(ptr %table, i64 %count, i64 %key) {
entry:
  %index = call i64 @__ncc_bridge_find(ptr %table, i64 %count, i64 %key)
  %present = icmp sge i64 %index, 0
  br i1 %present, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i64 %index
}

define ptr @__ncc_bridge_target(ptr %table, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %target_slot = getelementptr i8, ptr %record, i64 8
  %target = load ptr, ptr %target_slot, align 1
  ret ptr %target
}

define i64 @__ncc_bridge_mode(ptr %table, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %mode_slot = getelementptr i8, ptr %record, i64 16
  %mode = load i64, ptr %mode_slot, align 1
  ret i64 %mode
}

define i64 @__ncc_bridge_authority(ptr %table, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %authority_slot = getelementptr i8, ptr %record, i64 24
  %authority = load i64, ptr %authority_slot, align 1
  ret i64 %authority
}

define i1 @__ncc_bridge_erase(ptr %table, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %table, i64 %offset
  %target_slot = getelementptr i8, ptr %record, i64 8
  %mode_slot = getelementptr i8, ptr %record, i64 16
  %authority_slot = getelementptr i8, ptr %record, i64 24
  store i64 0, ptr %record, align 1
  store ptr null, ptr %target_slot, align 1
  store i64 0, ptr %mode_slot, align 1
  store i64 0, ptr %authority_slot, align 1
  ret i1 true
}
"#;

#[cfg(test)]
mod tests {
    use super::render_audit_bridge_runtime;

    #[test]
    fn bridge_runtime_is_caller_owned_raw_memory_without_c_provider() {
        let ir = render_audit_bridge_runtime();
        for material in [
            "__ncc_bridge_stride",
            "__ncc_bridge_write",
            "__ncc_bridge_find",
            "__ncc_bridge_active",
            "__ncc_bridge_require",
            "__ncc_bridge_target",
            "__ncc_bridge_mode",
            "__ncc_bridge_authority",
            "__ncc_bridge_erase",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        assert!(ir.contains("ret i64 32"));
        assert!(ir.contains("call void @llvm.trap()"));
        for forbidden in ["@malloc(", "@free(", "@printf(", "libc", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
