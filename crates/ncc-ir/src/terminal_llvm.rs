pub(crate) fn render_terminal_runtime(target: &str) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let platform = if target.contains("windows")
        && (target.contains("x86_64") || target.contains("aarch64"))
    {
        WINDOWS
    } else if target.contains("darwin") || target.contains("apple") {
        DARWIN
    } else if target.contains("wasi") || target.contains("wasm32") {
        WASI
    } else if target.contains("linux") || target.contains("android") {
        if target.contains("i386")
            || target.contains("i486")
            || target.contains("i586")
            || target.contains("i686")
            || target.contains("armv7")
            || target.contains("thumbv7")
            || target.contains("riscv32")
        {
            POSIX32
        } else {
            POSIX64
        }
    } else {
        return Err(format!(
            "terminal has no direct operating-system ABI for target '{target}'"
        ));
    };

    Ok(format!("{platform}\n{COMMON_ANSI}"))
}

const WINDOWS: &str = r#"
@ncc.terminal.saved.handle = internal global ptr null, align 8
@ncc.terminal.saved.mode = internal global i32 0, align 4
@ncc.terminal.saved.valid = internal global i1 false, align 1

declare ptr @GetStdHandle(i32)
declare i32 @GetConsoleMode(ptr, ptr)
declare i32 @SetConsoleMode(ptr, i32)
declare i32 @WriteFile(ptr, ptr, i32, ptr, ptr)
declare i32 @GetNumberOfConsoleInputEvents(ptr, ptr)
declare i32 @ReadConsoleInputW(ptr, ptr, i32, ptr)

define internal ptr @ncc_rt_terminal_output_handle() {
entry:
  %handle = call ptr @GetStdHandle(i32 -11)
  %null = icmp eq ptr %handle, null
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  br i1 %bad, label %failure, label %prepare

prepare:
  %mode.slot = alloca i32, align 4
  %mode.ok = call i32 @GetConsoleMode(ptr %handle, ptr %mode.slot)
  %is.console = icmp ne i32 %mode.ok, 0
  br i1 %is.console, label %enable.vt, label %success

enable.vt:
  %mode = load i32, ptr %mode.slot, align 4
  %vt.mode = or i32 %mode, 4
  %ignored = call i32 @SetConsoleMode(ptr %handle, i32 %vt.mode)
  br label %success

success:
  ret ptr %handle

failure:
  ret ptr null
}

