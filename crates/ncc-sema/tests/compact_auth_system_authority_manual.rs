fn analyze(source: &str) -> Result<(), String> {
    let tokens = ncc_lexer::lex(source).map_err(|error| error.to_string())?;
    let program = ncc_parser::parse(tokens).map_err(|error| error.to_string())?;
    ncc_sema::analyze(&program).map(|_| ())
}

#[test]
fn compact_auth_system_authority_fixtures_are_raw_and_typed() {
    for source in [
        include_str!("../../../examples/runtime/families/auth_system_service_manual_raw.n"),
        include_str!("../../../examples/runtime/families/auth_system_signal_manual_raw.n"),
        include_str!("../../../examples/runtime/families/auth_system_session_manual_raw.n"),
    ] {
        analyze(source).expect("manual system authority fixture must pass semantic analysis");
    }
}

#[test]
fn compact_auth_system_authority_rejects_managed_tables() {
    for (item, clear) in [
        ("service", "authServiceClear"),
        ("signal", "authSignalClear"),
        ("session", "authSystemSessionClear"),
    ] {
        let source = format!(
            "use <auth.system:{item}>;\nbadAuthority {{\nfn:\nrun() -> bool {{ return {clear}(\"managed\", 0); }}\nref main ->> run;\nfa:\nreturn true;\n}}"
        );
        let found = analyze(&source).expect_err("managed authority table must be rejected");
        assert!(found.contains("expected addr"), "{item}: {found}");
    }
}
