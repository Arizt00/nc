#[test]
fn compact_net_socket_is_raw_typed_and_explicitly_closed() {
    let source = include_str!("../../../examples/runtime/families/net_socket_raw.n");
    let tokens = ncc_lexer::lex(source).expect("socket fixture must lex");
    let program = ncc_parser::parse(tokens).expect("socket fixture must parse");
    ncc_sema::analyze(&program).expect("socket fixture must pass semantic analysis");
}

#[test]
fn compact_socket_send_rejects_non_address_data() {
    let source = r#"
use <net:socket>;
badSocket :: runtime {
    fn:
        run() -> bool {
            handle >> addr <- null;
            socketSend(handle, 7, 1, 0);
            return false;
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative socket fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative socket fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("integer data address must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
