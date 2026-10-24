pub(crate) fn render_task_event_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if !(target.contains("windows") || target.contains("msvc") || target.contains("win32")) {
        return Err(format!(
            "manual task event runtime requires a Windows target, got '{target_triple}'"
        ));
    }
    if !(target.contains("x86_64") || target.contains("aarch64")) {
        return Err(format!(
            "manual task event runtime supports Win64 x86_64/aarch64, got '{target_triple}'"
        ));
    }
    Ok(WIN64_TASK_EVENT_RUNTIME.to_string())
}

const WIN64_TASK_EVENT_RUNTIME: &str = r#"
declare ptr @CreateEventW(ptr, i32, i32, ptr)
declare ptr @CreateEventExW(ptr, ptr, i32, i32)
declare ptr @OpenEventW(i32, i32, ptr)
declare i32 @SetEvent(ptr)
declare i32 @ResetEvent(ptr)
declare i32 @PulseEvent(ptr)
declare i32 @WaitForSingleObject(ptr, i32)
declare i32 @WaitForSingleObjectEx(ptr, i32, i32)
declare i32 @WaitForMultipleObjects(i32, ptr, i32, i32)
declare i32 @WaitForMultipleObjectsEx(i32, ptr, i32, i32, i32)
declare i32 @SignalObjectAndWait(ptr, ptr, i32, i32)
declare i32 @CloseHandle(ptr)
declare i32 @GetLastError()

define ptr @__ncc_event_create(ptr %attributes, i1 %manual_reset, i1 %initial_state, ptr %wide_name) {
entry:
  %manual32 = zext i1 %manual_reset to i32
  %initial32 = zext i1 %initial_state to i32
  %event = call ptr @CreateEventW(ptr %attributes, i32 %manual32, i32 %initial32, ptr %wide_name)
  ret ptr %event
}

define ptr @__ncc_event_create_ex(ptr %attributes, ptr %wide_name, i64 %flags64, i64 %access64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %access = trunc i64 %access64 to i32
  %event = call ptr @CreateEventExW(ptr %attributes, ptr %wide_name, i32 %flags, i32 %access)
  ret ptr %event
}

define ptr @__ncc_event_open(i64 %access64, i1 %inherit, ptr %wide_name) {
entry:
  %access = trunc i64 %access64 to i32
  %inherit32 = zext i1 %inherit to i32
  %event = call ptr @OpenEventW(i32 %access, i32 %inherit32, ptr %wide_name)
  ret ptr %event
}

define i1 @__ncc_event_set(ptr %event) {
entry:
  %status = call i32 @SetEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_event_reset(ptr %event) {
entry:
  %status = call i32 @ResetEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_event_pulse(ptr %event) {
entry:
  %status = call i32 @PulseEvent(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_event_wait(ptr %event, i64 %timeout64) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %status32 = call i32 @WaitForSingleObject(ptr %event, i32 %timeout)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_event_wait_ex(ptr %event, i64 %timeout64, i1 %alertable) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %alertable32 = zext i1 %alertable to i32
  %status32 = call i32 @WaitForSingleObjectEx(ptr %event, i32 %timeout, i32 %alertable32)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_event_wait_many(i64 %count64, ptr %handles, i1 %wait_all, i64 %timeout64) {
entry:
  %count = trunc i64 %count64 to i32
  %wait32 = zext i1 %wait_all to i32
  %timeout = trunc i64 %timeout64 to i32
  %status32 = call i32 @WaitForMultipleObjects(i32 %count, ptr %handles, i32 %wait32, i32 %timeout)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_event_wait_many_ex(i64 %count64, ptr %handles, i1 %wait_all, i64 %timeout64, i1 %alertable) {
entry:
  %count = trunc i64 %count64 to i32
  %wait32 = zext i1 %wait_all to i32
  %timeout = trunc i64 %timeout64 to i32
  %alertable32 = zext i1 %alertable to i32
  %status32 = call i32 @WaitForMultipleObjectsEx(i32 %count, ptr %handles, i32 %wait32, i32 %timeout, i32 %alertable32)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i64 @__ncc_event_signal_and_wait(ptr %signal, ptr %wait, i64 %timeout64, i1 %alertable) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %alertable32 = zext i1 %alertable to i32
  %status32 = call i32 @SignalObjectAndWait(ptr %signal, ptr %wait, i32 %timeout, i32 %alertable32)
  %status64 = zext i32 %status32 to i64
  ret i64 %status64
}

define i1 @__ncc_event_close(ptr %event) {
entry:
  %status = call i32 @CloseHandle(ptr %event)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_event_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  ret i64 %error64
}
"#;

#[cfg(test)]
mod tests {
    use super::render_task_event_runtime;

    #[test]
    fn win64_task_event_runtime_is_manual_crt_free_and_covers_native_waits() {
        let ir =
            render_task_event_runtime("x86_64-pc-windows-msvc").expect("Win64 task event runtime");
        for material in [
            "__ncc_event_create",
            "__ncc_event_create_ex",
            "__ncc_event_wait_many_ex",
            "__ncc_event_signal_and_wait",
            "__ncc_event_close",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "CreateEventW",
            "CreateEventExW",
            "WaitForMultipleObjectsEx",
            "SignalObjectAndWait",
            "CloseHandle",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for forbidden in ["@malloc(", "@system(", "@popen(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
