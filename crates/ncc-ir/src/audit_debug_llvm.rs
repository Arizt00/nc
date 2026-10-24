pub(crate) fn render_audit_debug_runtime() -> String {
    AUDIT_DEBUG_RUNTIME.to_string()
}

const AUDIT_DEBUG_RUNTIME: &str = r#"
define i64 @__ncc_debug_stride() {
entry:
  ret i64 32
}

define i1 @__ncc_debug_record(ptr %buffer, i64 %index, i64 %tag, ptr %location, i64 %value, i64 %flags) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %buffer, i64 %offset
  %location_slot = getelementptr i8, ptr %record, i64 8
  %value_slot = getelementptr i8, ptr %record, i64 16
  %flags_slot = getelementptr i8, ptr %record, i64 24
  store i64 %tag, ptr %record, align 1
  store ptr %location, ptr %location_slot, align 1
  store i64 %value, ptr %value_slot, align 1
  store i64 %flags, ptr %flags_slot, align 1
  ret i1 true
}

define i64 @__ncc_debug_tag(ptr %buffer, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %buffer, i64 %offset
  %tag = load i64, ptr %record, align 1
  ret i64 %tag
}

define ptr @__ncc_debug_location(ptr %buffer, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %buffer, i64 %offset
  %slot = getelementptr i8, ptr %record, i64 8
  %location = load ptr, ptr %slot, align 1
  ret ptr %location
}

define i64 @__ncc_debug_value(ptr %buffer, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %buffer, i64 %offset
  %slot = getelementptr i8, ptr %record, i64 16
  %value = load i64, ptr %slot, align 1
  ret i64 %value
}

define i64 @__ncc_debug_flags(ptr %buffer, i64 %index) {
entry:
  %offset = mul i64 %index, 32
  %record = getelementptr i8, ptr %buffer, i64 %offset
  %slot = getelementptr i8, ptr %record, i64 24
  %flags = load i64, ptr %slot, align 1
  ret i64 %flags
}

define i64 @__ncc_debug_dump(ptr %destination, ptr %source, i64 %bytes) {
entry:
  br label %copy
copy:
  %index = phi i64 [ 0, %entry ], [ %next, %write ]
  %within = icmp slt i64 %index, %bytes
  br i1 %within, label %write, label %done
write:
  %source_byte = getelementptr i8, ptr %source, i64 %index
  %destination_byte = getelementptr i8, ptr %destination, i64 %index
  %value = load i8, ptr %source_byte, align 1
  store i8 %value, ptr %destination_byte, align 1
  %next = add i64 %index, 1
  br label %copy
done:
  ret i64 %bytes
}

define i64 @__ncc_debug_checksum(ptr %source, i64 %bytes) {
entry:
  br label %scan
scan:
  %index = phi i64 [ 0, %entry ], [ %next, %mix ]
  %hash = phi i64 [ 1469598103934665603, %entry ], [ %mixed, %mix ]
  %within = icmp slt i64 %index, %bytes
  br i1 %within, label %mix, label %done
mix:
  %slot = getelementptr i8, ptr %source, i64 %index
  %byte = load i8, ptr %slot, align 1
  %wide = zext i8 %byte to i64
  %xor = xor i64 %hash, %wide
  %mixed = mul i64 %xor, 1099511628211
  %next = add i64 %index, 1
  br label %scan
done:
  ret i64 %hash
}

define i1 @__ncc_debug_break() {
entry:
  call void @llvm.trap()
  unreachable
}

define i1 @__ncc_debug_assert(i1 %condition, ptr %context, i64 %bytes) {
entry:
  br i1 %condition, label %pass, label %fail
fail:
  call void @llvm.trap()
  unreachable
pass:
  ret i1 true
}
"#;

#[cfg(test)]
mod tests {
    use super::render_audit_debug_runtime;

    #[test]
    fn debug_runtime_is_caller_owned_bytes_without_c_provider() {
        let ir = render_audit_debug_runtime();
        for material in [
            "__ncc_debug_stride",
            "__ncc_debug_record",
            "__ncc_debug_tag",
            "__ncc_debug_location",
            "__ncc_debug_value",
            "__ncc_debug_flags",
            "__ncc_debug_dump",
            "__ncc_debug_checksum",
            "__ncc_debug_break",
            "__ncc_debug_assert",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        assert!(ir.contains("ret i64 32"));
        assert_eq!(ir.matches("call void @llvm.trap()").count(), 2);
        for forbidden in ["@malloc(", "@free(", "@printf(", "libc", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
