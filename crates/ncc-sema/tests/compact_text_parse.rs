#[test]
fn compact_text_parse_import_resolves_with_caller_owned_outputs() {
    let source = include_str!("../../../examples/runtime/families/parse_basic.n");
    let tokens = ncc_lexer::lex(source).expect("parse fixture must lex");
    let program = ncc_parser::parse(tokens).expect("parse fixture must parse");
    ncc_sema::analyze(&program).expect("parse fixture must pass semantic analysis");
}
