#[test]
fn compact_memory_bytes_manual_fixture_is_valid() {
    let source = include_str!("../../../examples/runtime/families/memory_bytes_manual_raw.n");
    let tokens = ncc_lexer::lex(source).expect("manual bytes fixture must lex");
    let program = ncc_parser::parse(tokens).expect("manual bytes fixture must parse");
    ncc_sema::analyze(&program).expect("manual bytes fixture must pass semantic analysis");
}

#[test]
fn compact_memory_bytes_rejects_managed_hex_result() {
    let source = r#"
use <memory:bytes>;
badBytes :: runtime {
    fn:
        run() -> bool {
            value >> str <- hexToBytes("6e43");
            return value == "";
        }
    ref main ->> run;
    fa:
        return true;
}
"#;
    let tokens = ncc_lexer::lex(source).expect("negative bytes fixture must lex");
    let program = ncc_parser::parse(tokens).expect("negative bytes fixture must parse");
    let found = ncc_sema::analyze(&program).expect_err("managed bytes API must fail");
    assert!(
        found.contains("no matching function 'hexToBytes'"),
        "{found}"
    );
}
