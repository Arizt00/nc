pub(crate) fn render_process_temp_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if !(target.contains("windows") || target.contains("msvc") || target.contains("win32")) {
        return Err(format!(
            "manual process temp runtime requires a Windows target, got '{target_triple}'"
        ));
    }
    if !(target.contains("x86_64") || target.contains("aarch64")) {
        return Err(format!(
            "manual process temp runtime supports Win64 x86_64/aarch64, got '{target_triple}'"
        ));
    }
    Ok(WIN64_PROCESS_TEMP_RUNTIME.to_string())
}

const WIN64_PROCESS_TEMP_RUNTIME: &str = r#"
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @GetTempPathW(i32, ptr)
declare i32 @GetTempFileNameW(ptr, ptr, i32, ptr)
declare ptr @CreateFileW(ptr, i32, i32, ptr, i32, i32, ptr)
declare i32 @DeleteFileW(ptr)
declare i32 @CreateDirectoryW(ptr, ptr)
declare i32 @RemoveDirectoryW(ptr)
declare i32 @MoveFileExW(ptr, ptr, i32)
declare i32 @GetFileAttributesW(ptr)
declare i32 @SetFileAttributesW(ptr, i32)
declare i32 @GetFinalPathNameByHandleW(ptr, ptr, i32, i32)
declare i32 @CloseHandle(ptr)
declare i32 @GetLastError()

define ptr @__ncc_temp_wide_alloc(ptr %text) {
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

define ptr @__ncc_temp_wide_buffer(i64 %chars) {
entry:
  %valid = icmp sgt i64 %chars, 0
  br i1 %valid, label %allocate, label %failure
allocate:
  %bytes = shl i64 %chars, 1
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  ret ptr %buffer
failure:
  ret ptr null
}

define i1 @__ncc_temp_wide_free(ptr %buffer) {
entry:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  ret i1 %released
}

define i64 @__ncc_temp_directory(ptr %buffer, i64 %capacity64) {
entry:
  %capacity = trunc i64 %capacity64 to i32
  %chars32 = call i32 @GetTempPathW(i32 %capacity, ptr %buffer)
  %chars64 = zext i32 %chars32 to i64
  ret i64 %chars64
}

define i64 @__ncc_temp_file_name(ptr %directory, ptr %prefix, i64 %unique64, ptr %output) {
entry:
  %unique = trunc i64 %unique64 to i32
  %value32 = call i32 @GetTempFileNameW(ptr %directory, ptr %prefix, i32 %unique, ptr %output)
  %value64 = zext i32 %value32 to i64
  ret i64 %value64
}

define ptr @__ncc_temp_open(ptr %path, i64 %access64, i64 %share64, ptr %security, i64 %creation64, i64 %flags64, ptr %template) {
entry:
  %access = trunc i64 %access64 to i32
  %share = trunc i64 %share64 to i32
  %creation = trunc i64 %creation64 to i32
  %flags = trunc i64 %flags64 to i32
  %handle = call ptr @CreateFileW(ptr %path, i32 %access, i32 %share, ptr %security, i32 %creation, i32 %flags, ptr %template)
  ret ptr %handle
}

define i1 @__ncc_temp_delete_file(ptr %path) {
entry:
  %status = call i32 @DeleteFileW(ptr %path)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_temp_create_directory(ptr %path, ptr %security) {
entry:
  %status = call i32 @CreateDirectoryW(ptr %path, ptr %security)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_temp_remove_directory(ptr %path) {
entry:
  %status = call i32 @RemoveDirectoryW(ptr %path)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i1 @__ncc_temp_move(ptr %from, ptr %to, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %status = call i32 @MoveFileExW(ptr %from, ptr %to, i32 %flags)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_temp_attributes(ptr %path) {
entry:
  %attributes32 = call i32 @GetFileAttributesW(ptr %path)
  %attributes64 = zext i32 %attributes32 to i64
  ret i64 %attributes64
}

define i1 @__ncc_temp_set_attributes(ptr %path, i64 %attributes64) {
entry:
  %attributes = trunc i64 %attributes64 to i32
  %status = call i32 @SetFileAttributesW(ptr %path, i32 %attributes)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_temp_final_path(ptr %handle, ptr %buffer, i64 %capacity64, i64 %flags64) {
entry:
  %capacity = trunc i64 %capacity64 to i32
  %flags = trunc i64 %flags64 to i32
  %chars32 = call i32 @GetFinalPathNameByHandleW(ptr %handle, ptr %buffer, i32 %capacity, i32 %flags)
  %chars64 = zext i32 %chars32 to i64
  ret i64 %chars64
}

define i1 @__ncc_temp_close(ptr %handle) {
entry:
  %status = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
}

define i64 @__ncc_temp_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error64 = zext i32 %error32 to i64
  ret i64 %error64
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_temp_runtime;

    #[test]
    fn win64_temp_runtime_is_manual_crt_free_and_owns_no_hidden_storage() {
        let ir = render_process_temp_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process temp runtime");
        for material in [
            "__ncc_temp_wide_alloc",
            "__ncc_temp_wide_buffer",
            "__ncc_temp_directory",
            "__ncc_temp_file_name",
            "__ncc_temp_open",
            "__ncc_temp_final_path",
            "__ncc_temp_close",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "GetTempPathW",
            "GetTempFileNameW",
            "CreateFileW",
            "DeleteFileW",
            "MoveFileExW",
            "GetFinalPathNameByHandleW",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        assert!(ir.contains("call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)"));
        for forbidden in ["@tmpfile(", "@tempnam(", "@malloc(", "@free(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
