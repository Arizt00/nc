#[test]
fn compact_audit_assert_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/audit_assert_raw.n");
    let tokens = ncc_lexer::lex(source).expect("audit assert fixture must lex");
    let program = ncc_parser::parse(tokens).expect("audit assert fixture must parse");
    ncc_sema::analyze(&program).expect("audit assert fixture must pass semantic analysis");
}

#[test]
fn audit_assert_rejects_integer_as_condition() {
    let source = r#"
use <audit:assert>;
badAssertCondition :: runtime {
    fn:
        run() -> bool { return assert(1); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit assert fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit assert fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("integer condition must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected bool"), "{found}");
}

#[test]
fn audit_assert_message_requires_raw_address_and_length() {
    let source = r#"
use <audit:assert>;
badAssertMessage :: runtime {
    fn:
        run() -> bool { return assertMsg(true, "managed", 7); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit message fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit message fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed message must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
