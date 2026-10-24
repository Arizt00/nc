#[test]
fn compact_memory_buffer_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/memory_buffer_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual buffer fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual buffer fixture must parse");
    ncc_sema::analyze(&program).expect("manual buffer fixture must pass semantic analysis");
}

#[test]
fn memory_buffer_rejects_managed_string_as_descriptor() {
    let source = r#"
use <memory:buffer>;
badBuffer :: runtime {
    fn:
        run() -> bool { return bufferWrite8("managed", 0, 1); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative buffer fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative buffer fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed descriptor must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
