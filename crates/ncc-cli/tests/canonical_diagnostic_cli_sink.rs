use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn temp_case() -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ncc-canonical-cli-sink-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create temporary case");
    let source = root.join("gestor invalid type.n");
    fs::write(
        &source,
        "gestor {\n    type:\n        Alumno {\n\n            42;\n        }\n    fa:\n        return true;\n}\n",
    )
    .expect("write invalid source");
    (root, source)
}

fn source_arg(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn check_emits_one_canonical_jsonl_diagnostic_without_terminal_text() {
    let (root, source) = temp_case();
    let output = run_ncc(&[
        "check".into(),
        source_arg(&source),
        "--diagnostic-format".into(),
        "jsonl".into(),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    let lines = stdout.lines().collect::<Vec<_>>();

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
    assert_eq!(lines.len(), 1, "expected one JSONL record: {stdout}");

    let diagnostic: serde_json::Value =
        serde_json::from_str(lines[0]).expect("valid canonical JSON diagnostic");
    assert_eq!(diagnostic["transport"], "nc.diagnostic.jsonl.v1");
    assert_eq!(diagnostic["abi"], "nc.diagnostic.machine.v2");
    assert_eq!(diagnostic["code"], "E_TYPE_CONTRACT_SECTION_001");
    assert_eq!(diagnostic["phase"], "parser.type");
    assert_eq!(diagnostic["line"], 5);
    assert_eq!(diagnostic["column"], 13);
    assert_eq!(
        diagnostic["machine_record_hex"]
            .as_str()
            .expect("machine record hex")
            .len(),
        ncc_diagnostics::DIAGNOSTIC_RECORD_BYTES * 2
    );

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn check_writes_a_decodable_ndiag_file_with_source_identity() {
    let (root, source) = temp_case();
    let diagnostic_path = root.join("diagnostics").join("check.ndiag");
    let output = run_ncc(&[
        "check".into(),
        source_arg(&source),
        "--diagnostic-format=ndiag".into(),
        format!("--diagnostic-out={}", diagnostic_path.to_string_lossy()),
    ]);

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(output.stdout.is_empty(), "unexpected stdout bytes");
    assert!(output.stderr.is_empty(), "unexpected stderr bytes");

    let encoded = fs::read(&diagnostic_path).expect("NDIAG output file");
    let decoded = ncc_diagnostics::decode_ndiag(&encoded).expect("decodable NDIAG stream");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].code, "E_TYPE_CONTRACT_SECTION_001");
    assert_eq!(decoded[0].phase, "parser.type");
    assert_eq!(decoded[0].span.line, 5);
    assert_eq!(decoded[0].span.column, 13);
    assert!(
        decoded[0].file.ends_with("gestor invalid type.n"),
        "unexpected source identity: {}",
        decoded[0].file
    );

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn run_preserves_the_same_parser_code_phase_and_span_in_jsonl() {
    let (root, source) = temp_case();
    let output = run_ncc(&[
        "run".into(),
        source_arg(&source),
        "--diagnostic-format=jsonl".into(),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
    let diagnostic: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("one canonical JSON diagnostic");
    assert_eq!(diagnostic["code"], "E_TYPE_CONTRACT_SECTION_001");
    assert_eq!(diagnostic["phase"], "parser.type");
    assert_eq!(diagnostic["line"], 5);
    assert_eq!(diagnostic["column"], 13);

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn terminal_sink_keeps_the_human_source_excerpt_and_caret() {
    let (root, source) = temp_case();
    let output = run_ncc(&[
        "--diagnostic-format".into(),
        "terminal".into(),
        "check".into(),
        source_arg(&source),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(
        stderr.contains("error[E_TYPE_CONTRACT_SECTION_001] parser.type"),
        "{stderr}"
    );
    assert!(stderr.contains(":5:13"), "{stderr}");
    assert!(stderr.contains("5 |             42;"), "{stderr}");
    assert!(stderr.contains("  |             ^"), "{stderr}");
    assert!(
        !stderr.contains("\u{1b}["),
        "ANSI escaped despite NC_NO_COLOR"
    );

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn invalid_global_diagnostic_format_is_itself_a_typed_diagnostic() {
    let output = run_ncc(&[
        "--diagnostic-format".into(),
        "xml".into(),
        "check".into(),
        "unused.n".into(),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(
        stderr.contains("error[E_CLI_DIAGNOSTIC_FORMAT_002] cli.diagnostics"),
        "{stderr}"
    );
    assert!(
        stderr.contains("unknown diagnostic format 'xml'"),
        "{stderr}"
    );
}

#[test]
fn tooling_and_package_entrypoints_retain_their_canonical_family() {
    let cases = [
        (vec!["nfmt"], "NCC-CLI-0001", "tooling.nfmt", 12_u64),
        (vec!["nlint"], "NCC-CLI-0001", "tooling.nlint", 12),
        (vec!["nide"], "NCC-CLI-0001", "tooling.nide", 12),
        (vec!["nclsp"], "NCC-CLI-0001", "tooling.nclsp", 12),
        (vec!["npkg"], "NCC-CLI-0001", "ghor.npkg", 13),
        (
            vec!["package", "unknown-action"],
            "NCC-CLI-0001",
            "package",
            13,
        ),
        (vec!["ghor", "unknown-action"], "E_GHOR_CLI_001", "ghor", 13),
    ];

    for (args, expected_code, expected_phase, expected_domain) in cases {
        let mut owned_args = args.into_iter().map(str::to_string).collect::<Vec<_>>();
        owned_args.push("--diagnostic-format=jsonl".into());
        let output = run_ncc(&owned_args);
        let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

        assert!(!output.status.success(), "command unexpectedly passed");
        assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
        let diagnostic: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("one canonical JSON diagnostic");
        assert_eq!(diagnostic["code"], expected_code, "{stdout}");
        assert_eq!(diagnostic["phase"], expected_phase, "{stdout}");
        assert_eq!(diagnostic["domain"], expected_domain, "{stdout}");
    }
}

#[test]
fn actionable_command_errors_cross_the_process_without_legacy_reparsing() {
    let cases = [
        (vec!["mct", "unknown-action"], "mct", 7_u64),
        (vec!["compiler", "unknown-action"], "mct.compiler", 7),
        (vec!["runtime", "unknown-action"], "runtime", 9),
        (vec!["high-level", "unknown-action"], "high-level", 11),
        (vec!["nbuild"], "tooling.nbuild", 12),
    ];

    for (args, expected_phase, expected_domain) in cases {
        let mut owned_args = args.into_iter().map(str::to_string).collect::<Vec<_>>();
        owned_args.push("--diagnostic-format=jsonl".into());
        let output = run_ncc(&owned_args);
        let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

        assert!(!output.status.success(), "command unexpectedly passed");
        assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
        let diagnostic: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("one canonical JSON diagnostic");
        assert_eq!(diagnostic["code"], "NCC-CLI-0001", "{stdout}");
        assert_eq!(diagnostic["phase"], expected_phase, "{stdout}");
        assert_eq!(diagnostic["domain"], expected_domain, "{stdout}");
        assert!(
            diagnostic["help"]
                .as_str()
                .is_some_and(|help| help.contains("--help")),
            "{stdout}"
        );
    }
}

#[test]
fn filesystem_failure_retains_path_and_cause_linkage_in_jsonl_and_ndiag() {
    let (root, _) = temp_case();
    let missing = root.join("missing formatter input.n");
    let jsonl = run_ncc(&[
        "nfmt".into(),
        source_arg(&missing),
        "--diagnostic-format=jsonl".into(),
    ]);
    let stdout = String::from_utf8(jsonl.stdout).expect("JSONL is UTF-8");
    let stderr = String::from_utf8(jsonl.stderr).expect("stderr is UTF-8");
    let entries = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("canonical JSON"))
        .collect::<Vec<_>>();

    assert!(!jsonl.status.success(), "command unexpectedly passed");
    assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
    assert_eq!(entries.len(), 2, "expected parent and cause: {stdout}");
    assert_eq!(entries[0]["code"], "E_CLI_FILESYSTEM_001");
    assert_eq!(entries[0]["phase"], "tooling.nfmt");
    assert_eq!(entries[0]["file"], source_arg(&missing));
    assert_eq!(entries[0]["cause_index"], 1);
    assert_eq!(entries[0]["cause_count"], 1);
    assert_eq!(entries[1]["severity"], "note");
    assert_eq!(entries[1]["code"], "N_CLI_FILESYSTEM_CAUSE_001");
    assert_eq!(entries[1]["phase"], "tooling.nfmt.cause");

    let diagnostic_path = root.join("diagnostics").join("filesystem.ndiag");
    let ndiag = run_ncc(&[
        "nfmt".into(),
        source_arg(&missing),
        "--diagnostic-format=ndiag".into(),
        format!("--diagnostic-out={}", diagnostic_path.to_string_lossy()),
    ]);
    assert!(!ndiag.status.success(), "command unexpectedly passed");
    let decoded =
        ncc_diagnostics::decode_ndiag(&fs::read(&diagnostic_path).expect("NDIAG output file"))
            .expect("decodable NDIAG stream");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].code, "E_CLI_FILESYSTEM_001");
    assert_eq!(decoded[0].cause_index, 1);
    assert_eq!(decoded[0].cause_count, 1);
    assert_eq!(decoded[1].code, "N_CLI_FILESYSTEM_CAUSE_001");

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn package_json_failure_retains_manifest_span_and_typed_cause() {
    let (root, _) = temp_case();
    let manifest = root.join("ncc.package.json");
    fs::write(
        &manifest,
        "{\n  \"schema\": \"ncc.package.v1\",\n  \"dependencies\": ]\n}\n",
    )
    .expect("write malformed package manifest");
    let output = run_ncc(&[
        "package".into(),
        "check".into(),
        "--path".into(),
        source_arg(&root),
        "--diagnostic-format=jsonl".into(),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    let entries = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("canonical JSON"))
        .collect::<Vec<_>>();

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
    assert_eq!(entries.len(), 2, "expected parent and cause: {stdout}");
    assert_eq!(entries[0]["code"], "E_CLI_JSON_001");
    assert_eq!(entries[0]["phase"], "package.manifest");
    assert_eq!(entries[0]["domain"], 13);
    assert_eq!(entries[0]["file"], source_arg(&manifest));
    assert_eq!(entries[0]["line"], 3);
    assert!(entries[0]["column"]
        .as_u64()
        .is_some_and(|column| column > 0));
    assert_eq!(entries[0]["cause_index"], 1);
    assert_eq!(entries[0]["cause_count"], 1);
    assert_eq!(entries[1]["severity"], "note");
    assert_eq!(entries[1]["code"], "N_CLI_JSON_CAUSE_001");
    assert_eq!(entries[1]["line"], 3);

    fs::remove_dir_all(root).expect("clean temporary case");
}

#[test]
fn ghor_manifest_filesystem_failure_retains_path_and_typed_cause() {
    let (root, _) = temp_case();
    let manifest = root.join("missing ghor.toml");
    let output = run_ncc(&[
        "ghor".into(),
        "manifest".into(),
        "--manifest-path".into(),
        source_arg(&manifest),
        "--diagnostic-format=jsonl".into(),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("JSONL is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    let entries = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("canonical JSON"))
        .collect::<Vec<_>>();

    assert!(!output.status.success(), "command unexpectedly passed");
    assert!(stderr.is_empty(), "unexpected terminal output: {stderr}");
    assert_eq!(entries.len(), 2, "expected parent and cause: {stdout}");
    assert_eq!(entries[0]["code"], "E_CLI_FILESYSTEM_001");
    assert_eq!(entries[0]["phase"], "ghor.manifest");
    assert_eq!(entries[0]["domain"], 13);
    assert_eq!(entries[0]["file"], source_arg(&manifest));
    assert_eq!(entries[0]["cause_index"], 1);
    assert_eq!(entries[0]["cause_count"], 1);
    assert_eq!(entries[1]["severity"], "note");
    assert_eq!(entries[1]["code"], "N_CLI_FILESYSTEM_CAUSE_001");
    assert_eq!(entries[1]["phase"], "ghor.manifest.cause");
    assert_eq!(entries[1]["file"], source_arg(&manifest));

    fs::remove_dir_all(root).expect("clean temporary case");
}
