use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

static BASAL_EVIDENCE_CLI_LOCK: Mutex<()> = Mutex::new(());

fn basal_evidence_cli_lock() -> MutexGuard<'static, ()> {
    BASAL_EVIDENCE_CLI_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let public = root.join(relative);
    if public.exists() {
        return public;
    }
    if let Some(suffix) = relative.strip_prefix("examples/basal/") {
        let archived = root.join("_archive/old_examples/basal").join(suffix);
        if archived.exists() {
            return archived;
        }
    }
    public
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_ncc_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(ncc_bin());
    command
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_basal_ok(rel: &str) -> String {
    let path = fixture_path(rel);
    let out = run_ncc(&["basal", "check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "basal check must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn assert_basal_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&["basal", "check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "basal check must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "basal check for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_basal_verify_lowering_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&[
        "basal",
        "verify-lowering",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "basal verify-lowering must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "basal verify-lowering for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn write_valid_nbb() -> PathBuf {
    let payload = b"basal-valid-contract";
    let mut header = vec![0u8; 116];
    header[0..4].copy_from_slice(b"NBB1");
    header[4..6].copy_from_slice(&1u16.to_le_bytes());
    header[6..8].copy_from_slice(&0u16.to_le_bytes());
    header[8..12].copy_from_slice(&116u32.to_le_bytes());
    header[12..20].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    header[20..52].copy_from_slice(&Sha256::digest(b"contract"));
    header[52..84].copy_from_slice(&Sha256::digest(payload));
    header[84..116].copy_from_slice(&Sha256::digest(b"seal"));
    let mut bytes = header;
    bytes.extend_from_slice(payload);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("ncc_basal_valid_{}_{}.nbb", std::process::id(), ts));
    std::fs::write(&path, bytes).expect("write valid nbb fixture");
    path
}

fn temp_phase5_nbb_path(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc_basal_phase5_{}_{}_{}.nbb",
        name,
        std::process::id(),
        ts
    ))
}

fn temp_basal_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("ncc_basal_{}_{}_{}", name, std::process::id(), ts));
    std::fs::create_dir_all(&path).expect("create basal temp dir");
    path
}

fn emit_phase5_nbb_fixture(rel: &str, name: &str) -> PathBuf {
    let source = fixture_path(rel);
    let out_path = temp_phase5_nbb_path(name);
    let out = run_ncc(&[
        "basal",
        "emit-nbb",
        source.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "emit-nbb must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    out_path
}

fn assert_nbb_verify_fail_contains(path: &PathBuf, needle: &str) {
    let out = run_ncc(&[
        "basal",
        "verify-nbb",
        path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "verify-nbb must fail for {path:?}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "verify-nbb for {path:?} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn mutate_phase5_nbb<F>(name: &str, mutate: F) -> PathBuf
where
    F: FnOnce(&mut Vec<u8>),
{
    let source = "examples/basal/phase5/nbb_full_structural.nb";
    let path = emit_phase5_nbb_fixture(source, name);
    let mut bytes = std::fs::read(&path).expect("read emitted nbb");
    mutate(&mut bytes);
    let mutated = temp_phase5_nbb_path(name);
    std::fs::write(&mutated, bytes).expect("write mutated nbb");
    mutated
}

fn assert_basal_emit_nbb_fail_contains(rel: &str, needle: &str) {
    let source = fixture_path(rel);
    let out_path = temp_phase5_nbb_path("negative_source");
    let out = run_ncc(&[
        "basal",
        "emit-nbb",
        source.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "emit-nbb must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "emit-nbb for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_basal_surface_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&[
        "basal",
        "verify-surface",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "basal verify-surface must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "basal verify-surface for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_basal_phase55_pipeline_ok(rel: &str, name: &str) -> PathBuf {
    let source = fixture_path(rel);
    for command in [
        "check",
        "tables",
        "ir",
        "verify-ir",
        "lower",
        "verify-lowering",
        "surface",
        "verify-surface",
        "report",
    ] {
        let mut args = vec!["basal", command, source.to_str().expect("utf8 source")];
        if command != "check" {
            args.push("--json");
        }
        let out = run_ncc(&args);
        let stdout = normalize(&out.stdout);
        let stderr = normalize(&out.stderr);
        assert!(
            out.status.success(),
            "basal {command} must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
        );
    }
    let out_path = temp_phase5_nbb_path(name);
    let out = run_ncc(&[
        "basal",
        "emit-nbb",
        source.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "emit-nbb must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    out_path
}

fn assert_basal_phase6_pipeline_ok(rel: &str, name: &str) -> PathBuf {
    let _guard = basal_evidence_cli_lock();
    let source = fixture_path(rel);
    for command in [
        "check",
        "tables",
        "ir",
        "verify-ir",
        "lower",
        "verify-lowering",
    ] {
        let mut args = vec!["basal", command, source.to_str().expect("utf8 source")];
        if command != "check" {
            args.push("--json");
        }
        let out = run_ncc(&args);
        let stdout = normalize(&out.stdout);
        let stderr = normalize(&out.stderr);
        assert!(
            out.status.success(),
            "basal {command} must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
        );
    }
    let pending = temp_phase5_nbb_path(&format!("{name}_pending"));
    let out = run_ncc(&[
        "basal",
        "emit-nbb",
        source.to_str().expect("utf8 source"),
        "--out",
        pending.to_str().expect("utf8 pending"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "emit-nbb must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    let evidence = temp_phase5_nbb_path(&format!("{name}_evidence"));
    let out = run_ncc(&[
        "basal",
        "integrate-evidence",
        source.to_str().expect("utf8 source"),
        "--out",
        evidence.to_str().expect("utf8 evidence"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "integrate-evidence must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    for command in [
        "verify-nbb",
        "seal",
        "verify-seal",
        "evidence-report",
        "seal-report",
    ] {
        let out = run_ncc(&[
            "basal",
            command,
            evidence.to_str().expect("utf8 evidence"),
            "--json",
        ]);
        let stdout = normalize(&out.stdout);
        let stderr = normalize(&out.stderr);
        assert!(
            out.status.success(),
            "basal {command} must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
        );
    }
    evidence
}

fn assert_basal_integrate_fail_contains(rel: &str, needle: &str) {
    let _guard = basal_evidence_cli_lock();
    let source = fixture_path(rel);
    let out_path = temp_phase5_nbb_path("phase6_negative");
    let out = run_ncc(&[
        "basal",
        "integrate-evidence",
        source.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "integrate-evidence must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "integrate-evidence for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn run_basal_json(args: &[&str]) -> String {
    let out = run_ncc(args);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "basal json command must pass for {args:?}\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

#[test]
fn basal_doctor_reports_phase1_machine_unblocked() {
    let out = run_ncc(&["basal", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "basal doctor must pass\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("basal_phase7_release_seal_closed"));
    assert!(stdout.contains("available_closed"));
    assert!(stdout.contains("\"machine_dependency\": \"unblocked\""));
    assert!(stdout.contains("\"nbb_writer\": \"closed\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
    assert!(stdout.contains("\"level_4_allowed\": false"));
}

#[test]
fn basal_phase1_status_exists() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase1_nb_parser_ast\": \"closed\""));
    assert!(stdout.contains("\"nb_grammar\": \"closed\""));
    assert!(stdout.contains("\"nb_ast\": \"closed\""));
}

#[test]
fn basal_phase1_machine_dependency_unblocked() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"machine\": \"available_closed\""));
    assert!(!stdout.contains("blocked_by_machine"));
}

#[test]
fn basal_root_container_parse_ok() {
    assert_basal_ok("examples/basal/root_identity.nb");
}

#[test]
fn basal_nb_file_classification_ok() {
    let path = fixture_path("examples/basal/root_full_contract.nb");
    let out = run_ncc(&[
        "basal",
        "report",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "report failed\n{stdout}");
    assert!(stdout.contains(".nb basal semantic source"));
}

#[test]
fn basal_nbb_file_classification_ok() {
    let path = write_valid_nbb();
    let out = run_ncc(&["basal", "read", path.to_str().expect("utf8 path"), "--json"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "read valid nbb failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains(".nbb basal binary source"));
}

#[test]
fn basal_root_section_ok() {
    assert_basal_ok("examples/basal/root_identity.nb");
}

#[test]
fn basal_binary_section_ok() {
    assert_basal_ok("examples/basal/root_binary_contract.nb");
}

#[test]
fn basal_site_section_ok() {
    assert_basal_ok("examples/basal/root_site_contract.nb");
}

#[test]
fn basal_seal_section_ok() {
    assert_basal_ok("examples/basal/root_seal_contract.nb");
}

#[test]
fn basal_evidence_section_ok() {
    assert_basal_ok("examples/basal/root_evidence_contract.nb");
}

#[test]
fn basal_bridge_compiler_ok() {
    assert_basal_ok("examples/basal/root_bridge_compiler.nb");
}

#[test]
fn basal_bridge_machine_deferred_ok() {
    let stdout = assert_basal_ok("examples/basal/root_bridge_machine_deferred.nb");
    assert!(stdout.contains("basal_phase6_machine_evidence_seal_closed"));
}

#[test]
fn basal_phase1_parser_root_minimal_ok() {
    assert_basal_ok("examples/basal/root_minimal.nb");
}

#[test]
fn basal_phase1_parser_root_identity_ok() {
    assert_basal_ok("examples/basal/root_identity.nb");
}

#[test]
fn basal_phase1_parser_site_table_ok() {
    assert_basal_ok("examples/basal/root_site_table.nb");
}

#[test]
fn basal_phase1_parser_bind_syntax_ok() {
    assert_basal_ok("examples/basal/root_bind_syntax.nb");
}

#[test]
fn basal_phase1_parser_seal_syntax_ok() {
    assert_basal_ok("examples/basal/root_seal_syntax.nb");
}

#[test]
fn basal_phase1_parser_evidence_syntax_ok() {
    assert_basal_ok("examples/basal/root_evidence_syntax.nb");
}

#[test]
fn basal_phase1_parser_target_profile_ok() {
    assert_basal_ok("examples/basal/root_target_profile.nb");
}

#[test]
fn basal_phase1_parser_full_syntax_ok() {
    assert_basal_ok("examples/basal/root_full_syntax.nb");
}

#[test]
fn basal_phase1_ast_root_span_ok() {
    let path = fixture_path("examples/basal/root_minimal.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalRootDecl\""));
    assert!(stdout.contains("\"span\""));
    assert!(stdout.contains("\"line\": 1"));
    assert!(stdout.contains("\"column\": 1"));
}

#[test]
fn basal_phase1_ast_site_span_ok() {
    let path = fixture_path("examples/basal/root_site_table.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalSiteDecl\""));
    assert!(stdout.contains("\"name\": \"heap_anchor\""));
    assert!(stdout.contains("\"span\""));
}

#[test]
fn basal_phase1_ast_bind_span_ok() {
    let path = fixture_path("examples/basal/root_bind_syntax.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalBindDecl\""));
    assert!(stdout.contains("\"name\": \"root_entry\""));
    assert!(stdout.contains("\"span\""));
}

#[test]
fn basal_phase1_ast_seal_span_ok() {
    let path = fixture_path("examples/basal/root_seal_syntax.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalSealRequirement\""));
    assert!(stdout.contains("\"name\": \"basal_ast_hash\""));
    assert!(stdout.contains("\"span\""));
}

#[test]
fn basal_phase1_ast_evidence_span_ok() {
    let path = fixture_path("examples/basal/root_evidence_syntax.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalEvidenceRequirement\""));
    assert!(stdout.contains("\"name\": \"machine_doctor\""));
    assert!(stdout.contains("\"span\""));
}

#[test]
fn basal_phase1_negative_missing_root_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/missing_root_fail.nb",
        "E_BASAL_EXPECTED_ROOT_001",
    );
}

#[test]
fn basal_phase1_negative_unclosed_root_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/unclosed_root_fail.nb",
        "E_BASAL_EXPECTED_RBRACE_001",
    );
}

#[test]
fn basal_phase1_negative_unknown_section_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/unknown_section_fail.nb",
        "E_BASAL_UNKNOWN_SECTION_001",
    );
}

#[test]
fn basal_phase1_negative_duplicate_section_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/duplicate_section_fail.nb",
        "E_BASAL_DUPLICATE_SECTION_001",
    );
}

#[test]
fn basal_phase1_negative_invalid_site_syntax_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/invalid_site_syntax_fail.nb",
        "E_BASAL_SITE_SYNTAX_001",
    );
}

#[test]
fn basal_phase1_negative_invalid_bind_syntax_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/invalid_bind_syntax_fail.nb",
        "E_BASAL_BIND_SYNTAX_001",
    );
}

#[test]
fn basal_phase1_negative_invalid_seal_syntax_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/invalid_seal_syntax_fail.nb",
        "E_BASAL_SEAL_SYNTAX_001",
    );
}

#[test]
fn basal_phase1_negative_invalid_evidence_syntax_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/invalid_evidence_syntax_fail.nb",
        "E_BASAL_EVIDENCE_SYNTAX_001",
    );
}

