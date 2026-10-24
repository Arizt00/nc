use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

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

fn temp_project(name: &str) -> PathBuf {
    workspace_root().join("target").join("tmp").join(name)
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_success(out: Output, label: &str) -> String {
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "{label} failed\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn assert_failure_contains(out: Output, label: &str, needle: &str) -> String {
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    let rendered = format!("{stdout}{stderr}");
    assert!(
        !out.status.success(),
        "{label} should fail but passed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        rendered.contains(needle),
        "{label} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
    rendered
}

fn run_ncc_in_dir(args: &[String], cwd: &Path) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(cwd)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute in cwd")
}

fn run_ncc_with_env(args: &[String], envs: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(ncc_bin());
    command.args(args).env("NC_NO_COLOR", "1");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("ncc command must execute with env")
}

#[cfg(unix)]
fn windows_drive_path_from_wsl(path: &Path) -> Option<String> {
    let official = path.canonicalize().ok()?;
    let raw = official.to_string_lossy().replace('\\', "/");
    let rest = raw.strip_prefix("/mnt/")?;
    let mut parts = rest.splitn(2, '/');
    let drive = parts.next()?.chars().next()?.to_ascii_uppercase();
    let tail = parts.next()?.replace('/', "\\");
    Some(format!("{drive}:\\{tail}"))
}

fn json_file(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.to_string_lossy()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("parse json {}: {err}\n{raw}", path.to_string_lossy()))
}

fn assert_json_tree_is_valid(root: &Path) {
    fn visit(path: &Path) {
        if path.is_dir() {
            for entry in fs::read_dir(path)
                .unwrap_or_else(|err| panic!("read dir {}: {err}", path.to_string_lossy()))
            {
                visit(&entry.expect("dir entry").path());
            }
            return;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            let raw = fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("read json {}: {err}", path.to_string_lossy()));
            serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|err| {
                panic!("parse json {}: {err}\n{raw}", path.to_string_lossy())
            });
        }
    }
    visit(&root.join("bridge.json"));
    visit(&root.join(".ncc"));
}

fn bridge_unit(root: &Path, name: &str) -> serde_json::Value {
    let bridge = json_file(&root.join("bridge.json"));
    bridge
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|units| {
            units
                .iter()
                .find(|unit| unit.get("name").and_then(|v| v.as_str()) == Some(name))
        })
        .cloned()
        .unwrap_or_else(|| panic!("bridge unit {name}"))
}

fn assert_project_object_contract(
    root: &Path,
    unit: &str,
    expected_export: &str,
) -> serde_json::Value {
    let bridge_unit = bridge_unit(root, unit);
    let object_rel = bridge_unit
        .get("object")
        .and_then(|v| v.as_str())
        .expect("bridge object path");
    let object_path = root.join(object_rel);
    let metadata_path = root.join(format!("{object_rel}.json"));
    let bytes = fs::read(&object_path)
        .unwrap_or_else(|err| panic!("read object {}: {err}", object_path.to_string_lossy()));

    let expected_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
    assert_eq!(
        bridge_unit.get("object_hash").and_then(|v| v.as_str()),
        Some(expected_hash.as_str())
    );

    let metadata = json_file(&metadata_path);
    assert_eq!(
        metadata.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.object.v1")
    );
    let object_format = metadata
        .pointer("/abi/object_format")
        .and_then(|v| v.as_str())
        .expect("object format");
    match object_format {
        "elf64" => assert!(bytes.starts_with(&[0x7f, b'E', b'L', b'F']), "{object_rel}"),
        "coff" => assert!(bytes.len() >= 20, "COFF object too small: {object_rel}"),
        "mach-o" => assert!(
            bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
                || bytes.starts_with(&[0xfe, 0xed, 0xfa, 0xcf]),
            "Mach-O object has unexpected magic: {object_rel}"
        ),
        other => panic!("unsupported object format in test helper: {other}"),
    }
    assert_eq!(
        metadata.get("object_hash").and_then(|v| v.as_str()),
        Some(expected_hash.as_str())
    );
    assert_eq!(
        metadata.get("object_byte_count").and_then(|v| v.as_u64()),
        Some(bytes.len() as u64)
    );
    let object_hex = metadata
        .get("object_bytes_hex")
        .and_then(|v| v.as_array())
        .expect("object bytes hex");
    assert_eq!(object_hex.len(), bytes.len());
    let first_hex = format!("{:02x}", bytes[0]);
    assert_eq!(
        object_hex.first().and_then(|v| v.as_str()),
        Some(first_hex.as_str())
    );
    assert_eq!(
        metadata.pointer("/abi/model").and_then(|v| v.as_str()),
        Some(default_host_abi_model())
    );
    assert_eq!(
        metadata
            .pointer("/abi/object_format")
            .and_then(|v| v.as_str()),
        Some(default_host_object_format())
    );
    assert!(metadata
        .get("exported_symbols")
        .and_then(|v| v.as_array())
        .is_some_and(|symbols| symbols
            .iter()
            .any(|symbol| symbol.as_str() == Some(expected_export))));
    metadata
}

fn project_fixture(name: &str) -> PathBuf {
    let root = workspace_root();
    let primary = root.join("examples").join("project_priority13").join(name);
    if primary.exists() {
        return primary;
    }
    let archived = root
        .join("_archive")
        .join("old_examples")
        .join("project_priority13")
        .join(name);
    if archived.exists() {
        return archived;
    }
    primary
}

fn copy_project_fixture(root: &Path, rel: &str, fixture: &str) {
    let source = project_fixture(fixture);
    let body = fs::read_to_string(&source)
        .unwrap_or_else(|err| panic!("read fixture {}: {err}", source.to_string_lossy()));
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("create {}: {err}", parent.to_string_lossy()));
    }
    fs::write(&dest, body)
        .unwrap_or_else(|err| panic!("write fixture {}: {err}", dest.to_string_lossy()));
}

fn init_temp_project(name: &str) -> PathBuf {
    let root = temp_project(name);
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init temp fixture",
    );
    root
}

fn sync_temp_project(root: &Path, label: &str) {
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        label,
    );
}

fn default_host_abi_model() -> &'static str {
    if cfg!(windows) {
        "win64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aapcs64"
    } else {
        "sysv64"
    }
}

fn default_host_object_format() -> &'static str {
    if cfg!(windows) {
        "coff"
    } else if cfg!(target_os = "macos") {
        "mach-o"
    } else {
        "elf64"
    }
}

fn host_cc_available() -> bool {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "ncc_project_bridge_cc_probe_{}_{}",
        std::process::id(),
        nonce
    ));
    if fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let source = dir.join("probe.c");
    let output = dir.join(if cfg!(windows) { "probe.exe" } else { "probe" });
    if fs::write(&source, "int main(void){return 0;}\n").is_err() {
        return false;
    }
    let ok = Command::new("cc")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .is_ok_and(|out| out.status.success() && output.exists());
    let _ = fs::remove_dir_all(&dir);
    ok
}

#[test]
fn priority13a_project_import_surface_checks() {
    let fixture = project_fixture("main_project_imports.n");
    let out = run_ncc(&["check".to_string(), fixture.to_string_lossy().to_string()]);
    assert_success(out, "priority13a project import check");
}

