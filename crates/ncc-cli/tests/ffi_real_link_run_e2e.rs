#[cfg(unix)]
mod ffi_real_link_run_e2e {
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn ncc_bin() -> PathBuf {
        std::env::var_os("CARGO_BIN_EXE_ncc")
            .map(PathBuf::from)
            .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root must resolve")
    }

    fn ffi_fixture(name: &str) -> PathBuf {
        let root = workspace_root();
        for base in ["examples/ffi", "_archive/old_examples/ffi"] {
            let candidate = root.join(base).join(name);
            if candidate.exists() {
                return candidate;
            }
        }
        root.join("examples/ffi").join(name)
    }

    fn normalize(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).replace("\r\n", "\n")
    }

    fn command_available(bin: &str) -> bool {
        Command::new(bin)
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn run_ncc(args: &[&str]) -> Output {
        Command::new(ncc_bin())
            .args(args)
            .env("NC_NO_COLOR", "1")
            .output()
            .expect("ncc command must execute")
    }

    fn run_tool(bin: &str, args: &[&str], cwd: &Path) -> Output {
        Command::new(bin)
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("tool command must execute")
    }

    fn sha256_hex_file(path: &Path) -> String {
        let bytes = fs::read(path).expect("artifact bytes must be readable");
        format!("{:x}", Sha256::digest(&bytes))
    }

    #[test]
    fn ffi_real_link_run_user_program_end_to_end() {
        if !command_available("cc")
            || !command_available("c++")
            || !command_available("rustc")
            || !command_available("nm")
        {
            return;
        }

        let fixture = ffi_fixture("ffi_real_link_run_user_program.n");
        assert!(
            fixture.exists(),
            "fixture must exist: {}",
            fixture.display()
        );

        let check = run_ncc(&["check", fixture.to_str().expect("utf8 path")]);
        assert!(
            check.status.success(),
            "ncc check must pass\nstdout={}\nstderr={}",
            normalize(&check.stdout),
            normalize(&check.stderr)
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for temp names")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_real_link_run_{}_{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&tmp).expect("temp workspace must be created");

        let c_src = tmp.join("ffi_c_add.c");
        let cpp_src = tmp.join("ffi_cpp_mul.cpp");
        let rust_src = tmp.join("ffi_rust_sub.rs");
        let c_obj = tmp.join("ffi_c_add.o");
        let cpp_obj = tmp.join("ffi_cpp_mul.o");
        let rust_obj = tmp.join("ffi_rust_sub.o");
        let driver_src = tmp.join("ffi_driver.c");
        let wrapper = tmp.join("ffi_linker_wrapper.sh");
        let out_exe = tmp.join("ffi_real_exec");

        fs::write(&c_src, "int nc_ffi_c_add(int a, int b) { return a + b; }\n")
            .expect("c source must write");
        fs::write(
            &cpp_src,
            "extern \"C\" int nc_ffi_cpp_mul(int a, int b) { return a * b; }\n",
        )
        .expect("c++ source must write");
        fs::write(
            &rust_src,
            "#[no_mangle]\npub extern \"C\" fn nc_ffi_rust_sub(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .expect("rust source must write");
        fs::write(
            &driver_src,
            "#include <dlfcn.h>\ntypedef int (*ncc_entry_fn)(void);\nint main(void) {\n  void *sym = dlsym(RTLD_DEFAULT, \"ffiRealLinkRunUserProgram.run\");\n  if (!sym) return 90;\n  ncc_entry_fn entry = (ncc_entry_fn)sym;\n  return entry() ? 0 : 91;\n}\n",
        )
        .expect("driver source must write");

        let c_status = run_tool(
            "cc",
            &[
                "-c",
                c_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                c_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            c_status.status.success(),
            "c compile failed\nstdout={}\nstderr={}",
            normalize(&c_status.stdout),
            normalize(&c_status.stderr)
        );

        let cpp_status = run_tool(
            "c++",
            &[
                "-c",
                cpp_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                cpp_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            cpp_status.status.success(),
            "c++ compile failed\nstdout={}\nstderr={}",
            normalize(&cpp_status.stdout),
            normalize(&cpp_status.stderr)
        );

        let rust_status = run_tool(
            "rustc",
            &[
                "--crate-type",
                "lib",
                "--emit=obj",
                rust_src.to_str().expect("utf8 path"),
                "-O",
                "-o",
                rust_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            rust_status.status.success(),
            "rust compile failed\nstdout={}\nstderr={}",
            normalize(&rust_status.stdout),
            normalize(&rust_status.stderr)
        );

        let wrapper_source = format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nout=\"\"\nobj=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    -o)\n      shift\n      out=\"$1\"\n      ;;\n    *.o)\n      obj=\"$1\"\n      ;;\n  esac\n  shift || true\ndone\nif [ -z \"$out\" ] || [ -z \"$obj\" ]; then\n  exit 64\nfi\ncc {} \"$obj\" {} {} {} -lstdc++ -ldl -rdynamic -o \"$out\"\n",
            driver_src.display(),
            c_obj.display(),
            cpp_obj.display(),
            rust_obj.display()
        );
        fs::write(&wrapper, wrapper_source).expect("wrapper source must write");
        let mut perms = fs::metadata(&wrapper)
            .expect("wrapper metadata must exist")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper, perms).expect("wrapper must be executable");

        let build = run_ncc(&[
            "build-exe",
            fixture.to_str().expect("utf8 path"),
            out_exe.to_str().expect("utf8 path"),
            "--linker",
            wrapper.to_str().expect("utf8 path"),
            "--entry",
            "ffiRealLinkRunUserProgram.run",
        ]);
        assert!(
            build.status.success(),
            "ncc build-exe must pass\nstdout={}\nstderr={}",
            normalize(&build.stdout),
            normalize(&build.stderr)
        );

        let exec = Command::new(&out_exe)
            .current_dir(&tmp)
            .output()
            .expect("linked executable must run");
        assert!(
            exec.status.success(),
            "linked executable must exit 0\nstdout={}\nstderr={}\nstatus={}",
            normalize(&exec.stdout),
            normalize(&exec.stderr),
            exec.status
        );

        let native_run = run_ncc(&[
            "run",
            fixture.to_str().expect("utf8 path"),
            "--native-ffi",
            "--linker",
            wrapper.to_str().expect("utf8 path"),
            "--entry",
            "ffiRealLinkRunUserProgram.run",
        ]);
        assert!(
            native_run.status.success(),
            "ncc run --native-ffi must pass\nstdout={}\nstderr={}",
            normalize(&native_run.stdout),
            normalize(&native_run.stderr)
        );
    }

    #[test]
    fn ffi_real_link_run_native_autolink_without_wrapper() {
        if !command_available("cc") || !command_available("c++") || !command_available("rustc") {
            return;
        }

        let fixture = ffi_fixture("ffi_real_link_run_user_program.n");
        assert!(
            fixture.exists(),
            "fixture must exist: {}",
            fixture.display()
        );
        for sidecar in ["ffi_c_add.c", "ffi_cpp_mul.cpp", "ffi_rust_sub.rs"] {
            let p = fixture
                .parent()
                .expect("FFI fixture must have a parent")
                .join(sidecar);
            assert!(
                p.exists(),
                "sidecar must exist for native autolink: {}",
                p.display()
            );
        }

        let run = run_ncc(&[
            "run",
            fixture.to_str().expect("utf8 path"),
            "--native-ffi",
            "--entry",
            "ffiRealLinkRunUserProgram.run",
        ]);
        assert!(
            run.status.success(),
            "ncc run --native-ffi autolink must pass\nstdout={}\nstderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
    }

    #[test]
    fn ffi_real_link_run_real_alias_autolinks_foreign_sidecars() {
        if !command_available("cc") || !command_available("c++") || !command_available("rustc") {
            return;
        }

        let fixture = ffi_fixture("ffi_real_link_run_user_program.n");
        let run = run_ncc(&[
            "run",
            fixture.to_str().expect("utf8 path"),
            "--real",
            "--entry",
            "ffiRealLinkRunUserProgram.run",
        ]);
        assert!(
            run.status.success(),
            "ncc run --real must autolink foreign sidecars\nstdout={}\nstderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
    }

    #[test]
    fn ffi_cpp_abi_real_toolchain_smoke() {
        if !command_available("c++")
            || !command_available("nm")
            || !command_available("c++filt")
            || !command_available("ar")
        {
            return;
        }

        let run = run_ncc(&["compiler", "ffi-cpp-abi", "--json", "--strict"]);
        assert!(
            run.status.success(),
            "ncc compiler ffi-cpp-abi --strict must pass\nstdout={}\nstderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
        let payload: serde_json::Value =
            serde_json::from_slice(&run.stdout).expect("ffi-cpp-abi must emit json");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["cpp_abi"]["status"], "closed_real_toolchain_smoke");
        assert_eq!(payload["name_mangling"]["status"], "closed_real");
        assert_eq!(payload["vtables"]["status"], "closed_real");
        assert_eq!(payload["destructors"]["status"], "closed_real");
        assert_eq!(payload["copy_move"]["status"], "closed_real");
        assert_eq!(payload["templates"]["status"], "closed_real");
        assert!(payload["destructors"]["raii_scope"]
            .as_str()
            .is_some_and(|value| value.contains("constructor+destructor")));
        assert_eq!(
            payload["copy_move"]["copy_constructor"],
            "nc_cpp_copy_constructor_count returned 2011"
        );
        assert_eq!(
            payload["copy_move"]["move_constructor"],
            "nc_cpp_move_constructor_count returned 1101"
        );
        assert_eq!(
            payload["exceptions"]["uncaught_crossing_rejected"],
            serde_json::json!(true)
        );
        assert!(payload["artifacts"]
            .as_array()
            .is_some_and(|items| items.len() >= 4));
    }

    #[test]
    fn ffi_external_bindings_matrix_reports_real_and_optional_lanes() {
        if !command_available("cc") || !command_available("rustc") || !command_available("nm") {
            return;
        }

        let run = run_ncc(&["compiler", "ffi-external-bindings", "--json"]);
        assert!(
            run.status.success(),
            "ncc compiler ffi-external-bindings must pass\nstdout={}\nstderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
        let payload: serde_json::Value =
            serde_json::from_slice(&run.stdout).expect("ffi-external-bindings must emit json");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["required_status"], "closed");
        assert_eq!(payload["lanes"]["c"]["status"], "executed");
        assert_eq!(payload["lanes"]["rust"]["status"], "executed");
        assert_eq!(payload["lanes"]["asm"]["status"], "executed");
        assert_eq!(payload["lanes"]["cpp"]["status"], "ok");
        assert!(payload["lanes"]["zig"]["status"]
            .as_str()
            .is_some_and(|status| status == "executed" || status == "skipped_missing_tool"));
        assert!(payload["lanes"]["go"]["status"]
            .as_str()
            .is_some_and(|status| status == "executed" || status == "skipped_missing_tool"));
        assert_eq!(
            payload["policy"]["no_fake_evidence"],
            serde_json::json!(true)
        );
        assert!(payload["artifacts"]
            .as_array()
            .is_some_and(|items| items.len() >= 4));
    }

    #[test]
    fn ffi_pointer_c_asm_smoke_writes_and_reads_pointer() {
        if !command_available("cc") {
            return;
        }

        let fixture = ffi_fixture("ffi_pointer_c_asm_contract.n");
        assert!(
            fixture.exists(),
            "fixture must exist: {}",
            fixture.display()
        );

        let check = run_ncc(&["check", fixture.to_str().expect("utf8 path")]);
        assert!(
            check.status.success(),
            "ncc check must pass\nstdout={}\nstderr={}",
            normalize(&check.stdout),
            normalize(&check.stderr)
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for temp names")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_ptr_c_asm_{}_{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&tmp).expect("temp workspace must be created");

        let c_src = tmp.join("nc_ptr_c_read_u64.c");
        let asm_src = tmp.join("nc_ptr_asm_write_u64.S");
        let driver_src = tmp.join("ptr_driver.c");
        let c_obj = tmp.join("nc_ptr_c_read_u64.o");
        let asm_obj = tmp.join("nc_ptr_asm_write_u64.o");
        let out_exe = tmp.join("ptr_c_asm_smoke");

        fs::write(
            &c_src,
            "#include <stdint.h>\nuint64_t nc_ptr_c_read_u64(const uint64_t *p) { return *p; }\n",
        )
        .expect("c source must write");
        fs::write(
            &asm_src,
            ".text\n.global nc_ptr_asm_write_u64\n.type nc_ptr_asm_write_u64,@function\nnc_ptr_asm_write_u64:\n  movq %rsi, (%rdi)\n  ret\n",
        )
        .expect("asm source must write");
        fs::write(
            &driver_src,
            "#include <stdint.h>\nextern uint64_t nc_ptr_c_read_u64(const uint64_t *p);\nextern void nc_ptr_asm_write_u64(uint64_t *p, uint64_t v);\nint main(void) { uint64_t value = 0; nc_ptr_asm_write_u64(&value, 0x1122334455667788ULL); return nc_ptr_c_read_u64(&value) == 0x1122334455667788ULL ? 0 : 97; }\n",
        )
        .expect("driver source must write");

        let c_status = run_tool(
            "cc",
            &[
                "-c",
                c_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                c_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            c_status.status.success(),
            "c compile failed\nstdout={}\nstderr={}",
            normalize(&c_status.stdout),
            normalize(&c_status.stderr)
        );

        let asm_status = run_tool(
            "cc",
            &[
                "-c",
                asm_src.to_str().expect("utf8 path"),
                "-o",
                asm_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            asm_status.status.success(),
            "asm compile failed\nstdout={}\nstderr={}",
            normalize(&asm_status.stdout),
            normalize(&asm_status.stderr)
        );

        let link_status = run_tool(
            "cc",
            &[
                driver_src.to_str().expect("utf8 path"),
                c_obj.to_str().expect("utf8 path"),
                asm_obj.to_str().expect("utf8 path"),
                "-o",
                out_exe.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            link_status.status.success(),
            "link failed\nstdout={}\nstderr={}",
            normalize(&link_status.stdout),
            normalize(&link_status.stderr)
        );

        let exec = Command::new(&out_exe)
            .current_dir(&tmp)
            .output()
            .expect("linked C/ASM pointer executable must run");
        assert!(
            exec.status.success(),
            "C/ASM pointer smoke must exit 0\nstdout={}\nstderr={}\nstatus={}",
            normalize(&exec.stdout),
            normalize(&exec.stderr),
            exec.status
        );
    }

    #[test]
    fn ffi_c_abi_header_static_shared_checksum_smoke() {
        if !command_available("cc") || !command_available("ar") || !command_available("nm") {
            return;
        }

        let positive = ffi_fixture("c_abi_industrial_static_shared.n");
        assert!(
            positive.exists(),
            "fixture must exist: {}",
            positive.display()
        );
        let check = run_ncc(&["check", positive.to_str().expect("utf8 path")]);
        assert!(
            check.status.success(),
            "industrial C ABI fixture must pass\nstdout={}\nstderr={}",
            normalize(&check.stdout),
            normalize(&check.stderr)
        );

        let missing_header = ffi_fixture("c_abi_missing_header_fail.n");
        assert!(
            missing_header.exists(),
            "negative fixture must exist: {}",
            missing_header.display()
        );
        let missing = run_ncc(&["check", missing_header.to_str().expect("utf8 path")]);
        assert!(
            !missing.status.success(),
            "missing header fixture must fail\nstdout={}\nstderr={}",
            normalize(&missing.stdout),
            normalize(&missing.stderr)
        );
        let missing_out = format!(
            "{}{}",
            normalize(&missing.stdout),
            normalize(&missing.stderr)
        );
        assert!(
            missing_out.contains("E_FFI_IMPORT_CONTRACT_001"),
            "missing header must report official diagnostic\n{}",
            missing_out
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for temp names")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_c_abi_industrial_{}_{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&tmp).expect("temp workspace must be created");

        let header = tmp.join("nc_c_abi_industrial.h");
        let c_src = tmp.join("nc_c_abi_industrial.c");
        let obj = tmp.join("nc_c_abi_industrial.o");
        let static_archive = tmp.join("libnc_c_abi_industrial.a");
        let shared_library = tmp.join("libnc_c_abi_industrial.so");
        let driver_src = tmp.join("nc_c_abi_driver.c");
        let static_exe = tmp.join("nc_c_abi_static_driver");
        let shared_exe = tmp.join("nc_c_abi_shared_driver");
        let manifest = tmp.join("c_abi_manifest.json");

        fs::write(
            &header,
            "#ifndef NC_C_ABI_INDUSTRIAL_H\n\
#define NC_C_ABI_INDUSTRIAL_H\n\
#include <stdint.h>\n\
#include <stddef.h>\n\
typedef struct NcCAbiPacket {\n\
    int32_t value;\n\
    uint64_t tag;\n\
} NcCAbiPacket;\n\
int32_t nc_c_abi_add(int32_t a, int32_t b);\n\
void nc_c_abi_packet_init(NcCAbiPacket *packet, int32_t value, uint64_t tag);\n\
int32_t nc_c_abi_packet_value(const NcCAbiPacket *packet);\n\
void *nc_c_abi_alloc_bytes(size_t len);\n\
void nc_c_abi_free_bytes(void *ptr);\n\
#endif\n",
        )
        .expect("header must write");
        fs::write(
            &c_src,
            "#include \"nc_c_abi_industrial.h\"\n\
#include <stdlib.h>\n\
int32_t nc_c_abi_add(int32_t a, int32_t b) { return a + b; }\n\
void nc_c_abi_packet_init(NcCAbiPacket *packet, int32_t value, uint64_t tag) {\n\
    packet->value = value;\n\
    packet->tag = tag;\n\
}\n\
int32_t nc_c_abi_packet_value(const NcCAbiPacket *packet) {\n\
    return packet->value + (int32_t)(packet->tag & 0x7);\n\
}\n\
void *nc_c_abi_alloc_bytes(size_t len) { return malloc(len); }\n\
void nc_c_abi_free_bytes(void *ptr) { free(ptr); }\n",
        )
        .expect("c source must write");
        fs::write(
            &driver_src,
            "#include \"nc_c_abi_industrial.h\"\n\
#include <stdint.h>\n\
int main(void) {\n\
    NcCAbiPacket packet;\n\
    nc_c_abi_packet_init(&packet, 35, 6);\n\
    if (nc_c_abi_add(20, 22) != 42) return 10;\n\
    if (nc_c_abi_packet_value(&packet) != 41) return 11;\n\
    void *buffer = nc_c_abi_alloc_bytes(32);\n\
    if (!buffer) return 12;\n\
    nc_c_abi_free_bytes(buffer);\n\
    return 0;\n\
}\n",
        )
        .expect("driver source must write");

        let compile = run_tool(
            "cc",
            &[
                "-fPIC",
                "-I",
                tmp.to_str().expect("utf8 path"),
                "-c",
                c_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            compile.status.success(),
            "C ABI object compile failed\nstdout={}\nstderr={}",
            normalize(&compile.stdout),
            normalize(&compile.stderr)
        );

        let archive = run_tool(
            "ar",
            &[
                "rcs",
                static_archive.to_str().expect("utf8 path"),
                obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            archive.status.success(),
            "static archive creation failed\nstdout={}\nstderr={}",
            normalize(&archive.stdout),
            normalize(&archive.stderr)
        );

        let shared = run_tool(
            "cc",
            &[
                "-shared",
                obj.to_str().expect("utf8 path"),
                "-o",
                shared_library.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            shared.status.success(),
            "shared library creation failed\nstdout={}\nstderr={}",
            normalize(&shared.stdout),
            normalize(&shared.stderr)
        );

        let nm_static = run_tool(
            "nm",
            &["-g", static_archive.to_str().expect("utf8 path")],
            &tmp,
        );
        let nm_shared = run_tool(
            "nm",
            &["-g", shared_library.to_str().expect("utf8 path")],
            &tmp,
        );
        assert!(
            nm_static.status.success() && nm_shared.status.success(),
            "nm must inspect static and shared artifacts\nstatic stdout={}\nstatic stderr={}\nshared stdout={}\nshared stderr={}",
            normalize(&nm_static.stdout),
            normalize(&nm_static.stderr),
            normalize(&nm_shared.stdout),
            normalize(&nm_shared.stderr)
        );
        let symbols = format!(
            "{}{}",
            normalize(&nm_static.stdout),
            normalize(&nm_shared.stdout)
        );
        for symbol in [
            "nc_c_abi_add",
            "nc_c_abi_packet_init",
            "nc_c_abi_packet_value",
            "nc_c_abi_alloc_bytes",
            "nc_c_abi_free_bytes",
        ] {
            assert!(
                symbols.contains(symbol),
                "exported C ABI symbol missing: {}\n{}",
                symbol,
                symbols
            );
        }

        let link_static = run_tool(
            "cc",
            &[
                "-I",
                tmp.to_str().expect("utf8 path"),
                driver_src.to_str().expect("utf8 path"),
                static_archive.to_str().expect("utf8 path"),
                "-o",
                static_exe.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            link_static.status.success(),
            "static C ABI driver link failed\nstdout={}\nstderr={}",
            normalize(&link_static.stdout),
            normalize(&link_static.stderr)
        );

        let rpath = format!("-Wl,-rpath,{}", tmp.display());
        let link_shared = Command::new("cc")
            .arg("-I")
            .arg(&tmp)
            .arg(&driver_src)
            .arg("-L")
            .arg(&tmp)
            .arg("-lnc_c_abi_industrial")
            .arg(&rpath)
            .arg("-o")
            .arg(&shared_exe)
            .current_dir(&tmp)
            .output()
            .expect("shared link command must execute");
        assert!(
            link_shared.status.success(),
            "shared C ABI driver link failed\nstdout={}\nstderr={}",
            normalize(&link_shared.stdout),
            normalize(&link_shared.stderr)
        );

        for exe in [&static_exe, &shared_exe] {
            let exec = Command::new(exe)
                .current_dir(&tmp)
                .output()
                .expect("C ABI driver executable must run");
            assert!(
                exec.status.success(),
                "C ABI driver must exit 0: {}\nstdout={}\nstderr={}\nstatus={}",
                exe.display(),
                normalize(&exec.stdout),
                normalize(&exec.stderr),
                exec.status
            );
        }

        let manifest_json = serde_json::json!({
            "status": "closed_real_c_abi_toolchain_smoke",
            "contracts": {
                "header": header.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
                "abi": "C",
                "layout": "repr(C)",
                "ownership": ["borrowed", "owned_with_free_symbol"],
                "nullability": "nonNull pointer contracts",
                "static_archive": true,
                "shared_library": true
            },
            "artifacts": [
                {"kind": "header", "path": header.display().to_string(), "sha256": sha256_hex_file(&header)},
                {"kind": "object", "path": obj.display().to_string(), "sha256": sha256_hex_file(&obj)},
                {"kind": "static_archive", "path": static_archive.display().to_string(), "sha256": sha256_hex_file(&static_archive)},
                {"kind": "shared_library", "path": shared_library.display().to_string(), "sha256": sha256_hex_file(&shared_library)},
                {"kind": "static_driver", "path": static_exe.display().to_string(), "sha256": sha256_hex_file(&static_exe)},
                {"kind": "shared_driver", "path": shared_exe.display().to_string(), "sha256": sha256_hex_file(&shared_exe)}
            ],
            "symbols": [
                "nc_c_abi_add",
                "nc_c_abi_packet_init",
                "nc_c_abi_packet_value",
                "nc_c_abi_alloc_bytes",
                "nc_c_abi_free_bytes"
            ]
        });
        fs::write(
            &manifest,
            serde_json::to_string_pretty(&manifest_json).expect("manifest json must render"),
        )
        .expect("manifest must write");
        for artifact in manifest_json["artifacts"]
            .as_array()
            .expect("artifacts array")
        {
            let hash = artifact["sha256"].as_str().expect("sha256 string");
            assert_eq!(hash.len(), 64, "artifact hash must be SHA-256 hex");
        }
    }

    #[test]
    fn ffi_native_autolink_consumes_declared_static_and_shared_artifacts() {
        if !command_available("cc") || !command_available("ar") {
            return;
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for temp names")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_declared_artifacts_{}_{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&tmp).expect("temp workspace must be created");

        let header = tmp.join("external_ffi.h");
        let static_src = tmp.join("external_static.c");
        let shared_src = tmp.join("external_shared.c");
        let static_obj = tmp.join("external_static.o");
        let shared_obj = tmp.join("external_shared.o");
        let static_archive = tmp.join("libexternal_static.a");
        let shared_library = tmp.join("libexternal_shared.so");
        let fixture = tmp.join("ffi_declared_artifacts.n");
        let bad_fixture = tmp.join("ffi_declared_artifacts_bad_checksum.n");

        fs::write(
            &header,
            "int nc_external_static_add(int a, int b);\nint nc_external_shared_mul(int a, int b);\n",
        )
        .expect("header must write");
        fs::write(
            &static_src,
            "int nc_external_static_add(int a, int b) { return a + b; }\n",
        )
        .expect("static source must write");
        fs::write(
            &shared_src,
            "int nc_external_shared_mul(int a, int b) { return a * b; }\n",
        )
        .expect("shared source must write");

        let compile_static = run_tool(
            "cc",
            &[
                "-c",
                static_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                static_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            compile_static.status.success(),
            "static object compile failed\nstdout={}\nstderr={}",
            normalize(&compile_static.stdout),
            normalize(&compile_static.stderr)
        );
        let archive = run_tool(
            "ar",
            &[
                "rcs",
                static_archive.to_str().expect("utf8 path"),
                static_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            archive.status.success(),
            "static archive creation failed\nstdout={}\nstderr={}",
            normalize(&archive.stdout),
            normalize(&archive.stderr)
        );

        let compile_shared = run_tool(
            "cc",
            &[
                "-fPIC",
                "-c",
                shared_src.to_str().expect("utf8 path"),
                "-O2",
                "-o",
                shared_obj.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            compile_shared.status.success(),
            "shared object compile failed\nstdout={}\nstderr={}",
            normalize(&compile_shared.stdout),
            normalize(&compile_shared.stderr)
        );
        let shared = run_tool(
            "cc",
            &[
                "-shared",
                shared_obj.to_str().expect("utf8 path"),
                "-o",
                shared_library.to_str().expect("utf8 path"),
            ],
            &tmp,
        );
        assert!(
            shared.status.success(),
            "shared library creation failed\nstdout={}\nstderr={}",
            normalize(&shared.stdout),
            normalize(&shared.stderr)
        );

        let static_hash = sha256_hex_file(&static_archive);
        let shared_hash = sha256_hex_file(&shared_library);
        let static_artifact = static_archive.display();
        let shared_artifact = shared_library.display();
        let source = format!(
            r#"use foreign.c;

ffiDeclaredArtifacts[@runtime] {{
    foreign:
        c symbol "nc_external_static_add" {{
            abi: C;
            header: "external_ffi.h";
            artifact: "{static_artifact}";
            library: "{static_artifact}";
            linkage: static;
            checksum: "sha256:{static_hash}";
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
            signature: fn(int, int) -> int;
        }}

        c symbol "nc_external_shared_mul" {{
            abi: C;
            header: "external_ffi.h";
            artifact: "{shared_artifact}";
            library: "{shared_artifact}";
            linkage: dynamic;
            checksum: "sha256:{shared_hash}";
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
            signature: fn(int, int) -> int;
        }}

    stc:
        staticOk: bool;
        sharedOk: bool;

    fn:
        run() -> bool {{
            public:
                @staticOk >> bool : nc_external_static_add(20, 22) == 42;
                @sharedOk >> bool : nc_external_shared_mul(6, 7) == 42;
                return @staticOk && @sharedOk;
        }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#
        );
        fs::write(&fixture, &source).expect("fixture must write");

        let check = run_ncc(&["check", fixture.to_str().expect("utf8 path")]);
        assert!(
            check.status.success(),
            "declared artifact fixture must pass check\nstdout={}\nstderr={}",
            normalize(&check.stdout),
            normalize(&check.stderr)
        );
        let run = run_ncc(&[
            "run",
            fixture.to_str().expect("utf8 path"),
            "--native-ffi",
            "--entry",
            "ffiDeclaredArtifacts.run",
        ]);
        assert!(
            run.status.success(),
            "ncc run --native-ffi must link declared .a/.so artifacts and pass\nstdout={}\nstderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );

        fs::write(
            &bad_fixture,
            source.replace(
                &format!("sha256:{shared_hash}"),
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            ),
        )
        .expect("bad fixture must write");
        let bad_run = run_ncc(&[
            "run",
            bad_fixture.to_str().expect("utf8 path"),
            "--native-ffi",
            "--entry",
            "ffiDeclaredArtifacts.run",
        ]);
        assert!(
            !bad_run.status.success(),
            "bad checksum fixture must fail\nstdout={}\nstderr={}",
            normalize(&bad_run.stdout),
            normalize(&bad_run.stderr)
        );
        let bad_out = format!(
            "{}{}",
            normalize(&bad_run.stdout),
            normalize(&bad_run.stderr)
        );
        assert!(
            bad_out.contains("E_FFI_CHECKSUM_001"),
            "bad checksum must report official diagnostic\n{}",
            bad_out
        );
    }
}
