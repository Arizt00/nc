pub(crate) fn render_math_random_runtime() -> &'static str {
    r#"
@__ncc_random_state = internal global i64 -7046029254386353131, align 8

define internal i64 @__ncc_random_next() {
entry:
  br label %retry
retry:
  %old = load atomic i64, ptr @__ncc_random_state seq_cst, align 8
  %left13 = shl i64 %old, 13
  %step1 = xor i64 %old, %left13
  %right7 = lshr i64 %step1, 7
  %step2 = xor i64 %step1, %right7
  %left17 = shl i64 %step2, 17
  %candidate = xor i64 %step2, %left17
  %zero = icmp eq i64 %candidate, 0
  %next = select i1 %zero, i64 -7046029254386353131, i64 %candidate
  %exchange = cmpxchg ptr @__ncc_random_state, i64 %old, i64 %next seq_cst seq_cst
  %stored = extractvalue { i64, i1 } %exchange, 1
  br i1 %stored, label %done, label %retry
done:
  ret i64 %next
}

define i32 @ncc_rt_random_seed(i32 %seed) {
entry:
  %wide = sext i32 %seed to i64
  store atomic i64 %wide, ptr @__ncc_random_state seq_cst, align 8
  ret i32 1
}

define i32 @ncc_rt_random_int() {
entry:
  %raw = call i64 @__ncc_random_next()
  %masked = and i64 %raw, 2147483647
  %value = trunc i64 %masked to i32
  ret i32 %value
}

define i32 @ncc_rt_random_bool() {
entry:
  %raw = call i64 @__ncc_random_next()
  %bit = and i64 %raw, 1
  %value = trunc i64 %bit to i32
  ret i32 %value
}

define double @ncc_rt_random_float() {
entry:
  %raw = call i64 @__ncc_random_next()
  %mantissa = lshr i64 %raw, 11
  %number = uitofp i64 %mantissa to double
  %value = fdiv double %number, 9.0071992547409920e+15
  ret double %value
}

define i64 @ncc_rt_random_float_bits() {
entry:
  %value = call double @ncc_rt_random_float()
  %bits = bitcast double %value to i64
  ret i64 %bits
}

define i32 @ncc_rt_random_range(i32 %low, i32 %high) {
entry:
  %ordered = icmp sle i32 %low, %high
  %minimum = select i1 %ordered, i32 %low, i32 %high
  %maximum = select i1 %ordered, i32 %high, i32 %low
  %minimum64 = sext i32 %minimum to i64
  %maximum64 = sext i32 %maximum to i64
  %distance = sub i64 %maximum64, %minimum64
  %width = add i64 %distance, 1
  %raw = call i64 @__ncc_random_next()
  %offset = urem i64 %raw, %width
  %result64 = add i64 %minimum64, %offset
  %result = trunc i64 %result64 to i32
  ret i32 %result
}

define i64 @ncc_rt_math_sign(i64 %value) {
entry:
  %positive = icmp sgt i64 %value, 0
  %negative = icmp slt i64 %value, 0
  %positive64 = zext i1 %positive to i64
  %negative64 = zext i1 %negative to i64
  %sign = sub i64 %positive64, %negative64
  ret i64 %sign
}

define i64 @ncc_rt_math_abs(i64 %value) {
entry:
  %minimum = icmp eq i64 %value, -9223372036854775808
  br i1 %minimum, label %saturate, label %normal
saturate:
  ret i64 9223372036854775807
normal:
  %negative = icmp slt i64 %value, 0
  %negated = sub i64 0, %value
  %absolute = select i1 %negative, i64 %negated, i64 %value
  ret i64 %absolute
}
"#
}

#[cfg(test)]
mod tests {
    use super::render_math_random_runtime;

    #[test]
    fn owns_every_math_random_symbol_without_c_or_hidden_allocation() {
        let llvm = render_math_random_runtime();
        for symbol in [
            "ncc_rt_random_seed",
            "ncc_rt_random_int",
            "ncc_rt_random_bool",
            "ncc_rt_random_float",
            "ncc_rt_random_float_bits",
            "ncc_rt_random_range",
            "ncc_rt_math_sign",
            "ncc_rt_math_abs",
        ] {
            assert!(llvm.contains(&format!("@{symbol}(")));
        }
        for forbidden in ["malloc", "calloc", "realloc", "free(", "libc", "msvcrt"] {
            assert!(
                !llvm.contains(forbidden),
                "unexpected dependency {forbidden}"
            );
        }
        assert!(llvm.contains("cmpxchg ptr @__ncc_random_state"));
    }
}