#[test]
fn basal_phase1_negative_invalid_literal_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/invalid_literal_fail.nb",
        "E_BASAL_INVALID_LITERAL_001",
    );
}

#[test]
fn basal_phase1_recovery_multiple_errors_ok() {
    assert_basal_fail_contains(
        "examples/basal/negative/recovery_multiple_errors_fail.nb",
        "E_BASAL_RECOVERY_001",
    );
}

#[test]
fn basal_phase1_no_panic_on_bad_input() {
    let path = fixture_path("examples/basal/negative/recovery_multiple_errors_fail.nb");
    let out = run_ncc(&[
        "basal",
        "parse",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(!out.status.success());
    assert!(stdout.contains("E_BASAL_RECOVERY_001") || stderr.contains("E_BASAL_RECOVERY_001"));
}

#[test]
fn basal_phase1_no_fake_parse() {
    let path = fixture_path("examples/basal/root_minimal.nb");
    let stdout = run_basal_json(&[
        "basal",
        "parse",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalParseReport\""));
    assert!(stdout.contains("\"kind\": \"BasalSourceFile\""));
    assert!(stdout.contains("\"phase1_nb_parser_ast\": \"closed\""));
}

#[test]
fn basal_phase1_no_unimplemented() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(!stdout.contains("unimplemented"));
    assert!(!stdout.contains("todo!"));
}

#[test]
fn basal_phase1_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase1_nb_parser_ast\": \"closed\""));
    assert!(stdout.contains("\"full_closure\": true"));
}

#[test]
fn basal_phase5_pending_emit_nbb_still_allows_phase6_integration() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase3_basal_ir_root_ir_verifier\": \"closed\""));
    assert!(stdout.contains("\"phase4_lowering_to_machine\": \"closed\""));
    assert!(stdout.contains("\"phase5_nbb_binary_writer_reader\": \"closed\""));
    assert!(stdout.contains("\"basal_ir\": \"closed\""));
    assert!(stdout.contains("\"nbb_writer\": \"closed\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
}

#[test]
fn basal_phase1_does_not_touch_highlevel_runtime_primitive() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"runtime\": \"ok\""));
    assert!(stdout.contains("\"primitive\": \"ok_emulated\""));
}

#[test]
fn basal_phase2_status_exists() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase2_root_site_bind_tables\": \"closed\""));
    assert!(stdout.contains("\"root_table\": \"closed\""));
    assert!(stdout.contains("\"site_table\": \"closed\""));
    assert!(stdout.contains("\"bind_table\": \"closed\""));
}

#[test]
fn basal_phase2_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"status\": \"basal_phase7_release_seal_closed\""));
    assert!(stdout.contains("\"full_closure\": true"));
}

#[test]
fn basal_phase2_machine_dependency_still_unblocked() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"machine\": \"available_closed\""));
    assert!(stdout.contains("\"machine_dependency\": \"unblocked\""));
}

#[test]
fn basal_phase2_root_table_minimal_ok() {
    assert_basal_ok("examples/basal/root_table_minimal.nb");
}

#[test]
fn basal_phase2_root_table_metadata_ok() {
    assert_basal_ok("examples/basal/root_table_metadata.nb");
}

#[test]
fn basal_phase2_root_duplicate_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_duplicate_fail.nb",
        "E_BASAL_ROOT_DUPLICATE_001",
    );
}

#[test]
fn basal_phase2_root_missing_id_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_missing_id_fail.nb",
        "E_BASAL_ROOT_ID_001",
    );
}

#[test]
fn basal_phase2_root_invalid_version_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_invalid_version_fail.nb",
        "E_BASAL_ROOT_VERSION_001",
    );
}

#[test]
fn basal_phase2_root_invalid_profile_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_invalid_profile_fail.nb",
        "E_BASAL_ROOT_PROFILE_001",
    );
}

