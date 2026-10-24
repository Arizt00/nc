pub(crate) fn render_process_thread_runtime(
    target: &str,
    explicit_environment: bool,
) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let body = if target.contains("windows") {
        render_windows_runtime(is_32_bit(&target))
    } else if target.contains("darwin") || target.contains("apple") {
        format!(
            "{COMMON}\n{}\n{POSIX_DARWIN64}",
            POSIX_COMMON.replace("@POINTER_BYTES@", "8")
        )
    } else if target.contains("linux") || target.contains("android") {
        if is_32_bit(&target) {
            format!(
                "{COMMON}\n{}\n{POSIX32}",
                POSIX_COMMON.replace("@POINTER_BYTES@", "4")
            )
        } else {
            format!(
                "{COMMON}\n{}\n{POSIX64}",
                POSIX_COMMON.replace("@POINTER_BYTES@", "8")
            )
        }
    } else {
        return Err(format!(
            "process/thread has no direct native process and thread ABI for target '{target}'"
        ));
    };
    if explicit_environment && target.contains("linux") {
        return Ok(body
            .replace("declare i32 @execvp(ptr, ptr)", "declare i32 @execvpe(ptr, ptr, ptr)")
            .replace(
                "  %ignored = call i32 @execvp(ptr %program, ptr %argv)",
                "  %environment = load ptr, ptr @__ncc_process_environ, align 1\n  %ignored = call i32 @execvpe(ptr %program, ptr %argv, ptr %environment)",
            ));
    }
    Ok(body)
}

fn render_windows_runtime(is_32_bit: bool) -> String {
    if !is_32_bit {
        return format!("{COMMON}\n{WINDOWS_TEXT}\n{WINDOWS64}");
    }

    let text = WINDOWS_TEXT.replace("align 8", "align 4");
    let abi = WINDOWS64
        .replace(
            "declare ptr @CreateThread(ptr, i64, ptr, ptr, i32, ptr)",
            "declare ptr @CreateThread(ptr, i32, ptr, ptr, i32, ptr)",
        )
        .replace(
            "  %raw = ptrtoint ptr %value to i64\n  %payload = shl i64 %raw, 1",
            "  %raw32 = ptrtoint ptr %value to i32\n  %raw = zext i32 %raw32 to i64\n  %payload = shl i64 %raw, 1",
        )
        .replace(
            "  %value = inttoptr i64 %raw to ptr",
            "  %raw32 = trunc i64 %raw to i32\n  %value = inttoptr i32 %raw32 to ptr",
        )
        .replace("[104 x i8]", "[68 x i8]")
        .replace("[24 x i8]", "[16 x i8]")
        .replace("i64 104", "i64 68")
        .replace("store i32 104", "store i32 68")
        .replace(
            "%thread.slot = getelementptr i8, ptr %info, i64 8",
            "%thread.slot = getelementptr i8, ptr %info, i64 4",
        )
        .replace(
            "@__ncc_raw_aligned_alloc(i64 8, i64 8)",
            "@__ncc_raw_aligned_alloc(i64 4, i64 4)",
        )
        .replace(
            "%function.slot = getelementptr i8, ptr %handle, i64 8",
            "%function.slot = getelementptr i8, ptr %handle, i64 4",
        )
        .replace(
            "%result.slot = getelementptr i8, ptr %handle, i64 16",
            "%result.slot = getelementptr i8, ptr %handle, i64 8",
        )
        .replace(
            "@__ncc_raw_aligned_alloc(i64 24, i64 8)",
            "@__ncc_raw_aligned_alloc(i64 16, i64 8)",
        )
        .replace(
            "@CreateThread(ptr null, i64 0,",
            "@CreateThread(ptr null, i32 0,",
        )
        .replace("align 8", "align 4");
    format!("{COMMON}\n{text}\n{abi}")
}

fn is_32_bit(target: &str) -> bool {
    target.contains("i386")
        || target.contains("i486")
        || target.contains("i586")
        || target.contains("i686")
        || target.contains("armv7")
        || target.contains("thumbv7")
        || target.contains("riscv32")
}

const COMMON: &str = r#"
define internal i64 @__ncc_process_thread_error(i64 %code) {
entry:
  %nonzero = icmp ne i64 %code, 0
  %normalized = select i1 %nonzero, i64 %code, i64 1
  %tagged = shl i64 %normalized, 1
  ret i64 %tagged
}

define internal i64 @__ncc_process_thread_ok_i64(i64 %value) {
entry:
  %payload = shl i64 %value, 1
  %tagged = or i64 %payload, 1
  ret i64 %tagged
}
"#;

const WINDOWS_TEXT: &str = r#"
@__ncc_pt_powershell = private unnamed_addr constant [15 x i8] c"powershell.exe\00"
@__ncc_pt_pwsh_exe = private unnamed_addr constant [9 x i8] c"pwsh.exe\00"
@__ncc_pt_pwsh = private unnamed_addr constant [5 x i8] c"pwsh\00"
@__ncc_pt_command = private unnamed_addr constant [9 x i8] c"-Command\00"
@__ncc_pt_ps_prefix = private unnamed_addr constant [5 x i8] c"& { \00"
@__ncc_pt_ps_suffix = private unnamed_addr constant [3 x i8] c" }\00"
@__ncc_pt_ext_exe = private unnamed_addr constant [5 x i8] c".exe\00"
@__ncc_pt_ext_com = private unnamed_addr constant [5 x i8] c".com\00"
@__ncc_pt_ext_cmd = private unnamed_addr constant [5 x i8] c".cmd\00"
@__ncc_pt_ext_bat = private unnamed_addr constant [5 x i8] c".bat\00"
@__ncc_pt_cmd_suffix = private unnamed_addr constant [5 x i8] c".cmd\00"
@__ncc_pt_bat_suffix = private unnamed_addr constant [5 x i8] c".bat\00"
@__ncc_pt_cmd_prefix = private unnamed_addr constant [21 x i8] c"\22cmd.exe\22 /D /S /C \22\00"

