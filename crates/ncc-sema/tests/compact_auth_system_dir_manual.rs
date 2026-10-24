#[test]
fn compact_auth_system_dir_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/auth_system_dir_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual auth dir fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual auth dir fixture must parse");
    ncc_sema::analyze(&program).expect("manual auth dir fixture must pass semantic analysis");
}

#[test]
fn auth_system_dir_rejects_managed_table() {
    let source = r#"
use <auth.system:dir>;
badAuthDir :: runtime {
    fn:
        run() -> bool { return authDirClear("managed", 0); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative auth dir fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative auth dir fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed auth dir table must be rejected");
    assert!(found.contains("expected addr"), "{found}");
}
