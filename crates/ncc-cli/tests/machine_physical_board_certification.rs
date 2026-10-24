use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const HMAC_ENV: &str = "NCC_TEST_BOARD_HMAC_KEY";
const HMAC_KEY: &str = "0123456789abcdef0123456789abcdef";

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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("ncc-board-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).expect("temp directory");
    path
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn run_ncc(args: &[String], hmac_key: Option<&str>) -> Output {
    let mut command = Command::new(ncc_bin());
    command
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .env("SOURCE_DATE_EPOCH", "1784318400");
    if let Some(key) = hmac_key {
        command.env(HMAC_ENV, key);
    } else {
        command.env_remove(HMAC_ENV);
    }
    command.output().expect("ncc executes")
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn build_helper(directory: &Path) -> PathBuf {
    let source = directory.join("board_helper.rs");
    let executable = directory.join(if cfg!(windows) {
        "board_helper.exe"
    } else {
        "board_helper"
    });
    fs::write(
        &source,
        r#"
use std::env;
use std::fs;
use std::process;

fn main() {
    if env::var_os("NCC_TEST_BOARD_HMAC_KEY").is_some() {
        eprintln!("HMAC_SECRET_LEAKED_TO_CHILD");
        process::exit(91);
    }
    let args = env::args().skip(1).collect::<Vec<_>>();
    let stage = args.first().map(String::as_str).unwrap_or("");
    if stage == "unavailable" {
        eprintln!("physical probe unavailable");
        process::exit(7);
    }
    if stage == "spam" {
        println!("{}", "X".repeat(131072));
        process::exit(0);
    }
    if stage == "framebuffer" {
        let path = args.get(1).expect("framebuffer path");
        fs::write(path, vec![0x5a; 64]).expect("framebuffer write");
        println!("FRAMEBUFFER_CAPTURED");
        return;
    }
    if stage == "spoof" {
        println!("QEMU TCG emulated transport");
    }
    for item in args.iter().skip(1) {
        println!("{item}");
    }
}
"#,
    )
    .expect("helper source");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc)
        .arg(&source)
        .arg("-O")
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("rustc executes");
    assert!(
        output.status.success(),
        "helper compilation failed\nstdout={}\nstderr={}",
        normalize(&output.stdout),
        normalize(&output.stderr)
    );
    executable
}

fn board_manifest(
    helper: &Path,
    artifact: &Path,
    probe_stage: &str,
    capture_stage: &str,
    ordered_capture: bool,
    max_log_bytes: u64,
) -> Value {
    let capture = if ordered_capture {
        vec![
            "NCC_PHYSICAL_BOARD=1",
            "NCC_BOOT_START",
            "NCC_BOARD_ID=test-board-01",
            "NCC_BOARD_SERIAL=BOARD-SERIAL-0001",
            "NCC_TARGET=aarch64-unknown-none",
            "NCC_FIRMWARE_SHA256={artifact_sha256}",
            "NCC_BOOT_OK",
        ]
    } else {
        vec![
            "NCC_PHYSICAL_BOARD=1",
            "NCC_BOOT_OK",
            "NCC_BOARD_ID=test-board-01",
            "NCC_BOARD_SERIAL=BOARD-SERIAL-0001",
            "NCC_TARGET=aarch64-unknown-none",
            "NCC_FIRMWARE_SHA256={artifact_sha256}",
            "NCC_BOOT_START",
        ]
    };
    let mut capture_argv = vec![
        Value::String(helper.to_string_lossy().into_owned()),
        Value::String(capture_stage.to_string()),
    ];
    capture_argv.extend(capture.into_iter().map(Value::from));
    json!({
        "schema": "ncc.machine.physical_board_manifest.v1",
        "manifest_version": 1,
        "board": {
            "id": "test-board-01",
            "vendor": "Test Vendor",
            "model": "Test Board",
            "hardware_revision": "A1",
            "serial": "BOARD-SERIAL-0001",
            "architecture": "aarch64",
            "target": "aarch64-unknown-none",
        },
        "artifact": {
            "path": artifact,
            "sha256": "auto",
        },
        "transport": {
            "kind": "uart+jtag",
            "physical": true,
            "port": "TEST-PORT-01",
            "baud": 115200,
            "probe_serial": "PROBE-SERIAL-0001",
        },
        "limits": {
            "max_artifact_bytes": 1048576,
            "max_log_bytes": max_log_bytes,
            "max_timeout_ms": 10000,
        },
        "commands": {
            "probe": {
                "argv": [helper, probe_stage, "PROBE-SERIAL-0001", "PROBE_OK"],
                "timeout_ms": 5000,
                "success_exit_codes": [0],
            },
            "flash": {
                "argv": [helper, "flash", "FLASH_OK", "{artifact}", "{artifact_sha256}"],
                "timeout_ms": 5000,
                "success_exit_codes": [0],
            },
            "reset": {
                "argv": [helper, "reset", "RESET_OK"],
                "timeout_ms": 5000,
                "success_exit_codes": [0],
            },
            "capture": {
                "argv": capture_argv,
                "timeout_ms": 5000,
                "success_exit_codes": [0],
            },
        },
        "identity": {
            "probe_markers": ["PROBE-SERIAL-0001", "PROBE_OK"],
            "required_markers": ["NCC_BOOT_START", "NCC_BOOT_OK"],
            "ordered_markers": ["NCC_BOOT_START", "NCC_BOOT_OK"],
            "forbidden_markers": ["QEMU", "TCG", "EMULATOR", "SIMULATOR"],
        },
        "policy": {
            "require_clean_git": false,
            "reject_emulated_transport": true,
            "retain_full_command_logs_within_limits": true,
        },
    })
}

fn write_manifest(directory: &Path, value: &Value) -> PathBuf {
    fs::create_dir_all(directory).expect("manifest directory");
    let path = directory.join("board.json");
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).expect("manifest json")
        ),
    )
    .expect("manifest write");
    path
}