define internal i64 @__ncc_pt_strlen(ptr %text) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %slot = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %slot, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %result, label %body
body:
  %next = add i64 %index, 1
  br label %loop
result:
  ret i64 %index
}

define internal i8 @__ncc_pt_fold_ascii(i8 %byte) {
entry:
  %above = icmp uge i8 %byte, 65
  %below = icmp ule i8 %byte, 90
  %upper = and i1 %above, %below
  %lowered = add i8 %byte, 32
  %out = select i1 %upper, i8 %lowered, i8 %byte
  ret i8 %out
}

define internal i1 @__ncc_pt_equal_folded(ptr %left, ptr %right) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %advance ]
  %left.slot = getelementptr i8, ptr %left, i64 %index
  %right.slot = getelementptr i8, ptr %right, i64 %index
  %left.byte = load i8, ptr %left.slot, align 1
  %right.byte = load i8, ptr %right.slot, align 1
  %left.folded = call i8 @__ncc_pt_fold_ascii(i8 %left.byte)
  %right.folded = call i8 @__ncc_pt_fold_ascii(i8 %right.byte)
  %same = icmp eq i8 %left.folded, %right.folded
  br i1 %same, label %same_byte, label %different
same_byte:
  %done = icmp eq i8 %left.byte, 0
  br i1 %done, label %equal, label %advance
advance:
  %next = add i64 %index, 1
  br label %loop
equal:
  ret i1 true
different:
  ret i1 false
}

define internal i1 @__ncc_pt_ends_folded(ptr %text, ptr %suffix) {
entry:
  %text.len = call i64 @__ncc_pt_strlen(ptr %text)
  %suffix.len = call i64 @__ncc_pt_strlen(ptr %suffix)
  %long.enough = icmp uge i64 %text.len, %suffix.len
  br i1 %long.enough, label %compare, label %different
compare:
  %offset = sub i64 %text.len, %suffix.len
  %tail = getelementptr i8, ptr %text, i64 %offset
  %same = call i1 @__ncc_pt_equal_folded(ptr %tail, ptr %suffix)
  ret i1 %same
different:
  ret i1 false
}

define internal i1 @__ncc_pt_is_command_script(ptr %program) {
entry:
  %cmd = call i1 @__ncc_pt_ends_folded(ptr %program, ptr @__ncc_pt_cmd_suffix)
  %bat = call i1 @__ncc_pt_ends_folded(ptr %program, ptr @__ncc_pt_bat_suffix)
  %script = or i1 %cmd, %bat
  ret i1 %script
}

define internal i32 @__ncc_pt_resolve_program(ptr %program, ptr %resolved) {
entry:
  %exe = call i32 @SearchPathA(ptr null, ptr %program, ptr @__ncc_pt_ext_exe, i32 32768, ptr %resolved, ptr null)
  %exe.ok = icmp ugt i32 %exe, 0
  %exe.fits = icmp ult i32 %exe, 32768
  %exe.found = and i1 %exe.ok, %exe.fits
  br i1 %exe.found, label %classify, label %try.com
try.com:
  %com = call i32 @SearchPathA(ptr null, ptr %program, ptr @__ncc_pt_ext_com, i32 32768, ptr %resolved, ptr null)
  %com.ok = icmp ugt i32 %com, 0
  %com.fits = icmp ult i32 %com, 32768
  %com.found = and i1 %com.ok, %com.fits
  br i1 %com.found, label %classify, label %try.cmd
try.cmd:
  %cmd = call i32 @SearchPathA(ptr null, ptr %program, ptr @__ncc_pt_ext_cmd, i32 32768, ptr %resolved, ptr null)
  %cmd.ok = icmp ugt i32 %cmd, 0
  %cmd.fits = icmp ult i32 %cmd, 32768
  %cmd.found = and i1 %cmd.ok, %cmd.fits
  br i1 %cmd.found, label %classify, label %try.bat
try.bat:
  %bat = call i32 @SearchPathA(ptr null, ptr %program, ptr @__ncc_pt_ext_bat, i32 32768, ptr %resolved, ptr null)
  %bat.ok = icmp ugt i32 %bat, 0
  %bat.fits = icmp ult i32 %bat, 32768
  %bat.found = and i1 %bat.ok, %bat.fits
  br i1 %bat.found, label %classify, label %missing
classify:
  %script = call i1 @__ncc_pt_is_command_script(ptr %resolved)
  %kind = select i1 %script, i32 2, i32 1
  ret i32 %kind
missing:
  ret i32 0
}

define internal ptr @__ncc_pt_program_leaf(ptr %program) {
entry:
  br label %loop
loop:
  %cursor = phi ptr [ %program, %entry ], [ %next, %advance ]
  %leaf = phi ptr [ %program, %entry ], [ %leaf.next, %advance ]
  %byte = load i8, ptr %cursor, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %result, label %check
check:
  %slash = icmp eq i8 %byte, 47
  %backslash = icmp eq i8 %byte, 92
  %separator = or i1 %slash, %backslash
  %next = getelementptr i8, ptr %cursor, i64 1
  %leaf.next = select i1 %separator, ptr %next, ptr %leaf
  br label %advance
advance:
  br label %loop
result:
  ret ptr %leaf
}

