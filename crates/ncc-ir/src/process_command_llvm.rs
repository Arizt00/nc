pub(crate) fn render_process_command_runtime(
    target: &str,
    explicit_environment: bool,
) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64")) {
        return Ok(WIN64_PROCESS_COMMAND_RUNTIME.to_string());
    }
    if lower.contains("linux") || lower.contains("android") || lower.contains("darwin") {
        let pointer_bytes = if lower.contains("i386")
            || lower.contains("i486")
            || lower.contains("i586")
            || lower.contains("i686")
            || lower.contains("armv7")
            || lower.contains("thumbv7")
            || lower.contains("riscv32")
        {
            "4"
        } else {
            "8"
        };
        let mut runtime = POSIX_PROCESS_COMMAND_RUNTIME.replace("@POINTER_BYTES@", pointer_bytes);
        if explicit_environment && lower.contains("linux") {
            runtime = runtime
                .replace(
                    "spawn:\n  %pid = call i32 @fork()",
                    "spawn:\n  call void @__ncc_env_platform_seed()\n  %pid = call i32 @fork()",
                )
                .replace("declare i32 @execvp(ptr, ptr)", "declare i32 @execvpe(ptr, ptr, ptr)")
                .replace(
                    "  %executed = call i32 @execvp(ptr %program, ptr %argv)",
                    // execvpe searches the caller's PATH, not the supplied envp.
                    // This executes after fork, so publishing the snapshot is child-local.
                    "  %environment = load ptr, ptr @__ncc_process_environ, align 1\n  store ptr %environment, ptr @environ, align 1\n  %executed = call i32 @execvpe(ptr %program, ptr %argv, ptr %environment)",
                );
        }
        return Ok(runtime);
    }
    Err(format!(
        "native raw process-command backend has no OS process ABI for target '{target}'"
    ))
}

const WIN64_PROCESS_COMMAND_RUNTIME: &str = r#"
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @CreateProcessW(ptr, ptr, ptr, ptr, i32, i32, ptr, ptr, ptr, ptr)
declare i32 @WaitForSingleObject(ptr, i32)
declare i32 @GetExitCodeProcess(ptr, ptr)
declare i32 @TerminateProcess(ptr, i32)
declare i32 @CreatePipe(ptr, ptr, ptr, i32)
declare i32 @SetHandleInformation(ptr, i32, i32)
declare i32 @ReadFile(ptr, ptr, i32, ptr, ptr)
declare i32 @WriteFile(ptr, ptr, i32, ptr, ptr)
declare ptr @GetStdHandle(i32)

define internal ptr @__ncc_command_utf8_to_wide(ptr %text) {
entry:
  %is_null = icmp eq ptr %text, null
  br i1 %is_null, label %bad, label %measure
measure:
  %chars = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr null, i32 0)
  %chars_ok = icmp sgt i32 %chars, 0
  br i1 %chars_ok, label %allocate, label %bad
allocate:
  %chars64 = zext i32 %chars to i64
  %bytes = shl i64 %chars64, 1
  %wide = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %wide_ok = icmp ne ptr %wide, null
  br i1 %wide_ok, label %convert, label %bad
convert:
  %written = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr %wide, i32 %chars)
  %converted = icmp eq i32 %written, %chars
  br i1 %converted, label %ok, label %free_bad
free_bad:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  br label %bad
ok:
  ret ptr %wide
bad:
  ret ptr null
}

define internal ptr @__ncc_command_set_error(ptr %descriptor, i64 %error) {
entry:
  %error_slot = getelementptr i8, ptr %descriptor, i64 32
  store i64 %error, ptr %error_slot, align 8
  ret ptr %descriptor
}

define ptr @__ncc_command_open(ptr %command, ptr %cwd, ptr %stdin_handle, ptr %stdout_handle, ptr %stderr_handle, i64 %flags64) {
entry:
  %startup = alloca [104 x i8], align 8
  %process_info = alloca [24 x i8], align 8
  %cwd_wide_slot = alloca ptr, align 8
  store ptr null, ptr %cwd_wide_slot, align 8
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 48, i64 8)
  %descriptor_ok = icmp ne ptr %descriptor, null
  br i1 %descriptor_ok, label %initialize, label %out_of_memory
initialize:
  call void @llvm.memset.p0.i64(ptr %descriptor, i8 0, i64 48, i1 false)
  call void @llvm.memset.p0.i64(ptr %startup, i8 0, i64 104, i1 false)
  call void @llvm.memset.p0.i64(ptr %process_info, i8 0, i64 24, i1 false)
  store i32 104, ptr %startup, align 8
  %command_wide = call ptr @__ncc_command_utf8_to_wide(ptr %command)
  %command_ok = icmp ne ptr %command_wide, null
  br i1 %command_ok, label %cwd_check, label %convert_error
cwd_check:
  %cwd_null = icmp eq ptr %cwd, null
  br i1 %cwd_null, label %prepare, label %cwd_content
cwd_content:
  %cwd_first = load i8, ptr %cwd, align 1
  %cwd_empty = icmp eq i8 %cwd_first, 0
  br i1 %cwd_empty, label %prepare, label %cwd_convert
cwd_convert:
  %cwd_wide = call ptr @__ncc_command_utf8_to_wide(ptr %cwd)
  store ptr %cwd_wide, ptr %cwd_wide_slot, align 8
  %cwd_ok = icmp ne ptr %cwd_wide, null
  br i1 %cwd_ok, label %prepare, label %cwd_error
