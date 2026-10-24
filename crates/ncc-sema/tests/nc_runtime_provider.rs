use std::fs;
use std::path::PathBuf;

#[test]
fn nc_runtime_provider_is_semantically_closed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut program = ncc_parser::Program {
        modules: Vec::new(),
        packages: Vec::new(),
        uses: Vec::new(),
        containers: Vec::new(),
    };
    for relative in [
        "runtime/nc/text_format.n",
        "runtime/nc/text_escape.n",
        "runtime/nc/text_parse.n",
        "runtime/nc/unicode_tables.n",
        "runtime/nc/unicode.n",
        "runtime/nc/json.n",
    ] {
        let source = fs::read_to_string(root.join(relative)).expect("runtime source");
        let mut provider = ncc_parser::parse(ncc_lexer::lex(&source).expect("runtime lex"))
            .expect("runtime parse");
        program.modules.append(&mut provider.modules);
        program.packages.append(&mut provider.packages);
        program.uses.append(&mut provider.uses);
        program.containers.append(&mut provider.containers);
    }
    ncc_sema::analyze(&program).expect("combined runtime sema");
    assert!(program
        .containers
        .iter()
        .any(|container| container.name == "nccJsonRuntime"));
}
