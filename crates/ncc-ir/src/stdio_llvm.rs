pub(crate) fn render_stdio_runtime(
    target: &str,
    include_std: bool,
    include_error: bool,
    include_flush: bool,
) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let (std_platform, error_platform) = if target.contains("windows")
        && (target.contains("x86_64") || target.contains("aarch64"))
    {
        (WINDOWS_STD, WINDOWS_ERROR)
    } else if target.contains("wasi") || target.contains("wasm32") {
        (WASI_STD, WASI_ERROR)
    } else if target.contains("darwin") || target.contains("apple") {
        (POSIX_STD, POSIX64_ERROR)
    } else if target.contains("linux") || target.contains("android") {
        if target.contains("i386")
            || target.contains("i486")
            || target.contains("i586")
            || target.contains("i686")
            || target.contains("armv7")
            || target.contains("thumbv7")
            || target.contains("riscv32")
        {
            (POSIX_STD, POSIX32_ERROR)
        } else {
            (POSIX_STD, POSIX64_ERROR)
        }
    } else {
        return Err(format!(
            "std/errorOutput/flush has no direct operating-system ABI for target '{target}'"
        ));
    };

    let mut runtime = String::new();
    if include_std {
        runtime.push_str(std_platform);
        runtime.push('\n');
    }
    if include_error {
        runtime.push_str(error_platform);
        runtime.push('\n');
        runtime.push_str(ERROR_COMMON);
        runtime.push('\n');
    }
    if include_flush {
        runtime.push_str(FLUSH_COMMON);
        runtime.push('\n');
    }
    Ok(runtime)
}

const WINDOWS_ERROR: &str = r#"
define internal i32 @__ncc_stdio_write_stderr(ptr %data, i64 %length) {
entry:
  %handle = call ptr @GetStdHandle(i32 -12)
  %null = icmp eq ptr %handle, null
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  br i1 %bad, label %failure, label %start

start:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop

loop:
  %offset = phi i64 [ 0, %start ], [ %next, %written.ok ]
  %remaining = sub i64 %length, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %source = getelementptr i8, ptr %data, i64 %offset
  %written.slot = alloca i32, align 4
  %status = call i32 @WriteFile(ptr %handle, ptr %source, i32 %chunk, ptr %written.slot, ptr null)
  %written = load i32, ptr %written.slot, align 4
  %status.ok = icmp ne i32 %status, 0
  %progress = icmp ugt i32 %written, 0
  %ok = and i1 %status.ok, %progress
  br i1 %ok, label %written.ok, label %failure

written.ok:
  %written64 = zext i32 %written to i64
  %next = add i64 %offset, %written64
  %done = icmp uge i64 %next, %length
  br i1 %done, label %success, label %loop

success:
  ret i32 1

failure:
  ret i32 0
}
"#;

const WINDOWS_STD: &str = r#"
define i64 @ncc_rt_std_in() {
entry:
  %handle = call ptr @GetStdHandle(i32 -10)
  %value = ptrtoint ptr %handle to i64
  ret i64 %value
}

define i64 @ncc_rt_std_out() {
entry:
  %handle = call ptr @GetStdHandle(i32 -11)
  %value = ptrtoint ptr %handle to i64
  ret i64 %value
}

define i64 @ncc_rt_std_err() {
entry:
  %handle = call ptr @GetStdHandle(i32 -12)
  %value = ptrtoint ptr %handle to i64
  ret i64 %value
}