prepare:
  %stdin_present = icmp ne ptr %stdin_handle, null
  %stdout_present = icmp ne ptr %stdout_handle, null
  %stderr_present = icmp ne ptr %stderr_handle, null
  %output_present = or i1 %stdout_present, %stderr_present
  %redirected = or i1 %stdin_present, %output_present
  %stdin_default = call ptr @GetStdHandle(i32 -10)
  %stdout_default = call ptr @GetStdHandle(i32 -11)
  %stderr_default = call ptr @GetStdHandle(i32 -12)
  %stdin_native = select i1 %stdin_present, ptr %stdin_handle, ptr %stdin_default
  %stdout_native = select i1 %stdout_present, ptr %stdout_handle, ptr %stdout_default
  %stderr_native = select i1 %stderr_present, ptr %stderr_handle, ptr %stderr_default
  %startup_flags = select i1 %redirected, i32 256, i32 0
  %startup_flags_slot = getelementptr i8, ptr %startup, i64 60
  %stdin_slot = getelementptr i8, ptr %startup, i64 80
  %stdout_slot = getelementptr i8, ptr %startup, i64 88
  %stderr_slot = getelementptr i8, ptr %startup, i64 96
  store i32 %startup_flags, ptr %startup_flags_slot, align 4
  store ptr %stdin_native, ptr %stdin_slot, align 8
  store ptr %stdout_native, ptr %stdout_slot, align 8
  store ptr %stderr_native, ptr %stderr_slot, align 8
  %inherit = zext i1 %redirected to i32
  %flags = trunc i64 %flags64 to i32
  %cwd_native = load ptr, ptr %cwd_wide_slot, align 8
  %created = call i32 @CreateProcessW(ptr null, ptr %command_wide, ptr null, ptr null, i32 %inherit, i32 %flags, ptr null, ptr %cwd_native, ptr %startup, ptr %process_info)
  %created_ok = icmp ne i32 %created, 0
  br i1 %created_ok, label %record, label %create_error
record:
  %process_handle = load ptr, ptr %process_info, align 8
  %thread_handle_slot = getelementptr i8, ptr %process_info, i64 8
  %thread_handle = load ptr, ptr %thread_handle_slot, align 8
  %pid_slot_native = getelementptr i8, ptr %process_info, i64 16
  %tid_slot_native = getelementptr i8, ptr %process_info, i64 20
  %pid32 = load i32, ptr %pid_slot_native, align 4
  %tid32 = load i32, ptr %tid_slot_native, align 4
  %pid = zext i32 %pid32 to i64
  %tid = zext i32 %tid32 to i64
  %thread_slot = getelementptr i8, ptr %descriptor, i64 8
  %pid_slot = getelementptr i8, ptr %descriptor, i64 16
  %tid_slot = getelementptr i8, ptr %descriptor, i64 24
  %flags_slot = getelementptr i8, ptr %descriptor, i64 40
  store ptr %process_handle, ptr %descriptor, align 8
  store ptr %thread_handle, ptr %thread_slot, align 8
  store i64 %pid, ptr %pid_slot, align 8
  store i64 %tid, ptr %tid_slot, align 8
  store i64 %flags64, ptr %flags_slot, align 8
  br label %release_inputs
create_error:
  %create_code32 = call i32 @GetLastError()
  %create_code = zext i32 %create_code32 to i64
  %failed_descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %create_code)
  br label %release_inputs
release_inputs:
  %result = phi ptr [ %descriptor, %record ], [ %failed_descriptor, %create_error ]
  %released_command = call i1 @__ncc_raw_aligned_free(ptr %command_wide)
  %cwd_to_free = load ptr, ptr %cwd_wide_slot, align 8
  %released_cwd = call i1 @__ncc_raw_aligned_free(ptr %cwd_to_free)
  ret ptr %result
cwd_error:
  %cwd_code32 = call i32 @GetLastError()
  %cwd_code = zext i32 %cwd_code32 to i64
  %released_command_on_cwd_error = call i1 @__ncc_raw_aligned_free(ptr %command_wide)
  %cwd_failed_descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %cwd_code)
  ret ptr %cwd_failed_descriptor
convert_error:
  %convert_code32 = call i32 @GetLastError()
  %convert_code = zext i32 %convert_code32 to i64
  %convert_failed_descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %convert_code)
  ret ptr %convert_failed_descriptor
out_of_memory:
  ret ptr null
}

define i1 @__ncc_command_started(ptr %descriptor) {
entry:
  %handle = load ptr, ptr %descriptor, align 8
  %started = icmp ne ptr %handle, null
  ret i1 %started
}

define i64 @__ncc_command_wait(ptr %descriptor, i64 %timeout64) {
entry:
  %handle = load ptr, ptr %descriptor, align 8
  %timeout = trunc i64 %timeout64 to i32
  %status32 = call i32 @WaitForSingleObject(ptr %handle, i32 %timeout)
  %failed = icmp eq i32 %status32, -1
  br i1 %failed, label %error, label %ok
ok:
  %status = zext i32 %status32 to i64
  ret i64 %status
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %code)
  %negative = sub i64 0, %code
  ret i64 %negative
}

define i64 @__ncc_command_exit_code(ptr %descriptor) {
entry:
  %status_slot = alloca i32, align 4
  %handle = load ptr, ptr %descriptor, align 8
  %read = call i32 @GetExitCodeProcess(ptr %handle, ptr %status_slot)
  %read_ok = icmp ne i32 %read, 0
  br i1 %read_ok, label %ok, label %error
ok:
  %status32 = load i32, ptr %status_slot, align 4
  %status = zext i32 %status32 to i64
  ret i64 %status
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %code)
  %negative = sub i64 0, %code
  ret i64 %negative
}

define i1 @__ncc_command_terminate(ptr %descriptor, i64 %exit_code64) {
entry:
  %handle = load ptr, ptr %descriptor, align 8
  %exit_code = trunc i64 %exit_code64 to i32
  %terminated = call i32 @TerminateProcess(ptr %handle, i32 %exit_code)
  %ok = icmp ne i32 %terminated, 0
  br i1 %ok, label %done, label %error
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_set_error(ptr %descriptor, i64 %code)
  br label %done
done:
  ret i1 %ok
}

