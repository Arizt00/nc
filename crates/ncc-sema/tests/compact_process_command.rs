#[test]
fn compact_process_command_is_raw_typed_and_explicitly_closed() {
    let source = include_str!("../../../examples/runtime/families/process_command_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process command fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process command fixture must parse");
    ncc_sema::analyze(&program).expect("process command fixture must pass semantic analysis");
}

#[test]
fn compact_process_command_rejects_non_address_pipe_destination() {
    let source = r#"
use <process:command>;
badCommand :: runtime {
    fn:
        run() -> bool {
            pipe >> addr <- commandPipeOpen(false, false);
            bytes >> int <- commandPipeRead(pipe, "managed", 7);
            commandPipeRelease(pipe);
            return bytes == 7;
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative command fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative command fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string buffer must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
