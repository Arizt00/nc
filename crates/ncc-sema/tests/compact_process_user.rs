#[test]
fn compact_process_user_is_raw_typed_and_token_explicit() {
    let source = include_str!("../../../examples/runtime/families/process_user_raw.n");
    let tokens = ncc_lexer::lex(source).expect("process user fixture must lex");
    let program = ncc_parser::parse(tokens).expect("process user fixture must parse");
    ncc_sema::analyze(&program).expect("process user fixture must pass semantic analysis");
}

#[test]
fn process_user_accepts_null_for_optional_token_and_domain_sid() {
    let source = r#"
use <process:user>;
userNullPointers :: runtime {
    fn:
        member(sid >> addr, outMember >> addr) -> bool {
            return userCheckMembership(null, sid, outMember);
        }
        createSid(sidKind >> int, sid >> addr, sizeSlot >> addr) -> bool {
            return userCreateWellKnownSid(sidKind, null, sid, sizeSlot);
        }
        run() -> bool { return true; }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("direct-null user fixture must lex");
    let program = ncc_parser::parse(tokens).expect("direct-null user fixture must parse");
    ncc_sema::analyze(&program).expect("null must coerce to optional raw user addresses");
}

#[test]
fn process_user_rejects_managed_text_as_a_token_handle() {
    let source = r#"
use <process:user>;
userBadToken :: runtime {
    fn:
        run() -> bool { return userImpersonate("managed"); }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative process user fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative process user fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed token must be rejected");
    assert!(
        found.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001"),
        "{found}"
    );
    assert!(found.contains("expected addr"), "{found}");
}