define ptr @__ncc_command_process_handle(ptr %descriptor) {
entry:
  %handle = load ptr, ptr %descriptor, align 8
  ret ptr %handle
}

define ptr @__ncc_command_thread_handle(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 8
  %handle = load ptr, ptr %slot, align 8
  ret ptr %handle
}

define i64 @__ncc_command_id(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 16
  %id = load i64, ptr %slot, align 8
  ret i64 %id
}

define i64 @__ncc_command_thread_id(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 24
  %id = load i64, ptr %slot, align 8
  ret i64 %id
}

define i64 @__ncc_command_error(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 32
  %error = load i64, ptr %slot, align 8
  ret i64 %error
}

define i1 @__ncc_command_release(ptr %descriptor) {
entry:
  %process = load ptr, ptr %descriptor, align 8
  %thread_slot = getelementptr i8, ptr %descriptor, i64 8
  %thread = load ptr, ptr %thread_slot, align 8
  %thread_present = icmp ne ptr %thread, null
  br i1 %thread_present, label %close_thread, label %process_check
close_thread:
  %thread_closed32 = call i32 @CloseHandle(ptr %thread)
  %thread_closed = icmp ne i32 %thread_closed32, 0
  br label %process_check
process_check:
  %thread_ok = phi i1 [ true, %entry ], [ %thread_closed, %close_thread ]
  %process_present = icmp ne ptr %process, null
  br i1 %process_present, label %close_process, label %release
close_process:
  %process_closed32 = call i32 @CloseHandle(ptr %process)
  %process_closed = icmp ne i32 %process_closed32, 0
  br label %release
release:
  %process_ok = phi i1 [ true, %process_check ], [ %process_closed, %close_process ]
  %handles_ok = and i1 %thread_ok, %process_ok
  %memory_released = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  %ok = and i1 %handles_ok, %memory_released
  ret i1 %ok
}

define internal ptr @__ncc_command_pipe_set_error(ptr %pipe, i64 %error) {
entry:
  %slot = getelementptr i8, ptr %pipe, i64 16
  store i64 %error, ptr %slot, align 8
  ret ptr %pipe
}

define ptr @__ncc_command_pipe_open(i1 %inherit_read, i1 %inherit_write) {
entry:
  %read_slot = alloca ptr, align 8
  %write_slot = alloca ptr, align 8
  %security = alloca [24 x i8], align 8
  store ptr null, ptr %read_slot, align 8
  store ptr null, ptr %write_slot, align 8
  call void @llvm.memset.p0.i64(ptr %security, i8 0, i64 24, i1 false)
  store i32 24, ptr %security, align 8
  %inherit_slot = getelementptr i8, ptr %security, i64 16
  store i32 1, ptr %inherit_slot, align 8
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %descriptor_ok = icmp ne ptr %descriptor, null
  br i1 %descriptor_ok, label %initialize, label %out_of_memory
initialize:
  call void @llvm.memset.p0.i64(ptr %descriptor, i8 0, i64 24, i1 false)
  %created = call i32 @CreatePipe(ptr %read_slot, ptr %write_slot, ptr %security, i32 0)
  %created_ok = icmp ne i32 %created, 0
  br i1 %created_ok, label %configure_read, label %create_error
configure_read:
  %read = load ptr, ptr %read_slot, align 8
  %read_value = zext i1 %inherit_read to i32
  %read_set = call i32 @SetHandleInformation(ptr %read, i32 1, i32 %read_value)
  %read_ok = icmp ne i32 %read_set, 0
  br i1 %read_ok, label %configure_write, label %configuration_error
configure_write:
  %write = load ptr, ptr %write_slot, align 8
  %write_value = zext i1 %inherit_write to i32
  %write_set = call i32 @SetHandleInformation(ptr %write, i32 1, i32 %write_value)
  %write_ok = icmp ne i32 %write_set, 0
  br i1 %write_ok, label %record, label %configuration_error
record:
  %descriptor_write_slot = getelementptr i8, ptr %descriptor, i64 8
  store ptr %read, ptr %descriptor, align 8
  store ptr %write, ptr %descriptor_write_slot, align 8
  ret ptr %descriptor
configuration_error:
  %configuration_code32 = call i32 @GetLastError()
  %configuration_code = zext i32 %configuration_code32 to i64
  %read_to_close = load ptr, ptr %read_slot, align 8
  %write_to_close = load ptr, ptr %write_slot, align 8
  %closed_read = call i32 @CloseHandle(ptr %read_to_close)
  %closed_write = call i32 @CloseHandle(ptr %write_to_close)
  %configuration_descriptor = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 %configuration_code)
  ret ptr %configuration_descriptor
create_error:
  %create_code32 = call i32 @GetLastError()
  %create_code = zext i32 %create_code32 to i64
  %create_descriptor = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 %create_code)
  ret ptr %create_descriptor
out_of_memory:
  ret ptr null
}

define ptr @__ncc_command_pipe_read_handle(ptr %pipe) {
entry:
  %handle = load ptr, ptr %pipe, align 8
  ret ptr %handle
}

define ptr @__ncc_command_pipe_write_handle(ptr %pipe) {
entry:
  %slot = getelementptr i8, ptr %pipe, i64 8
  %handle = load ptr, ptr %slot, align 8
  ret ptr %handle
}

define i64 @__ncc_command_pipe_read(ptr %pipe, ptr %destination, i64 %capacity64) {
entry:
  %read_slot = alloca i32, align 4
  %handle = load ptr, ptr %pipe, align 8
  %capacity = trunc i64 %capacity64 to i32
  %read = call i32 @ReadFile(ptr %handle, ptr %destination, i32 %capacity, ptr %read_slot, ptr null)
  %read_ok = icmp ne i32 %read, 0
  br i1 %read_ok, label %ok, label %error
ok:
  %bytes32 = load i32, ptr %read_slot, align 4
  %bytes = zext i32 %bytes32 to i64
  ret i64 %bytes
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_pipe_set_error(ptr %pipe, i64 %code)
  %negative = sub i64 0, %code
  ret i64 %negative
}

