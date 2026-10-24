pub(crate) fn render_convert_runtime() -> &'static str {
    r##"
@__ncc_convert_hex_spec = private constant [3 x i8] c"#x\00"

define internal i64 @__ncc_convert_ok_i64(i64 %value) {
entry:
  %shifted = shl i64 %value, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
}

define internal i64 @__ncc_convert_error(i64 %code) {
entry:
  %shifted = shl i64 %code, 1
  ret i64 %shifted
}

define internal i1 @__ncc_convert_finite(double %value) {
entry:
  %ordered = fcmp ord double %value, %value
  %low = fcmp oge double %value, -1.7976931348623157e+308
  %high = fcmp ole double %value, 1.7976931348623157e+308
  %bounded_low = and i1 %ordered, %low
  %bounded = and i1 %bounded_low, %high
  ret i1 %bounded
}

define internal i64 @__ncc_convert_force_f64(double %value) {
entry:
  %finite = call i1 @__ncc_convert_finite(double %value)
  br i1 %finite, label %range, label %zero
range:
  %above = fcmp oge double %value, 9.2233720368547758e+18
  br i1 %above, label %maximum, label %lower
lower:
  %below = fcmp ole double %value, -9.2233720368547758e+18
  br i1 %below, label %minimum, label %convert
convert:
  %integer = fptosi double %value to i64
  ret i64 %integer
maximum:
  ret i64 9223372036854775807
minimum:
  ret i64 -9223372036854775808
zero:
  ret i64 0
}

define internal i64 @__ncc_convert_box_f64(double %value) {
entry:
  %storage = call ptr @nccTextFormatRuntime_allocRaw(i32 16)
  %missing = icmp eq ptr %storage, null
  br i1 %missing, label %error, label %store
store:
  store i64 5639425166293747252, ptr %storage, align 1
  %payload = getelementptr i8, ptr %storage, i64 8
  store double %value, ptr %payload, align 1
  %address = ptrtoint ptr %storage to i64
  %shifted = shl i64 %address, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
error:
  %failure = call i64 @__ncc_convert_error(i64 4)
  ret i64 %failure
}

define ptr @ncc_rt_convert_to_str_i64(i64 %value) {
entry:
  %out = call ptr @nccTextFormatRuntime_fromIntRaw(i64 %value)
  ret ptr %out
}

define ptr @ncc_rt_convert_to_str_char(i64 %value) {
entry:
  %character = trunc i64 %value to i8
  %out = call ptr @nccTextFormatRuntime_charToStrRaw(i8 %character)
  ret ptr %out
}

define ptr @ncc_rt_convert_to_str_bool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %out = call ptr @nccTextFormatRuntime_fromBoolRaw(i1 %truth)
  ret ptr %out
}

define ptr @ncc_rt_convert_to_str_f64(double %value) {
entry:
  %out = call ptr @nccTextFormatRuntime_fromFloatRaw(double %value)
  ret ptr %out
}

define ptr @ncc_rt_convert_to_str_str(ptr %value) {
entry:
  %missing = icmp eq ptr %value, null
  br i1 %missing, label %empty, label %copy
copy:
  %length = call i32 @nccTextFormatRuntime_lengthRaw(ptr %value)
  %out = call ptr @nccTextFormatRuntime_sliceRaw(ptr %value, i32 0, i32 %length)
  ret ptr %out
empty:
  %blank = call ptr @nccTextFormatRuntime_emptyRaw()
  ret ptr %blank
}

define ptr @ncc_rt_convert_to_str_ptr(ptr %value) {
entry:
  %integer = ptrtoint ptr %value to i64
  %spec = getelementptr [3 x i8], ptr @__ncc_convert_hex_spec, i64 0, i64 0
  %out = call ptr @nccTextFormatRuntime_formatUIntRaw(i64 %integer, ptr %spec)
  ret ptr %out
}

