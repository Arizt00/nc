pub(crate) fn render_filesystem_runtime(target: &str) -> Result<String, String> {
    let normalized = target.to_ascii_lowercase();
    let platform = if normalized.contains("windows")
        && (normalized.contains("x86_64") || normalized.contains("aarch64"))
    {
        WINDOWS.to_string()
    } else if normalized.contains("darwin") || normalized.contains("apple") {
        render_posix_metadata(
            POSIX64
                .replace("@@OPEN_CREATE@@", "1537")
                .replace("@@OPEN_APPEND_CREATE@@", "521")
                .replace("@@OPEN_LOCK@@", "514")
                .replace("@@DIRENT_NAME@@", "21"),
            "@stat",
            "4",
            "96",
            "48",
            false,
        )
    } else if normalized.contains("linux") || normalized.contains("android") {
        let is_32 = normalized.contains("i386")
            || normalized.contains("i486")
            || normalized.contains("i586")
            || normalized.contains("i686")
            || normalized.contains("armv7")
            || normalized.contains("thumbv7")
            || normalized.contains("riscv32");
        let name_offset =
            if is_32 && !normalized.contains("android") && !normalized.contains("riscv32") {
                "11"
            } else {
                "19"
            };
        if is_32 {
            let (size_offset, modified_offset) = if normalized.contains("i386")
                || normalized.contains("i486")
                || normalized.contains("i586")
                || normalized.contains("i686")
            {
                ("44", "72")
            } else {
                ("48", "80")
            };
            render_posix_metadata(
                render_posix32("577", "66", name_offset),
                "@stat64",
                "16",
                size_offset,
                modified_offset,
                true,
            )
        } else {
            let mode_offset = if normalized.contains("x86_64") {
                "24"
            } else {
                "16"
            };
            render_posix_metadata(
                POSIX64
                    .replace("@@OPEN_CREATE@@", "577")
                    .replace("@@OPEN_APPEND_CREATE@@", "1089")
                    .replace("@@OPEN_LOCK@@", "66")
                    .replace("@@DIRENT_NAME@@", name_offset),
                "@stat",
                mode_offset,
                "48",
                "88",
                false,
            )
        }
    } else {
        return Err(format!(
            "filesystem has no direct operating-system ABI for target '{target}'"
        ));
    };
    Ok(format!("{platform}\n{COMMON}"))
}

fn render_posix_metadata(
    ir: String,
    stat_symbol: &str,
    mode_offset: &str,
    size_offset: &str,
    modified_offset: &str,
    time_is_32: bool,
) -> String {
    let load_modified = if time_is_32 {
        "%modified32 = load i32, ptr %modified.ptr, align 1\n  %modified = sext i32 %modified32 to i64"
    } else {
        "%modified = load i64, ptr %modified.ptr, align 1"
    };
    ir.replace("@NCC_STAT", stat_symbol)
        .replace("@@STAT_MODE_OFFSET@@", mode_offset)
        .replace("@@STAT_SIZE_OFFSET@@", size_offset)
        .replace("@@STAT_MODIFIED_OFFSET@@", modified_offset)
        .replace("@@STAT_LOAD_MODIFIED@@", load_modified)
}

fn render_posix32(open_create: &str, open_lock: &str, name_offset: &str) -> String {
    POSIX64
        .replace("declare i64 @read(i32, ptr, i64)", POSIX32_READ_ADAPTER)
        .replace("declare i64 @write(i32, ptr, i64)", POSIX32_WRITE_ADAPTER)
        .replace(
            "declare i64 @lseek(i32, i64, i32)",
            "declare i64 @lseek64(i32, i64, i32)",
        )
        .replace("declare ptr @getcwd(ptr, i64)", POSIX32_GETCWD_ADAPTER)
        .replace("call i64 @read(", "call i64 @ncc.fs.posix32.read(")
        .replace("call i64 @write(", "call i64 @ncc.fs.posix32.write(")
        .replace("call i64 @lseek(", "call i64 @lseek64(")
        .replace("call ptr @getcwd(", "call ptr @ncc.fs.posix32.getcwd(")
        .replace("@@OPEN_CREATE@@", open_create)
        .replace("@@OPEN_APPEND_CREATE@@", "1089")
        .replace("@@OPEN_LOCK@@", open_lock)
        .replace("@@DIRENT_NAME@@", name_offset)
}

const POSIX32_READ_ADAPTER: &str = r#"declare i32 @read(i32, ptr, i32)
define internal i64 @ncc.fs.posix32.read(i32 %fd, ptr %out, i64 %length) {
entry:
  %large = icmp ugt i64 %length, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %length
  %chunk = trunc i64 %chunk64 to i32
  %raw = call i32 @read(i32 %fd, ptr %out, i32 %chunk)
  %result = sext i32 %raw to i64
  ret i64 %result
}"#;

const POSIX32_WRITE_ADAPTER: &str = r#"declare i32 @write(i32, ptr, i32)
define internal i64 @ncc.fs.posix32.write(i32 %fd, ptr %data, i64 %length) {
entry:
  %large = icmp ugt i64 %length, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %length
  %chunk = trunc i64 %chunk64 to i32
  %raw = call i32 @write(i32 %fd, ptr %data, i32 %chunk)
  %result = sext i32 %raw to i64
  ret i64 %result
}"#;

const POSIX32_GETCWD_ADAPTER: &str = r#"declare ptr @getcwd(ptr, i32)
define internal ptr @ncc.fs.posix32.getcwd(ptr %out, i64 %capacity) {
entry:
  %large = icmp ugt i64 %capacity, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %capacity
  %chunk = trunc i64 %chunk64 to i32
  %result = call ptr @getcwd(ptr %out, i32 %chunk)
  ret ptr %result
}"#;

const WINDOWS: &str = r#"
declare ptr @CreateFileA(ptr, i32, i32, ptr, i32, i32, ptr)
declare i32 @ReadFile(ptr, ptr, i32, ptr, ptr)
declare i32 @WriteFile(ptr, ptr, i32, ptr, ptr)
declare i32 @FlushFileBuffers(ptr)
declare i32 @GetFileSizeEx(ptr, ptr)
declare i32 @SetFilePointerEx(ptr, i64, ptr, i32)
declare i32 @CloseHandle(ptr)
declare i32 @GetFileAttributesA(ptr)
declare i32 @DeleteFileA(ptr)
declare i32 @MoveFileExA(ptr, ptr, i32)
declare i32 @CreateDirectoryA(ptr, ptr)
declare i32 @RemoveDirectoryA(ptr)
declare i32 @GetCurrentDirectoryA(i32, ptr)
declare i32 @SetCurrentDirectoryA(ptr)
declare i32 @GetFullPathNameA(ptr, i32, ptr, ptr)
declare i32 @GetFinalPathNameByHandleA(ptr, ptr, i32, i32)
declare i32 @LockFileEx(ptr, i32, i32, i32, i32, ptr)
declare i32 @UnlockFileEx(ptr, i32, i32, i32, ptr)
declare ptr @FindFirstFileA(ptr, ptr)
declare i32 @FindNextFileA(ptr, ptr)
declare i32 @FindClose(ptr)
declare i32 @GetFileAttributesExA(ptr, i32, ptr)
declare i32 @SetFileAttributesA(ptr, i32)

