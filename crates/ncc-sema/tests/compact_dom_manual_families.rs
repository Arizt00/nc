#[test]
fn remaining_dom_families_are_import_free_raw_and_explicit() {
    for (name, source) in [
        (
            "ai",
            include_str!("../../../examples/runtime/families/dom_ai_manual_raw_no_import.n"),
        ),
        (
            "high",
            include_str!("../../../examples/runtime/families/dom_high_manual_raw_no_import.n"),
        ),
        (
            "inspect",
            include_str!("../../../examples/runtime/families/dom_inspect_manual_raw_no_import.n"),
        ),
        (
            "tooling",
            include_str!("../../../examples/runtime/families/dom_tooling_manual_raw_no_import.n"),
        ),
    ] {
        let tokens = ncc_lexer::lex(source).unwrap_or_else(|error| panic!("{name} lex: {error}"));
        let program =
            ncc_parser::parse(tokens).unwrap_or_else(|error| panic!("{name} parse: {error:?}"));
        assert_eq!(program.uses.len(), 1, "{name}: only output is imported");
        ncc_sema::analyze(&program).unwrap_or_else(|error| panic!("{name} sema: {error}"));
    }
}

#[test]
fn remaining_dom_families_reject_managed_storage() {
    for (name, call) in [
        ("ai", "aiClear(\"managed\", 0)"),
        ("high", "highClear(\"managed\", 0)"),
        ("inspect", "inspectClear(\"managed\", 0)"),
        ("tooling", "toolingClear(\"managed\", 0)"),
    ] {
        let source = format!(
            r#"
badDomManual :: runtime {{
    fn:
        run() -> bool {{ return {call}; }}
    ref main ->> run;
    fa:
        return true;
}}
"#
        );
        let tokens = ncc_lexer::lex(&source).unwrap_or_else(|error| panic!("{name} lex: {error}"));
        let program =
            ncc_parser::parse(tokens).unwrap_or_else(|error| panic!("{name} parse: {error:?}"));
        let found = ncc_sema::analyze(&program).expect_err("managed DOM storage must be rejected");
        assert!(found.contains("expected addr"), "{name}: {found}");
    }
}
