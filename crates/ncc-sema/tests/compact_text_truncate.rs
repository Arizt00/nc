#[test]
fn compact_text_truncate_import_resolves_with_manual_lifetimes() {
    let source = include_str!("../../../examples/runtime/families/truncate_basic.n");
    let tokens = ncc_lexer::lex(source).expect("truncate fixture must lex");
    let program = ncc_parser::parse(tokens).expect("truncate fixture must parse");
    ncc_sema::analyze(&program).expect("truncate fixture must pass semantic analysis");
}
