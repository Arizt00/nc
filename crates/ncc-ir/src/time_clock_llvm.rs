pub(crate) fn render_time_clock_runtime(
    target: &str,
    include_date: bool,
    include_compatibility: bool,
) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let platform = if target.contains("windows") {
        if target.starts_with("i386") || target.starts_with("i686") {
            render_windows_i686()
        } else {
            WINDOWS.to_string()
        }
    } else if target.contains("darwin") || target.contains("apple") {
        DARWIN.to_string()
    } else if target.contains("wasi") || target.contains("wasm32") {
        WASI.to_string()
    } else if target.contains("linux") || target.contains("android") {
        if target.contains("i386")
            || target.contains("i486")
            || target.contains("i586")
            || target.contains("i686")
            || target.contains("armv7")
            || target.contains("thumbv7")
            || target.contains("riscv32")
        {
            POSIX32.to_string()
        } else {
            POSIX64.to_string()
        }
    } else {
        return Err(format!(
            "time/clock has no direct operating-system ABI for target '{target}'"
        ));
    };

    let date = if include_date { COMMON_DATE } else { "" };
    let compatibility = if include_compatibility {
        COMMON_DATE_COMPATIBILITY
    } else {
        ""
    };
    Ok(format!(
        "{platform}\n{COMMON_CLOCK}\n{date}\n{compatibility}"
    ))
}

fn render_windows_i686() -> String {
    WINDOWS
        .replace(
            "@GetSystemTimeAsFileTime(",
            "@\"\\01_GetSystemTimeAsFileTime@4\"(",
        )
        .replace(
            "@QueryPerformanceCounter(",
            "@\"\\01_QueryPerformanceCounter@4\"(",
        )
        .replace(
            "@QueryPerformanceFrequency(",
            "@\"\\01_QueryPerformanceFrequency@4\"(",
        )
        .replace("@Sleep(", "@\"\\01_Sleep@4\"(")
        .replace("declare void @\"", "declare x86_stdcallcc void @\"")
        .replace("declare i32 @\"", "declare x86_stdcallcc i32 @\"")
        .replace("call void @\"", "call x86_stdcallcc void @\"")
        .replace("call i32 @\"", "call x86_stdcallcc i32 @\"")
}

const WINDOWS: &str = r#"
declare void @GetSystemTimeAsFileTime(ptr)
declare i32 @QueryPerformanceCounter(ptr)
declare i32 @QueryPerformanceFrequency(ptr)
declare void @Sleep(i32)

define i64 @ncc_rt_now_seconds() {
entry:
  %filetime = alloca i64, align 8
  call void @GetSystemTimeAsFileTime(ptr %filetime)
  %ticks = load i64, ptr %filetime, align 8
  %seconds1601 = udiv i64 %ticks, 10000000
  %seconds1970 = sub i64 %seconds1601, 11644473600
  ret i64 %seconds1970
}

define i64 @ncc_rt_now_millis() {
entry:
  %filetime = alloca i64, align 8
  call void @GetSystemTimeAsFileTime(ptr %filetime)
  %ticks = load i64, ptr %filetime, align 8
  %millis1601 = udiv i64 %ticks, 10000
  %millis1970 = sub i64 %millis1601, 11644473600000
  ret i64 %millis1970
}

