#[test]
fn compact_process_exit_is_raw_typed_and_handle_driven() {
    let source = include_str!("../../../examples/runtime/families/process_exit_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process exit fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process exit fixture must parse");
    ncc_sema::analyze(&program).expect("process exit fixture must pass semantic analysis");
}

#[test]
fn compact_process_exit_rejects_string_as_native_handle() {
    let source = r#"
use <process:exit>;
badExit :: runtime {
    fn:
        run() -> bool {
            return processTerminate("not-a-handle", 7);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process exit fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process exit fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string handle must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