fn certify_args(manifest: &Path, bundle: &Path) -> Vec<String> {
    vec![
        "machine".into(),
        "certify-board".into(),
        "--manifest".into(),
        manifest.to_string_lossy().into_owned(),
        "--bundle".into(),
        bundle.to_string_lossy().into_owned(),
        "--signing-key-env".into(),
        HMAC_ENV.into(),
        "--json".into(),
    ]
}

fn verify_args(bundle: &Path) -> Vec<String> {
    vec![
        "machine".into(),
        "verify-board-certificate".into(),
        bundle.to_string_lossy().into_owned(),
        "--signing-key-env".into(),
        HMAC_ENV.into(),
        "--require-attestation".into(),
        "--json".into(),
    ]
}

#[test]
fn board_template_is_versioned_and_uses_structured_argv() {
    let output = run_ncc(
        &[
            "machine".into(),
            "board-template".into(),
            "--target".into(),
            "riscv32imac-unknown-elf".into(),
            "--json".into(),
        ],
        None,
    );
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        normalize(&output.stdout),
        normalize(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("template json");
    assert_eq!(value["schema"], "ncc.machine.physical_board_manifest.v1");
    assert_eq!(value["manifest_version"], 1);
    assert_eq!(value["board"]["architecture"], "riscv32");
    for command in ["probe", "flash", "reset", "capture"] {
        assert!(value["commands"][command]["argv"].is_array());
        assert!(value["commands"][command].get("shell").is_none());
        assert!(value["commands"][command].get("program").is_none());
    }
}

#[test]
fn signed_physical_bundle_verifies_and_detects_log_tampering() {
    let temp = unique_temp_dir("signed");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"NCC-PHYSICAL-ARTIFACT-V1").expect("artifact");
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "probe", "capture", true, 65536),
    );
    let bundle = temp.join("bundle");
    let certified = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(
        certified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&certified.stdout),
        normalize(&certified.stderr)
    );
    let certificate: Value = serde_json::from_slice(&certified.stdout).expect("certificate json");
    assert_eq!(certificate["status"], "certified");
    assert_eq!(certificate["physical_board"], true);
    assert_eq!(certificate["attestation"]["secret_persisted"], false);
    assert!(bundle.join("bundle.json").is_file());
    assert!(bundle.join("metadata/session.json").is_file());
    assert!(
        !normalize(&fs::read(bundle.join("logs/probe.stderr.log")).expect("probe stderr"))
            .contains("HMAC_SECRET_LEAKED_TO_CHILD")
    );

    let verified = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(
        verified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&verified.stdout),
        normalize(&verified.stderr)
    );
    let report: Value = serde_json::from_slice(&verified.stdout).expect("verification json");
    assert_eq!(report["status"], "verified");
    assert_eq!(report["attestation_ok"], true);

    let index_path = bundle.join("bundle.json");
    let original_index = fs::read(&index_path).expect("bundle index");
    let mut altered_index: Value =
        serde_json::from_slice(&original_index).expect("bundle index json");
    altered_index["file_count"] = json!(999_u64);
    fs::write(
        &index_path,
        serde_json::to_vec_pretty(&altered_index).expect("altered bundle index"),
    )
    .expect("tamper index");
    let bad_index = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(!bad_index.status.success());
    assert!(normalize(&bad_index.stdout).contains("bundle file_count mismatch"));
    fs::write(&index_path, &original_index).expect("restore bundle index");

    let mut altered_entry: Value =
        serde_json::from_slice(&original_index).expect("bundle index json");
    altered_entry["files"][0]["unexpected"] = json!(true);
    fs::write(
        &index_path,
        serde_json::to_vec_pretty(&altered_entry).expect("altered file entry"),
    )
    .expect("tamper file entry");
    let bad_entry = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(!bad_entry.status.success());
    assert!(normalize(&bad_entry.stdout).contains("file entry fields differ"));
    fs::write(&index_path, &original_index).expect("restore bundle index");

    fs::write(bundle.join("unexpected.txt"), b"not indexed").expect("extra bundle file");
    let extra_file = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(!extra_file.status.success());
    assert!(normalize(&extra_file.stdout).contains("bundle file set differs"));
    fs::remove_file(bundle.join("unexpected.txt")).expect("remove extra bundle file");

    fs::write(
        bundle.join("logs/capture.stdout.log"),
        b"NCC_BOOT_START\nTAMPERED\nNCC_BOOT_OK\n",
    )
    .expect("tamper log");
    let tampered = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(!tampered.status.success());
    assert!(
        normalize(&tampered.stdout).contains("file hash/size mismatch")
            || normalize(&tampered.stderr).contains("E_MACHINE_BOARD_CERTIFICATE_001")
    );
}

