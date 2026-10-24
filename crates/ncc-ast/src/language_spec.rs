use crate::{
    runtime_api_family_specs, scalar_type_contract, NativePreludeFunctionSpec,
    RuntimeApiFamilySpec, SourceExecutionMode, NATIVE_PRELUDE_V1_FUNCTIONS,
};
use serde::Serialize;
use std::collections::HashSet;

include!(concat!(env!("OUT_DIR"), "/diagnostic_catalog_generated.rs"));

pub const LANGUAGE_SPEC_SCHEMA: &str = "nc.language.canonical.v1";
pub const LANGUAGE_SPEC_VERSION: &str = "2.0.0.2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrammarProductionSpec {
    pub name: &'static str,
    pub ebnf: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeCatalogSpec {
    pub name: String,
    pub class: String,
    pub bit_width: Option<u16>,
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub llvm_type: String,
    pub aliases: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeFamilySpec {
    pub name: &'static str,
    pub syntax: &'static str,
    pub storage: &'static str,
    pub constraints: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticSpec {
    pub code: &'static str,
    pub phase: &'static str,
    pub summary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilitySpec {
    pub name: &'static str,
    pub status: &'static str,
    pub authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionModeSpec {
    pub mode: SourceExecutionMode,
    pub activation: &'static str,
    pub compiler_role: &'static str,
    pub runtime_checks: &'static str,
    pub hidden_work: &'static str,
    pub expansion: &'static str,
    pub kernel_contract: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalLanguageSpec {
    pub schema: &'static str,
    pub version: &'static str,
    pub pointer_width_bytes: usize,
    pub grammar: Vec<GrammarProductionSpec>,
    pub types: Vec<TypeCatalogSpec>,
    pub type_families: Vec<TypeFamilySpec>,
    pub native_prelude: Vec<NativePreludeFunctionSpec>,
    pub imports: Vec<RuntimeApiFamilySpec>,
    pub diagnostics: Vec<DiagnosticSpec>,
    pub capabilities: Vec<CapabilitySpec>,
    pub execution_modes: Vec<ExecutionModeSpec>,
}

pub const CANONICAL_GRAMMAR: &[GrammarProductionSpec] = &[
    GrammarProductionSpec { name: "program", ebnf: "program = { module_decl | package_decl | use_decl | compatibility_global | container } ;" },
    GrammarProductionSpec { name: "package_profile", ebnf: "package_profile = 'package' identifier '{' 'profile:' identifier ';' '}' ;" },
    GrammarProductionSpec { name: "use_decl", ebnf: "use_decl = \"use\" \"<\" [ import_domain \".\" ] import_family \":\" import_item { \",\" import_item } \">\" \";\" | \"use\" module_path \"{\" import_item { \",\" import_item } \"}\" \";\" ;" },
    GrammarProductionSpec { name: "global_const", ebnf: "global_const = identifier \"::\" \"const\" [ type_name ] \"<-\" expression \";\" ;" },
    GrammarProductionSpec { name: "container", ebnf: "container = identifier [ \"::\" intentional_role ] \"{\" [ const_section ] [ type_section ] [ stc_section ] [ local_section ] { fn_section } [ main_entry | compact_main ] [ final_section ] \"}\" ; (* the ordinary header is `name {`; a role is written only when materially required *)" },
    GrammarProductionSpec { name: "intentional_role", ebnf: "intentional_role = \"runtime\"|\"network\"|\"system\"|\"kernel\"|\"tooling\"|\"compiler\"|\"security\"|\"inspect\"|\"ai\"|\"machine\"|\"primitive\"|\"basal\" ;" },
    GrammarProductionSpec { name: "definition", ebnf: "definition = const_section | type_section | stc_section | local_section | fn_section | abi_contract | abi_binding | main_entry | compact_main | final_section ;" },
    GrammarProductionSpec { name: "const_section", ebnf: "const_section = \"const\" \":\" { visibility_opt constant_decl } ;" },
    GrammarProductionSpec { name: "type_section", ebnf: "type_section = \"type\" \":\" { type_contract } ;" },
    GrammarProductionSpec { name: "stc_section", ebnf: "stc_section = \"stc\" \":\" { visibility_opt place_decl } ; (* at most one per container *)" },
    GrammarProductionSpec { name: "local_section", ebnf: "local_section = \"local\" \":\" { visibility_opt local_decl } ;" },
    GrammarProductionSpec { name: "fn_section", ebnf: "fn_section = \"fn\" \":\" { visibility_group | function } ;" },
    GrammarProductionSpec { name: "visibility_group", ebnf: "visibility_group = (\"public\"|\"internal\"|\"private\") \":\" { function | place_decl | type_field } ;" },
    GrammarProductionSpec { name: "type_contract", ebnf: "type_contract = identifier [ type_parameters ] [ \"enum\" | \"union\" | \"tagged\" \"union\" ] \"{\" { visibility_group | type_clause } \"}\" ;" },
    GrammarProductionSpec { name: "layout_clause", ebnf: "layout_clause = \"layout:\" { layout_item \";\" } ;" },
    GrammarProductionSpec { name: "layout_item", ebnf: "layout_item = \"repr\" \"(\" (\"C\"|\"native\") \")\" | \"packed\" | \"align\" \"(\" integer \")\" | \"size\" \"(\" integer \")\" | \"endian\" \"(\" (\"native\"|\"little\"|\"big\") \")\" | \"flexible\" \"(\" identifier \")\" | \"bitfield\" \"(\" identifier \",\" identifier \",\" integer \",\" integer \")\" ;" },
    GrammarProductionSpec { name: "function", ebnf: "function = [ \"async\" ] identifier [ type_parameters ] \"(\" [ parameter { \",\" parameter } ] \")\" \"->\" type_name block ; (* `run` is an ordinary intentional function name *)" },
    GrammarProductionSpec { name: "parameter", ebnf: "parameter = identifier \">>\" type_name ;" },
    GrammarProductionSpec { name: "statement", ebnf: "statement = place_decl | place_write | load_stmt | store_stmt | copy_stmt | fill_stmt | return_stmt | if_stmt | loop_stmt | guard_stmt | match_stmt | defer_stmt | expression \";\" ;" },
    GrammarProductionSpec { name: "place_decl", ebnf: "place_decl = identifier \">>\" type_name [ \"<-\" expression ] \";\" ;" },
    GrammarProductionSpec { name: "place_write", ebnf: "place_write = place \"<-\" expression \";\" ;" },
    GrammarProductionSpec { name: "load_stmt", ebnf: "load_stmt = place \"<-\" \"load\" type_name \"[\" expression \"]\" \";\" ;" },
    GrammarProductionSpec { name: "store_stmt", ebnf: "store_stmt = \"store\" type_name \"[\" expression \"]\" \"<-\" expression \";\" ;" },
    GrammarProductionSpec { name: "copy_stmt", ebnf: "copy_stmt = \"copy\" expression \"bytes\" \"[\" expression \"]\" \"<-\" \"[\" expression \"]\" \";\" ;" },
    GrammarProductionSpec { name: "fill_stmt", ebnf: "fill_stmt = \"fill\" expression \"bytes\" \"[\" expression \"]\" \"<-\" expression \";\" ;" },
    GrammarProductionSpec { name: "abi_contract", ebnf: "abi_contract = identifier \"::\" \"abi\" \"{\" { abi_field \"<-\" abi_value \";\" } \"}\" ;" },
    GrammarProductionSpec { name: "abi_binding", ebnf: "abi_binding = \"bind\" path \"<-\" path \"using\" identifier \";\" ;" },
    GrammarProductionSpec { name: "defer_stmt", ebnf: "defer_stmt = (\"defer\" | \"final\") block ;" },
    GrammarProductionSpec { name: "try_expr", ebnf: "try_expr = postfix_expr \"?\" | \"try\" unary_expr ;" },
    GrammarProductionSpec { name: "conditional_expr", ebnf: "conditional_expr = binary_expr [ \"?\" expression \":\" expression ] ;" },
    GrammarProductionSpec { name: "compile_time_query", ebnf: "compile_time_query = (\"sizeOf\"|\"alignOf\") \"(\" type_name \")\" | \"offsetOf\" \"(\" type_name \",\" identifier \")\" ;" },
    GrammarProductionSpec { name: "static_assert", ebnf: "static_assert = \"staticAssert\" \"(\" expression [ \",\" string ] \")\" \";\" ;" },
    GrammarProductionSpec { name: "main_entry", ebnf: "main_entry = \"ref\" \"main\" \"->>\" path \";\" ;" },
    GrammarProductionSpec { name: "compact_main", ebnf: "compact_main = \"to\" \"main\" block ; (* preferred only for basic bodies of at most 30 logical statements *)" },
    GrammarProductionSpec { name: "final_section", ebnf: "final_section = \"fa\" \":\" { statement } ; (* material final path lowered by backend/machine *)" },
];

pub const CANONICAL_DIAGNOSTICS: &[DiagnosticSpec] = &[
    DiagnosticSpec { code: "E_SPEC_DRIFT_001", phase: "spec", summary: "Generated language artifacts do not match the canonical manifest." },
    DiagnosticSpec { code: "E_HIGH_TRY_CONTEXT_001", phase: "sema", summary: "Error propagation is used outside a compatible Option or Result return contract." },
    DiagnosticSpec { code: "E_HIGH_TRY_OPERAND_001", phase: "sema", summary: "Error propagation requires an Option or Result operand." },
    DiagnosticSpec { code: "E_HIGH_DEFER_CONTROL_FLOW_001", phase: "sema", summary: "A deferred cleanup cannot return, break or continue." },
    DiagnosticSpec { code: "E_LAYOUT_REPR_001", phase: "sema.layout", summary: "The representation name is unsupported or conflicts with another representation." },
    DiagnosticSpec { code: "E_LAYOUT_ALIGN_001", phase: "sema.layout", summary: "Alignment must be a non-zero power of two." },
    DiagnosticSpec { code: "E_LAYOUT_SIZE_001", phase: "sema.layout", summary: "Explicit size must contain the natural layout and be a multiple of aggregate alignment." },
    DiagnosticSpec { code: "E_LAYOUT_ITEM_001", phase: "sema.layout", summary: "A layout clause contains an item outside the canonical catalog." },
    DiagnosticSpec { code: "E_LAYOUT_QUERY_001", phase: "sema.const", summary: "A compile-time size, alignment or offset query cannot be resolved." },
    DiagnosticSpec { code: "E_LAYOUT_PACKED_001", phase: "sema.layout", summary: "Packed layout conflicts with an incompatible field or aggregate contract." },
    DiagnosticSpec { code: "E_LAYOUT_UNION_001", phase: "sema.layout", summary: "A union field or initializer violates union storage rules." },
    DiagnosticSpec { code: "E_LAYOUT_BITFIELD_001", phase: "sema.layout", summary: "A bitfield width or offset exceeds its integer storage unit." },
    DiagnosticSpec { code: "E_LAYOUT_FLEXIBLE_ARRAY_001", phase: "sema.layout", summary: "A flexible array member must be the final member of a non-union aggregate." },
    DiagnosticSpec { code: "E_LAYOUT_ENDIAN_001", phase: "sema.layout", summary: "Endian must be native, little or big." },
    DiagnosticSpec { code: "E_STATIC_ASSERT_001", phase: "sema.const", summary: "A compile-time assertion evaluated to false." },
    DiagnosticSpec { code: "E_STC_DUPLICATE_001", phase: "parser", summary: "A container declares more than one material state section." },
    DiagnosticSpec { code: "E_STC_CANONICAL_001", phase: "parser", summary: "State storage must use the unique stc section." },
    DiagnosticSpec { code: "E_FA_CANONICAL_001", phase: "parser", summary: "Container finalization must use fa." },
    DiagnosticSpec { code: "E_ASSIGN_INIT_REMOVED_001", phase: "parser", summary: "Initialization and writes use <-; the removed := spelling is rejected." },
];

const GENERATED_DIAGNOSTIC_SUMMARY: &str =
    "Diagnostic registered from the exact-head compiler source inventory.";

fn canonical_diagnostics() -> Vec<DiagnosticSpec> {
    GENERATED_DIAGNOSTIC_CODES
        .iter()
        .map(|(code, phase)| {
            CANONICAL_DIAGNOSTICS
                .iter()
                .find(|diagnostic| diagnostic.code == *code)
                .cloned()
                .unwrap_or(DiagnosticSpec {
                    code,
                    phase,
                    summary: GENERATED_DIAGNOSTIC_SUMMARY,
                })
        })
        .collect()
}

pub const CANONICAL_CAPABILITIES: &[CapabilitySpec] = &[
    CapabilitySpec {
        name: "compact_surface",
        status: "implemented",
        authority: "parser+ast+sema",
    },
    CapabilitySpec {
        name: "option_result",
        status: "implemented",
        authority: "ast+sema+ir+runtime",
    },
    CapabilitySpec {
        name: "error_propagation",
        status: "implemented",
        authority: "ast+sema+ir+runtime",
    },
    CapabilitySpec {
        name: "deferred_cleanup",
        status: "implemented",
        authority: "ast+sema+ir+runtime",
    },
    CapabilitySpec {
        name: "explicit_binary_layout",
        status: "implemented",
        authority: "ast+sema+ir+backend",
    },
    CapabilitySpec {
        name: "runtime_imports",
        status: "generated",
        authority: "runtime_api_family_specs",
    },
    CapabilitySpec {
        name: "machine_continuity",
        status: "implemented",
        authority: "typed-machine-ir",
    },
    CapabilitySpec {
        name: "manual_raw_execution",
        status: "implemented",
        authority: "ast+sema+ir+backend",
    },
    CapabilitySpec {
        name: "inspectable_manual_raw_lowering",
        status: "implemented",
        authority: "ast+ir+build-artifacts",
    },
    CapabilitySpec {
        name: "kernel_zero_hidden_work",
        status: "implemented",
        authority: "sema+ir+backend",
    },
    CapabilitySpec {
        name: "canonical_machine_diagnostic_abi_v2",
        status: "implemented",
        authority: "core-nc+stage0-bridge+machine-record",
    },
];

pub const CANONICAL_EXECUTION_MODES: &[ExecutionModeSpec] = &[ExecutionModeSpec {
    mode: SourceExecutionMode::ManualRaw,
    activation: "always; package profile identifiers are open build metadata",
    compiler_role: "mechanical parsing, validation, lowering, encoding and semantics-preserving optimization only",
    runtime_checks: "none for bounds, overflow, null, provenance, lifetime or aliasing",
    hidden_work: "no unrequested checks, release, IO, process, thread, synchronization, recovery, scheduler or dynamic dispatch effects; material operators such as str + str request their native backend operation",
    expansion: "compact spellings and operators lower deterministically; every material effect remains visible in IR, machine artifacts and manifests",
    kernel_contract: "hosted runtime, implicit heap and hidden dispatch are forbidden",
}];

pub const CANONICAL_TYPE_FAMILIES: &[TypeFamilySpec] = &[
    TypeFamilySpec {
        name: "fixed_array",
        syntax: "array<T, N[, D...]>",
        storage: "inline contiguous elements",
        constraints: "every dimension is a compile-time non-negative integer",
    },
    TypeFamilySpec {
        name: "dynamic_array",
        syntax: "array<T>",
        storage: "data pointer, length and capacity descriptor",
        constraints: "element layout must be materializable",
    },
    TypeFamilySpec {
        name: "vector",
        syntax: "vector<T>",
        storage: "data pointer, length and capacity descriptor",
        constraints: "typed stride is sizeof(T)",
    },
    TypeFamilySpec {
        name: "list",
        syntax: "list<T>",
        storage: "runtime dynamic collection descriptor",
        constraints: "operations preserve T",
    },
    TypeFamilySpec {
        name: "queue",
        syntax: "queue<T>",
        storage: "runtime FIFO descriptor",
        constraints: "operations preserve T",
    },
    TypeFamilySpec {
        name: "slice",
        syntax: "slice<T> | T[]",
        storage: "data pointer and length view",
        constraints: "non-owning view",
    },
    TypeFamilySpec {
        name: "span",
        syntax: "span<T>",
        storage: "data pointer and length view",
        constraints: "non-owning contiguous view",
    },
    TypeFamilySpec {
        name: "pointer",
        syntax: "ptr<T> | ptr mut<T> | T*",
        storage: "target pointer-width address",
        constraints: "typed arithmetic uses sizeof(T)",
    },
    TypeFamilySpec {
        name: "region",
        syntax: "memRegion<T>",
        storage: "base pointer and bounded length",
        constraints: "typed stride and region generation are retained",
    },
    TypeFamilySpec {
        name: "option",
        syntax: "Option<T>",
        storage: "tagged algebraic value",
        constraints: "Some carries T; None carries no payload",
    },
    TypeFamilySpec {
        name: "result",
        syntax: "Result<T, E>",
        storage: "tagged algebraic value",
        constraints: "Ok carries T; Err carries E; neither channel may erase to any",
    },
    TypeFamilySpec {
        name: "function",
        syntax: "fn(P...) -> R",
        storage: "code pointer plus environment when captured",
        constraints: "parameters and return follow the selected ABI",
    },
    TypeFamilySpec {
        name: "record",
        syntax: "Name { ... }",
        storage: "declared field layout",
        constraints: "native or explicit binary layout",
    },
    TypeFamilySpec {
        name: "union",
        syntax: "Name union { ... }",
        storage: "overlapping member storage",
        constraints: "size and alignment are the maximum required by any member",
    },
];

const TYPE_ALIASES: &[(&str, &[&str])] = &[
    ("void", &["unit"]),
    ("bool", &[]),
    ("char", &[]),
    ("byte", &["u8", "uint8"]),
    ("sbyte", &["i8", "int8"]),
    ("word", &["u16", "uint16", "ushort"]),
    ("sword", &["i16", "int16", "short"]),
    ("dword", &["u32", "uint32", "uint"]),
    ("sdword", &["i32", "int32", "int"]),
    ("qword", &["u64", "uint64", "ulong"]),
    ("sqword", &["i64", "int64", "long"]),
    ("ft32", &["f32", "float"]),
    ("ft64", &["f64", "double", "ft"]),
    ("addr", &[]),
    ("str", &["string", "text"]),
    ("bytes", &[]),
    ("ptr<int>", &["ptr mut<int>", "rawPtr<int>"]),
];

fn scalar_class_name(class: crate::ScalarTypeClass) -> String {
    format!("{class:?}").to_ascii_lowercase()
}

pub fn canonical_language_spec(pointer_width_bytes: usize) -> CanonicalLanguageSpec {
    let pointer_width_bytes = pointer_width_bytes.max(1);
    let types = TYPE_ALIASES
        .iter()
        .map(|(name, aliases)| {
            let contract = scalar_type_contract(name, pointer_width_bytes)
                .expect("canonical scalar catalog contains a missing scalar contract");
            TypeCatalogSpec {
                name: contract.official_name.to_string(),
                class: scalar_class_name(contract.class),
                bit_width: contract.bit_width,
                size_bytes: contract.size_bytes,
                align_bytes: contract.align_bytes,
                llvm_type: contract.llvm_type.to_string(),
                aliases: aliases.to_vec(),
            }
        })
        .collect();

    CanonicalLanguageSpec {
        schema: LANGUAGE_SPEC_SCHEMA,
        version: LANGUAGE_SPEC_VERSION,
        pointer_width_bytes,
        grammar: CANONICAL_GRAMMAR.to_vec(),
        types,
        type_families: CANONICAL_TYPE_FAMILIES.to_vec(),
        native_prelude: NATIVE_PRELUDE_V1_FUNCTIONS.to_vec(),
        imports: runtime_api_family_specs().to_vec(),
        diagnostics: canonical_diagnostics(),
        capabilities: CANONICAL_CAPABILITIES.to_vec(),
        execution_modes: CANONICAL_EXECUTION_MODES.to_vec(),
    }
}

pub fn validate_canonical_language_spec(spec: &CanonicalLanguageSpec) -> Result<(), String> {
    if spec.schema != LANGUAGE_SPEC_SCHEMA {
        return Err("E_SPEC_DRIFT_001: invalid canonical schema".into());
    }
    let mut names = HashSet::new();
    for production in &spec.grammar {
        if !names.insert(production.name) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate grammar production '{}'",
                production.name
            ));
        }
    }
    names.clear();
    for ty in &spec.types {
        if !names.insert(ty.name.as_str()) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate scalar type '{}'",
                ty.name
            ));
        }
    }
    names.clear();
    for family in &spec.type_families {
        if !names.insert(family.name) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate type family '{}'",
                family.name
            ));
        }
    }
    names.clear();
    let mut symbols = HashSet::new();
    for function in &spec.native_prelude {
        if !names.insert(function.source_name) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate native prelude function '{}'",
                function.source_name
            ));
        }
        if !symbols.insert(function.canonical_symbol) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate native prelude symbol '{}'",
                function.canonical_symbol
            ));
        }
        if function.accepted_arities.is_empty() {
            return Err(format!(
                "E_SPEC_DRIFT_001: native prelude function '{}' has no accepted arity",
                function.source_name
            ));
        }
    }
    names.clear();
    for family in &spec.imports {
        if !names.insert(family.import_path.as_str()) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate import family '{}'",
                family.import_path
            ));
        }
    }
    names.clear();
    for diagnostic in &spec.diagnostics {
        if !names.insert(diagnostic.code) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate diagnostic '{}'",
                diagnostic.code
            ));
        }
    }
    names.clear();
    for mode in &spec.execution_modes {
        if !names.insert(mode.mode.as_str()) {
            return Err(format!(
                "E_SPEC_DRIFT_001: duplicate execution mode '{}'",
                mode.mode.as_str()
            ));
        }
    }
    Ok(())
}

