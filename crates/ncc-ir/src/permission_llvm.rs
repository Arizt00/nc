pub(crate) fn render_permission_runtime(target: &str) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    if target.contains("windows") {
        if is_32_bit_target(&target) {
            Ok(format!("{WINDOWS_I386_ABI}\n{WINDOWS_RUNTIME}"))
        } else {
            Ok(format!("{WINDOWS_ABI}\n{WINDOWS_RUNTIME}"))
        }
    } else if target.contains("linux")
        || target.contains("android")
        || target.contains("darwin")
        || target.contains("apple")
    {
        Ok(POSIX_RUNTIME.to_string())
    } else {
        Err(format!(
            "permission has no direct operating-system ABI for target '{target}'"
        ))
    }
}

fn is_32_bit_target(target: &str) -> bool {
    target.contains("i386")
        || target.contains("i486")
        || target.contains("i586")
        || target.contains("i686")
}

const WINDOWS_ABI: &str = r#"
declare ptr @CreateFileA(ptr, i32, i32, ptr, i32, i32, ptr)
declare i32 @CloseHandle(ptr)
declare i32 @GetFileAttributesA(ptr)

define internal ptr @ncc.permission.open(ptr %path, i32 %desired) {
entry:
  %handle = call ptr @CreateFileA(ptr %path, i32 %desired, i32 7, ptr null, i32 3, i32 33554432, ptr null)
  ret ptr %handle
}

define internal i32 @ncc.permission.close(ptr %handle) {
entry:
  %status = call i32 @CloseHandle(ptr %handle)
  ret i32 %status
}

define internal i32 @ncc.permission.attributes(ptr %path) {
entry:
  %attributes = call i32 @GetFileAttributesA(ptr %path)
  ret i32 %attributes
}
"#;

const WINDOWS_I386_ABI: &str = r#"
declare x86_stdcallcc ptr @"\01_CreateFileA@28"(ptr, i32, i32, ptr, i32, i32, ptr)
declare x86_stdcallcc i32 @"\01_CloseHandle@4"(ptr)
declare x86_stdcallcc i32 @"\01_GetFileAttributesA@4"(ptr)

define internal ptr @ncc.permission.open(ptr %path, i32 %desired) {
entry:
  %handle = call x86_stdcallcc ptr @"\01_CreateFileA@28"(ptr %path, i32 %desired, i32 7, ptr null, i32 3, i32 33554432, ptr null)
  ret ptr %handle
}

define internal i32 @ncc.permission.close(ptr %handle) {
entry:
  %status = call x86_stdcallcc i32 @"\01_CloseHandle@4"(ptr %handle)
  ret i32 %status
}

define internal i32 @ncc.permission.attributes(ptr %path) {
entry:
  %attributes = call x86_stdcallcc i32 @"\01_GetFileAttributesA@4"(ptr %path)
  ret i32 %attributes
}
"#;

const WINDOWS_RUNTIME: &str = r#"
define internal i32 @ncc.permission.probe(ptr %path, i32 %desired) {
entry:
  %null = icmp eq ptr %path, null
  br i1 %null, label %failure, label %open
open:
  %handle = call ptr @ncc.permission.open(ptr %path, i32 %desired)
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  br i1 %invalid, label %failure, label %close
close:
  %closed = call i32 @ncc.permission.close(ptr %handle)
  ret i32 1
failure:
  ret i32 0
}

define i32 @ncc_rt_permission_can_read(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.probe(ptr %path, i32 -2147483648)
  ret i32 %allowed
}

define i32 @ncc_rt_permission_can_write(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.probe(ptr %path, i32 1073741824)
  ret i32 %allowed
}

define i32 @ncc_rt_permission_can_execute(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.probe(ptr %path, i32 536870912)
  ret i32 %allowed
}

define i64 @ncc_rt_permission_require(ptr %path) {
entry:
  %null = icmp eq ptr %path, null
  br i1 %null, label %failure, label %query
query:
  %attributes = call i32 @ncc.permission.attributes(ptr %path)
  %exists = icmp ne i32 %attributes, -1
  %result = select i1 %exists, i64 3, i64 2
  ret i64 %result
failure:
  ret i64 2
}
"#;

const POSIX_RUNTIME: &str = r#"
declare i32 @access(ptr, i32)

define internal i32 @ncc.permission.access(ptr %path, i32 %mode) {
entry:
  %null = icmp eq ptr %path, null
  br i1 %null, label %failure, label %probe
probe:
  %status = call i32 @access(ptr %path, i32 %mode)
  %allowed = icmp eq i32 %status, 0
  %result = zext i1 %allowed to i32
  ret i32 %result
failure:
  ret i32 0
}

define i32 @ncc_rt_permission_can_read(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.access(ptr %path, i32 4)
  ret i32 %allowed
}

define i32 @ncc_rt_permission_can_write(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.access(ptr %path, i32 2)
  ret i32 %allowed
}

define i32 @ncc_rt_permission_can_execute(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.access(ptr %path, i32 1)
  ret i32 %allowed
}

define i64 @ncc_rt_permission_require(ptr %path) {
entry:
  %allowed = call i32 @ncc.permission.access(ptr %path, i32 0)
  %exists = icmp ne i32 %allowed, 0
  %result = select i1 %exists, i64 3, i64 2
  ret i64 %result
}
"#;

#[cfg(test)]
mod tests {
    use super::render_permission_runtime;

    #[test]
    fn permission_selects_material_os_abis_without_c_source() {
        let win64 =
            render_permission_runtime("x86_64-pc-windows-msvc").expect("Win64 permission runtime");
        assert!(win64.contains("@CreateFileA"));
        assert!(win64.contains("@GetFileAttributesA"));
        assert!(win64.contains("define i32 @ncc_rt_permission_can_execute"));

        let win32 =
            render_permission_runtime("i686-pc-windows-msvc").expect("Win32 permission runtime");
        assert!(win32.contains("x86_stdcallcc"));
        assert!(win32.contains("\\01_CreateFileA@28"));

        for target in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
            let unix = render_permission_runtime(target).expect("POSIX permission runtime");
            assert!(unix.contains("declare i32 @access(ptr, i32)"));
            assert!(unix.contains("define i64 @ncc_rt_permission_require"));
        }
    }

    #[test]
    fn unsupported_freestanding_target_is_rejected_explicitly() {
        let error = render_permission_runtime("riscv64-unknown-none")
            .expect_err("freestanding permission runtime must be rejected");
        assert!(error.contains("no direct operating-system ABI"));
    }
}