define internal i1 @__ncc_pt_is_powershell(ptr %program) {
entry:
  %leaf = call ptr @__ncc_pt_program_leaf(ptr %program)
  %powershell = call i1 @__ncc_pt_equal_folded(ptr %leaf, ptr @__ncc_pt_powershell)
  %pwsh_exe = call i1 @__ncc_pt_equal_folded(ptr %leaf, ptr @__ncc_pt_pwsh_exe)
  %pwsh = call i1 @__ncc_pt_equal_folded(ptr %leaf, ptr @__ncc_pt_pwsh)
  %either = or i1 %powershell, %pwsh_exe
  %result = or i1 %either, %pwsh
  ret i1 %result
}

define internal i64 @__ncc_pt_find_command(ptr %args, i64 %argc, i1 %powershell) {
entry:
  br i1 %powershell, label %loop, label %missing
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %advance ]
  %done = icmp uge i64 %index, %argc
  br i1 %done, label %missing, label %read
read:
  %slot = getelementptr ptr, ptr %args, i64 %index
  %arg = load ptr, ptr %slot, align 8
  %match = call i1 @__ncc_pt_equal_folded(ptr %arg, ptr @__ncc_pt_command)
  br i1 %match, label %found, label %advance
advance:
  %next = add i64 %index, 1
  br label %loop
found:
  %script.index = add i64 %index, 1
  %has_script = icmp ult i64 %script.index, %argc
  %result = select i1 %has_script, i64 %index, i64 %argc
  ret i64 %result
missing:
  ret i64 %argc
}

define internal i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 %byte) {
entry:
  %slot = getelementptr i8, ptr %out, i64 %cursor
  store i8 %byte, ptr %slot, align 1
  %next = add i64 %cursor, 1
  ret i64 %next
}

define internal i64 @__ncc_pt_append_text(ptr %out, i64 %cursor.in, ptr %text) {
entry:
  br label %loop
loop:
  %cursor = phi i64 [ %cursor.in, %entry ], [ %cursor.next, %body ]
  %index = phi i64 [ 0, %entry ], [ %index.next, %body ]
  %source = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %source, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %result, label %body
body:
  %cursor.next = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 %byte)
  %index.next = add i64 %index, 1
  br label %loop
result:
  ret i64 %cursor
}

define internal i64 @__ncc_pt_append_repeat(ptr %out, i64 %cursor.in, i8 %byte, i64 %count) {
entry:
  br label %loop
loop:
  %cursor = phi i64 [ %cursor.in, %entry ], [ %cursor.next, %body ]
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %result, label %body
body:
  %cursor.next = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 %byte)
  %next = add i64 %index, 1
  br label %loop
result:
  ret i64 %cursor
}

define internal i64 @__ncc_pt_append_windows_quoted(ptr %out, i64 %cursor.in, ptr %text) {
entry:
  %opened = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor.in, i8 34)
  br label %scan
scan:
  %cursor = phi i64 [ %opened, %entry ], [ %after.char, %ordinary ], [ %after.quote, %quoted ]
  %source = phi ptr [ %text, %entry ], [ %source.next, %ordinary ], [ %quote.next, %quoted ]
  br label %slashes
slashes:
  %slash.cursor = phi ptr [ %source, %scan ], [ %slash.next, %slash.body ]
  %slash.count = phi i64 [ 0, %scan ], [ %slash.count.next, %slash.body ]
  %byte = load i8, ptr %slash.cursor, align 1
  %is.slash = icmp eq i8 %byte, 92
  br i1 %is.slash, label %slash.body, label %classify
slash.body:
  %slash.next = getelementptr i8, ptr %slash.cursor, i64 1
  %slash.count.next = add i64 %slash.count, 1
  br label %slashes
classify:
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %finish, label %quote.check
quote.check:
  %is.quote = icmp eq i8 %byte, 34
  br i1 %is.quote, label %quoted, label %ordinary
quoted:
  %double = shl i64 %slash.count, 1
  %escaped.count = add i64 %double, 1
  %escaped.cursor = call i64 @__ncc_pt_append_repeat(ptr %out, i64 %cursor, i8 92, i64 %escaped.count)
  %after.quote = call i64 @__ncc_pt_append_byte(ptr %out, i64 %escaped.cursor, i8 34)
  %quote.next = getelementptr i8, ptr %slash.cursor, i64 1
  br label %scan
ordinary:
  %slashes.done = call i64 @__ncc_pt_append_repeat(ptr %out, i64 %cursor, i8 92, i64 %slash.count)
  %after.char = call i64 @__ncc_pt_append_byte(ptr %out, i64 %slashes.done, i8 %byte)
  %source.next = getelementptr i8, ptr %slash.cursor, i64 1
  br label %scan
finish:
  %trailing.count = shl i64 %slash.count, 1
  %trailing.done = call i64 @__ncc_pt_append_repeat(ptr %out, i64 %cursor, i8 92, i64 %trailing.count)
  %closed = call i64 @__ncc_pt_append_byte(ptr %out, i64 %trailing.done, i8 34)
  ret i64 %closed
}

define internal i64 @__ncc_pt_append_single_quoted(ptr %out, i64 %cursor.in, ptr %text) {
entry:
  %opened = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor.in, i8 39)
  br label %loop
loop:
  %cursor = phi i64 [ %opened, %entry ], [ %next.cursor, %body ]
  %source = phi ptr [ %text, %entry ], [ %next.source, %body ]
  %byte = load i8, ptr %source, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %finish, label %body
