#[test]
fn compact_audit_diagnostic_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/audit_diagnostic_raw.n");
    let tokens = ncc_lexer::lex(source).expect("audit diagnostic fixture must lex");
    let program = ncc_parser::parse(tokens).expect("audit diagnostic fixture must parse");
    ncc_sema::analyze(&program).expect("audit diagnostic fixture must pass semantic analysis");
}

#[test]
fn audit_diagnostic_rejects_managed_message() {
    let source = r#"
use <audit:diagnostic>;
badDiagnosticMessage :: runtime {
    fn:
        run() -> bool {
            return diagWrite(null, 0, 2, 7, "managed", 7, null, 1, 1, 0);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative diagnostic fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative diagnostic fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed message must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}

#[test]
fn audit_diagnostic_copy_requires_raw_source() {
    let source = r#"
use <audit:diagnostic>;
badDiagnosticCopy :: runtime {
    fn:
        run() -> bool { return diagCopy(null, 0, "managed", 0); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative diagnostic fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative diagnostic fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed source must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
