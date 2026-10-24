pub(crate) fn render_process_service_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if !lower.contains("windows") || !(lower.contains("x86_64") || lower.contains("aarch64")) {
        return Err(format!(
            "native raw process-service backend currently requires Win64, got target '{target}'"
        ));
    }
    Ok(WIN64_PROCESS_SERVICE_RUNTIME.to_string())
}

const WIN64_PROCESS_SERVICE_RUNTIME: &str = r#"
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare ptr @OpenSCManagerW(ptr, ptr, i32)
declare ptr @OpenServiceW(ptr, ptr, i32)
declare ptr @CreateServiceW(ptr, ptr, ptr, i32, i32, i32, i32, ptr, ptr, ptr, ptr, ptr, ptr)
declare i32 @DeleteService(ptr)
declare i32 @StartServiceW(ptr, i32, ptr)
declare i32 @ControlService(ptr, i32, ptr)
declare i32 @ControlServiceExW(ptr, i32, i32, ptr)
declare i32 @QueryServiceStatusEx(ptr, i32, ptr, i32, ptr)
declare i32 @QueryServiceConfigW(ptr, ptr, i32, ptr)
declare i32 @ChangeServiceConfigW(ptr, i32, i32, i32, ptr, ptr, ptr, ptr, ptr, ptr, ptr)
declare i32 @EnumServicesStatusExW(ptr, i32, i32, i32, ptr, i32, ptr, ptr, ptr, ptr)
declare i32 @EnumDependentServicesW(ptr, i32, ptr, i32, ptr, ptr)
declare i32 @GetServiceKeyNameW(ptr, ptr, ptr, ptr)
declare i32 @GetServiceDisplayNameW(ptr, ptr, ptr, ptr)
declare ptr @LockServiceDatabase(ptr)
declare i32 @UnlockServiceDatabase(ptr)
declare i32 @QueryServiceLockStatusW(ptr, ptr, i32, ptr)
declare i32 @NotifyServiceStatusChangeW(ptr, i32, ptr)
declare i32 @StartServiceCtrlDispatcherW(ptr)
declare ptr @RegisterServiceCtrlHandlerExW(ptr, ptr, ptr)
declare i32 @SetServiceStatus(ptr, ptr)
declare i32 @CloseServiceHandle(ptr)
declare i32 @GetLastError()

define ptr @__ncc_service_wide_alloc(ptr %text) {
entry:
  %is_null = icmp eq ptr %text, null
  br i1 %is_null, label %null_value, label %measure
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
null_value:
  ret ptr null
failure:
  ret ptr null
}

define i1 @__ncc_service_wide_free(ptr %wide) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  ret i1 %released
}

define ptr @__ncc_service_manager_open(ptr %machine, ptr %database, i64 %access64) {
entry:
  %access = trunc i64 %access64 to i32
  %handle = call ptr @OpenSCManagerW(ptr %machine, ptr %database, i32 %access)
  ret ptr %handle
}

define ptr @__ncc_service_open(ptr %manager, ptr %name, i64 %access64) {
entry:
  %access = trunc i64 %access64 to i32
  %handle = call ptr @OpenServiceW(ptr %manager, ptr %name, i32 %access)
  ret ptr %handle
}

define ptr @__ncc_service_create(ptr %manager, ptr %name, ptr %display, i64 %access64, i64 %type64, i64 %start64, i64 %error64, ptr %binary, ptr %load_group, ptr %tag_id, ptr %dependencies, ptr %account, ptr %password) {
entry:
  %access = trunc i64 %access64 to i32
  %type = trunc i64 %type64 to i32
  %start = trunc i64 %start64 to i32
  %error = trunc i64 %error64 to i32
  %handle = call ptr @CreateServiceW(ptr %manager, ptr %name, ptr %display, i32 %access, i32 %type, i32 %start, i32 %error, ptr %binary, ptr %load_group, ptr %tag_id, ptr %dependencies, ptr %account, ptr %password)
  ret ptr %handle
}