define internal i32 @ncc_rt_terminal_write(ptr %data, i64 %length) {
entry:
  %handle = call ptr @ncc_rt_terminal_output_handle()
  %bad = icmp eq ptr %handle, null
  %empty = icmp eq i64 %length, 0
  br i1 %bad, label %failure, label %start

start:
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

define i32 @ncc_rt_terminal_enter_raw_mode() {
entry:
  %handle = call ptr @GetStdHandle(i32 -10)
  %null = icmp eq ptr %handle, null
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  br i1 %bad, label %failure, label %read

read:
  %mode.slot = alloca i32, align 4
  %status = call i32 @GetConsoleMode(ptr %handle, ptr %mode.slot)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %save, label %failure

save:
  %mode = load i32, ptr %mode.slot, align 4
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %apply, label %save.first

save.first:
  store ptr %handle, ptr @ncc.terminal.saved.handle, align 8
  store i32 %mode, ptr @ncc.terminal.saved.mode, align 4
  store i1 true, ptr @ncc.terminal.saved.valid, align 1
  br label %apply

apply:
  %raw.mode = and i32 %mode, -7
  %applied = call i32 @SetConsoleMode(ptr %handle, i32 %raw.mode)
  %result = icmp ne i32 %applied, 0
  %value = zext i1 %result to i32
  ret i32 %value

failure:
  ret i32 0
}

define i32 @ncc_rt_terminal_exit_raw_mode() {
entry:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %restore, label %success

restore:
  %handle = load ptr, ptr @ncc.terminal.saved.handle, align 8
  %mode = load i32, ptr @ncc.terminal.saved.mode, align 4
  %status = call i32 @SetConsoleMode(ptr %handle, i32 %mode)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %clear, label %failure

clear:
  store i1 false, ptr @ncc.terminal.saved.valid, align 1
  br label %success

success:
  ret i32 1

failure:
  ret i32 0
}

define i64 @ncc_rt_terminal_read_key_nonblocking() {
entry:
  %handle = call ptr @GetStdHandle(i32 -10)
  %null = icmp eq ptr %handle, null
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  br i1 %bad, label %none, label %probe

probe:
  %available.slot = alloca i32, align 4
  %probe.status = call i32 @GetNumberOfConsoleInputEvents(ptr %handle, ptr %available.slot)
  %available = load i32, ptr %available.slot, align 4
  %probe.ok = icmp ne i32 %probe.status, 0
  %has.events = icmp ugt i32 %available, 0
  %can.read = and i1 %probe.ok, %has.events
  br i1 %can.read, label %consume, label %none

consume:
  %record = alloca [20 x i8], align 4
  %read.slot = alloca i32, align 4
  %read.status = call i32 @ReadConsoleInputW(ptr %handle, ptr %record, i32 1, ptr %read.slot)
  %read.count = load i32, ptr %read.slot, align 4
  %read.ok.0 = icmp ne i32 %read.status, 0
  %read.ok.1 = icmp eq i32 %read.count, 1
  %read.ok = and i1 %read.ok.0, %read.ok.1
  br i1 %read.ok, label %inspect, label %none

inspect:
  %event.type.ptr = getelementptr i8, ptr %record, i64 0
  %event.type = load i16, ptr %event.type.ptr, align 2
  %is.key = icmp eq i16 %event.type, 1
  br i1 %is.key, label %key, label %probe

key:
  %down.ptr = getelementptr i8, ptr %record, i64 4
  %down = load i32, ptr %down.ptr, align 4
  %is.down = icmp ne i32 %down, 0
  br i1 %is.down, label %decode, label %probe

decode:
  %char.ptr = getelementptr i8, ptr %record, i64 14
  %char = load i16, ptr %char.ptr, align 2
  %has.char = icmp ne i16 %char, 0
  br i1 %has.char, label %return.char, label %return.key

return.char:
  %char64 = zext i16 %char to i64
  ret i64 %char64

return.key:
  %key.ptr = getelementptr i8, ptr %record, i64 10
  %virtual.key = load i16, ptr %key.ptr, align 2
  %key64 = zext i16 %virtual.key to i64
  ret i64 %key64

none:
  ret i64 -1
}
"#;

const POSIX64: &str = r#"
%ncc.terminal.termios = type { i32, i32, i32, i32, i8, [32 x i8], i32, i32 }
%ncc.terminal.pollfd = type { i32, i16, i16 }
@ncc.terminal.saved.termios = internal global %ncc.terminal.termios zeroinitializer, align 4
@ncc.terminal.saved.valid = internal global i1 false, align 1

declare i64 @write(i32, ptr, i64)
declare i64 @read(i32, ptr, i64)
declare i32 @poll(ptr, i64, i32)
declare i32 @tcgetattr(i32, ptr)
declare i32 @tcsetattr(i32, i32, ptr)

define internal i32 @ncc_rt_terminal_write(ptr %data, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %written.ok ]
  %remaining = sub i64 %length, %offset
  %source = getelementptr i8, ptr %data, i64 %offset
  %written = call i64 @write(i32 1, ptr %source, i64 %remaining)
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

define i32 @ncc_rt_terminal_enter_raw_mode() {
entry:
  %mode = alloca %ncc.terminal.termios, align 4
  %status = call i32 @tcgetattr(i32 0, ptr %mode)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %save, label %failure
save:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %configure, label %save.first
save.first:
  %snapshot = load %ncc.terminal.termios, ptr %mode, align 4
  store %ncc.terminal.termios %snapshot, ptr @ncc.terminal.saved.termios, align 4
  store i1 true, ptr @ncc.terminal.saved.valid, align 1
  br label %configure
configure:
  %flags.ptr = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 3
  %flags = load i32, ptr %flags.ptr, align 4
  %raw.flags = and i32 %flags, -11
  store i32 %raw.flags, ptr %flags.ptr, align 4
  %chars = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 5
  %vmin = getelementptr [32 x i8], ptr %chars, i32 0, i32 6
  %vtime = getelementptr [32 x i8], ptr %chars, i32 0, i32 5
  store i8 0, ptr %vmin, align 1
  store i8 0, ptr %vtime, align 1
  %applied = call i32 @tcsetattr(i32 0, i32 0, ptr %mode)
  %result = icmp eq i32 %applied, 0
  %value = zext i1 %result to i32
  ret i32 %value
failure:
  ret i32 0
}

define i32 @ncc_rt_terminal_exit_raw_mode() {
entry:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %restore, label %success
restore:
  %status = call i32 @tcsetattr(i32 0, i32 0, ptr @ncc.terminal.saved.termios)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %clear, label %failure
clear:
  store i1 false, ptr @ncc.terminal.saved.valid, align 1
  br label %success
success:
  ret i32 1
failure:
  ret i32 0
}

define i64 @ncc_rt_terminal_read_key_nonblocking() {
entry:
  %descriptor = alloca %ncc.terminal.pollfd, align 4
  %fd = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 0
  %events = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 1
  %returned = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 2
  store i32 0, ptr %fd, align 4
  store i16 1, ptr %events, align 2
  store i16 0, ptr %returned, align 2
  %polled = call i32 @poll(ptr %descriptor, i64 1, i32 0)
  %ready = icmp sgt i32 %polled, 0
  br i1 %ready, label %inspect, label %none
inspect:
  %mask = load i16, ptr %returned, align 2
  %input = and i16 %mask, 1
  %has.input = icmp ne i16 %input, 0
  br i1 %has.input, label %consume, label %none
consume:
  %value = alloca i8, align 1
  %count = call i64 @read(i32 0, ptr %value, i64 1)
  %ok = icmp eq i64 %count, 1
  br i1 %ok, label %result, label %none
result:
  %byte = load i8, ptr %value, align 1
  %byte64 = zext i8 %byte to i64
  ret i64 %byte64
none:
  ret i64 -1
}
"#;

const POSIX32: &str = r#"
%ncc.terminal.termios = type { i32, i32, i32, i32, i8, [32 x i8], i32, i32 }
%ncc.terminal.pollfd = type { i32, i16, i16 }
@ncc.terminal.saved.termios = internal global %ncc.terminal.termios zeroinitializer, align 4
@ncc.terminal.saved.valid = internal global i1 false, align 1

declare i32 @write(i32, ptr, i32)
declare i32 @read(i32, ptr, i32)
declare i32 @poll(ptr, i32, i32)
declare i32 @tcgetattr(i32, ptr)
declare i32 @tcsetattr(i32, i32, ptr)

define internal i32 @ncc_rt_terminal_write(ptr %data, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %written.ok ]
  %remaining = sub i64 %length, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %source = getelementptr i8, ptr %data, i64 %offset
  %written32 = call i32 @write(i32 1, ptr %source, i32 %chunk)
  %ok = icmp sgt i32 %written32, 0
  br i1 %ok, label %written.ok, label %failure
written.ok:
  %written = sext i32 %written32 to i64
  %next = add i64 %offset, %written
  %done = icmp uge i64 %next, %length
  br i1 %done, label %success, label %loop
success:
  ret i32 1
failure:
  ret i32 0
}

