#[test]
fn compact_process_service_is_raw_typed_and_handle_driven() {
    let source = include_str!("../../../examples/runtime/families/process_service_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process service fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process service fixture must parse");
    ncc_sema::analyze(&program).expect("process service fixture must pass semantic analysis");
}

#[test]
fn compact_process_service_accepts_direct_null_for_optional_raw_addresses() {
    let source = r#"
use <process:service>;
nullService :: runtime {
    fn:
        run() -> bool {
            manager >> addr <- serviceManagerOpen(null, null, 1);
            if (manager != null) { serviceClose(manager); }
            return true;
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null service fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null service fixture must parse");
    ncc_sema::analyze(&program).expect("null must coerce to an explicit raw address parameter");
}

#[test]
fn compact_process_service_rejects_managed_string_as_scm_handle() {
    let source = r#"
use <process:service>;
badService :: runtime {
    fn:
        run() -> bool {
            return serviceClose("not-a-handle");
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process service fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process service fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string handle must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