define i1 @__ncc_service_delete(ptr %service) {
entry:
  %status = call i32 @DeleteService(ptr %service)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_start(ptr %service, i64 %argc64, ptr %argv) {
entry:
  %argc = trunc i64 %argc64 to i32
  %status = call i32 @StartServiceW(ptr %service, i32 %argc, ptr %argv)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_control(ptr %service, i64 %control64, ptr %status_out) {
entry:
  %control = trunc i64 %control64 to i32
  %status = call i32 @ControlService(ptr %service, i32 %control, ptr %status_out)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_control_ex(ptr %service, i64 %control64, i64 %level64, ptr %parameters) {
entry:
  %control = trunc i64 %control64 to i32
  %level = trunc i64 %level64 to i32
  %status = call i32 @ControlServiceExW(ptr %service, i32 %control, i32 %level, ptr %parameters)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_query_status(ptr %service, i64 %level64, ptr %buffer, i64 %capacity64, ptr %needed) {
entry:
  %level = trunc i64 %level64 to i32
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @QueryServiceStatusEx(ptr %service, i32 %level, ptr %buffer, i32 %capacity, ptr %needed)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_query_config(ptr %service, ptr %buffer, i64 %capacity64, ptr %needed) {
entry:
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @QueryServiceConfigW(ptr %service, ptr %buffer, i32 %capacity, ptr %needed)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_change_config(ptr %service, i64 %type64, i64 %start64, i64 %error64, ptr %binary, ptr %load_group, ptr %tag_id, ptr %dependencies, ptr %account, ptr %password, ptr %display) {
entry:
  %type = trunc i64 %type64 to i32
  %start = trunc i64 %start64 to i32
  %error = trunc i64 %error64 to i32
  %status = call i32 @ChangeServiceConfigW(ptr %service, i32 %type, i32 %start, i32 %error, ptr %binary, ptr %load_group, ptr %tag_id, ptr %dependencies, ptr %account, ptr %password, ptr %display)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_enum(ptr %manager, i64 %level64, i64 %type64, i64 %state64, ptr %buffer, i64 %capacity64, ptr %needed, ptr %returned, ptr %resume, ptr %group) {
entry:
  %level = trunc i64 %level64 to i32
  %type = trunc i64 %type64 to i32
  %state = trunc i64 %state64 to i32
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @EnumServicesStatusExW(ptr %manager, i32 %level, i32 %type, i32 %state, ptr %buffer, i32 %capacity, ptr %needed, ptr %returned, ptr %resume, ptr %group)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_enum_dependents(ptr %service, i64 %state64, ptr %buffer, i64 %capacity64, ptr %needed, ptr %returned) {
entry:
  %state = trunc i64 %state64 to i32
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @EnumDependentServicesW(ptr %service, i32 %state, ptr %buffer, i32 %capacity, ptr %needed, ptr %returned)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_key_name(ptr %manager, ptr %display, ptr %output, ptr %chars) {
entry:
  %status = call i32 @GetServiceKeyNameW(ptr %manager, ptr %display, ptr %output, ptr %chars)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_display_name(ptr %manager, ptr %name, ptr %output, ptr %chars) {
entry:
  %status = call i32 @GetServiceDisplayNameW(ptr %manager, ptr %name, ptr %output, ptr %chars)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_service_manager_lock(ptr %manager) {
entry:
  %lock = call ptr @LockServiceDatabase(ptr %manager)
  ret ptr %lock
}

define i1 @__ncc_service_manager_unlock(ptr %lock) {
entry:
  %status = call i32 @UnlockServiceDatabase(ptr %lock)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_manager_lock_status(ptr %manager, ptr %buffer, i64 %capacity64, ptr %needed) {
entry:
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @QueryServiceLockStatusW(ptr %manager, ptr %buffer, i32 %capacity, ptr %needed)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_service_notify(ptr %service, i64 %mask64, ptr %notify) {
entry:
  %mask = trunc i64 %mask64 to i32
  %status32 = call i32 @NotifyServiceStatusChangeW(ptr %service, i32 %mask, ptr %notify)
  %status = zext i32 %status32 to i64
  ret i64 %status
}

define i1 @__ncc_service_start_dispatcher(ptr %table) {
entry:
  %status = call i32 @StartServiceCtrlDispatcherW(ptr %table)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_service_register_handler(ptr %name, ptr %handler, ptr %context) {
entry:
  %handle = call ptr @RegisterServiceCtrlHandlerExW(ptr %name, ptr %handler, ptr %context)
  ret ptr %handle
}

define i1 @__ncc_service_set_status(ptr %status_handle, ptr %status_data) {
entry:
  %status = call i32 @SetServiceStatus(ptr %status_handle, ptr %status_data)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_service_close(ptr %handle) {
entry:
  %status = call i32 @CloseServiceHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_service_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error = zext i32 %error32 to i64
  ret i64 %error
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_service_runtime;

    #[test]
    fn win64_service_runtime_is_manual_crt_free_and_covers_scm_client_and_host_apis() {
        let ir = render_process_service_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process service runtime");
        for material in [
            "__ncc_service_wide_alloc",
            "__ncc_service_manager_open",
            "__ncc_service_open",
            "__ncc_service_create",
            "__ncc_service_query_status",
            "__ncc_service_change_config",
            "__ncc_service_enum",
            "__ncc_service_notify",
            "__ncc_service_start_dispatcher",
            "__ncc_service_register_handler",
            "__ncc_service_set_status",
            "__ncc_service_close",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "OpenSCManagerW",
            "OpenServiceW",
            "CreateServiceW",
            "QueryServiceStatusEx",
            "ChangeServiceConfigW",
            "EnumServicesStatusExW",
            "NotifyServiceStatusChangeW",
            "StartServiceCtrlDispatcherW",
            "RegisterServiceCtrlHandlerExW",
            "SetServiceStatus",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        assert!(ir.contains("call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)"));
        for forbidden in ["@system(", "@popen(", "@malloc(", "@free(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
