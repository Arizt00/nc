pub(crate) fn render_entropy_uuid_runtime(
    target: &str,
    include_uuid: bool,
) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    let entropy =
        if lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64")) {
            WINDOWS_ENTROPY
        } else if (lower.contains("linux") || lower.contains("android"))
            && (lower.contains("i386")
                || lower.contains("i486")
                || lower.contains("i586")
                || lower.contains("i686")
                || lower.contains("armv7")
                || lower.contains("riscv32"))
        {
            POSIX32_ENTROPY
        } else if lower.contains("linux") || lower.contains("android") {
            POSIX64_ENTROPY
        } else if lower.contains("apple") || lower.contains("darwin") {
            DARWIN_ENTROPY
        } else if lower.contains("wasm32") && lower.contains("wasi") {
            WASI_ENTROPY
        } else {
            return Err(format!(
                "native entropy/UUID backend has no hosted OS provider for target '{target}'"
            ));
        };
    let uuid = if include_uuid { COMMON_UUID } else { "" };
    Ok(format!("{entropy}\n{COMMON_ENTROPY}\n{uuid}"))
}

const WINDOWS_ENTROPY: &str = r#"
declare i8 @SystemFunction036(ptr, i32)

define i32 @ncc_rt_entropy_fill(ptr %out, i64 %len) {
entry:
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %filled ]
  %remaining = sub i64 %len, %offset
  %large = icmp ugt i64 %remaining, 4294967295
  %chunk64 = select i1 %large, i64 4294967295, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %destination = getelementptr i8, ptr %out, i64 %offset
  %status = call i8 @SystemFunction036(ptr %destination, i32 %chunk)
  %ok = icmp ne i8 %status, 0
  br i1 %ok, label %filled, label %failure
filled:
  %next = add i64 %offset, %chunk64
  %done = icmp eq i64 %next, %len
  br i1 %done, label %success, label %loop
success:
  ret i32 1
failure:
  ret i32 0
}
"#;

const POSIX64_ENTROPY: &str = r#"
declare i64 @getrandom(ptr, i64, i32)

define i32 @ncc_rt_entropy_fill(ptr %out, i64 %len) {
entry:
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %filled ]
  %remaining = sub i64 %len, %offset
  %destination = getelementptr i8, ptr %out, i64 %offset
  %received = call i64 @getrandom(ptr %destination, i64 %remaining, i32 0)
  %ok = icmp sgt i64 %received, 0
  br i1 %ok, label %filled, label %failure
filled:
  %next = add i64 %offset, %received
  %done = icmp eq i64 %next, %len
  br i1 %done, label %success, label %loop
success:
  ret i32 1
failure:
  ret i32 0
}
"#;

const POSIX32_ENTROPY: &str = r#"
declare i32 @getrandom(ptr, i32, i32)

define i32 @ncc_rt_entropy_fill(ptr %out, i64 %len) {
entry:
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %filled ]
  %remaining = sub i64 %len, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %destination = getelementptr i8, ptr %out, i64 %offset
  %received32 = call i32 @getrandom(ptr %destination, i32 %chunk, i32 0)
  %ok = icmp sgt i32 %received32, 0
  br i1 %ok, label %filled, label %failure
filled:
  %received = zext i32 %received32 to i64
  %next = add i64 %offset, %received
  %done = icmp eq i64 %next, %len
  br i1 %done, label %success, label %loop
success:
  ret i32 1
failure:
  ret i32 0
}
"#;

const DARWIN_ENTROPY: &str = r#"
declare i32 @getentropy(ptr, i64)

define i32 @ncc_rt_entropy_fill(ptr %out, i64 %len) {
entry:
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %filled ]
  %remaining = sub i64 %len, %offset
  %large = icmp ugt i64 %remaining, 256
  %chunk = select i1 %large, i64 256, i64 %remaining
  %destination = getelementptr i8, ptr %out, i64 %offset
  %status = call i32 @getentropy(ptr %destination, i64 %chunk)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %filled, label %failure
filled:
  %next = add i64 %offset, %chunk
  %done = icmp eq i64 %next, %len
  br i1 %done, label %success, label %loop
success:
  ret i32 1
failure:
  ret i32 0
}
"#;

const WASI_ENTROPY: &str = r#"
declare i32 @__wasi_random_get(ptr, i32)

define i32 @ncc_rt_entropy_fill(ptr %out, i64 %len) {
entry:
  %fits = icmp ule i64 %len, 4294967295
  br i1 %fits, label %call, label %failure
call:
  %len32 = trunc i64 %len to i32
  %status = call i32 @__wasi_random_get(ptr %out, i32 %len32)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
failure:
  ret i32 0
}
"#;

const COMMON_ENTROPY: &str = r#"
define i32 @ncc_rt_entropy_available() {
entry:
  %probe = alloca i8, align 1
  %status = call i32 @ncc_rt_entropy_fill(ptr %probe, i64 1)
  ret i32 %status
}

