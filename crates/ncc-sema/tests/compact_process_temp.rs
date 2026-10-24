#[test]
fn compact_process_temp_is_raw_typed_and_resource_explicit() {
    let source = include_str!("../../../examples/runtime/families/process_temp_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process temp fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process temp fixture must parse");
    ncc_sema::analyze(&program).expect("process temp fixture must pass semantic analysis");
}

#[test]
fn process_temp_accepts_null_for_optional_security_and_template_handles() {
    let source = r#"
use <process:temp>;
tempNullPointers :: runtime {
    fn:
        open(path >> addr) -> addr { return tempOpen(path, 0, 7, null, 3, 128, null); }
        run() -> bool { return true; }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null temp fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null temp fixture must parse");
    ncc_sema::analyze(&program).expect("null must coerce to optional raw temp addresses");
}

#[test]
fn process_temp_rejects_managed_text_as_a_wide_path() {
    let source = r#"
use <process:temp>;
tempBadPath :: runtime {
    fn:
        run() -> bool { return tempDeleteFile("managed"); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process temp fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process temp fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed path must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
