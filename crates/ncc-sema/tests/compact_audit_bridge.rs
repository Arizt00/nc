#[test]
fn compact_audit_bridge_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/audit_bridge_raw.n");
    let tokens = ncc_lexer::lex(source).expect("audit bridge fixture must lex");
    let program = ncc_parser::parse(tokens).expect("audit bridge fixture must parse");
    ncc_sema::analyze(&program).expect("audit bridge fixture must pass semantic analysis");
}

#[test]
fn audit_bridge_rejects_managed_table() {
    let source = r#"
use <audit:bridge>;
badBridgeTable :: runtime {
    fn:
        run() -> bool { return bridgeActive("managed", 1, 7); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit bridge fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit bridge fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed table must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}

#[test]
fn audit_bridge_write_requires_explicit_target_address() {
    let source = r#"
use <audit:bridge>;
badBridgeTarget :: runtime {
    fn:
        run() -> bool { return bridgeWrite(null, 0, 7, "managed", 1, 2); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative audit bridge fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative audit bridge fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed target must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