#[test]
fn priority13a_project_init_and_sync_generate_bridge_and_tiny_code_json() {
    let root = temp_project("priority13a_project_init_sync");
    let _ = fs::remove_dir_all(&root);

    let init_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project init",
    );
    let init_report: serde_json::Value =
        serde_json::from_str(&init_stdout).expect("init report json");
    assert_eq!(
        init_report.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.priority13a.report.v1")
    );

    fs::write(
        root.join("main.n"),
        "use <project:memoria, scheduler>;\n\nmainProject { fa: return true; }\n",
    )
    .expect("write main.n");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit { fa: return true; }\n",
    )
    .expect("write memoria");
    fs::write(
        root.join("src").join("scheduler.n"),
        "use <project:memoria>;\n\nschedulerUnit { fa: return true; }\n",
    )
    .expect("write scheduler");
    fs::write(
        root.join("src").join("machine_leaf.nm"),
        "target: x86_64-unknown-none;\n",
    )
    .expect("write ignored machine source");

    let sync_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project sync",
    );
    let sync_report: serde_json::Value =
        serde_json::from_str(&sync_stdout).expect("sync report json");
    assert_eq!(sync_report.get("units").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(sync_report.get("edges").and_then(|v| v.as_u64()), Some(3));

    let bridge = json_file(&root.join("bridge.json"));
    assert_eq!(
        bridge.get("schema").and_then(|v| v.as_str()),
        Some("ncc.bridge.v1")
    );
    let edges = bridge.get("edges").and_then(|v| v.as_array()).unwrap();
    assert!(edges
        .iter()
        .any(|edge| edge == &serde_json::json!(["main", "memoria"])));
    assert!(edges
        .iter()
        .any(|edge| edge == &serde_json::json!(["main", "scheduler"])));
    assert!(edges
        .iter()
        .any(|edge| edge == &serde_json::json!(["scheduler", "memoria"])));
    let main_bridge_unit = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|units| {
            units
                .iter()
                .find(|unit| unit.get("name").and_then(|v| v.as_str()) == Some("main"))
        })
        .expect("main unit in bridge");
    assert_eq!(
        main_bridge_unit.get("code").and_then(|v| v.as_str()),
        Some(".ncc/code/main.json")
    );
    let link_order = bridge
        .get("link_order")
        .and_then(|v| v.as_array())
        .expect("physical link_order");
    assert_eq!(
        link_order
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>(),
        vec!["memoria", "scheduler", "main"]
    );

    let memoria_bridge_unit = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|units| {
            units
                .iter()
                .find(|unit| unit.get("name").and_then(|v| v.as_str()) == Some("memoria"))
        })
        .expect("memoria unit in bridge");
    let object_relative = memoria_bridge_unit
        .get("object")
        .and_then(|v| v.as_str())
        .expect("memoria object path");
    assert!(
        object_relative.starts_with(".ncc/obj/"),
        "{object_relative}"
    );
    assert!(
        object_relative.ends_with(".o") || object_relative.ends_with(".obj"),
        "{object_relative}"
    );
    assert!(
        root.join(object_relative).is_file(),
        "object file must exist at {object_relative}"
    );
    assert!(memoria_bridge_unit
        .get("object_hash")
        .and_then(|v| v.as_str())
        .is_some_and(|hash| hash.starts_with("sha256:")));
    assert!(memoria_bridge_unit
        .get("source_hash")
        .and_then(|v| v.as_str())
        .is_some_and(|hash| hash.starts_with("sha256:")));
    assert!(memoria_bridge_unit
        .get("symbols")
        .and_then(|v| v.as_array())
        .is_some_and(|symbols| symbols.iter().any(|symbol| {
            symbol.get("name").and_then(|v| v.as_str()) == Some("memoriaUnit")
                && symbol.get("kind").and_then(|v| v.as_str()) == Some("container")
        })));
    assert_eq!(
        memoria_bridge_unit
            .get("imports")
            .and_then(|v| v.as_array())
            .map(|items| items.len()),
        Some(0)
    );

    let scheduler_bridge_unit = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|units| {
            units
                .iter()
                .find(|unit| unit.get("name").and_then(|v| v.as_str()) == Some("scheduler"))
        })
        .expect("scheduler unit in bridge");
    assert_eq!(
        scheduler_bridge_unit.get("imports"),
        Some(&serde_json::json!(["memoria"]))
    );
    assert_eq!(
        scheduler_bridge_unit.get("deps"),
        Some(&serde_json::json!(["memoria"]))
    );

    let main_json_path = root.join(".ncc").join("code").join("main.json");
    let main_raw = fs::read_to_string(&main_json_path).expect("main code json");
    assert!(
        main_raw.lines().count() <= 10,
        "main code json must stay tiny:\n{main_raw}"
    );
    let main_code = json_file(&main_json_path);
    assert_eq!(
        main_code.get("source").and_then(|v| v.as_str()),
        Some("main.n")
    );

    let memoria_json_path = root.join(".ncc").join("code").join("memoria.json");
    let memoria_raw = fs::read_to_string(&memoria_json_path).expect("memoria code json");
    assert!(
        memoria_raw.lines().count() <= 10,
        "code json must stay tiny:\n{memoria_raw}"
    );
    assert!(!memoria_raw.contains("native_fast"));
    assert!(!memoria_raw.contains("llvm"));
    assert!(!memoria_raw.contains("machine"));
    assert!(!memoria_raw.contains("basal"));

    let memoria = json_file(&memoria_json_path);
    assert_eq!(
        memoria.get("schema").and_then(|v| v.as_str()),
        Some("ncc.code.v1")
    );
    assert_eq!(
        memoria.get("unit").and_then(|v| v.as_str()),
        Some("memoria")
    );
    assert_eq!(
        memoria.get("source").and_then(|v| v.as_str()),
        Some("src/memoria.n")
    );
    let expected_hash = format!(
        "sha256:{:x}",
        Sha256::digest("memoriaUnit { fa: return true; }\n".as_bytes())
    );
    assert_eq!(
        memoria.get("hash").and_then(|v| v.as_str()),
        Some(expected_hash.as_str())
    );
    assert!(!root
        .join(".ncc")
        .join("code")
        .join("machine_leaf.json")
        .exists());
}

#[test]
fn project_init_accepts_an_existing_main_with_project_imports() {
    let root = temp_project("project_init_existing_imports");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create project src");
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\nmainApp { ref main ->> @fn(worker.workerUnit.run); fa: return true; }\n",
    )
    .expect("write populated main");
    fs::write(
        root.join("src").join("worker.n"),
        "workerUnit { fn: public: run() -> bool { return true; } fa: return true; }\n",
    )
    .expect("write worker");

    let init_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project init with existing imports",
    );
    let init_report: serde_json::Value =
        serde_json::from_str(&init_stdout).expect("init report json");
    assert_eq!(
        init_report.get("units").and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        init_report.get("edges").and_then(|value| value.as_u64()),
        Some(1)
    );

    let bridge = json_file(&root.join("bridge.json"));
    assert!(bridge
        .get("edges")
        .and_then(|value| value.as_array())
        .is_some_and(|edges| edges.contains(&serde_json::json!(["main", "worker"]))));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn priority13_project_model_reports_clean_bridge_for_ide() {
    let root = init_temp_project("priority13_project_model_clean");
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\n\nmainModel { fa: return true; }\n",
    )
    .expect("write model main");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaModel { fa: return true; }\n",
    )
    .expect("write model child");
    sync_temp_project(&root, "project sync for model");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project check materializes model artifacts",
    );

    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "model".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project model",
    );
    let model: serde_json::Value = serde_json::from_str(&stdout).expect("project model json");
    assert_eq!(
        model.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.model.v1")
    );
    assert_eq!(model.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(
        model
            .get("units")
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(2)
    );
    assert_eq!(
        model
            .get("dirty_units")
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(0)
    );
}

