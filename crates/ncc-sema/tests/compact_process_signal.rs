#[test]
fn compact_process_signal_is_raw_typed_and_event_driven() {
    let source = include_str!("../../../examples/runtime/families/process_signal_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process signal fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process signal fixture must parse");
    ncc_sema::analyze(&program).expect("process signal fixture must pass semantic analysis");
}

#[test]
fn process_signal_accepts_null_for_optional_security_and_name_pointers() {
    let source = r#"
use <process:signal>;
signalNullPointers :: runtime {
    fn:
        run() -> bool {
            event >> addr <- signalCreateEvent(null, false, false, null);
            if (event != null) { return signalClose(event); }
            return false;
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null signal fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null signal fixture must parse");
    ncc_sema::analyze(&program).expect("null must coerce to optional raw signal addresses");
}

#[test]
fn process_signal_rejects_managed_text_as_an_event_handle() {
    let source = r#"
use <process:signal>;
signalBadHandle :: runtime {
    fn:
        run() -> bool { return signalSetEvent("managed"); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process signal fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process signal fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed string handle must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