define i64 @__ncc_fs_write(ptr %path, ptr %data, i64 %length, i1 %append) {
entry:
  %disposition = select i1 %append, i32 4, i32 2
  %handle = call ptr @CreateFileA(ptr %path, i32 1073741824, i32 1, ptr null, i32 %disposition, i32 128, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure.open, label %position
position:
  br i1 %append, label %seek, label %loop.entry
seek:
  %seek.ok = call i32 @SetFilePointerEx(ptr %handle, i64 0, ptr null, i32 2)
  %seek.failed = icmp eq i32 %seek.ok, 0
  br i1 %seek.failed, label %failure.io, label %loop.entry
loop.entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %flush, label %loop
loop:
  %offset = phi i64 [ 0, %loop.entry ], [ %next, %progress ]
  %remaining = sub i64 %length, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %source = getelementptr i8, ptr %data, i64 %offset
  %written.slot = alloca i32, align 4
  %status = call i32 @WriteFile(ptr %handle, ptr %source, i32 %chunk, ptr %written.slot, ptr null)
  %written = load i32, ptr %written.slot, align 4
  %status.ok = icmp ne i32 %status, 0
  %has.progress = icmp ugt i32 %written, 0
  %ok = and i1 %status.ok, %has.progress
  br i1 %ok, label %progress, label %failure.io
progress:
  %written64 = zext i32 %written to i64
  %next = add i64 %offset, %written64
  %done = icmp uge i64 %next, %length
  br i1 %done, label %flush, label %loop
flush:
  %flushed = call i32 @FlushFileBuffers(ptr %handle)
  %closed = call i32 @CloseHandle(ptr %handle)
  %flush.ok = icmp ne i32 %flushed, 0
  br i1 %flush.ok, label %success, label %failure.flush
success:
  ret i64 %length
failure.io:
  %closed.io = call i32 @CloseHandle(ptr %handle)
  ret i64 -2
failure.flush:
  ret i64 -3
failure.open:
  ret i64 -1
}

define i64 @__ncc_fs_read(ptr %path, ptr %out, i64 %capacity) {
entry:
  %handle = call ptr @CreateFileA(ptr %path, i32 -2147483648, i32 1, ptr null, i32 3, i32 128, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure.open, label %loop.entry
loop.entry:
  %empty = icmp eq i64 %capacity, 0
  br i1 %empty, label %success.empty, label %loop
loop:
  %offset = phi i64 [ 0, %loop.entry ], [ %next, %progress ]
  %remaining = sub i64 %capacity, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk64 = select i1 %large, i64 2147483647, i64 %remaining
  %chunk = trunc i64 %chunk64 to i32
  %destination = getelementptr i8, ptr %out, i64 %offset
  %read.slot = alloca i32, align 4
  %status = call i32 @ReadFile(ptr %handle, ptr %destination, i32 %chunk, ptr %read.slot, ptr null)
  %read = load i32, ptr %read.slot, align 4
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %inspect, label %failure.io
inspect:
  %eof = icmp eq i32 %read, 0
  br i1 %eof, label %success, label %progress
progress:
  %read64 = zext i32 %read to i64
  %next = add i64 %offset, %read64
  %done = icmp uge i64 %next, %capacity
  br i1 %done, label %success, label %loop
success:
  %final = phi i64 [ %offset, %inspect ], [ %next, %progress ]
  %closed = call i32 @CloseHandle(ptr %handle)
  ret i64 %final
success.empty:
  %closed.empty = call i32 @CloseHandle(ptr %handle)
  ret i64 0
failure.io:
  %closed.io = call i32 @CloseHandle(ptr %handle)
  ret i64 -2
failure.open:
  ret i64 -1
}

define i64 @__ncc_fs_size(ptr %path) {
entry:
  %handle = call ptr @CreateFileA(ptr %path, i32 -2147483648, i32 1, ptr null, i32 3, i32 128, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure, label %read
read:
  %size.slot = alloca i64, align 8
  %status = call i32 @GetFileSizeEx(ptr %handle, ptr %size.slot)
  %size = load i64, ptr %size.slot, align 8
  %closed = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  %result = select i1 %ok, i64 %size, i64 -2
  ret i64 %result
failure:
  ret i64 -1
}

define i32 @__ncc_fs_sync(ptr %path) {
entry:
  %handle = call ptr @CreateFileA(ptr %path, i32 -1073741824, i32 1, ptr null, i32 3, i32 128, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure, label %flush
flush:
  %status = call i32 @FlushFileBuffers(ptr %handle)
  %closed = call i32 @CloseHandle(ptr %handle)
  ret i32 %status
failure:
  ret i32 0
}

define i32 @__ncc_fs_exists(ptr %path) { %v = call i32 @GetFileAttributesA(ptr %path) %ok = icmp ne i32 %v, -1 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_delete(ptr %path) { %r = call i32 @DeleteFileA(ptr %path) ret i32 %r }
define i32 @__ncc_fs_rename(ptr %from, ptr %to) { %r = call i32 @MoveFileExA(ptr %from, ptr %to, i32 9) ret i32 %r }
define i32 @__ncc_fs_create_dir(ptr %path) { %r = call i32 @CreateDirectoryA(ptr %path, ptr null) ret i32 %r }
define i32 @__ncc_fs_remove_dir(ptr %path) { %r = call i32 @RemoveDirectoryA(ptr %path) ret i32 %r }
define i32 @__ncc_fs_change_dir(ptr %path) { %r = call i32 @SetCurrentDirectoryA(ptr %path) ret i32 %r }

define i32 @__ncc_fs_metadata_raw(ptr %path, ptr %out) {
entry:
  %data = alloca [36 x i8], align 4
  %status = call i32 @GetFileAttributesExA(ptr %path, i32 0, ptr %data)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %decode, label %failure
decode:
  %attributes = load i32, ptr %data, align 1
  %size.high.ptr = getelementptr i8, ptr %data, i64 28
  %size.low.ptr = getelementptr i8, ptr %data, i64 32
  %size.high32 = load i32, ptr %size.high.ptr, align 1
  %size.low32 = load i32, ptr %size.low.ptr, align 1
  %size.high = zext i32 %size.high32 to i64
  %size.low = zext i32 %size.low32 to i64
  %size.shifted = shl i64 %size.high, 32
  %size = or i64 %size.shifted, %size.low
  %modified.low.ptr = getelementptr i8, ptr %data, i64 20
  %modified.high.ptr = getelementptr i8, ptr %data, i64 24
  %modified.low32 = load i32, ptr %modified.low.ptr, align 1
  %modified.high32 = load i32, ptr %modified.high.ptr, align 1
  %modified.low = zext i32 %modified.low32 to i64
  %modified.high = zext i32 %modified.high32 to i64
  %modified.shifted = shl i64 %modified.high, 32
  %filetime = or i64 %modified.shifted, %modified.low
  %after.epoch = icmp uge i64 %filetime, 116444736000000000
  %epoch.delta = sub i64 %filetime, 116444736000000000
  %seconds = udiv i64 %epoch.delta, 10000000
  %modified = select i1 %after.epoch, i64 %seconds, i64 0
  %directory.bits = and i32 %attributes, 16
  %is.directory = icmp ne i32 %directory.bits, 0
  %kind = select i1 %is.directory, i32 2, i32 1
  %readonly.bits = and i32 %attributes, 1
  %is.readonly = icmp ne i32 %readonly.bits, 0
  %readonly = zext i1 %is.readonly to i32
  %writable.mode = select i1 %is.directory, i32 511, i32 438
  %readonly.mode = select i1 %is.directory, i32 365, i32 292
  %mode = select i1 %is.readonly, i32 %readonly.mode, i32 %writable.mode
  %size.slot = getelementptr i8, ptr %out, i64 0
  %modified.slot = getelementptr i8, ptr %out, i64 8
  %kind.slot = getelementptr i8, ptr %out, i64 16
  %mode.slot = getelementptr i8, ptr %out, i64 20
  %readonly.slot = getelementptr i8, ptr %out, i64 24
  store i64 %size, ptr %size.slot, align 8
  store i64 %modified, ptr %modified.slot, align 8
  store i32 %kind, ptr %kind.slot, align 4
  store i32 %mode, ptr %mode.slot, align 4
  store i32 %readonly, ptr %readonly.slot, align 4
  ret i32 1
failure:
  ret i32 0
}

define i32 @__ncc_fs_set_readonly(ptr %path, i1 %readonly) {
entry:
  %attributes = call i32 @GetFileAttributesA(ptr %path)
  %invalid = icmp eq i32 %attributes, -1
  br i1 %invalid, label %failure, label %select
select:
  %with.readonly = or i32 %attributes, 1
  %without.readonly = and i32 %attributes, -2
  %next = select i1 %readonly, i32 %with.readonly, i32 %without.readonly
  %status = call i32 @SetFileAttributesA(ptr %path, i32 %next)
  ret i32 %status
failure:
  ret i32 0
}

define i1 @__ncc_fs_copy_permissions(ptr %source, ptr %destination) {
entry:
  ret i1 true
}

define i64 @__ncc_fs_current_dir_into(ptr %out, i64 %capacity) {
entry:
  %missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %query = or i1 %missing, %zero
  br i1 %query, label %insufficient, label %read
read:
  %large = icmp ugt i64 %capacity, 4294967295
  %cap32 = trunc i64 %capacity to i32
  %cap = select i1 %large, i32 -1, i32 %cap32
  %written = call i32 @GetCurrentDirectoryA(i32 %cap, ptr %out)
  %failed = icmp eq i32 %written, 0
  %too.small = icmp uge i32 %written, %cap
  br i1 %failed, label %failure, label %inspect
inspect:
  br i1 %too.small, label %insufficient, label %success
success:
  %result = zext i32 %written to i64
  ret i64 %result
insufficient:
  ret i64 -2
failure:
  ret i64 -1
}

define i64 @__ncc_fs_absolute_path_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %path.missing = icmp eq ptr %path, null
  %out.missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %buffer.invalid = or i1 %out.missing, %zero
  br i1 %path.missing, label %failure, label %buffer
buffer:
  br i1 %buffer.invalid, label %insufficient, label %resolve
resolve:
  %large = icmp ugt i64 %capacity, 4294967295
  %cap32 = trunc i64 %capacity to i32
  %cap = select i1 %large, i32 -1, i32 %cap32
  %written = call i32 @GetFullPathNameA(ptr %path, i32 %cap, ptr %out, ptr null)
  %failed = icmp eq i32 %written, 0
  %too.small = icmp uge i32 %written, %cap
  br i1 %failed, label %failure, label %inspect
inspect:
  br i1 %too.small, label %insufficient, label %success
success:
  %result = zext i32 %written to i64
  ret i64 %result
insufficient:
  ret i64 -2
failure:
  ret i64 -1
}

define i64 @__ncc_fs_official_path_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %path.missing = icmp eq ptr %path, null
  %out.missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %buffer.invalid = or i1 %out.missing, %zero
  br i1 %path.missing, label %failure.open, label %buffer
buffer:
  br i1 %buffer.invalid, label %insufficient, label %open
open:
  %handle = call ptr @CreateFileA(ptr %path, i32 0, i32 7, ptr null, i32 3, i32 33554432, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure.open, label %resolve
resolve:
  %large = icmp ugt i64 %capacity, 4294967295
  %cap32 = trunc i64 %capacity to i32
  %cap = select i1 %large, i32 -1, i32 %cap32
  %written = call i32 @GetFinalPathNameByHandleA(ptr %handle, ptr %out, i32 %cap, i32 0)
  %closed = call i32 @CloseHandle(ptr %handle)
  %failed = icmp eq i32 %written, 0
  %too.small = icmp uge i32 %written, %cap
  br i1 %failed, label %failure.resolve, label %inspect
inspect:
  br i1 %too.small, label %insufficient, label %success
success:
  %result = zext i32 %written to i64
  ret i64 %result
insufficient:
  ret i64 -2
failure.resolve:
  ret i64 -3
failure.open:
  ret i64 -1
}

define ptr @__ncc_fs_lock(ptr %path, i1 %exclusive, i1 %wait) {
entry:
  %path.missing = icmp eq ptr %path, null
  br i1 %path.missing, label %failure.open, label %open
open:
  %handle = call ptr @CreateFileA(ptr %path, i32 -1073741824, i32 7, ptr null, i32 4, i32 128, ptr null)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure.open, label %lock
lock:
  %exclusive.flag = select i1 %exclusive, i32 2, i32 0
  %no.wait = xor i1 %wait, true
  %wait.flag = select i1 %no.wait, i32 1, i32 0
  %flags = or i32 %exclusive.flag, %wait.flag
  %overlapped = alloca [32 x i8], align 8
  store i64 0, ptr %overlapped, align 8
  %overlapped.8 = getelementptr i8, ptr %overlapped, i64 8
  %overlapped.16 = getelementptr i8, ptr %overlapped, i64 16
  %overlapped.24 = getelementptr i8, ptr %overlapped, i64 24
  store i64 0, ptr %overlapped.8, align 8
  store i64 0, ptr %overlapped.16, align 8
  store i64 0, ptr %overlapped.24, align 8
  %status = call i32 @LockFileEx(ptr %handle, i32 %flags, i32 0, i32 -1, i32 -1, ptr %overlapped)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure.lock
success:
  ret ptr %handle
failure.lock:
  %closed.failure = call i32 @CloseHandle(ptr %handle)
  ret ptr null
failure.open:
  ret ptr null
}

define i32 @__ncc_fs_unlock(ptr %handle) {
entry:
  %missing = icmp eq ptr %handle, null
  br i1 %missing, label %failure, label %unlock
unlock:
  %overlapped = alloca [32 x i8], align 8
  store i64 0, ptr %overlapped, align 8
  %overlapped.8 = getelementptr i8, ptr %overlapped, i64 8
  %overlapped.16 = getelementptr i8, ptr %overlapped, i64 16
  %overlapped.24 = getelementptr i8, ptr %overlapped, i64 24
  store i64 0, ptr %overlapped.8, align 8
  store i64 0, ptr %overlapped.16, align 8
  store i64 0, ptr %overlapped.24, align 8
  %status = call i32 @UnlockFileEx(ptr %handle, i32 0, i32 -1, i32 -1, ptr %overlapped)
  %closed = call i32 @CloseHandle(ptr %handle)
  %unlocked = icmp ne i32 %status, 0
  %closed.ok = icmp ne i32 %closed, 0
  %ok = and i1 %unlocked, %closed.ok
  %result = zext i1 %ok to i32
  ret i32 %result
failure:
  ret i32 0
}

define i64 @__ncc_fs_list_dir_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %path.length = call i64 @ncc.fs.strlen(ptr %path)
  %pattern.bytes = add i64 %path.length, 4
  %pattern = alloca i8, i64 %pattern.bytes, align 1
  call void @ncc.fs.copy(ptr %pattern, ptr %path, i64 %path.length)
  %last.index = sub i64 %path.length, 1
  %has.path = icmp ugt i64 %path.length, 0
  br i1 %has.path, label %separator.inspect, label %separator.add
separator.inspect:
  %last.ptr = getelementptr i8, ptr %path, i64 %last.index
  %last = load i8, ptr %last.ptr, align 1
  %slash = icmp eq i8 %last, 47
  %backslash = icmp eq i8 %last, 92
  %has.separator = or i1 %slash, %backslash
  br i1 %has.separator, label %star, label %separator.add
separator.add:
  %separator.index = phi i64 [ 0, %entry ], [ %path.length, %separator.inspect ]
  %separator.ptr = getelementptr i8, ptr %pattern, i64 %separator.index
  store i8 92, ptr %separator.ptr, align 1
  %after.separator = add i64 %separator.index, 1
  br label %star
star:
  %cursor = phi i64 [ %path.length, %separator.inspect ], [ %after.separator, %separator.add ]
  %star.ptr = getelementptr i8, ptr %pattern, i64 %cursor
  store i8 42, ptr %star.ptr, align 1
  %nul.index = add i64 %cursor, 1
  %nul.ptr = getelementptr i8, ptr %pattern, i64 %nul.index
  store i8 0, ptr %nul.ptr, align 1
  %record = alloca [320 x i8], align 8
  %handle = call ptr @FindFirstFileA(ptr %pattern, ptr %record)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure, label %item
item:
  %needed = phi i64 [ 0, %star ], [ %carry, %next ]
  %name = getelementptr i8, ptr %record, i64 44
  %name.length = call i64 @ncc.fs.strlen(ptr %name)
  %first = load i8, ptr %name, align 1
  %is.dot.first = icmp eq i8 %first, 46
  %one = icmp eq i64 %name.length, 1
  %two = icmp eq i64 %name.length, 2
  %second.ptr = getelementptr i8, ptr %name, i64 1
  %second = load i8, ptr %second.ptr, align 1
  %is.dot.second = icmp eq i8 %second, 46
  %dotdot = and i1 %two, %is.dot.second
  %dot.entry = and i1 %is.dot.first, %one
  %skip = or i1 %dot.entry, %dotdot
  br i1 %skip, label %next, label %append
append:
  %item.bytes = add i64 %name.length, 1
  %next.needed.value = add i64 %needed, %item.bytes
  %with.nul = add i64 %next.needed.value, 1
  %fits = icmp ule i64 %with.nul, %capacity
  %has.output = icmp ne ptr %out, null
  %can.write = and i1 %fits, %has.output
  br i1 %can.write, label %write, label %advance
write:
  %destination = getelementptr i8, ptr %out, i64 %needed
  call void @ncc.fs.copy(ptr %destination, ptr %name, i64 %name.length)
  %newline.ptr = getelementptr i8, ptr %destination, i64 %name.length
  store i8 10, ptr %newline.ptr, align 1
  %term.ptr = getelementptr i8, ptr %out, i64 %next.needed.value
  store i8 0, ptr %term.ptr, align 1
  br label %advance
advance:
  %next.needed = phi i64 [ %next.needed.value, %append ], [ %next.needed.value, %write ]
  br label %next
next:
  %carry = phi i64 [ %needed, %item ], [ %next.needed, %advance ]
  %more = call i32 @FindNextFileA(ptr %handle, ptr %record)
  %has.more = icmp ne i32 %more, 0
  br i1 %has.more, label %item, label %done
done:
  %closed = call i32 @FindClose(ptr %handle)
  ret i64 %carry
failure:
  ret i64 -1
}
"#;

const POSIX64: &str = r#"
declare i32 @open(ptr, i32, ...)
declare i64 @read(i32, ptr, i64)
declare i64 @write(i32, ptr, i64)
declare i64 @lseek(i32, i64, i32)
declare i32 @fsync(i32)
declare i32 @close(i32)
declare i32 @access(ptr, i32)
declare i32 @unlink(ptr)
declare i32 @rename(ptr, ptr)
declare i32 @mkdir(ptr, i32)
declare i32 @rmdir(ptr)
declare ptr @getcwd(ptr, i64)
declare i32 @chdir(ptr)
declare i32 @NCC_STAT(ptr, ptr)
declare i32 @chmod(ptr, i32)
declare ptr @realpath(ptr, ptr)
declare i32 @flock(i32, i32)
declare ptr @opendir(ptr)
declare ptr @readdir(ptr)
declare i32 @closedir(ptr)

define i64 @__ncc_fs_write(ptr %path, ptr %data, i64 %length, i1 %append) {
entry:
  %flags = select i1 %append, i32 @@OPEN_APPEND_CREATE@@, i32 @@OPEN_CREATE@@
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 %flags, i32 438)
  %bad = icmp slt i32 %fd, 0
  br i1 %bad, label %failure.open, label %loop.entry
loop.entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %flush, label %loop
loop:
  %offset = phi i64 [ 0, %loop.entry ], [ %next, %progress ]
  %remaining = sub i64 %length, %offset
  %source = getelementptr i8, ptr %data, i64 %offset
  %written = call i64 @write(i32 %fd, ptr %source, i64 %remaining)
  %ok = icmp sgt i64 %written, 0
  br i1 %ok, label %progress, label %failure.io
progress:
  %next = add i64 %offset, %written
  %done = icmp uge i64 %next, %length
  br i1 %done, label %flush, label %loop
flush:
  %flushed = call i32 @fsync(i32 %fd)
  %closed = call i32 @close(i32 %fd)
  %ok.flush = icmp eq i32 %flushed, 0
  br i1 %ok.flush, label %success, label %failure.flush
success:
  ret i64 %length
failure.io:
  %closed.io = call i32 @close(i32 %fd)
  ret i64 -2
failure.flush:
  ret i64 -3
failure.open:
  ret i64 -1
}

define i64 @__ncc_fs_read(ptr %path, ptr %out, i64 %capacity) {
entry:
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 0)
  %bad = icmp slt i32 %fd, 0
  br i1 %bad, label %failure.open, label %loop.entry
loop.entry:
  %empty = icmp eq i64 %capacity, 0
  br i1 %empty, label %success.empty, label %loop
loop:
  %offset = phi i64 [ 0, %loop.entry ], [ %next, %progress ]
  %remaining = sub i64 %capacity, %offset
  %destination = getelementptr i8, ptr %out, i64 %offset
  %count = call i64 @read(i32 %fd, ptr %destination, i64 %remaining)
  %failed = icmp slt i64 %count, 0
  %eof = icmp eq i64 %count, 0
  br i1 %failed, label %failure.io, label %inspect
inspect:
  br i1 %eof, label %success, label %progress
progress:
  %next = add i64 %offset, %count
  %done = icmp uge i64 %next, %capacity
  br i1 %done, label %success, label %loop
success:
  %final = phi i64 [ %offset, %inspect ], [ %next, %progress ]
  %closed = call i32 @close(i32 %fd)
  ret i64 %final
success.empty:
  %closed.empty = call i32 @close(i32 %fd)
  ret i64 0
failure.io:
  %closed.io = call i32 @close(i32 %fd)
  ret i64 -2
failure.open:
  ret i64 -1
}

define i64 @__ncc_fs_size(ptr %path) {
entry:
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 0)
  %bad = icmp slt i32 %fd, 0
  br i1 %bad, label %failure, label %seek
seek:
  %size = call i64 @lseek(i32 %fd, i64 0, i32 2)
  %closed = call i32 @close(i32 %fd)
  ret i64 %size
failure:
  ret i64 -1
}

