#[test]
fn compact_memory_views_are_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/memory_views_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual views fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual views fixture must parse");
    ncc_sema::analyze(&program).expect("manual views fixture must pass semantic analysis");
}

#[test]
fn memory_views_reject_managed_descriptors() {
    for (item, call) in [
        ("slice", "sliceClear(\"managed\")"),
        ("span", "spanClear(\"managed\")"),
    ] {
        let source = format!(
            r#"
use <memory:{item}>;
badView :: runtime {{
    fn:
        run() -> bool {{ return {call}; }}
    ref main ->> run;
    fa:
        return true;
}}
"#
        );
        let tokens = ncc_lexer::lex(&source).expect("negative view fixture must lex");
        let program = ncc_parser::parse(tokens).expect("negative view fixture must parse");
        let found = ncc_sema::analyze(&program).expect_err("managed view must be rejected");
        assert!(
            found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
            "{found}"
        );
        assert!(found.contains("expected addr"), "{found}");
    }
}