define i64 @ncc_rt_entropy_bytes(i64 %len) {
entry:
  %empty = icmp eq i64 %len, 0
  %allocated = select i1 %empty, i64 1, i64 %len
  %bytes = call ptr @__ncc_raw_aligned_alloc(i64 %allocated, i64 8)
  %missing = icmp eq ptr %bytes, null
  br i1 %missing, label %alloc_error, label %fill
fill:
  %status = call i32 @ncc_rt_entropy_fill(ptr %bytes, i64 %len)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %fill_error
success:
  %address = ptrtoint ptr %bytes to i64
  %shifted = shl i64 %address, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
fill_error:
  %released = call i1 @__ncc_raw_aligned_free(ptr %bytes)
  ret i64 4
alloc_error:
  ret i64 2
}

define i32 @ncc_rt_entropy_release(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %positive = icmp eq i64 %tag, 1
  br i1 %positive, label %decode, label %failure
decode:
  %address = lshr i64 %tagged, 1
  %bytes = inttoptr i64 %address to ptr
  %released = call i1 @__ncc_raw_aligned_free(ptr %bytes)
  %result = zext i1 %released to i32
  ret i32 %result
failure:
  ret i32 0
}
"#;

const COMMON_UUID: &str = r#"
define internal i8 @__ncc_uuid_hex(i8 %value) {
entry:
  %decimal = icmp ult i8 %value, 10
  %number = add i8 %value, 48
  %letter_base = sub i8 %value, 10
  %letter = add i8 %letter_base, 97
  %out = select i1 %decimal, i8 %number, i8 %letter
  ret i8 %out
}

define i64 @ncc_rt_uuid_from_bytes_into(ptr %bytes, ptr %out, i64 %capacity) {
entry:
  %enough = icmp uge i64 %capacity, 37
  br i1 %enough, label %loop, label %failure
loop:
  %input = phi i64 [ 0, %entry ], [ %input_next, %store_low ]
  %output = phi i64 [ 0, %entry ], [ %output_next, %store_low ]
  %more = icmp ult i64 %input, 16
  br i1 %more, label %classify, label %terminate
classify:
  %at4 = icmp eq i64 %input, 4
  %at6 = icmp eq i64 %input, 6
  %at8 = icmp eq i64 %input, 8
  %at10 = icmp eq i64 %input, 10
  %dash_a = or i1 %at4, %at6
  %dash_b = or i1 %at8, %at10
  %dash = or i1 %dash_a, %dash_b
  br i1 %dash, label %store_dash, label %write_hex
store_dash:
  %dash_slot = getelementptr i8, ptr %out, i64 %output
  store i8 45, ptr %dash_slot, align 1
  %after_dash = add i64 %output, 1
  br label %write_hex
write_hex:
  %hex_output = phi i64 [ %after_dash, %store_dash ], [ %output, %classify ]
  %source = getelementptr i8, ptr %bytes, i64 %input
  %byte = load i8, ptr %source, align 1
  %high_raw = lshr i8 %byte, 4
  %low_raw = and i8 %byte, 15
  %high = call i8 @__ncc_uuid_hex(i8 %high_raw)
  %low = call i8 @__ncc_uuid_hex(i8 %low_raw)
  %high_slot = getelementptr i8, ptr %out, i64 %hex_output
  store i8 %high, ptr %high_slot, align 1
  %low_index = add i64 %hex_output, 1
  br label %store_low
store_low:
  %low_slot = getelementptr i8, ptr %out, i64 %low_index
  store i8 %low, ptr %low_slot, align 1
  %input_next = add i64 %input, 1
  %output_next = add i64 %hex_output, 2
  br label %loop
terminate:
  %terminator = getelementptr i8, ptr %out, i64 36
  store i8 0, ptr %terminator, align 1
  ret i64 36
failure:
  ret i64 0
}

define i64 @ncc_rt_uuid_v4_into(ptr %out, i64 %capacity) {
entry:
  %bytes = alloca [16 x i8], align 16
  %filled = call i32 @ncc_rt_entropy_fill(ptr %bytes, i64 16)
  %ok = icmp ne i32 %filled, 0
  br i1 %ok, label %version, label %failure
version:
  %version_slot = getelementptr i8, ptr %bytes, i64 6
  %version_old = load i8, ptr %version_slot, align 1
  %version_low = and i8 %version_old, 15
  %version_new = or i8 %version_low, 64
  store i8 %version_new, ptr %version_slot, align 1
  %variant_slot = getelementptr i8, ptr %bytes, i64 8
  %variant_old = load i8, ptr %variant_slot, align 1
  %variant_low = and i8 %variant_old, 63
  %variant_new = or i8 %variant_low, -128
  store i8 %variant_new, ptr %variant_slot, align 1
  %written = call i64 @ncc_rt_uuid_from_bytes_into(ptr %bytes, ptr %out, i64 %capacity)
  ret i64 %written
failure:
  ret i64 0
}

define internal ptr @__ncc_uuid_alloc_text() {
entry:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 37, i64 8)
  ret ptr %out
}