define i32 @__ncc_fs_sync(ptr %path) {
entry:
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 2)
  %bad = icmp slt i32 %fd, 0
  br i1 %bad, label %failure, label %flush
flush:
  %status = call i32 @fsync(i32 %fd)
  %closed = call i32 @close(i32 %fd)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
failure:
  ret i32 0
}

define i32 @__ncc_fs_exists(ptr %path) { %v = call i32 @access(ptr %path, i32 0) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_delete(ptr %path) { %v = call i32 @unlink(ptr %path) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_rename(ptr %from, ptr %to) { %v = call i32 @rename(ptr %from, ptr %to) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_create_dir(ptr %path) { %v = call i32 @mkdir(ptr %path, i32 511) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_remove_dir(ptr %path) { %v = call i32 @rmdir(ptr %path) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }
define i32 @__ncc_fs_change_dir(ptr %path) { %v = call i32 @chdir(ptr %path) %ok = icmp eq i32 %v, 0 %r = zext i1 %ok to i32 ret i32 %r }

define i32 @__ncc_fs_metadata_raw(ptr %path, ptr %out) {
entry:
  %data = alloca [256 x i8], align 8
  %status = call i32 @NCC_STAT(ptr %path, ptr %data)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %decode, label %failure
decode:
  %mode.ptr = getelementptr i8, ptr %data, i64 @@STAT_MODE_OFFSET@@
  %mode.raw = load i32, ptr %mode.ptr, align 1
  %mode = and i32 %mode.raw, 65535
  %size.ptr = getelementptr i8, ptr %data, i64 @@STAT_SIZE_OFFSET@@
  %size = load i64, ptr %size.ptr, align 1
  %modified.ptr = getelementptr i8, ptr %data, i64 @@STAT_MODIFIED_OFFSET@@
  @@STAT_LOAD_MODIFIED@@
  %kind.bits = and i32 %mode, 61440
  %is.file = icmp eq i32 %kind.bits, 32768
  %is.directory = icmp eq i32 %kind.bits, 16384
  %is.symlink = icmp eq i32 %kind.bits, 40960
  %non.file.kind = select i1 %is.symlink, i32 3, i32 0
  %non.directory.kind = select i1 %is.directory, i32 2, i32 %non.file.kind
  %kind = select i1 %is.file, i32 1, i32 %non.directory.kind
  %write.bits = and i32 %mode, 146
  %is.readonly = icmp eq i32 %write.bits, 0
  %readonly = zext i1 %is.readonly to i32
  %size.slot = getelementptr i8, ptr %out, i64 0
  %modified.slot = getelementptr i8, ptr %out, i64 8
  %kind.slot = getelementptr i8, ptr %out, i64 16
  %mode.slot = getelementptr i8, ptr %out, i64 20
  %readonly.slot = getelementptr i8, ptr %out, i64 24
  store i64 %size, ptr %size.slot, align 8
  store i64 %modified, ptr %modified.slot, align 8
  store i32 %kind, ptr %kind.slot, align 4
  store i32 %mode, ptr %mode.slot, align 4
  store i32 %readonly, ptr %readonly.slot, align 4
  ret i32 1
failure:
  ret i32 0
}

define i32 @__ncc_fs_set_readonly(ptr %path, i1 %readonly) {
entry:
  %data = alloca [256 x i8], align 8
  %status = call i32 @NCC_STAT(ptr %path, ptr %data)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %decode, label %failure
decode:
  %mode.ptr = getelementptr i8, ptr %data, i64 @@STAT_MODE_OFFSET@@
  %mode.raw = load i32, ptr %mode.ptr, align 1
  %mode = and i32 %mode.raw, 65535
  %without.write = and i32 %mode, -147
  %with.owner.write = or i32 %mode, 128
  %next = select i1 %readonly, i32 %without.write, i32 %with.owner.write
  %changed = call i32 @chmod(ptr %path, i32 %next)
  %changed.ok = icmp eq i32 %changed, 0
  %result = zext i1 %changed.ok to i32
  ret i32 %result
failure:
  ret i32 0
}

define i1 @__ncc_fs_copy_permissions(ptr %source, ptr %destination) {
entry:
  %data = alloca [256 x i8], align 8
  %status = call i32 @NCC_STAT(ptr %source, ptr %data)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %copy, label %failure
copy:
  %mode.ptr = getelementptr i8, ptr %data, i64 @@STAT_MODE_OFFSET@@
  %mode.raw = load i32, ptr %mode.ptr, align 1
  %mode = and i32 %mode.raw, 4095
  %changed = call i32 @chmod(ptr %destination, i32 %mode)
  %changed.ok = icmp eq i32 %changed, 0
  ret i1 %changed.ok
failure:
  ret i1 false
}

define i64 @__ncc_fs_current_dir_into(ptr %out, i64 %capacity) {
entry:
  %missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %invalid = or i1 %missing, %zero
  br i1 %invalid, label %insufficient, label %read
read:
  %result = call ptr @getcwd(ptr %out, i64 %capacity)
  %failed = icmp eq ptr %result, null
  br i1 %failed, label %insufficient, label %success
success:
  %length = call i64 @ncc.fs.strlen(ptr %out)
  ret i64 %length
insufficient:
  ret i64 -2
}

define i64 @__ncc_fs_absolute_path_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %path.missing = icmp eq ptr %path, null
  %out.missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %buffer.invalid = or i1 %out.missing, %zero
  br i1 %path.missing, label %failure, label %buffer
buffer:
  br i1 %buffer.invalid, label %insufficient, label %inspect
inspect:
  %first = load i8, ptr %path, align 1
  %absolute = icmp eq i8 %first, 47
  br i1 %absolute, label %copy.absolute, label %copy.relative
copy.absolute:
  %absolute.length = call i64 @ncc.fs.strlen(ptr %path)
  %absolute.bytes = add i64 %absolute.length, 1
  %absolute.fits = icmp ule i64 %absolute.bytes, %capacity
  br i1 %absolute.fits, label %absolute.write, label %insufficient
absolute.write:
  call void @ncc.fs.copy(ptr %out, ptr %path, i64 %absolute.bytes)
  ret i64 %absolute.length
copy.relative:
  %cwd.length = call i64 @__ncc_fs_current_dir_into(ptr %out, i64 %capacity)
  %cwd.ok = icmp sge i64 %cwd.length, 0
  br i1 %cwd.ok, label %relative.size, label %insufficient
relative.size:
  %path.length = call i64 @ncc.fs.strlen(ptr %path)
  %relative.bytes = add i64 %cwd.length, %path.length
  %relative.with.separator = add i64 %relative.bytes, 1
  %relative.required = add i64 %relative.with.separator, 1
  %relative.fits = icmp ule i64 %relative.required, %capacity
  br i1 %relative.fits, label %relative.write, label %insufficient
relative.write:
  %separator = getelementptr i8, ptr %out, i64 %cwd.length
  store i8 47, ptr %separator, align 1
  %path.offset = add i64 %cwd.length, 1
  %destination = getelementptr i8, ptr %out, i64 %path.offset
  %path.bytes = add i64 %path.length, 1
  call void @ncc.fs.copy(ptr %destination, ptr %path, i64 %path.bytes)
  %result.length = sub i64 %relative.required, 1
  ret i64 %result.length
insufficient:
  ret i64 -2
failure:
  ret i64 -1
}

define i64 @__ncc_fs_official_path_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %path.missing = icmp eq ptr %path, null
  %out.missing = icmp eq ptr %out, null
  %zero = icmp eq i64 %capacity, 0
  %buffer.invalid = or i1 %out.missing, %zero
  br i1 %path.missing, label %failure, label %buffer
buffer:
  br i1 %buffer.invalid, label %insufficient, label %allocate
allocate:
  %scratch = call ptr @__ncc_raw_aligned_alloc(i64 1048576, i64 8)
  %scratch.missing = icmp eq ptr %scratch, null
  br i1 %scratch.missing, label %failure.alloc, label %resolve
resolve:
  %resolved = call ptr @realpath(ptr %path, ptr %scratch)
  %failed = icmp eq ptr %resolved, null
  br i1 %failed, label %failure.resolve, label %size
size:
  %length = call i64 @ncc.fs.strlen(ptr %scratch)
  %bytes = add i64 %length, 1
  %fits = icmp ule i64 %bytes, %capacity
  br i1 %fits, label %copy, label %insufficient.release
copy:
  call void @ncc.fs.copy(ptr %out, ptr %scratch, i64 %bytes)
  %released = call i1 @__ncc_raw_aligned_free(ptr %scratch)
  ret i64 %length
insufficient.release:
  %released.small = call i1 @__ncc_raw_aligned_free(ptr %scratch)
  ret i64 -2
failure.resolve:
  %released.failure = call i1 @__ncc_raw_aligned_free(ptr %scratch)
  ret i64 -3
failure.alloc:
  ret i64 -4
insufficient:
  ret i64 -2
failure:
  ret i64 -1
}

define ptr @__ncc_fs_lock(ptr %path, i1 %exclusive, i1 %wait) {
entry:
  %path.missing = icmp eq ptr %path, null
  br i1 %path.missing, label %failure.open, label %open
open:
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 @@OPEN_LOCK@@, i32 438)
  %bad = icmp slt i32 %fd, 0
  br i1 %bad, label %failure.open, label %lock
lock:
  %kind = select i1 %exclusive, i32 2, i32 1
  %no.wait = xor i1 %wait, true
  %nonblock = select i1 %no.wait, i32 4, i32 0
  %operation = or i32 %kind, %nonblock
  %status = call i32 @flock(i32 %fd, i32 %operation)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failure.lock
success:
  %fd64 = zext i32 %fd to i64
  %encoded = add i64 %fd64, 1
  %handle = inttoptr i64 %encoded to ptr
  ret ptr %handle
failure.lock:
  %closed.failure = call i32 @close(i32 %fd)
  ret ptr null
failure.open:
  ret ptr null
}

define i32 @__ncc_fs_unlock(ptr %handle) {
entry:
  %missing = icmp eq ptr %handle, null
  br i1 %missing, label %failure, label %unlock
unlock:
  %encoded = ptrtoint ptr %handle to i64
  %fd64 = sub i64 %encoded, 1
  %fd = trunc i64 %fd64 to i32
  %status = call i32 @flock(i32 %fd, i32 8)
  %closed = call i32 @close(i32 %fd)
  %unlocked = icmp eq i32 %status, 0
  %closed.ok = icmp eq i32 %closed, 0
  %ok = and i1 %unlocked, %closed.ok
  %result = zext i1 %ok to i32
  ret i32 %result
failure:
  ret i32 0
}

define i64 @__ncc_fs_list_dir_into(ptr %path, ptr %out, i64 %capacity) {
entry:
  %directory = call ptr @opendir(ptr %path)
  %bad = icmp eq ptr %directory, null
  br i1 %bad, label %failure, label %next
next:
  %needed = phi i64 [ 0, %entry ], [ %next.needed, %advance ], [ %needed, %skip ]
  %entry.ptr = call ptr @readdir(ptr %directory)
  %done = icmp eq ptr %entry.ptr, null
  br i1 %done, label %finish, label %item
item:
  %name = getelementptr i8, ptr %entry.ptr, i64 @@DIRENT_NAME@@
  %name.length = call i64 @ncc.fs.strlen(ptr %name)
  %first = load i8, ptr %name, align 1
  %dot.first = icmp eq i8 %first, 46
  %one = icmp eq i64 %name.length, 1
  %two = icmp eq i64 %name.length, 2
  %second.ptr = getelementptr i8, ptr %name, i64 1
  %second = load i8, ptr %second.ptr, align 1
  %dot.second = icmp eq i8 %second, 46
  %dotdot = and i1 %two, %dot.second
  %dot = and i1 %dot.first, %one
  %omit = or i1 %dot, %dotdot
  br i1 %omit, label %skip, label %append
skip:
  br label %next
append:
  %item.bytes = add i64 %name.length, 1
  %next.needed.value = add i64 %needed, %item.bytes
  %with.nul = add i64 %next.needed.value, 1
  %fits = icmp ule i64 %with.nul, %capacity
  %has.output = icmp ne ptr %out, null
  %can.write = and i1 %fits, %has.output
  br i1 %can.write, label %write, label %advance
write:
  %destination = getelementptr i8, ptr %out, i64 %needed
  call void @ncc.fs.copy(ptr %destination, ptr %name, i64 %name.length)
  %newline.ptr = getelementptr i8, ptr %destination, i64 %name.length
  store i8 10, ptr %newline.ptr, align 1
  %term.ptr = getelementptr i8, ptr %out, i64 %next.needed.value
  store i8 0, ptr %term.ptr, align 1
  br label %advance
advance:
  %next.needed = phi i64 [ %next.needed.value, %append ], [ %next.needed.value, %write ]
  br label %next
finish:
  %closed = call i32 @closedir(ptr %directory)
  ret i64 %needed
failure:
  ret i64 -1
}
"#;

const COMMON: &str = r#"
%ncc.fs.list = type { ptr, i64, i64 }
%ncc.fs.array = type { ptr, i64 }
%ncc.fs.metadata.raw = type { i64, i64, i32, i32, i32 }

@ncc.fs.literal.type = private constant [5 x i8] c"type="
@ncc.fs.literal.file = private constant [4 x i8] c"file"
@ncc.fs.literal.directory = private constant [9 x i8] c"directory"
@ncc.fs.literal.symlink = private constant [7 x i8] c"symlink"
@ncc.fs.literal.other = private constant [5 x i8] c"other"
@ncc.fs.literal.length = private constant [8 x i8] c";length="
@ncc.fs.literal.readonly = private constant [10 x i8] c";readonly="
@ncc.fs.literal.readonly.start = private constant [9 x i8] c"readonly="
@ncc.fs.literal.modified = private constant [10 x i8] c";modified="
@ncc.fs.literal.mode = private constant [6 x i8] c";mode="
@ncc.fs.literal.true = private constant [4 x i8] c"true"
@ncc.fs.literal.false = private constant [5 x i8] c"false"

define internal i64 @ncc.fs.strlen(ptr %text) {
entry:
  %null = icmp eq ptr %text, null
  br i1 %null, label %empty, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %cursor = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %cursor, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %result, label %more
more:
  %next = add i64 %index, 1
  br label %loop
result:
  ret i64 %index
empty:
  ret i64 0
}

define internal void @ncc.fs.copy(ptr %out, ptr %source, i64 %length) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %done, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %loop ]
  %source.ptr = getelementptr i8, ptr %source, i64 %index
  %value = load i8, ptr %source.ptr, align 1
  %out.ptr = getelementptr i8, ptr %out, i64 %index
  store i8 %value, ptr %out.ptr, align 1
  %next = add i64 %index, 1
  %complete = icmp uge i64 %next, %length
  br i1 %complete, label %done, label %loop