define i32 @ncc_rt_std_is_interactive() {
entry:
  %mode = alloca i32, align 4
  %input = call ptr @GetStdHandle(i32 -10)
  %input.null = icmp eq ptr %input, null
  %input.invalid = icmp eq ptr %input, inttoptr (i64 -1 to ptr)
  %input.bad = or i1 %input.null, %input.invalid
  br i1 %input.bad, label %output, label %probe.input

probe.input:
  %input.status = call i32 @GetConsoleMode(ptr %input, ptr %mode)
  %input.ok = icmp ne i32 %input.status, 0
  br i1 %input.ok, label %interactive, label %output

output:
  %output.handle = call ptr @GetStdHandle(i32 -11)
  %output.null = icmp eq ptr %output.handle, null
  %output.invalid = icmp eq ptr %output.handle, inttoptr (i64 -1 to ptr)
  %output.bad = or i1 %output.null, %output.invalid
  br i1 %output.bad, label %noninteractive, label %probe.output

probe.output:
  %output.status = call i32 @GetConsoleMode(ptr %output.handle, ptr %mode)
  %output.ok = icmp ne i32 %output.status, 0
  br i1 %output.ok, label %interactive, label %noninteractive

interactive:
  ret i32 1

noninteractive:
  ret i32 0
}
"#;

const POSIX64_ERROR: &str = r#"
define internal i32 @__ncc_stdio_write_stderr(ptr %data, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop

loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %written.ok ]
  %remaining = sub i64 %length, %offset
  %source = getelementptr i8, ptr %data, i64 %offset
  %written = call i64 @write(i32 2, ptr %source, i64 %remaining)
  %ok = icmp sgt i64 %written, 0
  br i1 %ok, label %written.ok, label %failure

written.ok:
  %next = add i64 %offset, %written
  %done = icmp uge i64 %next, %length
  br i1 %done, label %success, label %loop

success:
  ret i32 1

failure:
  ret i32 0
}
"#;

const POSIX_STD: &str = r#"
define i64 @ncc_rt_std_in() { ret i64 0 }
define i64 @ncc_rt_std_out() { ret i64 1 }
define i64 @ncc_rt_std_err() { ret i64 2 }

define i32 @ncc_rt_std_is_interactive() {
entry:
  %input = call i32 @isatty(i32 0)
  %input.ok = icmp ne i32 %input, 0
  br i1 %input.ok, label %interactive, label %output
output:
  %out = call i32 @isatty(i32 1)
  %out.ok = icmp ne i32 %out, 0
  br i1 %out.ok, label %interactive, label %noninteractive
interactive:
  ret i32 1
noninteractive:
  ret i32 0
}
"#;

const POSIX32_ERROR: &str = r#"
define internal i32 @__ncc_stdio_write_stderr(ptr %data, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop

loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %written.ok ]
  %remaining64 = sub i64 %length, %offset
  %large = icmp ugt i64 %remaining64, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining64
  %chunk = trunc i64 %chunk64 to i32
  %source = getelementptr i8, ptr %data, i64 %offset
  %written = call i32 @write(i32 2, ptr %source, i32 %chunk)
  %ok = icmp sgt i32 %written, 0
  br i1 %ok, label %written.ok, label %failure

written.ok:
  %written64 = zext i32 %written to i64
  %next = add i64 %offset, %written64
  %done = icmp uge i64 %next, %length
  br i1 %done, label %success, label %loop

success:
  ret i32 1

failure:
  ret i32 0
}
"#;

const WASI_ERROR: &str = r#"
%ncc.stdio.wasi.iovec = type { ptr, i32 }

define internal i32 @__ncc_stdio_write_stderr(ptr %data, i64 %length) {
entry:
  %too.large = icmp ugt i64 %length, 4294967295
  br i1 %too.large, label %failure, label %write
write:
  %length32 = trunc i64 %length to i32
  %iov = alloca %ncc.stdio.wasi.iovec, align 4
  %iov.base = getelementptr %ncc.stdio.wasi.iovec, ptr %iov, i32 0, i32 0
  %iov.length = getelementptr %ncc.stdio.wasi.iovec, ptr %iov, i32 0, i32 1
  store ptr %data, ptr %iov.base, align 4
  store i32 %length32, ptr %iov.length, align 4
  %written = alloca i32, align 4
  %status = call i32 @__wasi_fd_write(i32 2, ptr %iov, i32 1, ptr %written)
  %count = load i32, ptr %written, align 4
  %status.ok = icmp eq i32 %status, 0
  %count.ok = icmp eq i32 %count, %length32
  %ok = and i1 %status.ok, %count.ok
  br i1 %ok, label %success, label %failure
success:
  ret i32 1
failure:
  ret i32 0
}
"#;