body:
  %is.quote = icmp eq i8 %byte, 39
  %escaped.cursor = select i1 %is.quote, i64 1, i64 0
  %with.escape = call i64 @__ncc_pt_append_repeat(ptr %out, i64 %cursor, i8 39, i64 %escaped.cursor)
  %next.cursor = call i64 @__ncc_pt_append_byte(ptr %out, i64 %with.escape, i8 %byte)
  %next.source = getelementptr i8, ptr %source, i64 1
  br label %loop
finish:
  %closed = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 39)
  ret i64 %closed
}

define internal ptr @__ncc_pt_build_ps_expression(ptr %args, i64 %argc, i64 %command.index, i64 %capacity) {
entry:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 8)
  %ok = icmp ne ptr %out, null
  br i1 %ok, label %prefix, label %failure
prefix:
  %c0 = call i64 @__ncc_pt_append_text(ptr %out, i64 0, ptr @__ncc_pt_ps_prefix)
  %script.index = add i64 %command.index, 1
  %script.slot = getelementptr ptr, ptr %args, i64 %script.index
  %script = load ptr, ptr %script.slot, align 8
  %c1 = call i64 @__ncc_pt_append_text(ptr %out, i64 %c0, ptr %script)
  %c2 = call i64 @__ncc_pt_append_text(ptr %out, i64 %c1, ptr @__ncc_pt_ps_suffix)
  %extra.start = add i64 %command.index, 2
  br label %extra.loop
extra.loop:
  %index = phi i64 [ %extra.start, %prefix ], [ %next, %extra.body ]
  %cursor = phi i64 [ %c2, %prefix ], [ %quoted, %extra.body ]
  %done = icmp uge i64 %index, %argc
  br i1 %done, label %finish, label %extra.body
extra.body:
  %space = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 32)
  %slot = getelementptr ptr, ptr %args, i64 %index
  %arg = load ptr, ptr %slot, align 8
  %quoted = call i64 @__ncc_pt_append_single_quoted(ptr %out, i64 %space, ptr %arg)
  %next = add i64 %index, 1
  br label %extra.loop
finish:
  %nul = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 0)
  ret ptr %out
failure:
  ret ptr null
}

define internal ptr @__ncc_pt_build_windows_command(ptr %program, ptr %args, i64 %argc) {
entry:
  %program.len = call i64 @__ncc_pt_strlen(ptr %program)
  br label %measure
measure:
  %index = phi i64 [ 0, %entry ], [ %next, %measure.body ]
  %total = phi i64 [ %program.len, %entry ], [ %total.next, %measure.body ]
  %done = icmp uge i64 %index, %argc
  br i1 %done, label %allocate, label %measure.body
measure.body:
  %slot = getelementptr ptr, ptr %args, i64 %index
  %measure.arg = load ptr, ptr %slot, align 8
  %len = call i64 @__ncc_pt_strlen(ptr %measure.arg)
  %with.arg = add i64 %total, %len
  %total.next = add i64 %with.arg, 1
  %next = add i64 %index, 1
  br label %measure
allocate:
  %slack = add i64 %total, 256
  %capacity = shl i64 %slack, 2
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 8)
  %allocated = icmp ne ptr %out, null
  br i1 %allocated, label %classify, label %failure
classify:
  %powershell = call i1 @__ncc_pt_is_powershell(ptr %program)
  %command.index = call i64 @__ncc_pt_find_command(ptr %args, i64 %argc, i1 %powershell)
  %has.command = icmp ult i64 %command.index, %argc
  %regular.limit.plus = add i64 %command.index, 1
  %regular.limit = select i1 %has.command, i64 %regular.limit.plus, i64 %argc
  %program.done = call i64 @__ncc_pt_append_windows_quoted(ptr %out, i64 0, ptr %program)
  br label %args.loop
args.loop:
  %arg.index = phi i64 [ 0, %classify ], [ %arg.next, %args.body ]
  %cursor = phi i64 [ %program.done, %classify ], [ %arg.done, %args.body ]
  %args.done = icmp uge i64 %arg.index, %regular.limit
  br i1 %args.done, label %special, label %args.body
args.body:
  %space = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 32)
  %arg.slot = getelementptr ptr, ptr %args, i64 %arg.index
  %arg = load ptr, ptr %arg.slot, align 8
  %arg.done = call i64 @__ncc_pt_append_windows_quoted(ptr %out, i64 %space, ptr %arg)
  %arg.next = add i64 %arg.index, 1
  br label %args.loop
special:
  br i1 %has.command, label %special.body, label %finish
special.body:
  %expression = call ptr @__ncc_pt_build_ps_expression(ptr %args, i64 %argc, i64 %command.index, i64 %capacity)
  %expression.ok = icmp ne ptr %expression, null
  br i1 %expression.ok, label %special.quote, label %release.failure
special.quote:
  %special.space = call i64 @__ncc_pt_append_byte(ptr %out, i64 %cursor, i8 32)
  %special.done = call i64 @__ncc_pt_append_windows_quoted(ptr %out, i64 %special.space, ptr %expression)
  %expression.released = call i1 @__ncc_raw_aligned_free(ptr %expression)
  br label %finish.special
finish.special:
  br label %finish
finish:
  %final.cursor = phi i64 [ %cursor, %special ], [ %special.done, %finish.special ]
  %nul = call i64 @__ncc_pt_append_byte(ptr %out, i64 %final.cursor, i8 0)
  ret ptr %out
release.failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %out)
  br label %failure
failure:
  ret ptr null
}

define internal ptr @__ncc_pt_wrap_command_script(ptr %command) {
entry:
  %length = call i64 @__ncc_pt_strlen(ptr %command)
  %capacity = add i64 %length, 64
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 8)
  %ok = icmp ne ptr %out, null
  br i1 %ok, label %write, label %failure
