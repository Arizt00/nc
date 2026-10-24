#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryMapResult {
    Direct,
    Pointer,
    Bool,
}

pub(crate) struct MemoryMapSignature {
    pub(crate) symbol: &'static str,
    pub(crate) params: &'static [&'static str],
    pub(crate) return_type: &'static str,
    pub(crate) result: MemoryMapResult,
}

pub(crate) fn is_memory_map_function(name: &str) -> bool {
    memory_map_signature(name).is_some()
}

pub(crate) fn memory_map_signature(name: &str) -> Option<MemoryMapSignature> {
    let signature = match name {
        "map" => (
            "__ncc_rt_map_default",
            &["i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapAnonymous" => (
            "__ncc_rt_map_anonymous",
            &["i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapAt" => (
            "__ncc_rt_map_at",
            &["i64", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapShared" => (
            "__ncc_rt_map_shared",
            &["i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapPrivate" => (
            "__ncc_rt_map_private",
            &["i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapFile" => (
            "__ncc_rt_map_file",
            &["ptr", "i64", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapFileShared" => (
            "__ncc_rt_map_file_shared",
            &["ptr", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapFilePrivate" => (
            "__ncc_rt_map_file_private",
            &["ptr", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "unmap" => (
            "__ncc_rt_map_unmap",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "remap" => (
            "__ncc_rt_map_remap",
            &["ptr", "i64", "i1"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "remapAt" => (
            "__ncc_rt_map_remap_at",
            &["ptr", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Pointer,
        ),
        "mapProtect" => (
            "__ncc_rt_map_protect",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "mapSync" => (
            "__ncc_rt_map_sync",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "mapAdvise" => (
            "__ncc_rt_map_advise",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "mapLock" => (
            "__ncc_rt_map_lock",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "mapUnlock" => (
            "__ncc_rt_map_unlock",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Bool,
        ),
        "mapLen" => (
            "__ncc_rt_map_len",
            &["ptr"][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "mapAddress" => (
            "__ncc_rt_map_address",
            &["ptr"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "mapIsMapped" => (
            "__ncc_rt_map_is_mapped",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "alignedAlloc" => (
            "__ncc_raw_aligned_alloc",
            &["i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "alignedRealloc" => (
            "__ncc_raw_aligned_realloc",
            &["ptr", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "alignedFree" => (
            "__ncc_raw_aligned_free",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmPageSize" => (
            "__ncc_raw_vm_page_size",
            &[][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "vmAllocationGranularity" => (
            "__ncc_raw_vm_allocation_granularity",
            &[][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "vmReserve" => (
            "__ncc_raw_vm_reserve",
            &["ptr", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "vmCommit" => (
            "__ncc_raw_vm_commit",
            &["ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmDecommit" => (
            "__ncc_raw_vm_decommit",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmRelease" => (
            "__ncc_raw_vm_release",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmProtect" => (
            "__ncc_raw_vm_protect",
            &["ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmGuard" => (
            "__ncc_raw_vm_guard",
            &["ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmMap" => (
            "__ncc_raw_vm_map",
            &["ptr", "i64", "i64", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "vmUnmap" => (
            "__ncc_raw_vm_unmap",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmRemap" => (
            "__ncc_raw_vm_remap",
            &["ptr", "i64", "i64", "i64", "i64", "ptr"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "vmAdvise" => (
            "__ncc_raw_vm_advise",
            &["ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmSync" => (
            "__ncc_raw_vm_sync",
            &["ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmLock" => (
            "__ncc_raw_vm_lock",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmUnlock" => (
            "__ncc_raw_vm_unlock",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmExecutable" => (
            "__ncc_raw_vm_executable",
            &["ptr", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmNumaAlloc" => (
            "__ncc_raw_vm_numa_alloc",
            &["i64", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "vmNumaBind" => (
            "__ncc_raw_vm_numa_bind",
            &["ptr", "i64", "i64", "ptr", "i64", "i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmLastError" => (
            "__ncc_raw_vm_last_error",
            &[][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "vmSharedCreate" => (
            "__ncc_raw_vm_shared_create",
            &["ptr", "i64", "i64"][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "vmSharedOpen" => (
            "__ncc_raw_vm_shared_open",
            &["ptr", "i64"][..],
            "i64",
            MemoryMapResult::Direct,
        ),
        "vmSharedMap" => (
            "__ncc_raw_vm_shared_map",
            &["i64", "ptr", "i64", "i64", "i64", "i64"][..],
            "ptr",
            MemoryMapResult::Direct,
        ),
        "vmSharedClose" => (
            "__ncc_raw_vm_shared_close",
            &["i64"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        "vmSharedUnlink" => (
            "__ncc_raw_vm_shared_unlink",
            &["ptr"][..],
            "i1",
            MemoryMapResult::Direct,
        ),
        _ => return None,
    };
    Some(MemoryMapSignature {
        symbol: signature.0,
        params: signature.1,
        return_type: signature.2,
        result: signature.3,
    })
}

#[cfg(test)]
mod signature_tests {
    use super::{
        memory_map_signature, render_memory_map_runtime, render_raw_memory_runtime, MemoryMapResult,
    };

    #[test]
    fn classifies_raw_runtime_returns_by_surface_contract() {
        for name in [
            "map",
            "mapAnonymous",
            "mapAt",
            "mapShared",
            "mapPrivate",
            "mapFile",
            "mapFileShared",
            "mapFilePrivate",
            "remap",
            "remapAt",
        ] {
            assert_eq!(
                memory_map_signature(name).expect(name).result,
                MemoryMapResult::Pointer,
                "{name} must be adapted to Result<ptr<byte>, MemoryError>"
            );
        }
        for name in [
            "unmap",
            "mapProtect",
            "mapSync",
            "mapAdvise",
            "mapLock",
            "mapUnlock",
        ] {
            assert_eq!(
                memory_map_signature(name).expect(name).result,
                MemoryMapResult::Bool,
                "{name} must be adapted to Result<bool, MemoryError>"
            );
        }
        for name in ["mapLen", "mapAddress", "mapIsMapped"] {
            assert_eq!(
                memory_map_signature(name).expect(name).result,
                MemoryMapResult::Direct,
                "{name} is not a Result-returning operation"
            );
        }
        for name in [
            "alignedAlloc",
            "alignedRealloc",
            "alignedFree",
            "vmPageSize",
            "vmReserve",
            "vmCommit",
            "vmDecommit",
            "vmRelease",
            "vmProtect",
            "vmGuard",
            "vmMap",
            "vmUnmap",
            "vmRemap",
            "vmAdvise",
            "vmSync",
            "vmLock",
            "vmUnlock",
            "vmExecutable",
            "vmNumaAlloc",
            "vmNumaBind",
            "vmLastError",
            "vmSharedCreate",
            "vmSharedOpen",
            "vmSharedMap",
            "vmSharedClose",
            "vmSharedUnlink",
        ] {
            assert_eq!(
                memory_map_signature(name).expect(name).result,
                MemoryMapResult::Direct,
                "{name} must expose the raw OS result without tagged adaptation"
            );
        }
        assert!(memory_map_signature("notAMemoryMapOperation").is_none());
    }

    #[test]
    fn raw_runtime_is_platform_specific_and_has_no_registry() {
        let linux = render_raw_memory_runtime("x86_64-unknown-linux-gnu", true)
            .expect("Linux raw memory runtime");
        assert!(linux.contains("@mmap("));
        assert!(linux.contains("@mremap("));
        assert!(linux.contains("@syscall(i64 237"));
        assert!(!linux.contains("__ncc_map_head"));
        assert!(!linux.contains("@PAGE_SIZE_SELECTOR@"));

        let windows = render_raw_memory_runtime("x86_64-pc-windows-msvc", true)
            .expect("Windows raw memory runtime");
        assert!(windows.contains("@VirtualAlloc("));
        assert!(windows.contains("@VirtualAllocExNuma("));
        assert!(windows.contains("@FlushInstructionCache("));
        assert!(windows.contains("define ptr @__ncc_raw_aligned_alloc"));
        assert!(windows.contains("call void @llvm.memcpy.p0.p0.i64"));
        assert!(!windows.contains("@_aligned_malloc"));
        assert!(!windows.contains("@_aligned_realloc"));
        assert!(!windows.contains("@_aligned_free"));
        assert!(!windows.contains("__ncc_map_head"));

        let darwin = render_raw_memory_runtime("aarch64-apple-darwin", true)
            .expect("Darwin raw memory runtime");
        assert!(darwin.contains("@__error()"));
        assert!(darwin.contains("@sysconf(i32 29)"));
        assert!(!darwin.contains("__ncc_map_head"));
    }

    #[test]
    fn windows_i386_adapts_u64_nc_lengths_to_stdcall_size_t() {
        let raw = render_raw_memory_runtime("i686-pc-windows-msvc", true)
            .expect("Win32 raw memory runtime");
        assert!(raw.contains("@\"\\01_VirtualAlloc@16\""), "{raw}");
        assert!(raw.contains("call x86_stdcallcc ptr @\"\\01_VirtualAlloc@16\""));
        assert!(raw.contains("%len32 = trunc i64 %len to i32"));
        assert!(!raw.contains("declare ptr @VirtualAlloc(ptr, i64"));
        assert!(raw.contains("%slot = getelementptr i8, ptr %info, i64 24"));

        let tracked = render_memory_map_runtime("i686-pc-windows-msvc")
            .expect("Win32 tracked memory runtime");
        assert!(tracked.contains("@\"\\01_MapViewOfFileEx@24\""));
        assert!(tracked.contains("define internal ptr @__ncc_winapi_MapViewOfFileEx"));
        assert!(!tracked.contains("declare ptr @MapViewOfFileEx"));
    }

    #[test]
    fn freestanding_raw_runtime_is_an_explicit_manual_hook_abi() {
        for target in [
            "riscv32imac-unknown-elf",
            "riscv64gc-unknown-elf",
            "armv7-unknown-none-eabi",
            "aarch64-unknown-none",
        ] {
            let raw = render_raw_memory_runtime(target, true).expect("freestanding hook ABI");
            assert!(
                raw.contains("declare ptr @__ncc_raw_vm_reserve(ptr, i64, i64)"),
                "{target}: {raw}"
            );
            assert!(
                raw.contains("declare i1 @__ncc_raw_vm_commit(ptr, i64, i64)"),
                "{target}: {raw}"
            );
            assert!(
                raw.contains("declare i1 @__ncc_raw_vm_executable(ptr, i64)"),
                "{target}: {raw}"
            );
            assert!(!raw.contains("@mmap("), "{target}: {raw}");
            assert!(!raw.contains("@VirtualAlloc("), "{target}: {raw}");
            assert!(!raw.contains("define "), "{target}: {raw}");
        }
    }
}

pub(crate) fn render_memory_map_runtime(target_triple: &str) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if target.contains("windows") || target.contains("msvc") || target.contains("win32") {
        return Ok(render_windows_runtime(&target));
    }
    if target.contains("linux")
        || target.contains("darwin")
        || target.contains("freebsd")
        || target.contains("netbsd")
        || target.contains("openbsd")
        || target.contains("dragonfly")
    {
        return Ok(render_posix_runtime(&target));
    }
    Err(format!(
        "memory mapping requires a hosted POSIX or Windows target; '{target_triple}' has no OS mapping contract"
    ))
}

fn render_posix_runtime(target: &str) -> String {
    let linux = target.contains("linux");
    let anonymous_flag = if target.contains("darwin") {
        0x1000
    } else {
        0x20
    };
    let fixed_flag = if target.contains("darwin") {
        0x10
    } else {
        0x10
    };
    let populate_flag = if linux { 0x8000 } else { 0 };
    let no_reserve_flag = if linux { 0x4000 } else { 0 };
    let stack_flag = if linux { 0x20000 } else { 0 };
    let locked_flag = if linux { 0x2000 } else { 0 };

    let mut runtime = POSIX_RUNTIME
        .replace("@MAP_ANONYMOUS@", &anonymous_flag.to_string())
        .replace("@MAP_FIXED@", &fixed_flag.to_string())
        .replace("@MAP_POPULATE@", &populate_flag.to_string())
        .replace("@MAP_NORESERVE@", &no_reserve_flag.to_string())
        .replace("@MAP_STACK@", &stack_flag.to_string())
        .replace("@MAP_LOCKED@", &locked_flag.to_string());
    runtime.push_str(if linux {
        POSIX_LINUX_REMAP_RUNTIME
    } else {
        POSIX_PORTABLE_REMAP_RUNTIME
    });
    runtime
}

pub(crate) fn render_raw_memory_runtime(
    target_triple: &str,
    include_common_declarations: bool,
) -> Result<String, String> {
    let target = target_triple.to_ascii_lowercase();
    if target.contains("windows") || target.contains("msvc") || target.contains("win32") {
        let mut runtime = String::new();
        if include_common_declarations {
            runtime.push_str(WINDOWS_RAW_COMMON_DECLARATIONS);
        }
        runtime.push_str(WINDOWS_RAW_MEMORY_RUNTIME);
        if is_i386_target(&target) {
            runtime = render_windows_i386_abi(&runtime, include_common_declarations);
            runtime = runtime.replace(
                "%slot = getelementptr i8, ptr %info, i64 40",
                "%slot = getelementptr i8, ptr %info, i64 24",
            );
        }
        return Ok(runtime);
    }
    if !(target.contains("linux")
        || target.contains("darwin")
        || target.contains("freebsd")
        || target.contains("netbsd")
        || target.contains("openbsd")
        || target.contains("dragonfly"))
    {
        return Ok(FREESTANDING_RAW_MEMORY_HOOK_DECLARATIONS.to_string());
    }
    let linux = target.contains("linux");
    let anonymous_flag = if target.contains("darwin") {
        0x1000
    } else {
        0x20
    };
    let page_size_selector = if target.contains("darwin") { 29 } else { 30 };
    let errno_symbol = if target.contains("darwin") {
        "__error"
    } else if target.contains("android") {
        "__errno"
    } else {
        "__errno_location"
    };
    let mbind_syscall = if target.contains("x86_64") {
        237
    } else if target.contains("aarch64") || target.contains("riscv64") {
        235
    } else if target.contains("i386") || target.contains("i686") {
        274
    } else {
        -1
    };
    let mut runtime = String::new();
    if include_common_declarations {
        runtime.push_str(POSIX_RAW_COMMON_DECLARATIONS);
    }
    runtime.push_str(
        &POSIX_RAW_MEMORY_RUNTIME
            .replace("@PAGE_SIZE_SELECTOR@", &page_size_selector.to_string())
            .replace("@ERRNO_SYMBOL@", errno_symbol)
            .replace("@MBIND_SYSCALL@", &mbind_syscall.to_string())
            .replace("@MAP_ANONYMOUS@", &anonymous_flag.to_string()),
    );
    runtime.push_str(if linux {
        POSIX_RAW_LINUX_REMAP_RUNTIME
    } else {
        POSIX_RAW_PORTABLE_REMAP_RUNTIME
    });
    Ok(runtime)
}

const FREESTANDING_RAW_MEMORY_HOOK_DECLARATIONS: &str = r#"
; nC freestanding raw-memory hook ABI. The image author supplies every live hook.
declare ptr @__ncc_raw_aligned_alloc(i64, i64)
declare ptr @__ncc_raw_aligned_realloc(ptr, i64, i64, i64)
declare i1 @__ncc_raw_aligned_free(ptr)
declare i64 @__ncc_raw_vm_page_size()
declare i64 @__ncc_raw_vm_allocation_granularity()
declare ptr @__ncc_raw_vm_reserve(ptr, i64, i64)
declare i1 @__ncc_raw_vm_commit(ptr, i64, i64)
declare i1 @__ncc_raw_vm_decommit(ptr, i64)
declare i1 @__ncc_raw_vm_release(ptr, i64)
declare i1 @__ncc_raw_vm_protect(ptr, i64, i64)
declare i1 @__ncc_raw_vm_guard(ptr, i64, i64)
declare ptr @__ncc_raw_vm_map(ptr, i64, i64, i64, i64, i64)
declare i1 @__ncc_raw_vm_unmap(ptr, i64)
declare ptr @__ncc_raw_vm_remap(ptr, i64, i64, i64, i64, ptr)
declare i1 @__ncc_raw_vm_advise(ptr, i64, i64)
declare i1 @__ncc_raw_vm_sync(ptr, i64, i64)
declare i1 @__ncc_raw_vm_lock(ptr, i64)
declare i1 @__ncc_raw_vm_unlock(ptr, i64)
declare i1 @__ncc_raw_vm_executable(ptr, i64)
declare ptr @__ncc_raw_vm_numa_alloc(i64, i64, i64, i64)
declare i1 @__ncc_raw_vm_numa_bind(ptr, i64, i64, ptr, i64, i64)
declare i64 @__ncc_raw_vm_last_error()
declare i64 @__ncc_raw_vm_shared_create(ptr, i64, i64)
declare i64 @__ncc_raw_vm_shared_open(ptr, i64)
declare ptr @__ncc_raw_vm_shared_map(i64, ptr, i64, i64, i64, i64)
declare i1 @__ncc_raw_vm_shared_close(i64)
declare i1 @__ncc_raw_vm_shared_unlink(ptr)
"#;

const POSIX_RUNTIME: &str = r#"
%ncc.map.node = type { ptr, i64, i32, i32, i1, ptr }

@__ncc_map_head = internal global ptr null, align 8
@__ncc_map_guard = internal global i8 0, align 1

declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i32 @mprotect(ptr, i64, i32)
declare i32 @msync(ptr, i64, i32)
declare i32 @madvise(ptr, i64, i32)
declare i32 @mlock(ptr, i64)
declare i32 @munlock(ptr, i64)
declare i32 @open(ptr, i32, ...)
declare i32 @close(i32)

define internal void @__ncc_map_registry_lock() {
entry:
  br label %spin
spin:
  %attempt = cmpxchg ptr @__ncc_map_guard, i8 0, i8 1 acquire monotonic
  %won = extractvalue { i8, i1 } %attempt, 1
  br i1 %won, label %done, label %spin
done:
  ret void
}

define internal void @__ncc_map_registry_unlock() {
entry:
  store atomic i8 0, ptr @__ncc_map_guard release, align 1
  ret void
}

define internal ptr @__ncc_map_find_locked(ptr %address, i1 %base_only) {
entry:
  %address.i = ptrtoint ptr %address to i64
  %first = load ptr, ptr @__ncc_map_head, align 8
  br label %loop
loop:
  %node = phi ptr [ %first, %entry ], [ %next, %step ]
  %empty = icmp eq ptr %node, null
  br i1 %empty, label %missing, label %check
check:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  %base = load ptr, ptr %base.slot, align 8
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %live.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 4
  %live = load i1, ptr %live.slot, align 1
  %base.i = ptrtoint ptr %base to i64
  %end.i = add i64 %base.i, %len
  %at.base = icmp eq i64 %address.i, %base.i
  %above = icmp uge i64 %address.i, %base.i
  %below = icmp ult i64 %address.i, %end.i
  %inside = and i1 %above, %below
  %address.ok = select i1 %base_only, i1 %at.base, i1 %inside
  %matches = and i1 %live, %address.ok
  br i1 %matches, label %found, label %step
step:
  %next.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 5
  %next = load ptr, ptr %next.slot, align 8
  br label %loop
found:
  ret ptr %node
missing:
  ret ptr null
}

define internal ptr @__ncc_map_register(ptr %base, i64 %len, i32 %protection, i32 %flags) {
entry:
  %node = call ptr @malloc(i64 48)
  %failed = icmp eq ptr %node, null
  br i1 %failed, label %error, label %store
store:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  store ptr %base, ptr %base.slot, align 8
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  store i64 %len, ptr %len.slot, align 8
  %protection.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 2
  store i32 %protection, ptr %protection.slot, align 4
  %flags.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 3
  store i32 %flags, ptr %flags.slot, align 4
  %live.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 4
  store i1 true, ptr %live.slot, align 1
  call void @__ncc_map_registry_lock()
  %head = load ptr, ptr @__ncc_map_head, align 8
  %next.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 5
  store ptr %head, ptr %next.slot, align 8
  store ptr %node, ptr @__ncc_map_head, align 8
  call void @__ncc_map_registry_unlock()
  ret ptr %node
error:
  ret ptr null
}

define internal void @__ncc_map_remove_locked(ptr %target) {
entry:
  %first = load ptr, ptr @__ncc_map_head, align 8
  %is.first = icmp eq ptr %first, %target
  br i1 %is.first, label %remove.head, label %prepare
remove.head:
  %head.next.slot = getelementptr %ncc.map.node, ptr %target, i32 0, i32 5
  %head.next = load ptr, ptr %head.next.slot, align 8
  store ptr %head.next, ptr @__ncc_map_head, align 8
  call void @free(ptr %target)
  ret void
prepare:
  %first.empty = icmp eq ptr %first, null
  br i1 %first.empty, label %missing, label %loop.entry
loop.entry:
  %current.slot = getelementptr %ncc.map.node, ptr %first, i32 0, i32 5
  %current.first = load ptr, ptr %current.slot, align 8
  br label %loop
loop:
  %previous = phi ptr [ %first, %loop.entry ], [ %current, %step ]
  %current = phi ptr [ %current.first, %loop.entry ], [ %next, %step ]
  %empty = icmp eq ptr %current, null
  br i1 %empty, label %missing, label %check
check:
  %matches = icmp eq ptr %current, %target
  br i1 %matches, label %remove, label %step
step:
  %next.slot = getelementptr %ncc.map.node, ptr %current, i32 0, i32 5
  %next = load ptr, ptr %next.slot, align 8
  br label %loop
remove:
  %target.next.slot = getelementptr %ncc.map.node, ptr %target, i32 0, i32 5
  %target.next = load ptr, ptr %target.next.slot, align 8
  %previous.next.slot = getelementptr %ncc.map.node, ptr %previous, i32 0, i32 5
  store ptr %target.next, ptr %previous.next.slot, align 8
  call void @free(ptr %target)
  ret void
missing:
  ret void
}

define internal i32 @__ncc_map_posix_flags(i32 %flags, i1 %file_backed) {
entry:
  %shared.bits = and i32 %flags, 1
  %shared = icmp ne i32 %shared.bits, 0
  %base = select i1 %shared, i32 1, i32 2
  %anonymous = select i1 %file_backed, i32 0, i32 @MAP_ANONYMOUS@
  %with.anonymous = or i32 %base, %anonymous
  %fixed.bits = and i32 %flags, 4
  %fixed = icmp ne i32 %fixed.bits, 0
  %fixed.value = select i1 %fixed, i32 @MAP_FIXED@, i32 0
  %with.fixed = or i32 %with.anonymous, %fixed.value
  %populate.bits = and i32 %flags, 16
  %populate = icmp ne i32 %populate.bits, 0
  %populate.value = select i1 %populate, i32 @MAP_POPULATE@, i32 0
  %with.populate = or i32 %with.fixed, %populate.value
  %noreserve.bits = and i32 %flags, 32
  %noreserve = icmp ne i32 %noreserve.bits, 0
  %noreserve.value = select i1 %noreserve, i32 @MAP_NORESERVE@, i32 0
  %with.noreserve = or i32 %with.populate, %noreserve.value
  %stack.bits = and i32 %flags, 64
  %stack = icmp ne i32 %stack.bits, 0
  %stack.value = select i1 %stack, i32 @MAP_STACK@, i32 0
  %with.stack = or i32 %with.noreserve, %stack.value
  %locked.bits = and i32 %flags, 128
  %locked = icmp ne i32 %locked.bits, 0
  %locked.value = select i1 %locked, i32 @MAP_LOCKED@, i32 0
  %result = or i32 %with.stack, %locked.value
  ret i32 %result
}

define internal ptr @__ncc_map_create(ptr %requested, i64 %len, i32 %protection, i32 %flags, i1 %file_backed, i32 %fd, i64 %offset) {
entry:
  %valid = icmp sgt i64 %len, 0
  br i1 %valid, label %map, label %error
map:
  %os.flags = call i32 @__ncc_map_posix_flags(i32 %flags, i1 %file_backed)
  %base = call ptr @mmap(ptr %requested, i64 %len, i32 %protection, i32 %os.flags, i32 %fd, i64 %offset)
  %failed = icmp eq ptr %base, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %error, label %register
register:
  %node = call ptr @__ncc_map_register(ptr %base, i64 %len, i32 %protection, i32 %flags)
  %registered = icmp ne ptr %node, null
  br i1 %registered, label %ok, label %rollback
rollback:
  %ignored = call i32 @munmap(ptr %base, i64 %len)
  br label %error
ok:
  ret ptr %base
error:
  ret ptr null
}

define ptr @__ncc_rt_map_default(i64 %len) {
entry:
  %out = call ptr @__ncc_map_create(ptr null, i64 %len, i32 3, i32 2, i1 false, i32 -1, i64 0)
  ret ptr %out
}

define ptr @__ncc_rt_map_anonymous(i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @__ncc_map_create(ptr null, i64 %len, i32 %protection, i32 %flags, i1 false, i32 -1, i64 0)
  ret ptr %out
}

define ptr @__ncc_rt_map_at(i64 %address, i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %valid.address = icmp ne i64 %address, 0
  br i1 %valid.address, label %map, label %error
map:
  %requested = inttoptr i64 %address to ptr
  %protection = trunc i64 %protection64 to i32
  %flags.raw = trunc i64 %flags64 to i32
  %flags = or i32 %flags.raw, 4
  %out = call ptr @__ncc_map_create(ptr %requested, i64 %len, i32 %protection, i32 %flags, i1 false, i32 -1, i64 0)
  ret ptr %out
error:
  ret ptr null
}

define ptr @__ncc_rt_map_shared(i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %out = call ptr @__ncc_map_create(ptr null, i64 %len, i32 %protection, i32 1, i1 false, i32 -1, i64 0)
  ret ptr %out
}

define ptr @__ncc_rt_map_private(i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %out = call ptr @__ncc_map_create(ptr null, i64 %len, i32 %protection, i32 2, i1 false, i32 -1, i64 0)
  ret ptr %out
}

define internal ptr @__ncc_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %valid.offset = icmp sge i64 %offset, 0
  %valid.len = icmp sgt i64 %len, 0
  %valid = and i1 %valid.offset, %valid.len
  br i1 %valid, label %open, label %error
open:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %write.bits = and i32 %protection, 2
  %write = icmp ne i32 %write.bits, 0
  %open.flags = select i1 %write, i32 2, i32 0
  %fd = call i32 (ptr, i32, ...) @open(ptr %path, i32 %open.flags)
  %opened = icmp sge i32 %fd, 0
  br i1 %opened, label %map, label %error
map:
  %out = call ptr @__ncc_map_create(ptr null, i64 %len, i32 %protection, i32 %flags, i1 true, i32 %fd, i64 %offset)
  %closed = call i32 @close(i32 %fd)
  ret ptr %out
error:
  ret ptr null
}

define ptr @__ncc_rt_map_file(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 %flags) {
entry:
  %out = call ptr @__ncc_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 %flags)
  ret ptr %out
}

define ptr @__ncc_rt_map_file_shared(ptr %path, i64 %offset, i64 %len, i64 %protection) {
entry:
  %out = call ptr @__ncc_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 1)
  ret ptr %out
}

define ptr @__ncc_rt_map_file_private(ptr %path, i64 %offset, i64 %len, i64 %protection) {
entry:
  %out = call ptr @__ncc_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 2)
  ret ptr %out
}

define i1 @__ncc_rt_map_unmap(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %unmap, label %missing
unmap:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @munmap(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %mark, label %done
mark:
  call void @__ncc_map_remove_locked(ptr %node)
  br label %done
missing:
  br label %done
done:
  %result = phi i1 [ false, %missing ], [ false, %unmap ], [ true, %mark ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_protect(ptr %address, i64 %protection64) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %protection = trunc i64 %protection64 to i32
  %rc = call i32 @mprotect(ptr %address, i64 %len, i32 %protection)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %record, label %done
record:
  %protection.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 2
  store i32 %protection, ptr %protection.slot, align 4
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ false, %apply ], [ true, %record ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_sync(ptr %address, i64 %mode64) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %sync = icmp eq i64 %mode64, 1
  %invalidate = icmp eq i64 %mode64, 2
  %base.mode = select i1 %sync, i32 4, i32 1
  %extra = select i1 %invalidate, i32 2, i32 0
  %mode = or i32 %base.mode, %extra
  %rc = call i32 @msync(ptr %address, i64 %len, i32 %mode)
  %ok = icmp eq i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_advise(ptr %address, i64 %advice64) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %advice = trunc i64 %advice64 to i32
  %rc = call i32 @madvise(ptr %address, i64 %len, i32 %advice)
  %ok = icmp eq i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_lock(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @mlock(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_unlock(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @munlock(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i64 @__ncc_rt_map_len(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  br i1 %found, label %read, label %done
read:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  br label %done
done:
  %result = phi i64 [ -1, %entry ], [ %len, %read ]
  call void @__ncc_map_registry_unlock()
  ret i64 %result
}

define ptr @__ncc_rt_map_address(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  br i1 %found, label %read, label %done
read:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  %base = load ptr, ptr %base.slot, align 8
  br label %done
done:
  %result = phi ptr [ null, %entry ], [ %base, %read ]
  call void @__ncc_map_registry_unlock()
  ret ptr %result
}

define i1 @__ncc_rt_map_is_mapped(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  call void @__ncc_map_registry_unlock()
  ret i1 %found
}
"#;

const POSIX_LINUX_REMAP_RUNTIME: &str = r#"
declare ptr @mremap(ptr, i64, i64, i32, ...)

define ptr @__ncc_rt_map_remap(ptr %address, i64 %new_len, i1 %may_move) {
entry:
  %valid = icmp sgt i64 %new_len, 0
  br i1 %valid, label %lookup, label %error
lookup:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %missing
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %old_len = load i64, ptr %len.slot, align 8
  %flags = select i1 %may_move, i32 1, i32 0
  %out = call ptr (ptr, i64, i64, i32, ...) @mremap(ptr %address, i64 %old_len, i64 %new_len, i32 %flags)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %failed.unlock, label %record
record:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  store ptr %out, ptr %base.slot, align 8
  store i64 %new_len, ptr %len.slot, align 8
  call void @__ncc_map_registry_unlock()
  ret ptr %out
missing:
  call void @__ncc_map_registry_unlock()
  br label %error
failed.unlock:
  call void @__ncc_map_registry_unlock()
  br label %error
error:
  ret ptr null
}

define ptr @__ncc_rt_map_remap_at(ptr %address, i64 %new_len, i64 %new_address) {
entry:
  %valid.len = icmp sgt i64 %new_len, 0
  %valid.address = icmp ne i64 %new_address, 0
  %valid = and i1 %valid.len, %valid.address
  br i1 %valid, label %lookup, label %error
lookup:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %missing
apply:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %old_len = load i64, ptr %len.slot, align 8
  %requested = inttoptr i64 %new_address to ptr
  %out = call ptr (ptr, i64, i64, i32, ...) @mremap(ptr %address, i64 %old_len, i64 %new_len, i32 3, ptr %requested)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %failed.unlock, label %record
record:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  store ptr %out, ptr %base.slot, align 8
  store i64 %new_len, ptr %len.slot, align 8
  call void @__ncc_map_registry_unlock()
  ret ptr %out
missing:
  call void @__ncc_map_registry_unlock()
  br label %error
failed.unlock:
  call void @__ncc_map_registry_unlock()
  br label %error
error:
  ret ptr null
}
"#;

const POSIX_PORTABLE_REMAP_RUNTIME: &str = r#"
define internal ptr @__ncc_map_portable_move(ptr %address, i64 %new_len, ptr %requested, i1 %fixed) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_map_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %allocate, label %missing
allocate:
  %len.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 1
  %old_len = load i64, ptr %len.slot, align 8
  %protection.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 2
  %protection = load i32, ptr %protection.slot, align 4
  %flags.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 3
  %flags.raw = load i32, ptr %flags.slot, align 4
  %flags.fixed = or i32 %flags.raw, 4
  %flags = select i1 %fixed, i32 %flags.fixed, i32 %flags.raw
  %os.flags = call i32 @__ncc_map_posix_flags(i32 %flags, i1 false)
  %out = call ptr @mmap(ptr %requested, i64 %new_len, i32 %protection, i32 %os.flags, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %failed.unlock, label %copy
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy_len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy_len, i1 false)
  %unmapped = call i32 @munmap(ptr %address, i64 %old_len)
  %unmap.ok = icmp eq i32 %unmapped, 0
  br i1 %unmap.ok, label %record, label %rollback
record:
  %base.slot = getelementptr %ncc.map.node, ptr %node, i32 0, i32 0
  store ptr %out, ptr %base.slot, align 8
  store i64 %new_len, ptr %len.slot, align 8
  call void @__ncc_map_registry_unlock()
  ret ptr %out
rollback:
  %ignored = call i32 @munmap(ptr %out, i64 %new_len)
  br label %failed.unlock
missing:
  call void @__ncc_map_registry_unlock()
  ret ptr null
failed.unlock:
  call void @__ncc_map_registry_unlock()
  ret ptr null
}

define ptr @__ncc_rt_map_remap(ptr %address, i64 %new_len, i1 %may_move) {
entry:
  %valid = icmp sgt i64 %new_len, 0
  %move = and i1 %valid, %may_move
  br i1 %move, label %apply, label %same.check
same.check:
  %old_len = call i64 @__ncc_rt_map_len(ptr %address)
  %same = icmp eq i64 %old_len, %new_len
  %same.result = select i1 %same, ptr %address, ptr null
  ret ptr %same.result
apply:
  %out = call ptr @__ncc_map_portable_move(ptr %address, i64 %new_len, ptr null, i1 false)
  ret ptr %out
}

define ptr @__ncc_rt_map_remap_at(ptr %address, i64 %new_len, i64 %new_address) {
entry:
  %valid.len = icmp sgt i64 %new_len, 0
  %valid.address = icmp ne i64 %new_address, 0
  %valid = and i1 %valid.len, %valid.address
  br i1 %valid, label %apply, label %error
apply:
  %requested = inttoptr i64 %new_address to ptr
  %out = call ptr @__ncc_map_portable_move(ptr %address, i64 %new_len, ptr %requested, i1 true)
  ret ptr %out
error:
  ret ptr null
}
"#;

const POSIX_RAW_MEMORY_RUNTIME: &str = r#"
declare i64 @sysconf(i32)
declare i32 @posix_memalign(ptr, i64, i64)
declare ptr @@ERRNO_SYMBOL@()
declare i32 @shm_open(ptr, i32, i32)
declare i32 @shm_unlink(ptr)
declare i32 @ftruncate(i32, i64)
declare i64 @syscall(i64, ...)
declare void @llvm.clear_cache(ptr, ptr)

define ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 %alignment) {
entry:
  %slot = alloca ptr, align 8
  store ptr null, ptr %slot, align 8
  %size.ok = icmp ugt i64 %bytes, 0
  %align.min = icmp uge i64 %alignment, 8
  %minus = sub i64 %alignment, 1
  %mask = and i64 %alignment, %minus
  %align.pow2 = icmp eq i64 %mask, 0
  %align.ok = and i1 %align.min, %align.pow2
  %valid = and i1 %size.ok, %align.ok
  br i1 %valid, label %allocate, label %error
allocate:
  %rc = call i32 @posix_memalign(ptr %slot, i64 %alignment, i64 %bytes)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %loaded, label %error
loaded:
  %out = load ptr, ptr %slot, align 8
  ret ptr %out
error:
  ret ptr null
}

define ptr @__ncc_raw_aligned_realloc(ptr %address, i64 %old_len, i64 %new_len, i64 %alignment) {
entry:
  %release = icmp eq i64 %new_len, 0
  br i1 %release, label %free.old, label %allocate
free.old:
  call void @free(ptr %address)
  ret ptr null
allocate:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %new_len, i64 %alignment)
  %failed = icmp eq ptr %out, null
  br i1 %failed, label %error, label %copy.check
copy.check:
  %has.old = icmp ne ptr %address, null
  br i1 %has.old, label %copy, label %done
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy.len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy.len, i1 false)
  call void @free(ptr %address)
  br label %done
done:
  ret ptr %out
error:
  ret ptr null
}

define i1 @__ncc_raw_aligned_free(ptr %address) {
entry:
  call void @free(ptr %address)
  ret i1 true
}

define i64 @__ncc_raw_vm_page_size() {
entry:
  %size = call i64 @sysconf(i32 @PAGE_SIZE_SELECTOR@)
  ret i64 %size
}

define i64 @__ncc_raw_vm_allocation_granularity() {
entry:
  %size = call i64 @sysconf(i32 @PAGE_SIZE_SELECTOR@)
  ret i64 %size
}

define ptr @__ncc_raw_vm_reserve(ptr %requested, i64 %len, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %anonymous = or i32 %flags, @MAP_ANONYMOUS@
  %native = or i32 %anonymous, 2
  %out = call ptr @mmap(ptr %requested, i64 %len, i32 0, i32 %native, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define i1 @__ncc_raw_vm_commit(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %rc = call i32 @mprotect(ptr %address, i64 %len, i32 %protection)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_decommit(ptr %address, i64 %len) {
entry:
  %discard = call i32 @madvise(ptr %address, i64 %len, i32 4)
  %protect = call i32 @mprotect(ptr %address, i64 %len, i32 0)
  %discard.ok = icmp eq i32 %discard, 0
  %protect.ok = icmp eq i32 %protect, 0
  %ok = and i1 %discard.ok, %protect.ok
  ret i1 %ok
}

define i1 @__ncc_raw_vm_release(ptr %address, i64 %len) {
entry:
  %rc = call i32 @munmap(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_protect(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %rc = call i32 @mprotect(ptr %address, i64 %len, i32 %protection)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_guard(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %rc = call i32 @mprotect(ptr %address, i64 %len, i32 %protection)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define ptr @__ncc_raw_vm_map(ptr %requested, i64 %len, i64 %protection64, i64 %flags64, i64 %handle64, i64 %offset) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %handle = trunc i64 %handle64 to i32
  %out = call ptr @mmap(ptr %requested, i64 %len, i32 %protection, i32 %flags, i32 %handle, i64 %offset)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define i1 @__ncc_raw_vm_unmap(ptr %address, i64 %len) {
entry:
  %rc = call i32 @munmap(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_advise(ptr %address, i64 %len, i64 %advice64) {
entry:
  %advice = trunc i64 %advice64 to i32
  %rc = call i32 @madvise(ptr %address, i64 %len, i32 %advice)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_sync(ptr %address, i64 %len, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %rc = call i32 @msync(ptr %address, i64 %len, i32 %flags)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_lock(ptr %address, i64 %len) {
entry:
  %rc = call i32 @mlock(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_unlock(ptr %address, i64 %len) {
entry:
  %rc = call i32 @munlock(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_executable(ptr %address, i64 %len) {
entry:
  %rc = call i32 @mprotect(ptr %address, i64 %len, i32 5)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %flush, label %done
flush:
  %end = getelementptr i8, ptr %address, i64 %len
  call void @llvm.clear_cache(ptr %address, ptr %end)
  br label %done
done:
  ret i1 %ok
}

define ptr @__ncc_raw_vm_numa_alloc(i64 %len, i64 %node, i64 %protection64, i64 %flags64) {
entry:
  %node.range = icmp ult i64 %node, 64
  br i1 %node.range, label %map, label %error
map:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %anonymous = or i32 %flags, @MAP_ANONYMOUS@
  %native = or i32 %anonymous, 2
  %out = call ptr @mmap(ptr null, i64 %len, i32 %protection, i32 %native, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %error, label %policy
policy:
  %supported = icmp sge i64 @MBIND_SYSCALL@, 0
  br i1 %supported, label %bind, label %uma
bind:
  %mask = alloca i64, align 8
  %bit = shl i64 1, %node
  store i64 %bit, ptr %mask, align 8
  %rc = call i64 (i64, ...) @syscall(i64 @MBIND_SYSCALL@, ptr %out, i64 %len, i32 2, ptr %mask, i64 64, i32 1)
  %ok = icmp eq i64 %rc, 0
  br i1 %ok, label %done, label %rollback
uma:
  %node.zero = icmp eq i64 %node, 0
  br i1 %node.zero, label %done, label %rollback
rollback:
  %ignored = call i32 @munmap(ptr %out, i64 %len)
  br label %error
done:
  ret ptr %out
error:
  ret ptr null
}

define i1 @__ncc_raw_vm_numa_bind(ptr %address, i64 %len, i64 %mode64, ptr %nodemask, i64 %maxnode, i64 %flags64) {
entry:
  %supported = icmp sge i64 @MBIND_SYSCALL@, 0
  br i1 %supported, label %bind, label %unsupported
bind:
  %mode = trunc i64 %mode64 to i32
  %flags = trunc i64 %flags64 to i32
  %rc = call i64 (i64, ...) @syscall(i64 @MBIND_SYSCALL@, ptr %address, i64 %len, i32 %mode, ptr %nodemask, i64 %maxnode, i32 %flags)
  %ok = icmp eq i64 %rc, 0
  ret i1 %ok
unsupported:
  ret i1 false
}

define i64 @__ncc_raw_vm_last_error() {
entry:
  %slot = call ptr @@ERRNO_SYMBOL@()
  %value = load i32, ptr %slot, align 4
  %wide = sext i32 %value to i64
  ret i64 %wide
}

define i64 @__ncc_raw_vm_shared_create(ptr %name, i64 %len, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %fd = call i32 @shm_open(ptr %name, i32 %flags, i32 384)
  %opened = icmp sge i32 %fd, 0
  br i1 %opened, label %size, label %error
size:
  %rc = call i32 @ftruncate(i32 %fd, i64 %len)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %done, label %close
close:
  %ignored = call i32 @close(i32 %fd)
  br label %error
done:
  %wide = sext i32 %fd to i64
  ret i64 %wide
error:
  ret i64 -1
}

define i64 @__ncc_raw_vm_shared_open(ptr %name, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %fd = call i32 @shm_open(ptr %name, i32 %flags, i32 0)
  %wide = sext i32 %fd to i64
  ret i64 %wide
}

define ptr @__ncc_raw_vm_shared_map(i64 %handle64, ptr %requested, i64 %len, i64 %protection64, i64 %flags64, i64 %offset) {
entry:
  %handle = trunc i64 %handle64 to i32
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @mmap(ptr %requested, i64 %len, i32 %protection, i32 %flags, i32 %handle, i64 %offset)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define i1 @__ncc_raw_vm_shared_close(i64 %handle64) {
entry:
  %handle = trunc i64 %handle64 to i32
  %rc = call i32 @close(i32 %handle)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_shared_unlink(ptr %name) {
entry:
  %rc = call i32 @shm_unlink(ptr %name)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
}
"#;

const POSIX_RAW_COMMON_DECLARATIONS: &str = r#"
declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i32 @mprotect(ptr, i64, i32)
declare i32 @msync(ptr, i64, i32)
declare i32 @madvise(ptr, i64, i32)
declare i32 @mlock(ptr, i64)
declare i32 @munlock(ptr, i64)
declare i32 @close(i32)
"#;

const POSIX_RAW_LINUX_REMAP_RUNTIME: &str = r#"
declare ptr @mremap(ptr, i64, i64, i32, ...)

define ptr @__ncc_raw_vm_remap(ptr %address, i64 %old_len, i64 %new_len, i64 %protection64, i64 %flags64, ptr %requested) {
entry:
  %flags = trunc i64 %flags64 to i32
  %fixed = icmp ne ptr %requested, null
  br i1 %fixed, label %at, label %anywhere
at:
  %at.out = call ptr (ptr, i64, i64, i32, ...) @mremap(ptr %address, i64 %old_len, i64 %new_len, i32 %flags, ptr %requested)
  br label %done
anywhere:
  %any.out = call ptr (ptr, i64, i64, i32, ...) @mremap(ptr %address, i64 %old_len, i64 %new_len, i32 %flags)
  br label %done
done:
  %out = phi ptr [ %at.out, %at ], [ %any.out, %anywhere ]
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}
"#;

const POSIX_RAW_PORTABLE_REMAP_RUNTIME: &str = r#"
define ptr @__ncc_raw_vm_remap(ptr %address, i64 %old_len, i64 %new_len, i64 %protection64, i64 %flags64, ptr %requested) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @mmap(ptr %requested, i64 %new_len, i32 %protection, i32 %flags, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  br i1 %failed, label %error, label %copy
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy.len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy.len, i1 false)
  %released = call i32 @munmap(ptr %address, i64 %old_len)
  %ok = icmp eq i32 %released, 0
  br i1 %ok, label %done, label %rollback
rollback:
  %ignored = call i32 @munmap(ptr %out, i64 %new_len)
  br label %error
done:
  ret ptr %out
error:
  ret ptr null
}
"#;

fn render_windows_runtime(target: &str) -> String {
    if is_i386_target(target) {
        render_windows_i386_abi(WINDOWS_RUNTIME, true)
    } else {
        WINDOWS_RUNTIME.to_string()
    }
}

fn is_i386_target(target: &str) -> bool {
    target.contains("i386")
        || target.contains("i486")
        || target.contains("i586")
        || target.contains("i686")
}

const WINDOWS_SYSTEM_API_NAMES: &[&str] = &[
    "VirtualAlloc",
    "VirtualFree",
    "VirtualProtect",
    "VirtualLock",
    "VirtualUnlock",
    "FlushViewOfFile",
    "DiscardVirtualMemory",
    "PrefetchVirtualMemory",
    "GetCurrentProcess",
    "CreateFileA",
    "CreateFileMappingA",
    "MapViewOfFileEx",
    "UnmapViewOfFile",
    "CloseHandle",
    "GetSystemInfo",
    "GetLastError",
    "FlushInstructionCache",
    "VirtualAllocExNuma",
    "OpenFileMappingA",
];

fn render_windows_i386_abi(body: &str, include_wrappers: bool) -> String {
    let mut runtime = body
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("declare ")
                || !WINDOWS_SYSTEM_API_NAMES
                    .iter()
                    .any(|name| trimmed.contains(&format!("@{name}(")))
        })
        .collect::<Vec<_>>()
        .join("\n");
    for name in WINDOWS_SYSTEM_API_NAMES {
        runtime = runtime.replace(&format!("@{name}"), &format!("@__ncc_winapi_{name}"));
    }
    if include_wrappers {
        runtime.push_str(WINDOWS_I386_SYSTEM_API_WRAPPERS);
    }
    runtime
}

const WINDOWS_RUNTIME: &str = r#"
%ncc.winmap.node = type { ptr, i64, i32, i32, i1, ptr, ptr, i32, i64 }

@__ncc_map_head = internal global ptr null, align 8
@__ncc_map_guard = internal global i8 0, align 1

declare ptr @VirtualAlloc(ptr, i64, i32, i32)
declare i32 @VirtualFree(ptr, i64, i32)
declare i32 @VirtualProtect(ptr, i64, i32, ptr)
declare i32 @VirtualLock(ptr, i64)
declare i32 @VirtualUnlock(ptr, i64)
declare i32 @FlushViewOfFile(ptr, i64)
declare i32 @DiscardVirtualMemory(ptr, i64)
declare i32 @PrefetchVirtualMemory(ptr, i64, ptr, i32)
declare ptr @GetCurrentProcess()
declare ptr @CreateFileA(ptr, i32, i32, ptr, i32, i32, ptr)
declare ptr @CreateFileMappingA(ptr, ptr, i32, i32, i32, ptr)
declare ptr @MapViewOfFileEx(ptr, i32, i32, i32, i64, ptr)
declare i32 @UnmapViewOfFile(ptr)
declare i32 @CloseHandle(ptr)

define internal void @__ncc_map_registry_lock() {
entry:
  br label %spin
spin:
  %attempt = cmpxchg ptr @__ncc_map_guard, i8 0, i8 1 acquire monotonic
  %won = extractvalue { i8, i1 } %attempt, 1
  br i1 %won, label %done, label %spin
done:
  ret void
}

define internal void @__ncc_map_registry_unlock() {
entry:
  store atomic i8 0, ptr @__ncc_map_guard release, align 1
  ret void
}

define internal i32 @__ncc_win_page_protection(i32 %protection) {
entry:
  %exec.bits = and i32 %protection, 4
  %write.bits = and i32 %protection, 2
  %read.bits = and i32 %protection, 1
  %exec = icmp ne i32 %exec.bits, 0
  %write = icmp ne i32 %write.bits, 0
  %read = icmp ne i32 %read.bits, 0
  %rw = select i1 %write, i32 4, i32 2
  %normal = select i1 %read, i32 %rw, i32 1
  %xw = select i1 %write, i32 64, i32 32
  %xr = select i1 %read, i32 %xw, i32 16
  %result = select i1 %exec, i32 %xr, i32 %normal
  ret i32 %result
}

define internal i64 @__ncc_win_reservation_len(ptr %requested, i64 %len, i32 %flags) {
entry:
  %requested.null = icmp eq ptr %requested, null
  %fixed.bits = and i32 %flags, 4
  %not.fixed = icmp eq i32 %fixed.bits, 0
  %anonymous.raw = and i1 %requested.null, %not.fixed
  %has.room = icmp ule i64 %len, 9223372036854710271
  %can.extend = and i1 %anonymous.raw, %has.room
  %extended = add i64 %len, 65536
  %result = select i1 %can.extend, i64 %extended, i64 %len
  ret i64 %result
}

define internal ptr @__ncc_win_find_locked(ptr %address, i1 %base_only) {
entry:
  %address.i = ptrtoint ptr %address to i64
  %first = load ptr, ptr @__ncc_map_head, align 8
  br label %loop
loop:
  %node = phi ptr [ %first, %entry ], [ %next, %step ]
  %empty = icmp eq ptr %node, null
  br i1 %empty, label %missing, label %check
check:
  %base.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 0
  %base = load ptr, ptr %base.slot, align 8
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %live.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 4
  %live = load i1, ptr %live.slot, align 1
  %base.i = ptrtoint ptr %base to i64
  %end.i = add i64 %base.i, %len
  %at.base = icmp eq i64 %address.i, %base.i
  %above = icmp uge i64 %address.i, %base.i
  %below = icmp ult i64 %address.i, %end.i
  %inside = and i1 %above, %below
  %address.ok = select i1 %base_only, i1 %at.base, i1 %inside
  %matches = and i1 %live, %address.ok
  br i1 %matches, label %found, label %step
step:
  %next.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 5
  %next = load ptr, ptr %next.slot, align 8
  br label %loop
found:
  ret ptr %node
missing:
  ret ptr null
}

define internal ptr @__ncc_win_register(ptr %base, i64 %len, i64 %allocation_len, i32 %protection, i32 %flags, ptr %mapping_handle, i32 %kind) {
entry:
  %node = call ptr @malloc(i64 64)
  %failed = icmp eq ptr %node, null
  br i1 %failed, label %error, label %store
store:
  %base.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 0
  store ptr %base, ptr %base.slot, align 8
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  store i64 %len, ptr %len.slot, align 8
  %protection.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 2
  store i32 %protection, ptr %protection.slot, align 4
  %flags.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 3
  store i32 %flags, ptr %flags.slot, align 4
  %live.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 4
  store i1 true, ptr %live.slot, align 1
  %handle.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 6
  store ptr %mapping_handle, ptr %handle.slot, align 8
  %kind.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 7
  store i32 %kind, ptr %kind.slot, align 4
  %allocation.len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 8
  store i64 %allocation_len, ptr %allocation.len.slot, align 8
  call void @__ncc_map_registry_lock()
  %head = load ptr, ptr @__ncc_map_head, align 8
  %next.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 5
  store ptr %head, ptr %next.slot, align 8
  store ptr %node, ptr @__ncc_map_head, align 8
  call void @__ncc_map_registry_unlock()
  ret ptr %node
error:
  ret ptr null
}

define internal void @__ncc_win_remove_locked(ptr %target) {
entry:
  %first = load ptr, ptr @__ncc_map_head, align 8
  %is.first = icmp eq ptr %first, %target
  br i1 %is.first, label %remove.head, label %prepare
remove.head:
  %head.next.slot = getelementptr %ncc.winmap.node, ptr %target, i32 0, i32 5
  %head.next = load ptr, ptr %head.next.slot, align 8
  store ptr %head.next, ptr @__ncc_map_head, align 8
  call void @free(ptr %target)
  ret void
prepare:
  %first.empty = icmp eq ptr %first, null
  br i1 %first.empty, label %missing, label %loop.entry
loop.entry:
  %current.slot = getelementptr %ncc.winmap.node, ptr %first, i32 0, i32 5
  %current.first = load ptr, ptr %current.slot, align 8
  br label %loop
loop:
  %previous = phi ptr [ %first, %loop.entry ], [ %current, %step ]
  %current = phi ptr [ %current.first, %loop.entry ], [ %next, %step ]
  %empty = icmp eq ptr %current, null
  br i1 %empty, label %missing, label %check
check:
  %matches = icmp eq ptr %current, %target
  br i1 %matches, label %remove, label %step
step:
  %next.slot = getelementptr %ncc.winmap.node, ptr %current, i32 0, i32 5
  %next = load ptr, ptr %next.slot, align 8
  br label %loop
remove:
  %target.next.slot = getelementptr %ncc.winmap.node, ptr %target, i32 0, i32 5
  %target.next = load ptr, ptr %target.next.slot, align 8
  %previous.next.slot = getelementptr %ncc.winmap.node, ptr %previous, i32 0, i32 5
  store ptr %target.next, ptr %previous.next.slot, align 8
  call void @free(ptr %target)
  ret void
missing:
  ret void
}

define internal ptr @__ncc_win_map_create(ptr %requested, i64 %len, i32 %protection, i32 %flags) {
entry:
  %valid = icmp sgt i64 %len, 0
  br i1 %valid, label %allocate, label %error
allocate:
  %page = call i32 @__ncc_win_page_protection(i32 %protection)
  %allocation_len = call i64 @__ncc_win_reservation_len(ptr %requested, i64 %len, i32 %flags)
  %base = call ptr @VirtualAlloc(ptr %requested, i64 %allocation_len, i32 12288, i32 %page)
  %failed = icmp eq ptr %base, null
  br i1 %failed, label %error, label %fixed.check
fixed.check:
  %fixed.bits = and i32 %flags, 4
  %fixed = icmp ne i32 %fixed.bits, 0
  %same = icmp eq ptr %base, %requested
  %not.fixed = xor i1 %fixed, true
  %fixed.ok = or i1 %same, %not.fixed
  br i1 %fixed.ok, label %register, label %rollback
register:
  %node = call ptr @__ncc_win_register(ptr %base, i64 %len, i64 %allocation_len, i32 %protection, i32 %flags, ptr null, i32 0)
  %registered = icmp ne ptr %node, null
  br i1 %registered, label %ok, label %rollback
rollback:
  %released = call i32 @VirtualFree(ptr %base, i64 0, i32 32768)
  br label %error
ok:
  ret ptr %base
error:
  ret ptr null
}

define internal ptr @__ncc_win_view_create(ptr %file, ptr %requested, i64 %mapping_len, i64 %offset, i64 %len, i32 %protection, i32 %flags) {
entry:
  %valid = icmp sgt i64 %len, 0
  br i1 %valid, label %mapping, label %error
mapping:
  %page = call i32 @__ncc_win_page_protection(i32 %protection)
  %mapping.high64 = lshr i64 %mapping_len, 32
  %mapping.high = trunc i64 %mapping.high64 to i32
  %mapping.low = trunc i64 %mapping_len to i32
  %handle = call ptr @CreateFileMappingA(ptr %file, ptr null, i32 %page, i32 %mapping.high, i32 %mapping.low, ptr null)
  %handle.failed = icmp eq ptr %handle, null
  br i1 %handle.failed, label %error, label %view
view:
  %private.bits = and i32 %flags, 2
  %private = icmp ne i32 %private.bits, 0
  %write.bits = and i32 %protection, 2
  %write = icmp ne i32 %write.bits, 0
  %write.access = select i1 %write, i32 2, i32 4
  %base.access = select i1 %private, i32 1, i32 %write.access
  %exec.bits = and i32 %protection, 4
  %exec = icmp ne i32 %exec.bits, 0
  %exec.access = select i1 %exec, i32 32, i32 0
  %access = or i32 %base.access, %exec.access
  %offset.high64 = lshr i64 %offset, 32
  %offset.high = trunc i64 %offset.high64 to i32
  %offset.low = trunc i64 %offset to i32
  %base = call ptr @MapViewOfFileEx(ptr %handle, i32 %access, i32 %offset.high, i32 %offset.low, i64 %len, ptr %requested)
  %view.failed = icmp eq ptr %base, null
  br i1 %view.failed, label %close.error, label %fixed.check
fixed.check:
  %fixed.bits = and i32 %flags, 4
  %fixed = icmp ne i32 %fixed.bits, 0
  %same = icmp eq ptr %base, %requested
  %not.fixed = xor i1 %fixed, true
  %fixed.ok = or i1 %same, %not.fixed
  br i1 %fixed.ok, label %register, label %unmap.error
register:
  %node = call ptr @__ncc_win_register(ptr %base, i64 %len, i64 %len, i32 %protection, i32 %flags, ptr %handle, i32 1)
  %registered = icmp ne ptr %node, null
  br i1 %registered, label %ok, label %unmap.error
unmap.error:
  %unmapped = call i32 @UnmapViewOfFile(ptr %base)
  br label %close.error
close.error:
  %closed = call i32 @CloseHandle(ptr %handle)
  br label %error
ok:
  ret ptr %base
error:
  ret ptr null
}

define internal ptr @__ncc_win_map_anonymous_common(ptr %requested, i64 %len, i32 %protection, i32 %flags) {
entry:
  %shared.bits = and i32 %flags, 1
  %shared = icmp ne i32 %shared.bits, 0
  br i1 %shared, label %view, label %virtual
view:
  %pagefile = inttoptr i64 -1 to ptr
  %shared.out = call ptr @__ncc_win_view_create(ptr %pagefile, ptr %requested, i64 %len, i64 0, i64 %len, i32 %protection, i32 %flags)
  ret ptr %shared.out
virtual:
  %private.out = call ptr @__ncc_win_map_create(ptr %requested, i64 %len, i32 %protection, i32 %flags)
  ret ptr %private.out
}

define ptr @__ncc_rt_map_default(i64 %len) {
entry:
  %out = call ptr @__ncc_win_map_anonymous_common(ptr null, i64 %len, i32 3, i32 2)
  ret ptr %out
}

define ptr @__ncc_rt_map_anonymous(i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @__ncc_win_map_anonymous_common(ptr null, i64 %len, i32 %protection, i32 %flags)
  ret ptr %out
}

define ptr @__ncc_rt_map_at(i64 %address, i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %requested = inttoptr i64 %address to ptr
  %protection = trunc i64 %protection64 to i32
  %flags.raw = trunc i64 %flags64 to i32
  %flags = or i32 %flags.raw, 4
  %out = call ptr @__ncc_win_map_anonymous_common(ptr %requested, i64 %len, i32 %protection, i32 %flags)
  ret ptr %out
}

define ptr @__ncc_rt_map_shared(i64 %len, i64 %protection64) {
entry:
  %out = call ptr @__ncc_rt_map_anonymous(i64 %len, i64 %protection64, i64 1)
  ret ptr %out
}

define ptr @__ncc_rt_map_private(i64 %len, i64 %protection64) {
entry:
  %out = call ptr @__ncc_rt_map_anonymous(i64 %len, i64 %protection64, i64 2)
  ret ptr %out
}

define internal ptr @__ncc_win_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection64, i64 %flags64) {
entry:
  %valid.offset = icmp sge i64 %offset, 0
  %valid.len = icmp sgt i64 %len, 0
  %valid = and i1 %valid.offset, %valid.len
  br i1 %valid, label %open, label %error
open:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %write.bits = and i32 %protection, 2
  %write = icmp ne i32 %write.bits, 0
  %access = select i1 %write, i32 -1073741824, i32 -2147483648
  %file = call ptr @CreateFileA(ptr %path, i32 %access, i32 3, ptr null, i32 3, i32 128, ptr null)
  %opened = icmp ne ptr %file, inttoptr (i64 -1 to ptr)
  br i1 %opened, label %map, label %error
map:
  %out = call ptr @__ncc_win_view_create(ptr %file, ptr null, i64 0, i64 %offset, i64 %len, i32 %protection, i32 %flags)
  %closed = call i32 @CloseHandle(ptr %file)
  ret ptr %out
error:
  ret ptr null
}

define ptr @__ncc_rt_map_file(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 %flags) {
entry:
  %out = call ptr @__ncc_win_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 %flags)
  ret ptr %out
}

define ptr @__ncc_rt_map_file_shared(ptr %path, i64 %offset, i64 %len, i64 %protection) {
entry:
  %out = call ptr @__ncc_win_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 1)
  ret ptr %out
}

define ptr @__ncc_rt_map_file_private(ptr %path, i64 %offset, i64 %len, i64 %protection) {
entry:
  %out = call ptr @__ncc_win_map_file_common(ptr %path, i64 %offset, i64 %len, i64 %protection, i64 2)
  ret ptr %out
}

define i1 @__ncc_rt_map_unmap(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %kind, label %done
kind:
  %kind.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 7
  %kind.value = load i32, ptr %kind.slot, align 4
  %is.view = icmp eq i32 %kind.value, 1
  br i1 %is.view, label %release.view, label %release.virtual
release.view:
  %view.rc = call i32 @UnmapViewOfFile(ptr %address)
  %view.ok = icmp ne i32 %view.rc, 0
  br i1 %view.ok, label %close.view, label %done
close.view:
  %handle.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 6
  %handle = load ptr, ptr %handle.slot, align 8
  %close.rc = call i32 @CloseHandle(ptr %handle)
  %close.ok = icmp ne i32 %close.rc, 0
  br i1 %close.ok, label %mark, label %done
release.virtual:
  %virtual.rc = call i32 @VirtualFree(ptr %address, i64 0, i32 32768)
  %virtual.ok = icmp ne i32 %virtual.rc, 0
  br i1 %virtual.ok, label %mark, label %done
mark:
  call void @__ncc_win_remove_locked(ptr %node)
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ false, %release.view ], [ false, %close.view ], [ false, %release.virtual ], [ true, %mark ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define internal ptr @__ncc_win_move(ptr %address, i64 %new_len, ptr %requested) {
entry:
  %valid = icmp sgt i64 %new_len, 0
  br i1 %valid, label %lookup, label %invalid
lookup:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %allocate, label %missing
allocate:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %old_len = load i64, ptr %len.slot, align 8
  %kind.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 7
  %kind = load i32, ptr %kind.slot, align 4
  %handle.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 6
  %handle = load ptr, ptr %handle.slot, align 8
  %protection.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 2
  %protection = load i32, ptr %protection.slot, align 4
  %flags.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 3
  %flags = load i32, ptr %flags.slot, align 4
  %page = call i32 @__ncc_win_page_protection(i32 %protection)
  %allocation_len = call i64 @__ncc_win_reservation_len(ptr %requested, i64 %new_len, i32 %flags)
  %out = call ptr @VirtualAlloc(ptr %requested, i64 %allocation_len, i32 12288, i32 %page)
  %failed = icmp eq ptr %out, null
  br i1 %failed, label %failed.unlock, label %copy
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy_len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy_len, i1 false)
  %is.view = icmp eq i32 %kind, 1
  br i1 %is.view, label %release.view, label %release.virtual
release.view:
  %view.rc = call i32 @UnmapViewOfFile(ptr %address)
  %view.ok = icmp ne i32 %view.rc, 0
  br i1 %view.ok, label %close.view, label %rollback
close.view:
  %close.rc = call i32 @CloseHandle(ptr %handle)
  br label %record
release.virtual:
  %virtual.rc = call i32 @VirtualFree(ptr %address, i64 0, i32 32768)
  %virtual.ok = icmp ne i32 %virtual.rc, 0
  br i1 %virtual.ok, label %record, label %rollback
record:
  %base.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 0
  store ptr %out, ptr %base.slot, align 8
  store i64 %new_len, ptr %len.slot, align 8
  %allocation.len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 8
  store i64 %allocation_len, ptr %allocation.len.slot, align 8
  store ptr null, ptr %handle.slot, align 8
  store i32 0, ptr %kind.slot, align 4
  call void @__ncc_map_registry_unlock()
  ret ptr %out
rollback:
  %ignored = call i32 @VirtualFree(ptr %out, i64 0, i32 32768)
  br label %failed.unlock
missing:
  call void @__ncc_map_registry_unlock()
  ret ptr null
failed.unlock:
  call void @__ncc_map_registry_unlock()
  ret ptr null
invalid:
  ret ptr null
}

define ptr @__ncc_rt_map_remap(ptr %address, i64 %new_len, i1 %may_move) {
entry:
  br i1 %may_move, label %move, label %same.check
same.check:
  %old_len = call i64 @__ncc_rt_map_len(ptr %address)
  %same = icmp eq i64 %old_len, %new_len
  %same.result = select i1 %same, ptr %address, ptr null
  ret ptr %same.result
move:
  %out = call ptr @__ncc_win_move(ptr %address, i64 %new_len, ptr null)
  ret ptr %out
}

define ptr @__ncc_rt_map_remap_at(ptr %address, i64 %new_len, i64 %new_address) {
entry:
  %requested = inttoptr i64 %new_address to ptr
  %out = call ptr @__ncc_win_move(ptr %address, i64 %new_len, ptr %requested)
  ret ptr %out
}

define i1 @__ncc_rt_map_protect(ptr %address, i64 %protection64) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %protection = trunc i64 %protection64 to i32
  %page = call i32 @__ncc_win_page_protection(i32 %protection)
  %old = alloca i32, align 4
  %rc = call i32 @VirtualProtect(ptr %address, i64 %len, i32 %page, ptr %old)
  %ok = icmp ne i32 %rc, 0
  br i1 %ok, label %record, label %done
record:
  %protection.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 2
  store i32 %protection, ptr %protection.slot, align 4
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ false, %apply ], [ true, %record ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_sync(ptr %address, i64 %mode) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %kind, label %done
kind:
  %kind.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 7
  %kind.value = load i32, ptr %kind.slot, align 4
  %is.view = icmp eq i32 %kind.value, 1
  br i1 %is.view, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @FlushViewOfFile(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ true, %kind ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_advise(ptr %address, i64 %advice) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %select, label %done
select:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %will.need = icmp eq i64 %advice, 3
  %dont.need = icmp eq i64 %advice, 4
  br i1 %will.need, label %prefetch, label %discard.check
discard.check:
  br i1 %dont.need, label %discard, label %hint.only
prefetch:
  %range = alloca { ptr, i64 }, align 8
  %range.address = getelementptr { ptr, i64 }, ptr %range, i32 0, i32 0
  store ptr %address, ptr %range.address, align 8
  %range.len = getelementptr { ptr, i64 }, ptr %range, i32 0, i32 1
  store i64 %len, ptr %range.len, align 8
  %process = call ptr @GetCurrentProcess()
  %prefetch.rc = call i32 @PrefetchVirtualMemory(ptr %process, i64 1, ptr %range, i32 0)
  %prefetch.ok = icmp ne i32 %prefetch.rc, 0
  br label %done
discard:
  %discard.rc = call i32 @DiscardVirtualMemory(ptr %address, i64 %len)
  %discard.ok = icmp eq i32 %discard.rc, 0
  br label %done
hint.only:
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %prefetch.ok, %prefetch ], [ %discard.ok, %discard ], [ true, %hint.only ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_lock(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @VirtualLock(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i1 @__ncc_rt_map_unlock(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 true)
  %found = icmp ne ptr %node, null
  br i1 %found, label %apply, label %done
apply:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %rc = call i32 @VirtualUnlock(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ false, %entry ], [ %ok, %apply ]
  call void @__ncc_map_registry_unlock()
  ret i1 %result
}

define i64 @__ncc_rt_map_len(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  br i1 %found, label %read, label %done
read:
  %len.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  br label %done
done:
  %result = phi i64 [ -1, %entry ], [ %len, %read ]
  call void @__ncc_map_registry_unlock()
  ret i64 %result
}

define ptr @__ncc_rt_map_address(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  br i1 %found, label %read, label %done
read:
  %base.slot = getelementptr %ncc.winmap.node, ptr %node, i32 0, i32 0
  %base = load ptr, ptr %base.slot, align 8
  br label %done
done:
  %result = phi ptr [ null, %entry ], [ %base, %read ]
  call void @__ncc_map_registry_unlock()
  ret ptr %result
}

define i1 @__ncc_rt_map_is_mapped(ptr %address) {
entry:
  call void @__ncc_map_registry_lock()
  %node = call ptr @__ncc_win_find_locked(ptr %address, i1 false)
  %found = icmp ne ptr %node, null
  call void @__ncc_map_registry_unlock()
  ret i1 %found
}
"#;

const WINDOWS_RAW_MEMORY_RUNTIME: &str = r#"
declare void @GetSystemInfo(ptr)
declare i32 @GetLastError()
declare i32 @FlushInstructionCache(ptr, ptr, i64)
declare ptr @VirtualAllocExNuma(ptr, ptr, i64, i32, i32, i32)
declare ptr @OpenFileMappingA(i32, i32, ptr)

define ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 %alignment) {
entry:
  %size.ok = icmp ugt i64 %bytes, 0
  %align.min = icmp uge i64 %alignment, 8
  %align.minus = sub i64 %alignment, 1
  %align.mask.test = and i64 %alignment, %align.minus
  %align.pow2 = icmp eq i64 %align.mask.test, 0
  %align.ok = and i1 %align.min, %align.pow2
  %valid = and i1 %size.ok, %align.ok
  br i1 %valid, label %allocate, label %error
allocate:
  %padding = add i64 %alignment, 24
  %total = add i64 %bytes, %padding
  %base = call ptr @VirtualAlloc(ptr null, i64 %total, i32 12288, i32 4)
  %failed = icmp eq ptr %base, null
  br i1 %failed, label %error, label %align
align:
  %base.int = ptrtoint ptr %base to i64
  %payload.start = add i64 %base.int, 24
  %rounded = add i64 %payload.start, %align.minus
  %negative.alignment = sub i64 0, %alignment
  %aligned.int = and i64 %rounded, %negative.alignment
  %out = inttoptr i64 %aligned.int to ptr
  %base.slot = getelementptr i8, ptr %out, i64 -24
  %total.slot = getelementptr i8, ptr %out, i64 -16
  %size.slot = getelementptr i8, ptr %out, i64 -8
  store ptr %base, ptr %base.slot, align 8
  store i64 %total, ptr %total.slot, align 8
  store i64 %bytes, ptr %size.slot, align 8
  ret ptr %out
error:
  ret ptr null
}

define ptr @__ncc_raw_aligned_realloc(ptr %address, i64 %old_len, i64 %new_len, i64 %alignment) {
entry:
  %release = icmp eq i64 %new_len, 0
  br i1 %release, label %free.old, label %allocate
free.old:
  %released = call i1 @__ncc_raw_aligned_free(ptr %address)
  ret ptr null
allocate:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %new_len, i64 %alignment)
  %failed = icmp eq ptr %out, null
  br i1 %failed, label %error, label %copy.check
copy.check:
  %has.old = icmp ne ptr %address, null
  br i1 %has.old, label %copy, label %done
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy.len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy.len, i1 false)
  %old.released = call i1 @__ncc_raw_aligned_free(ptr %address)
  br label %done
done:
  ret ptr %out
error:
  ret ptr null
}

define i1 @__ncc_raw_aligned_free(ptr %address) {
entry:
  %is.null = icmp eq ptr %address, null
  br i1 %is.null, label %done, label %release
release:
  %base.slot = getelementptr i8, ptr %address, i64 -24
  %base = load ptr, ptr %base.slot, align 8
  %rc = call i32 @VirtualFree(ptr %base, i64 0, i32 32768)
  %ok = icmp ne i32 %rc, 0
  br label %done
done:
  %result = phi i1 [ true, %entry ], [ %ok, %release ]
  ret i1 %result
}

define i64 @__ncc_raw_vm_page_size() {
entry:
  %info = alloca [64 x i8], align 8
  call void @GetSystemInfo(ptr %info)
  %slot = getelementptr i8, ptr %info, i64 4
  %size32 = load i32, ptr %slot, align 4
  %size = zext i32 %size32 to i64
  ret i64 %size
}

define i64 @__ncc_raw_vm_allocation_granularity() {
entry:
  %info = alloca [64 x i8], align 8
  call void @GetSystemInfo(ptr %info)
  %slot = getelementptr i8, ptr %info, i64 40
  %size32 = load i32, ptr %slot, align 4
  %size = zext i32 %size32 to i64
  ret i64 %size
}

define ptr @__ncc_raw_vm_reserve(ptr %requested, i64 %len, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %native = or i32 %flags, 8192
  %out = call ptr @VirtualAlloc(ptr %requested, i64 %len, i32 %native, i32 1)
  ret ptr %out
}

define i1 @__ncc_raw_vm_commit(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %out = call ptr @VirtualAlloc(ptr %address, i64 %len, i32 4096, i32 %protection)
  %ok = icmp ne ptr %out, null
  ret i1 %ok
}

define i1 @__ncc_raw_vm_decommit(ptr %address, i64 %len) {
entry:
  %rc = call i32 @VirtualFree(ptr %address, i64 %len, i32 16384)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_release(ptr %address, i64 %len) {
entry:
  %rc = call i32 @VirtualFree(ptr %address, i64 0, i32 32768)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_protect(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %old = alloca i32, align 4
  %rc = call i32 @VirtualProtect(ptr %address, i64 %len, i32 %protection, ptr %old)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_guard(ptr %address, i64 %len, i64 %protection64) {
entry:
  %protection = trunc i64 %protection64 to i32
  %guarded = or i32 %protection, 256
  %out = call ptr @VirtualAlloc(ptr %address, i64 %len, i32 4096, i32 %guarded)
  %ok = icmp ne ptr %out, null
  ret i1 %ok
}

define ptr @__ncc_raw_vm_map(ptr %requested, i64 %len, i64 %protection64, i64 %flags64, i64 %handle64, i64 %offset) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @VirtualAlloc(ptr %requested, i64 %len, i32 %flags, i32 %protection)
  ret ptr %out
}

define i1 @__ncc_raw_vm_unmap(ptr %address, i64 %len) {
entry:
  %view = call i32 @UnmapViewOfFile(ptr %address)
  %view.ok = icmp ne i32 %view, 0
  br i1 %view.ok, label %done, label %allocation
allocation:
  %rc = call i32 @VirtualFree(ptr %address, i64 0, i32 32768)
  %allocation.ok = icmp ne i32 %rc, 0
  br label %done
done:
  %ok = phi i1 [ true, %entry ], [ %allocation.ok, %allocation ]
  ret i1 %ok
}

define ptr @__ncc_raw_vm_remap(ptr %address, i64 %old_len, i64 %new_len, i64 %protection64, i64 %flags64, ptr %requested) {
entry:
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @VirtualAlloc(ptr %requested, i64 %new_len, i32 %flags, i32 %protection)
  %failed = icmp eq ptr %out, null
  br i1 %failed, label %error, label %copy
copy:
  %old.smaller = icmp ult i64 %old_len, %new_len
  %copy.len = select i1 %old.smaller, i64 %old_len, i64 %new_len
  call void @llvm.memcpy.p0.p0.i64(ptr %out, ptr %address, i64 %copy.len, i1 false)
  %released = call i32 @VirtualFree(ptr %address, i64 0, i32 32768)
  %ok = icmp ne i32 %released, 0
  br i1 %ok, label %done, label %rollback
rollback:
  %ignored = call i32 @VirtualFree(ptr %out, i64 0, i32 32768)
  br label %error
done:
  ret ptr %out
error:
  ret ptr null
}

define i1 @__ncc_raw_vm_advise(ptr %address, i64 %len, i64 %advice) {
entry:
  %discard = icmp eq i64 %advice, 4
  br i1 %discard, label %apply, label %accepted
apply:
  %rc = call i32 @DiscardVirtualMemory(ptr %address, i64 %len)
  %ok = icmp eq i32 %rc, 0
  ret i1 %ok
accepted:
  ret i1 true
}

define i1 @__ncc_raw_vm_sync(ptr %address, i64 %len, i64 %flags) {
entry:
  %rc = call i32 @FlushViewOfFile(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_lock(ptr %address, i64 %len) {
entry:
  %rc = call i32 @VirtualLock(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_unlock(ptr %address, i64 %len) {
entry:
  %rc = call i32 @VirtualUnlock(ptr %address, i64 %len)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_executable(ptr %address, i64 %len) {
entry:
  %old = alloca i32, align 4
  %protected = call i32 @VirtualProtect(ptr %address, i64 %len, i32 32, ptr %old)
  %protect.ok = icmp ne i32 %protected, 0
  br i1 %protect.ok, label %flush, label %done
flush:
  %process = call ptr @GetCurrentProcess()
  %flushed = call i32 @FlushInstructionCache(ptr %process, ptr %address, i64 %len)
  %flush.ok = icmp ne i32 %flushed, 0
  br label %done
done:
  %ok = phi i1 [ false, %entry ], [ %flush.ok, %flush ]
  ret i1 %ok
}

define ptr @__ncc_raw_vm_numa_alloc(i64 %len, i64 %node64, i64 %protection64, i64 %flags64) {
entry:
  %process = call ptr @GetCurrentProcess()
  %node = trunc i64 %node64 to i32
  %protection = trunc i64 %protection64 to i32
  %flags = trunc i64 %flags64 to i32
  %out = call ptr @VirtualAllocExNuma(ptr %process, ptr null, i64 %len, i32 %flags, i32 %protection, i32 %node)
  ret ptr %out
}

define i1 @__ncc_raw_vm_numa_bind(ptr %address, i64 %len, i64 %mode, ptr %nodemask, i64 %maxnode, i64 %flags) {
entry:
  ret i1 false
}

define i64 @__ncc_raw_vm_last_error() {
entry:
  %value = call i32 @GetLastError()
  %wide = zext i32 %value to i64
  ret i64 %wide
}

define i64 @__ncc_raw_vm_shared_create(ptr %name, i64 %len, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %high = lshr i64 %len, 32
  %high32 = trunc i64 %high to i32
  %low32 = trunc i64 %len to i32
  %invalid = inttoptr i64 -1 to ptr
  %handle = call ptr @CreateFileMappingA(ptr %invalid, ptr null, i32 %flags, i32 %high32, i32 %low32, ptr %name)
  %value = ptrtoint ptr %handle to i64
  ret i64 %value
}

define i64 @__ncc_raw_vm_shared_open(ptr %name, i64 %flags64) {
entry:
  %flags = trunc i64 %flags64 to i32
  %handle = call ptr @OpenFileMappingA(i32 %flags, i32 0, ptr %name)
  %value = ptrtoint ptr %handle to i64
  ret i64 %value
}

define ptr @__ncc_raw_vm_shared_map(i64 %handle64, ptr %requested, i64 %len, i64 %protection64, i64 %flags64, i64 %offset) {
entry:
  %handle = inttoptr i64 %handle64 to ptr
  %access = trunc i64 %protection64 to i32
  %high = lshr i64 %offset, 32
  %high32 = trunc i64 %high to i32
  %low32 = trunc i64 %offset to i32
  %out = call ptr @MapViewOfFileEx(ptr %handle, i32 %access, i32 %high32, i32 %low32, i64 %len, ptr %requested)
  ret ptr %out
}

define i1 @__ncc_raw_vm_shared_close(i64 %handle64) {
entry:
  %handle = inttoptr i64 %handle64 to ptr
  %rc = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %rc, 0
  ret i1 %ok
}

define i1 @__ncc_raw_vm_shared_unlink(ptr %name) {
entry:
  ret i1 true
}
"#;

const WINDOWS_RAW_COMMON_DECLARATIONS: &str = r#"
declare ptr @VirtualAlloc(ptr, i64, i32, i32)
declare i32 @VirtualFree(ptr, i64, i32)
declare i32 @VirtualProtect(ptr, i64, i32, ptr)
declare i32 @VirtualLock(ptr, i64)
declare i32 @VirtualUnlock(ptr, i64)
declare i32 @FlushViewOfFile(ptr, i64)
declare i32 @DiscardVirtualMemory(ptr, i64)
declare ptr @GetCurrentProcess()
declare ptr @CreateFileMappingA(ptr, ptr, i32, i32, i32, ptr)
declare ptr @MapViewOfFileEx(ptr, i32, i32, i32, i64, ptr)
declare i32 @UnmapViewOfFile(ptr)
declare i32 @CloseHandle(ptr)
"#;

const WINDOWS_I386_SYSTEM_API_WRAPPERS: &str = r#"
declare x86_stdcallcc ptr @"\01_VirtualAlloc@16"(ptr, i32, i32, i32)
declare x86_stdcallcc i32 @"\01_VirtualFree@12"(ptr, i32, i32)
declare x86_stdcallcc i32 @"\01_VirtualProtect@16"(ptr, i32, i32, ptr)
declare x86_stdcallcc i32 @"\01_VirtualLock@8"(ptr, i32)
declare x86_stdcallcc i32 @"\01_VirtualUnlock@8"(ptr, i32)
declare x86_stdcallcc i32 @"\01_FlushViewOfFile@8"(ptr, i32)
declare x86_stdcallcc i32 @"\01_DiscardVirtualMemory@8"(ptr, i32)
declare x86_stdcallcc i32 @"\01_PrefetchVirtualMemory@16"(ptr, i32, ptr, i32)
declare x86_stdcallcc ptr @"\01_GetCurrentProcess@0"()
declare x86_stdcallcc ptr @"\01_CreateFileA@28"(ptr, i32, i32, ptr, i32, i32, ptr)
declare x86_stdcallcc ptr @"\01_CreateFileMappingA@24"(ptr, ptr, i32, i32, i32, ptr)
declare x86_stdcallcc ptr @"\01_MapViewOfFileEx@24"(ptr, i32, i32, i32, i32, ptr)
declare x86_stdcallcc i32 @"\01_UnmapViewOfFile@4"(ptr)
declare x86_stdcallcc i32 @"\01_CloseHandle@4"(ptr)
declare x86_stdcallcc void @"\01_GetSystemInfo@4"(ptr)
declare x86_stdcallcc i32 @"\01_GetLastError@0"()
declare x86_stdcallcc i32 @"\01_FlushInstructionCache@12"(ptr, ptr, i32)
declare x86_stdcallcc ptr @"\01_VirtualAllocExNuma@24"(ptr, ptr, i32, i32, i32, i32)
declare x86_stdcallcc ptr @"\01_OpenFileMappingA@12"(i32, i32, ptr)

define internal ptr @__ncc_winapi_VirtualAlloc(ptr %address, i64 %len, i32 %flags, i32 %protection) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc ptr @"\01_VirtualAlloc@16"(ptr %address, i32 %len32, i32 %flags, i32 %protection)
  ret ptr %out
}

define internal i32 @__ncc_winapi_VirtualFree(ptr %address, i64 %len, i32 %flags) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_VirtualFree@12"(ptr %address, i32 %len32, i32 %flags)
  ret i32 %out
}

define internal i32 @__ncc_winapi_VirtualProtect(ptr %address, i64 %len, i32 %protection, ptr %old) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_VirtualProtect@16"(ptr %address, i32 %len32, i32 %protection, ptr %old)
  ret i32 %out
}

define internal i32 @__ncc_winapi_VirtualLock(ptr %address, i64 %len) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_VirtualLock@8"(ptr %address, i32 %len32)
  ret i32 %out
}

define internal i32 @__ncc_winapi_VirtualUnlock(ptr %address, i64 %len) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_VirtualUnlock@8"(ptr %address, i32 %len32)
  ret i32 %out
}

define internal i32 @__ncc_winapi_FlushViewOfFile(ptr %address, i64 %len) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_FlushViewOfFile@8"(ptr %address, i32 %len32)
  ret i32 %out
}

define internal i32 @__ncc_winapi_DiscardVirtualMemory(ptr %address, i64 %len) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_DiscardVirtualMemory@8"(ptr %address, i32 %len32)
  ret i32 %out
}

define internal i32 @__ncc_winapi_PrefetchVirtualMemory(ptr %process, i64 %count, ptr %entries, i32 %flags) {
entry:
  %count32 = trunc i64 %count to i32
  %out = call x86_stdcallcc i32 @"\01_PrefetchVirtualMemory@16"(ptr %process, i32 %count32, ptr %entries, i32 %flags)
  ret i32 %out
}

define internal ptr @__ncc_winapi_GetCurrentProcess() {
entry:
  %out = call x86_stdcallcc ptr @"\01_GetCurrentProcess@0"()
  ret ptr %out
}

define internal ptr @__ncc_winapi_CreateFileA(ptr %name, i32 %access, i32 %share, ptr %security, i32 %creation, i32 %flags, ptr %template) {
entry:
  %out = call x86_stdcallcc ptr @"\01_CreateFileA@28"(ptr %name, i32 %access, i32 %share, ptr %security, i32 %creation, i32 %flags, ptr %template)
  ret ptr %out
}

define internal ptr @__ncc_winapi_CreateFileMappingA(ptr %file, ptr %security, i32 %protection, i32 %high, i32 %low, ptr %name) {
entry:
  %out = call x86_stdcallcc ptr @"\01_CreateFileMappingA@24"(ptr %file, ptr %security, i32 %protection, i32 %high, i32 %low, ptr %name)
  ret ptr %out
}

define internal ptr @__ncc_winapi_MapViewOfFileEx(ptr %mapping, i32 %access, i32 %high, i32 %low, i64 %len, ptr %requested) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc ptr @"\01_MapViewOfFileEx@24"(ptr %mapping, i32 %access, i32 %high, i32 %low, i32 %len32, ptr %requested)
  ret ptr %out
}

define internal i32 @__ncc_winapi_UnmapViewOfFile(ptr %address) {
entry:
  %out = call x86_stdcallcc i32 @"\01_UnmapViewOfFile@4"(ptr %address)
  ret i32 %out
}

define internal i32 @__ncc_winapi_CloseHandle(ptr %handle) {
entry:
  %out = call x86_stdcallcc i32 @"\01_CloseHandle@4"(ptr %handle)
  ret i32 %out
}

define internal void @__ncc_winapi_GetSystemInfo(ptr %info) {
entry:
  call x86_stdcallcc void @"\01_GetSystemInfo@4"(ptr %info)
  ret void
}

define internal i32 @__ncc_winapi_GetLastError() {
entry:
  %out = call x86_stdcallcc i32 @"\01_GetLastError@0"()
  ret i32 %out
}

define internal i32 @__ncc_winapi_FlushInstructionCache(ptr %process, ptr %address, i64 %len) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc i32 @"\01_FlushInstructionCache@12"(ptr %process, ptr %address, i32 %len32)
  ret i32 %out
}

define internal ptr @__ncc_winapi_VirtualAllocExNuma(ptr %process, ptr %address, i64 %len, i32 %flags, i32 %protection, i32 %node) {
entry:
  %len32 = trunc i64 %len to i32
  %out = call x86_stdcallcc ptr @"\01_VirtualAllocExNuma@24"(ptr %process, ptr %address, i32 %len32, i32 %flags, i32 %protection, i32 %node)
  ret ptr %out
}

define internal ptr @__ncc_winapi_OpenFileMappingA(i32 %access, i32 %inherit, ptr %name) {
entry:
  %out = call x86_stdcallcc ptr @"\01_OpenFileMappingA@12"(i32 %access, i32 %inherit, ptr %name)
  ret ptr %out
}
"#;