define ptr @ncc_rt_uuid_from_bytes(ptr %bytes) {
entry:
  %out = call ptr @__ncc_uuid_alloc_text()
  %missing = icmp eq ptr %out, null
  br i1 %missing, label %failure, label %format
format:
  %written = call i64 @ncc_rt_uuid_from_bytes_into(ptr %bytes, ptr %out, i64 37)
  %ok = icmp eq i64 %written, 36
  br i1 %ok, label %success, label %release
release:
  %released = call i1 @__ncc_raw_aligned_free(ptr %out)
  br label %failure
success:
  ret ptr %out
failure:
  ret ptr null
}

define ptr @ncc_rt_uuid_v4() {
entry:
  %out = call ptr @__ncc_uuid_alloc_text()
  %missing = icmp eq ptr %out, null
  br i1 %missing, label %failure, label %generate
generate:
  %written = call i64 @ncc_rt_uuid_v4_into(ptr %out, i64 37)
  %ok = icmp eq i64 %written, 36
  br i1 %ok, label %success, label %release
release:
  %released = call i1 @__ncc_raw_aligned_free(ptr %out)
  br label %failure
success:
  ret ptr %out
failure:
  ret ptr null
}

define i32 @ncc_rt_uuid_release(ptr %value) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %value)
  %result = zext i1 %released to i32
  ret i32 %result
}

define i32 @ncc_rt_uuid_is_valid(ptr %value) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %accepted ]
  %more = icmp ult i64 %index, 36
  br i1 %more, label %classify, label %terminator
classify:
  %slot = getelementptr i8, ptr %value, i64 %index
  %byte = load i8, ptr %slot, align 1
  %at8 = icmp eq i64 %index, 8
  %at13 = icmp eq i64 %index, 13
  %at18 = icmp eq i64 %index, 18
  %at23 = icmp eq i64 %index, 23
  %dash_a = or i1 %at8, %at13
  %dash_b = or i1 %at18, %at23
  %dash_position = or i1 %dash_a, %dash_b
  br i1 %dash_position, label %expect_dash, label %expect_hex
expect_dash:
  %dash = icmp eq i8 %byte, 45
  br i1 %dash, label %accepted, label %failure
expect_hex:
  %digit_low = icmp uge i8 %byte, 48
  %digit_high = icmp ule i8 %byte, 57
  %digit = and i1 %digit_low, %digit_high
  %lower_low = icmp uge i8 %byte, 97
  %lower_high = icmp ule i8 %byte, 102
  %lower = and i1 %lower_low, %lower_high
  %upper_low = icmp uge i8 %byte, 65
  %upper_high = icmp ule i8 %byte, 70
  %upper = and i1 %upper_low, %upper_high
  %hex_a = or i1 %digit, %lower
  %hex = or i1 %hex_a, %upper
  br i1 %hex, label %accepted, label %failure
accepted:
  %next = add i64 %index, 1
  br label %loop
terminator:
  %end_slot = getelementptr i8, ptr %value, i64 36
  %end = load i8, ptr %end_slot, align 1
  %ended = icmp eq i8 %end, 0
  %result = zext i1 %ended to i32
  ret i32 %result
failure:
  ret i32 0
}
"#;

#[cfg(test)]
mod tests {
    use super::render_entropy_uuid_runtime;

    #[test]
    fn target_renderers_use_os_abi_and_no_c_provider_source() {
        let windows = render_entropy_uuid_runtime("x86_64-pc-windows-msvc", true).unwrap();
        assert!(windows.contains("@SystemFunction036"));
        let linux = render_entropy_uuid_runtime("x86_64-unknown-linux-gnu", true).unwrap();
        assert!(linux.contains("@getrandom"));
        let darwin = render_entropy_uuid_runtime("aarch64-apple-darwin", true).unwrap();
        assert!(darwin.contains("@getentropy"));
        let wasi = render_entropy_uuid_runtime("wasm32-wasi", true).unwrap();
        assert!(wasi.contains("@__wasi_random_get"));
        for llvm in [windows, linux, darwin, wasi] {
            for symbol in [
                "ncc_rt_entropy_fill",
                "ncc_rt_entropy_release",
                "ncc_rt_uuid_v4_into",
                "ncc_rt_uuid_from_bytes_into",
                "ncc_rt_uuid_release",
            ] {
                assert!(llvm.contains(&format!("@{symbol}(")));
            }
            for forbidden in [
                "@malloc(",
                "@calloc(",
                "@realloc(",
                "@free(",
                "runtime_hosted_native.c",
            ] {
                assert!(
                    !llvm.contains(forbidden),
                    "unexpected C provider token {forbidden}"
                );
            }
        }
        let entropy_only = render_entropy_uuid_runtime("x86_64-pc-windows-msvc", false).unwrap();
        assert!(entropy_only.contains("@ncc_rt_entropy_fill("));
        assert!(!entropy_only.contains("@ncc_rt_uuid_v4_into("));
    }
}