define i64 @__ncc_command_pipe_write(ptr %pipe, ptr %source, i64 %bytes64) {
entry:
  %written_slot = alloca i32, align 4
  %handle_slot = getelementptr i8, ptr %pipe, i64 8
  %handle = load ptr, ptr %handle_slot, align 8
  %bytes = trunc i64 %bytes64 to i32
  %written = call i32 @WriteFile(ptr %handle, ptr %source, i32 %bytes, ptr %written_slot, ptr null)
  %written_ok = icmp ne i32 %written, 0
  br i1 %written_ok, label %ok, label %error
ok:
  %count32 = load i32, ptr %written_slot, align 4
  %count = zext i32 %count32 to i64
  ret i64 %count
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_pipe_set_error(ptr %pipe, i64 %code)
  %negative = sub i64 0, %code
  ret i64 %negative
}

define i1 @__ncc_command_pipe_close_read(ptr %pipe) {
entry:
  %handle = load ptr, ptr %pipe, align 8
  %closed32 = call i32 @CloseHandle(ptr %handle)
  %closed = icmp ne i32 %closed32, 0
  br i1 %closed, label %clear, label %error
clear:
  store ptr null, ptr %pipe, align 8
  br label %done
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_pipe_set_error(ptr %pipe, i64 %code)
  br label %done
done:
  ret i1 %closed
}

define i1 @__ncc_command_pipe_close_write(ptr %pipe) {
entry:
  %slot = getelementptr i8, ptr %pipe, i64 8
  %handle = load ptr, ptr %slot, align 8
  %closed32 = call i32 @CloseHandle(ptr %handle)
  %closed = icmp ne i32 %closed32, 0
  br i1 %closed, label %clear, label %error
clear:
  store ptr null, ptr %slot, align 8
  br label %done
error:
  %code32 = call i32 @GetLastError()
  %code = zext i32 %code32 to i64
  %updated = call ptr @__ncc_command_pipe_set_error(ptr %pipe, i64 %code)
  br label %done
done:
  ret i1 %closed
}

define i64 @__ncc_command_pipe_error(ptr %pipe) {
entry:
  %slot = getelementptr i8, ptr %pipe, i64 16
  %error = load i64, ptr %slot, align 8
  ret i64 %error
}

define i1 @__ncc_command_pipe_release(ptr %pipe) {
entry:
  %read = load ptr, ptr %pipe, align 8
  %write_slot = getelementptr i8, ptr %pipe, i64 8
  %write = load ptr, ptr %write_slot, align 8
  %read_present = icmp ne ptr %read, null
  br i1 %read_present, label %close_read, label %write_check
close_read:
  %read_closed32 = call i32 @CloseHandle(ptr %read)
  %read_closed = icmp ne i32 %read_closed32, 0
  br label %write_check
write_check:
  %read_ok = phi i1 [ true, %entry ], [ %read_closed, %close_read ]
  %write_present = icmp ne ptr %write, null
  br i1 %write_present, label %close_write, label %release
close_write:
  %write_closed32 = call i32 @CloseHandle(ptr %write)
  %write_closed = icmp ne i32 %write_closed32, 0
  br label %release
release:
  %write_ok = phi i1 [ true, %write_check ], [ %write_closed, %close_write ]
  %handles_ok = and i1 %read_ok, %write_ok
  %memory_released = call i1 @__ncc_raw_aligned_free(ptr %pipe)
  %ok = and i1 %handles_ok, %memory_released
  ret i1 %ok
}
"#;

const POSIX_PROCESS_COMMAND_RUNTIME: &str = r#"
declare i32 @fork()
declare i32 @execvp(ptr, ptr)
declare void @_exit(i32)
declare i32 @waitpid(i32, ptr, i32)
declare i32 @kill(i32, i32)
declare i32 @pipe(ptr)
declare i32 @dup2(i32, i32)
declare i32 @close(i32)
declare i64 @read(i32, ptr, i64)
declare i64 @write(i32, ptr, i64)
declare i32 @fcntl(i32, i32, ...)
declare i32 @chdir(ptr)
declare i32 @usleep(i32)

define internal i64 @__ncc_command_posix_strlen(ptr %text) {
entry:
  br label %scan
scan:
  %at = phi i64 [ 0, %entry ], [ %next, %body ]
  %slot = getelementptr i8, ptr %text, i64 %at
  %byte = load i8, ptr %slot, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %finish, label %body
body:
  %next = add i64 %at, 1
  br label %scan
finish:
  ret i64 %at
}

define internal i1 @__ncc_command_posix_space(i8 %byte) {
entry:
  %space = icmp eq i8 %byte, 32
  %tab = icmp eq i8 %byte, 9
  %lf = icmp eq i8 %byte, 10
  %cr = icmp eq i8 %byte, 13
  %a = or i1 %space, %tab
  %b = or i1 %lf, %cr
  %result = or i1 %a, %b
  ret i1 %result
}