write:
  %prefix = call i64 @__ncc_pt_append_text(ptr %out, i64 0, ptr @__ncc_pt_cmd_prefix)
  %body = call i64 @__ncc_pt_append_text(ptr %out, i64 %prefix, ptr %command)
  %quote = call i64 @__ncc_pt_append_byte(ptr %out, i64 %body, i8 34)
  %nul = call i64 @__ncc_pt_append_byte(ptr %out, i64 %quote, i8 0)
  ret ptr %out
failure:
  ret ptr null
}
"#;

const WINDOWS64: &str = r#"
declare i32 @CreateProcessA(ptr, ptr, ptr, ptr, i32, i32, ptr, ptr, ptr, ptr)
declare i32 @CloseHandle(ptr)
declare i32 @WaitForSingleObject(ptr, i32)
declare i32 @GetExitCodeProcess(ptr, ptr)
declare ptr @CreateThread(ptr, i64, ptr, ptr, i32, ptr)
declare i32 @SearchPathA(ptr, ptr, ptr, i32, ptr, ptr)

define internal i64 @__ncc_process_thread_ok_ptr(ptr %value) {
entry:
  %raw = ptrtoint ptr %value to i64
  %payload = shl i64 %raw, 1
  %tagged = or i64 %payload, 1
  ret i64 %tagged
}

define internal ptr @__ncc_process_thread_payload_ptr(i64 %tagged) {
entry:
  %raw = lshr i64 %tagged, 1
  %value = inttoptr i64 %raw to ptr
  ret ptr %value
}

define internal i64 @__ncc_windows_process_spawn(ptr %program, ptr %args, i64 %argc, ptr %cwd) {
entry:
  %startup = alloca [104 x i8], align 8
  %info = alloca [24 x i8], align 8
  %resolved = call ptr @__ncc_raw_aligned_alloc(i64 32768, i64 8)
  %resolved.ok = icmp ne ptr %resolved, null
  br i1 %resolved.ok, label %resolve, label %oom
resolve:
  %kind = call i32 @__ncc_pt_resolve_program(ptr %program, ptr %resolved)
  %found = icmp ne i32 %kind, 0
  %effective = select i1 %found, ptr %resolved, ptr %program
  %regular.command = call ptr @__ncc_pt_build_windows_command(ptr %effective, ptr %args, i64 %argc)
  %regular.ok = icmp ne ptr %regular.command, null
  br i1 %regular.ok, label %classify.command, label %release.resolved.oom
classify.command:
  %is.script = icmp eq i32 %kind, 2
  br i1 %is.script, label %wrap.script, label %prepare.regular
wrap.script:
  %script.command = call ptr @__ncc_pt_wrap_command_script(ptr %regular.command)
  %script.ok = icmp ne ptr %script.command, null
  br i1 %script.ok, label %prepare.script, label %release.regular.oom
prepare.regular:
  br label %prepare
prepare.script:
  br label %prepare
prepare:
  %command = phi ptr [ %regular.command, %prepare.regular ], [ %script.command, %prepare.script ]
  call void @llvm.memset.p0.i64(ptr %startup, i8 0, i64 104, i1 false)
  call void @llvm.memset.p0.i64(ptr %info, i8 0, i64 24, i1 false)
  store i32 104, ptr %startup, align 8
  %created = call i32 @CreateProcessA(ptr null, ptr %command, ptr null, ptr null, i32 0, i32 0, ptr null, ptr %cwd, ptr %startup, ptr %info)
  %command.released = call i1 @__ncc_raw_aligned_free(ptr %command)
  %regular.release.required = icmp ne ptr %command, %regular.command
  br i1 %regular.release.required, label %release.regular, label %release.resolved
release.regular:
  %regular.released = call i1 @__ncc_raw_aligned_free(ptr %regular.command)
  br label %release.resolved
release.resolved:
  %resolved.released = call i1 @__ncc_raw_aligned_free(ptr %resolved)
  br label %created.check
created.check:
  %created.ok = icmp ne i32 %created, 0
  br i1 %created.ok, label %record, label %create.error
record:
  %process = load ptr, ptr %info, align 8
  %thread.slot = getelementptr i8, ptr %info, i64 8
  %thread = load ptr, ptr %thread.slot, align 8
  %thread.closed = call i32 @CloseHandle(ptr %thread)
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 8, i64 8)
  %handle.ok = icmp ne ptr %handle, null
  br i1 %handle.ok, label %return.handle, label %handle.error
return.handle:
  store ptr %process, ptr %handle, align 8
  %tagged = call i64 @__ncc_process_thread_ok_ptr(ptr %handle)
  ret i64 %tagged
handle.error:
  %process.closed = call i32 @CloseHandle(ptr %process)
  %err3 = call i64 @__ncc_process_thread_error(i64 3)
  ret i64 %err3
create.error:
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
release.regular.oom:
  %regular.released.oom = call i1 @__ncc_raw_aligned_free(ptr %regular.command)
  br label %release.resolved.oom
release.resolved.oom:
  %resolved.released.oom = call i1 @__ncc_raw_aligned_free(ptr %resolved)
  br label %oom
oom:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}

define i64 @ncc_rt_process_spawn(ptr %program, ptr %args, i64 %argc) {
entry:
  %result = call i64 @__ncc_windows_process_spawn(ptr %program, ptr %args, i64 %argc, ptr null)
  ret i64 %result
}

define i64 @ncc_rt_process_spawn_in(ptr %program, ptr %args, i64 %argc, ptr %cwd) {
entry:
  %result = call i64 @__ncc_windows_process_spawn(ptr %program, ptr %args, i64 %argc, ptr %cwd)
  ret i64 %result
}

