#[test]
fn compact_text_unicode_import_resolves_with_manual_owned_outputs() {
    let source = include_str!("../../../examples/runtime/families/unicode_basic.n");
    let tokens = ncc_lexer::lex(source).expect("unicode fixture must lex");
    let program = ncc_parser::parse(tokens).expect("unicode fixture must parse");
    ncc_sema::analyze(&program).expect("unicode fixture must pass semantic analysis");
}
