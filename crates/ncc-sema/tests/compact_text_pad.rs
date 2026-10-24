#[test]
fn compact_text_pad_import_resolves_from_external_consumer() {
    let source = include_str!("../../../examples/runtime/families/pad_basic.n");
    let tokens = ncc_lexer::lex(source).expect("pad fixture must lex");
    let program = ncc_parser::parse(tokens).expect("pad fixture must parse");
    ncc_sema::analyze(&program).expect("pad fixture must pass semantic analysis");
}
