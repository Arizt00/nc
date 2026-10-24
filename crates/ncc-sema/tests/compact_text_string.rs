#[test]
fn compact_text_string_import_resolves_with_manual_lifetimes() {
    let source = include_str!("../../../examples/runtime/families/string_basic.n");
    let tokens = ncc_lexer::lex(source).expect("string fixture must lex");
    let program = ncc_parser::parse(tokens).expect("string fixture must parse");
    ncc_sema::analyze(&program).expect("string fixture must pass semantic analysis");
}