define i32 @ncc_rt_terminal_enter_raw_mode() {
entry:
  %mode = alloca %ncc.terminal.termios, align 4
  %status = call i32 @tcgetattr(i32 0, ptr %mode)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %save, label %failure
save:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %configure, label %save.first
save.first:
  %snapshot = load %ncc.terminal.termios, ptr %mode, align 4
  store %ncc.terminal.termios %snapshot, ptr @ncc.terminal.saved.termios, align 4
  store i1 true, ptr @ncc.terminal.saved.valid, align 1
  br label %configure
configure:
  %flags.ptr = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 3
  %flags = load i32, ptr %flags.ptr, align 4
  %raw.flags = and i32 %flags, -11
  store i32 %raw.flags, ptr %flags.ptr, align 4
  %chars = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 5
  %vmin = getelementptr [32 x i8], ptr %chars, i32 0, i32 6
  %vtime = getelementptr [32 x i8], ptr %chars, i32 0, i32 5
  store i8 0, ptr %vmin, align 1
  store i8 0, ptr %vtime, align 1
  %applied = call i32 @tcsetattr(i32 0, i32 0, ptr %mode)
  %result = icmp eq i32 %applied, 0
  %value = zext i1 %result to i32
  ret i32 %value
failure:
  ret i32 0
}