done:
  ret void
}

define internal i64 @ncc.fs.append.literal(ptr %out, i64 %cursor, ptr %source, i64 %length) {
entry:
  %destination = getelementptr i8, ptr %out, i64 %cursor
  call void @ncc.fs.copy(ptr %destination, ptr %source, i64 %length)
  %next = add i64 %cursor, %length
  ret i64 %next
}

define internal i64 @ncc.fs.append.u64(ptr %out, i64 %cursor, i64 %value) {
entry:
  %zero = icmp eq i64 %value, 0
  br i1 %zero, label %write.zero, label %count.entry
write.zero:
  %zero.ptr = getelementptr i8, ptr %out, i64 %cursor
  store i8 48, ptr %zero.ptr, align 1
  %zero.next = add i64 %cursor, 1
  ret i64 %zero.next
count.entry:
  br label %count
count:
  %count.value = phi i64 [ %value, %count.entry ], [ %count.quotient, %count.more ]
  %digits = phi i64 [ 0, %count.entry ], [ %digits.next, %count.more ]
  %count.quotient = udiv i64 %count.value, 10
  %digits.next = add i64 %digits, 1
  %count.done = icmp eq i64 %count.quotient, 0
  br i1 %count.done, label %write.entry, label %count.more
count.more:
  br label %count
write.entry:
  br label %write
write:
  %write.value = phi i64 [ %value, %write.entry ], [ %quotient, %write.more ]
  %remaining = phi i64 [ %digits.next, %write.entry ], [ %remaining.next, %write.more ]
  %quotient = udiv i64 %write.value, 10
  %digit = urem i64 %write.value, 10
  %remaining.next = sub i64 %remaining, 1
  %index = add i64 %cursor, %remaining.next
  %digit.ptr = getelementptr i8, ptr %out, i64 %index
  %digit8 = trunc i64 %digit to i8
  %ascii = add i8 %digit8, 48
  store i8 %ascii, ptr %digit.ptr, align 1
  %write.done = icmp eq i64 %remaining.next, 0
  br i1 %write.done, label %done, label %write.more
write.more:
  br label %write
done:
  %next = add i64 %cursor, %digits.next
  ret i64 %next
}