#[test]
fn priority13_project_incremental_tracks_dirty_unit_and_dependents() {
    let root = init_temp_project("priority13_project_incremental_dirty");
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\n\nmainInc { fa: return true; }\n",
    )
    .expect("write incremental main");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaInc { fa: return true; }\n",
    )
    .expect("write incremental child");
    sync_temp_project(&root, "project sync for incremental");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project check materializes incremental artifacts",
    );
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaInc { fn: public: touched() -> bool { return true; } fa: return true; }\n",
    )
    .expect("dirty incremental child");

    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "incremental".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project incremental",
    );
    let incremental: serde_json::Value =
        serde_json::from_str(&stdout).expect("project incremental json");
    assert_eq!(
        incremental.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.incremental.v1")
    );
    assert_eq!(
        incremental.get("status").and_then(|v| v.as_str()),
        Some("rebuild_required")
    );
    let rebuild_units = incremental
        .get("rebuild_units")
        .and_then(|v| v.as_array())
        .expect("rebuild units")
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();
    assert!(rebuild_units.contains(&"memoria"));
    assert!(rebuild_units.contains(&"main"));
    assert!(root
        .join(".ncc")
        .join("incremental")
        .join("status.json")
        .is_file());
    let reasons = incremental
        .get("reason_by_unit")
        .and_then(|v| v.as_object())
        .expect("reason map");
    assert!(reasons
        .get("memoria")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items
            .iter()
            .any(|v| v.as_str() == Some("source_status=dirty"))));
    assert!(reasons
        .get("main")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items
            .iter()
            .any(|v| v.as_str() == Some("dependency_changed=memoria"))));
}

#[test]
fn priority13a_project_doctor_reports_existing_bridge() {
    let root = temp_project("priority13a_project_doctor");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init for doctor",
    );
    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "doctor".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project doctor",
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("doctor json");
    assert_eq!(report.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(
        report.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.priority13a.doctor.v1")
    );
    assert_eq!(report.get("units").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(report.get("code_files").and_then(|v| v.as_u64()), Some(1));
    assert!(report
        .get("identity_errors")
        .and_then(|v| v.as_array())
        .is_some_and(|errors| errors.is_empty()));
}

#[test]
fn priority13i_project_doctor_reports_missing_physical_object() {
    let root = temp_project("priority13i_project_missing_object");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init missing object",
    );
    fs::write(root.join("main.n"), "mainApp { fa: return true; }\n")
        .expect("write missing object main");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync missing object",
    );
    let bridge = json_file(&root.join("bridge.json"));
    let object_relative = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|units| units.first())
        .and_then(|unit| unit.get("object"))
        .and_then(|v| v.as_str())
        .expect("main object path");
    fs::remove_file(root.join(object_relative)).expect("remove physical object");

    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "doctor".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project doctor missing object",
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("doctor json");
    assert_eq!(report.get("status").and_then(|v| v.as_str()), Some("error"));
    assert!(report
        .get("bridge_errors")
        .and_then(|v| v.as_array())
        .is_some_and(|errors| errors.iter().any(|error| error
            .as_str()
            .is_some_and(|text| text.contains("E_PROJECT_BRIDGE_OBJECT_MISSING_001")))));
}

#[test]
fn priority13i_project_check_rejects_unknown_link_order_unit() {
    let root = temp_project("priority13i_project_bad_link_order");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init bad link order",
    );
    fs::write(root.join("main.n"), "mainApp { fa: return true; }\n")
        .expect("write bad link order main");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync bad link order",
    );
    let bridge_path = root.join("bridge.json");
    let mut bridge = json_file(&bridge_path);
    bridge["link_order"] = serde_json::json!(["ghost", "main"]);
    fs::write(
        &bridge_path,
        serde_json::to_string_pretty(&bridge).expect("render bad bridge"),
    )
    .expect("write bad bridge");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project check unknown link_order must fail",
        "E_PROJECT_BRIDGE_LINK_ORDER_002",
    );
}

#[test]
fn priority13b_project_run_links_main_to_child_and_writes_byte_lowering() {
    let root = temp_project("priority13b_project_run_linked");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init linked run",
    );
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\n\nmainApp {\n    ref main ->> @fn(memoriaUnit.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write linked main");
    fs::write(
        root.join("src").join("memoria.n"),
        "use::stoud::io::<output>;\n\nmemoriaUnit {\n    fn:\n        boot() -> bool {\n            public:\n                println(\"memoria-ok\");\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write linked child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync linked run",
    );
    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "project run linked child",
    );
    assert!(stdout.contains("memoria-ok"), "{stdout}");
    assert!(stdout.contains("[return: true profile="), "{stdout}");

    let lowering = json_file(
        &root
            .join(".ncc")
            .join("lowering")
            .join("memoria.lowering.json"),
    );
    assert_eq!(
        lowering.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.lowering.v1")
    );
    assert!(lowering
        .get("source_bytes_hex")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("6d"))));
    assert!(lowering
        .get("source_chars")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items.iter().any(|item| {
            item.get("text").and_then(|v| v.as_str()) == Some("m")
                && item
                    .get("utf8_bytes_hex")
                    .and_then(|v| v.as_array())
                    .is_some_and(|bytes| bytes.iter().any(|b| b.as_str() == Some("6d")))
        })));
    assert_eq!(
        lowering
            .get("hash_digest_bytes_hex")
            .and_then(|v| v.as_array())
            .map(|items| items.len()),
        Some(32)
    );
    assert!(lowering
        .pointer("/backend/debug_bytes_hex")
        .and_then(|v| v.as_array())
        .is_some_and(|items| !items.is_empty()));
}

#[test]
fn priority13b_plain_run_main_auto_uses_project_bridge() {
    let root = temp_project("priority13b_plain_run_main_auto_bridge");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init for plain run auto bridge",
    );
    fs::write(
        root.join("main.n"),
        "use <project:calculadora>;\n\nmochilaMain {\n    ref main ->> @fn(calculadora.calculadoraCore.menu);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write auto bridge main");
    fs::write(
        root.join("src").join("calculadora.n"),
        "use::stoud::io::<output>;\n\ncalculadoraCore {\n    fn:\n        public:\n            menu() -> bool {\n                println(\"calculadora-auto-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write auto bridge child");
    sync_temp_project(&root, "project sync for plain run auto bridge");

    let stdout = assert_success(
        run_ncc(&[
            "run".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
            "--interpreted".to_string(),
            "--show-return".to_string(),
        ]),
        "plain ncc run main.n must use bridge.json",
    );
    assert!(stdout.contains("calculadora-auto-ok"), "{stdout}");
    assert!(stdout.contains("[return: true profile="), "{stdout}");
    assert!(stdout.contains("OK: project run"), "{stdout}");
}

