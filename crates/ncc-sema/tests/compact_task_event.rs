#[test]
fn compact_task_event_is_raw_typed_and_handle_explicit() {
    let source = include_str!("../../../examples/runtime/families/task_event_raw.n");
    let tokens = ncc_lexer::lex(source).expect("task event fixture must lex");
    let program = ncc_parser::parse(tokens).expect("task event fixture must parse");
    ncc_sema::analyze(&program).expect("task event fixture must pass semantic analysis");
}

#[test]
fn task_event_accepts_null_for_optional_attributes_and_name() {
    let source = r#"
use <task:event>;
eventNullPointers :: runtime {
    fn:
        run() -> bool {
            event >> addr <- eventCreate(null, false, false, null);
            return eventClose(event);
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null event fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null event fixture must parse");
    ncc_sema::analyze(&program).expect("null must coerce to optional event addresses");
}

#[test]
fn task_event_rejects_managed_text_as_a_handle() {
    let source = r#"
use <task:event>;
eventBadHandle :: runtime {
    fn:
        run() -> bool { return eventSet("managed"); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative task event fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative task event fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed handle must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