define i64 @ncc_rt_monotonic_nanos() {
entry:
  %counter.slot = alloca i64, align 8
  %frequency.slot = alloca i64, align 8
  %counter.ok = call i32 @QueryPerformanceCounter(ptr %counter.slot)
  %frequency.ok = call i32 @QueryPerformanceFrequency(ptr %frequency.slot)
  %counter = load i64, ptr %counter.slot, align 8
  %frequency = load i64, ptr %frequency.slot, align 8
  %counter.failed = icmp eq i32 %counter.ok, 0
  %frequency.failed = icmp eq i32 %frequency.ok, 0
  %frequency.invalid = icmp sle i64 %frequency, 0
  %failed.0 = or i1 %counter.failed, %frequency.failed
  %failed = or i1 %failed.0, %frequency.invalid
  br i1 %failed, label %error, label %convert

convert:
  %whole = sdiv i64 %counter, %frequency
  %remainder = srem i64 %counter, %frequency
  %whole.ns = mul i64 %whole, 1000000000
  %remainder.scaled = mul i64 %remainder, 1000000000
  %fraction.ns = sdiv i64 %remainder.scaled, %frequency
  %result = add i64 %whole.ns, %fraction.ns
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_monotonic_millis() {
entry:
  %nanos = call i64 @ncc_rt_monotonic_nanos()
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_clock_resolution() {
entry:
  %frequency.slot = alloca i64, align 8
  %ok = call i32 @QueryPerformanceFrequency(ptr %frequency.slot)
  %frequency = load i64, ptr %frequency.slot, align 8
  %failed.0 = icmp eq i32 %ok, 0
  %failed.1 = icmp sle i64 %frequency, 0
  %failed = or i1 %failed.0, %failed.1
  br i1 %failed, label %error, label %convert

convert:
  %rounded = add i64 %frequency, 999999999
  %resolution = udiv i64 %rounded, %frequency
  ret i64 %resolution

error:
  ret i64 0
}

define void @ncc_rt_sleep_millis(i64 %millis) {
entry:
  %positive = icmp sgt i64 %millis, 0
  br i1 %positive, label %loop, label %done

loop:
  %remaining = phi i64 [ %millis, %entry ], [ %next, %loop ]
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk32 = trunc i64 %chunk64 to i32
  call void @Sleep(i32 %chunk32)
  %next = sub i64 %remaining, %chunk64
  %again = icmp ugt i64 %next, 0
  br i1 %again, label %loop, label %done

done:
  ret void
}
"#;

const POSIX64: &str = r#"
declare i32 @clock_gettime(i32, ptr)
declare i32 @clock_getres(i32, ptr)

define internal i64 @ncc_rt_posix_clock_nanos(i32 %clock) {
entry:
  %value = alloca { i64, i64 }, align 8
  %status = call i32 @clock_gettime(i32 %clock, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 0
  %nanos.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 1
  %seconds = load i64, ptr %seconds.ptr, align 8
  %nanos = load i64, ptr %nanos.ptr, align 8
  %whole = mul i64 %seconds, 1000000000
  %result = add i64 %whole, %nanos
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_now_seconds() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 0)
  %seconds = sdiv i64 %nanos, 1000000000
  ret i64 %seconds
}

define i64 @ncc_rt_now_millis() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 0)
  %millis = sdiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_monotonic_nanos() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 1)
  ret i64 %nanos
}

define i64 @ncc_rt_monotonic_millis() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 1)
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_clock_resolution() {
entry:
  %value = alloca { i64, i64 }, align 8
  %status = call i32 @clock_getres(i32 1, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 0
  %nanos.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 1
  %seconds = load i64, ptr %seconds.ptr, align 8
  %nanos = load i64, ptr %nanos.ptr, align 8
  %whole = mul i64 %seconds, 1000000000
  %result = add i64 %whole, %nanos
  ret i64 %result

error:
  ret i64 0
}

define void @ncc_rt_sleep_millis(i64 %millis) {
entry:
  %positive = icmp sgt i64 %millis, 0
  br i1 %positive, label %loop, label %done

loop:
  %remaining = phi i64 [ %millis, %entry ], [ %next, %loop ]
  %large = icmp ugt i64 %remaining, 1000
  %chunk = select i1 %large, i64 1000, i64 %remaining
  %usec64 = mul i64 %chunk, 1000
  %usec32 = trunc i64 %usec64 to i32
  %ignored = call i32 @usleep(i32 %usec32)
  %next = sub i64 %remaining, %chunk
  %again = icmp ugt i64 %next, 0
  br i1 %again, label %loop, label %done

done:
  ret void
}
"#;

const POSIX32: &str = r#"
declare i32 @clock_gettime(i32, ptr)
declare i32 @clock_getres(i32, ptr)

define internal i64 @ncc_rt_posix_clock_nanos(i32 %clock) {
entry:
  %value = alloca { i32, i32 }, align 4
  %status = call i32 @clock_gettime(i32 %clock, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i32, i32 }, ptr %value, i32 0, i32 0
  %nanos.ptr = getelementptr { i32, i32 }, ptr %value, i32 0, i32 1
  %seconds32 = load i32, ptr %seconds.ptr, align 4
  %nanos32 = load i32, ptr %nanos.ptr, align 4
  %seconds = sext i32 %seconds32 to i64
  %nanos = zext i32 %nanos32 to i64
  %whole = mul i64 %seconds, 1000000000
  %result = add i64 %whole, %nanos
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_now_seconds() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 0)
  %seconds = sdiv i64 %nanos, 1000000000
  ret i64 %seconds
}

