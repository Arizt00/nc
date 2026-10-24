pub(crate) fn render_audit_diagnostic_runtime() -> String {
    AUDIT_DIAGNOSTIC_RUNTIME.to_string()
}

const AUDIT_DIAGNOSTIC_RUNTIME: &str = r#"
define i64 @__ncc_diag_stride() {
entry:
  ret i64 64
}

define i1 @__ncc_diag_write(ptr %table, i64 %index, i64 %severity, i64 %code, ptr %message, i64 %message_bytes, ptr %source, i64 %line, i64 %column, i64 %flags) {
entry:
  %offset = mul i64 %index, 64
  %record = getelementptr i8, ptr %table, i64 %offset
  %code_slot = getelementptr i8, ptr %record, i64 8
  %message_slot = getelementptr i8, ptr %record, i64 16
  %message_bytes_slot = getelementptr i8, ptr %record, i64 24
  %source_slot = getelementptr i8, ptr %record, i64 32
  %line_slot = getelementptr i8, ptr %record, i64 40
  %column_slot = getelementptr i8, ptr %record, i64 48
  %flags_slot = getelementptr i8, ptr %record, i64 56
  store i64 %severity, ptr %record, align 1
  store i64 %code, ptr %code_slot, align 1
  store ptr %message, ptr %message_slot, align 1
  store i64 %message_bytes, ptr %message_bytes_slot, align 1
  store ptr %source, ptr %source_slot, align 1
  store i64 %line, ptr %line_slot, align 1
  store i64 %column, ptr %column_slot, align 1
  store i64 %flags, ptr %flags_slot, align 1
  ret i1 true
}

define i64 @__ncc_diag_severity(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %v = load i64, ptr %r, align 1
  ret i64 %v
}

define i64 @__ncc_diag_code(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 8
  %v = load i64, ptr %s, align 1
  ret i64 %v
}

define ptr @__ncc_diag_message(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 16
  %v = load ptr, ptr %s, align 1
  ret ptr %v
}

define i64 @__ncc_diag_message_bytes(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 24
  %v = load i64, ptr %s, align 1
  ret i64 %v
}

define ptr @__ncc_diag_source(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 32
  %v = load ptr, ptr %s, align 1
  ret ptr %v
}

define i64 @__ncc_diag_line(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 40
  %v = load i64, ptr %s, align 1
  ret i64 %v
}

define i64 @__ncc_diag_column(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 48
  %v = load i64, ptr %s, align 1
  ret i64 %v
}

define i64 @__ncc_diag_flags(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  %s = getelementptr i8, ptr %r, i64 56
  %v = load i64, ptr %s, align 1
  ret i64 %v
}

define i1 @__ncc_diag_copy(ptr %destination, i64 %destination_index, ptr %source, i64 %source_index) {
entry:
  %do = mul i64 %destination_index, 64
  %so = mul i64 %source_index, 64
  %d = getelementptr i8, ptr %destination, i64 %do
  %s = getelementptr i8, ptr %source, i64 %so
  br label %copy
copy:
  %i = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp slt i64 %i, 64
  br i1 %within, label %write, label %done
write:
  %sb = getelementptr i8, ptr %s, i64 %i
  %db = getelementptr i8, ptr %d, i64 %i
  %v = load i8, ptr %sb, align 1
  store i8 %v, ptr %db, align 1
  %next = add i64 %i, 1
  br label %copy
done:
  ret i1 true
}

define i1 @__ncc_diag_clear(ptr %table, i64 %index) {
entry:
  %o = mul i64 %index, 64
  %r = getelementptr i8, ptr %table, i64 %o
  br label %clear
clear:
  %i = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp slt i64 %i, 64
  br i1 %within, label %write, label %done
write:
  %slot = getelementptr i8, ptr %r, i64 %i
  store i8 0, ptr %slot, align 1
  %next = add i64 %i, 1
  br label %clear
done:
  ret i1 true
}
"#;

#[cfg(test)]
mod tests {
    use super::render_audit_diagnostic_runtime;

    #[test]
    fn diagnostic_runtime_is_raw_records_without_c_provider() {
        let ir = render_audit_diagnostic_runtime();
        for material in [
            "__ncc_diag_stride",
            "__ncc_diag_write",
            "__ncc_diag_severity",
            "__ncc_diag_code",
            "__ncc_diag_message",
            "__ncc_diag_message_bytes",
            "__ncc_diag_source",
            "__ncc_diag_line",
            "__ncc_diag_column",
            "__ncc_diag_flags",
            "__ncc_diag_copy",
            "__ncc_diag_clear",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        assert!(ir.contains("ret i64 64"));
        for forbidden in [
            "@malloc(", "@free(", "@memcpy(", "@printf(", "libc", "msvcrt",
        ] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
