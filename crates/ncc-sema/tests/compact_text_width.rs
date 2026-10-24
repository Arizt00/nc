#[test]
fn compact_text_width_import_resolves_to_integer_measurements() {
    let source = include_str!("../../../examples/runtime/families/width_basic.n");
    let tokens = ncc_lexer::lex(source).expect("width fixture must lex");
    let program = ncc_parser::parse(tokens).expect("width fixture must parse");
    ncc_sema::analyze(&program).expect("width fixture must pass semantic analysis");
}