define i64 @ncc_rt_convert_to_int_i64(i64 %value) {
entry:
  %out = call i64 @__ncc_convert_ok_i64(i64 %value)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_int_bool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_int_char(i64 %value) {
entry:
  %out = call i64 @__ncc_convert_ok_i64(i64 %value)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_int_f64(double %value) {
entry:
  %finite = call i1 @__ncc_convert_finite(double %value)
  %low = fcmp oge double %value, -9.2233720368547758e+18
  %high = fcmp olt double %value, 9.2233720368547758e+18
  %bounded_low = and i1 %finite, %low
  %bounded = and i1 %bounded_low, %high
  br i1 %bounded, label %convert, label %range_error
convert:
  %integer = fptosi double %value to i64
  %roundtrip = sitofp i64 %integer to double
  %exact = fcmp oeq double %roundtrip, %value
  br i1 %exact, label %success, label %parse_error
success:
  %ok = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %ok
parse_error:
  %bad = call i64 @__ncc_convert_error(i64 2)
  ret i64 %bad
range_error:
  %range_bad = call i64 @__ncc_convert_error(i64 3)
  ret i64 %range_bad
}

define i64 @ncc_rt_convert_to_int_str(ptr %value) {
entry:
  %integer_slot = alloca i32, align 4
  %integer_ok = call i1 @nccTextParseRuntime_tryParseIntRaw(ptr %value, ptr %integer_slot)
  br i1 %integer_ok, label %integer, label %floating_try
integer:
  %narrow = load i32, ptr %integer_slot, align 4
  %wide = sext i32 %narrow to i64
  %ok = call i64 @__ncc_convert_ok_i64(i64 %wide)
  ret i64 %ok
floating_try:
  %float_slot = alloca double, align 8
  %float_ok = call i1 @nccTextParseRuntime_tryParseFloatRaw(ptr %value, ptr %float_slot)
  br i1 %float_ok, label %floating, label %error
floating:
  %float = load double, ptr %float_slot, align 8
  %converted = call i64 @ncc_rt_convert_to_int_f64(double %float)
  ret i64 %converted
error:
  %bad = call i64 @__ncc_convert_error(i64 2)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_int_ptr(ptr %value) {
entry:
  %bad = call i64 @__ncc_convert_error(i64 1)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_float_i64(i64 %value) {
entry:
  %floating = sitofp i64 %value to double
  %out = call i64 @__ncc_convert_box_f64(double %floating)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_float_bool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  %floating = uitofp i64 %integer to double
  %out = call i64 @__ncc_convert_box_f64(double %floating)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_float_char(i64 %value) {
entry:
  %floating = sitofp i64 %value to double
  %out = call i64 @__ncc_convert_box_f64(double %floating)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_float_f64(double %value) {
entry:
  %finite = call i1 @__ncc_convert_finite(double %value)
  br i1 %finite, label %success, label %error
success:
  %out = call i64 @__ncc_convert_box_f64(double %value)
  ret i64 %out
error:
  %bad = call i64 @__ncc_convert_error(i64 3)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_float_str(ptr %value) {
entry:
  %slot = alloca double, align 8
  %parsed = call i1 @nccTextParseRuntime_tryParseFloatRaw(ptr %value, ptr %slot)
  br i1 %parsed, label %success, label %error
success:
  %floating = load double, ptr %slot, align 8
  %out = call i64 @__ncc_convert_box_f64(double %floating)
  ret i64 %out
error:
  %bad = call i64 @__ncc_convert_error(i64 2)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_float_ptr(ptr %value) {
entry:
  %bad = call i64 @__ncc_convert_error(i64 1)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_bool_i64(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_bool_bool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_bool_char(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
}

define i64 @ncc_rt_convert_to_bool_f64(double %value) {
entry:
  %finite = call i1 @__ncc_convert_finite(double %value)
  br i1 %finite, label %success, label %error
success:
  %truth = fcmp one double %value, 0.0
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
error:
  %bad = call i64 @__ncc_convert_error(i64 3)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_bool_str(ptr %value) {
entry:
  %slot = alloca i1, align 1
  %parsed = call i1 @nccTextParseRuntime_tryParseBoolRaw(ptr %value, ptr %slot)
  br i1 %parsed, label %success, label %error
success:
  %truth = load i1, ptr %slot, align 1
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
error:
  %bad = call i64 @__ncc_convert_error(i64 2)
  ret i64 %bad
}

define i64 @ncc_rt_convert_to_bool_ptr(ptr %value) {
entry:
  %truth = icmp ne ptr %value, null
  %integer = zext i1 %truth to i64
  %out = call i64 @__ncc_convert_ok_i64(i64 %integer)
  ret i64 %out
}

define i64 @ncc_rt_convert_force_int_i64(i64 %value) { ret i64 %value }

define i64 @ncc_rt_convert_force_int_bool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %integer = zext i1 %truth to i64
  ret i64 %integer
}

define i64 @ncc_rt_convert_force_int_char(i64 %value) { ret i64 %value }

define i64 @ncc_rt_convert_force_int_f64(double %value) {
entry:
  %out = call i64 @__ncc_convert_force_f64(double %value)
  ret i64 %out
}

define i64 @ncc_rt_convert_force_int_str(ptr %value) {
entry:
  %integer_slot = alloca i32, align 4
  %integer_ok = call i1 @nccTextParseRuntime_tryParseIntRaw(ptr %value, ptr %integer_slot)
  br i1 %integer_ok, label %integer, label %floating_try
integer:
  %narrow = load i32, ptr %integer_slot, align 4
  %wide = sext i32 %narrow to i64
  ret i64 %wide
floating_try:
  %float_slot = alloca double, align 8
  %float_ok = call i1 @nccTextParseRuntime_tryParseFloatRaw(ptr %value, ptr %float_slot)
  br i1 %float_ok, label %floating, label %boolean_try
floating:
  %float = load double, ptr %float_slot, align 8
  %converted = call i64 @__ncc_convert_force_f64(double %float)
  ret i64 %converted
boolean_try:
  %bool_slot = alloca i1, align 1
  %bool_ok = call i1 @nccTextParseRuntime_tryParseBoolRaw(ptr %value, ptr %bool_slot)
  br i1 %bool_ok, label %boolean, label %zero
boolean:
  %truth = load i1, ptr %bool_slot, align 1
  %integer_bool = zext i1 %truth to i64
  ret i64 %integer_bool
zero:
  ret i64 0
}

define i64 @ncc_rt_convert_force_int_ptr(ptr %value) {
entry:
  %integer = ptrtoint ptr %value to i64
  ret i64 %integer
}

define double @ncc_rt_convert_float_payload(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %positive = icmp eq i64 %tag, 1
  br i1 %positive, label %decode, label %invalid
decode:
  %address = lshr i64 %tagged, 1
  %storage = inttoptr i64 %address to ptr
  %missing = icmp eq ptr %storage, null
  br i1 %missing, label %invalid, label %validate
validate:
  %magic = load i64, ptr %storage, align 1
  %valid = icmp eq i64 %magic, 5639425166293747252
  br i1 %valid, label %consume, label %invalid
consume:
  %payload = getelementptr i8, ptr %storage, i64 8
  %value = load double, ptr %payload, align 1
  store i64 0, ptr %storage, align 1
  %released = call i1 @nccTextFormatRuntime_releaseRaw(ptr %storage)
  ret double %value
invalid:
  ret double 0.0
}

define i32 @ncc_rt_convert_float_release(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %positive = icmp eq i64 %tag, 1
  br i1 %positive, label %decode, label %invalid
decode:
  %address = lshr i64 %tagged, 1
  %storage = inttoptr i64 %address to ptr
  %missing = icmp eq ptr %storage, null
  br i1 %missing, label %invalid, label %validate
validate:
  %magic = load i64, ptr %storage, align 1
  %valid = icmp eq i64 %magic, 5639425166293747252
  br i1 %valid, label %release, label %invalid
release:
  store i64 0, ptr %storage, align 1
  %released = call i1 @nccTextFormatRuntime_releaseRaw(ptr %storage)
  %result = zext i1 %released to i32
  ret i32 %result
invalid:
  ret i32 0
}
"##
}

#[cfg(test)]
mod tests {
    use super::render_convert_runtime;

    #[test]
    fn convert_runtime_is_material_and_has_no_c_provider_calls() {
        let llvm = render_convert_runtime();
        for symbol in [
            "ncc_rt_convert_to_str_i64",
            "ncc_rt_convert_to_int_str",
            "ncc_rt_convert_to_float_str",
            "ncc_rt_convert_to_bool_str",
            "ncc_rt_convert_force_int_str",
            "ncc_rt_convert_float_payload",
            "ncc_rt_convert_float_release",
        ] {
            assert!(llvm.lines().any(|line| {
                line.starts_with("define ") && line.contains(&format!("@{symbol}("))
            }));
        }
        for forbidden in [
            "@malloc(",
            "@free(",
            "@strtod(",
            "@snprintf(",
            "getelementptr inbounds",
        ] {
            assert!(
                !llvm.contains(forbidden),
                "forbidden provider dependency {forbidden}"
            );
        }
    }
}
