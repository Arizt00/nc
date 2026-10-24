use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ncc-{label}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("binary parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH")
        .to_string_lossy()
        .to_string()
}

fn run_ncc(args: &[String]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn canonical_language_artifacts_generate_check_and_detect_drift() {
    let output_dir = unique_temp_dir("canonical-language-spec");
    let generate = run_ncc(&[
        "language-spec".into(),
        "--generate".into(),
        output_dir.to_string_lossy().to_string(),
    ]);
    assert!(generate.status.success(), "{}", rendered(&generate));

    for name in [
        "nc-language-spec.json",
        "nc-language.ebnf",
        "nc-language-spec.md",
    ] {
        assert!(output_dir.join(name).is_file(), "missing {name}");
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output_dir.join("nc-language-spec.json")).expect("manifest bytes"),
    )
    .expect("manifest JSON");
    assert_eq!(manifest["schema"], "nc.language.canonical.v1");
    assert!(manifest["grammar"]
        .as_array()
        .expect("grammar array")
        .iter()
        .any(|item| item["name"] == "try_expr"));
    assert!(manifest["capabilities"]
        .as_array()
        .expect("capability array")
        .iter()
        .any(|item| item["name"] == "explicit_binary_layout"));
    assert!(manifest["type_families"]
        .as_array()
        .expect("type family array")
        .iter()
        .any(|item| item["name"] == "result"));

    let check = run_ncc(&[
        "language-spec".into(),
        "--check".into(),
        output_dir.to_string_lossy().to_string(),
    ]);
    assert!(check.status.success(), "{}", rendered(&check));

    std::fs::write(output_dir.join("nc-language.ebnf"), "drift\n").expect("inject drift");
    let drift = run_ncc(&[
        "language-spec".into(),
        "--check".into(),
        output_dir.to_string_lossy().to_string(),
    ]);
    assert!(!drift.status.success(), "drift must be rejected");
    assert!(rendered(&drift).contains("E_SPEC_DRIFT_001"));

    std::fs::remove_dir_all(output_dir).expect("temporary spec tree clean");
}

#[test]
fn error_propagation_and_binary_layout_run_in_interpreter() {
    let root = workspace_root();
    for source in [
        root.join("examples/03_types/errorPropagationDefer.n"),
        root.join("examples/03_types/errorPropagationAdvanced.n"),
        root.join("examples/03_types/explicitBinaryLayout.n"),
    ] {
        let check = run_ncc(&["check".into(), source.to_string_lossy().to_string()]);
        assert!(check.status.success(), "{}", rendered(&check));
        let run = run_ncc(&[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--interpreted".into(),
            "--show-return".into(),
        ]);
        assert!(run.status.success(), "{}", rendered(&run));
        assert!(rendered(&run).contains("[return: true"));
    }
}

#[test]
fn closure_try_uses_the_closure_return_contract() {
    let source =
        workspace_root().join("examples/03_types/negative/errorPropagationClosureReturnMismatch.n");
    let check = run_ncc(&["check".into(), source.to_string_lossy().to_string()]);
    assert!(!check.status.success(), "closure mismatch must be rejected");
    assert!(
        rendered(&check).contains("E_HIGH_TRY_CONTEXT_001"),
        "{}",
        rendered(&check)
    );
}

#[test]
fn llvm_materializes_cleanup_bitfields_endian_and_union_storage() {
    let root = workspace_root();
    let output_dir = unique_temp_dir("canonical-llvm");

    let error_llvm = output_dir.join("error.ll");
    let error = run_ncc(&[
        "mct".into(),
        "emit-llvm".into(),
        root.join("examples/03_types/errorPropagationDefer.n")
            .to_string_lossy()
            .to_string(),
        "--out".into(),
        error_llvm.to_string_lossy().to_string(),
    ]);
    assert!(error.status.success(), "{}", rendered(&error));
    let error_text = std::fs::read_to_string(error_llvm).expect("error LLVM");
    assert!(error_text.contains("try_propagate"));
    assert!(error_text.contains("__ncc_defer_active_0"));
    assert!(error_text.contains("defer_run"));

    let advanced_llvm = output_dir.join("error-advanced.ll");
    let advanced = run_ncc(&[
        "mct".into(),
        "emit-llvm".into(),
        root.join("examples/03_types/errorPropagationAdvanced.n")
            .to_string_lossy()
            .to_string(),
        "--out".into(),
        advanced_llvm.to_string_lossy().to_string(),
    ]);
    assert!(advanced.status.success(), "{}", rendered(&advanced));
    let advanced_text = std::fs::read_to_string(advanced_llvm).expect("advanced error LLVM");
    assert!(advanced_text.contains("define i1 @errorPropagationAdvanced_testAsync()"));
    assert!(advanced_text.contains("@errorPropagationAdvanced_asyncRelay(ptr "));
    assert!(advanced_text.contains(", i1 false)"));
    assert!(advanced_text.contains("errorPropagationAdvanced___ncc_coroutine_asyncRelay_resume"));
    assert!(advanced_text.contains("coroutine_await_poll"));
    assert!(
        !advanced_text.contains("@nC_runtime_async_await"),
        "high-level await must poll the native descriptor, not call a hosted runtime shim"
    );

    let layout_llvm = output_dir.join("layout.ll");
    let layout = run_ncc(&[
        "mct".into(),
        "emit-llvm".into(),
        root.join("examples/03_types/explicitBinaryLayout.n")
            .to_string_lossy()
            .to_string(),
        "--out".into(),
        layout_llvm.to_string_lossy().to_string(),
    ]);
    assert!(layout.status.success(), "{}", rendered(&layout));
    let layout_text = std::fs::read_to_string(layout_llvm).expect("layout LLVM");
    for marker in [
        "%struct.explicitBinaryLayout_Header = type { [8 x i8] }",
        "%struct.explicitBinaryLayout_Value = type { [8 x i8] }",
        "getelementptr inbounds [2 x i16]",
        "getelementptr inbounds [2 x %struct.explicitBinaryLayout_Cell]",
        "@llvm.bswap.i32",
        "and i32",
        "shl i32",
    ] {
        assert!(layout_text.contains(marker), "LLVM missing {marker}");
    }
    assert!(
        !layout_text.contains("raw statement boundary"),
        "every explicit-layout statement must lower to typed LLVM"
    );

    std::fs::remove_dir_all(output_dir).expect("temporary LLVM tree clean");
}