#[test]
fn basal_phase2_root_spans_ok() {
    let path = fixture_path("examples/basal/root_table_minimal.nb");
    let stdout = run_basal_json(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalRootEntry\""));
    assert!(stdout.contains("\"node_kind\": \"BasalRootDecl\""));
    assert!(stdout.contains("\"line\": 1"));
}

#[test]
fn basal_phase2_site_table_memory_ok() {
    assert_basal_ok("examples/basal/site_table_memory.nb");
}

#[test]
fn basal_phase2_site_table_permissions_ok() {
    assert_basal_ok("examples/basal/site_table_permissions.nb");
}

#[test]
fn basal_phase2_site_table_multiple_ok() {
    assert_basal_ok("examples/basal/site_table_multiple.nb");
}

#[test]
fn basal_phase2_site_duplicate_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_duplicate_fail.nb",
        "E_BASAL_SITE_DUPLICATE_001",
    );
}

#[test]
fn basal_phase2_site_invalid_kind_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_invalid_kind_fail.nb",
        "E_BASAL_SITE_KIND_001",
    );
}

#[test]
fn basal_phase2_site_invalid_size_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_invalid_size_fail.nb",
        "E_BASAL_SITE_SIZE_001",
    );
}

#[test]
fn basal_phase2_site_invalid_align_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_invalid_align_fail.nb",
        "E_BASAL_SITE_ALIGN_001",
    );
}

#[test]
fn basal_phase2_site_invalid_permission_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_invalid_permission_fail.nb",
        "E_BASAL_SITE_PERMISSION_001",
    );
}

#[test]
fn basal_phase2_site_bad_permission_combo_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/site_bad_permission_combo_fail.nb",
        "E_BASAL_SITE_PERMISSION_COMBO_001",
    );
}

#[test]
fn basal_phase2_site_spans_ok() {
    let path = fixture_path("examples/basal/site_table_memory.nb");
    let stdout = run_basal_json(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalSiteEntry\""));
    assert!(stdout.contains("\"name\": \"heap_anchor\""));
    assert!(stdout.contains("\"node_kind\": \"BasalSiteDecl\""));
}

#[test]
fn basal_phase2_bind_machine_pending_ok() {
    let stdout = assert_basal_ok("examples/basal/bind_table_machine_pending.nb");
    assert!(stdout.contains("basal_phase6_machine_evidence_seal_closed"));
}

#[test]
fn basal_phase2_bind_local_site_ok() {
    assert_basal_ok("examples/basal/bind_table_local_site.nb");
}

#[test]
fn basal_phase2_bind_invalid_source_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/bind_invalid_source_fail.nb",
        "E_BASAL_BIND_SOURCE_001",
    );
}

#[test]
fn basal_phase2_bind_invalid_dest_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/bind_invalid_dest_fail.nb",
        "E_BASAL_BIND_DEST_001",
    );
}

#[test]
fn basal_phase2_bind_unknown_kind_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/bind_unknown_kind_fail.nb",
        "E_BASAL_BIND_KIND_001",
    );
}

#[test]
fn basal_phase2_bind_duplicate_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/bind_duplicate_fail.nb",
        "E_BASAL_BIND_DUPLICATE_001",
    );
}

#[test]
fn basal_phase2_bind_bad_local_site_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/bind_bad_local_site_fail.nb",
        "E_BASAL_BIND_LOCAL_TARGET_001",
    );
}

#[test]
fn basal_phase2_bind_spans_ok() {
    let path = fixture_path("examples/basal/bind_table_machine_pending.nb");
    let stdout = run_basal_json(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalBindEntry\""));
    assert!(stdout.contains("\"node_kind\": \"BasalBindDecl\""));
    assert!(stdout.contains("\"resolution_status\": \"pending_lowering_or_machine_resolution\""));
}

#[test]
fn basal_phase2_tables_json_ok() {
    let path = fixture_path("examples/basal/root_site_bind_full.nb");
    let stdout = run_basal_json(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalTableSet\""));
    assert!(stdout.contains("\"phase\": \"phase2_tables\""));
    assert!(stdout.contains("\"status\": \"basal_phase2_root_site_bind_tables_closed\""));
}

#[test]
fn basal_phase2_report_includes_tables() {
    let path = fixture_path("examples/basal/root_site_bind_full.nb");
    let stdout = run_basal_json(&[
        "basal",
        "report",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"tables\""));
    assert!(stdout.contains("\"root_table\""));
    assert!(stdout.contains("\"site_table\""));
    assert!(stdout.contains("\"bind_table\""));
}

#[test]
fn basal_phase2_no_fake_tables() {
    let path = fixture_path("examples/basal/root_site_bind_full.nb");
    let stdout = run_basal_json(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(!stdout.contains("fake"));
    assert!(stdout.contains("\"BasalRootTable\""));
    assert!(stdout.contains("\"BasalSiteTable\""));
    assert!(stdout.contains("\"BasalBindTable\""));
}

#[test]
fn basal_phase2_no_panic_on_bad_input() {
    let path = fixture_path("examples/basal/negative/bind_bad_local_site_fail.nb");
    let out = run_ncc(&[
        "basal",
        "tables",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stdout.contains("E_BASAL_BIND_LOCAL_TARGET_001")
            || stderr.contains("E_BASAL_BIND_LOCAL_TARGET_001")
    );
}

#[test]
fn basal_phase1_parser_ast_regressions_ok() {
    let path = fixture_path("examples/basal/root_full_syntax.nb");
    let stdout = run_basal_json(&["basal", "ast", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalSourceFile\""));
    assert!(stdout.contains("\"phase1_nb_parser_ast\": \"closed\""));
}

#[test]
fn basal_phase3_ir_module_exists() {
    let path = fixture_path("examples/basal/phase3/root_site_bind_ir_full.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalIrModule\""));
    assert!(stdout.contains("\"phase3_basal_ir_root_ir_verifier\": \"closed\""));
    assert!(stdout.contains("\"lowering\": \"not_started\""));
}

#[test]
fn basal_phase3_root_ir_ok() {
    let path = fixture_path("examples/basal/phase3/root_ir_minimal.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalRootIr\""));
    assert!(stdout.contains("\"root_id\": \"nC.root.ir.minimal\""));
    assert!(stdout.contains("\"root_ir\": \"closed\""));
}

#[test]
fn basal_phase3_site_ir_ok() {
    let path = fixture_path("examples/basal/phase3/site_ir_memory.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalSiteIr\""));
    assert!(stdout.contains("\"site_id\": \"heap_anchor\""));
    assert!(stdout.contains("\"site_kind\": \"memory_anchor\""));
}

#[test]
fn basal_phase3_bind_ir_ok() {
    let path = fixture_path("examples/basal/phase3/bind_ir_local_site.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"kind\": \"BasalBindIr\""));
    assert!(stdout.contains("\"bind_id\": \"heap_binding\""));
    assert!(stdout.contains("\"resolution_status\": \"resolved_local_site\""));
}

#[test]
fn basal_phase3_ir_spans_ok() {
    let path = fixture_path("examples/basal/phase3/root_site_bind_ir_full.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"span\""));
    assert!(stdout.contains("\"node_kind\": \"BasalRootDecl\""));
    assert!(stdout.contains("\"node_kind\": \"BasalSiteDecl\""));
    assert!(stdout.contains("\"node_kind\": \"BasalBindDecl\""));
}

#[test]
fn basal_phase3_ir_permissions_normalized_ok() {
    let path = fixture_path("examples/basal/phase3/permissions_ir_full.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"read\""));
    assert!(stdout.contains("\"execute\""));
    assert!(stdout.contains("\"evidence\""));
    assert!(stdout.contains("\"seal\""));
}

#[test]
fn basal_phase3_ir_local_site_resolution_ok() {
    let path = fixture_path("examples/basal/phase3/bind_ir_local_site.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-ir",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"local_refs_resolved\": 1"));
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn basal_phase3_ir_machine_refs_pending_ok() {
    let path = fixture_path("examples/basal/phase3/bind_ir_machine_pending.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-ir",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_refs_pending\": 1"));
    assert!(stdout.contains("pending_machine_resolution"));
    assert!(stdout.contains("E_BASAL_IR_MACHINE_PENDING_001"));
}

#[test]
fn basal_phase3_ir_no_fake_machine_evidence() {
    let path = fixture_path("examples/basal/phase3/bind_ir_machine_pending.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"no_machine_evidence\": true"));
    assert!(stdout.contains("\"machine_evidence_integration\": \"not_started\""));
    assert!(!stdout.contains("fake"));
}