#[test]
fn physical_bundle_requires_and_signs_exact_framebuffer_evidence() {
    let temp = unique_temp_dir("framebuffer");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"NCC-FRAMEBUFFER-ARTIFACT-V1").expect("artifact");
    let mut value = board_manifest(&helper, &artifact, "probe", "capture", true, 65536);
    value["policy"]["require_framebuffer_capture"] = json!(true);
    value["framebuffer"] = json!({
        "width": 4,
        "height": 4,
        "stride": 16,
        "pixel_format": "xrgb8888",
    });
    value["commands"]["framebuffer"] = json!({
        "argv": [helper, "framebuffer", "{framebuffer_path}"],
        "timeout_ms": 5000,
        "success_exit_codes": [0],
    });
    let manifest = write_manifest(&temp, &value);
    let bundle = temp.join("bundle");
    let certified = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(
        certified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&certified.stdout),
        normalize(&certified.stderr)
    );
    let certificate: Value = serde_json::from_slice(&certified.stdout).expect("certificate json");
    assert_eq!(certificate["status"], "certified");
    assert_eq!(certificate["framebuffer"]["required"], true);
    assert_eq!(certificate["framebuffer"]["captured"], true);
    assert_eq!(certificate["framebuffer"]["bytes"], 64);
    assert_eq!(
        certificate["framebuffer"]["sha256"],
        sha256(&vec![0x5a; 64])
    );
    assert_eq!(
        fs::metadata(bundle.join("evidence/framebuffer.raw"))
            .expect("framebuffer evidence")
            .len(),
        64
    );

    let verified = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(
        verified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&verified.stdout),
        normalize(&verified.stderr)
    );
}

#[test]
fn absent_probe_is_validated_not_executed_and_never_certified() {
    let temp = unique_temp_dir("absent");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"NO-BOARD-ARTIFACT").expect("artifact");
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "unavailable", "capture", true, 65536),
    );
    let bundle = temp.join("bundle");
    let output = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        normalize(&output.stdout),
        normalize(&output.stderr)
    );
    let certificate: Value = serde_json::from_slice(&output.stdout).expect("certificate json");
    assert_eq!(certificate["status"], "validated_not_executed");
    assert_eq!(certificate["physical_board"], false);
    assert_eq!(
        certificate["execution_class"],
        "physical_hardware_unavailable"
    );
    assert!(bundle.join("logs/probe.stderr.log").is_file());
    assert!(!bundle.join("logs/flash.stdout.log").exists());
    assert!(!bundle.join("logs/capture.stdout.log").exists());
    let verified = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(
        verified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&verified.stdout),
        normalize(&verified.stderr)
    );
}