define i32 @ncc_rt_terminal_exit_raw_mode() {
entry:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %restore, label %success
restore:
  %status = call i32 @tcsetattr(i32 0, i32 0, ptr @ncc.terminal.saved.termios)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %clear, label %failure
clear:
  store i1 false, ptr @ncc.terminal.saved.valid, align 1
  br label %success
success:
  ret i32 1
failure:
  ret i32 0
}

define i64 @ncc_rt_terminal_read_key_nonblocking() {
entry:
  %descriptor = alloca %ncc.terminal.pollfd, align 4
  %fd = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 0
  %events = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 1
  %returned = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 2
  store i32 0, ptr %fd, align 4
  store i16 1, ptr %events, align 2
  store i16 0, ptr %returned, align 2
  %polled = call i32 @poll(ptr %descriptor, i32 1, i32 0)
  %ready = icmp sgt i32 %polled, 0
  br i1 %ready, label %inspect, label %none
inspect:
  %mask = load i16, ptr %returned, align 2
  %input = and i16 %mask, 1
  %has.input = icmp ne i16 %input, 0
  br i1 %has.input, label %consume, label %none
consume:
  %value = alloca i8, align 1
  %count = call i32 @read(i32 0, ptr %value, i32 1)
  %ok = icmp eq i32 %count, 1
  br i1 %ok, label %result, label %none
result:
  %byte = load i8, ptr %value, align 1
  %byte64 = zext i8 %byte to i64
  ret i64 %byte64
none:
  ret i64 -1
}
"#;

const DARWIN: &str = r#"
%ncc.terminal.termios = type { i64, i64, i64, i64, [20 x i8], i64, i64 }
%ncc.terminal.pollfd = type { i32, i16, i16 }
@ncc.terminal.saved.termios = internal global %ncc.terminal.termios zeroinitializer, align 8
@ncc.terminal.saved.valid = internal global i1 false, align 1

declare i64 @write(i32, ptr, i64)
declare i64 @read(i32, ptr, i64)
declare i32 @poll(ptr, i64, i32)
declare i32 @tcgetattr(i32, ptr)
declare i32 @tcsetattr(i32, i32, ptr)