#[test]
fn priority13b_plain_build_main_materializes_linked_ast_with_native_linker() {
    let root = temp_project("priority13b_plain_build_main_native_bridge");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init for plain native bridge build",
    );
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\nnativeBridgeMain {\n    ref main ->> @fn(worker.workerUnit.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write native bridge main");
    fs::write(
        root.join("src").join("worker.n"),
        "workerUnit {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write native bridge child");
    sync_temp_project(&root, "project sync for plain native bridge build");

    let out_path = root
        .join(".ncc")
        .join("bin")
        .join(format!("native-bridge{}", std::env::consts::EXE_SUFFIX));
    let stdout = assert_success(
        run_ncc(&[
            "build".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
            out_path.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "plain ncc build main.n must use linked bridge AST",
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("native bridge build report must be JSON");
    assert_eq!(
        report
            .pointer("/project_linkage/schema")
            .and_then(|value| value.as_str()),
        Some("ncc.project.native_build_input.v1")
    );
    assert_eq!(
        report
            .pointer("/project_linkage/source_mode")
            .and_then(|value| value.as_str()),
        Some("formal-bridge")
    );
    assert_eq!(
        report
            .pointer("/project_linkage/unit_count")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        report
            .pointer("/project_linkage/runtime_c_shim_used")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        report
            .pointer("/link/internal_os_linker")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(out_path.is_file(), "native bridge executable must exist");

    let run = Command::new(&out_path)
        .output()
        .unwrap_or_else(|err| panic!("run native bridge {}: {err}", out_path.display()));
    assert!(
        run.status.success(),
        "linked native bridge executable must succeed\nstdout={}\nstderr={}",
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
}

#[test]
fn priority13b_single_file_run_links_sibling_without_project_init() {
    let root = temp_project("priority13b_single_file_run_no_project");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("single-file fixture dirs");
    fs::write(
        root.join("main.n"),
        "use::stoud::io::<output>;\nuse <project:calculadora>;\n\nmochilaMain {\n    ref main ->> @fn(calculadora.calculadoraCore.menu);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write single-file main");
    fs::write(
        root.join("src").join("calculadora.n"),
        "use::stoud::io::<output>;\n\ncalculadoraCore {\n    fn:\n        public:\n            menu() -> bool {\n                println(\"single-file-link-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write single-file child");

    let stdout = assert_success(
        run_ncc(&[
            "run".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
            "--interpreted".to_string(),
            "--show-return".to_string(),
        ]),
        "single-file run must link sibling project import without project init",
    );
    assert!(stdout.contains("single-file-link-ok"), "{stdout}");
    assert!(stdout.contains("[return: true profile="), "{stdout}");
    assert!(
        root.join(".ncc").join("code").join("main.json").is_file(),
        "single-file run must write main code identity"
    );
    assert!(
        root.join(".ncc")
            .join("code")
            .join("calculadora.json")
            .is_file(),
        "single-file run must write child code identity"
    );
    assert!(
        root.join(".ncc")
            .join("lowering")
            .join("__linked.lowering.json")
            .is_file(),
        "single-file run must write linked lowering identity"
    );
    let code = json_file(&root.join(".ncc").join("code").join("calculadora.json"));
    assert_eq!(
        code.get("schema").and_then(|v| v.as_str()),
        Some("ncc.code.v1")
    );
    assert_eq!(
        code.get("unit").and_then(|v| v.as_str()),
        Some("calculadora")
    );
    let route = json_file(&root.join(".ncc").join("run").join("last.json"));
    assert_eq!(
        route.get("route").and_then(|v| v.as_str()),
        Some("single-file-workspace-interpreter")
    );
    assert_eq!(
        route.get("source_mode").and_then(|v| v.as_str()),
        Some("single-file-workspace")
    );
}

#[test]
fn priority13b_run_resolves_source_in_folder_with_spaces() {
    let root = temp_project("priority13b universal path spaces");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("space fixture dir");
    let source = root.join("helloSpace.n");
    fs::write(
        &source,
        "use <io:output>;\n\nhelloSpace {\n    to main {\n        println(\"space-path-ok\");\n    }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write space path fixture");

    let stdout = assert_success(
        run_ncc(&[
            "run".to_string(),
            source.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "ncc run must resolve absolute path with spaces",
    );
    assert!(stdout.contains("space-path-ok"), "{stdout}");
    assert!(stdout.contains("route="), "{stdout}");
}

#[test]
fn priority13b_run_resolves_bare_filename_from_src_without_project() {
    let root = temp_project("priority13b_run_bare_filename_src");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("bare src dirs");
    fs::write(
        root.join("src").join("tool.n"),
        "use <io:output>;\n\ntool {\n    to main {\n        println(\"bare-src-ok\");\n    }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write bare src fixture");

    let stdout = assert_success(
        run_ncc_in_dir(
            &[
                "run".to_string(),
                "tool.n".to_string(),
                "--show-return".to_string(),
            ],
            &root,
        ),
        "ncc run bare filename must search src",
    );
    assert!(stdout.contains("bare-src-ok"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn priority13b_run_accepts_windows_drive_path_from_wsl() {
    let source = workspace_root()
        .join("examples")
        .join("main_family_v2_0_0")
        .join("main_ref_int_ok.n");
    let Some(windows_path) = windows_drive_path_from_wsl(&source) else {
        return;
    };

    let stdout = assert_success(
        run_ncc(&["run".to_string(), windows_path, "--show-return".to_string()]),
        "ncc run from WSL must accept C:\\ style source paths",
    );
    assert!(stdout.contains("[return: 7 profile="), "{stdout}");
}

#[test]
fn priority13b_single_file_import_uses_configurable_project_path() {
    let root = temp_project("priority13b_single_file_config_path_root");
    let external = temp_project("priority13b_single_file_config_path_external");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&external);
    fs::create_dir_all(&root).expect("config root dir");
    fs::create_dir_all(&external).expect("config external dir");
    fs::write(
        root.join("main.n"),
        "use <project:shared>;\n\nmainApp {\n    ref main ->> @fn(shared.sharedUnit.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write config main");
    fs::write(
        external.join("shared.n"),
        "use::stoud::io::<output>;\n\nsharedUnit {\n    fn:\n        public:\n            boot() -> bool {\n                println(\"external-project-path-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write external shared");

    let stdout = assert_success(
        run_ncc_with_env(
            &[
                "run".to_string(),
                root.join("main.n").to_string_lossy().to_string(),
                "--show-return".to_string(),
            ],
            &[("NCC_PROJECT_PATH", &external)],
        ),
        "implicit graph must search NCC_PROJECT_PATH",
    );
    assert!(stdout.contains("external-project-path-ok"), "{stdout}");
}

#[test]
fn priority13b_single_file_import_cycle_reports_explicit_chain() {
    let root = temp_project("priority13b_single_file_cycle");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("cycle dirs");
    fs::write(
        root.join("main.n"),
        "use <project:a>;\n\nmainApp { fa: return true; }\n",
    )
    .expect("write cycle main");
    fs::write(
        root.join("src").join("a.n"),
        "use <project:b>;\n\naUnit { fa: return true; }\n",
    )
    .expect("write cycle a");
    fs::write(
        root.join("src").join("b.n"),
        "use <project:a>;\n\nbUnit { fa: return true; }\n",
    )
    .expect("write cycle b");

    let rendered = assert_failure_contains(
        run_ncc(&[
            "run".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
        ]),
        "implicit graph cycle must fail clearly",
        "E_SINGLE_FILE_IMPORT_CYCLE_001",
    );
    assert!(rendered.contains("a -> b -> a"), "{rendered}");
}

#[test]
fn priority13b_single_file_import_ambiguity_reports_candidates() {
    let root = temp_project("priority13b_single_file_ambiguous_import");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src").join("one")).expect("ambiguous one");
    fs::create_dir_all(root.join("src").join("two")).expect("ambiguous two");
    fs::write(
        root.join("main.n"),
        "use <project:shared>;\n\nmainApp { fa: return true; }\n",
    )
    .expect("write ambiguous main");
    for folder in ["one", "two"] {
        fs::write(
            root.join("src").join(folder).join("shared.n"),
            "sharedUnit { fa: return true; }\n",
        )
        .expect("write ambiguous shared");
    }

    let rendered = assert_failure_contains(
        run_ncc(&[
            "run".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
        ]),
        "ambiguous implicit import must fail clearly",
        "E_SINGLE_FILE_IMPORT_002",
    );
    assert!(rendered.contains("2 candidates"), "{rendered}");
    assert!(rendered.contains("src/one/shared.n"), "{rendered}");
    assert!(rendered.contains("src/two/shared.n"), "{rendered}");
}

#[test]
fn priority13b_single_file_workspace_materializes_ast_cache_for_each_hash() {
    let root = temp_project("priority13b_single_file_ast_hash_cache");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("cache dirs");
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\ncacheMain {\n    ref main ->> @fn(worker.workerUnit.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write cache main");
    fs::write(
        root.join("src").join("worker.n"),
        "use::stoud::io::<output>;\n\nworkerUnit {\n    fn:\n        public:\n            boot() -> bool {\n                println(\"single-file-cache-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write cache worker");

    for label in ["first", "second"] {
        let stdout = assert_success(
            run_ncc(&[
                "run".to_string(),
                root.join("main.n").to_string_lossy().to_string(),
                "--show-return".to_string(),
            ]),
            &format!("{label} implicit graph cache run"),
        );
        assert!(stdout.contains("single-file-cache-ok"), "{stdout}");
    }
    let cache_dir = root.join(".ncc").join("cache").join("ast");
    let files = fs::read_dir(&cache_dir)
        .expect("cache dir exists")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        files.iter().any(|name| name.starts_with("main-")),
        "main hash cache missing: {files:?}"
    );
    assert!(
        files.iter().any(|name| name.starts_with("worker-")),
        "worker hash cache missing: {files:?}"
    );
}

#[test]
fn priority13b_concurrent_project_cache_writes_are_locked_and_atomic() {
    let root = temp_project("priority13b_concurrent_cache_writes");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("main.n"),
        "mainConcurrent {\n    fn:\n        public:\n            run() -> bool {\n                return true;\n            }\n\n    ref main ->> @fn(run);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write main");

    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "initial sync before concurrent checks",
    );

    let args = [
        "project".to_string(),
        "check".to_string(),
        root.to_string_lossy().to_string(),
    ];
    let first = Command::new(ncc_bin())
        .args(&args)
        .env("NC_NO_COLOR", "1")
        .env("NCC_CACHE_LOCK_TIMEOUT_MS", "10000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn first concurrent ncc");
    let second = Command::new(ncc_bin())
        .args(&args)
        .env("NC_NO_COLOR", "1")
        .env("NCC_CACHE_LOCK_TIMEOUT_MS", "10000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn second concurrent ncc");

    let first_out = first.wait_with_output().expect("wait first concurrent ncc");
    let second_out = second
        .wait_with_output()
        .expect("wait second concurrent ncc");
    assert_success(first_out, "first concurrent project check");
    assert_success(second_out, "second concurrent project check");

    assert!(
        !root
            .join(".ncc")
            .join("locks")
            .join("project-cache.lock")
            .exists(),
        "cache lock directory must be released after concurrent checks"
    );
    assert!(
        root.join(".ncc")
            .join("locks")
            .join("last_report.json")
            .is_file(),
        "cache lock report must be materialized"
    );
    assert_json_tree_is_valid(&root);
}

#[test]
fn priority13_incremental_project_run_reuses_ast_cache_on_second_run() {
    let root = temp_project("priority13_incremental_ast_cache_run");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init ast cache",
    );
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\ncacheMain {\n    ref main ->> @fn(worker.workerMain.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write ast cache main");
    fs::write(
        root.join("src").join("worker.n"),
        "use::stoud::io::<output>;\n\nworkerMain {\n    fn:\n        public:\n            boot() -> bool {\n                println(\"ast-cache-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write ast cache worker");
    sync_temp_project(&root, "project sync ast cache");

    let first = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "first project run writes ast cache",
    );
    assert!(first.contains("ast-cache-ok"), "{first}");
    assert!(
        first.contains("route=project-bridge-interpreter"),
        "{first}"
    );
    let cache_dir = root.join(".ncc").join("cache").join("ast");
    assert!(cache_dir.is_dir(), "AST cache dir must exist");
    let cache_files = fs::read_dir(&cache_dir)
        .expect("read ast cache dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|v| v.to_str()) == Some("json"))
        .count();
    assert!(
        cache_files >= 2,
        "main and worker AST caches must be materialized, found {cache_files}"
    );

    let second = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "second project run reuses ast cache",
    );
    assert!(second.contains("ast-cache-ok"), "{second}");
    assert!(
        second.contains("ast_cache_hits=2"),
        "second run must reuse both cached ASTs\n{second}"
    );
    let route = json_file(&root.join(".ncc").join("run").join("last.json"));
    assert_eq!(
        route.get("route").and_then(|v| v.as_str()),
        Some("project-bridge-interpreter")
    );
}

#[test]
fn priority13_incremental_detects_missing_unit_lowering_as_stale() {
    let root = temp_project("priority13_incremental_lowering_stale");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init lowering stale",
    );
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\nloweringMain {\n    ref main ->> @fn(worker.workerMain.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write lowering stale main");
    fs::write(
        root.join("src").join("worker.n"),
        "workerMain {\n    fn:\n        public:\n            boot() -> bool {\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write lowering stale worker");
    sync_temp_project(&root, "project sync lowering stale");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project check writes lowering stale fixture artifacts",
    );
    fs::remove_file(
        root.join(".ncc")
            .join("lowering")
            .join("worker.lowering.json"),
    )
    .expect("remove worker lowering");

    let report = assert_success(
        run_ncc(&[
            "project".to_string(),
            "incremental".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project incremental detects missing lowering",
    );
    let payload: serde_json::Value =
        serde_json::from_str(&report).expect("incremental report must be JSON");
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("rebuild_required")
    );
    assert!(payload
        .pointer("/reason_by_unit/worker")
        .and_then(|v| v.as_array())
        .is_some_and(|reasons| reasons.iter().any(|reason| reason
            .as_str()
            .is_some_and(|text| text.contains("lowering_status=missing")))));
}

#[test]
fn priority13b_single_file_check_writes_code_json_for_plain_file() {
    let root = temp_project("priority13b_single_file_check_plain");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("single-file check dir");
    fs::write(
        root.join("solo.n"),
        "solo {\n    fn:\n        public:\n            boot() -> bool {\n                return true;\n            }\n\n    ref main ->> @fn(boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write standalone solo file");

    let stdout = assert_success(
        run_ncc(&[
            "check".to_string(),
            root.join("solo.n").to_string_lossy().to_string(),
        ]),
        "single-file check must pass",
    );
    assert!(stdout.contains("OK:"), "{stdout}");
    let code = json_file(&root.join(".ncc").join("code").join("solo.json"));
    assert_eq!(
        code.get("schema").and_then(|v| v.as_str()),
        Some("ncc.code.v1")
    );
    assert_eq!(code.get("unit").and_then(|v| v.as_str()), Some("solo"));
    assert_eq!(code.get("source").and_then(|v| v.as_str()), Some("solo.n"));
    assert!(
        code.get("hash")
            .and_then(|v| v.as_str())
            .is_some_and(|hash| hash.starts_with("sha256:")),
        "single-file code identity must store source hash: {code}"
    );
}

#[test]
fn priority13b_project_run_skips_physical_object_hashing_for_runtime_speed() {
    let root = temp_project("priority13b_project_run_runtime_fast_no_object_hash");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init runtime fast bridge",
    );
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\nfastMain {\n    ref main ->> @fn(workerCore.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write runtime fast main");
    fs::write(
        root.join("src").join("worker.n"),
        "use::stoud::io::<output>;\n\nworkerCore {\n    fn:\n        public:\n            boot() -> bool {\n                println(\"runtime-fast-ok\");\n                return true;\n            }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write runtime fast child");
    sync_temp_project(&root, "project sync runtime fast bridge");

    let worker = bridge_unit(&root, "worker");
    let object_rel = worker
        .get("object")
        .and_then(|v| v.as_str())
        .expect("worker object path");
    fs::write(
        root.join(object_rel),
        b"corrupted-object-never-read-by-project-run",
    )
    .expect("corrupt worker object");

    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "project run must not hash physical objects on runtime path",
    );
    assert!(stdout.contains("runtime-fast-ok"), "{stdout}");
    assert!(stdout.contains("[return: true profile="), "{stdout}");
}

#[test]
fn priority132a_project_graph_check_run_write_real_objects_for_main_and_children() {
    let root = temp_project("priority132a_project_real_objects");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init real objects",
    );
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\n\nmainObj {\n    ref main ->> @fn(mainObj.entry);\n\n    fn:\n        entry() -> bool {\n            public:\n                memoria.memoriaUnit.print();\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write object main");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit {\n    fn:\n        print() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write object child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync real objects",
    );

    let graph_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "graph".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project graph real objects",
    );
    let graph: serde_json::Value = serde_json::from_str(&graph_stdout).expect("graph json");
    assert_eq!(
        graph.get("object_dir").and_then(|v| v.as_str()),
        Some(".ncc/obj")
    );

    let main_object = assert_project_object_contract(&root, "main", "mainObj.entry");
    assert!(main_object
        .pointer("/imports/project_units")
        .and_then(|v| v.as_array())
        .is_some_and(|imports| imports
            .iter()
            .any(|import| import.as_str() == Some("memoria"))));
    assert_project_object_contract(&root, "memoria", "memoriaUnit.print");

    let check_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project check real objects",
    );
    let check: serde_json::Value = serde_json::from_str(&check_stdout).expect("check json");
    assert_eq!(
        check.get("object_artifacts").and_then(|v| v.as_u64()),
        Some(2)
    );
    let run_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "project run real objects",
    );
    assert!(
        run_stdout.contains("[return: true profile="),
        "{run_stdout}"
    );
    assert_project_object_contract(&root, "main", "mainObj.entry");
    assert_project_object_contract(&root, "memoria", "memoriaUnit.print");
}

#[test]
fn priority132a_project_check_emits_real_objects_for_os_arch_targets() {
    let root = temp_project("priority132a_project_cross_target_objects");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init cross targets",
    );
    fs::write(
        root.join("main.n"),
        "crossMain {\n    ref main ->> @fn(crossMain.entry);\n\n    fn:\n        entry() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write cross target main");
    sync_temp_project(&root, "project sync cross targets");

    for (selector, triple, format) in [
        ("nC.win64", "x86_64-pc-windows-msvc", "coff"),
        ("nC.aapcs64", "aarch64-apple-darwin", "mach-o"),
        (
            "aarch64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "elf64",
        ),
    ] {
        let stdout = assert_success(
            run_ncc(&[
                "project".to_string(),
                "check".to_string(),
                root.to_string_lossy().to_string(),
                "--cc".to_string(),
                selector.to_string(),
                "--json".to_string(),
            ]),
            "project check cross target",
        );
        let report: serde_json::Value =
            serde_json::from_str(&stdout).expect("cross target report json");
        assert_eq!(
            report
                .pointer("/target/target_triple")
                .and_then(|v| v.as_str()),
            Some(triple),
            "{stdout}"
        );
        assert_eq!(
            report
                .pointer("/target/object_format")
                .and_then(|v| v.as_str()),
            Some(format),
            "{stdout}"
        );

        let unit = bridge_unit(&root, "main");
        let object_rel = unit
            .get("object")
            .and_then(|v| v.as_str())
            .expect("target object path");
        assert!(
            object_rel.contains(&triple.replace('-', "_")),
            "object path must be target-scoped: {object_rel}"
        );
        let metadata = json_file(&root.join(format!("{object_rel}.json")));
        assert_eq!(
            metadata
                .pointer("/abi/object_format")
                .and_then(|v| v.as_str()),
            Some(format)
        );
        assert_eq!(
            metadata.pointer("/abi/triple").and_then(|v| v.as_str()),
            Some(triple)
        );
        let bytes = fs::read(root.join(object_rel)).expect("target object bytes");
        match format {
            "elf64" => assert!(bytes.starts_with(&[0x7f, b'E', b'L', b'F'])),
            "coff" => assert!(bytes.len() >= 20),
            "mach-o" => assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])),
            _ => unreachable!(),
        }
    }
}

#[test]
fn priority132b_project_link_builds_standalone_executable_from_bridge_objects() {
    if !host_cc_available() {
        eprintln!("skipping project physical link test: host cc is not available");
        return;
    }
    let root = temp_project("priority132b_project_physical_link");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init physical link",
    );
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\n\nmainObj {\n    ref main ->> @fn(mainObj.entry);\n\n    fn:\n        entry() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write link main");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write link child");
    sync_temp_project(&root, "project sync physical link");

    let out_path = root.join(".ncc").join("bin").join("linked-main");
    let link_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "link".to_string(),
            root.to_string_lossy().to_string(),
            "--out".to_string(),
            out_path.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project physical link",
    );
    let report: serde_json::Value = serde_json::from_str(&link_stdout).expect("link json");
    assert_eq!(
        report.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.physical_link.v1")
    );
    assert_eq!(report.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(report.get("object_count").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(
        report.pointer("/entry/symbol").and_then(|v| v.as_str()),
        Some("mainObj.entry")
    );
    assert!(out_path.exists(), "linked executable should exist");
    let run = Command::new(&out_path)
        .output()
        .unwrap_or_else(|err| panic!("run linked executable {}: {err}", out_path.display()));
    assert!(
        run.status.success(),
        "linked executable should map nC true to process success\nstdout={}\nstderr={}",
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
}

#[test]
fn project_link_exports_qualified_read_int_runtime_symbol_for_child_entry() {
    if !host_cc_available() {
        eprintln!("skipping project runtime ABI link test: host cc is not available");
        return;
    }
    let root = temp_project("project_link_qualified_read_int");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create project src");
    fs::write(
        root.join("main.n"),
        "use <project:worker>;\n\nmainApp { ref main ->> @fn(worker.workerUnit.run); fa: return true; }\n",
    )
    .expect("write main");
    fs::write(
        root.join("src").join("worker.n"),
        "use::stoud::io::<input, output>;\n\nworkerUnit { stc: value >> int = 0; stop >> int = 1; fn: public: run() -> bool { value = readInt(); println(value + 1); stop = readInt(); return stop == 0; } fa: return true; }\n",
    )
    .expect("write worker");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init runtime ABI",
    );
    sync_temp_project(&root, "project sync runtime ABI");
    let executable = root.join(".ncc").join("bin").join("runtime-input");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "link".to_string(),
            root.to_string_lossy().to_string(),
            "--out".to_string(),
            executable.to_string_lossy().to_string(),
        ]),
        "project link runtime ABI",
    );
    let mut child = Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn linked runtime executable");
    let mut stdin = child.stdin.take().expect("runtime stdin");
    let stdout = child.stdout.take().expect("runtime stdout");
    stdin.write_all(b"41\n").expect("write runtime input");
    stdin.flush().expect("flush runtime input");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });
    let line = receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("runtime output must flush while process remains alive")
        .expect("read runtime output");
    assert_eq!(line.trim(), "42");
    stdin.write_all(b"0\n").expect("write runtime stop");
    stdin.flush().expect("flush runtime stop");
    drop(stdin);
    let status = child.wait().expect("wait runtime executable");
    assert!(status.success(), "runtime executable must succeed");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn priority13b_project_check_rejects_stale_code_identity_hash() {
    let root = temp_project("priority13b_project_stale_hash");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init stale hash",
    );
    fs::write(
        root.join("main.n"),
        "use <project:memoria>;\nmainApp { fa: return true; }\n",
    )
    .expect("write stale main");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit { fa: return true; }\n",
    )
    .expect("write stale child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync stale hash",
    );
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit { fn: ping() -> bool { public: return true; } fa: return true; }\n",
    )
    .expect("mutate child after sync");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project stale hash must fail",
        "E_PROJECT_CODE_IDENTITY_002",
    );
}