#[test]
fn validate_only_is_distinct_from_unavailable_physical_hardware() {
    let temp = unique_temp_dir("validate-only");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"VALIDATION-ONLY-ARTIFACT").expect("artifact");
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "probe", "capture", true, 65536),
    );
    let bundle = temp.join("bundle");
    let mut args = certify_args(&manifest, &bundle);
    args.insert(args.len() - 1, "--validate-only".into());
    let output = run_ncc(&args, Some(HMAC_KEY));
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        normalize(&output.stdout),
        normalize(&output.stderr)
    );
    let certificate: Value = serde_json::from_slice(&output.stdout).expect("certificate json");
    assert_eq!(certificate["status"], "validated_not_executed");
    assert_eq!(certificate["physical_board"], false);
    assert_eq!(certificate["execution_class"], "validation_only");
    assert_eq!(
        certificate["execution"]["not_executed_reason"],
        "validation_only_requested"
    );
    assert!(!bundle.join("logs/probe.stdout.log").exists());
    let verified = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(
        verified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&verified.stdout),
        normalize(&verified.stderr)
    );
}

#[test]
fn emulator_argv_and_missing_physical_identity_are_rejected() {
    let temp = unique_temp_dir("manifest-negative");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"NEGATIVE-ARTIFACT").expect("artifact");
    let mut qemu = board_manifest(&helper, &artifact, "probe", "capture", true, 65536);
    qemu["commands"]["capture"]["argv"][0] = Value::String("qemu-system-aarch64".into());
    let manifest = write_manifest(&temp, &qemu);
    let output = run_ncc(
        &[
            "machine".into(),
            "certify-board".into(),
            "--manifest".into(),
            manifest.to_string_lossy().into_owned(),
            "--bundle".into(),
            temp.join("qemu-bundle").to_string_lossy().into_owned(),
            "--validate-only".into(),
            "--json".into(),
        ],
        None,
    );
    assert!(!output.status.success());
    assert!(normalize(&output.stderr).contains("E_MACHINE_BOARD_EMULATOR_001"));

    let mut missing_serial = board_manifest(&helper, &artifact, "probe", "capture", true, 65536);
    missing_serial["board"]
        .as_object_mut()
        .expect("board object")
        .remove("serial");
    let missing_manifest = write_manifest(&temp.join("missing"), &missing_serial);
    let missing = run_ncc(
        &[
            "machine".into(),
            "certify-board".into(),
            "--manifest".into(),
            missing_manifest.to_string_lossy().into_owned(),
            "--bundle".into(),
            temp.join("missing-bundle").to_string_lossy().into_owned(),
            "--validate-only".into(),
        ],
        None,
    );
    assert!(!missing.status.success());
    assert!(normalize(&missing.stderr).contains("E_MACHINE_BOARD_MANIFEST_001"));

    let mut implicit_shell = board_manifest(&helper, &artifact, "probe", "capture", true, 65536);
    implicit_shell["commands"]["capture"]["argv"][0] = Value::String("capture.cmd".into());
    let shell_manifest = write_manifest(&temp.join("shell"), &implicit_shell);
    let shell = run_ncc(
        &[
            "machine".into(),
            "certify-board".into(),
            "--manifest".into(),
            shell_manifest.to_string_lossy().into_owned(),
            "--bundle".into(),
            temp.join("shell-bundle").to_string_lossy().into_owned(),
            "--validate-only".into(),
        ],
        None,
    );
    assert!(!shell.status.success());
    assert!(normalize(&shell.stderr).contains("implicit Windows command shell"));

    let mut unsafe_timeout = board_manifest(&helper, &artifact, "probe", "capture", true, 65536);
    unsafe_timeout["commands"]["flash"]["allow_timeout"] = Value::Bool(true);
    let timeout_manifest = write_manifest(&temp.join("timeout"), &unsafe_timeout);
    let timeout = run_ncc(
        &[
            "machine".into(),
            "certify-board".into(),
            "--manifest".into(),
            timeout_manifest.to_string_lossy().into_owned(),
            "--bundle".into(),
            temp.join("timeout-bundle").to_string_lossy().into_owned(),
            "--validate-only".into(),
        ],
        None,
    );
    assert!(!timeout.status.success());
    assert!(normalize(&timeout.stderr).contains("only capture"));
}

