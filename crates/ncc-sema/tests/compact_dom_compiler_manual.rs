#[test]
fn compiler_manual_workspace_is_import_free_raw_and_explicit() {
    let source =
        include_str!("../../../examples/runtime/families/dom_compiler_manual_raw_no_import.n");
    let tokens = ncc_lexer::lex(source).expect("manual compiler fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual compiler fixture must parse");
    assert_eq!(
        program.uses.len(),
        1,
        "only output remains an explicit import"
    );
    ncc_sema::analyze(&program).expect("manual compiler fixture must pass semantic analysis");
}

#[test]
fn compiler_manual_workspace_rejects_managed_storage() {
    let source = r#"
badCompilerWorkspace :: runtime {
    fn:
        run() -> bool { return compilerReset("managed"); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative compiler fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative compiler fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed compiler storage must be rejected");
    assert!(found.contains("expected addr"), "{found}");
}