define internal ptr @__ncc_command_posix_parse(ptr %command) {
entry:
  %length = call i64 @__ncc_command_posix_strlen(ptr %command)
  %slots = add i64 %length, 2
  %argv.bytes = mul i64 %slots, @POINTER_BYTES@
  %text.bytes = add i64 %length, 1
  %total = add i64 %argv.bytes, %text.bytes
  %block = call ptr @__ncc_raw_aligned_alloc(i64 %total, i64 @POINTER_BYTES@)
  %allocated = icmp ne ptr %block, null
  br i1 %allocated, label %initialize, label %failure
initialize:
  %text = getelementptr i8, ptr %block, i64 %argv.bytes
  br label %skip
skip:
  %source.at = phi i64 [ 0, %initialize ], [ %source.space.next, %skip.space ], [ %source.argument.next, %end.argument ]
  %write.at = phi i64 [ 0, %initialize ], [ %write.at, %skip.space ], [ %write.argument.next, %end.argument ]
  %argc = phi i64 [ 0, %initialize ], [ %argc, %skip.space ], [ %argc.next, %end.argument ]
  %source.slot = getelementptr i8, ptr %command, i64 %source.at
  %source.byte = load i8, ptr %source.slot, align 1
  %source.done = icmp eq i8 %source.byte, 0
  br i1 %source.done, label %finish, label %skip.classify
skip.classify:
  %source.space = call i1 @__ncc_command_posix_space(i8 %source.byte)
  br i1 %source.space, label %skip.space, label %start.argument
skip.space:
  %source.space.next = add i64 %source.at, 1
  br label %skip
start.argument:
  %argument.slot = getelementptr ptr, ptr %block, i64 %argc
  %argument.text = getelementptr i8, ptr %text, i64 %write.at
  store ptr %argument.text, ptr %argument.slot, align @POINTER_BYTES@
  br label %scan
scan:
  %scan.source = phi i64 [ %source.at, %start.argument ], [ %copy.source.next, %copy ], [ %enter.source.next, %enter.quote ], [ %leave.source.next, %leave.quote ], [ %escape.source.next, %arm.escape ]
  %scan.write = phi i64 [ %write.at, %start.argument ], [ %copy.write.next, %copy ], [ %scan.write, %enter.quote ], [ %scan.write, %leave.quote ], [ %scan.write, %arm.escape ]
  %quote = phi i8 [ 0, %start.argument ], [ %quote, %copy ], [ %scan.byte, %enter.quote ], [ 0, %leave.quote ], [ %quote, %arm.escape ]
  %escaped = phi i1 [ false, %start.argument ], [ false, %copy ], [ false, %enter.quote ], [ false, %leave.quote ], [ true, %arm.escape ]
  %scan.slot = getelementptr i8, ptr %command, i64 %scan.source
  %scan.byte = load i8, ptr %scan.slot, align 1
  %scan.done = icmp eq i8 %scan.byte, 0
  br i1 %scan.done, label %finish.argument, label %scan.escape
scan.escape:
  br i1 %escaped, label %copy, label %scan.quote
scan.quote:
  %quoted = icmp ne i8 %quote, 0
  br i1 %quoted, label %inside.quote, label %outside.quote
inside.quote:
  %closes = icmp eq i8 %scan.byte, %quote
  br i1 %closes, label %leave.quote, label %inside.escape
inside.escape:
  %double.quote = icmp eq i8 %quote, 34
  %backslash.in = icmp eq i8 %scan.byte, 92
  %escape.in = and i1 %double.quote, %backslash.in
  br i1 %escape.in, label %arm.escape, label %copy
outside.quote:
  %separator = call i1 @__ncc_command_posix_space(i8 %scan.byte)
  br i1 %separator, label %end.argument, label %outside.special
outside.special:
  %single = icmp eq i8 %scan.byte, 39
  %double = icmp eq i8 %scan.byte, 34
  %opens = or i1 %single, %double
  br i1 %opens, label %enter.quote, label %outside.escape
outside.escape:
  %backslash.out = icmp eq i8 %scan.byte, 92
  br i1 %backslash.out, label %arm.escape, label %copy
enter.quote:
  %enter.source.next = add i64 %scan.source, 1
  br label %scan
leave.quote:
  %leave.source.next = add i64 %scan.source, 1
  br label %scan
arm.escape:
  %escape.source.next = add i64 %scan.source, 1
  br label %scan
copy:
  %copy.slot = getelementptr i8, ptr %text, i64 %scan.write
  store i8 %scan.byte, ptr %copy.slot, align 1
  %copy.source.next = add i64 %scan.source, 1
  %copy.write.next = add i64 %scan.write, 1
  br label %scan
end.argument:
  %argument.end = getelementptr i8, ptr %text, i64 %scan.write
  store i8 0, ptr %argument.end, align 1
  %source.argument.next = add i64 %scan.source, 1
  %write.argument.next = add i64 %scan.write, 1
  %argc.next = add i64 %argc, 1
  br label %skip
finish.argument:
  %final.end = getelementptr i8, ptr %text, i64 %scan.write
  store i8 0, ptr %final.end, align 1
  %final.argc = add i64 %argc, 1
  br label %terminate
finish:
  br label %terminate
terminate:
  %count = phi i64 [ %argc, %finish ], [ %final.argc, %finish.argument ]
  %null.slot = getelementptr ptr, ptr %block, i64 %count
  store ptr null, ptr %null.slot, align @POINTER_BYTES@
  %nonempty = icmp ugt i64 %count, 0
  br i1 %nonempty, label %success, label %empty
empty:
  %released = call i1 @__ncc_raw_aligned_free(ptr %block)
  br label %failure
success:
  ret ptr %block
failure:
  ret ptr null
}

define internal ptr @__ncc_command_set_error(ptr %descriptor, i64 %error) {
entry:
  %error.slot = getelementptr i8, ptr %descriptor, i64 32
  store i64 %error, ptr %error.slot, align 8
  ret ptr %descriptor
}

define internal i32 @__ncc_command_decode_fd(ptr %encoded) {
entry:
  %raw = ptrtoint ptr %encoded to i64
  %fd64 = sub i64 %raw, 1
  %fd = trunc i64 %fd64 to i32
  ret i32 %fd
}

