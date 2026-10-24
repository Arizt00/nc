#[test]
fn compact_process_session_is_raw_typed_and_buffer_driven() {
    let source = include_str!("../../../examples/runtime/families/process_session_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process session fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process session fixture must parse");
    ncc_sema::analyze(&program).expect("process session fixture must pass semantic analysis");
}

#[test]
fn process_session_accepts_null_as_current_wts_server_handle() {
    let source = r#"
use <process:session>;
sessionNullHandle :: runtime {
    fn:
        run() -> bool {
            outBuffer >> addr <- null;
            outBytes >> int <- 0;
            return sessionQuery(null, sessionCurrentId(), 4, &outBuffer, &outBytes);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null session fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null session fixture must parse");
    ncc_sema::analyze(&program).expect("null must be a valid current WTS server handle");
}

#[test]
fn process_session_rejects_text_as_a_raw_wts_handle() {
    let source = r#"
use <process:session>;
sessionBadHandle :: runtime {
    fn:
        run() -> bool {
            outBuffer >> addr <- null;
            outBytes >> int <- 0;
            return sessionQuery("managed", 1, 4, &outBuffer, &outBytes);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process session fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process session fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string handle must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