#[test]
fn basal_phase3_verifier_ok() {
    let path = fixture_path("examples/basal/phase3/root_site_bind_ir_full.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-ir",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalIrVerifierReport\""));
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn basal_phase3_report_includes_ir() {
    let path = fixture_path("examples/basal/phase3/root_site_bind_ir_full.nb");
    let stdout = run_basal_json(&[
        "basal",
        "report",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"ir\""));
    assert!(stdout.contains("\"BasalIrModule\""));
    assert!(stdout.contains("\"phase3_basal_ir_root_ir_verifier\": \"closed\""));
}

#[test]
fn basal_phase3_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"basal_phase3_basal_ir_root_ir_verifier_closed\": true"));
    assert!(stdout.contains("\"basal_ir\": \"closed\""));
    assert!(stdout.contains("\"semantic_verifier\": \"closed\""));
    assert!(stdout.contains("\"nbb_writer\": \"closed\""));
}

#[test]
fn basal_phase4_lowering_plan_exists() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalLoweringPlan\""));
    assert!(stdout.contains("\"phase4_lowering_to_machine\": \"closed\""));
    assert!(stdout.contains("\"machine_plan\""));
}

#[test]
fn basal_phase4_root_lowering_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_minimal_root.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lowering-plan",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalRootLowering\""));
    assert!(stdout.contains("\"root_id\": \"nC.root.lowering.minimal\""));
    assert!(stdout.contains("\"target_machine_profile\": \"nC-x86_64-native-v1\""));
}

#[test]
fn basal_phase4_site_lowering_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_site_to_section.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalSiteLowering\""));
    assert!(stdout.contains("\"site_id\": \"data_anchor\""));
    assert!(stdout.contains("\"target_section\": \".data\""));
}

#[test]
fn basal_phase4_bind_lowering_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_bind_machine_symbol.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"kind\": \"BasalBindLowering\""));
    assert!(stdout.contains("\"machine_symbol_resolution\": \"resolved_to_machine_symbol\""));
}

#[test]
fn basal_phase4_machine_ref_resolution_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_bind_machine_symbol.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-lowering",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_refs_resolved\": 1"));
    assert!(stdout.contains("\"machine_ref_resolution\": \"closed\""));
}

#[test]
fn basal_phase4_local_ref_resolution_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_bind_local_site.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-lowering",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"local_refs_resolved\": 1"));
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn basal_phase4_permission_mapping_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_permissions.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"execute\""));
    assert!(stdout.contains("\"evidence\""));
    assert!(stdout.contains("\"diagnostics\": []"));
}

#[test]
fn basal_phase4_span_preservation_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"source_span\""));
    assert!(stdout.contains("\"node_kind\": \"BasalRootDecl\""));
    assert!(stdout.contains("\"node_kind\": \"BasalSiteDecl\""));
    assert!(stdout.contains("\"node_kind\": \"BasalBindDecl\""));
}

#[test]
fn basal_phase4_evidence_boundary_pending_ok() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "verify-lowering",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"evidence_boundary_pending\": true"));
    assert!(stdout.contains("pending_phase6_machine_evidence_integration"));
}

#[test]
fn basal_phase4_no_fake_machine_evidence() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_evidence_integration\": \"not_started\""));
    assert!(!stdout.contains("fake"));
}

#[test]
fn basal_phase4_no_nbb_writer() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"nbb_writer\": \"not_started\""));
    assert!(stdout.contains("\"no_nbb_writer\": true"));
}

#[test]
fn basal_phase4_report_includes_lowering() {
    let path = fixture_path("examples/basal/phase4/lowering_full_plan.nb");
    let stdout = run_basal_json(&[
        "basal",
        "report",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"lowering\""));
    assert!(stdout.contains("\"BasalLoweringPlan\""));
    assert!(stdout.contains("\"phase4_lowering_to_machine\": \"closed\""));
}

#[test]
fn basal_phase4_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"basal_phase4_lowering_to_machine_closed\": true"));
    assert!(stdout.contains("\"lowering\": \"closed\""));
    assert!(stdout.contains("\"machine_ref_resolution\": \"closed\""));
    assert!(stdout.contains("\"nbb_writer\": \"closed\""));
}

#[test]
fn basal_phase4_missing_target_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_missing_target_fail.nb",
        "E_BASAL_LOWERING_TARGET_001",
    );
}

#[test]
fn basal_phase4_invalid_machine_symbol_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_invalid_machine_symbol_fail.nb",
        "E_BASAL_LOWERING_SYMBOL_001",
    );
}

#[test]
fn basal_phase4_duplicate_machine_symbol_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_duplicate_machine_symbol_fail.nb",
        "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
    );
}

#[test]
fn basal_phase4_unresolved_local_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_unresolved_local_fail.nb",
        "E_BASAL_LOWERING_UNRESOLVED_LOCAL_001",
    );
}

#[test]
fn basal_phase4_unresolved_machine_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_unresolved_machine_fail.nb",
        "E_BASAL_LOWERING_UNRESOLVED_MACHINE_001",
    );
}

#[test]
fn basal_phase4_invalid_permission_mapping_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_invalid_permission_mapping_fail.nb",
        "E_BASAL_LOWERING_PERMISSION_001",
    );
}

#[test]
fn basal_phase4_invalid_alignment_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_invalid_alignment_fail.nb",
        "E_BASAL_LOWERING_ALIGN_001",
    );
}

#[test]
fn basal_phase4_invalid_section_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_invalid_section_fail.nb",
        "E_BASAL_LOWERING_SECTION_001",
    );
}

#[test]
fn basal_phase4_fake_evidence_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_fake_evidence_fail.nb",
        "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
    );
}

#[test]
fn basal_phase4_nbb_write_forbidden_fail() {
    assert_basal_verify_lowering_fail_contains(
        "examples/basal/phase4/negative/lowering_nbb_write_forbidden_fail.nb",
        "E_BASAL_LOWERING_NBB_FORBIDDEN_001",
    );
}

#[test]
fn basal_phase3_missing_local_site_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_missing_local_site_fail.nb",
        "E_BASAL_IR_LOCAL_REF_001",
    );
}

#[test]
fn basal_phase3_invalid_permission_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_invalid_permission_fail.nb",
        "E_BASAL_IR_PERMISSION_001",
    );
}

#[test]
fn basal_phase3_invalid_align_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_invalid_align_fail.nb",
        "E_BASAL_IR_ALIGN_001",
    );
}

#[test]
fn basal_phase3_zero_size_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_zero_size_fail.nb",
        "E_BASAL_IR_SIZE_001",
    );
}

#[test]
fn basal_phase3_duplicate_symbol_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_duplicate_symbol_fail.nb",
        "E_BASAL_IR_DUPLICATE_001",
    );
}

#[test]
fn basal_phase3_invalid_bind_kind_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_invalid_bind_kind_fail.nb",
        "E_BASAL_IR_BIND_001",
    );
}

#[test]
fn basal_phase3_local_ref_unresolved_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_local_ref_unresolved_fail.nb",
        "E_BASAL_IR_LOCAL_REF_001",
    );
}

#[test]
fn basal_phase3_bad_machine_ref_silent_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_bad_machine_ref_silent_fail.nb",
        "E_BASAL_IR_MACHINE_PENDING_001",
    );
}

#[test]
fn basal_phase3_table_consistency_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_table_consistency_fail.nb",
        "E_BASAL_IR_CONSISTENCY_001",
    );
}

#[test]
fn basal_phase3_span_missing_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase3/negative/ir_span_missing_fail.nb",
        "E_BASAL_IR_SPAN_001",
    );
}

#[test]
fn basal_phase1_examples_still_pass() {
    for rel in [
        "examples/basal/root_minimal.nb",
        "examples/basal/root_identity.nb",
        "examples/basal/root_full_syntax.nb",
    ] {
        assert_basal_ok(rel);
    }
}

#[test]
fn basal_phase2_examples_still_pass() {
    for rel in [
        "examples/basal/root_table_minimal.nb",
        "examples/basal/site_table_memory.nb",
        "examples/basal/bind_table_machine_pending.nb",
        "examples/basal/root_site_bind_full.nb",
    ] {
        assert_basal_ok(rel);
    }
}

#[test]
fn machine_asm_complete_total_still_true() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "native doctor must remain ok\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_regression_still_ok() {
    let out = run_ncc(&["machine", "doctor"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "machine doctor must remain ok\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("machine doctor: OK"));
    assert!(stdout.contains("asm_complete_total: true"));
}

#[test]
fn basal_bridge_primitive_mutation_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_primitive_mutation_fail.nb",
        "E_BASAL_BRIDGE_PRIMITIVE_MUTATION_001",
    );
}