#[test]
fn kernel_error_propagation_materializes_for_every_host_certification_target() {
    let root = workspace_root();
    let source = root.join("examples/11_machine/kernelErrorPropagation.n");
    let temp = unique_temp_dir("kernel-error-propagation");
    let targets: &[&str] = if cfg!(target_os = "linux") {
        &[
            "x86_64-unknown-none",
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "x86_64-apple-darwin",
            "aarch64-unknown-none",
            "aarch64-unknown-linux-gnu",
            "i386-unknown-none-elf",
            "i386-unknown-linux-gnu",
            "riscv64-unknown-elf",
            "riscv32imac-unknown-elf",
        ]
    } else if cfg!(target_os = "windows") {
        &["x86_64-unknown-none", "x86_64-pc-windows-msvc"]
    } else {
        &[
            "x86_64-unknown-none",
            "x86_64-apple-darwin",
            "aarch64-unknown-none",
        ]
    };

    for target in targets {
        let out_dir = temp.join(target.replace('-', "_"));
        let build = run_ncc(&[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--profile".into(),
            "kernel".into(),
            "--target".into(),
            (*target).into(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--json".into(),
        ]);
        assert!(build.status.success(), "{target}: {}", rendered(&build));
        let report: serde_json::Value = serde_json::from_slice(&build.stdout)
            .unwrap_or_else(|error| panic!("{target} report: {error}\n{}", rendered(&build)));
        assert_eq!(report["status"], "kernel_build_passed", "{target}");
        assert_eq!(
            report["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"], true,
            "{target}"
        );
        let llvm_path = PathBuf::from(
            report["paths"]["high_level_body_llvm_ir"]
                .as_str()
                .expect("kernel LLVM path"),
        );
        let object_path = PathBuf::from(
            report["paths"]["high_level_body_object"]
                .as_str()
                .expect("kernel object path"),
        );
        assert!(object_path.is_file(), "{target}: {}", object_path.display());
        let llvm = std::fs::read_to_string(&llvm_path).expect("kernel LLVM");
        for marker in [
            "define i64 @kernelErrorPropagation_boot()",
            "try_propagate",
            "__ncc_defer_active_0",
            "defer_run",
        ] {
            assert!(llvm.contains(marker), "{target}: LLVM missing {marker}");
        }
    }

    std::fs::remove_dir_all(temp).expect("temporary kernel propagation tree clean");
}

#[cfg(all(unix, target_arch = "x86_64"))]
#[test]
fn explicit_layout_builds_and_executes_as_external_elf() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }
    let root = workspace_root();
    let output_dir = unique_temp_dir("explicit-layout-native");
    let build = run_ncc(&[
        "build".into(),
        root.join("examples/03_types/explicitBinaryLayout.n")
            .to_string_lossy()
            .to_string(),
        "--out-dir".into(),
        output_dir.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(build.status.success(), "{}", rendered(&build));
    let report: serde_json::Value = serde_json::from_slice(&build.stdout)
        .unwrap_or_else(|error| panic!("native build JSON: {error}\n{}", rendered(&build)));
    assert_eq!(report["status"], "native_build_passed");
    let executable = PathBuf::from(
        report["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let execution = Command::new(&executable)
        .current_dir(root)
        .output()
        .expect("external layout executable");
    assert!(execution.status.success(), "{}", rendered(&execution));

    std::fs::remove_dir_all(output_dir).expect("temporary native tree clean");
}

#[cfg(all(unix, target_arch = "x86_64"))]
#[test]
fn advanced_error_propagation_builds_and_executes_as_external_elf() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }
    let root = workspace_root();
    let output_dir = unique_temp_dir("advanced-error-native");
    let build = run_ncc(&[
        "build".into(),
        root.join("examples/03_types/errorPropagationAdvanced.n")
            .to_string_lossy()
            .to_string(),
        "--out-dir".into(),
        output_dir.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(build.status.success(), "{}", rendered(&build));
    let report: serde_json::Value = serde_json::from_slice(&build.stdout)
        .unwrap_or_else(|error| panic!("native build JSON: {error}\n{}", rendered(&build)));
    assert_eq!(report["status"], "native_build_passed");
    let executable = PathBuf::from(
        report["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let execution = Command::new(&executable)
        .current_dir(root)
        .output()
        .expect("external error propagation executable");
    assert!(execution.status.success(), "{}", rendered(&execution));

    std::fs::remove_dir_all(output_dir).expect("temporary native tree clean");
}