pub fn canonical_grammar_ebnf() -> String {
    CANONICAL_GRAMMAR
        .iter()
        .map(|production| production.ebnf)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub fn canonical_spec_markdown(spec: &CanonicalLanguageSpec) -> String {
    let mut out = format!("# nC canonical language specification\n\nSchema: `{}`\n\nVersion: `{}`\n\n## Grammar\n\n```ebnf\n{}\n```\n\n## Types\n\n| Type | Class | Bits | Size | Align | LLVM |\n|---|---|---:|---:|---:|---|\n", spec.schema, spec.version, canonical_grammar_ebnf());
    for ty in &spec.types {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | `{}` |\n",
            ty.name,
            ty.class,
            ty.bit_width.map_or_else(|| "-".into(), |v| v.to_string()),
            ty.size_bytes,
            ty.align_bytes,
            ty.llvm_type
        ));
    }
    out.push_str(
        "\n## Type families\n\n| Family | Syntax | Storage | Constraints |\n|---|---|---|---|\n",
    );
    for family in &spec.type_families {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            family.name, family.syntax, family.storage, family.constraints
        ));
    }
    out.push_str(
        "\n## Native prelude\n\nThese names resolve without a source import. The compiler records the canonical symbol and selects the intrinsic or runtime backend explicitly.\n\n| Name | Canonical symbol | Arity | Lowering |\n|---|---|---|---|\n",
    );
    for function in &spec.native_prelude {
        let arities = function
            .accepted_arities
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(" or ");
        out.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` |\n",
            function.source_name,
            function.canonical_symbol,
            arities,
            function.lowering.as_str()
        ));
    }
    out.push_str(&format!(
        "\n## Imports\n\n{} families, generated from the runtime catalog.\n\n## Diagnostics\n\n",
        spec.imports.len()
    ));
    for diagnostic in &spec.diagnostics {
        out.push_str(&format!(
            "- `{}` ({}): {}\n",
            diagnostic.code, diagnostic.phase, diagnostic.summary
        ));
    }
    out.push_str("\n## Execution modes\n\n| Mode | Activation | Compiler role | Runtime checks | Hidden work | Expansion | Kernel |\n|---|---|---|---|---|---|---|\n");
    for mode in &spec.execution_modes {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} |\n",
            mode.mode.as_str(),
            mode.activation,
            mode.compiler_role,
            mode.runtime_checks,
            mode.hidden_work,
            mode.expansion,
            mode.kernel_contract
        ));
    }
    out.push_str("\n## Capabilities\n\n");
    for capability in &spec.capabilities {
        out.push_str(&format!(
            "- `{}`: {} (`{}`)\n",
            capability.name, capability.status, capability.authority
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_manifest_is_unique_and_uses_runtime_authority() {
        let spec = canonical_language_spec(8);
        validate_canonical_language_spec(&spec).expect("canonical manifest must validate");
        assert_eq!(spec.imports.len(), runtime_api_family_specs().len());
        assert!(spec.grammar.iter().any(|item| item.name == "try_expr"));
        assert!(spec.types.iter().any(|item| item.name == "sdword"));
        assert!(spec.type_families.iter().any(|item| item.name == "result"));
        for name in [
            "alloc",
            "calloc",
            "realloc",
            "free",
            "ptrRead",
            "ptrWrite",
            "ptrOffset",
            "sizeOf",
            "alignOf",
            "offsetOf",
            "atomicLoad",
            "atomicStore",
            "fence",
            "sleepMs",
            "monotonicMs",
        ] {
            assert!(
                spec.native_prelude
                    .iter()
                    .any(|function| function.source_name == name),
                "missing native prelude function {name}"
            );
        }
        assert_eq!(spec.execution_modes.len(), 1);
        assert_eq!(spec.execution_modes[0].mode, SourceExecutionMode::ManualRaw);
        assert!(
            spec.diagnostics.len() >= 1_500,
            "the exact-head compiler diagnostic inventory must be complete"
        );
        for expected in [
            "E_HIGH_TRY_CONTEXT_001",
            "E_MACHINE_ENCODING_001",
            "E_PRIMITIVE_RUNTIME_001",
            "E_RUNTIME_JSON_ABI_001",
            "E_BASAL_EVIDENCE_001",
        ] {
            assert!(
                spec.diagnostics.iter().any(|item| item.code == expected),
                "missing canonical diagnostic {expected}"
            );
        }
    }

    #[test]
    fn canonical_grammar_and_markdown_are_deterministic() {
        let spec = canonical_language_spec(8);
        assert_eq!(canonical_grammar_ebnf(), canonical_grammar_ebnf());
        let markdown = canonical_spec_markdown(&spec);
        assert!(markdown.contains(LANGUAGE_SPEC_SCHEMA));
        assert!(markdown.contains("error_propagation"));
        assert!(markdown.contains("manual_raw"));
        assert!(markdown.matches("manual_raw").count() >= 1);
        assert!(markdown.contains("## Native prelude"));
        assert!(markdown.contains("nC.intrinsic.memory.alloc"));
    }
}
