fn analyze(source: &str) -> Result<ncc_sema::SemanticResult, String> {
    let tokens = ncc_lexer::lex(source).expect("raw virtual-memory fixture must lex");
    let program = ncc_parser::parse(tokens).expect("raw virtual-memory fixture must parse");
    ncc_sema::analyze(&program)
}

#[test]
fn raw_virtual_memory_prelude_is_typed_without_imports() {
    analyze(
        r#"
rawVmSema {
    fn:
        run() -> bool {
            page >> int = vmPageSize();
            block >> p<byte> = alignedAlloc(page, 64);
            reservation >> p<byte> = vmReserve(null, page, 0);
            guard block != null && reservation != null else return false;
            vmCommit(reservation, page, 3);
            vmProtect(reservation, page, 1);
            vmAdvise(reservation, page, 4);
            vmExecutable(reservation, page);
            alignedFree(block);
            return vmRelease(reservation, page);
        }
    ref main ->> @fn(run);
}
"#,
    )
    .expect("the raw virtual-memory prelude must type-check");
}

#[test]
fn raw_virtual_memory_rejects_wrong_types_and_missing_abi_arguments() {
    let wrong_type = analyze(
        r#"
rawVmWrongType {
    fn:
        run() -> bool {
            block >> p<byte> = alignedAlloc("128", 64);
            return block != null;
        }
    ref main ->> @fn(run);
}
"#,
    )
    .expect_err("a string is not a byte count");
    assert!(wrong_type.contains("alignedAlloc"), "{wrong_type}");

    let wrong_arity = analyze(
        r#"
rawVmWrongArity {
    fn:
        run() -> bool {
            block >> p<byte> = vmReserve(null, 4096);
            return block != null;
        }
    ref main ->> @fn(run);
}
"#,
    )
    .expect_err("native reserve flags are an explicit ABI argument");
    assert!(wrong_arity.contains("vmReserve"), "{wrong_arity}");
}