#[test]
fn priority13d_project_check_rejects_stale_main_code_identity_hash() {
    let root = temp_project("priority13d_project_stale_main_hash");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init stale main hash",
    );
    fs::write(root.join("main.n"), "mainApp { fa: return true; }\n")
        .expect("write stale main initial");
    fs::write(
        root.join("src").join("memoria.n"),
        "memoriaUnit { fa: return true; }\n",
    )
    .expect("write stale main child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync stale main hash",
    );
    fs::write(
        root.join("main.n"),
        "mainApp { to main { println(\"changed\"); } fa: return true; }\n",
    )
    .expect("mutate main after sync");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project stale main hash must fail",
        "E_PROJECT_CODE_IDENTITY_002",
    );
}

#[test]
fn priority13f_project_doctor_reports_stale_code_identity_without_running() {
    let root = temp_project("priority13f_project_doctor_stale_identity");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init doctor stale identity",
    );
    fs::write(root.join("main.n"), "mainApp { fa: return true; }\n")
        .expect("write doctor stale main initial");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync doctor stale identity",
    );
    fs::write(
        root.join("main.n"),
        "mainApp { to main { println(\"doctor changed\"); } fa: return true; }\n",
    )
    .expect("mutate main before doctor");
    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "doctor".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project doctor stale identity",
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("doctor stale json");
    assert_eq!(report.get("status").and_then(|v| v.as_str()), Some("error"));
    assert!(report
        .get("identity_errors")
        .and_then(|v| v.as_array())
        .is_some_and(|errors| errors.iter().any(|error| error
            .as_str()
            .is_some_and(|text| text.contains("E_PROJECT_CODE_IDENTITY_002")))));
}