const WASI_STD: &str = r#"
define i64 @ncc_rt_std_in() { ret i64 0 }
define i64 @ncc_rt_std_out() { ret i64 1 }
define i64 @ncc_rt_std_err() { ret i64 2 }
define i32 @ncc_rt_std_is_interactive() { ret i32 0 }
"#;

const ERROR_COMMON: &str = r#"
@__ncc_stdio.warn = private unnamed_addr constant [8 x i8] c"[warn] \00", align 1
@__ncc_stdio.error = private unnamed_addr constant [9 x i8] c"[error] \00", align 1
@__ncc_stdio.newline = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@__ncc_stdio.null = private unnamed_addr constant [5 x i8] c"null\00", align 1
@__ncc_stdio.true = private unnamed_addr constant [5 x i8] c"true\00", align 1
@__ncc_stdio.false = private unnamed_addr constant [6 x i8] c"false\00", align 1

define internal i64 @__ncc_stdio_cstr_len(ptr %text) {
entry:
  %is.null = icmp eq ptr %text, null
  br i1 %is.null, label %done.null, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %step ]
  %at = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %at, align 1
  %end = icmp eq i8 %byte, 0
  br i1 %end, label %done, label %step
step:
  %next = add i64 %index, 1
  br label %loop
done:
  ret i64 %index
done.null:
  ret i64 0
}

define internal i32 @__ncc_stdio_error_emit(ptr %text, i32 %mode) {
entry:
  %is.null = icmp eq ptr %text, null
  %value = select i1 %is.null, ptr @__ncc_stdio.null, ptr %text
  %warn = icmp eq i32 %mode, 2
  br i1 %warn, label %prefix.warn, label %test.error
prefix.warn:
  %warn.status = call i32 @__ncc_stdio_write_stderr(ptr @__ncc_stdio.warn, i64 7)
  %warn.ok = icmp ne i32 %warn.status, 0
  br i1 %warn.ok, label %body, label %failure
test.error:
  %error = icmp eq i32 %mode, 3
  br i1 %error, label %prefix.error, label %body
prefix.error:
  %error.status = call i32 @__ncc_stdio_write_stderr(ptr @__ncc_stdio.error, i64 8)
  %error.ok = icmp ne i32 %error.status, 0
  br i1 %error.ok, label %body, label %failure
body:
  %length = call i64 @__ncc_stdio_cstr_len(ptr %value)
  %body.status = call i32 @__ncc_stdio_write_stderr(ptr %value, i64 %length)
  %body.ok = icmp ne i32 %body.status, 0
  br i1 %body.ok, label %newline.test, label %failure
newline.test:
  %raw = icmp eq i32 %mode, 0
  br i1 %raw, label %success, label %newline
newline:
  %newline.status = call i32 @__ncc_stdio_write_stderr(ptr @__ncc_stdio.newline, i64 1)
  %newline.ok = icmp ne i32 %newline.status, 0
  br i1 %newline.ok, label %success, label %failure
success:
  ret i32 1
failure:
  ret i32 0
}

define i32 @ncc_rt_error_text(ptr %text, i32 %mode) {
entry:
  %status = call i32 @__ncc_stdio_error_emit(ptr %text, i32 %mode)
  ret i32 %status
}

define i32 @ncc_rt_error_i64(i64 %value, i32 %mode) {
entry:
  %text = call ptr @nccTextFormatRuntime_fromIntRaw(i64 %value)
  %status = call i32 @__ncc_stdio_error_emit(ptr %text, i32 %mode)
  %released = call i1 @nccTextFormatRuntime_releaseRaw(ptr %text)
  ret i32 %status
}

