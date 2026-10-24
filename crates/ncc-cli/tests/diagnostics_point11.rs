use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn temp_source_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc_point11_diag_{}_{}.n",
        std::process::id(),
        nanos
    ))
}

#[test]
fn parser_diagnostic_prints_header_source_line_and_caret_without_suggestion() {
    let path = temp_source_path();
    fs::write(
        &path,
        "use::stoud::io::<output>\nhello {\n    fa:\n        return true;\n}\n",
    )
    .expect("write temp source");

    let path_arg = path.to_string_lossy().to_string();
    let out = run_ncc(&["check", &path_arg]);
    let rendered = combined(&out);
    let _ = fs::remove_file(&path);

    assert!(!out.status.success(), "{rendered}");
    assert!(
        rendered.contains("error[E_PARSER_EXPECTED_SEMICOLON_001] parser"),
        "{rendered}"
    );
    assert!(rendered.contains(":2:1"), "{rendered}");
    assert!(rendered.contains("2 | hello {"), "{rendered}");
    assert!(rendered.contains("\n  | ^"), "{rendered}");
    assert!(!rendered.contains("Suggestion:"), "{rendered}");
}

#[test]
fn parser_diagnostic_keeps_source_context_for_paths_with_spaces() {
    let root = std::env::temp_dir().join(format!("ncc point11 diag spaces {}", std::process::id()));
    fs::create_dir_all(&root).expect("create temp dir with spaces");
    let path = root.join("archivo con espacios.n");
    fs::write(
        &path,
        "use::stoud::io::<output>\nhello {\n    fa:\n        return true;\n}\n",
    )
    .expect("write temp source with spaces");

    let path_arg = path.to_string_lossy().to_string();
    let out = run_ncc(&["check", &path_arg]);
    let rendered = combined(&out);
    let _ = fs::remove_dir_all(&root);

    assert!(!out.status.success(), "{rendered}");
    assert!(
        rendered.contains("error[E_PARSER_EXPECTED_SEMICOLON_001] parser"),
        "{rendered}"
    );
    assert!(rendered.contains(":2:1"), "{rendered}");
    assert!(rendered.contains("2 | hello {"), "{rendered}");
    assert!(rendered.contains("\n  | ^"), "{rendered}");
    assert!(
        !rendered.contains("source file") && !rendered.contains("not found"),
        "{rendered}"
    );
}

#[test]
fn type_contract_error_keeps_its_real_code_and_parser_subphase() {
    let path = temp_source_path();
    fs::write(
        &path,
        "gestor {\n    type:\n        Alumno {\n\n            42;\n        }\n    fa:\n        return true;\n}\n",
    )
    .expect("write invalid type contract source");

    let path_arg = path.to_string_lossy().to_string();
    let out = run_ncc(&["check", &path_arg]);
    let rendered = combined(&out);
    let _ = fs::remove_file(&path);

    assert!(!out.status.success(), "{rendered}");
    assert!(
        rendered.contains("error[E_TYPE_CONTRACT_SECTION_001] parser.type"),
        "{rendered}"
    );
    assert!(rendered.contains(":5:13"), "{rendered}");
    assert!(rendered.contains("5 |             42;"), "{rendered}");
    assert!(rendered.contains("  |             ^"), "{rendered}");
}