#[test]
fn priority13e_project_sync_rejects_unknown_project_import_target() {
    let root = temp_project("priority13e_project_unknown_import_target");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init unknown import target",
    );
    fs::write(
        root.join("main.n"),
        "use <project:ghost>;\n\nmainApp { fa: return true; }\n",
    )
    .expect("write unknown import main");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync unknown import target must fail",
        "E_PROJECT_BRIDGE_EDGE_003",
    );
}

#[test]
fn priority13b_project_cycles_and_duplicate_child_containers_check() {
    let root = temp_project("priority13b_project_cycles_duplicates");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init cycles duplicates",
    );
    fs::write(
        root.join("main.n"),
        "use <project:a, b>;\nmainApp { fa: return true; }\n",
    )
    .expect("write cycle main");
    fs::write(
        root.join("src").join("a.n"),
        "use <project:b>;\n\nsameName { fa: return true; }\n",
    )
    .expect("write cycle a");
    fs::write(
        root.join("src").join("b.n"),
        "use <project:a>;\n\nsameName { fa: return true; }\n",
    )
    .expect("write cycle b");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync cycles duplicates",
    );
    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "check".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project check cycles duplicates",
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("project check json");
    assert_eq!(report.get("units").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(report.get("edges").and_then(|v| v.as_u64()), Some(4));
    assert_eq!(
        report.get("linked_containers").and_then(|v| v.as_u64()),
        Some(3)
    );
}