define i64 @ncc_rt_process_wait(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %wait, label %bad
wait:
  %handle = call ptr @__ncc_process_thread_payload_ptr(i64 %tagged)
  %process = load ptr, ptr %handle, align 8
  %status.slot = alloca i32, align 4
  %waited = call i32 @WaitForSingleObject(ptr %process, i32 -1)
  %wait.ok = icmp ne i32 %waited, -1
  br i1 %wait.ok, label %read, label %failed
read:
  %read.ok32 = call i32 @GetExitCodeProcess(ptr %process, ptr %status.slot)
  %read.ok = icmp ne i32 %read.ok32, 0
  br i1 %read.ok, label %success, label %failed
success:
  %status32 = load i32, ptr %status.slot, align 4
  %status = zext i32 %status32 to i64
  %closed = call i32 @CloseHandle(ptr %process)
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %result = call i64 @__ncc_process_thread_ok_i64(i64 %status)
  ret i64 %result
failed:
  %closed.failed = call i32 @CloseHandle(ptr %process)
  %released.failed = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
bad:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}

define internal i32 @__ncc_windows_thread_start(ptr %handle) {
entry:
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %function = load ptr, ptr %function.slot, align 8
  %result = call i64 %function()
  store i64 %result, ptr %result.slot, align 8
  ret i32 0
}

define i64 @ncc_rt_thread_spawn(ptr %function) {
entry:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %create, label %oom
create:
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  store ptr %function, ptr %function.slot, align 8
  store i64 0, ptr %result.slot, align 8
  %thread = call ptr @CreateThread(ptr null, i64 0, ptr @__ncc_windows_thread_start, ptr %handle, i32 0, ptr null)
  %created = icmp ne ptr %thread, null
  br i1 %created, label %success, label %failure
success:
  store ptr %thread, ptr %handle, align 8
  %tagged = call i64 @__ncc_process_thread_ok_ptr(ptr %handle)
  ret i64 %tagged
failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
oom:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}

define i64 @ncc_rt_thread_join(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %wait, label %bad
wait:
  %handle = call ptr @__ncc_process_thread_payload_ptr(i64 %tagged)
  %thread = load ptr, ptr %handle, align 8
  %waited = call i32 @WaitForSingleObject(ptr %thread, i32 -1)
  %wait.ok = icmp ne i32 %waited, -1
  br i1 %wait.ok, label %success, label %failed
success:
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %result = load i64, ptr %result.slot, align 8
  %closed = call i32 @CloseHandle(ptr %thread)
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %tagged.result = call i64 @__ncc_process_thread_ok_i64(i64 %result)
  ret i64 %tagged.result
failed:
  %closed.failed = call i32 @CloseHandle(ptr %thread)
  %released.failed = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
bad:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}
"#;

const POSIX_COMMON: &str = r#"
declare i32 @fork()
declare i32 @execvp(ptr, ptr)
declare i32 @chdir(ptr)
declare void @_exit(i32)
declare i32 @waitpid(i32, ptr, i32)

define internal i64 @__ncc_posix_process_spawn(ptr %program, ptr %args, i64 %argc, ptr %cwd, i64 %pointer.bytes) {
entry:
  %slots = add i64 %argc, 2
  %bytes = mul i64 %slots, %pointer.bytes
  %argv = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 %pointer.bytes)
  %argv.ok = icmp ne ptr %argv, null
  br i1 %argv.ok, label %initialize, label %oom
initialize:
  store ptr %program, ptr %argv, align 4
  br label %copy
copy:
  %index = phi i64 [ 0, %initialize ], [ %next, %copy.body ]
  %done = icmp uge i64 %index, %argc
  br i1 %done, label %terminate, label %copy.body
copy.body:
  %source.slot = getelementptr ptr, ptr %args, i64 %index
  %value = load ptr, ptr %source.slot, align 4
  %destination.index = add i64 %index, 1
  %destination.slot = getelementptr ptr, ptr %argv, i64 %destination.index
  store ptr %value, ptr %destination.slot, align 4
  %next = add i64 %index, 1
  br label %copy
terminate:
  %null.index = add i64 %argc, 1
  %null.slot = getelementptr ptr, ptr %argv, i64 %null.index
  store ptr null, ptr %null.slot, align 4
  %pid = call i32 @fork()
  %child = icmp eq i32 %pid, 0
  br i1 %child, label %cwd.check, label %parent
cwd.check:
  %cwd.present = icmp ne ptr %cwd, null
  br i1 %cwd.present, label %cwd.change, label %exec
cwd.change:
  %cwd.status = call i32 @chdir(ptr %cwd)
  %cwd.ok = icmp eq i32 %cwd.status, 0
  br i1 %cwd.ok, label %exec, label %cwd.failure
cwd.failure:
  call void @_exit(i32 126)
  unreachable
exec:
  %ignored = call i32 @execvp(ptr %program, ptr %argv)
  call void @_exit(i32 127)
  unreachable
parent:
  %released.argv = call i1 @__ncc_raw_aligned_free(ptr %argv)
  %failed = icmp slt i32 %pid, 0
  br i1 %failed, label %fork.error, label %record
record:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 8, i64 8)
  %handle.ok = icmp ne ptr %handle, null
  br i1 %handle.ok, label %success, label %handle.error
success:
  store i32 %pid, ptr %handle, align 4
  %raw = ptrtoint ptr %handle to i64
  %shifted = shl i64 %raw, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
handle.error:
  %err3 = call i64 @__ncc_process_thread_error(i64 3)
  ret i64 %err3
fork.error:
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
oom:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}

