pub(crate) fn render_process_user_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if !(target.contains("windows") || target.contains("msvc") || target.contains("win32")) {
        return Err(format!(
            "manual process user runtime requires a Windows target, got '{target_triple}'"
        ));
    }
    if !(target.contains("x86_64") || target.contains("aarch64")) {
        return Err(format!(
            "manual process user runtime supports Win64 x86_64/aarch64, got '{target_triple}'"
        ));
    }
    Ok(WIN64_PROCESS_USER_RUNTIME.to_string())
}

const WIN64_PROCESS_USER_RUNTIME: &str = r#"
declare ptr @GetCurrentProcess()
declare i32 @GetUserNameW(ptr, ptr)
declare i32 @GetUserNameExW(i32, ptr, ptr)
declare i32 @GetUserProfileDirectoryW(ptr, ptr, ptr)
declare i32 @OpenProcessToken(ptr, i32, ptr)
declare i32 @GetTokenInformation(ptr, i32, ptr, i32, ptr)
declare i32 @CheckTokenMembership(ptr, ptr, ptr)
declare i32 @CreateWellKnownSid(i32, ptr, ptr, ptr)
declare i32 @DuplicateToken(ptr, i32, ptr)
declare i32 @SetThreadToken(ptr, ptr)
declare i32 @ImpersonateLoggedOnUser(ptr)
declare i32 @RevertToSelf()
declare i32 @LoadUserProfileW(ptr, ptr)
declare i32 @UnloadUserProfile(ptr, ptr)
declare i32 @CloseHandle(ptr)
declare i32 @GetLastError()

define ptr @__ncc_user_buffer(i64 %bytes) {
entry:
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  ret ptr %buffer
}

define i1 @__ncc_user_free(ptr %buffer) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  ret i1 %released
}

define ptr @__ncc_user_current_process() {
entry:
  %process = call ptr @GetCurrentProcess()
  ret ptr %process
}

define i1 @__ncc_user_name(ptr %buffer, ptr %size_slot) {
entry:
  %status = call i32 @GetUserNameW(ptr %buffer, ptr %size_slot)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_name_ex(i64 %format64, ptr %buffer, ptr %size_slot) {
entry:
  %format = trunc i64 %format64 to i32
  %status = call i32 @GetUserNameExW(i32 %format, ptr %buffer, ptr %size_slot)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_profile_directory(ptr %token, ptr %buffer, ptr %size_slot) {
entry:
  %status = call i32 @GetUserProfileDirectoryW(ptr %token, ptr %buffer, ptr %size_slot)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_open_process_token(ptr %process, i64 %access64, ptr %out_token) {
entry:
  %access = trunc i64 %access64 to i32
  %status = call i32 @OpenProcessToken(ptr %process, i32 %access, ptr %out_token)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_token_information(ptr %token, i64 %class64, ptr %buffer, i64 %capacity64, ptr %required) {
entry:
  %class = trunc i64 %class64 to i32
  %capacity = trunc i64 %capacity64 to i32
  %status = call i32 @GetTokenInformation(ptr %token, i32 %class, ptr %buffer, i32 %capacity, ptr %required)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_check_membership(ptr %token, ptr %sid, ptr %is_member) {
entry:
  %status = call i32 @CheckTokenMembership(ptr %token, ptr %sid, ptr %is_member)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_create_well_known_sid(i64 %kind64, ptr %domain_sid, ptr %sid, ptr %size_slot) {
entry:
  %kind = trunc i64 %kind64 to i32
  %status = call i32 @CreateWellKnownSid(i32 %kind, ptr %domain_sid, ptr %sid, ptr %size_slot)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_duplicate_token(ptr %token, i64 %level64, ptr %out_token) {
entry:
  %level = trunc i64 %level64 to i32
  %status = call i32 @DuplicateToken(ptr %token, i32 %level, ptr %out_token)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_set_thread_token(ptr %thread_slot, ptr %token) {
entry:
  %status = call i32 @SetThreadToken(ptr %thread_slot, ptr %token)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_impersonate(ptr %token) {
entry:
  %status = call i32 @ImpersonateLoggedOnUser(ptr %token)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_revert() {
entry:
  %status = call i32 @RevertToSelf()
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_load_profile(ptr %token, ptr %profile_info) {
entry:
  %status = call i32 @LoadUserProfileW(ptr %token, ptr %profile_info)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_unload_profile(ptr %token, ptr %profile) {
entry:
  %status = call i32 @UnloadUserProfile(ptr %token, ptr %profile)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_user_close(ptr %handle) {
entry:
  %status = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_user_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  ret i64 %error64
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_user_runtime;

    #[test]
    fn win64_user_runtime_is_manual_crt_free_and_exposes_tokens_and_profiles() {
        let ir = render_process_user_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process user runtime");
        for material in [
            "__ncc_user_buffer",
            "__ncc_user_name",
            "__ncc_user_open_process_token",
            "__ncc_user_token_information",
            "__ncc_user_create_well_known_sid",
            "__ncc_user_impersonate",
            "__ncc_user_load_profile",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "GetUserNameW",
            "GetUserNameExW",
            "OpenProcessToken",
            "GetTokenInformation",
            "CreateWellKnownSid",
            "ImpersonateLoggedOnUser",
            "LoadUserProfileW",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for forbidden in ["@getpwuid(", "@getuid(", "@malloc(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