define internal i32 @ncc_rt_terminal_write(ptr %data, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %loop
loop:
  %offset = phi i64 [ 0, %entry ], [ %next, %written.ok ]
  %remaining = sub i64 %length, %offset
  %source = getelementptr i8, ptr %data, i64 %offset
  %written = call i64 @write(i32 1, ptr %source, i64 %remaining)
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

define i32 @ncc_rt_terminal_enter_raw_mode() {
entry:
  %mode = alloca %ncc.terminal.termios, align 8
  %status = call i32 @tcgetattr(i32 0, ptr %mode)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %save, label %failure
save:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %configure, label %save.first
save.first:
  %snapshot = load %ncc.terminal.termios, ptr %mode, align 8
  store %ncc.terminal.termios %snapshot, ptr @ncc.terminal.saved.termios, align 8
  store i1 true, ptr @ncc.terminal.saved.valid, align 1
  br label %configure
configure:
  %flags.ptr = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 3
  %flags = load i64, ptr %flags.ptr, align 8
  %raw.flags = and i64 %flags, -265
  store i64 %raw.flags, ptr %flags.ptr, align 8
  %chars = getelementptr %ncc.terminal.termios, ptr %mode, i32 0, i32 4
  %vmin = getelementptr [20 x i8], ptr %chars, i32 0, i32 16
  %vtime = getelementptr [20 x i8], ptr %chars, i32 0, i32 17
  store i8 0, ptr %vmin, align 1
  store i8 0, ptr %vtime, align 1
  %applied = call i32 @tcsetattr(i32 0, i32 0, ptr %mode)
  %result = icmp eq i32 %applied, 0
  %value = zext i1 %result to i32
  ret i32 %value
failure:
  ret i32 0
}

define i32 @ncc_rt_terminal_exit_raw_mode() {
entry:
  %saved = load i1, ptr @ncc.terminal.saved.valid, align 1
  br i1 %saved, label %restore, label %success
restore:
  %status = call i32 @tcsetattr(i32 0, i32 0, ptr @ncc.terminal.saved.termios)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %clear, label %failure
clear:
  store i1 false, ptr @ncc.terminal.saved.valid, align 1
  br label %success
success:
  ret i32 1
failure:
  ret i32 0
}

define i64 @ncc_rt_terminal_read_key_nonblocking() {
entry:
  %descriptor = alloca %ncc.terminal.pollfd, align 4
  %fd = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 0
  %events = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 1
  %returned = getelementptr %ncc.terminal.pollfd, ptr %descriptor, i32 0, i32 2
  store i32 0, ptr %fd, align 4
  store i16 1, ptr %events, align 2
  store i16 0, ptr %returned, align 2
  %polled = call i32 @poll(ptr %descriptor, i64 1, i32 0)
  %ready = icmp sgt i32 %polled, 0
  br i1 %ready, label %inspect, label %none
inspect:
  %mask = load i16, ptr %returned, align 2
  %input = and i16 %mask, 1
  %has.input = icmp ne i16 %input, 0
  br i1 %has.input, label %consume, label %none
consume:
  %value = alloca i8, align 1
  %count = call i64 @read(i32 0, ptr %value, i64 1)
  %ok = icmp eq i64 %count, 1
  br i1 %ok, label %result, label %none
result:
  %byte = load i8, ptr %value, align 1
  %byte64 = zext i8 %byte to i64
  ret i64 %byte64
none:
  ret i64 -1
}
"#;

const WASI: &str = r#"
declare i32 @__wasi_fd_write(i32, ptr, i32, ptr)

define internal i32 @ncc_rt_terminal_write(ptr %data, i64 %length) {
entry:
  %iov = alloca { ptr, i32 }, align 4
  %written = alloca i32, align 4
  %large = icmp ugt i64 %length, 4294967295
  %chunk64 = select i1 %large, i64 4294967295, i64 %length
  %chunk = trunc i64 %chunk64 to i32
  %base.ptr = getelementptr { ptr, i32 }, ptr %iov, i32 0, i32 0
  %length.ptr = getelementptr { ptr, i32 }, ptr %iov, i32 0, i32 1
  store ptr %data, ptr %base.ptr, align 4
  store i32 %chunk, ptr %length.ptr, align 4
  %status = call i32 @__wasi_fd_write(i32 1, ptr %iov, i32 1, ptr %written)
  %ok = icmp eq i32 %status, 0
  %value = zext i1 %ok to i32
  ret i32 %value
}

define i32 @ncc_rt_terminal_enter_raw_mode() {
entry:
  ret i32 0
}

define i32 @ncc_rt_terminal_exit_raw_mode() {
entry:
  ret i32 1
}

define i64 @ncc_rt_terminal_read_key_nonblocking() {
entry:
  ret i64 -1
}
"#;

const COMMON_ANSI: &str = r#"
@ncc.terminal.clear = private unnamed_addr constant [7 x i8] c"\1B[2J\1B[H"
@ncc.terminal.hide = private unnamed_addr constant [6 x i8] c"\1B[?25l"
@ncc.terminal.show = private unnamed_addr constant [6 x i8] c"\1B[?25h"

define internal i64 @ncc_rt_terminal_append_decimal(ptr %buffer, i64 %cursor, i64 %raw.value) {
entry:
  %positive = icmp sgt i64 %raw.value, 0
  %value = select i1 %positive, i64 %raw.value, i64 1
  %digits = alloca [20 x i8], align 1
  br label %encode

encode:
  %remaining = phi i64 [ %value, %entry ], [ %next.value, %encode ]
  %count = phi i64 [ 0, %entry ], [ %next.count, %encode ]
  %digit = urem i64 %remaining, 10
  %ascii64 = add i64 %digit, 48
  %ascii = trunc i64 %ascii64 to i8
  %slot = getelementptr i8, ptr %digits, i64 %count
  store i8 %ascii, ptr %slot, align 1
  %next.value = udiv i64 %remaining, 10
  %next.count = add i64 %count, 1
  %encoded = icmp eq i64 %next.value, 0
  br i1 %encoded, label %copy, label %encode

copy:
  %index = phi i64 [ 0, %encode ], [ %next.index, %copy ]
  %reverse.base = sub i64 %next.count, 1
  %reverse = sub i64 %reverse.base, %index
  %source = getelementptr i8, ptr %digits, i64 %reverse
  %byte = load i8, ptr %source, align 1
  %destination.offset = add i64 %cursor, %index
  %destination = getelementptr i8, ptr %buffer, i64 %destination.offset
  store i8 %byte, ptr %destination, align 1
  %next.index = add i64 %index, 1
  %done = icmp eq i64 %next.index, %next.count
  br i1 %done, label %finish, label %copy

finish:
  %result = add i64 %cursor, %next.count
  ret i64 %result
}

define i32 @ncc_rt_terminal_clear() {
entry:
  %status = call i32 @ncc_rt_terminal_write(ptr @ncc.terminal.clear, i64 7)
  ret i32 %status
}

define i32 @ncc_rt_terminal_hide_cursor() {
entry:
  %status = call i32 @ncc_rt_terminal_write(ptr @ncc.terminal.hide, i64 6)
  ret i32 %status
}

define i32 @ncc_rt_terminal_show_cursor() {
entry:
  %status = call i32 @ncc_rt_terminal_write(ptr @ncc.terminal.show, i64 6)
  ret i32 %status
}

define i32 @ncc_rt_terminal_move_cursor(i64 %row, i64 %column) {
entry:
  %sequence = alloca [48 x i8], align 1
  %start.0 = getelementptr i8, ptr %sequence, i64 0
  %start.1 = getelementptr i8, ptr %sequence, i64 1
  store i8 27, ptr %start.0, align 1
  store i8 91, ptr %start.1, align 1
  %after.row = call i64 @ncc_rt_terminal_append_decimal(ptr %sequence, i64 2, i64 %row)
  %separator = getelementptr i8, ptr %sequence, i64 %after.row
  store i8 59, ptr %separator, align 1
  %column.start = add i64 %after.row, 1
  %after.column = call i64 @ncc_rt_terminal_append_decimal(ptr %sequence, i64 %column.start, i64 %column)
  %finish = getelementptr i8, ptr %sequence, i64 %after.column
  store i8 72, ptr %finish, align 1
  %length = add i64 %after.column, 1
  %status = call i32 @ncc_rt_terminal_write(ptr %sequence, i64 %length)
  ret i32 %status
}
"#;

#[cfg(test)]
mod tests {
    use super::render_terminal_runtime;

    #[test]
    fn terminal_targets_materialize_without_c_provider() {
        let windows = render_terminal_runtime("x86_64-pc-windows-msvc").unwrap();
        assert!(windows.contains("@ReadConsoleInputW"));
        assert!(windows.contains("@SetConsoleMode"));
        assert!(!windows.contains("@_kbhit"));
        assert!(!windows.contains("@_getch"));

        let linux = render_terminal_runtime("x86_64-unknown-linux-gnu").unwrap();
        assert!(linux.contains("%ncc.terminal.termios"));
        assert!(linux.contains("@tcsetattr"));

        let linux32 = render_terminal_runtime("i386-unknown-linux-gnu").unwrap();
        assert!(linux32.contains("declare i32 @write(i32, ptr, i32)"));

        let darwin = render_terminal_runtime("aarch64-apple-darwin").unwrap();
        assert!(darwin.contains("[20 x i8]"));
        assert!(darwin.contains("and i64 %flags, -265"));

        let wasi = render_terminal_runtime("wasm32-wasi").unwrap();
        assert!(wasi.contains("@__wasi_fd_write"));
        assert!(wasi.contains("define i32 @ncc_rt_terminal_enter_raw_mode"));

        for rendered in [windows, linux, linux32, darwin, wasi] {
            for symbol in [
                "ncc_rt_terminal_clear",
                "ncc_rt_terminal_move_cursor",
                "ncc_rt_terminal_hide_cursor",
                "ncc_rt_terminal_show_cursor",
                "ncc_rt_terminal_enter_raw_mode",
                "ncc_rt_terminal_exit_raw_mode",
                "ncc_rt_terminal_read_key_nonblocking",
            ] {
                assert!(
                    rendered.contains(&format!("@{symbol}(")),
                    "missing {symbol}"
                );
            }
        }
    }
}