define i64 @ncc_rt_process_spawn(ptr %program, ptr %args, i64 %argc) {
entry:
  %result = call i64 @__ncc_posix_process_spawn(ptr %program, ptr %args, i64 %argc, ptr null, i64 @POINTER_BYTES@)
  ret i64 %result
}

define i64 @ncc_rt_process_spawn_in(ptr %program, ptr %args, i64 %argc, ptr %cwd) {
entry:
  %result = call i64 @__ncc_posix_process_spawn(ptr %program, ptr %args, i64 %argc, ptr %cwd, i64 @POINTER_BYTES@)
  ret i64 %result
}

define i64 @ncc_rt_process_wait(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %wait, label %bad
wait:
  %raw = lshr i64 %tagged, 1
  %handle = inttoptr i64 %raw to ptr
  %pid = load i32, ptr %handle, align 4
  %status.slot = alloca i32, align 4
  store i32 0, ptr %status.slot, align 4
  %waited = call i32 @waitpid(i32 %pid, ptr %status.slot, i32 0)
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %ok = icmp sge i32 %waited, 0
  br i1 %ok, label %decode, label %failed
decode:
  %status = load i32, ptr %status.slot, align 4
  %signal = and i32 %status, 127
  %exited = icmp eq i32 %signal, 0
  %exit.shift = lshr i32 %status, 8
  %exit.code = and i32 %exit.shift, 255
  %signal.code = add i32 %signal, 128
  %code32 = select i1 %exited, i32 %exit.code, i32 %signal.code
  %code = sext i32 %code32 to i64
  %result = call i64 @__ncc_process_thread_ok_i64(i64 %code)
  ret i64 %result
failed:
  %err2 = call i64 @__ncc_process_thread_error(i64 2)
  ret i64 %err2
bad:
  %err1 = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %err1
}
"#;

const POSIX64: &str = r#"
declare i32 @pthread_create(ptr, ptr, ptr, ptr)
declare i32 @pthread_join(i64, ptr)

define internal ptr @__ncc_posix_thread_start(ptr %handle) {
entry:
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %function = load ptr, ptr %function.slot, align 8
  %result = call i64 %function()
  store i64 %result, ptr %result.slot, align 8
  ret ptr %handle
}

define i64 @ncc_rt_thread_spawn(ptr %function) {
entry:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %create, label %oom
create:
  %thread.slot = getelementptr i8, ptr %handle, i64 0
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  store ptr %function, ptr %function.slot, align 8
  store i64 0, ptr %result.slot, align 8
  %status = call i32 @pthread_create(ptr %thread.slot, ptr null, ptr @__ncc_posix_thread_start, ptr %handle)
  %created = icmp eq i32 %status, 0
  br i1 %created, label %success, label %failure
success:
  %raw = ptrtoint ptr %handle to i64
  %shifted = shl i64 %raw, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
oom:
  %error.oom = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.oom
}

define i64 @ncc_rt_thread_join(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %join, label %bad
join:
  %raw = lshr i64 %tagged, 1
  %handle = inttoptr i64 %raw to ptr
  %thread = load i64, ptr %handle, align 8
  %status = call i32 @pthread_join(i64 %thread, ptr null)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %result = load i64, ptr %result.slot, align 8
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %tagged.result = call i64 @__ncc_process_thread_ok_i64(i64 %result)
  ret i64 %tagged.result
failed:
  %released.failed = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
bad:
  %error.bad = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.bad
}
"#;

const POSIX_DARWIN64: &str = r#"
declare i32 @pthread_create(ptr, ptr, ptr, ptr)
declare i32 @pthread_join(ptr, ptr)

define internal ptr @__ncc_posix_thread_start(ptr %handle) {
entry:
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %function = load ptr, ptr %function.slot, align 8
  %result = call i64 %function()
  store i64 %result, ptr %result.slot, align 8
  ret ptr %handle
}

define i64 @ncc_rt_thread_spawn(ptr %function) {
entry:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %create, label %oom
create:
  %thread.slot = getelementptr i8, ptr %handle, i64 0
  %function.slot = getelementptr i8, ptr %handle, i64 8
  %result.slot = getelementptr i8, ptr %handle, i64 16
  store ptr %function, ptr %function.slot, align 8
  store i64 0, ptr %result.slot, align 8
  %status = call i32 @pthread_create(ptr %thread.slot, ptr null, ptr @__ncc_posix_thread_start, ptr %handle)
  %created = icmp eq i32 %status, 0
  br i1 %created, label %success, label %failure
success:
  %raw = ptrtoint ptr %handle to i64
  %shifted = shl i64 %raw, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
oom:
  %error.oom = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.oom
}

define i64 @ncc_rt_thread_join(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %join, label %bad
join:
  %raw = lshr i64 %tagged, 1
  %handle = inttoptr i64 %raw to ptr
  %thread = load ptr, ptr %handle, align 8
  %status = call i32 @pthread_join(ptr %thread, ptr null)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  %result.slot = getelementptr i8, ptr %handle, i64 16
  %result = load i64, ptr %result.slot, align 8
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %tagged.result = call i64 @__ncc_process_thread_ok_i64(i64 %result)
  ret i64 %tagged.result
failed:
  %released.failed = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
bad:
  %error.bad = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.bad
}
"#;

const POSIX32: &str = r#"
declare i32 @pthread_create(ptr, ptr, ptr, ptr)
declare i32 @pthread_join(i32, ptr)

define internal ptr @__ncc_posix_thread_start(ptr %handle) {
entry:
  %function.slot = getelementptr i8, ptr %handle, i64 4
  %result.slot = getelementptr i8, ptr %handle, i64 8
  %function = load ptr, ptr %function.slot, align 4
  %result = call i64 %function()
  store i64 %result, ptr %result.slot, align 8
  ret ptr %handle
}