define internal i64 @ncc.fs.append.bool(ptr %out, i64 %cursor, i1 %value) {
entry:
  br i1 %value, label %write.true, label %write.false
write.true:
  %true.next = call i64 @ncc.fs.append.literal(ptr %out, i64 %cursor, ptr @ncc.fs.literal.true, i64 4)
  ret i64 %true.next
write.false:
  %false.next = call i64 @ncc.fs.append.literal(ptr %out, i64 %cursor, ptr @ncc.fs.literal.false, i64 5)
  ret i64 %false.next
}

define internal i64 @ncc.fs.describe(ptr %path, ptr %out, i64 %capacity, i1 %permissions.only) {
entry:
  %metadata = alloca %ncc.fs.metadata.raw, align 8
  %status = call i32 @__ncc_fs_metadata_raw(ptr %path, ptr %metadata)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %load, label %failure
load:
  %size.slot = getelementptr %ncc.fs.metadata.raw, ptr %metadata, i32 0, i32 0
  %modified.slot = getelementptr %ncc.fs.metadata.raw, ptr %metadata, i32 0, i32 1
  %kind.slot = getelementptr %ncc.fs.metadata.raw, ptr %metadata, i32 0, i32 2
  %mode.slot = getelementptr %ncc.fs.metadata.raw, ptr %metadata, i32 0, i32 3
  %readonly.slot = getelementptr %ncc.fs.metadata.raw, ptr %metadata, i32 0, i32 4
  %size = load i64, ptr %size.slot, align 8
  %modified = load i64, ptr %modified.slot, align 8
  %kind = load i32, ptr %kind.slot, align 4
  %mode32 = load i32, ptr %mode.slot, align 4
  %readonly32 = load i32, ptr %readonly.slot, align 4
  %mode = zext i32 %mode32 to i64
  %readonly = icmp ne i32 %readonly32, 0
  %temporary = alloca [256 x i8], align 8
  br i1 %permissions.only, label %permissions, label %metadata.start
permissions:
  %permissions.prefix = call i64 @ncc.fs.append.literal(ptr %temporary, i64 0, ptr @ncc.fs.literal.readonly.start, i64 9)
  %permissions.bool = call i64 @ncc.fs.append.bool(ptr %temporary, i64 %permissions.prefix, i1 %readonly)
  %permissions.mode = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %permissions.bool, ptr @ncc.fs.literal.mode, i64 6)
  %permissions.end = call i64 @ncc.fs.append.u64(ptr %temporary, i64 %permissions.mode, i64 %mode)
  br label %emit
metadata.start:
  %type.prefix = call i64 @ncc.fs.append.literal(ptr %temporary, i64 0, ptr @ncc.fs.literal.type, i64 5)
  %is.file = icmp eq i32 %kind, 1
  %is.directory = icmp eq i32 %kind, 2
  %is.symlink = icmp eq i32 %kind, 3
  br i1 %is.file, label %type.file, label %type.not.file
type.file:
  %file.end = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %type.prefix, ptr @ncc.fs.literal.file, i64 4)
  br label %type.done
type.not.file:
  br i1 %is.directory, label %type.directory, label %type.not.directory
type.directory:
  %directory.end = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %type.prefix, ptr @ncc.fs.literal.directory, i64 9)
  br label %type.done
type.not.directory:
  br i1 %is.symlink, label %type.symlink, label %type.other
type.symlink:
  %symlink.end = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %type.prefix, ptr @ncc.fs.literal.symlink, i64 7)
  br label %type.done
