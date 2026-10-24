pub(crate) fn render_process_session_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if !(target.contains("windows") || target.contains("msvc") || target.contains("win32")) {
        return Err(format!(
            "manual process session runtime requires a Windows WTS target, got '{target_triple}'"
        ));
    }
    if !(target.contains("x86_64") || target.contains("aarch64")) {
        return Err(format!(
            "manual process session runtime supports Win64 x86_64/aarch64, got '{target_triple}'"
        ));
    }
    Ok(WIN64_PROCESS_SESSION_RUNTIME.to_string())
}

const WIN64_PROCESS_SESSION_RUNTIME: &str = r#"
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @GetCurrentProcessId()
declare i32 @ProcessIdToSessionId(i32, ptr)
declare i32 @WTSGetActiveConsoleSessionId()
declare ptr @WTSOpenServerW(ptr)
declare void @WTSCloseServer(ptr)
declare i32 @WTSEnumerateSessionsW(ptr, i32, i32, ptr, ptr)
declare i32 @WTSQuerySessionInformationW(ptr, i32, i32, ptr, ptr)
declare void @WTSFreeMemory(ptr)
declare i32 @WTSQueryUserToken(i32, ptr)
declare i32 @WTSSendMessageW(ptr, i32, ptr, i32, ptr, i32, i32, i32, ptr, i32)
declare i32 @WTSDisconnectSession(ptr, i32, i32)
declare i32 @WTSLogoffSession(ptr, i32, i32)
declare i32 @WTSConnectSessionW(i32, i32, ptr, i32)
declare i32 @WTSRegisterSessionNotification(ptr, i32)
declare i32 @WTSUnRegisterSessionNotification(ptr)
declare i32 @WTSWaitSystemEvent(ptr, i32, ptr)
declare ptr @WTSVirtualChannelOpenEx(i32, ptr, i32)
declare i32 @WTSVirtualChannelClose(ptr)
declare i32 @WTSVirtualChannelRead(ptr, i32, ptr, i32, ptr)
declare i32 @WTSVirtualChannelWrite(ptr, ptr, i32, ptr)
declare i32 @WTSVirtualChannelQuery(ptr, i32, ptr, ptr)
declare i32 @WTSVirtualChannelPurgeInput(ptr)
declare i32 @WTSVirtualChannelPurgeOutput(ptr)
declare i32 @GetLastError()

define ptr @__ncc_session_wide_alloc(ptr %text) {
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

define i1 @__ncc_session_wide_free(ptr %wide) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  ret i1 %released
}

define i64 @__ncc_session_current_id() {
entry:
  %slot = alloca i32, align 4
  %pid = call i32 @GetCurrentProcessId()
  %status = call i32 @ProcessIdToSessionId(i32 %pid, ptr %slot)
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %success, label %failure
success:
  %id32 = load i32, ptr %slot, align 4
  %id64 = zext i32 %id32 to i64
  ret i64 %id64
failure:
  ret i64 -1
}

define i64 @__ncc_session_active_console_id() {
entry:
  %id32 = call i32 @WTSGetActiveConsoleSessionId()
  %invalid = icmp eq i32 %id32, -1
  %id64 = zext i32 %id32 to i64
  %result = select i1 %invalid, i64 -1, i64 %id64
  ret i64 %result
}