define i32 @ncc_rt_error_f64(double %value, i32 %mode) {
entry:
  %text = call ptr @nccTextFormatRuntime_fromFloatRaw(double %value)
  %status = call i32 @__ncc_stdio_error_emit(ptr %text, i32 %mode)
  %released = call i1 @nccTextFormatRuntime_releaseRaw(ptr %text)
  ret i32 %status
}

define i32 @ncc_rt_error_f64_bits(i64 %bits, i32 %mode) {
entry:
  %value = bitcast i64 %bits to double
  %status = call i32 @ncc_rt_error_f64(double %value, i32 %mode)
  ret i32 %status
}

define i32 @ncc_rt_error_bool(i32 %value, i32 %mode) {
entry:
  %truth = icmp ne i32 %value, 0
  %text = select i1 %truth, ptr @__ncc_stdio.true, ptr @__ncc_stdio.false
  %status = call i32 @__ncc_stdio_error_emit(ptr %text, i32 %mode)
  ret i32 %status
}

define i32 @ncc_rt_error_flush() {
entry:
  fence seq_cst
  ret i32 1
}
"#;

const FLUSH_COMMON: &str = r#"
@__ncc_stdio.auto_flush = internal global i8 1, align 1

define i32 @ncc_rt_flush_all() {
entry:
  fence seq_cst
  ret i32 1
}

define i32 @ncc_rt_flush_stdout() {
entry:
  fence seq_cst
  ret i32 1
}

define i32 @ncc_rt_flush_stderr() {
entry:
  fence seq_cst
  ret i32 1
}

define i32 @ncc_rt_auto_flush_on_input(i32 %enabled) {
entry:
  %truth = icmp ne i32 %enabled, 0
  %value = zext i1 %truth to i8
  store atomic i8 %value, ptr @__ncc_stdio.auto_flush seq_cst, align 1
  ret i32 1
}

define i32 @ncc_rt_flush_before_input() {
entry:
  %value = load atomic i8, ptr @__ncc_stdio.auto_flush seq_cst, align 1
  %enabled = icmp ne i8 %value, 0
  br i1 %enabled, label %flush, label %success
flush:
  %status = call i32 @ncc_rt_flush_all()
  ret i32 %status
success:
  ret i32 1
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_materializes_native_handles_and_stderr() {
        let ir = render_stdio_runtime("x86_64-pc-windows-msvc", true, true, true).unwrap();
        assert!(ir.contains("call ptr @GetStdHandle(i32 -12)"));
        assert!(ir.contains("call i32 @WriteFile("));
        assert!(ir.contains("define i64 @ncc_rt_std_in()"));
        assert!(ir.contains("define i32 @ncc_rt_error_f64("));
    }

    #[test]
    fn posix_width_tracks_the_target_abi() {
        let x64 = render_stdio_runtime("x86_64-unknown-linux-gnu", false, true, false).unwrap();
        let x86 = render_stdio_runtime("i386-unknown-linux-gnu", false, true, false).unwrap();
        assert!(x64.contains("call i64 @write(i32 2"));
        assert!(x86.contains("call i32 @write(i32 2"));
    }

    #[test]
    fn wasi_uses_fd_write_and_reports_noninteractive_streams() {
        let ir = render_stdio_runtime("wasm32-wasi", true, true, false).unwrap();
        assert!(ir.contains("call i32 @__wasi_fd_write(i32 2"));
        assert!(ir.contains("define i32 @ncc_rt_std_is_interactive() { ret i32 0 }"));
    }

    #[test]
    fn families_materialize_independently() {
        let flush = render_stdio_runtime("x86_64-pc-windows-msvc", false, false, true).unwrap();
        assert!(flush.contains("define i32 @ncc_rt_flush_all()"));
        assert!(!flush.contains("@GetStdHandle("));
        assert!(!flush.contains("define i32 @ncc_rt_error_text("));

        let std = render_stdio_runtime("x86_64-pc-windows-msvc", true, false, false).unwrap();
        assert!(std.contains("define i64 @ncc_rt_std_in()"));
        assert!(!std.contains("@WriteFile("));
        assert!(!std.contains("define i32 @ncc_rt_flush_all()"));
    }
}
