#[test]
fn compact_process_args_is_raw_typed_and_explicitly_released() {
    let source = include_str!("../../../examples/runtime/families/process_args_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process args fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process args fixture must parse");
    ncc_sema::analyze(&program).expect("process args fixture must pass semantic analysis");
}

#[test]
fn compact_process_args_rejects_non_integer_raw_index() {
    let source = r#"
use <process:args>;
badArgs :: runtime {
    fn:
        run() -> bool {
            handle >> addr <- argsOpen();
            value >> str <- argsAt(handle, "one");
            argsRelease(handle);
            return value == "";
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process args fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process args fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("string index must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected int"), "{found}");
}
