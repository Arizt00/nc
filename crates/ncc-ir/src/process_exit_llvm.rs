pub(crate) fn render_process_exit_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if !lower.contains("windows") || !(lower.contains("x86_64") || lower.contains("aarch64")) {
        return Err(format!(
            "native raw process-exit backend currently requires Win64, got target '{target}'"
        ));
    }
    Ok(WIN64_PROCESS_EXIT_RUNTIME.to_string())
}

const WIN64_PROCESS_EXIT_RUNTIME: &str = r#"
declare ptr @GetCurrentProcess()
declare ptr @OpenProcess(i32, i32, i32)
declare i32 @GetProcessId(ptr)
declare i32 @GetExitCodeProcess(ptr, ptr)
declare i32 @TerminateProcess(ptr, i32)
declare i32 @CloseHandle(ptr)
declare void @ExitProcess(i32)
declare void @RaiseFailFastException(ptr, ptr, i32)
declare i32 @GetLastError()

define ptr @__ncc_exit_current_process() {
entry:
  %handle = call ptr @GetCurrentProcess()
  ret ptr %handle
}

define ptr @__ncc_exit_open_process(i64 %access64, i1 %inherit, i64 %pid64) {
entry:
  %access = trunc i64 %access64 to i32
  %inherit32 = zext i1 %inherit to i32
  %pid = trunc i64 %pid64 to i32
  %handle = call ptr @OpenProcess(i32 %access, i32 %inherit32, i32 %pid)
  ret ptr %handle
}

define i64 @__ncc_exit_process_id(ptr %handle) {
entry:
  %pid32 = call i32 @GetProcessId(ptr %handle)
  %pid = zext i32 %pid32 to i64
  ret i64 %pid
}

define i64 @__ncc_exit_process_code(ptr %handle) {
entry:
  %code_slot = alloca i32, align 4
  store i32 0, ptr %code_slot, align 4
  %status = call i32 @GetExitCodeProcess(ptr %handle, ptr %code_slot)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %error
success:
  %code32 = load i32, ptr %code_slot, align 4
  %code = zext i32 %code32 to i64
  ret i64 %code
error:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
}

define i1 @__ncc_exit_terminate_process(ptr %handle, i64 %code64) {
entry:
  %code = trunc i64 %code64 to i32
  %status = call i32 @TerminateProcess(ptr %handle, i32 %code)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_exit_close_process(ptr %handle) {
entry:
  %status = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_exit_now(i64 %code64) {
entry:
  %code = trunc i64 %code64 to i32
  call void @ExitProcess(i32 %code)
  ret i1 false
}

define i1 @__ncc_exit_fail_fast(ptr %exception_record, ptr %context_record, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  call void @RaiseFailFastException(ptr %exception_record, ptr %context_record, i32 %flags)
  ret i1 false
}

define i64 @__ncc_exit_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error = zext i32 %error32 to i64
  ret i64 %error
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_exit_runtime;

    #[test]
    fn win64_exit_runtime_is_manual_crt_free_and_uses_native_process_apis() {
        let ir = render_process_exit_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process exit runtime");
        for material in [
            "__ncc_exit_current_process",
            "__ncc_exit_open_process",
            "__ncc_exit_process_id",
            "__ncc_exit_process_code",
            "__ncc_exit_terminate_process",
            "__ncc_exit_close_process",
            "__ncc_exit_now",
            "__ncc_exit_fail_fast",
            "__ncc_exit_last_error",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "GetCurrentProcess",
            "OpenProcess",
            "GetProcessId",
            "GetExitCodeProcess",
            "TerminateProcess",
            "CloseHandle",
            "ExitProcess",
            "RaiseFailFastException",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for forbidden in ["@exit(", "@_exit(", "@abort(", "@malloc(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
