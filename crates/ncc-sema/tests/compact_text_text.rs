#[test]
fn compact_text_text_import_resolves_with_manual_lifetimes() {
    let source = include_str!("../../../examples/runtime/families/text_basic.n");
    let tokens = ncc_lexer::lex(source).expect("text fixture must lex");
    let program = ncc_parser::parse(tokens).expect("text fixture must parse");
    ncc_sema::analyze(&program).expect("text fixture must pass semantic analysis");
}