#[test]
fn runtime_emulator_evidence_is_rejected_but_the_rejection_bundle_verifies() {
    let temp = unique_temp_dir("runtime-reject");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"RUNTIME-REJECTION-EVIDENCE").expect("artifact");
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "spoof", "capture", true, 65536),
    );
    let bundle = temp.join("bundle");
    let rejected = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(!rejected.status.success());
    let certificate: Value =
        serde_json::from_slice(&rejected.stdout).expect("rejection certificate");
    assert_eq!(certificate["status"], "rejected_emulator");
    assert_eq!(certificate["execution_class"], "emulator_rejected");
    assert_eq!(certificate["physical_board"], false);

    let verified = run_ncc(&verify_args(&bundle), Some(HMAC_KEY));
    assert!(
        verified.status.success(),
        "stdout={}\nstderr={}",
        normalize(&verified.stdout),
        normalize(&verified.stderr)
    );
    let report: Value = serde_json::from_slice(&verified.stdout).expect("verification report");
    assert_eq!(report["status"], "verified");
    assert_eq!(report["certificate_status"], "rejected_emulator");
    assert_eq!(report["physical_board_claim"], false);
}

#[test]
fn marker_order_and_log_limits_prevent_certification() {
    let temp = unique_temp_dir("negative-runtime");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    fs::write(&artifact, b"ORDER-ARTIFACT").expect("artifact");
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "probe", "capture", false, 65536),
    );
    let bundle = temp.join("order-bundle");
    let output = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(!output.status.success());
    let certificate: Value = serde_json::from_slice(&output.stdout).expect("failure certificate");
    assert_eq!(certificate["status"], "certification_failed");
    assert_eq!(certificate["identity"]["ordered_markers_present"], false);
    assert_eq!(certificate["physical_board"], false);

    let log_dir = temp.join("log-limit");
    fs::create_dir_all(&log_dir).expect("log-limit dir");
    let limited_manifest = write_manifest(
        &log_dir,
        &board_manifest(&helper, &artifact, "probe", "spam", true, 1024),
    );
    let limited_bundle = log_dir.join("bundle");
    let limited = run_ncc(
        &certify_args(&limited_manifest, &limited_bundle),
        Some(HMAC_KEY),
    );
    assert!(!limited.status.success());
    let limited_certificate: Value =
        serde_json::from_slice(&limited.stdout).expect("limited certificate");
    assert_eq!(limited_certificate["status"], "certification_failed");
    let session: Value = serde_json::from_slice(
        &fs::read(limited_bundle.join("metadata/session.json")).expect("session"),
    )
    .expect("session json");
    assert!(session["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .any(|command| command["log_limit_exceeded"] == true));
    assert!(
        fs::metadata(limited_bundle.join("logs/capture.stdout.log"))
            .expect("limited log")
            .len()
            <= 1024
    );
}

#[test]
fn wrong_or_missing_hmac_key_cannot_verify_signed_bundle() {
    let temp = unique_temp_dir("hmac-negative");
    let helper = build_helper(&temp);
    let artifact = temp.join("kernel.elf");
    let artifact_bytes = b"HMAC-ARTIFACT";
    fs::write(&artifact, artifact_bytes).expect("artifact");
    assert_eq!(
        sha256(artifact_bytes),
        sha256(&fs::read(&artifact).expect("artifact read"))
    );
    let manifest = write_manifest(
        &temp,
        &board_manifest(&helper, &artifact, "probe", "capture", true, 65536),
    );
    let bundle = temp.join("bundle");
    let output = run_ncc(&certify_args(&manifest, &bundle), Some(HMAC_KEY));
    assert!(output.status.success());

    let wrong = run_ncc(
        &verify_args(&bundle),
        Some("fedcba9876543210fedcba9876543210"),
    );
    assert!(!wrong.status.success());
    let missing = run_ncc(&verify_args(&bundle), None);
    assert!(!missing.status.success());
    assert!(normalize(&missing.stderr).contains("E_MACHINE_BOARD_ATTESTATION_001"));
}