define internal i1 @__ncc_command_redirect(ptr %encoded, i32 %target) {
entry:
  %present = icmp ne ptr %encoded, null
  br i1 %present, label %redirect, label %success
redirect:
  %fd = call i32 @__ncc_command_decode_fd(ptr %encoded)
  %same = icmp eq i32 %fd, %target
  br i1 %same, label %success, label %duplicate
duplicate:
  %duplicated = call i32 @dup2(i32 %fd, i32 %target)
  %ok = icmp sge i32 %duplicated, 0
  br i1 %ok, label %success, label %failure
success:
  ret i1 true
failure:
  ret i1 false
}

define ptr @__ncc_command_open(ptr %command, ptr %cwd, ptr %stdin.handle, ptr %stdout.handle, ptr %stderr.handle, i64 %flags) {
entry:
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 48, i64 8)
  %descriptor.ok = icmp ne ptr %descriptor, null
  br i1 %descriptor.ok, label %initialize, label %failure
initialize:
  call void @llvm.memset.p0.i64(ptr %descriptor, i8 0, i64 48, i1 false)
  %status.slot = getelementptr i8, ptr %descriptor, i64 8
  %flags.slot = getelementptr i8, ptr %descriptor, i64 40
  store i64 -1, ptr %status.slot, align 8
  store i64 %flags, ptr %flags.slot, align 8
  %argv = call ptr @__ncc_command_posix_parse(ptr %command)
  %argv.ok = icmp ne ptr %argv, null
  br i1 %argv.ok, label %spawn, label %parse.error
spawn:
  %pid = call i32 @fork()
  %child = icmp eq i32 %pid, 0
  br i1 %child, label %child.cwd, label %parent
child.cwd:
  %cwd.present = icmp ne ptr %cwd, null
  br i1 %cwd.present, label %child.cwd.content, label %child.redirect.stdin
child.cwd.content:
  %cwd.first = load i8, ptr %cwd, align 1
  %cwd.empty = icmp eq i8 %cwd.first, 0
  br i1 %cwd.empty, label %child.redirect.stdin, label %child.chdir
child.chdir:
  %changed = call i32 @chdir(ptr %cwd)
  %changed.ok = icmp eq i32 %changed, 0
  br i1 %changed.ok, label %child.redirect.stdin, label %child.fail
child.redirect.stdin:
  %stdin.ok = call i1 @__ncc_command_redirect(ptr %stdin.handle, i32 0)
  br i1 %stdin.ok, label %child.redirect.stdout, label %child.fail
child.redirect.stdout:
  %stdout.ok = call i1 @__ncc_command_redirect(ptr %stdout.handle, i32 1)
  br i1 %stdout.ok, label %child.redirect.stderr, label %child.fail
child.redirect.stderr:
  %stderr.ok = call i1 @__ncc_command_redirect(ptr %stderr.handle, i32 2)
  br i1 %stderr.ok, label %child.exec, label %child.fail
child.exec:
  %program.slot = getelementptr ptr, ptr %argv, i64 0
  %program = load ptr, ptr %program.slot, align @POINTER_BYTES@
  %executed = call i32 @execvp(ptr %program, ptr %argv)
  br label %child.fail
child.fail:
  call void @_exit(i32 127)
  unreachable
parent:
  %released.argv = call i1 @__ncc_raw_aligned_free(ptr %argv)
  %fork.failed = icmp slt i32 %pid, 0
  br i1 %fork.failed, label %fork.error, label %record
record:
  %pid64 = sext i32 %pid to i64
  %pid.slot = getelementptr i8, ptr %descriptor, i64 16
  store i64 %pid64, ptr %descriptor, align 8
  store i64 %pid64, ptr %pid.slot, align 8
  ret ptr %descriptor
fork.error:
  %fork.failed.descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 2)
  ret ptr %fork.failed.descriptor
parse.error:
  %parse.failed.descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 1)
  ret ptr %parse.failed.descriptor
failure:
  ret ptr null
}

define i1 @__ncc_command_started(ptr %descriptor) {
entry:
  %pid = load i64, ptr %descriptor, align 8
  %started = icmp sgt i64 %pid, 0
  ret i1 %started
}

define internal i64 @__ncc_command_record_status(ptr %descriptor, i32 %native.status) {
entry:
  %signal.bits = and i32 %native.status, 127
  %exited = icmp eq i32 %signal.bits, 0
  %exit.shifted = lshr i32 %native.status, 8
  %exit.code = and i32 %exit.shifted, 255
  %signal.code = add i32 %signal.bits, 128
  %code32 = select i1 %exited, i32 %exit.code, i32 %signal.code
  %code = zext i32 %code32 to i64
  %status.slot = getelementptr i8, ptr %descriptor, i64 8
  store i64 %code, ptr %status.slot, align 8
  ret i64 0
}

define i64 @__ncc_command_wait(ptr %descriptor, i64 %timeout) {
entry:
  %pid64 = load i64, ptr %descriptor, align 8
  %pid = trunc i64 %pid64 to i32
  %status = alloca i32, align 4
  %infinite.negative = icmp slt i64 %timeout, 0
  %infinite.win32 = icmp eq i64 %timeout, 4294967295
  %infinite = or i1 %infinite.negative, %infinite.win32
  br i1 %infinite, label %blocking, label %poll
blocking:
  %waited.blocking = call i32 @waitpid(i32 %pid, ptr %status, i32 0)
  %blocking.ok = icmp eq i32 %waited.blocking, %pid
  br i1 %blocking.ok, label %complete, label %wait.error
poll:
  %elapsed = phi i64 [ 0, %entry ], [ %elapsed.next, %sleep ]
  %waited = call i32 @waitpid(i32 %pid, ptr %status, i32 1)
  %finished = icmp eq i32 %waited, %pid
  br i1 %finished, label %complete, label %poll.error.check
poll.error.check:
  %poll.failed = icmp slt i32 %waited, 0
  br i1 %poll.failed, label %wait.error, label %timeout.check
timeout.check:
  %expired = icmp uge i64 %elapsed, %timeout
  br i1 %expired, label %timeout.done, label %sleep
sleep:
  %slept = call i32 @usleep(i32 1000)
  %elapsed.next = add i64 %elapsed, 1
  br label %poll
complete:
  %native.status = load i32, ptr %status, align 4
  %recorded = call i64 @__ncc_command_record_status(ptr %descriptor, i32 %native.status)
  ret i64 0
timeout.done:
  ret i64 258
wait.error:
  %failed.descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 4)
  ret i64 -4
}

