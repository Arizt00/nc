pub(crate) fn render_process_signal_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if !(target.contains("windows") || target.contains("msvc") || target.contains("win32")) {
        return Err(format!(
            "manual process signal runtime requires a Windows target, got '{target_triple}'"
        ));
    }
    if !(target.contains("x86_64") || target.contains("aarch64")) {
        return Err(format!(
            "manual process signal runtime supports Win64 x86_64/aarch64, got '{target_triple}'"
        ));
    }
    Ok(WIN64_PROCESS_SIGNAL_RUNTIME.to_string())
}

const WIN64_PROCESS_SIGNAL_RUNTIME: &str = r#"
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @SetConsoleCtrlHandler(ptr, i32)
declare i32 @GenerateConsoleCtrlEvent(i32, i32)
declare ptr @CreateEventW(ptr, i32, i32, ptr)
declare ptr @OpenEventW(i32, i32, ptr)
declare i32 @SetEvent(ptr)
declare i32 @ResetEvent(ptr)
declare i32 @PulseEvent(ptr)
declare i32 @WaitForSingleObject(ptr, i32)
declare i32 @WaitForSingleObjectEx(ptr, i32, i32)
declare i32 @WaitForMultipleObjects(i32, ptr, i32, i32)
declare i32 @WaitForMultipleObjectsEx(i32, ptr, i32, i32, i32)
declare i32 @QueueUserAPC(ptr, ptr, i64)
declare ptr @AddVectoredExceptionHandler(i32, ptr)
declare i32 @RemoveVectoredExceptionHandler(ptr)
declare ptr @SetUnhandledExceptionFilter(ptr)
declare void @RaiseException(i32, i32, i32, ptr)
declare i32 @CloseHandle(ptr)
declare i32 @GetLastError()

define ptr @__ncc_signal_wide_alloc(ptr %text) {
entry:
  %is_null = icmp eq ptr %text, null
  br i1 %is_null, label %failure, label %measure
measure:
  %chars = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr null, i32 0)
  %measured = icmp sgt i32 %chars, 0
  br i1 %measured, label %allocate, label %failure
allocate:
  %chars64 = zext i32 %chars to i64
  %bytes = shl i64 %chars64, 1
  %wide = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %allocated = icmp ne ptr %wide, null
  br i1 %allocated, label %convert, label %failure
convert:
  %written = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr %wide, i32 %chars)
  %converted = icmp eq i32 %written, %chars
  br i1 %converted, label %success, label %release_failure
release_failure:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  br label %failure
success:
  ret ptr %wide
failure:
  ret ptr null
}

define i1 @__ncc_signal_wide_free(ptr %wide) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  ret i1 %released
}

define i1 @__ncc_signal_ctrl_handler(ptr %handler, i1 %add) {
entry:
  %add32 = zext i1 %add to i32
  %status = call i32 @SetConsoleCtrlHandler(ptr %handler, i32 %add32)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_signal_ctrl_event(i64 %event64, i64 %group64) {
entry:
  %event = trunc i64 %event64 to i32
  %group = trunc i64 %group64 to i32
  %status = call i32 @GenerateConsoleCtrlEvent(i32 %event, i32 %group)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_signal_create_event(ptr %attributes, i1 %manual_reset, i1 %initial_state, ptr %wide_name) {
entry:
  %manual32 = zext i1 %manual_reset to i32
  %initial32 = zext i1 %initial_state to i32
  %event = call ptr @CreateEventW(ptr %attributes, i32 %manual32, i32 %initial32, ptr %wide_name)
  ret ptr %event
}

define ptr @__ncc_signal_open_event(i64 %access64, i1 %inherit, ptr %wide_name) {
entry:
  %access = trunc i64 %access64 to i32
  %inherit32 = zext i1 %inherit to i32
  %event = call ptr @OpenEventW(i32 %access, i32 %inherit32, ptr %wide_name)
  ret ptr %event
}

define i1 @__ncc_signal_set_event(ptr %event) {
entry:
  %status = call i32 @SetEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_signal_reset_event(ptr %event) {
entry:
  %status = call i32 @ResetEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_signal_pulse_event(ptr %event) {
entry:
  %status = call i32 @PulseEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_signal_wait_one(ptr %handle, i64 %timeout64) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %status32 = call i32 @WaitForSingleObject(ptr %handle, i32 %timeout)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_signal_wait_one_ex(ptr %handle, i64 %timeout64, i1 %alertable) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %alertable32 = zext i1 %alertable to i32
  %status32 = call i32 @WaitForSingleObjectEx(ptr %handle, i32 %timeout, i32 %alertable32)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_signal_wait_many(i64 %count64, ptr %handles, i1 %wait_all, i64 %timeout64) {
entry:
  %count = trunc i64 %count64 to i32
  %wait32 = zext i1 %wait_all to i32
  %timeout = trunc i64 %timeout64 to i32
  %status32 = call i32 @WaitForMultipleObjects(i32 %count, ptr %handles, i32 %wait32, i32 %timeout)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_signal_wait_many_ex(i64 %count64, ptr %handles, i1 %wait_all, i64 %timeout64, i1 %alertable) {
entry:
  %count = trunc i64 %count64 to i32
  %wait32 = zext i1 %wait_all to i32
  %timeout = trunc i64 %timeout64 to i32
  %alertable32 = zext i1 %alertable to i32
  %status32 = call i32 @WaitForMultipleObjectsEx(i32 %count, ptr %handles, i32 %wait32, i32 %timeout, i32 %alertable32)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_signal_queue_apc(ptr %callback, ptr %thread, i64 %data) {
entry:
  %status32 = call i32 @QueueUserAPC(ptr %callback, ptr %thread, i64 %data)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define ptr @__ncc_signal_add_vectored(i64 %first64, ptr %handler) {
entry:
  %first = trunc i64 %first64 to i32
  %registration = call ptr @AddVectoredExceptionHandler(i32 %first, ptr %handler)
  ret ptr %registration
}

define i1 @__ncc_signal_remove_vectored(ptr %registration) {
entry:
  %status = call i32 @RemoveVectoredExceptionHandler(ptr %registration)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_signal_set_unhandled_filter(ptr %handler) {
entry:
  %previous = call ptr @SetUnhandledExceptionFilter(ptr %handler)
  ret ptr %previous
}

define i1 @__ncc_signal_raise_exception(i64 %code64, i64 %flags64, i64 %count64, ptr %arguments) {
entry:
  %code = trunc i64 %code64 to i32
  %flags = trunc i64 %flags64 to i32
  %count = trunc i64 %count64 to i32
  call void @RaiseException(i32 %code, i32 %flags, i32 %count, ptr %arguments)
  ret i1 true
}

define i1 @__ncc_signal_close(ptr %handle) {
entry:
  %status = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_signal_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  ret i64 %error64
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_signal_runtime;

    #[test]
    fn win64_signal_runtime_is_manual_crt_free_and_covers_control_events_and_exceptions() {
        let ir = render_process_signal_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process signal runtime");
        for material in [
            "__ncc_signal_ctrl_handler",
            "__ncc_signal_create_event",
            "__ncc_signal_wait_many_ex",
            "__ncc_signal_queue_apc",
            "__ncc_signal_add_vectored",
            "__ncc_signal_raise_exception",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "SetConsoleCtrlHandler",
            "GenerateConsoleCtrlEvent",
            "CreateEventW",
            "WaitForMultipleObjectsEx",
            "QueueUserAPC",
            "AddVectoredExceptionHandler",
            "RaiseException",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        assert!(ir.contains("call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)"));
        for forbidden in ["@signal(", "@raise(", "@system(", "@malloc(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