define i1 @__ncc_session_process_id(i64 %pid64, ptr %out_id) {
entry:
  %pid = trunc i64 %pid64 to i32
  %status = call i32 @ProcessIdToSessionId(i32 %pid, ptr %out_id)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_session_server_open(ptr %name) {
entry:
  %server = call ptr @WTSOpenServerW(ptr %name)
  ret ptr %server
}

define i1 @__ncc_session_server_close(ptr %server) {
entry:
  call void @WTSCloseServer(ptr %server)
  ret i1 true
}

define i1 @__ncc_session_enumerate(ptr %server, ptr %out_entries, ptr %out_count) {
entry:
  %status = call i32 @WTSEnumerateSessionsW(ptr %server, i32 0, i32 1, ptr %out_entries, ptr %out_count)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_query(ptr %server, i64 %id64, i64 %class64, ptr %out_buffer, ptr %out_bytes) {
entry:
  %id = trunc i64 %id64 to i32
  %class = trunc i64 %class64 to i32
  %status = call i32 @WTSQuerySessionInformationW(ptr %server, i32 %id, i32 %class, ptr %out_buffer, ptr %out_bytes)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_free(ptr %buffer) {
entry:
  call void @WTSFreeMemory(ptr %buffer)
  ret i1 true
}

define i1 @__ncc_session_query_user_token(i64 %id64, ptr %out_token) {
entry:
  %id = trunc i64 %id64 to i32
  %status = call i32 @WTSQueryUserToken(i32 %id, ptr %out_token)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_send_message(ptr %server, i64 %id64, ptr %title, i64 %title_bytes64, ptr %message, i64 %message_bytes64, i64 %style64, i64 %timeout64, ptr %out_response, i1 %wait) {
entry:
  %id = trunc i64 %id64 to i32
  %title_bytes = trunc i64 %title_bytes64 to i32
  %message_bytes = trunc i64 %message_bytes64 to i32
  %style = trunc i64 %style64 to i32
  %timeout = trunc i64 %timeout64 to i32
  %wait32 = zext i1 %wait to i32
  %status = call i32 @WTSSendMessageW(ptr %server, i32 %id, ptr %title, i32 %title_bytes, ptr %message, i32 %message_bytes, i32 %style, i32 %timeout, ptr %out_response, i32 %wait32)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_disconnect(ptr %server, i64 %id64, i1 %wait) {
entry:
  %id = trunc i64 %id64 to i32
  %wait32 = zext i1 %wait to i32
  %status = call i32 @WTSDisconnectSession(ptr %server, i32 %id, i32 %wait32)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_logoff(ptr %server, i64 %id64, i1 %wait) {
entry:
  %id = trunc i64 %id64 to i32
  %wait32 = zext i1 %wait to i32
  %status = call i32 @WTSLogoffSession(ptr %server, i32 %id, i32 %wait32)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_connect(i64 %id64, i64 %target64, ptr %password, i1 %wait) {
entry:
  %id = trunc i64 %id64 to i32
  %target = trunc i64 %target64 to i32
  %wait32 = zext i1 %wait to i32
  %status = call i32 @WTSConnectSessionW(i32 %id, i32 %target, ptr %password, i32 %wait32)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_register_notification(ptr %window, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %status = call i32 @WTSRegisterSessionNotification(ptr %window, i32 %flags)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_unregister_notification(ptr %window) {
entry:
  %status = call i32 @WTSUnRegisterSessionNotification(ptr %window)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_wait_event(ptr %server, i64 %mask64, ptr %out_flags) {
entry:
  %mask = trunc i64 %mask64 to i32
  %status = call i32 @WTSWaitSystemEvent(ptr %server, i32 %mask, ptr %out_flags)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define ptr @__ncc_session_channel_open(i64 %id64, ptr %name, i64 %flags64) {
entry:
  %id = trunc i64 %id64 to i32
  %flags = trunc i64 %flags64 to i32
  %channel = call ptr @WTSVirtualChannelOpenEx(i32 %id, ptr %name, i32 %flags)
  ret ptr %channel
}

define i1 @__ncc_session_channel_close(ptr %channel) {
entry:
  %status = call i32 @WTSVirtualChannelClose(ptr %channel)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_channel_read(ptr %channel, i64 %timeout64, ptr %buffer, i64 %capacity64, ptr %out_read) {
entry:
  %timeout = trunc i64 %timeout64 to i32
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @WTSVirtualChannelRead(ptr %channel, i32 %timeout, ptr %buffer, i32 %capacity, ptr %out_read)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_channel_write(ptr %channel, ptr %buffer, i64 %bytes64, ptr %out_written) {
entry:
  %bytes = trunc i64 %bytes64 to i32
  %status = call i32 @WTSVirtualChannelWrite(ptr %channel, ptr %buffer, i32 %bytes, ptr %out_written)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_channel_query(ptr %channel, i64 %class64, ptr %out_buffer, ptr %out_bytes) {
entry:
  %class = trunc i64 %class64 to i32
  %status = call i32 @WTSVirtualChannelQuery(ptr %channel, i32 %class, ptr %out_buffer, ptr %out_bytes)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_channel_purge_input(ptr %channel) {
entry:
  %status = call i32 @WTSVirtualChannelPurgeInput(ptr %channel)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_session_channel_purge_output(ptr %channel) {
entry:
  %status = call i32 @WTSVirtualChannelPurgeOutput(ptr %channel)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_session_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  ret i64 %error64
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_session_runtime;

    #[test]
    fn win64_session_runtime_is_manual_crt_free_and_covers_wts_and_channels() {
        let ir = render_process_session_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process session runtime");
        for material in [
            "__ncc_session_current_id",
            "__ncc_session_enumerate",
            "__ncc_session_query",
            "__ncc_session_query_user_token",
            "__ncc_session_send_message",
            "__ncc_session_wait_event",
            "__ncc_session_channel_open",
            "__ncc_session_channel_read",
            "__ncc_session_channel_write",
            "__ncc_session_channel_query",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "ProcessIdToSessionId",
            "WTSEnumerateSessionsW",
            "WTSQuerySessionInformationW",
            "WTSQueryUserToken",
            "WTSSendMessageW",
            "WTSVirtualChannelOpenEx",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        assert!(ir.contains("call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)"));
        for forbidden in ["@system(", "@popen(", "@malloc(", "@free(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
