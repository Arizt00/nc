#[test]
fn compact_process_env_is_raw_typed_and_explicitly_released() {
    let source = include_str!("../../../examples/runtime/families/process_env_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process env fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process env fixture must parse");
    ncc_sema::analyze(&program).expect("process env fixture must pass semantic analysis");
}

#[test]
fn compact_process_env_rejects_managed_destination_as_raw_buffer() {
    let source = r#"
use <process:env>;
badEnv :: runtime {
    fn:
        run() -> bool {
            written >> int <- envGet("PATH", "managed", 32);
            return written >= 0;
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative env fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative env fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string buffer must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