define i64 @ncc_rt_now_millis() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 0)
  %millis = sdiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_monotonic_nanos() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 1)
  ret i64 %nanos
}

define i64 @ncc_rt_monotonic_millis() {
entry:
  %nanos = call i64 @ncc_rt_posix_clock_nanos(i32 1)
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_clock_resolution() {
entry:
  %value = alloca { i32, i32 }, align 4
  %status = call i32 @clock_getres(i32 1, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i32, i32 }, ptr %value, i32 0, i32 0
  %nanos.ptr = getelementptr { i32, i32 }, ptr %value, i32 0, i32 1
  %seconds32 = load i32, ptr %seconds.ptr, align 4
  %nanos32 = load i32, ptr %nanos.ptr, align 4
  %seconds = zext i32 %seconds32 to i64
  %nanos = zext i32 %nanos32 to i64
  %whole = mul i64 %seconds, 1000000000
  %result = add i64 %whole, %nanos
  ret i64 %result

error:
  ret i64 0
}

define void @ncc_rt_sleep_millis(i64 %millis) {
entry:
  %positive = icmp sgt i64 %millis, 0
  br i1 %positive, label %loop, label %done

loop:
  %remaining = phi i64 [ %millis, %entry ], [ %next, %loop ]
  %large = icmp ugt i64 %remaining, 1000
  %chunk = select i1 %large, i64 1000, i64 %remaining
  %usec64 = mul i64 %chunk, 1000
  %usec32 = trunc i64 %usec64 to i32
  %ignored = call i32 @usleep(i32 %usec32)
  %next = sub i64 %remaining, %chunk
  %again = icmp ugt i64 %next, 0
  br i1 %again, label %loop, label %done

done:
  ret void
}
"#;

const DARWIN: &str = r#"
declare i32 @gettimeofday(ptr, ptr)
declare i64 @mach_absolute_time()
declare i32 @mach_timebase_info(ptr)

define i64 @ncc_rt_now_seconds() {
entry:
  %value = alloca { i64, i64 }, align 8
  %status = call i32 @gettimeofday(ptr %value, ptr null)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 0
  %seconds = load i64, ptr %seconds.ptr, align 8
  ret i64 %seconds

error:
  ret i64 0
}

define i64 @ncc_rt_now_millis() {
entry:
  %value = alloca { i64, i64 }, align 8
  %status = call i32 @gettimeofday(ptr %value, ptr null)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %seconds.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 0
  %micros.ptr = getelementptr { i64, i64 }, ptr %value, i32 0, i32 1
  %seconds = load i64, ptr %seconds.ptr, align 8
  %micros = load i64, ptr %micros.ptr, align 8
  %whole = mul i64 %seconds, 1000
  %fraction = sdiv i64 %micros, 1000
  %result = add i64 %whole, %fraction
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_monotonic_nanos() {
entry:
  %scale = alloca { i32, i32 }, align 4
  %ticks = call i64 @mach_absolute_time()
  %status = call i32 @mach_timebase_info(ptr %scale)
  %numerator.ptr = getelementptr { i32, i32 }, ptr %scale, i32 0, i32 0
  %denominator.ptr = getelementptr { i32, i32 }, ptr %scale, i32 0, i32 1
  %numerator32 = load i32, ptr %numerator.ptr, align 4
  %denominator32 = load i32, ptr %denominator.ptr, align 4
  %numerator = zext i32 %numerator32 to i64
  %denominator = zext i32 %denominator32 to i64
  %status.failed = icmp ne i32 %status, 0
  %scale.failed = icmp eq i64 %denominator, 0
  %failed = or i1 %status.failed, %scale.failed
  br i1 %failed, label %error, label %convert

convert:
  %whole = udiv i64 %ticks, %denominator
  %remainder = urem i64 %ticks, %denominator
  %whole.scaled = mul i64 %whole, %numerator
  %remainder.scaled = mul i64 %remainder, %numerator
  %fraction = udiv i64 %remainder.scaled, %denominator
  %result = add i64 %whole.scaled, %fraction
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_monotonic_millis() {
entry:
  %nanos = call i64 @ncc_rt_monotonic_nanos()
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_clock_resolution() {
entry:
  %scale = alloca { i32, i32 }, align 4
  %status = call i32 @mach_timebase_info(ptr %scale)
  %numerator.ptr = getelementptr { i32, i32 }, ptr %scale, i32 0, i32 0
  %denominator.ptr = getelementptr { i32, i32 }, ptr %scale, i32 0, i32 1
  %numerator32 = load i32, ptr %numerator.ptr, align 4
  %denominator32 = load i32, ptr %denominator.ptr, align 4
  %numerator = zext i32 %numerator32 to i64
  %denominator = zext i32 %denominator32 to i64
  %status.failed = icmp ne i32 %status, 0
  %scale.failed = icmp eq i64 %denominator, 0
  %failed = or i1 %status.failed, %scale.failed
  br i1 %failed, label %error, label %convert

convert:
  %rounded = add i64 %numerator, %denominator
  %rounded.minus.one = sub i64 %rounded, 1
  %resolution = udiv i64 %rounded.minus.one, %denominator
  ret i64 %resolution

error:
  ret i64 0
}

define void @ncc_rt_sleep_millis(i64 %millis) {
entry:
  %positive = icmp sgt i64 %millis, 0
  br i1 %positive, label %loop, label %done

loop:
  %remaining = phi i64 [ %millis, %entry ], [ %next, %loop ]
  %large = icmp ugt i64 %remaining, 1000
  %chunk = select i1 %large, i64 1000, i64 %remaining
  %usec64 = mul i64 %chunk, 1000
  %usec32 = trunc i64 %usec64 to i32
  %ignored = call i32 @usleep(i32 %usec32)
  %next = sub i64 %remaining, %chunk
  %again = icmp ugt i64 %next, 0
  br i1 %again, label %loop, label %done

done:
  ret void
}
"#;

const WASI: &str = r#"
declare i32 @__wasi_clock_time_get(i32, i64, ptr)
declare i32 @__wasi_clock_res_get(i32, ptr)
declare i32 @__wasi_poll_oneoff(ptr, ptr, i32, ptr)

define internal i64 @ncc_rt_wasi_clock_nanos(i32 %clock) {
entry:
  %value = alloca i64, align 8
  %status = call i32 @__wasi_clock_time_get(i32 %clock, i64 1, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %result = load i64, ptr %value, align 8
  ret i64 %result

error:
  ret i64 0
}

define i64 @ncc_rt_now_seconds() {
entry:
  %nanos = call i64 @ncc_rt_wasi_clock_nanos(i32 0)
  %seconds = udiv i64 %nanos, 1000000000
  ret i64 %seconds
}

define i64 @ncc_rt_now_millis() {
entry:
  %nanos = call i64 @ncc_rt_wasi_clock_nanos(i32 0)
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_monotonic_nanos() {
entry:
  %nanos = call i64 @ncc_rt_wasi_clock_nanos(i32 1)
  ret i64 %nanos
}

define i64 @ncc_rt_monotonic_millis() {
entry:
  %nanos = call i64 @ncc_rt_wasi_clock_nanos(i32 1)
  %millis = udiv i64 %nanos, 1000000
  ret i64 %millis
}

define i64 @ncc_rt_clock_resolution() {
entry:
  %value = alloca i64, align 8
  %status = call i32 @__wasi_clock_res_get(i32 1, ptr %value)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %error, label %read

read:
  %result = load i64, ptr %value, align 8
  ret i64 %result

error:
  ret i64 0
}

define void @ncc_rt_sleep_millis(i64 %millis) {
entry:
  %positive = icmp sgt i64 %millis, 0
  br i1 %positive, label %wait, label %done

wait:
  %subscription = alloca [48 x i8], align 8
  %event = alloca [32 x i8], align 8
  %events = alloca i32, align 4
  call void @llvm.memset.p0.i64(ptr %subscription, i8 0, i64 48, i1 false)
  %event.type = getelementptr i8, ptr %subscription, i64 8
  store i8 0, ptr %event.type, align 1
  %clock.id = getelementptr i8, ptr %subscription, i64 16
  store i32 1, ptr %clock.id, align 4
  %timeout.ptr = getelementptr i8, ptr %subscription, i64 24
  %timeout = mul i64 %millis, 1000000
  store i64 %timeout, ptr %timeout.ptr, align 8
  %status = call i32 @__wasi_poll_oneoff(ptr %subscription, ptr %event, i32 1, ptr %events)
  br label %done

done:
  ret void
}
"#;

const COMMON_CLOCK: &str = r#"
define i64 @ncc_rt_clock_monotonic() {
entry:
  %now = call i64 @ncc_rt_monotonic_nanos()
  ret i64 %now
}

define i64 @ncc_rt_clock_elapsed(i64 %start) {
entry:
  %now = call i64 @ncc_rt_clock_monotonic()
  %ordered = icmp uge i64 %now, %start
  %delta = sub i64 %now, %start
  %result = select i1 %ordered, i64 %delta, i64 0
  ret i64 %result
}

define i64 @ncc_rt_clock_measure(ptr %function) {
entry:
  %missing = icmp eq ptr %function, null
  br i1 %missing, label %done, label %invoke

invoke:
  %start = call i64 @ncc_rt_clock_monotonic()
  %ignored = call i1 %function()
  %finish = call i64 @ncc_rt_clock_monotonic()
  %ordered = icmp uge i64 %finish, %start
  %delta = sub i64 %finish, %start
  %result = select i1 %ordered, i64 %delta, i64 0
  ret i64 %result

done:
  ret i64 0
}

define i32 @ncc_rt_sleep_until_millis(i64 %deadline) {
entry:
  br label %check

check:
  %now = call i64 @ncc_rt_monotonic_millis()
  %finished = icmp sge i64 %now, %deadline
  br i1 %finished, label %done, label %wait

wait:
  %remaining = sub i64 %deadline, %now
  %large = icmp sgt i64 %remaining, 1000
  %chunk = select i1 %large, i64 1000, i64 %remaining
  call void @ncc_rt_sleep_millis(i64 %chunk)
  br label %check

done:
  ret i32 1
}
"#;

const COMMON_DATE: &str = r#"
@ncc.rt.time.invalid = private unnamed_addr constant [18 x i8] c"date-out-of-range\00"

define internal i64 @ncc_rt_time_floor_div(i64 %value, i64 %divisor) {
entry:
  %quotient = sdiv i64 %value, %divisor
  %remainder = srem i64 %value, %divisor
  %negative = icmp slt i64 %remainder, 0
  %adjustment = select i1 %negative, i64 -1, i64 0
  %result = add i64 %quotient, %adjustment
  ret i64 %result
}

define internal void @ncc_rt_time_write_digit(ptr %out, i64 %offset, i64 %digit) {
entry:
  %ascii64 = add i64 %digit, 48
  %ascii8 = trunc i64 %ascii64 to i8
  %slot = getelementptr i8, ptr %out, i64 %offset
  store i8 %ascii8, ptr %slot, align 1
  ret void
}

define internal void @ncc_rt_time_write_two(ptr %out, i64 %offset, i64 %value) {
entry:
  %tens = sdiv i64 %value, 10
  %units = srem i64 %value, 10
  call void @ncc_rt_time_write_digit(ptr %out, i64 %offset, i64 %tens)
  %next = add i64 %offset, 1
  call void @ncc_rt_time_write_digit(ptr %out, i64 %next, i64 %units)
  ret void
}

define internal void @ncc_rt_time_write_four(ptr %out, i64 %offset, i64 %value) {
entry:
  %thousands = sdiv i64 %value, 1000
  %hundreds.base = sdiv i64 %value, 100
  %hundreds = srem i64 %hundreds.base, 10
  %tens.base = sdiv i64 %value, 10
  %tens = srem i64 %tens.base, 10
  %units = srem i64 %value, 10
  call void @ncc_rt_time_write_digit(ptr %out, i64 %offset, i64 %thousands)
  %offset.1 = add i64 %offset, 1
  call void @ncc_rt_time_write_digit(ptr %out, i64 %offset.1, i64 %hundreds)
  %offset.2 = add i64 %offset, 2
  call void @ncc_rt_time_write_digit(ptr %out, i64 %offset.2, i64 %tens)
  %offset.3 = add i64 %offset, 3
  call void @ncc_rt_time_write_digit(ptr %out, i64 %offset.3, i64 %units)
  ret void
}

define i64 @ncc_rt_format_time_into(i64 %seconds, ptr %out, i64 %capacity) {
entry:
  %out.missing = icmp eq ptr %out, null
  %too.small = icmp ult i64 %capacity, 24
  %invalid.buffer = or i1 %out.missing, %too.small
  br i1 %invalid.buffer, label %buffer.error, label %calculate

calculate:
  %days = call i64 @ncc_rt_time_floor_div(i64 %seconds, i64 86400)
  %days.seconds = mul i64 %days, 86400
  %day.seconds = sub i64 %seconds, %days.seconds
  %z = add i64 %days, 719468
  %z.negative = icmp slt i64 %z, 0
  %z.adjusted.value = sub i64 %z, 146096
  %z.adjusted = select i1 %z.negative, i64 %z.adjusted.value, i64 %z
  %era = sdiv i64 %z.adjusted, 146097
  %era.days = mul i64 %era, 146097
  %day.of.era = sub i64 %z, %era.days
  %leap.4 = sdiv i64 %day.of.era, 1460
  %leap.100 = sdiv i64 %day.of.era, 36524
  %leap.400 = sdiv i64 %day.of.era, 146096
  %year.numerator.0 = sub i64 %day.of.era, %leap.4
  %year.numerator.1 = add i64 %year.numerator.0, %leap.100
  %year.numerator = sub i64 %year.numerator.1, %leap.400
  %year.of.era = sdiv i64 %year.numerator, 365
  %era.years = mul i64 %era, 400
  %year.base = add i64 %year.of.era, %era.years
  %years.days.0 = mul i64 %year.of.era, 365
  %years.leap.4 = sdiv i64 %year.of.era, 4
  %years.leap.100 = sdiv i64 %year.of.era, 100
  %years.days.1 = add i64 %years.days.0, %years.leap.4
  %years.days = sub i64 %years.days.1, %years.leap.100
  %day.of.year = sub i64 %day.of.era, %years.days
  %month.numerator.0 = mul i64 %day.of.year, 5
  %month.numerator = add i64 %month.numerator.0, 2
  %month.prime = sdiv i64 %month.numerator, 153
  %month.days.0 = mul i64 %month.prime, 153
  %month.days.1 = add i64 %month.days.0, 2
  %month.days = sdiv i64 %month.days.1, 5
  %day.base = sub i64 %day.of.year, %month.days
  %day = add i64 %day.base, 1
  %spring = icmp slt i64 %month.prime, 10
  %month.adjust = select i1 %spring, i64 3, i64 -9
  %month = add i64 %month.prime, %month.adjust
  %jan.feb = icmp sle i64 %month, 2
  %year.adjust = select i1 %jan.feb, i64 1, i64 0
  %year = add i64 %year.base, %year.adjust
  %year.low = icmp slt i64 %year, 0
  %year.high = icmp sgt i64 %year, 9999
  %year.invalid = or i1 %year.low, %year.high
  br i1 %year.invalid, label %range.error, label %write

write:
  %hour = sdiv i64 %day.seconds, 3600
  %hour.seconds = srem i64 %day.seconds, 3600
  %minute = sdiv i64 %hour.seconds, 60
  %second = srem i64 %hour.seconds, 60
  call void @ncc_rt_time_write_four(ptr %out, i64 0, i64 %year)
  %dash.0 = getelementptr i8, ptr %out, i64 4
  store i8 45, ptr %dash.0, align 1
  call void @ncc_rt_time_write_two(ptr %out, i64 5, i64 %month)
  %dash.1 = getelementptr i8, ptr %out, i64 7
  store i8 45, ptr %dash.1, align 1
  call void @ncc_rt_time_write_two(ptr %out, i64 8, i64 %day)
  %space.0 = getelementptr i8, ptr %out, i64 10
  store i8 32, ptr %space.0, align 1
  call void @ncc_rt_time_write_two(ptr %out, i64 11, i64 %hour)
  %colon.0 = getelementptr i8, ptr %out, i64 13
  store i8 58, ptr %colon.0, align 1
  call void @ncc_rt_time_write_two(ptr %out, i64 14, i64 %minute)
  %colon.1 = getelementptr i8, ptr %out, i64 16
  store i8 58, ptr %colon.1, align 1
  call void @ncc_rt_time_write_two(ptr %out, i64 17, i64 %second)
  %space.1 = getelementptr i8, ptr %out, i64 19
  store i8 32, ptr %space.1, align 1
  %utc.0 = getelementptr i8, ptr %out, i64 20
  store i8 85, ptr %utc.0, align 1
  %utc.1 = getelementptr i8, ptr %out, i64 21
  store i8 84, ptr %utc.1, align 1
  %utc.2 = getelementptr i8, ptr %out, i64 22
  store i8 67, ptr %utc.2, align 1
  %nul = getelementptr i8, ptr %out, i64 23
  store i8 0, ptr %nul, align 1
  ret i64 23

range.error:
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr @ncc.rt.time.invalid, i64 18, i1 false)
  ret i64 17

buffer.error:
  ret i64 -1
}

define i64 @ncc_rt_date_into(ptr %out, i64 %capacity) {
entry:
  %now = call i64 @ncc_rt_now_seconds()
  %written = call i64 @ncc_rt_format_time_into(i64 %now, ptr %out, i64 %capacity)
  ret i64 %written
}
"#;

const COMMON_DATE_COMPATIBILITY: &str = r#"
define ptr @ncc_rt_format_time(i64 %seconds) {
entry:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %missing = icmp eq ptr %out, null
  br i1 %missing, label %error, label %format

format:
  %written = call i64 @ncc_rt_format_time_into(i64 %seconds, ptr %out, i64 24)
  %failed = icmp slt i64 %written, 0
  br i1 %failed, label %release, label %done

release:
  call void @__ncc_raw_aligned_free(ptr %out)
  br label %error

done:
  ret ptr %out

error:
  ret ptr null
}

define ptr @ncc_rt_date() {
entry:
  %now = call i64 @ncc_rt_now_seconds()
  %out = call ptr @ncc_rt_format_time(i64 %now)
  ret ptr %out
}

define i32 @ncc_rt_time_release(ptr %value) {
entry:
  %missing = icmp eq ptr %value, null
  br i1 %missing, label %error, label %release

release:
  call void @__ncc_raw_aligned_free(ptr %value)
  ret i32 1

error:
  ret i32 0
}
"#;

#[cfg(test)]
mod tests {
    use super::render_time_clock_runtime;

    #[test]
    fn target_renderers_use_native_os_abis_without_c_provider() {
        let windows = render_time_clock_runtime("x86_64-pc-windows-msvc", true, true).unwrap();
        assert!(windows.contains("@GetSystemTimeAsFileTime"));
        assert!(windows.contains("@QueryPerformanceCounter"));
        assert!(windows.contains("@Sleep"));

        let windows32 = render_time_clock_runtime("i686-pc-windows-msvc", false, false).unwrap();
        assert!(windows32.contains("x86_stdcallcc void @\"\\01_Sleep@4\""));
        assert!(windows32.contains("x86_stdcallcc i32 @\"\\01_QueryPerformanceCounter@4\""));
        assert!(!windows32.contains("declare void @Sleep"));

        let linux = render_time_clock_runtime("x86_64-unknown-linux-gnu", false, false).unwrap();
        assert!(linux.contains("@clock_gettime"));
        assert!(linux.contains("@usleep"));

        let linux32 = render_time_clock_runtime("i386-unknown-linux-gnu", false, false).unwrap();
        assert!(linux32.contains("alloca { i32, i32 }"));

        let darwin = render_time_clock_runtime("aarch64-apple-darwin", false, false).unwrap();
        assert!(darwin.contains("@mach_absolute_time"));

        let wasi = render_time_clock_runtime("wasm32-wasi", false, false).unwrap();
        assert!(wasi.contains("@__wasi_clock_time_get"));
        assert!(wasi.contains("@__wasi_poll_oneoff"));

        for rendered in [windows, windows32, linux, linux32, darwin, wasi] {
            assert!(rendered.contains("define i64 @ncc_rt_clock_monotonic"));
            assert!(rendered.contains("define void @ncc_rt_sleep_millis"));
            assert!(!rendered.contains("@malloc"));
            assert!(!rendered.contains("@free("));
        }
    }

    #[test]
    fn date_compatibility_is_optional_and_uses_raw_allocator() {
        let raw = render_time_clock_runtime("x86_64-pc-windows-msvc", true, false).unwrap();
        assert!(raw.contains("define i64 @ncc_rt_format_time_into"));
        assert!(!raw.contains("define ptr @ncc_rt_format_time"));

        let compatibility =
            render_time_clock_runtime("x86_64-pc-windows-msvc", true, true).unwrap();
        assert!(compatibility.contains("define ptr @ncc_rt_format_time"));
        assert!(compatibility.contains("@__ncc_raw_aligned_alloc(i64 24, i64 8)"));
        assert!(compatibility.contains("define i32 @ncc_rt_time_release"));
    }
}
