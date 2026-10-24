pub(crate) struct MemoryBytesRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const MEMORY_BYTES_MANUAL_CALLS: &[MemoryBytesRuntimeCall] = &[
    call(
        "nC.runtime.bytes.byteAt",
        "__ncc_bytes_at",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.bytes.bytesToHex",
        "__ncc_bytes_to_hex",
        &["ptr", "i64", "ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.bytes.hexToBytes",
        "__ncc_hex_to_bytes",
        &["ptr", "i64", "ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.bytes.bytesEq",
        "__ncc_bytes_equal",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.bytes.bytesLen",
        "__ncc_bytes_length",
        &["ptr", "i64"],
        "i64",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> MemoryBytesRuntimeCall {
    MemoryBytesRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static MemoryBytesRuntimeCall> {
    MEMORY_BYTES_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    MEMORY_BYTES_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_memory_bytes_manual_runtime() -> &'static str {
    r#"
define i64 @__ncc_bytes_at(ptr %data, i64 %offset) {
  %slot = getelementptr i8, ptr %data, i64 %offset
  %raw = load i8, ptr %slot, align 1
  %value = zext i8 %raw to i64
  ret i64 %value
}

define internal i8 @__ncc_hex_digit(i8 %value) {
  %low = icmp ult i8 %value, 10
  %decimal = add i8 %value, 48
  %letter_base = sub i8 %value, 10
  %letter = add i8 %letter_base, 97
  %digit = select i1 %low, i8 %decimal, i8 %letter
  ret i8 %digit
}

define i64 @__ncc_bytes_to_hex(ptr %source, i64 %source_bytes, ptr %output, i64 %output_capacity) {
  %required = mul i64 %source_bytes, 2
  %fits = icmp uge i64 %output_capacity, %required
  br i1 %fits, label %loop, label %capacity_error
capacity_error:
  ret i64 -1
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %index, %source_bytes
  br i1 %done, label %exit, label %write
write:
  %source_slot = getelementptr i8, ptr %source, i64 %index
  %value = load i8, ptr %source_slot, align 1
  %high_raw = lshr i8 %value, 4
  %low_raw = and i8 %value, 15
  %high = call i8 @__ncc_hex_digit(i8 %high_raw)
  %low = call i8 @__ncc_hex_digit(i8 %low_raw)
  %output_index = mul i64 %index, 2
  %high_slot = getelementptr i8, ptr %output, i64 %output_index
  %low_index = add i64 %output_index, 1
  %low_slot = getelementptr i8, ptr %output, i64 %low_index
  store i8 %high, ptr %high_slot, align 1
  store i8 %low, ptr %low_slot, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %required
}

define internal i64 @__ncc_hex_nibble(i8 %value) {
  %decimal_low = icmp uge i8 %value, 48
  %decimal_high = icmp ule i8 %value, 57
  %decimal = and i1 %decimal_low, %decimal_high
  %lower_low = icmp uge i8 %value, 97
  %lower_high = icmp ule i8 %value, 102
  %lower = and i1 %lower_low, %lower_high
  %upper_low = icmp uge i8 %value, 65
  %upper_high = icmp ule i8 %value, 70
  %upper = and i1 %upper_low, %upper_high
  %decimal_value8 = sub i8 %value, 48
  %lower_value8 = sub i8 %value, 87
  %upper_value8 = sub i8 %value, 55
  %decimal_value = zext i8 %decimal_value8 to i64
  %lower_value = zext i8 %lower_value8 to i64
  %upper_value = zext i8 %upper_value8 to i64
  %letter_value = select i1 %lower, i64 %lower_value, i64 %upper_value
  %is_letter = or i1 %lower, %upper
  %selected = select i1 %decimal, i64 %decimal_value, i64 %letter_value
  %valid = or i1 %decimal, %is_letter
  %result = select i1 %valid, i64 %selected, i64 -1
  ret i64 %result
}

define i64 @__ncc_hex_to_bytes(ptr %source, i64 %source_bytes, ptr %output, i64 %output_capacity) {
  %odd = and i64 %source_bytes, 1
  %even = icmp eq i64 %odd, 0
  br i1 %even, label %capacity, label %format_error
format_error:
  ret i64 -2
capacity:
  %required = udiv i64 %source_bytes, 2
  %fits = icmp uge i64 %output_capacity, %required
  br i1 %fits, label %loop, label %capacity_error
capacity_error:
  ret i64 -1
loop:
  %index = phi i64 [ 0, %capacity ], [ %next, %store ]
  %done = icmp uge i64 %index, %required
  br i1 %done, label %exit, label %read
read:
  %source_index = mul i64 %index, 2
  %high_slot = getelementptr i8, ptr %source, i64 %source_index
  %low_index = add i64 %source_index, 1
  %low_slot = getelementptr i8, ptr %source, i64 %low_index
  %high_raw = load i8, ptr %high_slot, align 1
  %low_raw = load i8, ptr %low_slot, align 1
  %high = call i64 @__ncc_hex_nibble(i8 %high_raw)
  %low = call i64 @__ncc_hex_nibble(i8 %low_raw)
  %high_valid = icmp sge i64 %high, 0
  %low_valid = icmp sge i64 %low, 0
  %valid = and i1 %high_valid, %low_valid
  br i1 %valid, label %store, label %digit_error
digit_error:
  ret i64 -2
store:
  %shifted = shl i64 %high, 4
  %combined = or i64 %shifted, %low
  %byte = trunc i64 %combined to i8
  %output_slot = getelementptr i8, ptr %output, i64 %index
  store i8 %byte, ptr %output_slot, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i64 %required
}

define i1 @__ncc_bytes_equal(ptr %left, i64 %left_bytes, ptr %right, i64 %right_bytes) {
  %same_size = icmp eq i64 %left_bytes, %right_bytes
  br i1 %same_size, label %loop, label %not_equal
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %advance ]
  %done = icmp uge i64 %index, %left_bytes
  br i1 %done, label %equal, label %compare
compare:
  %left_slot = getelementptr i8, ptr %left, i64 %index
  %right_slot = getelementptr i8, ptr %right, i64 %index
  %left_value = load i8, ptr %left_slot, align 1
  %right_value = load i8, ptr %right_slot, align 1
  %same = icmp eq i8 %left_value, %right_value
  br i1 %same, label %advance, label %not_equal
advance:
  %next = add i64 %index, 1
  br label %loop
equal:
  ret i1 true
not_equal:
  ret i1 false
}

define i64 @__ncc_bytes_length(ptr %data, i64 %bytes) {
  ret i64 %bytes
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_runtime_is_caller_owned_and_has_no_c_provider() {
        let llvm = render_memory_bytes_manual_runtime();
        assert!(llvm.contains("@__ncc_bytes_to_hex"));
        assert!(llvm.contains("@__ncc_hex_to_bytes"));
        assert!(!llvm.contains("BufferHandle"));
        assert!(!llvm.contains("malloc"));
        assert!(!llvm.contains("ncc_rt_"));
        assert!(!llvm.contains("getelementptr inbounds"));
    }
}