define i64 @ncc_rt_thread_spawn(ptr %function) {
entry:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 16, i64 8)
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %create, label %oom
create:
  %thread.slot = getelementptr i8, ptr %handle, i64 0
  %function.slot = getelementptr i8, ptr %handle, i64 4
  %result.slot = getelementptr i8, ptr %handle, i64 8
  store ptr %function, ptr %function.slot, align 4
  store i64 0, ptr %result.slot, align 8
  %status = call i32 @pthread_create(ptr %thread.slot, ptr null, ptr @__ncc_posix_thread_start, ptr %handle)
  %created = icmp eq i32 %status, 0
  br i1 %created, label %success, label %failure
success:
  %raw32 = ptrtoint ptr %handle to i32
  %raw = zext i32 %raw32 to i64
  %shifted = shl i64 %raw, 1
  %tagged = or i64 %shifted, 1
  ret i64 %tagged
failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
oom:
  %error.oom = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.oom
}

define i64 @ncc_rt_thread_join(i64 %tagged) {
entry:
  %positive = and i64 %tagged, 1
  %valid = icmp eq i64 %positive, 1
  br i1 %valid, label %join, label %bad
join:
  %raw = lshr i64 %tagged, 1
  %raw32 = trunc i64 %raw to i32
  %handle = inttoptr i32 %raw32 to ptr
  %thread = load i32, ptr %handle, align 4
  %status = call i32 @pthread_join(i32 %thread, ptr null)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  %result.slot = getelementptr i8, ptr %handle, i64 8
  %result = load i64, ptr %result.slot, align 8
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %tagged.result = call i64 @__ncc_process_thread_ok_i64(i64 %result)
  ret i64 %tagged.result
failed:
  %released.failed = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %code = zext i32 %status to i64
  %error = call i64 @__ncc_process_thread_error(i64 %code)
  ret i64 %error
bad:
  %error.bad = call i64 @__ncc_process_thread_error(i64 1)
  ret i64 %error.bad
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_provider_owns_process_thread_and_powershell_quoting() {
        let llvm = render_process_thread_runtime("x86_64-pc-windows-msvc", false)
            .expect("Win64 process/thread provider");
        for body in [
            "define i64 @ncc_rt_process_spawn",
            "define i64 @ncc_rt_process_spawn_in",
            "define i64 @ncc_rt_process_wait",
            "define i64 @ncc_rt_thread_spawn",
            "define i64 @ncc_rt_thread_join",
            "@__ncc_pt_build_ps_expression",
            "@__ncc_pt_append_windows_quoted",
        ] {
            assert!(llvm.contains(body), "missing {body}");
        }
        assert!(llvm.contains("@CreateProcessA"));
        assert!(llvm.contains("ptr %cwd, ptr %startup"));
        assert!(llvm.contains("@CreateThread"));
        assert!(!llvm.contains("@malloc"));
        assert!(!llvm.contains("@free"));
    }

    #[test]
    fn win32_provider_uses_32_bit_windows_abi_layouts() {
        let llvm = render_process_thread_runtime("i686-pc-windows-msvc", false)
            .expect("Win32 process/thread provider");
        assert!(llvm.contains("alloca [68 x i8]"));
        assert!(llvm.contains("alloca [16 x i8]"));
        assert!(llvm.contains("@CreateThread(ptr, i32,"));
        assert!(llvm.contains("ptrtoint ptr %value to i32"));
        assert!(llvm.contains("inttoptr i32 %raw32 to ptr"));
        assert!(llvm.contains("ptr %info, i64 4"));
        assert!(!llvm.contains("alloca [104 x i8]"));
    }

    #[test]
    fn posix_provider_selects_native_thread_width_and_process_calls() {
        let elf64 = render_process_thread_runtime("x86_64-unknown-linux-gnu", false)
            .expect("Linux64 process/thread provider");
        let elf32 = render_process_thread_runtime("i386-unknown-linux-gnu", false)
            .expect("Linux32 process/thread provider");
        let macho = render_process_thread_runtime("aarch64-apple-darwin", false)
            .expect("Darwin process/thread provider");
        assert!(elf64.contains("@pthread_join(i64"));
        assert!(elf32.contains("@pthread_join(i32"));
        assert!(macho.contains("@pthread_join(ptr"));
        for llvm in [&elf64, &elf32, &macho] {
            assert!(llvm.contains("@fork()"));
            assert!(llvm.contains("@chdir(ptr %cwd)"));
            assert!(llvm.contains("@execvp"));
            assert!(llvm.contains("@waitpid"));
            assert!(llvm.contains("define i64 @ncc_rt_process_spawn_in"));
        }
    }

    #[test]
    fn linux_process_spawn_can_pass_the_nc_environment_vector_explicitly() {
        let llvm = render_process_thread_runtime("x86_64-unknown-linux-gnu", true)
            .expect("Linux64 process/thread provider with explicit environment");
        assert!(llvm.contains("declare i32 @execvpe(ptr, ptr, ptr)"));
        assert!(llvm.contains("load ptr, ptr @__ncc_process_environ"));
        assert!(llvm.contains("@execvpe(ptr %program, ptr %argv, ptr %environment)"));
        assert!(!llvm.contains("declare i32 @execvp(ptr, ptr)"));
    }

    #[test]
    fn unsupported_process_thread_target_is_explicit() {
        let error = render_process_thread_runtime("wasm32-wasi", false)
            .expect_err("WASI has no admitted process/thread backend");
        assert!(error.contains("no direct native process and thread ABI"));
    }
}
