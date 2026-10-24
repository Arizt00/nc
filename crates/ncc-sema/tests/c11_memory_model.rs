fn analyze_source(source: &str) -> Result<ncc_sema::SemanticResult, String> {
    let tokens = ncc_lexer::lex(source).expect("fixture must lex");
    let program = ncc_parser::parse(tokens).expect("fixture must parse");
    ncc_sema::analyze(&program)
}

fn program_with(call: &str) -> String {
    format!(
        r#"
use::core::sync::<atomic>;

c11MemoryModel {{
    stc:
        cell: atomic<int>;

    fn:
        run() -> bool {{
            public:
                {call}
                return true;
        }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#
    )
}

#[test]
fn canonical_core_atomic_import_and_ordered_calls_pass_sema() {
    let source = program_with(
        r#"atomicStoreOrder(@cell, 7, "release");
                atomicLoadOrder(@cell, "acquire");
                atomicFetchAdd(@cell, 2, "acq_rel");
                atomicCompareExchangeStrong(@cell, 9, 11, "seq_cst", "acquire");
                atomicWait(@cell, 11, "acquire");
                atomicNotifyAll(@cell);
                volatileStore(@cell, 13);
                volatileLoad(@cell);"#,
    );
    analyze_source(&source).expect("valid C11 atomic program must pass sema");
}

#[test]
fn release_load_is_rejected_before_backend_lowering() {
    let err = analyze_source(&program_with(r#"atomicLoadOrder(@cell, "release");"#))
        .expect_err("release load must be rejected");
    assert!(err.contains("E_ATOMIC_LOAD_ORDER_001"), "{err}");
}

#[test]
fn compare_exchange_failure_order_cannot_be_release_or_stronger() {
    let err = analyze_source(&program_with(
        r#"atomicCompareExchangeWeak(@cell, 0, 1, "acq_rel", "seq_cst");"#,
    ))
    .expect_err("stronger failure order must be rejected");
    assert!(err.contains("E_ATOMIC_CAS_FAILURE_STRONGER_001"), "{err}");
}

#[test]
fn unknown_order_has_a_canonical_diagnostic() {
    let err = analyze_source(&program_with(r#"atomicStoreOrder(@cell, 1, "fast");"#))
        .expect_err("unknown memory order must be rejected");
    assert!(err.contains("E_ATOMIC_ORDER_UNKNOWN_001"), "{err}");
}

#[test]
fn direct_atomic_payload_type_is_enforced() {
    let err = analyze_source(&program_with(
        r#"atomicStoreOrder(@cell, "wrong", "release");"#,
    ))
    .expect_err("atomic payload mismatch must be rejected");
    assert!(err.contains("E_ATOMIC_VALUE_TYPE_001"), "{err}");
}

#[test]
fn direct_atomic_store_and_fences_are_void_side_effects() {
    let source = program_with(
        r#"atomicStoreOrder(@cell, 1, "release");
                atomicThreadFence("seq_cst");
                atomicCompilerFence("acq_rel");"#,
    );
    analyze_source(&source).expect("void atomic side effects must pass as statements");
}
