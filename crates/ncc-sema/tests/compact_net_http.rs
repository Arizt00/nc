#[test]
fn compact_net_http_import_is_typed_raw_and_caller_owned() {
    let source = include_str!("../../../examples/runtime/families/http_basic.n");
    let tokens = ncc_lexer::lex(source).expect("HTTP fixture must lex");
    let program = ncc_parser::parse(tokens).expect("HTTP fixture must parse");
    ncc_sema::analyze(&program).expect("HTTP fixture must pass semantic analysis");
}

#[test]
fn compact_net_http_rejects_non_address_response_handles() {
    let source = r#"
use <net:http>;
badHttp :: runtime {
    fn:
        run() -> bool {
            return httpRelease(7);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative HTTP fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative HTTP fixture must parse");
    let diagnostics = ncc_sema::analyze(&program).expect_err("integer handle must be rejected");
    assert!(
        diagnostics.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001")
            && diagnostics.to_ascii_lowercase().contains("expected addr"),
        "unexpected diagnostics: {diagnostics}"
    );
}