#[test]
fn priority13c_project_run_resolves_unit_qualified_duplicate_container() {
    let root = temp_project("priority13c_project_unit_qualified_duplicate");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init unit-qualified duplicate",
    );
    fs::write(
        root.join("main.n"),
        "use <project:a, b>;\n\nmainApp {\n    ref main ->> @fn(a.sameName.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unit-qualified main");
    fs::write(
        root.join("src").join("a.n"),
        "use::stoud::io::<output>;\n\nsameName {\n    fn:\n        boot() -> bool {\n            public:\n                println(\"unit-a-selected\");\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unit-qualified child a");
    fs::write(
        root.join("src").join("b.n"),
        "use::stoud::io::<output>;\n\nsameName {\n    fn:\n        boot() -> bool {\n            public:\n                println(\"unit-b-selected\");\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unit-qualified child b");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync unit-qualified duplicate",
    );
    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "project run unit-qualified duplicate",
    );
    assert!(stdout.contains("unit-a-selected"), "{stdout}");
    assert!(!stdout.contains("unit-b-selected"), "{stdout}");
    assert!(stdout.contains("[return: true profile="), "{stdout}");

    let linked_lowering = json_file(
        &root
            .join(".ncc")
            .join("lowering")
            .join("__linked.lowering.json"),
    );
    assert_eq!(
        linked_lowering
            .pointer("/project_linker/status")
            .and_then(|v| v.as_str()),
        Some("closed_unit_namespace_materialization")
    );
    let materialized = linked_lowering
        .pointer("/project_linker/materialized_containers")
        .and_then(|v| v.as_array())
        .expect("materialized containers array");
    let a_same_name = materialized
        .iter()
        .find(|item| {
            item.get("unit").and_then(|v| v.as_str()) == Some("a")
                && item.get("source_container").and_then(|v| v.as_str()) == Some("sameName")
        })
        .expect("a.sameName materialization");
    assert_eq!(
        a_same_name
            .get("materialized_container")
            .and_then(|v| v.as_str()),
        Some("a__sameName")
    );
    assert_eq!(
        a_same_name
            .get("duplicate_source_name")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(a_same_name
        .get("materialized_container_bytes_hex")
        .and_then(|v| v.as_array())
        .is_some_and(|bytes| bytes.iter().any(|b| b.as_str() == Some("5f"))));

    let graph_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "graph".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project graph unit-qualified duplicate",
    );
    let graph: serde_json::Value = serde_json::from_str(&graph_stdout).expect("project graph json");
    assert_eq!(
        graph.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.priority13g.graph.v1")
    );
    assert!(graph
        .pointer("/project_linker/materialized_containers")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items.iter().any(|item| {
            item.get("unit_qualified_prefix").and_then(|v| v.as_str()) == Some("a.sameName")
                && item.get("materialized_container").and_then(|v| v.as_str())
                    == Some("a__sameName")
        })));
}

