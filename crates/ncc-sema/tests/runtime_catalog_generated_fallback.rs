fn analyze_source(source: &str) -> Result<ncc_sema::SemanticResult, String> {
    let tokens = ncc_lexer::lex(source).expect("fixture must lex");
    let program = ncc_parser::parse(tokens).expect("fixture must parse");
    ncc_sema::analyze(&program)
}

#[test]
fn generated_runtime_items_remain_visible_beside_legacy_module_entries() {
    let source = r#"
use::stoud::io::<output>;
use::stoud::terminal::<clear, hideCursor, showCursor, exitRawMode>;
use nC.runtime.sleep;
use nC.runtime.time;

runtimeCatalogFallback[@runtime] {
    fn:
        run() -> bool {
            public:
                start >> int : monotonicMs();
                hideCursor();
                clear();
                sleepMs(1);
                sleepUntilMs(start + 1);
                showCursor();
                exitRawMode();
                return monotonicMs() >= start;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;

    analyze_source(source).expect("generated time and terminal items must pass sema");
}
