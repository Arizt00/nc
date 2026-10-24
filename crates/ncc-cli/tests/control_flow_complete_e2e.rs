#![cfg(all(unix, target_arch = "x86_64"))]

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
        .expect("workspace root must resolve")
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ncc-control-flow-e2e-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temporary output directory must create");
    dir
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("ncc binary parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
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
        .expect("ncc command must execute")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn complete_control_flow_builds_object_and_runs_as_external_elf() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }

    let root = workspace_root();
    let source = root.join("examples/02_flow/control_flow_complete.n");
    let out_dir = unique_temp_dir();

    let check = run_ncc(&["check".into(), source.to_string_lossy().to_string()]);
    assert!(check.status.success(), "{}", rendered(&check));

    let build = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(build.status.success(), "{}", rendered(&build));
    let json: serde_json::Value = serde_json::from_slice(&build.stdout)
        .unwrap_or_else(|err| panic!("native build JSON failed: {err}\n{}", rendered(&build)));

    assert_eq!(json["status"], "native_build_passed");
    assert_eq!(json["target"]["object_format"], "elf64");
    assert_eq!(json["speed_contract"]["interpreter_fallback"], false);

    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let object = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let machine = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    let backend_object = PathBuf::from(
        json["paths"]["machine_backend_object_ir"]
            .as_str()
            .expect("backend object path"),
    );
    for artifact in [&executable, &object, &machine, &backend_object] {
        assert!(artifact.is_file(), "missing {}", artifact.display());
        assert!(
            std::fs::metadata(artifact)
                .expect("artifact metadata")
                .len()
                > 0,
            "empty {}",
            artifact.display()
        );
    }
    assert_eq!(
        &std::fs::read(&executable).expect("ELF executable")[..4],
        b"\x7fELF"
    );
    assert_eq!(
        &std::fs::read(&object).expect("ELF object")[..4],
        b"\x7fELF"
    );

    let backend_text = std::fs::read_to_string(&backend_object).expect("backend object IR");
    for marker in [
        "range_positive",
        "range_zero_step",
        "repeat_count_negative",
        "__ncc_stc_controlFlowComplete_values",
    ] {
        assert!(
            backend_text.contains(marker),
            "missing backend marker {marker}"
        );
    }

    let execution = Command::new(&executable)
        .current_dir(&root)
        .output()
        .expect("external ELF must execute directly");
    assert!(execution.status.success(), "{}", rendered(&execution));
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).replace("\r\n", "\n"),
        "control_flow_complete=true\n"
    );

    std::fs::remove_dir_all(out_dir).expect("temporary build tree must clean");
}