#[test]
fn priority13c_project_run_resolves_unit_qualified_paths_in_weird_contexts() {
    let root = init_temp_project("priority13c_project_unit_qualified_weird_paths");
    copy_project_fixture(&root, "main.n", "unit_qualified_paths_weird_main.n");
    copy_project_fixture(&root, "src/alpha.n", "unit_qualified_paths_weird_alpha.n");
    copy_project_fixture(&root, "src/beta.n", "unit_qualified_paths_weird_beta.n");
    sync_temp_project(&root, "project sync unit-qualified weird paths");

    let stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "project run unit-qualified weird paths",
    );
    for expected in [
        "normal-call-ok",
        "if-call-ok",
        "if-body-ok",
        "while-call-ok",
        "for-call-ok",
        "match-case-ok",
        "guard-call-ok",
        "defer-call-ok",
        "[return: true profile=",
    ] {
        assert!(stdout.contains(expected), "missing {expected:?}\n{stdout}");
    }
    assert!(!stdout.contains("beta-unexpected"), "{stdout}");

    let graph_stdout = assert_success(
        run_ncc(&[
            "project".to_string(),
            "graph".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "project graph unit-qualified weird paths",
    );
    let graph: serde_json::Value = serde_json::from_str(&graph_stdout).expect("graph json");
    let materialized = graph
        .pointer("/project_linker/materialized_containers")
        .and_then(|v| v.as_array())
        .expect("materialized containers");
    assert!(materialized.iter().any(|item| {
        item.get("unit_qualified_prefix").and_then(|v| v.as_str()) == Some("alpha.sameName")
            && item.get("materialized_container").and_then(|v| v.as_str())
                == Some("alpha__sameName")
            && item.get("duplicate_source_name").and_then(|v| v.as_bool()) == Some(true)
    }));
}

#[test]
fn priority13e_project_run_rejects_unknown_unit_in_qualified_call_path() {
    let root = init_temp_project("priority13e_project_unknown_unit_call_path");
    fs::write(
        root.join("main.n"),
        "use <project:alpha>;\n\nunknownUnitMain {\n    ref main ->> @fn(unknownUnitMain.boot);\n\n    fn:\n        boot() -> bool {\n            public:\n                ghost.sameName.boot();\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown unit main");
    fs::write(
        root.join("src").join("alpha.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown unit child");
    sync_temp_project(&root, "project sync unknown unit call path");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unknown unit call path must fail",
        "unresolved function path 'ghost.sameName.boot'",
    );
}

#[test]
fn priority13e_project_run_rejects_unknown_unit_qualified_container_in_normal_call() {
    let root = init_temp_project("priority13e_project_unknown_container_call_path");
    fs::write(
        root.join("main.n"),
        "use <project:alpha>;\n\nunknownContainerMain {\n    ref main ->> @fn(unknownContainerMain.boot);\n\n    fn:\n        boot() -> bool {\n            public:\n                alpha.missing.boot();\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown container normal call main");
    fs::write(
        root.join("src").join("alpha.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown container normal call child");
    sync_temp_project(&root, "project sync unknown container normal call");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unknown container normal call must fail",
        "E_PROJECT_UNIT_QUALIFIED_001",
    );
}

#[test]
fn priority13e_project_run_rejects_unknown_unit_qualified_container_in_const_and_contract() {
    let root = init_temp_project("priority13e_project_unknown_container_const_contract");
    fs::write(
        root.join("main.n"),
        "use <project:alpha>;\n\nconstContractMain {\n    ref main ->> @fn(constContractMain.boot);\n\n    const:\n        constProbe: bool = alpha.missing.constProbe();\n\n    contract:\n        LinkReady {\n        require alpha.missing.contractProbe();\n    }\n\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write const and contract unknown container main");
    fs::write(
        root.join("src").join("alpha.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write const and contract unknown container child");
    sync_temp_project(&root, "project sync unknown container const contract");
    let rendered = assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unknown container in const and contract must fail",
        "E_PROJECT_UNIT_QUALIFIED_001",
    );
    assert!(rendered.contains("alpha.missing.constProbe"), "{rendered}");
    assert!(
        rendered.contains("alpha.missing.contractProbe"),
        "{rendered}"
    );
}

#[test]
fn priority13e_project_run_rejects_unknown_unit_qualified_function_after_materialization() {
    let root = init_temp_project("priority13e_project_unknown_function_materialized");
    fs::write(
        root.join("main.n"),
        "use <project:alpha, beta>;\n\nunknownFunctionMain {\n    ref main ->> @fn(alpha.sameName.missing);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown function main");
    fs::write(
        root.join("src").join("alpha.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown function alpha");
    fs::write(
        root.join("src").join("beta.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown function beta");
    sync_temp_project(&root, "project sync unknown function materialized");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unknown function must fail",
        "function 'alpha__sameName.missing' not found",
    );
}

#[test]
fn priority13e_project_run_rejects_unqualified_duplicate_container_collision() {
    let root = init_temp_project("priority13e_project_unqualified_duplicate_collision");
    copy_project_fixture(
        &root,
        "main.n",
        "unit_qualified_collision_unqualified_main_fail.n",
    );
    copy_project_fixture(&root, "src/alpha.n", "unit_qualified_paths_weird_alpha.n");
    copy_project_fixture(&root, "src/beta.n", "unit_qualified_paths_weird_beta.n");
    sync_temp_project(&root, "project sync unqualified duplicate collision");
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unqualified duplicate collision must fail",
        "container 'sameName' not found",
    );
}

#[test]
fn priority13e_project_run_rejects_unknown_unit_qualified_container() {
    let root = temp_project("priority13e_project_unknown_unit_container");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init unknown unit container",
    );
    fs::write(
        root.join("main.n"),
        "use <project:a>;\n\nmainApp {\n    ref main ->> @fn(a.missing.boot);\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown unit container main");
    fs::write(
        root.join("src").join("a.n"),
        "sameName {\n    fn:\n        boot() -> bool {\n            public:\n                return true;\n        }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("write unknown unit container child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync unknown unit container",
    );
    assert_failure_contains(
        run_ncc(&[
            "project".to_string(),
            "run".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project run unknown unit container must fail",
        "E_PROJECT_UNIT_QUALIFIED_001",
    );
}