type.other:
  %other.end = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %type.prefix, ptr @ncc.fs.literal.other, i64 5)
  br label %type.done
type.done:
  %type.end = phi i64 [ %file.end, %type.file ], [ %directory.end, %type.directory ], [ %symlink.end, %type.symlink ], [ %other.end, %type.other ]
  %length.prefix = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %type.end, ptr @ncc.fs.literal.length, i64 8)
  %length.end = call i64 @ncc.fs.append.u64(ptr %temporary, i64 %length.prefix, i64 %size)
  %readonly.prefix = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %length.end, ptr @ncc.fs.literal.readonly, i64 10)
  %readonly.end = call i64 @ncc.fs.append.bool(ptr %temporary, i64 %readonly.prefix, i1 %readonly)
  %modified.prefix = call i64 @ncc.fs.append.literal(ptr %temporary, i64 %readonly.end, ptr @ncc.fs.literal.modified, i64 10)
  %metadata.end = call i64 @ncc.fs.append.u64(ptr %temporary, i64 %modified.prefix, i64 %modified)
  br label %emit
emit:
  %length = phi i64 [ %permissions.end, %permissions ], [ %metadata.end, %type.done ]
  %bytes = add i64 %length, 1
  %has.output = icmp ne ptr %out, null
  %fits = icmp ule i64 %bytes, %capacity
  %can.write = and i1 %has.output, %fits
  br i1 %can.write, label %write, label %insufficient
write:
  %terminator = getelementptr i8, ptr %temporary, i64 %length
  store i8 0, ptr %terminator, align 1
  call void @ncc.fs.copy(ptr %out, ptr %temporary, i64 %bytes)
  ret i64 %length
insufficient:
  ret i64 -2
failure:
  ret i64 -1
}

define internal i64 @ncc.fs.ok.i64(i64 %value) { %shift = shl i64 %value, 1 %tag = or i64 %shift, 1 ret i64 %tag }
define internal i64 @ncc.fs.ok.ptr(ptr %value) { %raw = ptrtoint ptr %value to i64 %shift = shl i64 %raw, 1 %tag = or i64 %shift, 1 ret i64 %tag }
define internal i64 @ncc.fs.error(i64 %code) { %zero = icmp eq i64 %code, 0 %value = select i1 %zero, i64 1, i64 %code %tag = shl i64 %value, 1 ret i64 %tag }

define i64 @ncc_rt_fswrite(ptr %path, ptr %text) {
  %length = call i64 @ncc.fs.strlen(ptr %text)
  %written = call i64 @__ncc_fs_write(ptr %path, ptr %text, i64 %length, i1 false)
  %ok = icmp sge i64 %written, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 %written) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_fsappend(ptr %path, ptr %text) {
  %length = call i64 @ncc.fs.strlen(ptr %text)
  %written = call i64 @__ncc_fs_write(ptr %path, ptr %text, i64 %length, i1 true)
  %ok = icmp sge i64 %written, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 %written) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_fswrite_bytes(ptr %path, ptr %bytes, i64 %length) {
  %written = call i64 @__ncc_fs_write(ptr %path, ptr %bytes, i64 %length, i1 false)
  %ok = icmp sge i64 %written, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 %written) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_fsread_bytes(ptr %path, ptr %out, i64 %capacity) {
  %read = call i64 @__ncc_fs_read(ptr %path, ptr %out, i64 %capacity)
  %ok = icmp sge i64 %read, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 %read) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_fsync(ptr %path) {
  %status = call i32 @__ncc_fs_sync(ptr %path)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 1) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_fsread(ptr %path) {
entry:
  %length = call i64 @__ncc_fs_size(ptr %path)
  %valid = icmp sge i64 %length, 0
  br i1 %valid, label %allocate, label %failure.open
allocate:
  %bytes = add i64 %length, 1
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure.alloc, label %read
read:
  %count = call i64 @__ncc_fs_read(ptr %path, ptr %buffer, i64 %length)
  %ok = icmp sge i64 %count, 0
  br i1 %ok, label %terminate, label %failure.read
terminate:
  %end = getelementptr i8, ptr %buffer, i64 %count
  store i8 0, ptr %end, align 1
  %tag = call i64 @ncc.fs.ok.ptr(ptr %buffer)
  ret i64 %tag
failure.read:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %read.error = call i64 @ncc.fs.error(i64 3)
  ret i64 %read.error
failure.alloc:
  %alloc.error = call i64 @ncc.fs.error(i64 2)
  ret i64 %alloc.error
failure.open:
  %open.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %open.error
}

define internal i64 @ncc.fs.copy.file(ptr %source, ptr %destination) {
entry:
  %length = call i64 @__ncc_fs_size(ptr %source)
  %valid = icmp sge i64 %length, 0
  br i1 %valid, label %allocate, label %failure.size
allocate:
  %empty = icmp eq i64 %length, 0
  %bytes = select i1 %empty, i64 1, i64 %length
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure.alloc, label %read
read:
  %read.bytes = call i64 @__ncc_fs_read(ptr %source, ptr %buffer, i64 %length)
  %read.ok = icmp eq i64 %read.bytes, %length
  br i1 %read.ok, label %write, label %failure.read
write:
  %written = call i64 @__ncc_fs_write(ptr %destination, ptr %buffer, i64 %length, i1 false)
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %bytes.ok = icmp eq i64 %written, %length
  br i1 %bytes.ok, label %permissions, label %failure.write
permissions:
  %permissions.ok = call i1 @__ncc_fs_copy_permissions(ptr %source, ptr %destination)
  %write.ok = and i1 %bytes.ok, %permissions.ok
  br i1 %write.ok, label %success, label %failure.write
success:
  ret i64 %length
failure.read:
  %released.read = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  ret i64 -3
failure.write:
  ret i64 -4
failure.alloc:
  ret i64 -2
failure.size:
  ret i64 -1
}

define i64 @ncc_rt_copy_file(ptr %source, ptr %destination) {
entry:
  %copied = call i64 @ncc.fs.copy.file(ptr %source, ptr %destination)
  %ok = icmp sge i64 %copied, 0
  br i1 %ok, label %success, label %failure
success:
  %tag = call i64 @ncc.fs.ok.i64(i64 %copied)
  ret i64 %tag
failure:
  %code = sub i64 0, %copied
  %tag.error = call i64 @ncc.fs.error(i64 %code)
  ret i64 %tag.error
}

define i64 @ncc_rt_read_bytes(ptr %path) {
entry:
  %length = call i64 @__ncc_fs_size(ptr %path)
  %valid = icmp sge i64 %length, 0
  br i1 %valid, label %allocate.descriptor, label %failure.size
allocate.descriptor:
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 16, i64 8)
  %descriptor.missing = icmp eq ptr %descriptor, null
  br i1 %descriptor.missing, label %failure.alloc, label %allocate.data
allocate.data:
  %empty = icmp eq i64 %length, 0
  %bytes = select i1 %empty, i64 1, i64 %length
  %data = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %data.missing = icmp eq ptr %data, null
  br i1 %data.missing, label %failure.data, label %read
read:
  %read.bytes = call i64 @__ncc_fs_read(ptr %path, ptr %data, i64 %length)
  %read.ok = icmp eq i64 %read.bytes, %length
  br i1 %read.ok, label %bind, label %failure.read
bind:
  %data.slot = getelementptr %ncc.fs.array, ptr %descriptor, i32 0, i32 0
  %len.slot = getelementptr %ncc.fs.array, ptr %descriptor, i32 0, i32 1
  store ptr %data, ptr %data.slot, align 8
  store i64 %length, ptr %len.slot, align 8
  %tag = call i64 @ncc.fs.ok.ptr(ptr %descriptor)
  ret i64 %tag
failure.read:
  %released.read.data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released.read.descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  %tag.read = call i64 @ncc.fs.error(i64 3)
  ret i64 %tag.read
failure.data:
  %released.descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  %tag.data = call i64 @ncc.fs.error(i64 2)
  ret i64 %tag.data
failure.alloc:
  %tag.alloc = call i64 @ncc.fs.error(i64 2)
  ret i64 %tag.alloc
failure.size:
  %tag.size = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.size
}

define internal i64 @ncc.fs.describe.result(ptr %path, i1 %permissions.only) {
entry:
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 256, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure.alloc, label %describe
describe:
  %length = call i64 @ncc.fs.describe(ptr %path, ptr %buffer, i64 256, i1 %permissions.only)
  %ok = icmp sge i64 %length, 0
  br i1 %ok, label %success, label %failure.describe
success:
  %tag = call i64 @ncc.fs.ok.ptr(ptr %buffer)
  ret i64 %tag
failure.describe:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %code = sub i64 0, %length
  %tag.describe = call i64 @ncc.fs.error(i64 %code)
  ret i64 %tag.describe
failure.alloc:
  %tag.alloc = call i64 @ncc.fs.error(i64 3)
  ret i64 %tag.alloc
}

define i64 @ncc_rt_metadata(ptr %path) {
  %result = call i64 @ncc.fs.describe.result(ptr %path, i1 false)
  ret i64 %result
}

define i64 @ncc_rt_permissions(ptr %path) {
  %result = call i64 @ncc.fs.describe.result(ptr %path, i1 true)
  ret i64 %result
}

define i64 @ncc_rt_set_readonly(ptr %path, i1 %readonly) {
entry:
  %status = call i32 @__ncc_fs_set_readonly(ptr %path, i1 %readonly)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success:
  %tag = call i64 @ncc.fs.ok.i64(i64 1)
  ret i64 %tag
failure:
  %tag.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.error
}

define i1 @ncc_rt_exists(ptr %path) { %v = call i32 @__ncc_fs_exists(ptr %path) %r = icmp ne i32 %v, 0 ret i1 %r }
define i32 @ncc_rt_fs_delete(ptr %path) { %v = call i32 @__ncc_fs_delete(ptr %path) ret i32 %v }
define i32 @ncc_rt_fs_rename(ptr %from, ptr %to) { %v = call i32 @__ncc_fs_rename(ptr %from, ptr %to) ret i32 %v }

