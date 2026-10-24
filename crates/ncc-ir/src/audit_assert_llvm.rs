pub(crate) fn render_audit_assert_runtime() -> String {
    AUDIT_ASSERT_RUNTIME.to_string()
}

const AUDIT_ASSERT_RUNTIME: &str = r#"
define i1 @__ncc_assert_check(i1 %condition) {
entry:
  br i1 %condition, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_message(i1 %condition, ptr %message, i64 %message_bytes) {
entry:
  br i1 %condition, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_eq_int(i64 %left, i64 %right) {
entry:
  %equal = icmp eq i64 %left, %right
  br i1 %equal, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_eq_addr(ptr %left, ptr %right) {
entry:
  %equal = icmp eq ptr %left, %right
  br i1 %equal, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_not_null(ptr %value) {
entry:
  %present = icmp ne ptr %value, null
  br i1 %present, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_range(i64 %value, i64 %minimum, i64 %maximum) {
entry:
  %above = icmp sge i64 %value, %minimum
  %below = icmp sle i64 %value, %maximum
  %inside = and i1 %above, %below
  br i1 %inside, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}

define i1 @__ncc_assert_fail() {
entry:
  call void @llvm.trap()
  unreachable
}

define i1 @__ncc_assert_unreachable() {
entry:
  call void @llvm.trap()
  unreachable
}
"#;

#[cfg(test)]
mod tests {
    use super::render_audit_assert_runtime;

    #[test]
    fn assertion_runtime_is_portable_raw_and_has_no_c_provider() {
        let ir = render_audit_assert_runtime();
        for material in [
            "__ncc_assert_check",
            "__ncc_assert_message",
            "__ncc_assert_eq_int",
            "__ncc_assert_eq_addr",
            "__ncc_assert_not_null",
            "__ncc_assert_range",
            "__ncc_assert_fail",
            "__ncc_assert_unreachable",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        assert_eq!(ir.matches("call void @llvm.trap()").count(), 8);
        for forbidden in ["@abort(", "@printf(", "@malloc(", "libc", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