define i64 @__ncc_command_exit_code(ptr %descriptor) {
entry:
  %status.slot = getelementptr i8, ptr %descriptor, i64 8
  %status = load i64, ptr %status.slot, align 8
  ret i64 %status
}

define i1 @__ncc_command_terminate(ptr %descriptor, i64 %requested) {
entry:
  %pid64 = load i64, ptr %descriptor, align 8
  %pid = trunc i64 %pid64 to i32
  %positive = icmp sgt i64 %requested, 0
  %bounded = icmp sle i64 %requested, 64
  %valid = and i1 %positive, %bounded
  %requested32 = trunc i64 %requested to i32
  %signal = select i1 %valid, i32 %requested32, i32 15
  %sent = call i32 @kill(i32 %pid, i32 %signal)
  %ok = icmp eq i32 %sent, 0
  br i1 %ok, label %done, label %error
error:
  %failed.descriptor = call ptr @__ncc_command_set_error(ptr %descriptor, i64 5)
  br label %done
done:
  ret i1 %ok
}

define ptr @__ncc_command_process_handle(ptr %descriptor) {
entry:
  %pid = load i64, ptr %descriptor, align 8
  %encoded = add i64 %pid, 1
  %handle = inttoptr i64 %encoded to ptr
  ret ptr %handle
}

define ptr @__ncc_command_thread_handle(ptr %descriptor) {
entry:
  ret ptr null
}

define i64 @__ncc_command_id(ptr %descriptor) {
entry:
  %pid.slot = getelementptr i8, ptr %descriptor, i64 16
  %pid = load i64, ptr %pid.slot, align 8
  ret i64 %pid
}

define i64 @__ncc_command_thread_id(ptr %descriptor) {
entry:
  ret i64 0
}

define i64 @__ncc_command_error(ptr %descriptor) {
entry:
  %error.slot = getelementptr i8, ptr %descriptor, i64 32
  %error = load i64, ptr %error.slot, align 8
  ret i64 %error
}

define i1 @__ncc_command_release(ptr %descriptor) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  ret i1 %released
}

define internal ptr @__ncc_command_pipe_set_error(ptr %pipe.descriptor, i64 %error) {
entry:
  %error.slot = getelementptr i8, ptr %pipe.descriptor, i64 16
  store i64 %error, ptr %error.slot, align 8
  ret ptr %pipe.descriptor
}

define ptr @__ncc_command_pipe_open(i1 %inherit.read, i1 %inherit.write) {
entry:
  %native = alloca [2 x i32], align 4
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %descriptor.ok = icmp ne ptr %descriptor, null
  br i1 %descriptor.ok, label %initialize, label %failure
initialize:
  call void @llvm.memset.p0.i64(ptr %descriptor, i8 0, i64 24, i1 false)
  store i64 -1, ptr %descriptor, align 8
  %descriptor.write.slot = getelementptr i8, ptr %descriptor, i64 8
  store i64 -1, ptr %descriptor.write.slot, align 8
  %created = call i32 @pipe(ptr %native)
  %created.ok = icmp eq i32 %created, 0
  br i1 %created.ok, label %configure, label %pipe.error
configure:
  %read.fd = load i32, ptr %native, align 4
  %native.write.slot = getelementptr i32, ptr %native, i64 1
  %write.fd = load i32, ptr %native.write.slot, align 4
  br i1 %inherit.read, label %configure.write, label %close_on_exec.read
close_on_exec.read:
  %read.configured = call i32 (i32, i32, ...) @fcntl(i32 %read.fd, i32 2, i32 1)
  %read.ok = icmp sge i32 %read.configured, 0
  br i1 %read.ok, label %configure.write, label %configure.error
configure.write:
  br i1 %inherit.write, label %record, label %close_on_exec.write
close_on_exec.write:
  %write.configured = call i32 (i32, i32, ...) @fcntl(i32 %write.fd, i32 2, i32 1)
  %write.ok = icmp sge i32 %write.configured, 0
  br i1 %write.ok, label %record, label %configure.error
record:
  %read.fd64 = sext i32 %read.fd to i64
  %write.fd64 = sext i32 %write.fd to i64
  store i64 %read.fd64, ptr %descriptor, align 8
  store i64 %write.fd64, ptr %descriptor.write.slot, align 8
  ret ptr %descriptor
configure.error:
  %closed.read = call i32 @close(i32 %read.fd)
  %closed.write = call i32 @close(i32 %write.fd)
  %configured.failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 6)
  ret ptr %configured.failed
pipe.error:
  %pipe.failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 5)
  ret ptr %pipe.failed
failure:
  ret ptr null
}

define internal ptr @__ncc_command_encode_fd(i64 %fd) {
entry:
  %encoded = add i64 %fd, 1
  %handle = inttoptr i64 %encoded to ptr
  ret ptr %handle
}

define ptr @__ncc_command_pipe_read_handle(ptr %descriptor) {
entry:
  %fd = load i64, ptr %descriptor, align 8
  %handle = call ptr @__ncc_command_encode_fd(i64 %fd)
  ret ptr %handle
}

define ptr @__ncc_command_pipe_write_handle(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 8
  %fd = load i64, ptr %slot, align 8
  %handle = call ptr @__ncc_command_encode_fd(i64 %fd)
  ret ptr %handle
}