#[test]
fn basal_runtime_import_forbidden_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_runtime_import_forbidden_fail.nb",
        "E_BASAL_FORBIDDEN_RUNTIME_001",
    );
}

#[test]
fn basal_binary_unstable_type_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_binary_unstable_field_fail.nb",
        "E_BASAL_SYMBOL_BINARY_UNSTABLE_001",
    );
}

#[test]
fn basal_seal_required_fail() {
    assert_basal_fail_contains(
        "examples/basal/negative/root_seal_missing_fail.nb",
        "E_BASAL_SEAL_REQUIRED_001",
    );
}

#[test]
fn basal_nbb_magic_fail() {
    let path = fixture_path("examples/basal/negative/nbb_bad_magic_fail.nbb");
    let out = run_ncc(&[
        "basal",
        "verify",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "bad magic must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("E_NBB_MAGIC_001") || stderr.contains("E_NBB_MAGIC_001"));
}

#[test]
fn basal_nbb_verify_ok() {
    let path = write_valid_nbb();
    let out = run_ncc(&[
        "basal",
        "verify",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "valid nbb must verify\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("structurally_valid"));
}

#[test]
fn basal_phase5_nbb_header_ok() {
    let path = emit_phase5_nbb_fixture("examples/basal/phase5/nbb_minimal.nb", "header");
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"magic\": \"NBB0\""));
    assert!(stdout.contains("\"version\": 5"));
    assert!(stdout.contains("\"header_size\": 64"));
    assert!(stdout.contains("\"structurally_valid\""));
}

#[test]
fn basal_phase5_nbb_section_table_ok() {
    let path = emit_phase5_nbb_fixture("examples/basal/phase5/nbb_full_structural.nb", "sections");
    let stdout = run_basal_json(&[
        "basal",
        "inspect-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    for section in [
        "string_table",
        "root",
        "site",
        "bind",
        "lowering",
        "pending_evidence",
    ] {
        assert!(stdout.contains(section), "missing section {section}");
    }
}

#[test]
fn basal_phase5_nbb_root_records_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_root_site_bind.nb",
        "root_records",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"root_records\""));
    assert!(stdout.contains("nC.root.nbb.root_site_bind"));
}

#[test]
fn basal_phase5_nbb_site_records_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_multiple_sites.nb",
        "site_records",
    );
    let stdout = run_basal_json(&[
        "basal",
        "inspect-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"site_records\""));
    assert!(stdout.contains("heap_anchor"));
    assert!(stdout.contains("code_anchor"));
    assert!(stdout.contains("evidence_anchor"));
}

#[test]
fn basal_phase5_nbb_bind_records_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_root_site_bind.nb",
        "bind_records",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"bind_records\""));
    assert!(stdout.contains("\"machine.proc\""));
    assert!(stdout.contains("\"root.site\""));
}

#[test]
fn basal_phase5_nbb_lowering_records_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_lowering_plan.nb",
        "lowering_records",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"lowering_record\""));
    assert!(stdout.contains("\"target_triple\": \"x86_64-unknown-linux-gnu\""));
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn basal_phase5_nbb_pending_evidence_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_machine_refs_pending.nb",
        "pending_evidence",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"pending_machine_evidence_integration\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"not_started\""));
    assert!(stdout.contains("\"final_evidence_integrated\": false"));
}

#[test]
fn basal_phase5_emit_read_inspect_verify_nbb_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_inspect_full.nb",
        "emit_read_inspect_verify",
    );
    for command in ["verify-nbb", "read-nbb", "inspect-nbb", "nbb-info"] {
        let stdout = run_basal_json(&[
            "basal",
            command,
            path.to_str().expect("utf8 path"),
            "--json",
        ]);
        assert!(
            stdout.contains("structurally_valid")
                || stdout.contains("\"status\": \"structurally_valid\"")
        );
    }
}

#[test]
fn basal_phase5_nbb_checksum_ok() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_full_structural.nb",
        "checksum_ok",
    );
    let stdout = run_basal_json(&[
        "basal",
        "verify-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"structural_hash\""));
    assert!(stdout.contains("\"structural_hash_computed\""));
    assert!(stdout.contains("\"diagnostics\": []"));
}

#[test]
fn basal_phase5_no_json_disguised_as_nbb() {
    let path = emit_phase5_nbb_fixture("examples/basal/phase5/nbb_minimal.nb", "binary_prefix");
    let bytes = std::fs::read(&path).expect("read emitted nbb");
    assert!(bytes.starts_with(b"NBB0"));
    assert!(!bytes.starts_with(b"{"));
}

#[test]
fn basal_phase5_no_fake_machine_evidence() {
    let path = emit_phase5_nbb_fixture(
        "examples/basal/phase5/nbb_full_structural.nb",
        "evidence_guard",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_evidence_integration\": \"not_started\""));
    assert!(stdout.contains("\"final_evidence_integrated\": false"));
    assert!(!stdout.contains("\"final_integrated\""));
}

#[test]
fn basal_phase5_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase5_nbb_binary_writer_reader\": \"closed\""));
    assert!(stdout.contains("\"nbb_writer\": \"closed\""));
    assert!(stdout.contains("\"nbb_reader\": \"closed\""));
    assert!(stdout.contains("\"nbb_verifier\": \"closed\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
}

#[test]
fn basal_phase5_invalid_magic_fail() {
    let path = mutate_phase5_nbb("invalid_magic", |bytes| {
        bytes[0..4].copy_from_slice(b"BAD!");
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_MAGIC_001");
}

#[test]
fn basal_phase5_unsupported_version_fail() {
    let path = mutate_phase5_nbb("unsupported_version", |bytes| {
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_VERSION_001");
}

#[test]
fn basal_phase5_bad_checksum_fail() {
    let path = mutate_phase5_nbb("bad_checksum", |bytes| {
        bytes[32] ^= 0x5a;
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_CHECKSUM_001");
}

#[test]
fn basal_phase5_section_out_of_bounds_fail() {
    let path = mutate_phase5_nbb("section_out_of_bounds", |bytes| {
        bytes[72..80].copy_from_slice(&(u64::MAX - 8).to_le_bytes());
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_SECTION_BOUNDS_001");
}

#[test]
fn basal_phase5_section_overlap_fail() {
    let path = mutate_phase5_nbb("section_overlap", |bytes| {
        let first_offset = bytes[72..80].to_vec();
        bytes[136..144].copy_from_slice(&first_offset);
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_SECTION_OVERLAP_001");
}

#[test]
fn basal_phase5_bad_alignment_fail() {
    let path = mutate_phase5_nbb("bad_alignment", |bytes| {
        bytes[68..72].copy_from_slice(&3u32.to_le_bytes());
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_SECTION_ALIGN_001");
}

#[test]
fn basal_phase5_missing_root_section_fail() {
    let path = mutate_phase5_nbb("missing_root_section", |bytes| {
        bytes[128..130].copy_from_slice(&7u16.to_le_bytes());
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_ROOT_RECORD_001");
}

#[test]
fn basal_phase5_missing_pending_evidence_fail() {
    let path = mutate_phase5_nbb("missing_pending_evidence", |bytes| {
        bytes[384..386].copy_from_slice(&7u16.to_le_bytes());
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_EVIDENCE_PENDING_001");
}

#[test]
fn basal_phase5_fake_final_evidence_fail() {
    let path = mutate_phase5_nbb("final_evidence_record", |bytes| {
        let needle = b"pending_machine_evidence_integration";
        if let Some(index) = bytes
            .windows(needle.len())
            .position(|window| window == needle)
        {
            bytes[index..index + 5].copy_from_slice(b"final");
        }
    });
    assert_nbb_verify_fail_contains(&path, "E_NBB_FAKE_EVIDENCE_001");
}

#[test]
fn basal_phase5_emit_invalid_lowering_fail() {
    assert_basal_emit_nbb_fail_contains(
        "examples/basal/phase5/negative/nbb_emit_invalid_lowering_fail.nb",
        "E_BASAL_LOWERING_UNRESOLVED_LOCAL_001",
    );
}

#[test]
fn basal_phase5_emit_final_evidence_fail() {
    assert_basal_emit_nbb_fail_contains(
        "examples/basal/phase5/negative/nbb_emit_fake_evidence_fail.nb",
        "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
    );
}

#[test]
fn basal_phase5_emit_unresolved_machine_fail() {
    assert_basal_emit_nbb_fail_contains(
        "examples/basal/phase5/negative/nbb_emit_unresolved_machine_fail.nb",
        "E_BASAL_LOWERING_UNRESOLVED_MACHINE_001",
    );
}

#[test]
fn basal_phase5_5_const_surface_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/const_widths.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"const_surface\": \"closed\""));
    assert!(stdout.contains("\"BasalConstIr\""));
}

#[test]
fn basal_phase5_5_const_widths_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/const_widths.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    for ty in ["byte", "word", "dword", "qword", "str"] {
        assert!(stdout.contains(ty), "missing width type {ty}");
    }
}

#[test]
fn basal_phase5_5_const_hash_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/const_widths.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"value_hash\""));
}

#[test]
fn basal_phase5_5_routine_surface_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/routine_minimal.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"BasalRoutineIr\""));
    assert!(stdout.contains("\"routine_surface\": \"closed\""));
}

#[test]
fn basal_phase5_5_routine_lowering_plan_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        fixture_path("examples/basal/phase5_5/routine_minimal.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"surface_records\""));
    assert!(stdout.contains("lowering_hook_planned"));
}

#[test]
fn basal_phase5_5_raw_surface_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/raw_memory_dword.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"raw_surface\": \"closed\""));
    assert!(stdout.contains("raw_memory_read"));
    assert!(stdout.contains("raw_memory_write"));
}

#[test]
fn basal_phase5_5_raw_hazard_boundary_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/raw_memory_dword.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("pending_phase6_machine_evidence_integration"));
}

