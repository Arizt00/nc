#[test]
fn compact_auth_resource_family_is_raw_typed_and_explicit() {
    let source = include_str!("../../../examples/runtime/families/auth_resource_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual auth resource fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual auth resource fixture must parse");
    ncc_sema::analyze(&program).expect("manual auth resource fixture must pass semantic analysis");
}

#[test]
fn auth_resource_families_reject_managed_memory_arguments() {
    let cases = [
        ("handle", "handleWrite(null, 0, \"managed\", 1, 1, 0, 0)"),
        (
            "resource",
            "resourceWrite(null, 0, 1, null, \"managed\", 8, 1, 1, 0, 0)",
        ),
        (
            "session",
            "sessionWrite(null, 0, 1, 1, null, \"managed\", 0, 1, 0)",
        ),
    ];
    for (item, call) in cases {
        let source = format!(
            r#"
use <auth.resource:{item}>;
badAuthResource :: runtime {{
    fn:
        run() -> bool {{ return {call}; }}
    ref main ->> run;
    fa:
        return true;
}}
"#
        );
        let tokens = ncc_lexer::lex(&source).expect("negative auth resource fixture must lex");
        let program = ncc_parser::parse(tokens).expect("negative auth resource fixture must parse");
        let found = ncc_sema::analyze(&program).expect_err("managed address must be rejected");
        assert!(
            found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
            "{item}: {found}"
        );
        assert!(found.contains("expected addr"), "{item}: {found}");
    }
}