define i64 @__ncc_command_pipe_read(ptr %descriptor, ptr %destination, i64 %capacity) {
entry:
  %fd64 = load i64, ptr %descriptor, align 8
  %fd = trunc i64 %fd64 to i32
  %read.bytes = call i64 @read(i32 %fd, ptr %destination, i64 %capacity)
  %ok = icmp sge i64 %read.bytes, 0
  br i1 %ok, label %done, label %error
error:
  %failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 5)
  ret i64 -5
done:
  ret i64 %read.bytes
}

define i64 @__ncc_command_pipe_write(ptr %descriptor, ptr %source, i64 %bytes) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 8
  %fd64 = load i64, ptr %slot, align 8
  %fd = trunc i64 %fd64 to i32
  %written = call i64 @write(i32 %fd, ptr %source, i64 %bytes)
  %ok = icmp sge i64 %written, 0
  br i1 %ok, label %done, label %error
error:
  %failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 5)
  ret i64 -5
done:
  ret i64 %written
}

define i1 @__ncc_command_pipe_close_read(ptr %descriptor) {
entry:
  %fd64 = load i64, ptr %descriptor, align 8
  %present = icmp sge i64 %fd64, 0
  br i1 %present, label %close.fd, label %done
close.fd:
  %fd = trunc i64 %fd64 to i32
  %closed = call i32 @close(i32 %fd)
  %ok.closed = icmp eq i32 %closed, 0
  br i1 %ok.closed, label %clear, label %error
clear:
  store i64 -1, ptr %descriptor, align 8
  br label %done
error:
  %failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 5)
  br label %done
done:
  %ok = phi i1 [ true, %entry ], [ true, %clear ], [ false, %error ]
  ret i1 %ok
}

define i1 @__ncc_command_pipe_close_write(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 8
  %fd64 = load i64, ptr %slot, align 8
  %present = icmp sge i64 %fd64, 0
  br i1 %present, label %close.fd, label %done
close.fd:
  %fd = trunc i64 %fd64 to i32
  %closed = call i32 @close(i32 %fd)
  %ok.closed = icmp eq i32 %closed, 0
  br i1 %ok.closed, label %clear, label %error
clear:
  store i64 -1, ptr %slot, align 8
  br label %done
error:
  %failed = call ptr @__ncc_command_pipe_set_error(ptr %descriptor, i64 5)
  br label %done
done:
  %ok = phi i1 [ true, %entry ], [ true, %clear ], [ false, %error ]
  ret i1 %ok
}

define i64 @__ncc_command_pipe_error(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 16
  %error = load i64, ptr %slot, align 8
  ret i64 %error
}

define i1 @__ncc_command_pipe_release(ptr %descriptor) {
entry:
  %read.closed = call i1 @__ncc_command_pipe_close_read(ptr %descriptor)
  %write.closed = call i1 @__ncc_command_pipe_close_write(ptr %descriptor)
  %released = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  %handles.closed = and i1 %read.closed, %write.closed
  %ok = and i1 %handles.closed, %released
  ret i1 %ok
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_command_runtime;

    #[test]
    fn win64_command_runtime_is_manual_crt_free_and_uses_native_process_apis() {
        let ir = render_process_command_runtime("x86_64-pc-windows-msvc", false)
            .expect("Win64 process command runtime");
        for material in [
            "__ncc_command_open",
            "__ncc_command_wait",
            "__ncc_command_exit_code",
            "__ncc_command_terminate",
            "__ncc_command_release",
            "__ncc_command_pipe_open",
            "__ncc_command_pipe_read",
            "__ncc_command_pipe_write",
            "__ncc_command_pipe_release",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "CreateProcessW",
            "WaitForSingleObject",
            "CreatePipe",
            "ReadFile",
            "WriteFile",
            "GetStdHandle",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for inherited in ["%stdin_native", "%stdout_native", "%stderr_native"] {
            assert!(
                ir.contains(inherited),
                "missing inherited standard handle {inherited}"
            );
        }
        for forbidden in ["@system(", "@popen(", "@malloc(", "@free(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }

    #[test]
    fn posix_command_runtime_is_manual_and_materializes_process_pipe_abis() {
        for target in [
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "i386-unknown-linux-gnu",
            "riscv64gc-unknown-linux-gnu",
        ] {
            let ir = render_process_command_runtime(target, false)
                .expect("POSIX process command runtime");
            for material in [
                "__ncc_command_open",
                "__ncc_command_wait",
                "__ncc_command_terminate",
                "__ncc_command_pipe_open",
                "__ncc_command_pipe_read",
                "__ncc_command_pipe_write",
                "__ncc_command_pipe_release",
            ] {
                assert!(
                    ir.contains(&format!("@{material}(")),
                    "{target}: {material}"
                );
            }
            for native in ["fork", "execvp", "waitpid", "dup2", "pipe", "read", "write"] {
                assert!(ir.contains(&format!("@{native}(")), "{target}: {native}");
            }
            for forbidden in ["@system(", "@popen(", "@wordexp("] {
                assert!(
                    !ir.contains(forbidden),
                    "{target}: unexpected shell {forbidden}"
                );
            }
        }
    }

    #[test]
    fn linux_command_runtime_can_pass_the_nc_environment_vector_explicitly() {
        let ir = render_process_command_runtime("x86_64-unknown-linux-gnu", true)
            .expect("Linux process command runtime with explicit environment");
        assert!(ir.contains("declare i32 @execvpe(ptr, ptr, ptr)"));
        assert!(ir.contains("load ptr, ptr @__ncc_process_environ"));
        assert!(ir.contains("store ptr %environment, ptr @environ"));
        assert!(ir.contains("call void @__ncc_env_platform_seed()"));
        assert!(ir.contains("@execvpe(ptr %program, ptr %argv, ptr %environment)"));
        assert!(!ir.contains("declare i32 @execvp(ptr, ptr)"));
    }
}