#[test]
fn basal_phase5_5_verify_surface_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "verify-surface",
        fixture_path("examples/basal/phase5_5/verify_basic.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"verify_surface\": \"closed\""));
}

#[test]
fn basal_phase5_5_probe_alias_ok_or_disabled() {
    let stdout = run_basal_json(&[
        "basal",
        "verify-surface",
        fixture_path("examples/basal/phase5_5/probe_alias_compat.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"probe_alias\": \"closed\""));
    assert!(stdout.contains("\"probe_alias\": true"));
}

#[test]
fn basal_phase5_5_report_surface_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "report-plan",
        fixture_path("examples/basal/phase5_5/report_status_emit.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"BasalReportPlan\""));
    assert!(stdout.contains("\"report_surface\": \"closed\""));
}

#[test]
fn basal_phase5_5_width_types_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"width_types\": \"closed\""));
}

#[test]
fn basal_phase5_5_mem_addr_site_ok() {
    let stdout = run_basal_json(&[
        "basal",
        "surface",
        fixture_path("examples/basal/phase5_5/mem_addr_site.nb")
            .to_str()
            .expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"target\": \"@src\""));
    assert!(stdout.contains("\"target\": \"output_site\""));
}

#[test]
fn basal_phase5_5_terminal_authority_docs_ok() {
    let doc = std::fs::read_to_string(fixture_path("docs/spec/basal/basal_terminal_authority.md"))
        .expect("terminal authority doc");
    assert!(doc.contains("El IDE asiste. El terminal sentencia."));
}

#[test]
fn basal_phase5_5_nbb_records_ok() {
    let nbb = assert_basal_phase55_pipeline_ok(
        "examples/basal/phase5_5/bit_weight_pure_root.nb",
        "phase55_records",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("executable_surface"));
    assert!(stdout.contains("\"surface_records\""));
}

#[test]
fn basal_phase5_5_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase5_5_executable_surface\": \"closed\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
    assert!(stdout.contains("\"seal_final\": \"closed\""));
}

#[test]
fn basal_phase5_5_const_mutation_fail() {
    assert_basal_surface_fail_contains(
        "examples/basal/phase5_5/negative/const_mutation_fail.nb",
        "E_BASAL_CONST_MUTATION_001",
    );
}

#[test]
fn basal_phase5_5_ambiguous_int_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/const_ambiguous_int_fail.nb",
        "E_BASAL_INT_AMBIGUOUS_001",
    );
}

#[test]
fn basal_phase5_5_width_overflow_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/const_width_overflow_fail.nb",
        "E_BASAL_CONST_RANGE_001",
    );
}

#[test]
fn basal_phase5_5_fn_in_basal_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/fn_in_basal_fail.nb",
        "E_BASAL_FN_LEGACY_001",
    );
}

#[test]
fn basal_phase5_5_routine_bad_param_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/routine_bad_param_fail.nb",
        "E_BASAL_INT_AMBIGUOUS_001",
    );
}

#[test]
fn basal_phase5_5_raw_missing_width_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/raw_missing_width_fail.nb",
        "E_BASAL_RAW_WIDTH_001",
    );
}

#[test]
fn basal_phase5_5_raw_invalid_mem_permission_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/raw_invalid_mem_permission_fail.nb",
        "E_BASAL_RAW_PERMISSION_001",
    );
}

#[test]
fn basal_phase5_5_verify_bad_routine_call_fail() {
    assert_basal_surface_fail_contains(
        "examples/basal/phase5_5/negative/verify_bad_routine_call_fail.nb",
        "E_BASAL_VERIFY_ROUTINE_CALL_001",
    );
}

#[test]
fn basal_phase5_5_report_runtime_print_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/report_runtime_print_fail.nb",
        "E_BASAL_REPORT_RUNTIME_PRINT_FORBIDDEN_001",
    );
}

#[test]
fn basal_phase5_5_mem_unknown_site_fail() {
    assert_basal_fail_contains(
        "examples/basal/phase5_5/negative/mem_unknown_site_fail.nb",
        "E_BASAL_MEM_SITE_001",
    );
}

#[test]
fn basal_phase5_5_regressions_still_pass() {
    for rel in [
        "examples/basal/root_minimal.nb",
        "examples/basal/root_table_minimal.nb",
        "examples/basal/phase3/root_ir_minimal.nb",
        "examples/basal/phase4/lowering_minimal_root.nb",
        "examples/basal/phase5/nbb_minimal.nb",
    ] {
        assert_basal_ok(rel);
    }
}