define i64 @ncc_rt_create_dir(ptr %path) {
  %status = call i32 @__ncc_fs_create_dir(ptr %path)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 1) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}
define i64 @ncc_rt_remove_dir(ptr %path) {
  %status = call i32 @__ncc_fs_remove_dir(ptr %path)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 1) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}
define i64 @ncc_rt_change_dir(ptr %path) {
  %status = call i32 @__ncc_fs_change_dir(ptr %path)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success: %tag = call i64 @ncc.fs.ok.i64(i64 1) ret i64 %tag
failure: %tag.error = call i64 @ncc.fs.error(i64 1) ret i64 %tag.error
}

define i64 @ncc_rt_current_dir() {
entry:
  br label %attempt
attempt:
  %capacity = phi i64 [ 256, %entry ], [ %next.capacity, %grow ]
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure, label %read
read:
  %length = call i64 @__ncc_fs_current_dir_into(ptr %buffer, i64 %capacity)
  %ok = icmp sge i64 %length, 0
  br i1 %ok, label %success, label %inspect
inspect:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %can.grow = icmp ult i64 %capacity, 16777216
  br i1 %can.grow, label %grow, label %failure
grow:
  %next.capacity = shl i64 %capacity, 1
  br label %attempt
success:
  %tag = call i64 @ncc.fs.ok.ptr(ptr %buffer)
  ret i64 %tag
failure:
  %tag.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.error
}

define internal i64 @ncc.fs.path.result(ptr %path, i1 %official) {
entry:
  br label %attempt
attempt:
  %capacity = phi i64 [ 256, %entry ], [ %next.capacity, %grow ]
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure.alloc, label %select
select:
  br i1 %official, label %official.call, label %absolute.call
absolute.call:
  %absolute.length = call i64 @__ncc_fs_absolute_path_into(ptr %path, ptr %buffer, i64 %capacity)
  br label %inspect
official.call:
  %official.length = call i64 @__ncc_fs_official_path_into(ptr %path, ptr %buffer, i64 %capacity)
  br label %inspect
inspect:
  %length = phi i64 [ %absolute.length, %absolute.call ], [ %official.length, %official.call ]
  %ok = icmp sge i64 %length, 0
  br i1 %ok, label %success, label %release.failure
release.failure:
  %released.failure = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %too.small = icmp eq i64 %length, -2
  %can.grow = icmp ult i64 %capacity, 16777216
  %retry = and i1 %too.small, %can.grow
  br i1 %retry, label %grow, label %failure.resolve
grow:
  %next.capacity = shl i64 %capacity, 1
  br label %attempt
success:
  %tag = call i64 @ncc.fs.ok.ptr(ptr %buffer)
  ret i64 %tag
failure.resolve:
  %code = sub i64 0, %length
  %tag.resolve = call i64 @ncc.fs.error(i64 %code)
  ret i64 %tag.resolve
failure.alloc:
  %tag.alloc = call i64 @ncc.fs.error(i64 4)
  ret i64 %tag.alloc
}

define i64 @ncc_rt_absolute_path(ptr %path) {
  %result = call i64 @ncc.fs.path.result(ptr %path, i1 false)
  ret i64 %result
}

define i64 @ncc_rt_official_path(ptr %path) {
  %result = call i64 @ncc.fs.path.result(ptr %path, i1 true)
  ret i64 %result
}

define i64 @ncc_rt_lock_file(ptr %path) {
entry:
  %handle = call ptr @__ncc_fs_lock(ptr %path, i1 true, i1 true)
  %ok = icmp ne ptr %handle, null
  br i1 %ok, label %success, label %failure
success:
  %tag = call i64 @ncc.fs.ok.ptr(ptr %handle)
  ret i64 %tag
failure:
  %tag.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.error
}

define i64 @ncc_rt_unlock_file(ptr %handle) {
entry:
  %status = call i32 @__ncc_fs_unlock(ptr %handle)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success:
  %tag = call i64 @ncc.fs.ok.i64(i64 1)
  ret i64 %tag
failure:
  %tag.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.error
}

define internal void @ncc.fs.list.drop(ptr %list) {
entry:
  %missing = icmp eq ptr %list, null
  br i1 %missing, label %done, label %load
load:
  %data.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 0
  %len.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 1
  %data = load ptr, ptr %data.slot, align 1
  %len = load i64, ptr %len.slot, align 8
  br label %loop
loop:
  %index = phi i64 [ 0, %load ], [ %next, %release ]
  %complete = icmp uge i64 %index, %len
  br i1 %complete, label %release.data, label %release
release:
  %slot = getelementptr ptr, ptr %data, i64 %index
  %item = load ptr, ptr %slot, align 1
  %released.item = call i1 @__ncc_raw_aligned_free(ptr %item)
  %next = add i64 %index, 1
  br label %loop
release.data:
  %released.data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released.list = call i1 @__ncc_raw_aligned_free(ptr %list)
  br label %done
done:
  ret void
}

define internal ptr @ncc.fs.split(ptr %text, i64 %length) {
entry:
  %list = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %missing = icmp eq ptr %list, null
  br i1 %missing, label %failure, label %initialize
initialize:
  %data.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 0
  %len.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 1
  %cap.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 2
  store ptr null, ptr %data.slot, align 1
  store i64 0, ptr %len.slot, align 8
  store i64 0, ptr %cap.slot, align 8
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %success, label %count.entry
count.entry:
  br label %count
count:
  %scan = phi i64 [ 0, %count.entry ], [ %scan.next, %count.more ]
  %lines = phi i64 [ 1, %count.entry ], [ %lines.next, %count.more ]
  %cursor = getelementptr i8, ptr %text, i64 %scan
  %byte = load i8, ptr %cursor, align 1
  %newline = icmp eq i8 %byte, 10
  %candidate = zext i1 %newline to i64
  %lines.next = add i64 %lines, %candidate
  %scan.next = add i64 %scan, 1
  %scan.done = icmp uge i64 %scan.next, %length
  br i1 %scan.done, label %count.done, label %count.more
count.more:
  br label %count
count.done:
  %last.index = sub i64 %length, 1
  %last.ptr = getelementptr i8, ptr %text, i64 %last.index
  %last = load i8, ptr %last.ptr, align 1
  %trailing = icmp eq i8 %last, 10
  %trailing.value = zext i1 %trailing to i64
  %count.final = sub i64 %lines.next, %trailing.value
  %ptr.one = getelementptr ptr, ptr null, i64 1
  %ptr.bytes = ptrtoint ptr %ptr.one to i64
  %table.bytes = mul i64 %count.final, %ptr.bytes
  %data = call ptr @__ncc_raw_aligned_alloc(i64 %table.bytes, i64 8)
  %no.data = icmp eq ptr %data, null
  br i1 %no.data, label %failure.list, label %bind
bind:
  store ptr %data, ptr %data.slot, align 1
  store i64 %count.final, ptr %cap.slot, align 8
  br label %segment
segment:
  %index = phi i64 [ 0, %bind ], [ %index.next, %segment.continue ]
  %start = phi i64 [ 0, %bind ], [ %start.next, %segment.continue ]
  %out.index = phi i64 [ 0, %bind ], [ %out.next, %segment.continue ]
  %at.end = icmp uge i64 %index, %length
  br i1 %at.end, label %segment.final, label %segment.inspect
segment.inspect:
  %char.ptr = getelementptr i8, ptr %text, i64 %index
  %char = load i8, ptr %char.ptr, align 1
  %delimiter = icmp eq i8 %char, 10
  br i1 %delimiter, label %segment.emit, label %segment.advance
segment.advance:
  %index.more = add i64 %index, 1
  br label %segment.continue
segment.final:
  %has.final = icmp ult i64 %start, %length
  br i1 %has.final, label %segment.emit, label %success
segment.emit:
  %end = phi i64 [ %index, %segment.inspect ], [ %length, %segment.final ]
  %segment.length = sub i64 %end, %start
  %segment.bytes = add i64 %segment.length, 1
  %item = call ptr @__ncc_raw_aligned_alloc(i64 %segment.bytes, i64 8)
  %no.item = icmp eq ptr %item, null
  br i1 %no.item, label %failure.list, label %segment.copy
segment.copy:
  %source = getelementptr i8, ptr %text, i64 %start
  call void @ncc.fs.copy(ptr %item, ptr %source, i64 %segment.length)
  %term = getelementptr i8, ptr %item, i64 %segment.length
  store i8 0, ptr %term, align 1
  %item.slot = getelementptr ptr, ptr %data, i64 %out.index
  store ptr %item, ptr %item.slot, align 1
  %out.after = add i64 %out.index, 1
  store i64 %out.after, ptr %len.slot, align 8
  %after.end = add i64 %end, 1
  br label %segment.continue
segment.continue:
  %index.next = phi i64 [ %index.more, %segment.advance ], [ %after.end, %segment.copy ]
  %start.next = phi i64 [ %start, %segment.advance ], [ %after.end, %segment.copy ]
  %out.next = phi i64 [ %out.index, %segment.advance ], [ %out.after, %segment.copy ]
  br label %segment
success:
  ret ptr %list
failure.list:
  call void @ncc.fs.list.drop(ptr %list)
  ret ptr null
failure:
  ret ptr null
}

define i64 @ncc_rt_list_dir(ptr %path) {
entry:
  %needed = call i64 @__ncc_fs_list_dir_into(ptr %path, ptr null, i64 0)
  %valid = icmp sge i64 %needed, 0
  br i1 %valid, label %allocate, label %failure
allocate:
  %bytes = add i64 %needed, 1
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %missing = icmp eq ptr %buffer, null
  br i1 %missing, label %failure, label %read
read:
  %actual = call i64 @__ncc_fs_list_dir_into(ptr %path, ptr %buffer, i64 %bytes)
  %ok = icmp sge i64 %actual, 0
  br i1 %ok, label %split, label %failure.buffer
split:
  %list = call ptr @ncc.fs.split(ptr %buffer, i64 %actual)
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %has.list = icmp ne ptr %list, null
  br i1 %has.list, label %success, label %failure
success:
  %tag = call i64 @ncc.fs.ok.ptr(ptr %list)
  ret i64 %tag
failure.buffer:
  %released.failure = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  br label %failure
failure:
  %tag.error = call i64 @ncc.fs.error(i64 1)
  ret i64 %tag.error
}

