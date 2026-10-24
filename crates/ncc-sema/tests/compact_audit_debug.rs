#[test]
fn compact_audit_debug_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/audit_debug_raw.n");
    let tokens = ncc_lexer::lex(source).expect("audit debug fixture must lex");
    let program = ncc_parser::parse(tokens).expect("audit debug fixture must parse");
    ncc_sema::analyze(&program).expect("audit debug fixture must pass semantic analysis");
}

#[test]
fn audit_debug_record_rejects_managed_buffer() {
    let source = r#"
use <audit:debug>;
badDebugRecord :: runtime {
    fn:
        run() -> bool { return debugRecord("managed", 0, 1, null, 2, 3); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit debug fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit debug fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed buffer must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}

#[test]
fn audit_debug_assert_requires_boolean_condition() {
    let source = r#"
use <audit:debug>;
badDebugAssert :: runtime {
    fn:
        run() -> bool { return debugAssert(1, null, 0); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit debug fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit debug fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("integer condition must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected bool"), "{found}");
}
