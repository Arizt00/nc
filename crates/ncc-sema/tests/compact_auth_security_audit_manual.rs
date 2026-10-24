#[test]
fn compact_auth_security_audit_is_raw_typed_and_explicit() {
    let source =
        include_str!("../../../examples/runtime/families/auth_security_audit_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual security audit fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual security audit fixture must parse");
    ncc_sema::analyze(&program).expect("manual security audit fixture must pass semantic analysis");
}

#[test]
fn auth_security_audit_rejects_managed_table() {
    let source = r#"
use <auth.security:audit>;
badSecurityAudit :: runtime {
    fn:
        run() -> bool { return securityAuditClear("managed", 0); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative security audit fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative security audit fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed audit table must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
