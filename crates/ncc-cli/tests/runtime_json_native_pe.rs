#![cfg(target_os = "windows")]

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn clang_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
        || Path::new(r"C:\Program Files\LLVM\bin\clang.exe").is_file()
}

fn absolute_from(root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn assert_no_materialized_c_sources(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("artifact directory") {
            let path = entry.expect("artifact entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                assert!(
                    !path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("c")),
                    "compiler-owned C source leaked into .ncc artifacts: {}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn json_runtime_coff_is_linked_only_when_referenced_and_executes_full_pe_contract() {
    if !clang_available() {
        return;
    }

    let root = workspace_root();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = root.join("target").join("tmp").join(format!(
        "runtime-json-native-pe-{}-{nonce}",
        std::process::id()
    ));
    let isolated_cwd = base.join("cwd");
    let out_dir = base.join("out");
    fs::create_dir_all(&isolated_cwd).expect("isolated cwd");
    fs::create_dir_all(&out_dir).expect("native output");
    fs::create_dir_all(out_dir.join("lowering")).expect("legacy lowering directory");
    fs::write(
        out_dir.join("lowering/old-main.runtime-json.c"),
        "legacy generated runtime source",
    )
    .expect("legacy JSON runtime source");
    fs::write(
        out_dir.join("lowering/another-unit.runtime-hosted.c"),
        "legacy generated hosted runtime source",
    )
    .expect("legacy hosted runtime source");
    fs::create_dir_all(out_dir.join("lowering/cache/legacy"))
        .expect("nested legacy lowering directory");
    fs::write(
        out_dir.join("lowering/cache/legacy/arbitrary-provider.C"),
        "legacy generated compiler source",
    )
    .expect("nested legacy compiler source");

    let source = root.join("examples/runtime/json/json_native_pe_status_ok.n");
    let output = Command::new(env!("CARGO_BIN_EXE_ncc"))
        .current_dir(&root)
        .args([
            "compiler",
            "build-native",
            source.to_str().expect("source path"),
            "--out-dir",
            out_dir.to_str().expect("output path"),
            "--cc",
            "nC.win64",
            "-O2",
            "--json",
            "--strict",
        ])
        .output()
        .expect("run compiler build-native");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 compiler output");
    let json_start = stdout.find('{').expect("native-build JSON payload");
    let payload: Value = serde_json::from_str(&stdout[json_start..]).expect("native-build JSON");

    assert_eq!(payload["status"], "native_build_passed");
    assert_eq!(payload["runtime_json_native_object"], true);
    assert_eq!(payload["runtime_json"]["referenced"], true);
    assert_eq!(payload["runtime_json"]["native_object_enabled"], true);
    assert_eq!(payload["runtime_json"]["symbol_count"], 46);
    assert!(payload["runtime_json"]["source"].is_null());
    assert_eq!(
        payload["runtime_json"]["source_materialization"],
        "compiler-embedded-stdin"
    );
    assert_no_materialized_c_sources(&out_dir);
    assert!(payload["link"]["phase12_report"]["unresolved_symbols"]
        .as_array()
        .expect("unresolved symbols")
        .is_empty());
    assert!(payload["runtime_json"]["source_sha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert!(payload["runtime_json"]["object_sha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert!(
        payload["stages"]
            .as_array()
            .expect("stages")
            .iter()
            .any(|stage| stage["stage"] == "json-runtime-target-object"
                && stage["status"] == "passed")
    );

    let executable = absolute_from(
        &root,
        payload["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let json_object = absolute_from(
        &root,
        payload["runtime_json"]["native_object"]
            .as_str()
            .expect("runtime JSON object path"),
    );
    assert_eq!(&fs::read(&executable).expect("PE executable")[..2], b"MZ");
    assert_eq!(
        &fs::read(&json_object).expect("COFF JSON object")[..2],
        &[0x64, 0x86]
    );

    // A stale cwd settings file must not mask the valid deployment copy that
    // jsonSaveFile deliberately places alongside the executable.
    fs::write(isolated_cwd.join("settings.json"), b"{broken]").expect("stale cwd settings fixture");
    let run = Command::new(&executable)
        .current_dir(&isolated_cwd)
        .output()
        .expect("execute generated PE");
    assert_eq!(
        run.status.code(),
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(isolated_cwd
        .join("runtime_json_native_roundtrip.json")
        .is_file());
    let deployed_settings = executable
        .parent()
        .expect("executable directory")
        .join("settings.json");
    let settings: Value = serde_json::from_slice(
        &fs::read(&deployed_settings).expect("settings saved beside executable"),
    )
    .expect("valid deployed settings JSON");
    assert_eq!(settings["root"]["element"], "Cm");
    assert_eq!(settings["mutated"], serde_json::json!([2, 3, 4]));

    let plain_source = base.join("no_json_runtime.n");
    let plain_out = base.join("plain-out");
    fs::write(
        &plain_source,
        r#"noJsonRuntime :: runtime {
    fn:
        run() -> int {
            public:
                return 0;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    )
    .expect("plain nC fixture");
    let plain = Command::new(env!("CARGO_BIN_EXE_ncc"))
        .current_dir(&root)
        .args([
            "compiler",
            "build-native",
            plain_source.to_str().expect("plain source path"),
            "--out-dir",
            plain_out.to_str().expect("plain output path"),
            "--cc",
            "nC.win64",
            "-O2",
            "--json",
            "--strict",
        ])
        .output()
        .expect("build plain native fixture");
    assert!(
        plain.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&plain.stdout),
        String::from_utf8_lossy(&plain.stderr)
    );
    let plain_stdout = String::from_utf8(plain.stdout).expect("plain compiler UTF-8");
    let plain_payload: Value =
        serde_json::from_str(&plain_stdout[plain_stdout.find('{').expect("plain JSON payload")..])
            .expect("plain native-build JSON");
    assert_eq!(plain_payload["runtime_json_native_object"], false);
    assert_eq!(plain_payload["runtime_json"]["referenced"], false);
    assert!(plain_payload["stages"]
        .as_array()
        .expect("plain stages")
        .iter()
        .all(|stage| stage["stage"] != "json-runtime-target-object"));
    assert!(plain_payload["artifacts"]
        .as_array()
        .expect("plain artifacts")
        .iter()
        .all(|artifact| artifact["kind"] != "runtime_json_native_object"));
}
