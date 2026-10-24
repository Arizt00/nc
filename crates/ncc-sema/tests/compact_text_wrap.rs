#[test]
fn compact_text_wrap_import_is_typed_and_caller_owned() {
    let source = include_str!("../../../examples/runtime/families/wrap_basic.n");
    let tokens = ncc_lexer::lex(source).expect("wrap fixture must lex");
    let program = ncc_parser::parse(tokens).expect("wrap fixture must parse");
    ncc_sema::analyze(&program).expect("wrap fixture must pass semantic analysis");
}