define i32 @ncc_rt_fread_lines_into(ptr %path, ptr %out) {
entry:
  %data.slot = getelementptr %ncc.fs.array, ptr %out, i32 0, i32 0
  %len.slot = getelementptr %ncc.fs.array, ptr %out, i32 0, i32 1
  store ptr null, ptr %data.slot, align 1
  store i64 0, ptr %len.slot, align 8
  %tagged = call i64 @ncc_rt_fsread(ptr %path)
  %tag = and i64 %tagged, 1
  %ok = icmp eq i64 %tag, 1
  br i1 %ok, label %decode, label %failure
decode:
  %payload = lshr i64 %tagged, 1
  %text = inttoptr i64 %payload to ptr
  %length = call i64 @ncc.fs.strlen(ptr %text)
  %list = call ptr @ncc.fs.split(ptr %text, i64 %length)
  %released.text = call i1 @__ncc_raw_aligned_free(ptr %text)
  %has.list = icmp ne ptr %list, null
  br i1 %has.list, label %take, label %failure
take:
  %list.data.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 0
  %list.len.slot = getelementptr %ncc.fs.list, ptr %list, i32 0, i32 1
  %data = load ptr, ptr %list.data.slot, align 1
  %len = load i64, ptr %list.len.slot, align 8
  store ptr %data, ptr %data.slot, align 1
  store i64 %len, ptr %len.slot, align 8
  %released.list = call i1 @__ncc_raw_aligned_free(ptr %list)
  ret i32 1
failure:
  ret i32 0
}

define i32 @ncc_rt_fs_release(ptr %address) {
  %released = call i1 @__ncc_raw_aligned_free(ptr %address)
  %result = zext i1 %released to i32
  ret i32 %result
}

define i32 @ncc_rt_fs_text_release(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %ok = icmp eq i64 %tag, 1
  br i1 %ok, label %release, label %failure
release:
  %payload = lshr i64 %tagged, 1
  %text = inttoptr i64 %payload to ptr
  %released = call i1 @__ncc_raw_aligned_free(ptr %text)
  %result = zext i1 %released to i32
  ret i32 %result
failure:
  ret i32 0
}

define i32 @ncc_rt_fs_bytes_release(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %ok = icmp eq i64 %tag, 1
  br i1 %ok, label %release, label %failure
release:
  %payload = lshr i64 %tagged, 1
  %descriptor = inttoptr i64 %payload to ptr
  %data.slot = getelementptr %ncc.fs.array, ptr %descriptor, i32 0, i32 0
  %data = load ptr, ptr %data.slot, align 8
  %released.data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released.descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  %released = and i1 %released.data, %released.descriptor
  %result = zext i1 %released to i32
  ret i32 %result
failure:
  ret i32 0
}

define i32 @ncc_rt_fs_list_release(i64 %tagged) {
entry:
  %tag = and i64 %tagged, 1
  %ok = icmp eq i64 %tag, 1
  br i1 %ok, label %release, label %failure
release:
  %payload = lshr i64 %tagged, 1
  %list = inttoptr i64 %payload to ptr
  call void @ncc.fs.list.drop(ptr %list)
  ret i32 1
failure:
  ret i32 0
}

define i32 @ncc_rt_fs_lines_release(ptr %out) {
entry:
  %data.slot = getelementptr %ncc.fs.array, ptr %out, i32 0, i32 0
  %len.slot = getelementptr %ncc.fs.array, ptr %out, i32 0, i32 1
  %data = load ptr, ptr %data.slot, align 1
  %len = load i64, ptr %len.slot, align 8
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %release ]
  %done = icmp uge i64 %index, %len
  br i1 %done, label %release.data, label %release
release:
  %slot = getelementptr ptr, ptr %data, i64 %index
  %item = load ptr, ptr %slot, align 1
  %released.item = call i1 @__ncc_raw_aligned_free(ptr %item)
  %next = add i64 %index, 1
  br label %loop
release.data:
  %released.data = call i1 @__ncc_raw_aligned_free(ptr %data)
  store ptr null, ptr %data.slot, align 1
  store i64 0, ptr %len.slot, align 8
  ret i32 1
}

define i64 @ncc_rt_fs_write_from(ptr %path, ptr %data, i64 %length, i1 %append) { %r = call i64 @__ncc_fs_write(ptr %path, ptr %data, i64 %length, i1 %append) ret i64 %r }
define i64 @ncc_rt_fs_read_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @__ncc_fs_read(ptr %path, ptr %out, i64 %capacity) ret i64 %r }
define i64 @ncc_rt_fs_size(ptr %path) { %r = call i64 @__ncc_fs_size(ptr %path) ret i64 %r }
define i64 @ncc_rt_fs_copy(ptr %source, ptr %destination) { %r = call i64 @ncc.fs.copy.file(ptr %source, ptr %destination) ret i64 %r }
define i64 @ncc_rt_fs_list_dir_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @__ncc_fs_list_dir_into(ptr %path, ptr %out, i64 %capacity) ret i64 %r }
define i64 @ncc_rt_fs_current_dir_into(ptr %out, i64 %capacity) { %r = call i64 @__ncc_fs_current_dir_into(ptr %out, i64 %capacity) ret i64 %r }
define i64 @ncc_rt_fs_absolute_path_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @__ncc_fs_absolute_path_into(ptr %path, ptr %out, i64 %capacity) ret i64 %r }
define i64 @ncc_rt_fs_official_path_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @__ncc_fs_official_path_into(ptr %path, ptr %out, i64 %capacity) ret i64 %r }
define i64 @ncc_rt_fs_metadata_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @ncc.fs.describe(ptr %path, ptr %out, i64 %capacity, i1 false) ret i64 %r }
define i64 @ncc_rt_fs_permissions_into(ptr %path, ptr %out, i64 %capacity) { %r = call i64 @ncc.fs.describe(ptr %path, ptr %out, i64 %capacity, i1 true) ret i64 %r }
define i32 @ncc_rt_fs_set_readonly(ptr %path, i1 %readonly) { %r = call i32 @__ncc_fs_set_readonly(ptr %path, i1 %readonly) ret i32 %r }
define ptr @ncc_rt_fs_lock(ptr %path, i1 %exclusive, i1 %wait) { %r = call ptr @__ncc_fs_lock(ptr %path, i1 %exclusive, i1 %wait) ret ptr %r }
define i32 @ncc_rt_fs_unlock(ptr %handle) { %r = call i32 @__ncc_fs_unlock(ptr %handle) ret i32 %r }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_targets_materialize_without_c_provider() {
        for target in [
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "i386-unknown-linux-gnu",
            "aarch64-apple-darwin",
        ] {
            let ir = render_filesystem_runtime(target).expect(target);
            for definition in [
                "define i64 @ncc_rt_fswrite",
                "define i64 @ncc_rt_fsread",
                "define i64 @ncc_rt_list_dir",
                "define i64 @ncc_rt_fs_read_into",
                "define i64 @ncc_rt_fs_list_dir_into",
                "define i64 @ncc_rt_fs_absolute_path_into",
                "define i64 @ncc_rt_fs_official_path_into",
                "define ptr @ncc_rt_fs_lock",
                "define i32 @ncc_rt_fs_unlock",
                "define i32 @ncc_rt_fs_text_release",
                "define i64 @ncc_rt_fs_copy",
                "define i1 @__ncc_fs_copy_permissions",
                "define i64 @ncc_rt_read_bytes",
                "define i32 @ncc_rt_fs_bytes_release",
                "define i64 @ncc_rt_fs_metadata_into",
                "define i64 @ncc_rt_fs_permissions_into",
                "define i32 @ncc_rt_fs_set_readonly",
                "define i64 @ncc_rt_metadata",
                "define i64 @ncc_rt_permissions",
                "define i64 @ncc_rt_set_readonly",
            ] {
                assert!(ir.contains(definition), "{target}: missing {definition}");
            }
            for forbidden in ["@fopen(", "@fread(", "@fwrite(", "@malloc(", "@realloc("] {
                assert!(!ir.contains(forbidden), "{target}: found {forbidden}");
            }
            assert!(!ir.contains("@@"), "{target}: unresolved provider token");
            if target.contains("windows") {
                assert!(ir.contains("@LockFileEx"));
                assert!(ir.contains("@UnlockFileEx"));
                assert!(ir.contains("@GetFileAttributesExA"));
                assert!(ir.contains("@SetFileAttributesA"));
            } else {
                assert!(ir.contains("@flock"));
                assert!(ir.contains("@chmod"));
            }
        }
    }

    #[test]
    fn posix_write_and_lock_open_modes_include_material_access_bits() {
        let linux = render_filesystem_runtime("x86_64-unknown-linux-gnu").expect("linux");
        assert!(linux.contains("i32 577"), "Linux write open lacks O_WRONLY");
        assert!(
            linux.contains("select i1 %append, i32 1089, i32 577"),
            "Linux append open lacks O_APPEND"
        );
        assert!(linux.contains("i32 66"), "Linux lock open lacks O_RDWR");
        assert!(linux.contains("and i32 %mode.raw, 4095"));
        assert!(linux.contains("@chmod(ptr %destination, i32 %mode)"));

        let darwin = render_filesystem_runtime("aarch64-apple-darwin").expect("darwin");
        assert!(
            darwin.contains("i32 1537"),
            "Darwin write open lacks O_WRONLY"
        );
        assert!(
            darwin.contains("select i1 %append, i32 521, i32 1537"),
            "Darwin append open lacks O_APPEND"
        );
        assert!(darwin.contains("i32 514"), "Darwin lock open lacks O_RDWR");
    }

    #[test]
    fn wasi_is_rejected_instead_of_claiming_a_host_filesystem() {
        assert!(render_filesystem_runtime("wasm32-wasi").is_err());
    }
}
