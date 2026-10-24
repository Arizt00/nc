use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const JSON_SYMBOLS: &[&str] = &[
    "jsonParse",
    "jsonValidate",
    "jsonParseFile",
    "jsonStringify",
    "jsonStringifyPretty",
    "jsonSaveFile",
    "jsonClone",
    "jsonRelease",
    "jsonKind",
    "jsonIsNull",
    "jsonEquals",
    "jsonGetString",
    "jsonGetBool",
    "jsonGetInt",
    "jsonGetFloat",
    "jsonGetNumberText",
    "jsonGetStringOr",
    "jsonGetBoolOr",
    "jsonGetIntOr",
    "jsonGetFloatOr",
    "jsonArrayLen",
    "jsonArrayGet",
    "jsonArrayPush",
    "jsonArrayInsert",
    "jsonArrayRemove",
    "jsonObjectLen",
    "jsonObjectHas",
    "jsonObjectGet",
    "jsonObjectKeyAt",
    "jsonObjectSet",
    "jsonObjectRemove",
    "jsonPointer",
    "jsonNewNull",
    "jsonNewBool",
    "jsonNewInt",
    "jsonNewFloat",
    "jsonNewString",
    "jsonNewArray",
    "jsonNewObject",
    "jsonErrorCode",
    "jsonErrorName",
    "jsonErrorMessage",
    "jsonErrorLine",
    "jsonErrorColumn",
    "jsonErrorPath",
    "jsonClearError",
];

fn runtime_program() -> ncc_parser::Program {
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
    program
}

#[test]
fn all_json_runtime_symbols_materialize_as_mct_function_bodies() {
    let program = runtime_program();
    ncc_sema::analyze(&program).expect("runtime sema");
    let ir =
        ncc_ir::lower_with_backend_profile(program, ncc_ir::BackendLoweringProfileIR::nc_sysv64());
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config(
        &ir,
        ncc_ir::mct_llvm_pass_pipeline(ncc_ir::MctLlvmOptimizationLevelIR::O3),
        ncc_ir::MctLlvmDebugConfigIR {
            enabled: false,
            source_filename: "runtime/nc/json.n".to_string(),
            source_directory: ".".to_string(),
            producer: "ncc runtime test".to_string(),
            dwarf_version: 5,
            debug_info_version: 3,
            emission_kind: "NoDebug",
        },
    );
    let emitted = module
        .functions
        .iter()
        .filter(|function| {
            function.status == ncc_ir::MctLlvmFunctionStatusIR::Emitted
                && function.symbol_name.starts_with("nccJsonRuntime_")
        })
        .filter_map(|function| function.source_name.strip_prefix("nccJsonRuntime."))
        .collect::<BTreeSet<_>>();
    let missing = JSON_SYMBOLS
        .iter()
        .copied()
        .filter(|symbol| !emitted.contains(symbol))
        .collect::<Vec<_>>();
    let rejected = module
        .functions
        .iter()
        .filter(|function| function.status == ncc_ir::MctLlvmFunctionStatusIR::Rejected)
        .map(|function| (&function.symbol_name, &function.diagnostics))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing JSON bodies: {missing:?}; functions={:?}",
        module
            .functions
            .iter()
            .filter(|function| function.symbol_name.contains("Json"))
            .map(|function| (
                &function.source_name,
                &function.symbol_name,
                function.status
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        rejected.is_empty(),
        "rejected runtime functions: {rejected:?}"
    );
    for symbol in JSON_SYMBOLS {
        assert!(
            module
                .llvm_ir
                .contains(&format!("@nccJsonRuntime_{symbol}(")),
            "LLVM module omitted {symbol}"
        );
    }
    for source_name in [
        "nccJsonRuntime.ensureHandleCapacity",
        "nccJsonRuntime.nodeOf",
        "nccJsonRuntime.handleOf",
        "nccJsonRuntime.takeHandle",
    ] {
        assert!(
            module.functions.iter().any(|function| {
                function.source_name == source_name
                    && function.status == ncc_ir::MctLlvmFunctionStatusIR::Emitted
            }),
            "MCT omitted token-handle function {source_name}"
        );
    }
    assert!(
        module.llvm_ir.contains("@__ncc_stc_nccJsonRuntime_handles"),
        "JSON handle table was not materialized"
    );
    assert!(
        module
            .llvm_ir
            .contains("@__ncc_stc_nccJsonRuntime_nextHandle"),
        "JSON token counter was not materialized"
    );
}
