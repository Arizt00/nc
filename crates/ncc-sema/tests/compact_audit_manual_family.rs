#[test]
fn compact_audit_manual_family_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/audit_manual_family_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual audit family fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual audit family fixture must parse");
    ncc_sema::analyze(&program).expect("manual audit family fixture must pass semantic analysis");
}

#[test]
fn audit_manual_families_reject_managed_memory_arguments() {
    let cases = [
        ("log", "logWrite(null, 0, 1, 2, \"managed\", 7, 0, 0)"),
        (
            "profile",
            "profileWrite(null, 0, 1, 2, 3, 4, \"managed\", 7)",
        ),
        ("status", "statusWrite(null, 0, 1, 2, \"managed\", 7, 0)"),
        (
            "trace",
            "traceWrite(null, 0, 1, 2, 3, 4, \"managed\", 7, 0)",
        ),
        ("validate", "validateAscii(\"managed\", 7)"),
    ];
    for (family, call) in cases {
        let source = format!(
            r#"
use <audit:{family}>;
badAuditMemory :: runtime {{
    fn:
        run() -> bool {{ return {call}; }}
    ref main ->> run;
    fa:
        return true;
}}
"#
        );
        let tokens = ncc_lexer::lex(&source).expect("negative audit fixture must lex");
        let program = ncc_parser::parse(tokens).expect("negative audit fixture must parse");
        let found = ncc_sema::analyze(&program).expect_err("managed address must be rejected");
        assert!(
            found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
            "{family}: {found}"
        );
        assert!(found.contains("expected addr"), "{family}: {found}");
    }
}