#[test]
fn basal_phase6_machine_evidence_record_ok() {
    let nbb = assert_basal_phase6_pipeline_ok(
        "examples/basal/phase6/evidence_minimal.nb",
        "phase6_evidence_minimal",
    );
    let stdout = run_basal_json(&[
        "basal",
        "evidence-report",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("\"NbbMachineEvidenceRecord\""));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
}

#[test]
fn basal_phase6_machine_doctor_evidence_ok() {
    let _guard = basal_evidence_cli_lock();
    let stdout = run_basal_json(&[
        "basal",
        "evidence",
        fixture_path("examples/basal/phase6/evidence_machine_refs.nb")
            .to_str()
            .expect("utf8 source"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_doctor\""));
    assert!(stdout.contains("\"status\": \"ok\""));
}

#[test]
fn basal_phase6_native_doctor_evidence_ok() {
    let _guard = basal_evidence_cli_lock();
    let stdout = run_basal_json(&[
        "basal",
        "evidence",
        fixture_path("examples/basal/phase6/evidence_machine_refs.nb")
            .to_str()
            .expect("utf8 source"),
        "--json",
    ]);
    assert!(stdout.contains("\"native_doctor\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn basal_phase6_object_disasm_roundtrip_evidence_ok() {
    let nbb = assert_basal_phase6_pipeline_ok(
        "examples/basal/phase6/evidence_routine_raw.nb",
        "phase6_routine_raw",
    );
    let stdout = run_basal_json(&[
        "basal",
        "read-nbb",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("\"machine_emit_object\""));
    assert!(stdout.contains("\"machine_verify_object\""));
    assert!(stdout.contains("\"machine_disasm\""));
    assert!(stdout.contains("\"machine_roundtrip\""));
    assert!(stdout.contains("\"artifact_hash\""));
}

#[test]
fn basal_phase6_hash_chain_and_reports_ok() {
    let nbb = assert_basal_phase6_pipeline_ok(
        "examples/basal/phase6/evidence_verify_report.nb",
        "phase6_hash_chain",
    );
    let stdout = run_basal_json(&[
        "basal",
        "seal-report",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("\"seal_verified\""));
    assert!(stdout.contains("\"deterministic_inputs_hash\""));
}

#[test]
fn basal_phase6_seal_hash_ok() {
    let nbb =
        assert_basal_phase6_pipeline_ok("examples/basal/phase6/seal_minimal.nb", "phase6_seal");
    let stdout = run_basal_json(&[
        "basal",
        "verify-seal",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("\"status\": \"seal_verified\""));
    assert!(stdout.contains("\"algorithm\": \"SHA-256\""));
}

#[test]
fn basal_phase6_bit_weight_pure_root_sealed_ok() {
    let nbb = assert_basal_phase6_pipeline_ok(
        "examples/basal/phase6/bit_weight_pure_root_sealed.nb",
        "phase6_bit_weight",
    );
    let stdout = run_basal_json(&[
        "basal",
        "verify-seal",
        nbb.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(stdout.contains("seal_verified"));
    assert!(stdout.contains("machine_roundtrip"));
}

#[test]
fn basal_phase6_doctor_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"phase6_machine_evidence_seal\": \"closed\""));
    assert!(stdout.contains("\"basal_phase6_machine_evidence_seal_closed\": true"));
    assert!(stdout.contains("\"machine_evidence_integration\": \"closed\""));
    assert!(stdout.contains("\"seal_final\": \"closed\""));
}

#[test]
fn basal_phase7_release_seal_gate_ok() {
    let stdout = run_basal_json(&["basal", "doctor", "--json"]);
    assert!(stdout.contains("\"status\": \"basal_phase7_release_seal_closed\""));
    assert!(stdout.contains("\"phase7_release_seal\": \"closed\""));
    assert!(stdout.contains("\"basal_phase7_release_seal_closed\": true"));
    assert!(stdout.contains("\"basal_closed\": true"));
    assert!(stdout.contains("\"conformance_matrix\": \"closed\""));
    assert!(stdout.contains("\"evidence_bundle\": \"closed\""));
    assert!(stdout.contains("\"worktree_classification\": \"closed\""));
}

fn seal_phase7_bit_weight_fixture(name: &str) -> PathBuf {
    let _guard = basal_evidence_cli_lock();
    let source = fixture_path("examples/basal/phase6/bit_weight_pure_root_sealed.nb");
    let out_path = temp_phase5_nbb_path(name);
    let out = run_ncc(&[
        "basal",
        "seal",
        source.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "basal seal must write release .nbb\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"status\": \"sealed\""));
    assert!(stdout.contains("\"bytes_written\""));
    let bytes = std::fs::read(&out_path).expect("sealed nbb must exist");
    assert!(bytes.len() > 64, "sealed nbb must contain NBB0 payload");
    assert_eq!(&bytes[0..4], b"NBB0");
    out_path
}

#[test]
fn basal_seal_machine_closed_writes_nbb_ok() {
    let nbb = seal_phase7_bit_weight_fixture("seal_machine_closed_writes");
    let bytes = std::fs::read(&nbb).expect("sealed nbb");
    assert!(bytes.len() > 64);
}

#[test]
fn basal_seal_machine_not_closed_deferred_fail() {
    let source = fixture_path("examples/basal/phase6/bit_weight_pure_root_sealed.nb");
    let out_path = temp_phase5_nbb_path("seal_machine_not_closed");
    let out = run_ncc_with_env(
        &[
            "basal",
            "seal",
            source.to_str().expect("utf8 source"),
            "--out",
            out_path.to_str().expect("utf8 out"),
        ],
        &[("NCC_TEST_BASAL_SEAL_MACHINE_NOT_CLOSED", "1")],
    );
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "simulated not-closed machine gate must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_BASAL_SEAL_DEFERRED_001")
            || stderr.contains("E_BASAL_SEAL_DEFERRED_001")
    );
    assert!(stdout.contains("\"bytes_written\": 0"));
    assert!(
        !out_path.exists(),
        "deferred seal must not write a partial .nbb artifact"
    );
}

#[test]
fn basal_verify_written_nbb_ok() {
    let nbb = seal_phase7_bit_weight_fixture("seal_verify_written");
    let stdout = run_basal_json(&["basal", "verify", nbb.to_str().expect("utf8 nbb"), "--json"]);
    assert!(stdout.contains("\"status\": \"structurally_valid\""));
    assert!(stdout.contains("\"machine_evidence\""));
}

#[test]
fn basal_read_written_nbb_ok() {
    let nbb = seal_phase7_bit_weight_fixture("seal_read_written");
    let stdout = run_basal_json(&["basal", "read", nbb.to_str().expect("utf8 nbb"), "--json"]);
    assert!(stdout.contains("\"magic\": \"NBB0\""));
    assert!(stdout.contains("\"seal_verified\""));
}

#[test]
fn basal_foreign_external_bindings_ir_ok() {
    assert_basal_ok("examples/basal/ffi/foreign_c_contract.nb");
    let path = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let stdout = run_basal_json(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(stdout.contains("\"BasalForeignBindingIr\""));
    assert!(stdout.contains("\"foreign_surface\": \"closed\""));
    assert!(stdout.contains("\"ffi_link_manifest_hash\""));
    assert!(stdout.contains("\"schema\": \"ghor.ffi_manifest.v1\""));
    assert!(stdout.contains("\"language\": \"c\""));
    assert!(stdout.contains("\"symbol\": \"nc_basal_c_add\""));
}

#[test]
fn basal_foreign_ffi_manifest_ghor_compatible_ok() {
    let path = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let stdout = run_basal_json(&[
        "basal",
        "ffi-manifest",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"schema\": \"basal.ffi_package_manifest.v1\""));
    assert!(stdout.contains("\"schema\": \"ghor.ffi_manifest.v1\""));
    assert!(stdout.contains("\"ghor_compatible\": true"));
    assert!(stdout.contains("\"ffi_link_manifest_hash\""));
    assert!(stdout.contains("\"package_manifest_hash\""));
    assert!(stdout.contains("\"compiler_ffi_entry_count\": 1"));
    assert!(stdout.contains("\"symbol\": \"nc_basal_c_add\""));
}

#[test]
fn basal_package_writes_nbb_manifest_and_hashes_ok() {
    let path = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let out_dir = temp_basal_dir("ffi_package");
    let stdout = run_basal_json(&[
        "basal",
        "package",
        path.to_str().expect("utf8 path"),
        "--out",
        out_dir.to_str().expect("utf8 out"),
        "--json",
    ]);
    assert!(stdout.contains("\"schema\": \"basal.package.result.v1\""));
    assert!(stdout.contains("\"status\": \"packaged\""));
    assert!(stdout.contains("\"ghor_compatible\": true"));
    assert!(stdout.contains("\"package_manifest_hash\""));
    assert!(stdout.contains("\"ffi_link_manifest_hash\""));
    assert!(stdout.contains("\"nbb_hash\""));
    assert!(out_dir.join("nc_root_ffi_foreign_c-1.nbp.tar.gz").exists());
    assert!(out_dir.join("manifest.json").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out_dir.join("manifest.json")).expect("manifest"))
            .expect("manifest json");
    assert_eq!(manifest["schema"], "basal.package.v1");
    assert_eq!(manifest["nbb"]["magic"], "NBB0");
    assert_eq!(
        manifest["ffi_link_manifest"]["schema"],
        "ghor.ffi_manifest.v1"
    );
}

#[test]
fn basal_registry_publish_install_vendor_ok() {
    let path = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let registry = temp_basal_dir("ffi_registry");
    let publish = run_basal_json(&[
        "basal",
        "publish",
        path.to_str().expect("utf8 path"),
        "--registry",
        registry.to_str().expect("utf8 registry"),
        "--json",
    ]);
    assert!(publish.contains("\"schema\": \"basal.registry.publish.v1\""));
    assert!(publish.contains("\"status\": \"published\""));
    assert!(publish.contains("\"ffi_link_manifest_hash\""));
    assert!(registry.join("index.json").exists());
    assert!(registry
        .join("index/nc_root_ffi_foreign_c/versions.jsonl")
        .exists());
    assert!(registry
        .join("packages/nc_root_ffi_foreign_c/1/nc_root_ffi_foreign_c-1.nbp.tar.gz")
        .exists());

    let install_root = temp_basal_dir("ffi_install");
    let install = run_basal_json(&[
        "basal",
        "install",
        "nC.root.ffi.foreign_c",
        "--registry",
        registry.to_str().expect("utf8 registry"),
        "--root",
        install_root.to_str().expect("utf8 install"),
        "--json",
    ]);
    assert!(install.contains("\"schema\": \"basal.registry.install.v1\""));
    assert!(install.contains("\"status\": \"installed\""));
    assert!(install.contains("\"validation\""));
    assert!(install_root
        .join("registry-src/nc_root_ffi_foreign_c-1/manifest.json")
        .exists());
    assert!(install_root
        .join("registry-src/nc_root_ffi_foreign_c-1/artifacts/nc_root_ffi_foreign_c.nbb")
        .exists());

    let vendor_dir = temp_basal_dir("ffi_vendor");
    let vendor = run_basal_json(&[
        "basal",
        "vendor",
        vendor_dir.to_str().expect("utf8 vendor"),
        "--registry",
        registry.to_str().expect("utf8 registry"),
        "--package",
        "nC.root.ffi.foreign_c",
        "--json",
    ]);
    assert!(vendor.contains("\"schema\": \"basal.vendor.v1\""));
    assert!(vendor.contains("\"status\": \"ok\""));
    assert!(vendor.contains("\"ghor_compatible\": true"));
    assert!(vendor_dir
        .join("nc_root_ffi_foreign_c-1/.basal-checksum.json")
        .exists());
}

#[test]
fn basal_foreign_external_bindings_lower_to_machine_plan_ok() {
    let path = fixture_path("examples/basal/ffi/foreign_zig_go_cabi.nb");
    let stdout = run_basal_json(&[
        "basal",
        "lower",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(stdout.contains("\"BasalExternalBindingLowering\""));
    assert!(stdout.contains("\"external_abi_symbol_reference\""));
    assert!(stdout.contains("\"resolved_to_external_abi_symbol\""));
    assert!(stdout.contains("\"ffi_link_manifest_hash\""));
    assert!(stdout.contains("\"schema\": \"ghor.ffi_manifest.v1\""));
    assert!(stdout.contains("\"symbol\": \"nc_basal_zig_add\""));
    assert!(stdout.contains("\"symbol\": \"nc_basal_go_add\""));
}

#[test]
fn basal_foreign_external_bindings_nbb_records_ok() {
    let path = fixture_path("examples/basal/ffi/foreign_rust_contract.nb");
    let out_path = temp_phase5_nbb_path("foreign_external_bindings");
    let out = run_ncc(&[
        "basal",
        "emit-nbb",
        path.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "basal emit-nbb must pass for foreign binding\nstdout={stdout}\nstderr={stderr}"
    );
    let read = run_basal_json(&[
        "basal",
        "read",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(read.contains("\"foreign_binding\""));
    assert!(read.contains("\"foreign_ffi_manifest\""));
    assert!(read.contains("\"ffi_link_manifest_hash\""));
    assert!(read.contains("nc_basal_rust_pair_sum"));
    assert!(read.contains("external_abi_symbol_reference"));
}

#[test]
fn basal_foreign_external_evidence_integrates_real_artifact_ok() {
    let _guard = basal_evidence_cli_lock();
    let path = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let out_path = temp_phase5_nbb_path("foreign_external_evidence");
    let out = run_ncc(&[
        "basal",
        "integrate-evidence",
        path.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    if !out.status.success() && stderr.contains("E_BASAL_EVIDENCE_EXTERNAL_LINK_001") {
        return;
    }
    assert!(
        out.status.success(),
        "basal integrate-evidence must compile/link/run external C artifact\nstdout={stdout}\nstderr={stderr}"
    );

    let evidence = run_basal_json(&[
        "basal",
        "evidence-report",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(evidence.contains("\"external_artifacts\""));
    assert!(evidence.contains("\"ffi_link_manifest\""));
    assert!(evidence.contains("\"ffi_link_manifest_hash\""));
    assert!(evidence.contains("\"language\": \"c\""));
    assert!(evidence.contains("\"symbol\": \"nc_basal_c_add\""));
    assert!(evidence.contains("\"link_status\": \"linked_and_executed\""));
    assert!(evidence.contains("basal_external_c_compile"));
    assert!(evidence.contains("basal_external_c_link"));
    assert!(evidence.contains("basal_external_c_run"));

    let read = run_basal_json(&[
        "basal",
        "read",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(read.contains("\"magic\": \"NBB0\""));
    assert!(read.contains("\"external_artifacts\""));

    let seal = run_basal_json(&[
        "basal",
        "verify-seal",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(seal.contains("\"status\": \"seal_verified\""));
}

#[test]
fn basal_foreign_cpp_abi_full_evidence_integrates_real_artifacts_ok() {
    let _guard = basal_evidence_cli_lock();
    let path = fixture_path("examples/basal/ffi/foreign_cpp_abi_full_contract.nb");
    let out_path = temp_phase5_nbb_path("foreign_cpp_abi_full_evidence");
    let out = run_ncc(&[
        "basal",
        "integrate-evidence",
        path.to_str().expect("utf8 source"),
        "--out",
        out_path.to_str().expect("utf8 out"),
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    if !out.status.success() && stderr.contains("E_BASAL_EVIDENCE_EXTERNAL_LINK_001") {
        return;
    }
    assert!(
        out.status.success(),
        "basal integrate-evidence must compile/link/run C++ ABI artifacts\nstdout={stdout}\nstderr={stderr}"
    );

    let evidence = run_basal_json(&[
        "basal",
        "evidence-report",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(evidence.contains("\"external_artifacts\""));
    assert!(evidence.contains("\"language\": \"cpp\""));
    assert!(evidence.contains("\"abi\": \"itanium-cxxabi\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_call_template_int\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_vtable_value\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_destructor_count\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_catch_exception_value\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_static_value\""));
    assert!(evidence.contains("\"symbol\": \"nc_cpp_shared_value\""));
    assert!(evidence.contains("\"link_status\": \"linked_and_executed\""));
    assert!(evidence.contains("basal_external_cpp_compile"));
    assert!(evidence.contains("basal_external_cpp_driver_compile"));
    assert!(evidence.contains("basal_external_cpp_link"));
    assert!(evidence.contains("basal_external_cpp_run"));

    let read = run_basal_json(&[
        "basal",
        "read",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(read.contains("\"magic\": \"NBB0\""));
    assert!(read.contains("nc_cpp_vtable_value"));
    assert!(read.contains("nc_cpp_catch_exception_value"));

    let seal = run_basal_json(&[
        "basal",
        "verify-seal",
        out_path.to_str().expect("utf8 nbb"),
        "--json",
    ]);
    assert!(seal.contains("\"status\": \"seal_verified\""));
}

#[test]
fn basal_foreign_language_unknown_fails() {
    assert_basal_fail_contains(
        "examples/basal/ffi/negative/foreign_unknown_language_fail.nb",
        "E_BASAL_FOREIGN_LANGUAGE_001",
    );
}

#[test]
fn basal_foreign_missing_symbol_fails() {
    assert_basal_fail_contains(
        "examples/basal/ffi/negative/foreign_missing_symbol_fail.nb",
        "E_BASAL_FOREIGN_SYMBOL_001",
    );
}

#[test]
fn basal_phase6_missing_machine_ref_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/evidence_missing_machine_ref_fail.nb",
        "E_BASAL_LOWERING_UNRESOLVED_MACHINE_001",
    );
}

#[test]
fn basal_phase6_machine_doctor_fail_simulated() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/evidence_machine_doctor_fail_simulated.nb",
        "E_BASAL_EVIDENCE_MACHINE_DOCTOR_001",
    );
}

#[test]
fn basal_phase6_verify_object_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/evidence_verify_object_fail.nb",
        "E_BASAL_EVIDENCE_VERIFY_OBJECT_001",
    );
}

#[test]
fn basal_phase6_roundtrip_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/evidence_roundtrip_fail.nb",
        "E_BASAL_EVIDENCE_ROUNDTRIP_001",
    );
}

#[test]
fn basal_phase6_seal_missing_evidence_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/seal_missing_evidence_fail.nb",
        "E_BASAL_SEAL_MISSING_EVIDENCE_001",
    );
}

#[test]
fn basal_phase6_seal_hash_mismatch_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/seal_hash_mismatch_fail.nb",
        "E_BASAL_SEAL_MISMATCH_001",
    );
}

#[test]
fn basal_phase6_modified_nbb_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/seal_modified_nbb_fail.nb",
        "E_BASAL_SEAL_VERIFY_001",
    );
}

#[test]
fn basal_phase6_fake_evidence_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/seal_fake_evidence_fail.nb",
        "E_BASAL_EVIDENCE_FAKE_001",
    );
}

#[test]
fn basal_phase6_non_deterministic_hash_fail() {
    assert_basal_integrate_fail_contains(
        "examples/basal/phase6/negative/seal_non_deterministic_hash_fail.nb",
        "E_BASAL_SEAL_NON_DETERMINISTIC_001",
    );
}
