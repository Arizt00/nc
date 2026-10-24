#[test]
fn compact_auth_system_file_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/auth_system_file_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual auth file fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual auth file fixture must parse");
    ncc_sema::analyze(&program).expect("manual auth file fixture must pass semantic analysis");
}

#[test]
fn auth_system_file_rejects_managed_table() {
    let source = r#"
use <auth.system:file>;
badAuthFile :: runtime {
    fn:
        run() -> bool { return authFileClear("managed", 0); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative auth file fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative auth file fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed auth file table must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
