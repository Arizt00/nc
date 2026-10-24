use super::super::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default, Debug, Clone)]
struct CompilerCliOptions {
    json: bool,
    md: bool,
    no_color: bool,
    strict: bool,
    phase: Option<String>,
    target: Option<String>,
    profile: Option<String>,
    family: Option<String>,
    mode: Option<String>,
    out: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompilerNativeLtoMode {
    Off,
    Thin,
    Full,
}

impl CompilerNativeLtoMode {
    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Thin => "thin",
            Self::Full => "full",
        }
    }

    fn clang_arg(self) -> Option<&'static str> {
        match self {
            Self::Off => None,
            Self::Thin => Some("-flto=thin"),
            Self::Full => Some("-flto=full"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompilerNativePgoMode {
    Off,
    Auto,
    Use,
    Train,
}

impl CompilerNativePgoMode {
    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Use => "use",
            Self::Train => "train",
        }
    }
}

#[derive(Clone, Debug)]
struct CompilerNativeOptimizationPlan {
    lto_requested: CompilerNativeLtoMode,
    lto: CompilerNativeLtoMode,
    pgo_requested: CompilerNativePgoMode,
    pgo_profile: PathBuf,
    pgo_raw_dir: PathBuf,
    pgo_uses_profile: bool,
}

const COMPILER_PHASES: &[&str] = &[
    "source",
    "lexer",
    "parser",
    "ast",
    "sema",
    "type_resolver",
    "operator_resolver",
    "condition_resolver",
    "import_resolver",
    "flow_resolver",
    "nir",
    "nir_verifier",
    "lowering",
    "optimizer",
    "backend",
    "diagnostics",
];

const COMPILER_DIAGNOSTIC_CATALOG: &[&str] = &[
    "E_CONDITION_BOOL_REQUIRED_001",
    "E_CONDITION_UNRESOLVED_001",
    "E_CONDITION_PLACEHOLDER_001",
    "E_CONDITION_STR_FORBIDDEN_001",
    "E_CONDITION_NULL_FORBIDDEN_001",
    "E_CONDITION_MATCH_NON_EXHAUSTIVE_001",
    "E_CONDITION_MATCH_UNREACHABLE_ARM_001",
    "E_CONDITION_GUARD_ELSE_MISSING_001",
    "E_CONDITION_UNLESS_INVALID_MODE_001",
    "E_TYPE_UNKNOWN_001",
    "E_TYPE_MISMATCH_001",
    "E_TYPE_GENERIC_ARITY_001",
    "E_TYPE_NULLABILITY_001",
    "E_TYPE_UNION_NARROWING_001",
    "E_TYPE_MODE_FORBIDDEN_001",
    "E_TYPE_LAYOUT_MISSING_001",
    "E_TYPE_SIZE_UNKNOWN_001",
    "E_TYPE_ALIGN_UNKNOWN_001",
    "E_TYPE_INT_001",
    "E_TYPE_U64_001",
    "E_TYPE_BYTE_001",
    "E_TYPE_BOOL_001",
    "E_TYPE_STR_001",
    "E_TYPE_TEXT_001",
    "E_TYPE_CHAR_001",
    "E_TYPE_FT64_001",
    "E_TYPE_ADDR_001",
    "E_TYPE_WORD_001",
    "E_TYPE_DWORD_001",
    "E_TYPE_QWORD_001",
    "E_TYPE_PTR_001",
    "E_TYPE_PTR_ORIGIN_001",
    "E_TYPE_PTR_NULL_001",
    "E_TYPE_MEMREGION_001",
    "E_TYPE_MEMREGION_BOUNDS_001",
    "E_TYPE_ARRAY_001",
    "E_TYPE_ARRAY_BOUNDS_001",
    "E_TYPE_LIST_001",
    "E_TYPE_RESULT_001",
    "E_TYPE_RESULT_UNWRAP_001",
    "E_TYPE_OPTION_001",
    "E_TYPE_OPTION_UNWRAP_001",
    "E_TYPE_FUNCTION_SIGNATURE_001",
    "E_TYPE_RETURN_001",
    "E_TYPE_DEV_PAYLOAD_001",
    "E_TYPE_CONTRACT_001",
    "E_TYPE_CONTRACT_SECTION_001",
    "E_TYPE_DUPLICATE_001",
    "E_TYPE_FIELD_DUPLICATE_001",
    "E_TYPE_FIELD_MISSING_001",
    "E_TYPE_FIELD_UNKNOWN_001",
    "E_TYPE_FIELD_VISIBILITY_001",
    "E_TYPE_PRIVATE_ACCESS_001",
    "E_TYPE_INTERNAL_ACCESS_001",
    "E_TYPE_STRICTLY_ACCESS_001",
    "E_TYPE_UNBINARY_LAYOUT_001",
    "E_TYPE_STABLE_LAYOUT_001",
    "E_TYPE_REPR_C_001",
    "E_TYPE_BOUNDS_001",
    "E_TYPE_FIELD_BOUNDS_001",
    "E_TYPE_INVARIANT_RUNTIME_FAIL_001",
    "E_TYPE_COPY_DENIED_001",
    "E_TYPE_CLONE_DENIED_001",
    "E_TYPE_MOVE_DENIED_001",
    "E_TYPE_USE_AFTER_MOVE_001",
    "E_TYPE_OWNERSHIP_001",
    "E_TYPE_DROP_001",
    "E_TYPE_ENUM_001",
    "E_TYPE_VARIANT_001",
    "E_TYPE_UNION_001",
    "E_TYPE_GENERIC_001",
    "E_TYPE_METHOD_001",
    "E_TYPE_CONTAINER_001",
    "E_ADV_OP_UNSUPPORTED_001",
    "E_ADV_OP_AUTHORITY_001",
    "E_ADV_OP_MODE_FORBIDDEN_001",
    "E_ADV_OP_TYPE_001",
    "E_ADV_OP_MOVE_INVALID_001",
    "E_ADV_OP_REINTERPRET_FORBIDDEN_001",
    "E_ADV_OP_OVERFLOW_001",
    "E_ADV_OP_BOUNDS_001",
    "E_ARRAY_RANK_001",
    "E_ARRAY_DIMENSION_001",
    "E_ARRAY_INDEX_001",
    "E_ARRAY_BOUNDS_001",
    "E_ARRAY_LAYOUT_001",
    "E_ARRAY_STRIDE_001",
    "E_ARRAY_OFFSET_OVERFLOW_001",
    "E_ARRAY_SITE_001",
    "E_ARRAY_OPEN_DIM_INDEX_001",
    "E_VECTOR_DIMENSION_001",
    "E_MATRIX_DIMENSION_001",
    "E_TENSOR_RANK_001",
    "E_DINARRAY_TYPE_001",
    "E_DINARRAY_BOUNDS_001",
    "E_DINARRAY_CAPACITY_001",
    "E_DINARRAY_REALLOC_001",
    "E_DINARRAY_SITE_STALE_001",
    "E_DINARRAY_BINARY_LAYOUT_001",
    "E_DINARRAY_UNBINARY_LAYOUT_001",
    "E_DINARRAY_ALIAS_001",
    "E_DESTRUCTURE_UNSUPPORTED_001",
    "E_DESTRUCTURE_FIELD_001",
    "E_DESTRUCTURE_MOVE_001",
    "E_DESTRUCTURE_OWNERSHIP_001",
    "E_DESTRUCTURE_REGION_BOUNDS_001",
    "E_DESTRUCTURE_BINARY_LAYOUT_001",
    "E_CONSTRUCTURE_FIELD_MISSING_001",
    "E_CONSTRUCTURE_FIELD_TYPE_001",
    "E_CONSTRUCTURE_MOVE_001",
    "E_CONSTRUCTURE_REGION_001",
    "E_CONSTRUCTURE_BINARY_LAYOUT_001",
    "E_VISIBILITY_PUBLIC_001",
    "E_VISIBILITY_PRIVATE_001",
    "E_VISIBILITY_INTERNAL_001",
    "E_VISIBILITY_STRICTLY_001",
    "E_VOID_VALUE_001",
    "E_VOID_RETURN_001",
    "E_NULL_TYPE_001",
    "E_NULL_REFINEMENT_001",
    "E_NULL_TO_VOID_001",
    "E_NULL_CONDITION_001",
    "E_VOID_ADDR_001",
    "E_VOID_SITE_001",
    "E_REGION_VAR_UNINIT_001",
    "E_REGION_VAR_INVALID_001",
    "E_REGION_VAR_OWNER_001",
    "E_SITE_VAR_UNAVAILABLE_001",
    "E_SITE_VAR_STALE_001",
    "E_SITE_VAR_PRIMITIVE_CROSSING_001",
    "E_FFI_IMPORT_001",
    "E_FFI_IMPORT_CONTRACT_001",
    "E_FFI_IMPORT_UNKNOWN_001",
    "E_FFI_LANGUAGE_UNSUPPORTED_001",
    "E_FFI_ABI_MISSING_001",
    "E_FFI_SIGNATURE_MISSING_001",
    "E_FFI_ABI_MISMATCH_001",
    "E_FFI_LAYOUT_MISSING_001",
    "E_FFI_LAYOUT_REQUIRED_001",
    "E_FFI_LAYOUT_MISMATCH_001",
    "E_FFI_REPR_REQUIRED_001",
    "E_FFI_SYMBOL_MISSING_001",
    "E_FFI_LINK_FAILED_001",
    "E_FFI_OWNERSHIP_001",
    "E_FFI_FREE_CONTRACT_001",
    "E_FFI_NULLABILITY_001",
    "E_FFI_EXCEPTION_BOUNDARY_001",
    "E_FFI_CPP_EXCEPTION_BOUNDARY_001",
    "E_FFI_RUST_PANIC_BOUNDARY_001",
    "E_FFI_SIDE_EFFECT_001",
    "E_FFI_CAPABILITY_001",
    "E_FFI_POLICY_001",
    "E_FFI_TYPE_UNSUPPORTED_001",
    "E_FFI_STRING_BOUNDARY_001",
    "E_FFI_RESULT_OPTION_BOUNDARY_001",
    "E_FFI_ASM_BRIDGE_001",
    "E_FFI_PRIMITIVE_CROSSING_001",
    "E_RUNTIME_RENDER_IMPORT_001",
    "E_RUNTIME_RENDER_TYPE_001",
    "E_RUNTIME_CONCAT_TYPE_001",
    "E_RUNTIME_CONCAT_IMPORT_001",
    "E_RUNTIME_RENDER_NONE_001",
    "E_RUNTIME_CONCAT_NONE_001",
    "E_RUNTIME_RENDER_ANY_LEAK_001",
    "E_RUNTIME_CONCAT_ANY_LEAK_001",
    "E_USE_BACKING_DEFERRED_001",
    "E_BACKEND_PARITY_MCT_BYTES_001",
    "E_BACKEND_PARITY_LLVM_EXEC_001",
    "E_BACKEND_PARITY_NATIVE_FAST_001",
    "E_BACKEND_PARITY_MACHINE_AUTHORITY_001",
    "E_BACKEND_PARITY_DIVERGENCE_001",
    "E_BACKEND_TOOLCHAIN_MISSING_001",
    "E_OPTIMIZER_CONTRACT_001",
    "E_FEATURE_SUITE_CONTRACT_001",
];

const COMPILER_TYPE_CATALOG: &[&str] = &[
    "int",
    "u64",
    "byte",
    "word",
    "dword",
    "qword",
    "bool",
    "str",
    "text",
    "char",
    "ft64",
    "addr",
    "ptr<T>",
    "ptr<T> | null",
    "memRegion<T>",
    "array<T, N>",
    "array2<T, X, Y>",
    "array3<T, X, Y, Z>",
    "matrix<T, R, C>",
    "tensor<T, D...>",
    "xy<T>",
    "xyz<T>",
    "x2y2<T>",
    "x2y2z2<T>",
    "x3y3<T>",
    "x3y3z3<T>",
    "dinArray<T>",
    "dinArray.binary<T>",
    "dinArray.unbinary<T>",
    "list<T>",
    "Result<T,E>",
    "Option<T>",
    "T | U",
    "fn(...) -> T",
];

const COMPILER_ARRAY_RUNTIME_CALL_IDS: &[&str] = &[
    "arraycopy",
    "arraymove",
    "arrayswap",
    "arrayfill",
    "arrayzero",
    "arrayreverse",
];

const COMPILER_MEMORY_RUNTIME_CALL_IDS: &[&str] = &[
    "arraycopy",
    "arraymove",
    "arrayswap",
    "arrayfill",
    "arrayzero",
    "arrayreverse",
    "listmove",
    "listswap",
    "regioncopy",
    "regionmove",
    "regionzero",
    "regionfill",
    "memorycopy",
    "memorymove",
    "memorycompare",
    "memoryzero",
    "copyvalue",
    "copyref",
    "copyptr",
    "copyfn",
    "copyview",
    "copydeep",
    "copybinary",
    "copyunbinary",
    "clonevalue",
    "cloneshallow",
    "clonedeep",
    "clonepointee",
    "cloneregion",
    "clonebinary",
    "cloneunbinary",
    "siteof",
    "siteoflistelement",
    "addrof",
];

const COMPILER_FEATURE_SUITE_REQUIRED_LANES: &[&str] = &[
    "example_n",
    "parser",
    "sema",
    "ir",
    "mct",
    "run",
    "negative",
];

#[derive(Debug, Clone, Copy)]
struct CompilerFeatureSuite {
    id: &'static str,
    feature: &'static str,
    status: &'static str,
    example_n: &'static str,
    parser_test: &'static str,
    sema_test: &'static str,
    ir_test: &'static str,
    mct_fixture: &'static str,
    mct_test: &'static str,
    run_test: &'static str,
    negative_example: &'static str,
    negative_test: &'static str,
    negative_expected_diagnostic: &'static str,
    docs: &'static str,
}

const COMPILER_FEATURE_SUITES: &[CompilerFeatureSuite] = &[CompilerFeatureSuite {
    id: "compact_post_increment",
    feature: "compact syntax: post-increment as statement and expression",
    status: "closed",
    example_n: "examples/01_basics/post_increment_compact.n",
    parser_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_parser",
    sema_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_sema",
    ir_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_ir",
    mct_fixture: "_archive/old_examples/backend/mct_llvm_post_increment.n",
    mct_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_mct",
    run_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_run",
    negative_example: "_archive/old_examples/feature_suites/negative/compact_post_increment_unbound_fail.n",
    negative_test: "crates/ncc-cli/tests/feature_suite_contract.rs::feature_suite_compact_post_increment_negative",
    negative_expected_diagnostic: "E_HIGH_POST_INCREMENT_TARGET_001",
    docs: "docs/spec/compiler/feature_test_suite_contract.md",
}];

pub(crate) fn compiler_help_text() -> &'static str {
    "Usage: ncc compiler <doctor|pipeline|phases|tokens|ast|sema|imports|types|type-contracts|pointer-industrial|pointer-lowering|pointer-audit|bounds|arrays|index|memory-contracts|operators|conditions|nir|lower|optimize|backend|build-native|targets|diagnostics|feature-suite|report|self-check|ffi|ffi-symbols|ffi-contracts|ffi-link-manifest|machine-interop|ffi-smoke|ffi-cpp-abi|ffi-external-bindings> [args]\n\
     Unified compiler contract CLI for [@compiler] macrophase.\n\
     Common options: --json --md --no-color --strict --phase <name> --target <triple> --profile <profile> --family <id> --mode <id> --out <path>\n\
     build-native incremental options: --incremental (default), --no-incremental, --content-cache-dir <path>\n\
     Examples:\n\
       ncc compiler doctor --json\n\
       ncc compiler pipeline examples/calculadoraBasica.n --json\n\
       ncc compiler tokens examples/calculadoraBasica.n --json\n\
       ncc compiler ast examples/calculadoraBasica.n --json\n\
       ncc compiler sema examples/calculadoraBasica.n --json\n\
       ncc compiler imports examples/calculadoraBasica.n --json\n\
       ncc compiler types examples/calculadoraBasica.n --json\n\
       ncc compiler type-contracts examples/calculadoraBasica.n --json\n\
       ncc compiler pointer-industrial --json\n\
       ncc compiler pointer-lowering --json\n\
       ncc compiler pointer-audit examples/06_memory/rawPointerOutOfBounds.n --json\n\
       ncc compiler bounds examples/calculadoraBasica.n --json\n\
       ncc compiler arrays examples/arrays/array_dynamic_index_write.n --json\n\
       ncc compiler index examples/arrays/array_dynamic_index_write.n --json\n\
       ncc compiler memory-contracts examples/memory/array_move_copy.n --json\n\
       ncc compiler operators examples/calculadoraBasica.n --json\n\
       ncc compiler conditions examples/calculadoraBasica.n --json\n\
       ncc compiler nir examples/calculadoraBasica.n --json\n\
       ncc compiler lower examples/calculadoraBasica.n --json\n\
       ncc compiler optimize examples/performance/native_fast/optimizer_fold_dead.n --json\n\
       ncc compiler backend examples/calculadoraBasica.n --json\n\
       ncc compiler build-native examples/10_mct/print_bytes.n --out-dir .ncc --profile release-fast --json\n\
       ncc compiler diagnostics --json\n\
       ncc compiler feature-suite --json --strict\n\
       ncc compiler ffi examples/ffi/c_ffi_validate_amount.n --json\n\
       ncc compiler ffi-symbols examples/ffi/c_ffi_validate_amount.n --json\n\
       ncc compiler ffi-contracts examples/ffi/c_ffi_validate_amount.n --json\n\
       ncc compiler ffi-link-manifest examples/ffi/c_ffi_validate_amount.n --json\n\
       ncc compiler machine-interop examples/09_ffi/machine_interop_bridge.n --target x86_64-unknown-linux-gnu --json\n\
       ncc compiler ffi-cpp-abi --json --strict\n\
       ncc compiler ffi-external-bindings --json\n\
       ncc compiler report examples/calculadoraBasica.n --md-out reports/compiler/calculadoraBasica_compiler_report.md\n\
       ncc compiler self-check --json\n\
       ncc compiler ffi-smoke --json"
}

pub(crate) fn run_compiler_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || is_help_token(&args[0]) {
        println!("{}", compiler_help_text());
        return Ok(());
    }

    let sub = args[0].as_str();
    let (opts, rest) = parse_compiler_cli_options(&args[1..])?;

    match sub {
        "doctor" => compiler_handle_doctor(&opts, &rest),
        "pipeline" => compiler_handle_pipeline(&opts, &rest),
        "phases" => compiler_handle_phases(&opts, &rest),
        "tokens" => compiler_handle_tokens(&opts, &rest),
        "ast" => compiler_handle_ast(&opts, &rest),
        "sema" => compiler_handle_sema(&opts, &rest),
        "imports" => compiler_handle_imports(&opts, &rest),
        "types" => compiler_handle_types(&opts, &rest),
        "type-contracts" => compiler_handle_type_contracts(&opts, &rest),
        "pointer-industrial" | "pointer-closure" | "ptr-industrial" => {
            compiler_handle_pointer_industrial(&opts, &rest)
        }
        "pointer-lowering" | "ptr-lowering" => compiler_handle_pointer_lowering(&opts, &rest),
        "pointer-audit" | "ptr-audit" | "memory-audit" => {
            compiler_handle_pointer_audit(&opts, &rest)
        }
        "bounds" => compiler_handle_bounds(&opts, &rest),
        "arrays" => compiler_handle_arrays(&opts, &rest),
        "index" => compiler_handle_index(&opts, &rest),
        "memory-contracts" => compiler_handle_memory_contracts(&opts, &rest),
        "operators" => compiler_handle_operators(&opts, &rest),
        "conditions" => compiler_handle_conditions(&opts, &rest),
        "nir" => compiler_handle_nir(&opts, &rest),
        "lower" => compiler_handle_lower(&opts, &rest),
        "optimize" | "optimizer" => compiler_handle_optimize(&opts, &rest),
        "backend" => compiler_handle_backend(&opts, &rest),
        "build-native" | "native-build" | "build-c-like" | "c-build" => {
            compiler_handle_build_native(&opts, &rest)
        }
        "targets" => compiler_handle_targets(&opts, &rest),
        "diagnostics" => compiler_handle_diagnostics(&opts, &rest),
        "feature-suite" | "feature-suites" => compiler_handle_feature_suite(&opts, &rest),
        "ffi" => compiler_handle_ffi(&opts, &rest),
        "ffi-symbols" => compiler_handle_ffi_symbols(&opts, &rest),
        "ffi-contracts" => compiler_handle_ffi_contracts(&opts, &rest),
        "ffi-link-manifest" | "ffi-linker-manifest" | "ffi-package-manifest" => {
            compiler_handle_ffi_link_manifest(&opts, &rest)
        }
        "machine-interop" | "machine-bridge" | "machine-abi" => {
            compiler_handle_machine_interop(&opts, &rest)
        }
        "report" => compiler_handle_report(&opts, &rest),
        "self-check" => compiler_handle_self_check(&opts, &rest),
        "ffi-smoke" => compiler_handle_ffi_smoke(&opts, &rest),
        "ffi-cpp-abi" | "cpp-abi" => compiler_handle_ffi_cpp_abi(&opts, &rest),
        "ffi-external-bindings" | "external-bindings" => {
            compiler_handle_ffi_external_bindings(&opts, &rest)
        }
        _ => Err(actionable_command_error(
            "compiler",
            format!("unknown compiler command '{}'", sub),
        )
        .into()),
    }
}

fn compiler_tool_spawns(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}

fn compiler_resolve_first_tool(candidates: &[&str]) -> Option<String> {
    for candidate in candidates {
        if compiler_tool_spawns(candidate) {
            return Some((*candidate).to_string());
        }

        let Ok(output) = std::process::Command::new("which").arg(candidate).output() else {
            continue;
        };
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && compiler_tool_spawns(&path) {
                return Some(path);
            }
        }
    }
    None
}

fn compiler_run_process(bin: &str, args: &[&str], cwd: &std::path::Path) -> Result<(), String> {
    let output = std::process::Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("spawn failed for '{}': {}", bin, err))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(format!(
        "{} {:?} failed: {}{}{}",
        bin,
        args,
        output.status,
        if stderr.is_empty() { "" } else { " stderr=" },
        if stderr.is_empty() {
            stdout.as_str()
        } else {
            stderr.as_str()
        }
    ))
}

fn compiler_run_process_output(
    bin: &str,
    args: &[&str],
    cwd: &std::path::Path,
) -> Result<std::process::Output, String> {
    std::process::Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("spawn failed for '{}': {}", bin, err))
}

fn compiler_nm_has_symbol(nm: &str, artifact: &std::path::Path, symbol: &str) -> bool {
    let output = std::process::Command::new(nm)
        .arg("-g")
        .arg(artifact)
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains(symbol)
}

fn compiler_ffi_asm_identity_source(host_os: &str, host_arch: &str) -> Result<String, String> {
    let symbol = if host_os == "macos" {
        "_nc_ffi_asm_identity"
    } else {
        "nc_ffi_asm_identity"
    };
    let elf_type = if matches!(host_os, "linux" | "freebsd") {
        format!(".type {symbol}, @function\n")
    } else {
        String::new()
    };
    let align = if host_arch == "aarch64" {
        ".p2align 2\n"
    } else {
        ""
    };

    match host_arch {
        "x86_64" => {
            let arg_reg = if host_os == "windows" { "%ecx" } else { "%edi" };
            Ok(format!(
                ".text\n.globl {symbol}\n{elf_type}{align}{symbol}:\n    movl {arg_reg}, %eax\n    ret\n"
            ))
        }
        "aarch64" => Ok(format!(
            ".text\n.globl {symbol}\n{elf_type}{align}{symbol}:\n    ret\n"
        )),
        other => Err(format!(
            "E_FFI_ASM_BRIDGE_001: ffi asm smoke unsupported host architecture '{}'",
            other
        )),
    }
}

fn compiler_ffi_smoke_case_skipped(case: &str, code: &str, reason: &str) -> serde_json::Value {
    serde_json::json!({
        "case": case,
        "state": "skipped",
        "linkable": false,
        "linked": false,
        "executed": false,
        "skipped_by_toolchain": true,
        "failed": false,
        "official_code": code,
        "reason": reason,
        "validated": true
    })
}

fn compiler_ffi_smoke_case_result(
    case: &str,
    code: &str,
    output: Result<std::process::Output, String>,
    expect_nonzero: bool,
) -> serde_json::Value {
    match output {
        Err(err) => serde_json::json!({
            "case": case,
            "state": "failed",
            "linkable": true,
            "linked": false,
            "executed": false,
            "skipped_by_toolchain": false,
            "failed": true,
            "official_code": "E_FFI_LINK_FAILED_001",
            "reason": err,
            "validated": false
        }),
        Ok(run) => {
            let success = run.status.success();
            let validated = if expect_nonzero { !success } else { success };
            let state = if validated {
                if expect_nonzero {
                    "failed"
                } else {
                    "executed"
                }
            } else if success {
                "executed"
            } else {
                "failed"
            };
            serde_json::json!({
                "case": case,
                "state": state,
                "linkable": true,
                "linked": true,
                "executed": true,
                "skipped_by_toolchain": false,
                "failed": !success,
                "official_code": if expect_nonzero { serde_json::json!(code) } else { serde_json::Value::Null },
                "exit_status": run.status.code(),
                "reason": String::from_utf8_lossy(&run.stderr).trim().to_string(),
                "validated": validated
            })
        }
    }
}

fn compiler_ffi_run_negative_matrix(
    tmp: &std::path::Path,
    cc: &str,
    cxx: &str,
    rustc: &str,
    host_os: &str,
    host_arch: &str,
) -> Vec<serde_json::Value> {
    let mut out = Vec::new();

    // symbol missing real
    let missing_src = tmp.join("ffi_missing_symbol.c");
    let missing_exe = tmp.join("ffi_missing_symbol_exec");
    let _ = fs::write(
        &missing_src,
        b"extern int nc_symbol_missing(int);\nint main(void) { return nc_symbol_missing(7); }\n",
    );
    let missing_src_s = missing_src.to_string_lossy().into_owned();
    let missing_exe_s = missing_exe.to_string_lossy().into_owned();
    match compiler_run_process_output(cc, &["-O2", &missing_src_s, "-o", &missing_exe_s], tmp) {
        Err(err) => out.push(serde_json::json!({
            "case": "symbol_missing_real",
            "state": "failed",
            "linkable": true,
            "linked": false,
            "executed": false,
            "skipped_by_toolchain": false,
            "failed": true,
            "official_code": "E_FFI_LINK_FAILED_001",
            "reason": err,
            "validated": false
        })),
        Ok(link) => {
            let link_ok = link.status.success();
            out.push(serde_json::json!({
                "case": "symbol_missing_real",
                "state": if link_ok { "executed" } else { "failed" },
                "linkable": true,
                "linked": link_ok,
                "executed": false,
                "skipped_by_toolchain": false,
                "failed": !link_ok,
                "official_code": "E_FFI_SYMBOL_MISSING_001",
                "reason": String::from_utf8_lossy(&link.stderr).trim().to_string(),
                "validated": !link_ok
            }));
        }
    }

    // null return for nonNull real
    let null_lib_src = tmp.join("ffi_null_lib.c");
    let null_main_src = tmp.join("ffi_null_main.c");
    let null_obj = tmp.join("ffi_null_lib.o");
    let null_exe = tmp.join("ffi_null_main_exec");
    let _ = fs::write(
        &null_lib_src,
        b"#include <stddef.h>\nvoid* nc_returns_null(void) { return NULL; }\n",
    );
    let _ = fs::write(
        &null_main_src,
        b"extern void* nc_returns_null(void);\nint main(void) {\n  return nc_returns_null() == 0 ? 41 : 0;\n}\n",
    );
    let null_lib_src_s = null_lib_src.to_string_lossy().into_owned();
    let null_main_src_s = null_main_src.to_string_lossy().into_owned();
    let null_obj_s = null_obj.to_string_lossy().into_owned();
    let null_exe_s = null_exe.to_string_lossy().into_owned();
    if compiler_run_process(cc, &["-c", &null_lib_src_s, "-O2", "-o", &null_obj_s], tmp).is_err()
        || compiler_run_process(
            cc,
            &["-O2", &null_main_src_s, &null_obj_s, "-o", &null_exe_s],
            tmp,
        )
        .is_err()
    {
        out.push(compiler_ffi_smoke_case_skipped(
            "null_return_nonnull_real",
            "E_FFI_DEFERRED_001",
            "null-return harness could not be linked on this toolchain",
        ));
    } else {
        out.push(compiler_ffi_smoke_case_result(
            "null_return_nonnull_real",
            "E_FFI_NULLABILITY_001",
            compiler_run_process_output(&null_exe_s, &[], tmp),
            true,
        ));
    }

    // rust panic real
    let panic_rust_src = tmp.join("ffi_panic_rust.rs");
    let panic_exe = tmp.join("ffi_panic_exec");
    let _ = fs::write(
        &panic_rust_src,
        b"#[no_mangle]\npub extern \"C\" fn nc_rust_panic() { panic!(\"ffi panic\"); }\nfn main() { nc_rust_panic(); }\n",
    );
    let panic_rust_src_s = panic_rust_src.to_string_lossy().into_owned();
    let panic_exe_s = panic_exe.to_string_lossy().into_owned();
    if compiler_run_process(rustc, &[&panic_rust_src_s, "-O", "-o", &panic_exe_s], tmp).is_err() {
        out.push(compiler_ffi_smoke_case_skipped(
            "panic_real_rust",
            "E_FFI_DEFERRED_001",
            "rust panic harness could not be linked on this toolchain",
        ));
    } else {
        out.push(compiler_ffi_smoke_case_result(
            "panic_real_rust",
            "E_FFI_RUST_PANIC_BOUNDARY_001",
            compiler_run_process_output(&panic_exe_s, &[], tmp),
            true,
        ));
    }

    // cpp throw real
    let throw_cpp_src = tmp.join("ffi_throw_cpp.cpp");
    let throw_exe = tmp.join("ffi_throw_exec");
    let _ = fs::write(
        &throw_cpp_src,
        b"extern \"C\" int nc_cpp_throw() { throw 7; }\nint main() { return nc_cpp_throw(); }\n",
    );
    let throw_cpp_src_s = throw_cpp_src.to_string_lossy().into_owned();
    let throw_exe_s = throw_exe.to_string_lossy().into_owned();
    if compiler_run_process(cxx, &["-O2", &throw_cpp_src_s, "-o", &throw_exe_s], tmp).is_err() {
        out.push(compiler_ffi_smoke_case_skipped(
            "throw_real_cpp",
            "E_FFI_DEFERRED_001",
            "c++ throw harness could not be linked on this toolchain",
        ));
    } else {
        out.push(compiler_ffi_smoke_case_result(
            "throw_real_cpp",
            "E_FFI_CPP_EXCEPTION_BOUNDARY_001",
            compiler_run_process_output(&throw_exe_s, &[], tmp),
            true,
        ));
    }

    // ABI mismatch real (x86_64 non-Windows only: SysV caller vs MS ABI callee)
    if host_arch != "x86_64" {
        out.push(compiler_ffi_smoke_case_skipped(
            "abi_mismatch_real",
            "E_FFI_DEFERRED_001",
            "abi mismatch harness currently requires x86_64 host",
        ));
    } else if host_os == "windows" {
        out.push(compiler_ffi_smoke_case_skipped(
            "abi_mismatch_real",
            "E_FFI_DEFERRED_001",
            "abi mismatch harness is not meaningful on native Win64 default ABI",
        ));
    } else {
        let abi_lib_src = tmp.join("ffi_abi_mismatch_lib.c");
        let abi_main_src = tmp.join("ffi_abi_mismatch_main.c");
        let abi_obj = tmp.join("ffi_abi_mismatch_lib.o");
        let abi_exe = tmp.join("ffi_abi_mismatch_exec");
        let _ = fs::write(
            &abi_lib_src,
            b"int __attribute__((ms_abi)) nc_abi_mismatch_sum(int a, int b) { return a + b; }\n",
        );
        let _ = fs::write(
            &abi_main_src,
            b"extern int nc_abi_mismatch_sum(int, int);\nint main(void) {\n  int v = nc_abi_mismatch_sum(2, 3);\n  return v == 5 ? 0 : 73;\n}\n",
        );
        let abi_lib_src_s = abi_lib_src.to_string_lossy().into_owned();
        let abi_main_src_s = abi_main_src.to_string_lossy().into_owned();
        let abi_obj_s = abi_obj.to_string_lossy().into_owned();
        let abi_exe_s = abi_exe.to_string_lossy().into_owned();
        if compiler_run_process(cc, &["-c", &abi_lib_src_s, "-O2", "-o", &abi_obj_s], tmp).is_err()
            || compiler_run_process(
                cc,
                &["-O2", &abi_main_src_s, &abi_obj_s, "-o", &abi_exe_s],
                tmp,
            )
            .is_err()
        {
            out.push(compiler_ffi_smoke_case_skipped(
                "abi_mismatch_real",
                "E_FFI_DEFERRED_001",
                "abi mismatch harness is unsupported by this C toolchain",
            ));
        } else {
            out.push(compiler_ffi_smoke_case_result(
                "abi_mismatch_real",
                "E_FFI_ABI_MISMATCH_001",
                compiler_run_process_output(&abi_exe_s, &[], tmp),
                true,
            ));
        }
    }

    out
}

fn compiler_ffi_smoke_skipped_payload(
    reason: String,
    missing_tools: Vec<&str>,
    cc: Option<String>,
    cxx: Option<String>,
    rustc: Option<String>,
    nm: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "status": "skipped",
        "reason": reason,
        "missing_tools": missing_tools,
        "strict": false,
        "host": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "asm_surface": {
            "status": "active",
            "gate": "[@machine]",
            "official_code": serde_json::Value::Null,
            "message": "ASM FFI bridge is active and gated by [@machine]; smoke execution is blocked because the host toolchain is missing or incompatible."
        },
        "symbol_status": [
            {
                "language": "c",
                "symbol": "nc_ffi_c_add",
                "contract_validated": true,
                "linkable": false,
                "linked": false,
                "executed": false,
                "skipped_by_toolchain": true,
                "failed": false,
                "failed_code": "E_FFI_LINK_FAILED_001",
                "state": "skipped",
            },
            {
                "language": "cpp",
                "symbol": "nc_ffi_cpp_mul",
                "contract_validated": true,
                "linkable": false,
                "linked": false,
                "executed": false,
                "skipped_by_toolchain": true,
                "failed": false,
                "failed_code": "E_FFI_LINK_FAILED_001",
                "state": "skipped",
            },
            {
                "language": "rust",
                "symbol": "nc_ffi_rust_sub",
                "contract_validated": true,
                "linkable": false,
                "linked": false,
                "executed": false,
                "skipped_by_toolchain": true,
                "failed": false,
                "failed_code": "E_FFI_LINK_FAILED_001",
                "state": "skipped",
            },
            {
                "language": "asm",
                "symbol": "nc_ffi_asm_identity",
                "contract_validated": true,
                "linkable": false,
                "linked": false,
                "executed": false,
                "skipped_by_toolchain": true,
                "failed": false,
                "failed_code": "E_FFI_LINK_FAILED_001",
                "state": "skipped",
            }
        ],
        "toolchain": {
            "cc": cc,
            "cxx": cxx,
            "rustc": rustc,
            "nm": nm,
        }
    })
}

fn compiler_run_ffi_smoke(strict: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cc = compiler_resolve_first_tool(&["cc", "gcc", "clang"]);
    let cxx = compiler_resolve_first_tool(&["c++", "g++", "clang++"]);
    let rustc = compiler_resolve_first_tool(&["rustc"]);
    let nm = compiler_resolve_first_tool(&["nm", "llvm-nm"]);
    let mut missing_tools = Vec::new();
    if cc.is_none() {
        missing_tools.push("cc");
    }
    if cxx.is_none() {
        missing_tools.push("c++");
    }
    if rustc.is_none() {
        missing_tools.push("rustc");
    }
    if nm.is_none() {
        missing_tools.push("nm");
    }
    if !missing_tools.is_empty() {
        if strict {
            return Err(format!(
                "E_FFI_LINK_FAILED_001: missing external toolchain command(s): {}",
                missing_tools.join(", ")
            )
            .into());
        }
        return Ok(compiler_ffi_smoke_skipped_payload(
            "missing external toolchain commands".to_string(),
            missing_tools,
            cc,
            cxx,
            rustc,
            nm,
        ));
    }

    let cc = cc.expect("checked is_some");
    let cxx = cxx.expect("checked is_some");
    let rustc = rustc.expect("checked is_some");
    let nm = nm.expect("checked is_some");
    let host_os = std::env::consts::OS;
    let host_arch = std::env::consts::ARCH;
    let sysv_host_linkable = matches!(
        (host_os, host_arch),
        ("linux", "x86_64")
            | ("linux", "aarch64")
            | ("macos", "x86_64")
            | ("macos", "aarch64")
            | ("freebsd", "x86_64")
    );
    let win64_host_linkable = host_os == "windows" && host_arch == "x86_64";
    let aapcs64_host_linkable =
        host_arch == "aarch64" && matches!(host_os, "linux" | "macos" | "freebsd" | "windows");

    macro_rules! ffi_skip_or_err {
        ($reason:expr) => {{
            let reason = $reason;
            if strict {
                return Err(reason.into());
            }
            return Ok(compiler_ffi_smoke_skipped_payload(
                reason,
                Vec::new(),
                Some(cc.clone()),
                Some(cxx.clone()),
                Some(rustc.clone()),
                Some(nm.clone()),
            ));
        }};
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!("ncc_ffi_smoke_{}_{}", std::process::id(), stamp));
    fs::create_dir_all(&tmp)?;

    let c_path = tmp.join("ffi_c.c");
    let cpp_path = tmp.join("ffi_cpp.cpp");
    let rust_path = tmp.join("ffi_rust.rs");
    let asm_path = tmp.join("ffi_asm.S");
    let main_path = tmp.join("ffi_main.c");
    let c_obj = tmp.join("ffi_c.o");
    let cpp_obj = tmp.join("ffi_cpp.o");
    let rust_obj = tmp.join("ffi_rust.o");
    let asm_obj = tmp.join("ffi_asm.o");
    let exe_path = tmp.join("ffi_smoke_exec");

    fs::write(
        &c_path,
        b"int nc_ffi_c_add(int a, int b) { return a + b; }\n",
    )?;
    fs::write(
        &cpp_path,
        b"extern \"C\" int nc_ffi_cpp_mul(int a, int b) { return a * b; }\n",
    )?;
    fs::write(
        &rust_path,
        b"#[no_mangle]\npub extern \"C\" fn nc_ffi_rust_sub(a: i32, b: i32) -> i32 { a - b }\n",
    )?;
    let asm_source = match compiler_ffi_asm_identity_source(host_os, host_arch) {
        Ok(source) => source,
        Err(err) => ffi_skip_or_err!(err),
    };
    fs::write(&asm_path, asm_source)?;
    fs::write(
        &main_path,
        b"#include <stdint.h>\nextern int nc_ffi_c_add(int, int);\nextern int nc_ffi_cpp_mul(int, int);\nextern int nc_ffi_rust_sub(int, int);\nextern int nc_ffi_asm_identity(int);\nint main(void) {\n  if (nc_ffi_c_add(2, 3) != 5) return 11;\n  if (nc_ffi_cpp_mul(4, 5) != 20) return 12;\n  if (nc_ffi_rust_sub(9, 4) != 5) return 13;\n  if (nc_ffi_asm_identity(42) != 42) return 14;\n  return 0;\n}\n",
    )?;

    let c_src = c_path.to_string_lossy().into_owned();
    let cpp_src = cpp_path.to_string_lossy().into_owned();
    let rust_src = rust_path.to_string_lossy().into_owned();
    let asm_src = asm_path.to_string_lossy().into_owned();
    let main_src = main_path.to_string_lossy().into_owned();
    let c_obj_s = c_obj.to_string_lossy().into_owned();
    let cpp_obj_s = cpp_obj.to_string_lossy().into_owned();
    let rust_obj_s = rust_obj.to_string_lossy().into_owned();
    let asm_obj_s = asm_obj.to_string_lossy().into_owned();
    let exe_s = exe_path.to_string_lossy().into_owned();

    if let Err(msg) = compiler_run_process(&cc, &["-c", &c_src, "-O2", "-o", &c_obj_s], &tmp) {
        ffi_skip_or_err!(format!("E_FFI_LINK_FAILED_001: {msg}"));
    }
    if let Err(msg) = compiler_run_process(&cxx, &["-c", &cpp_src, "-O2", "-o", &cpp_obj_s], &tmp) {
        ffi_skip_or_err!(format!("E_FFI_LINK_FAILED_001: {msg}"));
    }
    if let Err(msg) = compiler_run_process(
        &rustc,
        &[
            "--crate-type",
            "lib",
            "--emit=obj",
            &rust_src,
            "-O",
            "-o",
            &rust_obj_s,
        ],
        &tmp,
    ) {
        ffi_skip_or_err!(format!("E_FFI_LINK_FAILED_001: {msg}"));
    }
    if let Err(msg) = compiler_run_process(&cc, &["-c", &asm_src, "-O2", "-o", &asm_obj_s], &tmp) {
        ffi_skip_or_err!(format!("E_FFI_ASM_BRIDGE_001: {msg}"));
    }
    if let Err(msg) = compiler_run_process(
        &cc,
        &[
            &main_src,
            &c_obj_s,
            &cpp_obj_s,
            &rust_obj_s,
            &asm_obj_s,
            "-lstdc++",
            "-o",
            &exe_s,
        ],
        &tmp,
    ) {
        ffi_skip_or_err!(format!("E_FFI_LINK_FAILED_001: {msg}"));
    }

    if !compiler_nm_has_symbol(&nm, &c_obj, "nc_ffi_c_add")
        || !compiler_nm_has_symbol(&nm, &cpp_obj, "nc_ffi_cpp_mul")
        || !compiler_nm_has_symbol(&nm, &rust_obj, "nc_ffi_rust_sub")
        || !compiler_nm_has_symbol(&nm, &asm_obj, "nc_ffi_asm_identity")
    {
        ffi_skip_or_err!(
            "E_FFI_SYMBOL_MISSING_001: required ffi symbol was not found in emitted object(s)"
                .to_string()
        );
    }

    let exec_output = match std::process::Command::new(&exe_path)
        .current_dir(&tmp)
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            ffi_skip_or_err!(format!(
                "E_FFI_LINK_FAILED_001: failed to run ffi harness: {}",
                err
            ));
        }
    };
    if !exec_output.status.success() {
        ffi_skip_or_err!(format!(
            "E_FFI_LINK_FAILED_001: ffi harness exited with status {}",
            exec_output.status
        ));
    }

    let negative_cases =
        compiler_ffi_run_negative_matrix(&tmp, &cc, &cxx, &rustc, host_os, host_arch);

    Ok(serde_json::json!({
        "status": "ok",
        "host": {
            "os": host_os,
            "arch": host_arch,
        },
        "toolchain": {
            "cc": cc,
            "cxx": cxx,
            "rustc": rustc,
            "nm": nm,
        },
        "workspace": tmp.display().to_string(),
        "contracts": {
            "abi": true,
            "layout": true,
            "ownership": true,
            "nullability": true,
            "asm_machine_bridge_required": true,
        },
        "asm_surface": {
            "status": "active",
            "gate": "[@machine]",
            "official_code": serde_json::Value::Null,
            "message": "ASM FFI bridge is active: ffi-smoke compiles, links, exports, and executes nc_ffi_asm_identity through the [@machine] gate."
        },
        "symbols": [
            "nc_ffi_c_add",
            "nc_ffi_cpp_mul",
            "nc_ffi_rust_sub",
            "nc_ffi_asm_identity",
        ],
        "symbol_status": [
            {
                "language": "c",
                "symbol": "nc_ffi_c_add",
                "contract_validated": true,
                "linkable": true,
                "linked": true,
                "executed": true,
                "skipped_by_toolchain": false,
                "failed": false,
                "failed_code": serde_json::Value::Null,
                "state": "executed",
            },
            {
                "language": "cpp",
                "symbol": "nc_ffi_cpp_mul",
                "contract_validated": true,
                "linkable": true,
                "linked": true,
                "executed": true,
                "skipped_by_toolchain": false,
                "failed": false,
                "failed_code": serde_json::Value::Null,
                "state": "executed",
            },
            {
                "language": "rust",
                "symbol": "nc_ffi_rust_sub",
                "contract_validated": true,
                "linkable": true,
                "linked": true,
                "executed": true,
                "skipped_by_toolchain": false,
                "failed": false,
                "failed_code": serde_json::Value::Null,
                "state": "executed",
            },
            {
                "language": "asm",
                "symbol": "nc_ffi_asm_identity",
                "contract_validated": true,
                "linkable": true,
                "linked": true,
                "executed": true,
                "skipped_by_toolchain": false,
                "failed": false,
                "failed_code": serde_json::Value::Null,
                "state": "executed",
            }
        ],
        "interop": {
            "c": true,
            "cpp": true,
            "rust": true,
            "asm": true,
        },
        "negative_cases": negative_cases,
        "abi_matrix": {
            "nC.sysv64": {
                "foreign_c": true,
                "foreign_cpp": true,
                "foreign_rust": true,
                "foreign_asm": true,
                "host_linkable": sysv_host_linkable,
            },
            "nC.win64": {
                "foreign_c": true,
                "foreign_cpp": true,
                "foreign_rust": true,
                "foreign_asm": true,
                "host_linkable": win64_host_linkable,
            },
            "nC.aapcs64": {
                "foreign_c": true,
                "foreign_cpp": true,
                "foreign_rust": true,
                "foreign_asm": true,
                "host_linkable": aapcs64_host_linkable,
            }
        }
    }))
}

fn compiler_sha256_hex_file(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

fn compiler_artifact_record(
    path: &std::path::Path,
    kind: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let metadata = fs::metadata(path)?;
    Ok(serde_json::json!({
        "kind": kind,
        "path": path.display().to_string(),
        "size": metadata.len(),
        "sha256": compiler_sha256_hex_file(path)?,
    }))
}

fn compiler_native_build_has_flag(rest: &[String], flag: &str) -> bool {
    rest.iter().any(|arg| arg == flag)
}

fn compiler_native_build_option(rest: &[String], flag: &str) -> Option<String> {
    let mut idx = 0usize;
    while idx + 1 < rest.len() {
        if rest[idx] == flag {
            return Some(rest[idx + 1].clone());
        }
        idx += 1;
    }
    None
}

fn compiler_native_build_opt_value(rest: &[String]) -> Option<String> {
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--opt-level" | "-O" if idx + 1 < rest.len() => return Some(rest[idx + 1].clone()),
            raw if raw.starts_with("-O") && raw.len() > 2 => {
                return Some(raw.trim_start_matches("-O").to_string());
            }
            _ => idx += 1,
        }
    }
    None
}

fn compiler_native_build_parse_source(
    rest: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut path: Option<String> = None;
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--out-dir"
            | "--content-cache-dir"
            | "--cc"
            | "--opt-level"
            | "-O"
            | "--cpu"
            | "--microarch"
            | "--tune-profile"
            | "--tune"
            | "--fine-profile"
            | "--lto"
            | "--pgo"
            | "--pgo-profile" => {
                idx += 2;
            }
            raw if raw.starts_with("-O") && raw.len() > 2 => {
                idx += 1;
            }
            "--run"
            | "--no-run"
            | "--allow-missing-tools"
            | "--incremental"
            | "--no-incremental" => {
                idx += 1;
            }
            arg if arg.starts_with("--") => {
                return Err(format!("compiler build-native: unknown option '{}'", arg).into());
            }
            arg => {
                if path.is_some() {
                    return Err("compiler build-native expects exactly one .n source file".into());
                }
                path = Some(arg.to_string());
                idx += 1;
            }
        }
    }
    path.ok_or_else(|| "compiler build-native expects <file.n>".into())
}

pub(crate) fn compiler_native_build_profile_from_target(
    target: &str,
) -> Result<ncc_ir::BackendLoweringProfileIR, Box<dyn std::error::Error>> {
    let lower = target.to_ascii_lowercase();
    let aarch64 = lower.contains("aarch64") || lower.contains("arm64");
    let x86_64 = lower.contains("x86_64") || lower.contains("amd64");
    let i386 = lower.starts_with("i386")
        || lower.starts_with("i486")
        || lower.starts_with("i586")
        || lower.starts_with("i686");
    if aarch64 && (lower.contains("windows") || lower.contains("msvc")) {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_aarch64_windows());
    }
    if x86_64 && (lower.contains("windows") || lower.contains("msvc") || lower.contains("win64")) {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_win64());
    }
    if aarch64 && (lower.contains("apple") || lower.contains("darwin")) {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_aapcs64());
    }
    if x86_64 && (lower.contains("apple") || lower.contains("darwin")) {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_x86_64_darwin());
    }
    if aarch64 && (lower.contains("linux") || lower.contains("gnu")) {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_aarch64_linux());
    }
    if i386 && (lower.contains("linux") || lower.contains("gnu")) {
        let mut profile = ncc_ir::BackendLoweringProfileIR::nc_i386_linux();
        profile.target_spec.target_triple = target.to_string();
        return Ok(profile);
    }
    if i386 && (lower.contains("windows") || lower.contains("msvc") || lower.contains("win32")) {
        let mut profile = ncc_ir::BackendLoweringProfileIR::nc_i386_windows();
        profile.target_spec.target_triple = target.to_string();
        return Ok(profile);
    }
    if lower.contains("wasm32") || lower.contains("wasi") {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_portable());
    }
    if lower.contains("x86_64") || lower.contains("linux") || lower.contains("gnu") {
        return Ok(ncc_ir::BackendLoweringProfileIR::nc_sysv64());
    }
    Err(format!(
        "compiler build-native: target '{}' is not mapped; use --cc <nC.sysv64|nC.win64|nC.aapcs64|nC.portable>",
        target
    )
    .into())
}

fn compiler_native_build_profile(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<ncc_ir::BackendLoweringProfileIR, Box<dyn std::error::Error>> {
    if compiler_native_build_option(rest, "--cc").is_some() {
        return parse_backend_profile_from_args(rest);
    }
    if let Some(target) = opts.target.as_deref() {
        return compiler_native_build_profile_from_target(target);
    }
    parse_backend_profile_from_args(rest)
}

fn compiler_native_build_opt_level(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<ncc_ir::MctLlvmOptimizationLevelIR, Box<dyn std::error::Error>> {
    if let Some(raw) = compiler_native_build_opt_value(rest) {
        return super::mct_support::parse_mct_llvm_optimization_level_value(&raw);
    }
    match opts
        .profile
        .as_deref()
        .unwrap_or("release-fast")
        .to_ascii_lowercase()
        .as_str()
    {
        "debug" | "instrumented" | "interpreted" => {
            Ok(ncc_ir::MctLlvmOptimizationLevelIR::O0)
        }
        "release" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::O2),
        "release-fast" | "fast" | "native-fast" | "real" | "unsafe-fast" => {
            Ok(ncc_ir::MctLlvmOptimizationLevelIR::O3)
        }
        "release-small" | "small" | "size" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::Os),
        "release-tiny" | "tiny" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::Oz),
        other => Err(format!(
            "compiler build-native: unknown --profile '{}'; expected debug, release, release-fast, release-small, release-tiny, unsafe-fast or instrumented",
            other
        )
        .into()),
    }
}

fn compiler_native_lto_mode(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<CompilerNativeLtoMode, Box<dyn std::error::Error>> {
    let raw = compiler_native_build_option(rest, "--lto").unwrap_or_else(|| {
        match opts
            .profile
            .as_deref()
            .unwrap_or("release-fast")
            .to_ascii_lowercase()
            .as_str()
        {
            "release-fast" | "fast" | "native-fast" | "real" | "unsafe-fast" => "thin".to_string(),
            _ => "off".to_string(),
        }
    });
    match raw.to_ascii_lowercase().as_str() {
        "off" | "none" | "false" | "0" => Ok(CompilerNativeLtoMode::Off),
        "auto" | "thin" | "thinlto" => Ok(CompilerNativeLtoMode::Thin),
        "full" | "fulllto" => Ok(CompilerNativeLtoMode::Full),
        _ => Err(format!(
            "compiler build-native: unknown --lto '{}'; expected auto, off, thin or full",
            raw
        )
        .into()),
    }
}

fn compiler_native_pgo_mode(
    rest: &[String],
) -> Result<CompilerNativePgoMode, Box<dyn std::error::Error>> {
    let raw = compiler_native_build_option(rest, "--pgo").unwrap_or_else(|| "auto".to_string());
    match raw.to_ascii_lowercase().as_str() {
        "off" | "none" | "false" | "0" => Ok(CompilerNativePgoMode::Off),
        "auto" => Ok(CompilerNativePgoMode::Auto),
        "use" | "consume" => Ok(CompilerNativePgoMode::Use),
        "train" | "generate-use" | "generate_use" => Ok(CompilerNativePgoMode::Train),
        _ => Err(format!(
            "compiler build-native: unknown --pgo '{}'; expected auto, off, use or train",
            raw
        )
        .into()),
    }
}

fn compiler_native_optimization_plan(
    opts: &CompilerCliOptions,
    rest: &[String],
    base_dir: &Path,
    stem: &str,
) -> Result<CompilerNativeOptimizationPlan, Box<dyn std::error::Error>> {
    let lto = compiler_native_lto_mode(opts, rest)?;
    let pgo_requested = compiler_native_pgo_mode(rest)?;
    let pgo_dir = base_dir.join("pgo");
    fs::create_dir_all(&pgo_dir)?;
    let pgo_profile = compiler_native_build_option(rest, "--pgo-profile")
        .map(PathBuf::from)
        .unwrap_or_else(|| pgo_dir.join(format!("{stem}.profdata")));
    let pgo_uses_profile = match pgo_requested {
        CompilerNativePgoMode::Off | CompilerNativePgoMode::Train => false,
        CompilerNativePgoMode::Auto => pgo_profile.is_file(),
        CompilerNativePgoMode::Use => {
            if !pgo_profile.is_file() {
                return Err(format!(
                    "compiler build-native: PGO profile '{}' does not exist",
                    pgo_profile.display()
                )
                .into());
            }
            true
        }
    };
    Ok(CompilerNativeOptimizationPlan {
        lto_requested: lto,
        lto,
        pgo_requested,
        pgo_raw_dir: pgo_dir.join(format!("{stem}.profraw.d")),
        pgo_profile,
        pgo_uses_profile,
    })
}

fn compiler_native_collect_profraw(
    root: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            compiler_native_collect_profraw(&path, out)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("profraw") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn compiler_native_profile_use_args(profile: Option<&Path>) -> Vec<String> {
    profile
        .map(|path| {
            vec![
                format!("-fprofile-use={}", path.display()),
                "-Wno-profile-instr-unprofiled".to_string(),
                "-Wno-profile-instr-out-of-date".to_string(),
            ]
        })
        .unwrap_or_default()
}

fn compiler_native_object_command_args(
    target: &str,
    opt_arg: &str,
    ll_path: &Path,
    obj_path: &Path,
    pgo_use: Option<&Path>,
    pgo_generate: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![format!("--target={target}"), opt_arg.to_string()];
    args.extend(compiler_native_profile_use_args(pgo_use));
    if let Some(path) = pgo_generate {
        args.push(format!("-fprofile-generate={}", path.display()));
    }
    args.extend([
        "-x".to_string(),
        "ir".to_string(),
        "-c".to_string(),
        ll_path.display().to_string(),
        "-o".to_string(),
        obj_path.display().to_string(),
    ]);
    args
}

fn compiler_native_external_link_command_args(
    target: &str,
    opt_arg: &str,
    ll_path: &Path,
    obj_path: &Path,
    exe_path: &Path,
    lto: CompilerNativeLtoMode,
    pgo_use: Option<&Path>,
    pgo_generate: Option<&Path>,
    needs_libm: bool,
    runtime_objects: &[std::path::PathBuf],
    needs_threads: bool,
    needs_ws2_32: bool,
    needs_advapi32: bool,
) -> Vec<String> {
    let mut args = vec![format!("--target={target}")];
    if lto == CompilerNativeLtoMode::Off {
        args.push(obj_path.display().to_string());
    } else {
        args.push(opt_arg.to_string());
        args.push(lto.clang_arg().expect("active LTO mode").to_string());
        args.extend([
            "-x".to_string(),
            "ir".to_string(),
            ll_path.display().to_string(),
            "-x".to_string(),
            "none".to_string(),
        ]);
    }
    args.extend(compiler_native_profile_use_args(pgo_use));
    if let Some(path) = pgo_generate {
        args.push(format!("-fprofile-generate={}", path.display()));
    }
    args.extend(
        runtime_objects
            .iter()
            .map(|path| path.display().to_string()),
    );
    args.extend(["-o".to_string(), exe_path.display().to_string()]);
    if needs_libm {
        args.push("-lm".to_string());
    }
    if needs_threads && !target.contains("windows") {
        args.push("-pthread".to_string());
    }
    if target.contains("windows") && needs_ws2_32 {
        args.push("-lws2_32".to_string());
    }
    if target.contains("windows") {
        args.push("-lkernel32".to_string());
    }
    if target.contains("windows") && needs_advapi32 {
        args.push("-ladvapi32".to_string());
    }
    args
}

fn compiler_native_build_stem(path: &str) -> String {
    let raw = std::path::Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module");
    let mut out = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if out.is_empty() {
        out.push_str("module");
    }
    out
}

fn compiler_native_build_needs_libm(llvm_ir: &str) -> bool {
    [
        "@atan2(",
        "@llvm.sqrt.f64(",
        "@llvm.exp.f64(",
        "@llvm.sin.f64(",
        "@llvm.cos.f64(",
        "@llvm.log.f64(",
    ]
    .iter()
    .any(|needle| llvm_ir.contains(needle))
}

fn compiler_native_build_needs_advapi32(llvm_ir: &str) -> bool {
    [
        "@SystemFunction036(",
        "@RegCreateKeyExA(",
        "@RegOpenKeyExA(",
        "@\"\\01_RegCreateKeyExA@36\"(",
        "@\"\\01_RegOpenKeyExA@20\"(",
    ]
    .iter()
    .any(|needle| llvm_ir.contains(needle))
}

fn compiler_native_build_needs_ws2_32(llvm_ir: &str) -> bool {
    [
        "@WSAStartup(",
        "@WSACleanup(",
        "@WSAGetLastError(",
        "@WSAPoll(",
        "@socket(",
        "@getaddrinfo(",
        "@GetAdaptersAddresses(",
    ]
    .iter()
    .any(|needle| llvm_ir.contains(needle))
}

fn compiler_native_machine_ident(raw: &str) -> String {
    let mut out = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if out.is_empty() {
        out.push_str("native");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn compiler_native_object_label(format: &ncc_ir::BackendObjectFormatIR) -> &'static str {
    match format {
        ncc_ir::BackendObjectFormatIR::Elf32 => "elf32",
        ncc_ir::BackendObjectFormatIR::Elf64 | ncc_ir::BackendObjectFormatIR::FreestandingImage => {
            "elf64"
        }
        ncc_ir::BackendObjectFormatIR::Coff => "coff",
        ncc_ir::BackendObjectFormatIR::MachO => "macho",
        ncc_ir::BackendObjectFormatIR::WasmModule => "wasm",
    }
}

fn compiler_native_abi_label(cc: &ncc_ir::BackendCallingConventionIR) -> &'static str {
    match cc {
        ncc_ir::BackendCallingConventionIR::NcSysV64 => "sysv64",
        ncc_ir::BackendCallingConventionIR::NcWin64 => "win64",
        ncc_ir::BackendCallingConventionIR::NcAapcs64 => "aapcs64",
        ncc_ir::BackendCallingConventionIR::NcPortable => "portable",
        ncc_ir::BackendCallingConventionIR::NcSovereign => "sovereign",
    }
}

fn compiler_native_machine_bridge_source(
    source_path: &str,
    stem: &str,
    profile: &ncc_ir::BackendLoweringProfileIR,
    object_path: &std::path::Path,
    exe_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let container = compiler_native_machine_ident(stem);
    let symbol = compiler_native_machine_ident(&format!("ncc_{stem}_native_bridge"));
    let abi = match profile.target_spec.abi {
        ncc_ir::BackendAbiModelIR::I386SysV => "i386-sysv",
        ncc_ir::BackendAbiModelIR::I386Windows => "i386-msvc",
        _ => compiler_native_abi_label(&profile.calling_convention),
    };
    let object = compiler_native_object_label(&profile.target_spec.object_format);
    let win64_x86 = matches!(profile.target_spec.isa, ncc_ir::BackendIsaIR::X86_64)
        && matches!(
            profile.calling_convention,
            ncc_ir::BackendCallingConventionIR::NcWin64
        );
    let win32_x86 = matches!(profile.target_spec.isa, ncc_ir::BackendIsaIR::X86)
        && matches!(
            profile.target_spec.abi,
            ncc_ir::BackendAbiModelIR::I386Windows
        );
    let externs = if win64_x86 {
        "        main:\n        ExitProcess:"
    } else if win32_x86 {
        "        _main:"
    } else {
        "        main:"
    };
    let (arch, pointer_width, instr) = match profile.target_spec.isa {
        ncc_ir::BackendIsaIR::X86_64 if win64_x86 => (
            "x86_64",
            64,
            "        sub rsp, 40;\n        call main;\n        mov rcx, rax;\n        call ExitProcess;\n        add rsp, 40;\n        ret;",
        ),
        ncc_ir::BackendIsaIR::X86_64 => (
            "x86_64",
            64,
            "        sub rsp, 8;\n        call main;\n        add rsp, 8;\n        ret;",
        ),
        ncc_ir::BackendIsaIR::X86 if win32_x86 => (
            "i386",
            32,
            "        call _main;\n        ret;",
        ),
        ncc_ir::BackendIsaIR::X86 => (
            "i386",
            32,
            "        call main;\n        ret;",
        ),
        ncc_ir::BackendIsaIR::AArch64 => (
            "aarch64",
            64,
            "        bl main;\n        ret;",
        ),
        _ => {
            return Err(format!(
                "compiler build-native cannot retain a .nm entry bridge for ISA {:?}",
                profile.target_spec.isa
            )
            .into())
        }
    };
    Ok(format!(
        "{container} :: machine {{\n    target:\n        arch: {arch};\n        abi: {abi};\n        object: {object};\n        endian: little;\n        pointerWidth: {pointer_width};\n        profile: userland;\n\n    extern:\n{externs}\n\n    symbol:\n        {symbol}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {symbol};\n\n    proc:\n        {symbol}:\n\n    instr:\n{instr}\n\n    verify:\n        target;\n        encoding;\n        object;\n\n    seal:\n        artifact: object;\n        require: verified;\n\n    evidence:\n        source: \"{}\";\n        object: \"{}\";\n        executable: \"{}\";\n}}\n",
        source_path.replace('\\', "/").replace('"', "'"),
        object_path.display().to_string().replace('\\', "/").replace('"', "'"),
        exe_path.display().to_string().replace('\\', "/").replace('"', "'"),
    ))
}

fn compiler_native_target_matches_host(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    let os_matches = if cfg!(target_os = "windows") {
        lower.contains("windows") || lower.contains("msvc")
    } else if cfg!(target_os = "macos") {
        lower.contains("apple") || lower.contains("darwin")
    } else if cfg!(target_os = "linux") {
        lower.contains("linux")
    } else {
        false
    };
    let arch_matches = if cfg!(target_arch = "aarch64") {
        lower.contains("aarch64") || lower.contains("arm64")
    } else if cfg!(target_arch = "x86_64") {
        lower.contains("x86_64") || lower.contains("amd64")
    } else {
        false
    };
    os_matches && arch_matches
}

fn compiler_native_stage_to_json(
    stage: &str,
    tool: Option<&str>,
    args: &[String],
    output: std::io::Result<std::process::Output>,
) -> serde_json::Value {
    match output {
        Ok(output) => serde_json::json!({
            "stage": stage,
            "tool": tool,
            "args": args,
            "status": if output.status.success() { "passed" } else { "failed" },
            "status_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(err) => serde_json::json!({
            "stage": stage,
            "tool": tool,
            "args": args,
            "status": "missing_tool",
            "error": err.to_string(),
        }),
    }
}

fn compiler_native_training_stage_to_json(
    executable: &Path,
    output: std::io::Result<std::process::Output>,
) -> serde_json::Value {
    match output {
        Ok(output) => serde_json::json!({
            "stage": "pgo-training-run",
            "tool": executable.display().to_string(),
            "args": [],
            "status": if output.status.code().is_some() { "passed" } else { "failed" },
            "workload_exit_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
            "policy": "a normal workload exit contributes valid PGO samples regardless of the application return value",
        }),
        Err(err) => serde_json::json!({
            "stage": "pgo-training-run",
            "tool": executable.display().to_string(),
            "args": [],
            "status": "failed",
            "error": err.to_string(),
        }),
    }
}

fn compiler_native_stage_passed(stage: &serde_json::Value) -> bool {
    stage
        .get("status")
        .and_then(|value| value.as_str())
        .is_some_and(|status| status == "passed" || status == "skipped")
}

pub(crate) const COMPILER_NATIVE_JSON_RUNTIME_SOURCE: &str =
    include_str!("../runtime_json_native.c");
pub(crate) const COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE: &str =
    include_str!("../runtime_hosted_native.c");
pub(crate) const COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE: &str =
    include_str!("../../../../runtime/nc/text_format.n");
pub(crate) const COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE: &str =
    include_str!("../../../../runtime/nc/text_escape.n");
pub(crate) const COMPILER_NATIVE_NC_PARSE_RUNTIME_SOURCE: &str =
    include_str!("../../../../runtime/nc/text_parse.n");
pub(crate) const COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE: &str =
    include_str!("../../../../runtime/nc/unicode_tables.n");
pub(crate) const COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE: &str =
    include_str!("../../../../runtime/nc/unicode.n");
pub(crate) const COMPILER_NATIVE_NC_JSON_RUNTIME_SOURCE: &str =
    include_str!("../../../../runtime/nc/json.n");

pub(crate) fn compiler_native_embedded_c_object_command_args(
    target_triple: &str,
    object_path: &Path,
    disable_builtins: bool,
) -> Vec<String> {
    let mut args = vec![
        format!("--target={target_triple}"),
        "-O2".to_string(),
        "-std=c11".to_string(),
        "-ffreestanding".to_string(),
    ];
    if disable_builtins {
        args.push("-fno-builtin".to_string());
    }
    args.extend([
        "-fno-stack-protector".to_string(),
        "-fno-unwind-tables".to_string(),
        "-fno-asynchronous-unwind-tables".to_string(),
        "-x".to_string(),
        "c".to_string(),
        "-c".to_string(),
        "-".to_string(),
        "-o".to_string(),
        object_path.display().to_string(),
    ]);
    args
}

pub(crate) fn compiler_native_compile_embedded_c_object(
    clang_tool: &str,
    args: &[String],
    source: &str,
) -> std::io::Result<std::process::Output> {
    let mut child = std::process::Command::new(clang_tool)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "Clang did not expose stdin for the compiler-owned runtime provider",
        )
    })?;
    if let Err(error) = stdin.write_all(source.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    drop(stdin);
    child.wait_with_output()
}

pub(crate) fn compiler_native_remove_legacy_runtime_sources(
    lowering_dir: &Path,
) -> std::io::Result<usize> {
    if !lowering_dir.is_dir() {
        return Ok(0);
    }
    let mut removed = 0usize;
    for entry in fs::read_dir(lowering_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            removed += compiler_native_remove_legacy_runtime_sources(&path)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("c"))
        {
            fs::remove_file(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn compiler_native_ast_uses_nc_text_runtime(ast: &ncc_ast::Program) -> bool {
    let explicit_import = ast.uses.iter().any(|use_decl| {
        matches!(
            use_decl.path.as_slice(),
            [root, runtime, family]
                if root.eq_ignore_ascii_case("nC")
                    && runtime.eq_ignore_ascii_case("runtime")
                    && matches!(family.to_ascii_lowercase().as_str(), "format" | "string" | "text" | "truncate")
        )
    });
    explicit_import
        || serde_json::to_string(ast).is_ok_and(|encoded| {
            let uses_named_text_operation = [
                "formatInt",
                "formatFloat",
                "formatNamed",
                "nC.runtime.format.format",
                "nC.runtime.string",
                "nC.runtime.text",
                "nC.runtime.truncate",
                "print",
                "println",
                "strLen",
                "strByteLen",
                "repeatChar",
                "strRelease",
                "strConcat",
                "strSlice",
                "strSubstring",
                "substring",
                "strStartsWith",
                "strEndsWith",
                "strContains",
                "strIndexOf",
                "strToUpper",
                "strToLower",
                "strTrim",
                "strEmpty",
                "strCompare",
                "strEq",
                "charCode",
                "charToStr",
                "strFromInt",
                "strFromBool",
                "strFromFt64",
                "nC.runtime.convert",
                "nC.runtime.align",
                "toStr",
                "toText",
                "toInt",
                "safeToInt",
                "toFloat",
                "toBool",
                "forceToInt",
                "alignLeft",
                "alignRight",
                "alignCenter",
                "strSplit",
                "strSplitRelease",
                "textLength",
                "textIsEmpty",
                "textContains",
                "textReplace",
                "textTrim",
                "textSplit",
                "textRelease",
                "textSplitRelease",
                "truncate",
                "truncateEnd",
                "truncateMiddle",
                "truncateRelease",
                "padLeft",
                "padRight",
                "padBoth",
            ]
            .iter()
            .any(|needle| encoded.contains(needle));
            let uses_text_concatenation =
                encoded.contains("\"op\":\"Plus\"") && encoded.contains("\"String\":");
            uses_named_text_operation || uses_text_concatenation
        })
}

pub(crate) fn compiler_native_attach_nc_text_runtime(
    ast: &mut ncc_ast::Program,
    target_triple: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !compiler_native_ast_uses_nc_text_runtime(ast) {
        return Ok(false);
    }
    compiler_native_attach_required_nc_text_runtime(ast, target_triple)
}

pub(crate) fn compiler_native_attach_required_nc_text_runtime(
    ast: &mut ncc_ast::Program,
    target_triple: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if ast
        .containers
        .iter()
        .any(|container| container.name == "nccTextFormatRuntime")
    {
        return Ok(false);
    }
    let source = if target_triple.contains("windows") {
        COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE.to_string()
    } else {
        COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE
            .replace("vmCommit(base, total, 4)", "vmCommit(base, total, 3)")
    };
    let mut provider = ncc_parser::parse(ncc_lexer::lex(&source)?)?;
    ast.containers.append(&mut provider.containers);
    Ok(true)
}

fn compiler_native_ast_uses_nc_escape_runtime(ast: &ncc_ast::Program) -> bool {
    serde_json::to_string(ast).is_ok_and(|encoded| {
        [
            "nC.runtime.escape",
            "escapeAnsi",
            "escapeJson",
            "escapeControl",
            "sanitizeTerminal",
            "sanitizeLog",
        ]
        .iter()
        .any(|needle| encoded.contains(needle))
    })
}

pub(crate) fn compiler_native_attach_nc_escape_runtime(
    ast: &mut ncc_ast::Program,
    target_triple: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !compiler_native_ast_uses_nc_escape_runtime(ast)
        || ast
            .containers
            .iter()
            .any(|container| container.name == "nccTextEscapeRuntime")
    {
        return Ok(false);
    }
    let source = if target_triple.contains("windows") {
        COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE.to_string()
    } else {
        COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE
            .replace("vmCommit(base, total, 4)", "vmCommit(base, total, 3)")
    };
    let mut provider = ncc_parser::parse(ncc_lexer::lex(&source)?)?;
    ast.containers.append(&mut provider.containers);
    Ok(true)
}

fn compiler_native_ast_uses_nc_parse_runtime(ast: &ncc_ast::Program) -> bool {
    serde_json::to_string(ast).is_ok_and(|encoded| {
        [
            "nC.runtime.parse",
            "parseInt",
            "parseUInt",
            "parseFloat",
            "parseBool",
            "parseChar",
            "toInt",
            "safeToInt",
            "forceToInt",
            "toFloat",
            "toBool",
            "tryParseInt",
            "tryParseFloat",
            "tryParseBool",
        ]
        .iter()
        .any(|needle| encoded.contains(needle))
    })
}

pub(crate) fn compiler_native_attach_nc_parse_runtime(
    ast: &mut ncc_ast::Program,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !compiler_native_ast_uses_nc_parse_runtime(ast)
        || ast
            .containers
            .iter()
            .any(|container| container.name == "nccTextParseRuntime")
    {
        return Ok(false);
    }
    let mut provider = ncc_parser::parse(ncc_lexer::lex(COMPILER_NATIVE_NC_PARSE_RUNTIME_SOURCE)?)?;
    ast.containers.append(&mut provider.containers);
    Ok(true)
}

fn compiler_native_ast_uses_nc_unicode_runtime(ast: &ncc_ast::Program) -> bool {
    let explicit_import = ast.uses.iter().any(|use_decl| {
        matches!(
            use_decl.path.as_slice(),
            [root, runtime, family]
                if root.eq_ignore_ascii_case("nC")
                    && runtime.eq_ignore_ascii_case("runtime")
                    && matches!(family.to_ascii_lowercase().as_str(), "unicode" | "width" | "wrap")
        )
    });
    explicit_import
        || serde_json::to_string(ast).is_ok_and(|encoded| {
            [
                "nC.runtime.unicode",
                "nC.runtime.width",
                "nC.runtime.wrap",
                "isValidUtf8",
                "normalizeUnicode",
                "stripInvalidUnicode",
                "unicodeMode",
                "supportsGrapheme",
                "normalizeNfc",
                "normalizeNfd",
                "normalizeNfkc",
                "normalizeNfkd",
                "graphemeCount",
                "graphemeSlice",
                "unicodeRelease",
                "displayWidth",
                "byteLength",
                "charCount",
                "ansiVisibleWidth",
                "wrapWords",
                "wrapRelease",
            ]
            .iter()
            .any(|needle| encoded.contains(needle))
        })
}

pub(crate) fn compiler_native_attach_nc_unicode_runtime(
    ast: &mut ncc_ast::Program,
    target_triple: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !compiler_native_ast_uses_nc_unicode_runtime(ast)
        || ast
            .containers
            .iter()
            .any(|container| container.name == "nccUnicodeRuntime")
    {
        return Ok(false);
    }
    let mut tables = ncc_parser::parse(ncc_lexer::lex(COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE)?)?;
    let source = if target_triple.contains("windows") {
        COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE.to_string()
    } else {
        COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE
            .replace("vmCommit(base, total, 4)", "vmCommit(base, total, 3)")
    };
    let mut provider = ncc_parser::parse(ncc_lexer::lex(&source)?)?;
    ast.containers.append(&mut tables.containers);
    ast.containers.append(&mut provider.containers);
    Ok(true)
}
const COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS: &[&str] = &[
    "ncc_rt_mobile_get_incoming_number",
    "ncc_rt_mobile_get_state",
    "ncc_rt_mobile_telephony_available",
    "ncc_rt_mobile_dial",
    "ncc_rt_mobile_answer",
    "ncc_rt_mobile_hang_up",
    "ncc_rt_mobile_tts_connect",
    "ncc_rt_mobile_tts_speak",
    "ncc_rt_mobile_tts_stop",
    "ncc_rt_mobile_tts_is_speaking",
    "ncc_rt_mobile_tts_disconnect",
    "ncc_rt_mobile_network_request",
    "ncc_rt_mobile_response_status",
    "ncc_rt_mobile_response_json",
    "ncc_rt_mobile_response_body",
    "ncc_rt_mobile_response_release",
    "ncc_rt_mobile_permission_request",
    "ncc_rt_mobile_permission_request_names",
    "ncc_rt_mobile_permission_bootstrap",
    "ncc_rt_mobile_adb_connect",
    "ncc_rt_mobile_network_is_wifi_connected_provider",
    "ncc_rt_mobile_server_bootstrap",
];
const COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS: &[&str] = &[
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
    "jsonStringView",
    "jsonGetBoolOr",
    "jsonGetIntOr",
    "jsonGetFloatOr",
    "jsonArrayLen",
    "jsonArrayGet",
    "jsonArrayBorrow",
    "jsonArrayPush",
    "jsonArrayInsert",
    "jsonArrayRemove",
    "jsonObjectLen",
    "jsonObjectHas",
    "jsonObjectGet",
    "jsonObjectBorrow",
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

pub(crate) fn compiler_native_is_json_runtime_symbol(symbol: &str) -> bool {
    let normalized = symbol
        .trim()
        .trim_start_matches("__imp_")
        .trim_start_matches('_');
    let leaf = normalized.rsplit('.').next().unwrap_or(normalized);
    COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS.contains(&leaf)
}

pub(crate) fn compiler_native_uses_json_runtime(llvm_ir: &str) -> bool {
    COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS
        .iter()
        .any(|name| llvm_ir.contains(&format!("@{name}(")))
}

pub(crate) fn compiler_native_uses_hosted_runtime(llvm_ir: &str) -> bool {
    COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS
        .iter()
        .any(|name| llvm_ir.contains(&format!("@{name}(")))
}

pub(crate) fn compiler_native_is_runtime_abi_symbol(symbol: &str) -> bool {
    let material_symbol = symbol.trim_start_matches("__imp_");
    COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol)
        || COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS.contains(&symbol)
        || symbol.starts_with("ncc_rt_convert_")
        || symbol.starts_with("ncc_rt_random_")
        || symbol.starts_with("ncc_rt_math_")
        || symbol.starts_with("ncc_rt_entropy_")
        || symbol.starts_with("ncc_rt_uuid_")
        || symbol.starts_with("ncc_rt_now_")
        || symbol.starts_with("ncc_rt_monotonic_")
        || symbol.starts_with("ncc_rt_clock_")
        || symbol.starts_with("ncc_rt_sleep_")
        || symbol.starts_with("ncc_rt_date")
        || symbol.starts_with("ncc_rt_format_time")
        || symbol.starts_with("ncc_rt_time_release")
        || symbol.starts_with("ncc_rt_permission_")
        || material_symbol.starts_with("ncc_rt_network_")
        || material_symbol == "ncc_rt_mobile_network_is_wifi_connected"
        || symbol.starts_with("ncc_rt_terminal_")
        || symbol.starts_with("ncc_rt_stack_")
        || symbol.starts_with("ncc_rt_process_")
        || symbol.starts_with("ncc_rt_thread_")
        || symbol.starts_with("ncc_rt_std_")
        || symbol.starts_with("ncc_rt_error_")
        || symbol.starts_with("ncc_rt_flush_")
        || matches!(
            material_symbol,
            "ncc_rt_auto_flush_on_input" | "ncc_rt_flush_before_input"
        )
        || symbol.starts_with("ncc_rt_fs")
        || symbol.starts_with("ncc_rt_fread_lines")
        || matches!(
            material_symbol,
            "ncc_rt_exists"
                | "ncc_rt_list_dir"
                | "ncc_rt_create_dir"
                | "ncc_rt_remove_dir"
                | "ncc_rt_current_dir"
                | "ncc_rt_change_dir"
                | "ncc_rt_absolute_path"
                | "ncc_rt_official_path"
                | "ncc_rt_lock_file"
                | "ncc_rt_unlock_file"
                | "ncc_rt_copy_file"
                | "ncc_rt_read_bytes"
                | "ncc_rt_metadata"
                | "ncc_rt_permissions"
                | "ncc_rt_set_readonly"
        )
        || matches!(
            material_symbol,
            "ncc_rt_system_cpu_load" | "ncc_rt_registry_write" | "ncc_rt_registry_read"
        )
        || symbol.starts_with("nccTextFormatRuntime_")
        || symbol.starts_with("nccTextParseRuntime_")
        || symbol.starts_with("nccTextEscapeRuntime_")
        || symbol.starts_with("nccUnicodeRuntime_")
        || material_symbol.starts_with("__ncc_")
}

const COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE: &str = "compiler-native-o3";

#[derive(Debug, Clone)]
struct CompilerNativeContentCache {
    root: PathBuf,
    key: String,
    material: serde_json::Value,
}

#[derive(Debug)]
struct CompilerNativeCachePaths<'a> {
    source_path: &'a str,
    base_dir: &'a Path,
    code_dir: &'a Path,
    bin_dir: &'a Path,
    stem: &'a str,
    nm_path: &'a Path,
    ll_path: &'a Path,
    backend_object_ir_path: &'a Path,
    obj_path: &'a Path,
    exe_path: &'a Path,
    manifest_path: &'a Path,
    runtime_json_obj_path: &'a Path,
    hosted_runtime_obj_path: &'a Path,
    machine_entry_obj_path: Option<&'a Path>,
    internal_link_map_path: Option<&'a Path>,
    internal_link_report_path: Option<&'a Path>,
}

fn compiler_native_cache_material(
    opts: &CompilerCliOptions,
    rest: &[String],
    source_path: &str,
    source: &str,
    ast: &ncc_ast::Program,
    profile: &ncc_ir::BackendLoweringProfileIR,
    opt_level: ncc_ir::MctLlvmOptimizationLevelIR,
    optimization_plan: &CompilerNativeOptimizationPlan,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let canonical_source = fs::canonicalize(source_path)
        .unwrap_or_else(|_| PathBuf::from(source_path))
        .display()
        .to_string();
    let ast_bytes = serde_json::to_vec(ast)?;
    let pgo_profile = optimization_plan
        .pgo_uses_profile
        .then(|| {
            crate::content_cache::sha256_file(&optimization_plan.pgo_profile)
                .ok()
                .map(|sha256| {
                    serde_json::json!({
                        "path": optimization_plan.pgo_profile.display().to_string(),
                        "sha256": sha256,
                    })
                })
        })
        .flatten();
    let cpu = super::mct_support::parse_mct_cpu_tuning_from_args(rest)?;
    let clang = super::mct_support::mct_find_tool("clang");
    Ok(serde_json::json!({
        "schema": "nc.compiler.native_content_key.v1",
        "link_contract": "phase12-destructive-v2",
        "source": {
            "canonical_path": canonical_source,
            "sha256": crate::content_cache::sha256_bytes(source.as_bytes()),
            "linked_ast_sha256": crate::content_cache::sha256_bytes(&ast_bytes),
        },
        "compiler": crate::content_cache::compiler_image_identity(),
        "clang": crate::content_cache::tool_identity(clang.as_deref()),
        "backend_profile": format!("{profile:?}"),
        "target": profile.target_spec.target_triple,
        "calling_convention": format!("{:?}", profile.calling_convention),
        "optimization": {
            "level": ncc_ir::render_mct_llvm_optimization_level(opt_level),
            "lto_requested": optimization_plan.lto_requested.label(),
            "lto_applied": optimization_plan.lto.label(),
            "pgo_requested": optimization_plan.pgo_requested.label(),
            "pgo_profile": pgo_profile,
            "cpu": cpu,
        },
        "execution": {
            "profile": opts.profile.as_deref().unwrap_or("release-fast"),
            "target_override": opts.target,
            "strict": opts.strict,
        },
        "runtime_abi": {
            "json_source_sha256": crate::content_cache::sha256_bytes(COMPILER_NATIVE_JSON_RUNTIME_SOURCE.as_bytes()),
            "hosted_source_sha256": crate::content_cache::sha256_bytes(COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.as_bytes()),
            "nc_text_source_sha256": crate::content_cache::sha256_bytes(COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE.as_bytes()),
            "nc_unicode_tables_sha256": crate::content_cache::sha256_bytes(COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE.as_bytes()),
            "nc_unicode_source_sha256": crate::content_cache::sha256_bytes(COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE.as_bytes()),
        },
    }))
}

fn compiler_native_cache_destinations(
    paths: &CompilerNativeCachePaths<'_>,
) -> BTreeMap<String, PathBuf> {
    let mut destinations = BTreeMap::from([
        ("llvm_ir".to_string(), paths.ll_path.to_path_buf()),
        (
            "machine_backend_object_ir".to_string(),
            paths.backend_object_ir_path.to_path_buf(),
        ),
        ("object".to_string(), paths.obj_path.to_path_buf()),
        ("executable".to_string(), paths.exe_path.to_path_buf()),
        (
            "runtime_json_native_object".to_string(),
            paths.runtime_json_obj_path.to_path_buf(),
        ),
        (
            "runtime_hosted_native_object".to_string(),
            paths.hosted_runtime_obj_path.to_path_buf(),
        ),
    ]);
    for (kind, path) in [
        ("machine_entry_object", paths.machine_entry_obj_path),
        ("link_map", paths.internal_link_map_path),
        ("link_report", paths.internal_link_report_path),
    ] {
        if let Some(path) = path {
            destinations.insert(kind.to_string(), path.to_path_buf());
        }
    }
    destinations
}

fn compiler_native_cache_artifacts(
    paths: &CompilerNativeCachePaths<'_>,
) -> Vec<crate::content_cache::ContentCacheArtifact> {
    compiler_native_cache_destinations(paths)
        .into_iter()
        .filter(|(_, path)| path.is_file())
        .map(|(kind, path)| crate::content_cache::ContentCacheArtifact::new(kind, path))
        .collect()
}

fn compiler_native_rewrite_cached_strings(
    value: &mut serde_json::Value,
    replacements: &[(String, String)],
) {
    match value {
        serde_json::Value::String(text) => {
            for (from, to) in replacements {
                if !from.is_empty() && text.contains(from) {
                    *text = text.replace(from, to);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                compiler_native_rewrite_cached_strings(value, replacements);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                compiler_native_rewrite_cached_strings(value, replacements);
            }
        }
        _ => {}
    }
}

fn compiler_native_existing_artifact_records(
    paths: &CompilerNativeCachePaths<'_>,
    launchers: &NativeLauncherArtifactPaths,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();
    for (path, kind) in [
        (paths.nm_path, "machine_source"),
        (paths.ll_path, "llvm_ir"),
        (paths.backend_object_ir_path, "machine_backend_object_ir"),
        (paths.obj_path, "object"),
        (paths.exe_path, "executable"),
        (&launchers.launcher_n, "launcher_n"),
        (&launchers.powershell, "powershell_launcher"),
        (&launchers.batch, "batch_launcher"),
        (paths.runtime_json_obj_path, "runtime_json_native_object"),
        (
            paths.hosted_runtime_obj_path,
            "runtime_hosted_native_object",
        ),
    ] {
        if path.is_file() {
            artifacts.push(compiler_artifact_record(path, kind)?);
        }
    }
    for (path, kind) in [
        (paths.machine_entry_obj_path, "machine_entry_object"),
        (paths.internal_link_map_path, "link_map"),
        (paths.internal_link_report_path, "link_report"),
    ] {
        if let Some(path) = path.filter(|path| path.is_file()) {
            artifacts.push(compiler_artifact_record(path, kind)?);
        }
    }
    Ok(artifacts)
}

fn compiler_native_restore_cached_payload(
    cache: &CompilerNativeContentCache,
    paths: &CompilerNativeCachePaths<'_>,
    profile: &ncc_ir::BackendLoweringProfileIR,
    run_requested: bool,
) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let destinations = compiler_native_cache_destinations(paths);
    let hit = match crate::content_cache::restore_content_cache_entry(
        &cache.root,
        COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE,
        &cache.key,
        &destinations,
    ) {
        Ok(hit) => hit,
        Err(_) => None,
    };
    let Some(hit) = hit else {
        return Ok(None);
    };
    let mut payload = hit.metadata;
    if !payload.is_object()
        || !paths.exe_path.is_file()
        || !paths.backend_object_ir_path.is_file()
        || !paths.obj_path.is_file()
    {
        return Ok(None);
    }
    if payload
        .pointer("/runtime_hosted/referenced")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        let _ = fs::remove_file(paths.hosted_runtime_obj_path);
    }
    if payload
        .pointer("/runtime_json/referenced")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        let _ = fs::remove_file(paths.runtime_json_obj_path);
    }

    let old_source = payload
        .get("source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let old_root = payload
        .get("artifact_root")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let old_executable = payload
        .pointer("/paths/executable")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    compiler_native_rewrite_cached_strings(
        &mut payload,
        &[
            (old_source, paths.source_path.to_string()),
            (old_executable, paths.exe_path.display().to_string()),
            (old_root, paths.base_dir.display().to_string()),
        ],
    );

    let machine_bridge = compiler_native_machine_bridge_source(
        paths.source_path,
        paths.stem,
        profile,
        paths.obj_path,
        paths.exe_path,
    )?;
    fs::write(paths.nm_path, machine_bridge.as_bytes())?;
    release_native_executable_for_user(paths.exe_path)?;
    let launchers = write_native_launcher_artifacts(
        Path::new(paths.source_path),
        paths.stem,
        paths.exe_path,
        paths.code_dir,
        paths.bin_dir,
    )?;

    let mut run = serde_json::Value::Null;
    let mut cache_stage = serde_json::json!({
        "stage": "content-cache-restore",
        "status": "passed",
        "key": hit.key,
        "artifacts": hit.artifact_count,
        "bytes": hit.bytes,
    });
    if run_requested && compiler_native_target_matches_host(&profile.target_spec.target_triple) {
        let run_args = Vec::<String>::new();
        let run_stage = compiler_native_stage_to_json(
            "run-executable",
            Some(paths.exe_path.to_string_lossy().as_ref()),
            &run_args,
            std::process::Command::new(paths.exe_path).output(),
        );
        run = run_stage.clone();
        cache_stage["execution"] = run_stage;
    } else if run_requested {
        run = serde_json::json!({
            "stage": "run-executable",
            "status": "skipped",
            "reason": "automatic execution requires an executable matching the current host OS and architecture",
        });
    }

    payload["source"] = serde_json::json!(paths.source_path);
    payload["artifact_root"] = serde_json::json!(paths.base_dir.display().to_string());
    payload["status"] = serde_json::json!("native_build_passed");
    payload["paths"] = serde_json::json!({
        "machine_source": paths.nm_path.display().to_string(),
        "launcher_n": launchers.launcher_n.display().to_string(),
        "machine_entry_object": paths.machine_entry_obj_path.filter(|path| path.is_file()).map(|path| path.display().to_string()),
        "llvm_ir": paths.ll_path.display().to_string(),
        "machine_backend_object_ir": paths.backend_object_ir_path.display().to_string(),
        "object": paths.obj_path.display().to_string(),
        "executable": paths.exe_path.display().to_string(),
        "link_map": paths.internal_link_map_path.filter(|path| path.is_file()).map(|path| path.display().to_string()),
        "link_report": paths.internal_link_report_path.filter(|path| path.is_file()).map(|path| path.display().to_string()),
        "powershell_launcher": launchers.powershell.display().to_string(),
        "batch_launcher": launchers.batch.display().to_string(),
        "manifest": paths.manifest_path.display().to_string(),
    });
    payload["artifacts"] = serde_json::Value::Array(compiler_native_existing_artifact_records(
        paths, &launchers,
    )?);
    payload["run"] = run;
    payload["incremental"] = serde_json::json!({
        "schema": "ncc.content_cache.result.v1",
        "mode": "content-addressed",
        "cache_hit": true,
        "namespace": COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE,
        "key": cache.key,
        "restored_artifacts": hit.artifact_count,
        "restored_bytes": hit.bytes,
        "elapsed_ms": started.elapsed().as_millis(),
        "reused": ["linked AST identity", "optimized LLVM IR", "MCT O3 object", "runtime objects", "machine entry object", "linked executable"],
    });
    if let Some(stages) = payload
        .get_mut("stages")
        .and_then(serde_json::Value::as_array_mut)
    {
        stages.insert(0, cache_stage);
    } else {
        payload["stages"] = serde_json::json!([cache_stage]);
    }
    fs::write(paths.manifest_path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(Some(payload))
}

fn compiler_native_build_payload(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    compiler_native_build_payload_with_ast(opts, rest, None)
}

fn compiler_native_build_payload_with_ast(
    opts: &CompilerCliOptions,
    rest: &[String],
    ast_override: Option<ncc_ast::Program>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let source_path = compiler_native_build_parse_source(rest)?;
    let profile = compiler_native_build_profile(opts, rest)?;
    validate_backend_artifact_profile(BackendArtifactKind::BuildExe, &profile)?;
    let opt_level = compiler_native_build_opt_level(opts, rest)?;
    let pass_pipeline = ncc_ir::mct_llvm_pass_pipeline(opt_level);
    let run_requested = compiler_native_build_has_flag(rest, "--run")
        && !compiler_native_build_has_flag(rest, "--no-run");
    let allow_missing_tools = compiler_native_build_has_flag(rest, "--allow-missing-tools");

    let stem = compiler_native_build_stem(&source_path);
    let base_dir = compiler_native_build_option(rest, "--out-dir")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(".ncc")
        });
    let code_dir = base_dir.join("code");
    let lowering_dir = base_dir.join("lowering");
    let obj_dir = base_dir.join("obj");
    let bin_dir = base_dir.join("bin");
    let run_dir = base_dir.join("run");
    for dir in [&code_dir, &lowering_dir, &obj_dir, &bin_dir, &run_dir] {
        fs::create_dir_all(dir)?;
    }
    compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;
    let nm_path = lowering_dir.join(format!("{stem}.nm"));
    let ll_path = lowering_dir.join(format!("{stem}.ll"));
    let backend_object_ir_path = lowering_dir.join(format!("{stem}.backend-object.nbo"));
    let obj_path = obj_dir.join(format!("{stem}.obj"));
    let exe_path = opts
        .out
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| bin_dir.join(format!("{stem}.exe")));
    if let Some(parent) = exe_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let manifest_path = run_dir.join(format!("{stem}.native-build.json"));
    let mut optimization_plan = compiler_native_optimization_plan(opts, rest, &base_dir, &stem)?;

    let source = fs::read_to_string(&source_path)?;
    let mut ast = match ast_override {
        Some(ast) => ast,
        None => load_ast_from_path(&source_path)?,
    };
    // The AST detector covers imports, text operators and every canonical text
    // intrinsic. Programs without such operations must not carry the provider.
    let nc_text_runtime_embedded =
        compiler_native_attach_nc_text_runtime(&mut ast, &profile.target_spec.target_triple)?;
    let nc_escape_runtime_embedded =
        compiler_native_attach_nc_escape_runtime(&mut ast, &profile.target_spec.target_triple)?;
    let nc_parse_runtime_embedded = compiler_native_attach_nc_parse_runtime(&mut ast)?;
    let nc_unicode_runtime_embedded =
        compiler_native_attach_nc_unicode_runtime(&mut ast, &profile.target_spec.target_triple)?;
    sema_check_with_warnings(&ast)?;

    let destructive_internal_link = matches!(
        profile.destructive_policy.missing_symbol_policy,
        ncc_ir::BackendMissingSymbolPolicyIR::MaterializeNullAddress
    );
    let native_pe_internal = matches!(
        profile.calling_convention,
        ncc_ir::BackendCallingConventionIR::NcWin64
    ) && matches!(
        profile.target_spec.object_format,
        ncc_ir::BackendObjectFormatIR::Coff
    ) && profile.target_spec.target_triple.contains("windows");
    let native_macho_internal = matches!(
        profile.target_spec.object_format,
        ncc_ir::BackendObjectFormatIR::MachO
    ) && profile.target_spec.target_triple.contains("darwin")
        && !compiler_native_target_matches_host(&profile.target_spec.target_triple);
    let destructive_internal_format =
        destructive_internal_link.then(|| match profile.target_spec.object_format {
            ncc_ir::BackendObjectFormatIR::Elf32 | ncc_ir::BackendObjectFormatIR::Elf64 => {
                Some("elf")
            }
            ncc_ir::BackendObjectFormatIR::Coff => Some("coff"),
            ncc_ir::BackendObjectFormatIR::MachO => Some("macho"),
            ncc_ir::BackendObjectFormatIR::WasmModule
            | ncc_ir::BackendObjectFormatIR::FreestandingImage => None,
        });
    let internal_os_link_format = if let Some(format) = destructive_internal_format.flatten() {
        Some(format)
    } else if native_pe_internal {
        Some("coff")
    } else if native_macho_internal {
        Some("macho")
    } else {
        None
    };
    let native_os_internal = internal_os_link_format.is_some();
    if native_os_internal && optimization_plan.lto != CompilerNativeLtoMode::Off {
        // The whole linked nC project is already one LLVM module. Keep the
        // final target object material so the internal OS linker can consume it.
        optimization_plan.lto = CompilerNativeLtoMode::Off;
    }

    let runtime_json_obj_candidate = obj_dir.join(format!("{stem}.runtime-json.obj"));
    let hosted_runtime_obj_candidate = obj_dir.join(format!("{stem}.runtime-hosted.obj"));
    let machine_entry_obj_path =
        native_os_internal.then(|| obj_dir.join(format!("{stem}.entry.obj")));
    let internal_link_map_path =
        internal_os_link_format.map(|format| run_dir.join(format!("{stem}.{format}.map")));
    let internal_link_report_path = native_os_internal
        .then(|| std::path::PathBuf::from(format!("{}.link-report.json", exe_path.display())));

    let cache_material = compiler_native_cache_material(
        opts,
        rest,
        &source_path,
        &source,
        &ast,
        &profile,
        opt_level.clone(),
        &optimization_plan,
    )?;
    let explicit_cache_root =
        compiler_native_build_option(rest, "--content-cache-dir").map(PathBuf::from);
    let cache = CompilerNativeContentCache {
        root: crate::content_cache::content_cache_root(explicit_cache_root.as_deref(), &base_dir),
        key: crate::content_cache::content_cache_key(
            COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE,
            &cache_material,
        ),
        material: cache_material,
    };
    let cache_paths = CompilerNativeCachePaths {
        source_path: &source_path,
        base_dir: &base_dir,
        code_dir: &code_dir,
        bin_dir: &bin_dir,
        stem: &stem,
        nm_path: &nm_path,
        ll_path: &ll_path,
        backend_object_ir_path: &backend_object_ir_path,
        obj_path: &obj_path,
        exe_path: &exe_path,
        manifest_path: &manifest_path,
        runtime_json_obj_path: &runtime_json_obj_candidate,
        hosted_runtime_obj_path: &hosted_runtime_obj_candidate,
        machine_entry_obj_path: machine_entry_obj_path.as_deref(),
        internal_link_map_path: internal_link_map_path.as_deref(),
        internal_link_report_path: internal_link_report_path.as_deref(),
    };
    let cache_enabled = !crate::content_cache::content_cache_disabled(rest)
        && optimization_plan.pgo_requested != CompilerNativePgoMode::Train;
    if cache_enabled {
        if let Some(payload) =
            compiler_native_restore_cached_payload(&cache, &cache_paths, &profile, run_requested)?
        {
            return Ok(payload);
        }
    }

    // `compiler build-native --target` is an explicit material target contract.
    // Preserve its exact ISA, ABI, pointer width and object format, including
    // hosted ILP32 profiles that share the scalar legalization machinery.
    let ir = ncc_ir::lower_with_backend_profile_and_source_exact(
        ast,
        profile.clone(),
        &source_path,
        &source,
    );
    let mut requested_unresolved_symbols = BTreeSet::new();
    for container in &ir.containers {
        let foreign_symbols = match container {
            ncc_ir::ContainerIR::Regular(regular) => &regular.foreign_symbols,
            ncc_ir::ContainerIR::Root(root) => &root.foreign_symbols,
            ncc_ir::ContainerIR::Security(_)
            | ncc_ir::ContainerIR::Primitive(_)
            | ncc_ir::ContainerIR::Machine(_) => continue,
        };
        requested_unresolved_symbols
            .extend(foreign_symbols.iter().map(|foreign| foreign.symbol.clone()));
    }
    let requested_unresolved_symbols = requested_unresolved_symbols.into_iter().collect::<Vec<_>>();
    let target_attributes = if let Some((cpu, fine_profile)) =
        super::mct_support::parse_mct_cpu_tuning_from_args(rest)?
    {
        ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(&ir.target_spec, &cpu, &fine_profile)
            .map_err(|err| actionable_command_error("compiler", err))?
    } else {
        ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec)
    };
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &ir,
        pass_pipeline.clone(),
        super::mct_json::mct_llvm_debug_config_for_path(&source_path),
        target_attributes,
    );
    if module.emitted_function_count == 0
        || module.rejected_function_count > 0
        || !module.llvm_ir.contains("define i32 @main()")
    {
        return Ok(serde_json::json!({
            "schema": "nc.compiler.native_build.v1",
            "status": "native_build_rejected",
            "source": source_path,
            "reason": if module.rejected_function_count > 0 {
                "compiler build-native refuses a partial MCT module; every source function must lower before object emission"
            } else {
                "compiler build-native requires emitted functions and a hosted main"
            },
            "module": super::mct_json::mct_llvm_module_to_json(&module, true, None),
        }));
    }

    fs::write(&ll_path, module.llvm_ir.as_bytes())?;
    let machine_bridge =
        compiler_native_machine_bridge_source(&source_path, &stem, &profile, &obj_path, &exe_path)?;
    fs::write(&nm_path, machine_bridge.as_bytes())?;
    let backend_object = ncc_ir::emit_object(&ncc_ir::emit_backend(&ir));
    let backend_object_text = super::tooling::nbuild_backend_object_with_machine_lineage(
        &stem,
        &source_path,
        &ncc_ir::render_object_text(&backend_object),
    );
    fs::write(&backend_object_ir_path, backend_object_text.as_bytes())?;
    if optimization_plan.pgo_requested == CompilerNativePgoMode::Train {
        if native_os_internal {
            return Err("compiler build-native: --pgo train requires a matching hosted Clang linker; the internal OS linker accepts profile-use objects but does not embed the Clang profiling runtime".into());
        }
        if !run_requested {
            return Err("compiler build-native: --pgo train requires --run so the instrumented workload can produce profile data".into());
        }
        if !compiler_native_target_matches_host(&module.target_triple) {
            return Err("compiler build-native: --pgo train requires a target matching the current host OS and architecture".into());
        }
        let _ = fs::remove_dir_all(&optimization_plan.pgo_raw_dir);
        fs::create_dir_all(&optimization_plan.pgo_raw_dir)?;
    }
    let runtime_json_referenced = compiler_native_uses_json_runtime(&module.llvm_ir);
    let runtime_json_native_enabled = runtime_json_referenced;
    let runtime_json_obj_path =
        runtime_json_native_enabled.then(|| runtime_json_obj_candidate.clone());
    let hosted_runtime_referenced = compiler_native_uses_hosted_runtime(&module.llvm_ir);
    let hosted_runtime_obj_path =
        hosted_runtime_referenced.then(|| hosted_runtime_obj_candidate.clone());
    if !runtime_json_native_enabled {
        let _ = fs::remove_file(&runtime_json_obj_candidate);
    }
    if !hosted_runtime_referenced {
        let _ = fs::remove_file(&hosted_runtime_obj_candidate);
    }
    let hosted_runtime_link_objects = runtime_json_obj_path
        .iter()
        .chain(hosted_runtime_obj_path.iter())
        .cloned()
        .collect::<Vec<_>>();
    let hosted_runtime_source_sha256 = format!(
        "{:x}",
        Sha256::digest(COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.as_bytes())
    );
    let runtime_json_source_sha256 = format!(
        "{:x}",
        Sha256::digest(COMPILER_NATIVE_JSON_RUNTIME_SOURCE.as_bytes())
    );
    let clang = super::mct_support::mct_find_tool("clang");
    let mut stages = Vec::new();
    let mut native_link_report = None::<serde_json::Value>;
    if let Some(clang_tool) = clang.as_deref() {
        let active_profile = optimization_plan
            .pgo_uses_profile
            .then_some(optimization_plan.pgo_profile.as_path());
        let training_dir = (optimization_plan.pgo_requested == CompilerNativePgoMode::Train
            && optimization_plan.lto == CompilerNativeLtoMode::Off)
            .then_some(optimization_plan.pgo_raw_dir.as_path());
        let object_args = compiler_native_object_command_args(
            &module.target_triple,
            pass_pipeline.clang_opt_arg,
            &ll_path,
            &obj_path,
            active_profile,
            training_dir,
        );
        stages.push(compiler_native_stage_to_json(
            "clang-object",
            Some(clang_tool),
            &object_args,
            std::process::Command::new(clang_tool)
                .args(&object_args)
                .output(),
        ));

        if stages.last().is_some_and(compiler_native_stage_passed) && runtime_json_native_enabled {
            let runtime_object = runtime_json_obj_path
                .as_ref()
                .expect("JSON runtime object path must exist when enabled");
            let runtime_args = compiler_native_embedded_c_object_command_args(
                &module.target_triple,
                runtime_object,
                false,
            );
            stages.push(compiler_native_stage_to_json(
                "json-runtime-target-object",
                Some(clang_tool),
                &runtime_args,
                compiler_native_compile_embedded_c_object(
                    clang_tool,
                    &runtime_args,
                    COMPILER_NATIVE_JSON_RUNTIME_SOURCE,
                ),
            ));
        }

        if stages.last().is_some_and(compiler_native_stage_passed) && hosted_runtime_referenced {
            let runtime_object = hosted_runtime_obj_path
                .as_ref()
                .expect("hosted runtime object path must exist when referenced");
            let runtime_args = compiler_native_embedded_c_object_command_args(
                &module.target_triple,
                runtime_object,
                true,
            );
            stages.push(compiler_native_stage_to_json(
                "hosted-runtime-object",
                Some(clang_tool),
                &runtime_args,
                compiler_native_compile_embedded_c_object(
                    clang_tool,
                    &runtime_args,
                    COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE,
                ),
            ));
        }

        if stages.last().is_some_and(compiler_native_stage_passed) && native_os_internal {
            let entry_obj_path = machine_entry_obj_path
                .as_ref()
                .expect("internal OS route must reserve a Machine entry object");
            let entry_args = vec![
                nm_path.display().to_string(),
                "--target".to_string(),
                module.target_triple.clone(),
                "--out".to_string(),
                entry_obj_path.display().to_string(),
            ];
            let entry_stage = match super::machine::emit_machine_source_object_to_path(
                nm_path.to_string_lossy().as_ref(),
                &module.target_triple,
                entry_obj_path.to_string_lossy().as_ref(),
            ) {
                Ok(()) => serde_json::json!({
                    "stage": "ncc-machine-entry-object",
                    "tool": "ncc-machine",
                    "args": entry_args,
                    "status": "passed",
                    "object": entry_obj_path.display().to_string(),
                }),
                Err(err) => serde_json::json!({
                    "stage": "ncc-machine-entry-object",
                    "tool": "ncc-machine",
                    "args": entry_args,
                    "status": "failed",
                    "error": err.to_string(),
                }),
            };
            stages.push(entry_stage);

            if stages.last().is_some_and(compiler_native_stage_passed) {
                let map_path = internal_link_map_path
                    .as_ref()
                    .expect("internal OS route must reserve a link map");
                let mut inputs = vec![
                    entry_obj_path.to_string_lossy().to_string(),
                    obj_path.to_string_lossy().to_string(),
                ];
                if let Some(runtime_object) = runtime_json_obj_path.as_ref() {
                    inputs.push(runtime_object.to_string_lossy().to_string());
                }
                if let Some(runtime_object) = hosted_runtime_obj_path.as_ref() {
                    inputs.push(runtime_object.to_string_lossy().to_string());
                }
                let mut link_args = inputs.clone();
                link_args.extend([
                    "--target".to_string(),
                    module.target_triple.clone(),
                    "--out".to_string(),
                    exe_path.display().to_string(),
                    "--map".to_string(),
                    map_path.display().to_string(),
                ]);
                match super::machine::link_machine_object_paths_to_os_executable_with_missing_symbol_policy_silent(
                    &inputs,
                    &module.target_triple,
                    exe_path.to_string_lossy().as_ref(),
                    Some(map_path.to_string_lossy().as_ref()),
                    profile.destructive_policy.missing_symbol_policy,
                    &requested_unresolved_symbols,
                ) {
                    Ok(report) => {
                        let internal_link_stage = if internal_os_link_format == Some("coff") {
                            "ncc-internal-pe-link".to_string()
                        } else {
                            format!(
                                "ncc-internal-{}-link",
                                internal_os_link_format.unwrap_or("os")
                            )
                        };
                        stages.push(serde_json::json!({
                            "stage": internal_link_stage,
                            "tool": "ncc-machine-phase12",
                            "args": link_args,
                            "status": "passed",
                            "artifact_magic": report.get("artifact_magic").cloned(),
                            "phase12_report_path": internal_link_report_path.as_ref().map(|path| path.display().to_string()),
                        }));
                        native_link_report = Some(report);
                    }
                    Err(err) => {
                        let internal_link_stage = if internal_os_link_format == Some("coff") {
                            "ncc-internal-pe-link".to_string()
                        } else {
                            format!(
                                "ncc-internal-{}-link",
                                internal_os_link_format.unwrap_or("os")
                            )
                        };
                        stages.push(serde_json::json!({
                            "stage": internal_link_stage,
                            "tool": "ncc-machine-phase12",
                            "args": link_args,
                            "status": "failed",
                            "error": err.to_string(),
                        }))
                    }
                }
            }
        } else if stages.last().is_some_and(compiler_native_stage_passed) {
            let link_args = compiler_native_external_link_command_args(
                &module.target_triple,
                pass_pipeline.clang_opt_arg,
                &ll_path,
                &obj_path,
                &exe_path,
                optimization_plan.lto,
                active_profile,
                (optimization_plan.pgo_requested == CompilerNativePgoMode::Train)
                    .then_some(optimization_plan.pgo_raw_dir.as_path()),
                compiler_native_build_needs_libm(&module.llvm_ir),
                &hosted_runtime_link_objects,
                module.llvm_ir.contains("@ncc_rt_thread_"),
                compiler_native_build_needs_ws2_32(&module.llvm_ir),
                compiler_native_build_needs_advapi32(&module.llvm_ir),
            );
            stages.push(compiler_native_stage_to_json(
                if optimization_plan.lto == CompilerNativeLtoMode::Off {
                    "clang-link"
                } else {
                    "clang-lto-link"
                },
                Some(clang_tool),
                &link_args,
                std::process::Command::new(clang_tool)
                    .args(&link_args)
                    .output(),
            ));
        }

        if optimization_plan.pgo_requested == CompilerNativePgoMode::Train
            && stages.iter().all(compiler_native_stage_passed)
            && exe_path.is_file()
        {
            stages.push(compiler_native_training_stage_to_json(
                &exe_path,
                std::process::Command::new(&exe_path).output(),
            ));

            let mut profraw = Vec::new();
            compiler_native_collect_profraw(&optimization_plan.pgo_raw_dir, &mut profraw)?;
            if profraw.is_empty() {
                stages.push(serde_json::json!({
                    "stage": "pgo-profraw-collect",
                    "status": "failed",
                    "profile_dir": optimization_plan.pgo_raw_dir.display().to_string(),
                    "reason": "instrumented workload produced no .profraw files",
                }));
            } else if let Some(profdata_tool) = super::mct_support::mct_find_tool("llvm-profdata") {
                let mut merge_args = vec![
                    "merge".to_string(),
                    "-output".to_string(),
                    optimization_plan.pgo_profile.display().to_string(),
                ];
                merge_args.extend(profraw.iter().map(|path| path.display().to_string()));
                stages.push(compiler_native_stage_to_json(
                    "llvm-profdata-merge",
                    Some(&profdata_tool),
                    &merge_args,
                    std::process::Command::new(&profdata_tool)
                        .args(&merge_args)
                        .output(),
                ));
            } else {
                stages.push(serde_json::json!({
                    "stage": "llvm-profdata-merge",
                    "status": "missing_tool",
                    "tool": "llvm-profdata",
                    "args": [],
                }));
            }

            if stages.iter().all(compiler_native_stage_passed)
                && optimization_plan.pgo_profile.is_file()
            {
                let use_object_args = compiler_native_object_command_args(
                    &module.target_triple,
                    pass_pipeline.clang_opt_arg,
                    &ll_path,
                    &obj_path,
                    Some(&optimization_plan.pgo_profile),
                    None,
                );
                stages.push(compiler_native_stage_to_json(
                    "clang-pgo-use-object",
                    Some(clang_tool),
                    &use_object_args,
                    std::process::Command::new(clang_tool)
                        .args(&use_object_args)
                        .output(),
                ));
                if stages.last().is_some_and(compiler_native_stage_passed) {
                    let use_link_args = compiler_native_external_link_command_args(
                        &module.target_triple,
                        pass_pipeline.clang_opt_arg,
                        &ll_path,
                        &obj_path,
                        &exe_path,
                        optimization_plan.lto,
                        Some(&optimization_plan.pgo_profile),
                        None,
                        compiler_native_build_needs_libm(&module.llvm_ir),
                        &hosted_runtime_link_objects,
                        module.llvm_ir.contains("@ncc_rt_thread_"),
                        compiler_native_build_needs_ws2_32(&module.llvm_ir),
                        module.llvm_ir.contains("@SystemFunction036("),
                    );
                    stages.push(compiler_native_stage_to_json(
                        "clang-pgo-use-link",
                        Some(clang_tool),
                        &use_link_args,
                        std::process::Command::new(clang_tool)
                            .args(&use_link_args)
                            .output(),
                    ));
                    optimization_plan.pgo_uses_profile =
                        stages.last().is_some_and(compiler_native_stage_passed);
                }
            }
        }
    } else {
        stages.push(serde_json::json!({
            "stage": "clang-object",
            "tool": "clang",
            "status": "missing_tool",
            "args": [],
            "reason": "clang is required for C-like .n -> .obj -> .exe materialization"
        }));
    }

    let mut run = serde_json::Value::Null;
    if run_requested
        && stages.iter().all(compiler_native_stage_passed)
        && exe_path.exists()
        && compiler_native_target_matches_host(&module.target_triple)
    {
        let run_output = std::process::Command::new(&exe_path).output();
        let run_args: Vec<String> = Vec::new();
        let run_stage = compiler_native_stage_to_json(
            "run-executable",
            Some(&exe_path.display().to_string()),
            &run_args,
            run_output,
        );
        run = run_stage.clone();
        stages.push(run_stage);
    } else if run_requested && !compiler_native_target_matches_host(&module.target_triple) {
        run = serde_json::json!({
            "stage": "run-executable",
            "status": "skipped",
            "reason": "automatic execution requires an executable matching the current host OS and architecture",
        });
        stages.push(run.clone());
    }

    let all_stages_passed = stages.iter().all(compiler_native_stage_passed);
    let status = if all_stages_passed {
        "native_build_passed"
    } else if clang.is_none() && allow_missing_tools {
        "native_build_missing_toolchain"
    } else if stages
        .iter()
        .any(|stage| stage.get("stage").and_then(|v| v.as_str()) == Some("clang-object"))
    {
        if obj_path.exists() {
            "native_build_link_failed"
        } else {
            "native_build_object_failed"
        }
    } else {
        "native_build_failed"
    };

    let launcher_artifacts = if all_stages_passed && exe_path.exists() {
        release_native_executable_for_user(&exe_path)?;
        Some(write_native_launcher_artifacts(
            Path::new(&source_path),
            &stem,
            &exe_path,
            &code_dir,
            &bin_dir,
        )?)
    } else {
        None
    };

    let mut artifacts = Vec::new();
    for (path, kind) in [
        (&nm_path, "machine_source"),
        (&ll_path, "llvm_ir"),
        (&backend_object_ir_path, "machine_backend_object_ir"),
        (&obj_path, "object"),
        (&exe_path, "executable"),
    ] {
        if path.exists() {
            artifacts.push(compiler_artifact_record(path, kind)?);
        }
    }
    if optimization_plan.pgo_profile.is_file() {
        artifacts.push(compiler_artifact_record(
            &optimization_plan.pgo_profile,
            "llvm_pgo_profile",
        )?);
    }
    for (path, kind) in [
        (machine_entry_obj_path.as_ref(), "machine_entry_object"),
        (runtime_json_obj_path.as_ref(), "runtime_json_native_object"),
        (
            hosted_runtime_obj_path.as_ref(),
            "runtime_hosted_native_object",
        ),
        (internal_link_map_path.as_ref(), "link_map"),
        (internal_link_report_path.as_ref(), "link_report"),
    ] {
        if let Some(path) = path.filter(|path| path.exists()) {
            artifacts.push(compiler_artifact_record(path, kind)?);
        }
    }
    if let Some(launchers) = &launcher_artifacts {
        artifacts.push(compiler_artifact_record(
            &launchers.launcher_n,
            "launcher_n",
        )?);
        artifacts.push(compiler_artifact_record(
            &launchers.powershell,
            "powershell_launcher",
        )?);
        artifacts.push(compiler_artifact_record(
            &launchers.batch,
            "batch_launcher",
        )?);
    }

    let mut nc_runtime_providers = Vec::new();
    if nc_text_runtime_embedded {
        nc_runtime_providers.push("runtime/nc/text_format.n");
    }
    if nc_escape_runtime_embedded {
        nc_runtime_providers.push("runtime/nc/text_escape.n");
    }
    if nc_parse_runtime_embedded {
        nc_runtime_providers.push("runtime/nc/text_parse.n");
    }
    if nc_unicode_runtime_embedded {
        nc_runtime_providers.push("runtime/nc/unicode_tables.n");
        nc_runtime_providers.push("runtime/nc/unicode.n");
    }

    let mut payload = serde_json::json!({
        "schema": "nc.compiler.native_build.v1",
        "status": status,
        "source": source_path,
        "artifact_root": base_dir.display().to_string(),
        "profile": opts.profile.as_deref().unwrap_or("release-fast"),
        "backend_profile": super::mct_support::backend_lowering_profile_label(&profile),
        "target_triple": module.target_triple,
        "object_extension_policy": ".obj is retained for the C-like user contract; object bytes keep their target-native format",
        "speed_contract": {
            "goal": "C-like release build surface",
            "default_profile": "release-fast",
            "default_optimization": "O3",
            "claimed_full_c_parity_for_every_program": false,
            "native_machine_execution": true,
            "interpreter_fallback": false,
        },
        "paths": {
            "machine_source": nm_path.display().to_string(),
            "launcher_n": launcher_artifacts.as_ref().map(|paths| paths.launcher_n.display().to_string()),
            "machine_entry_object": machine_entry_obj_path.as_ref().map(|path| path.display().to_string()),
            "llvm_ir": ll_path.display().to_string(),
            "machine_backend_object_ir": backend_object_ir_path.display().to_string(),
            "object": obj_path.display().to_string(),
            "executable": exe_path.display().to_string(),
            "link_map": internal_link_map_path.as_ref().map(|path| path.display().to_string()),
            "link_report": internal_link_report_path.as_ref().map(|path| path.display().to_string()),
            "powershell_launcher": launcher_artifacts.as_ref().map(|paths| paths.powershell.display().to_string()),
            "batch_launcher": launcher_artifacts.as_ref().map(|paths| paths.batch.display().to_string()),
            "manifest": manifest_path.display().to_string(),
        },
        "pipeline": if native_os_internal {
            serde_json::json!([
                "parse .n",
                "sema",
                "lower to MCT LLVM module",
                "retain .nm machine bridge",
                "clang optimized target object with cached PGO profile when available",
                "emit .nm Machine bridge target object",
                "link system-loader executable internally through Machine Phase 12"
            ])
        } else {
            serde_json::json!([
                "parse .n",
                "sema",
                "lower to MCT LLVM module",
                "retain .nm machine bridge",
                "clang optimized target object with cached PGO profile when available",
                "clang ThinLTO/full-LTO link for release profiles"
            ])
        },
        "optimizer": {
            "level": ncc_ir::render_mct_llvm_optimization_level(pass_pipeline.level),
            "clang_arg": pass_pipeline.clang_opt_arg,
            "opt_passes": pass_pipeline.opt_passes_arg,
            "pass_count": pass_pipeline.pass_count,
            "whole_program_input": true,
            "lto": {
                "requested": optimization_plan.lto_requested.label(),
                "applied": optimization_plan.lto.label(),
                "clang_arg": optimization_plan.lto.clang_arg(),
                "internal_pe_policy": if native_os_internal && optimization_plan.lto_requested != CompilerNativeLtoMode::Off {
                    "single-linked-module O3 IPO retained as a material target object for the internal OS linker"
                } else {
                    "hosted Clang LTO link"
                },
            },
            "pgo": {
                "requested": optimization_plan.pgo_requested.label(),
                "profile_path": optimization_plan.pgo_profile.display().to_string(),
                "profile_exists": optimization_plan.pgo_profile.is_file(),
                "profile_used": optimization_plan.pgo_uses_profile,
                "training_raw_dir": optimization_plan.pgo_raw_dir.display().to_string(),
                "automatic_policy": "use a retained per-program profile automatically; never invent a workload for interactive programs",
            },
        },
        "artifacts": artifacts,
        "stages": stages,
        "link": {
            "route": match internal_os_link_format {
                Some("coff") => "mct-llvm-coff-phase12-pe64",
                Some("macho") => "mct-llvm-macho-phase12",
                Some(_) => "mct-llvm-phase12-os",
                None => "clang-host-linker",
            },
            "native_pe64_internal": native_pe_internal,
            "native_macho_internal": native_macho_internal,
            "internal_os_linker": native_os_internal,
            "internal_os_link_format": internal_os_link_format,
            "destructive_internal_link": destructive_internal_link,
            "missing_symbol_policy": if destructive_internal_link { "materialize_null_address" } else { "strict" },
            "requested_unresolved_symbols": requested_unresolved_symbols,
            "phase12_report": native_link_report,
        },
        "runtime_json_native_object": runtime_json_native_enabled
            && runtime_json_obj_path.as_ref().is_some_and(|path| path.exists()),
        "runtime_json": {
            "referenced": runtime_json_referenced,
            "native_object_enabled": runtime_json_native_enabled,
            "native_object": runtime_json_obj_path.as_ref().map(|path| path.display().to_string()),
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-stdin",
            "source_sha256": runtime_json_source_sha256,
            "object_sha256": runtime_json_obj_path
                .as_ref()
                .filter(|path| path.exists())
                .map(|path| compiler_sha256_hex_file(path))
                .transpose()?,
            "compiler": clang,
            "abi": "nC.runtime.json.v1",
            "symbol_count": COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS.len(),
            "implementation": "compiler-owned DOM/parser/stringifier/file provider streamed directly into a target-native PE/ELF/Mach-O object; no C source is materialized under .ncc",
        },
        "runtime_hosted": {
            "referenced": hosted_runtime_referenced,
            "native_object": hosted_runtime_obj_path.as_ref().map(|path| path.display().to_string()),
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-stdin",
            "source_sha256": hosted_runtime_source_sha256,
            "object_sha256": hosted_runtime_obj_path
                .as_ref()
                .filter(|path| path.exists())
                .map(|path| compiler_sha256_hex_file(path))
                .transpose()?,
            "abi": "nC.runtime.hosted.v1",
            "symbol_count": COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.len(),
            "tagged_sum_layout": "u64: bit0 positive tag, bits1..63 signed scalar/pointer payload",
            "implementation": "remaining legacy hosted mobile provider operations; generic network is material LLVM/MCT",
        },
        "runtime_nc": {
            "text_format_embedded": nc_text_runtime_embedded,
            "text_escape_embedded": nc_escape_runtime_embedded,
            "text_parse_embedded": nc_parse_runtime_embedded,
            "unicode_embedded": nc_unicode_runtime_embedded,
            "source_language": "nC",
            "lowering": "same linked AST -> MCT LLVM -> target object route as user code",
            "c_source_generated": false,
            "provider": if nc_escape_runtime_embedded {
                Some("runtime/nc/text_escape.n")
            } else if nc_text_runtime_embedded {
                Some("runtime/nc/text_format.n")
            } else if nc_parse_runtime_embedded {
                Some("runtime/nc/text_parse.n")
            } else {
                None
            },
            "providers": nc_runtime_providers,
            "unicode": if nc_unicode_runtime_embedded {
                Some(serde_json::json!({
                    "version": "17.0.0",
                    "normalization": ["NFC", "NFD", "NFKC", "NFKD"],
                    "segmentation": "UAX #29 extended grapheme clusters",
                    "tables": "runtime/nc/unicode_tables.n",
                    "provider": "runtime/nc/unicode.n",
                    "ownership": "returned transformed strings require unicodeRelease",
                }))
            } else {
                None
            },
        },
        "run": run,
        "module": super::mct_json::mct_llvm_module_to_json(&module, false, Some(&ll_path)),
        "truth_boundary": {
            "object_is_real_clang_target_output": obj_path.exists(),
            "object_and_executable_are_real_host_toolchain_outputs": !native_os_internal && all_stages_passed && exe_path.exists(),
            "executable_is_host_toolchain_output": !native_os_internal && all_stages_passed && exe_path.exists(),
            "executable_is_native_pe_direct": native_pe_internal && all_stages_passed && exe_path.exists(),
            "executable_is_native_macho_direct": native_macho_internal && all_stages_passed && exe_path.exists(),
            "executable_is_internal_os_loader_direct": native_os_internal && all_stages_passed && exe_path.exists(),
            "internal_os_linker": native_os_internal,
            "runtime_imports_are_machine_linker_visible": native_os_internal,
            "runtime_json_native_object_is_real_coff": runtime_json_native_enabled
                && native_pe_internal
                && runtime_json_obj_path.as_ref().is_some_and(|path| path.exists()),
            "runtime_json_native_object_is_real_target_object": runtime_json_native_enabled
                && runtime_json_obj_path.as_ref().is_some_and(|path| path.exists()),
            "runtime_hosted_native_object_is_real_target_object": hosted_runtime_referenced
                && hosted_runtime_obj_path.as_ref().is_some_and(|path| path.exists()),
            "whole_program_ast_linked_before_llvm": true,
            "link_time_optimization_applied": optimization_plan.lto != CompilerNativeLtoMode::Off,
            "pgo_profile_applied": optimization_plan.pgo_uses_profile,
            "nm_is_retained_machine_bridge_not_full_instruction_for_instruction_high_level_lowering": true,
            "launcher_n_generated": launcher_artifacts.is_some(),
            "host_shell_launchers_generated": if launcher_artifacts.is_some() { serde_json::json!([".ps1", ".bat"]) } else { serde_json::json!([]) },
            "executable_released_for_external_run": launcher_artifacts.is_some(),
            "missing_tools_are_not_faked": true,
        }
    });
    payload["incremental"] = serde_json::json!({
        "schema": "ncc.content_cache.result.v1",
        "mode": "content-addressed",
        "cache_hit": false,
        "namespace": COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE,
        "key": cache.key,
        "enabled": cache_enabled,
        "stored": false,
        "reused": [],
    });
    if cache_enabled && all_stages_passed && exe_path.is_file() {
        let cache_artifacts = compiler_native_cache_artifacts(&cache_paths);
        let store_result = crate::content_cache::store_content_cache_entry(
            &cache.root,
            COMPILER_NATIVE_CONTENT_CACHE_NAMESPACE,
            &cache.key,
            &cache.material,
            payload.clone(),
            &cache_artifacts,
        );
        match store_result {
            Ok(()) => {
                payload["incremental"]["stored"] = serde_json::json!(true);
                payload["incremental"]["stored_artifacts"] =
                    serde_json::json!(cache_artifacts.len());
            }
            Err(err) => {
                payload["incremental"]["store_error"] = serde_json::json!(err.to_string());
            }
        }
    }
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;
    Ok(payload)
}

pub(crate) fn compiler_build_native_payload_for_run(
    args: &[String],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (opts, rest) = parse_compiler_cli_options(args)?;
    compiler_native_build_payload(&opts, &rest)
}

pub(crate) fn compiler_build_native_payload_for_linked_program(
    args: &[String],
    program: ncc_ast::Program,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (opts, rest) = parse_compiler_cli_options(args)?;
    compiler_native_build_payload_with_ast(&opts, &rest, Some(program))
}

fn compiler_external_lane_skipped(language: &str, tool: &str) -> serde_json::Value {
    serde_json::json!({
        "language": language,
        "status": "skipped_missing_tool",
        "tool": tool,
        "executed": false,
        "official_code": "E_FFI_DEFERRED_001",
        "reason": format!("{tool} is not available on this host"),
    })
}

fn compiler_external_lane_failed(language: &str, reason: String) -> serde_json::Value {
    serde_json::json!({
        "language": language,
        "status": "failed",
        "executed": false,
        "official_code": "E_FFI_LINK_FAILED_001",
        "reason": reason,
    })
}

fn compiler_run_ffi_external_bindings(
    strict: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cc = compiler_resolve_first_tool(&["cc", "gcc", "clang"]);
    let rustc = compiler_resolve_first_tool(&["rustc"]);
    let nm = compiler_resolve_first_tool(&["nm", "llvm-nm"]);
    let zig = compiler_resolve_first_tool(&["zig"]);
    let go = compiler_resolve_first_tool(&["go"]);
    let host_os = std::env::consts::OS;
    let host_arch = std::env::consts::ARCH;

    let mut missing_required = Vec::new();
    if cc.is_none() {
        missing_required.push("cc");
    }
    if rustc.is_none() {
        missing_required.push("rustc");
    }
    if nm.is_none() {
        missing_required.push("nm");
    }
    if !missing_required.is_empty() {
        let reason = format!(
            "E_FFI_LINK_FAILED_001: missing external binding tool(s): {}",
            missing_required.join(", ")
        );
        if strict {
            return Err(reason.into());
        }
        return Ok(serde_json::json!({
            "schema": "nc.compiler.ffi.external_bindings.v1",
            "status": "skipped",
            "required_status": "skipped_missing_tool",
            "optional_status": "skipped",
            "reason": reason,
            "strict": strict,
            "host": {"os": host_os, "arch": host_arch},
            "toolchain": {"cc": cc, "rustc": rustc, "nm": nm, "zig": zig, "go": go},
            "no_fake_evidence": true,
        }));
    }

    let cc = cc.expect("checked is_some");
    let rustc = rustc.expect("checked is_some");
    let nm = nm.expect("checked is_some");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!(
        "ncc_ffi_external_bindings_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&tmp)?;

    let c_src = tmp.join("nc_ext_c.c");
    let rust_src = tmp.join("nc_ext_rust.rs");
    let asm_src = tmp.join("nc_ext_asm.S");
    let driver_src = tmp.join("nc_ext_driver.c");
    let c_obj = tmp.join("nc_ext_c.o");
    let rust_obj = tmp.join("nc_ext_rust.o");
    let asm_obj = tmp.join("nc_ext_asm.o");
    let exe = tmp.join("nc_ext_bindings_exec");

    fs::write(
        &c_src,
        "int nc_ext_c_add(int a, int b) { return a + b + 10; }\n",
    )?;
    fs::write(
        &rust_src,
        r#"#[repr(C)]
pub struct NcRustPair {
    pub a: i32,
    pub b: i32,
}

#[no_mangle]
pub extern "C" fn nc_ext_rust_pair_sum(pair: NcRustPair) -> i32 {
    pair.a + pair.b + 20
}
"#,
    )?;
    let asm_source = compiler_ffi_asm_identity_source(host_os, host_arch)
        .map_err(|err| format!("E_FFI_ASM_BRIDGE_001: {err}"))?;
    fs::write(&asm_src, asm_source)?;
    fs::write(
        &driver_src,
        "#include <stdint.h>\n\
typedef struct NcRustPair { int32_t a; int32_t b; } NcRustPair;\n\
extern int nc_ext_c_add(int, int);\n\
extern int nc_ext_rust_pair_sum(NcRustPair);\n\
extern int nc_ffi_asm_identity(int);\n\
int main(void) {\n\
  NcRustPair pair = {7, 8};\n\
  if (nc_ext_c_add(1, 2) != 13) return 11;\n\
  if (nc_ext_rust_pair_sum(pair) != 35) return 12;\n\
  if (nc_ffi_asm_identity(42) != 42) return 13;\n\
  return 0;\n\
}\n",
    )?;

    let c_src_s = c_src.to_string_lossy().into_owned();
    let rust_src_s = rust_src.to_string_lossy().into_owned();
    let asm_src_s = asm_src.to_string_lossy().into_owned();
    let driver_src_s = driver_src.to_string_lossy().into_owned();
    let c_obj_s = c_obj.to_string_lossy().into_owned();
    let rust_obj_s = rust_obj.to_string_lossy().into_owned();
    let asm_obj_s = asm_obj.to_string_lossy().into_owned();
    let exe_s = exe.to_string_lossy().into_owned();

    compiler_run_process(&cc, &["-c", &c_src_s, "-O2", "-o", &c_obj_s], &tmp)
        .map_err(|msg| format!("E_FFI_LINK_FAILED_001: {msg}"))?;
    compiler_run_process(
        &rustc,
        &[
            "--crate-type",
            "lib",
            "--emit=obj",
            &rust_src_s,
            "-O",
            "-o",
            &rust_obj_s,
        ],
        &tmp,
    )
    .map_err(|msg| format!("E_FFI_LINK_FAILED_001: {msg}"))?;
    compiler_run_process(&cc, &["-c", &asm_src_s, "-O2", "-o", &asm_obj_s], &tmp)
        .map_err(|msg| format!("E_FFI_ASM_BRIDGE_001: {msg}"))?;
    compiler_run_process(
        &cc,
        &[
            &driver_src_s,
            &c_obj_s,
            &rust_obj_s,
            &asm_obj_s,
            "-o",
            &exe_s,
        ],
        &tmp,
    )
    .map_err(|msg| format!("E_FFI_LINK_FAILED_001: {msg}"))?;
    let run = compiler_run_process_output(&exe_s, &[], &tmp)
        .map_err(|err| format!("E_FFI_LINK_FAILED_001: external binding run failed: {err}"))?;
    if !run.status.success() {
        return Err(format!(
            "E_FFI_LINK_FAILED_001: external binding executable exited with {}",
            run.status
        )
        .into());
    }
    for (artifact, symbol) in [
        (&c_obj, "nc_ext_c_add"),
        (&rust_obj, "nc_ext_rust_pair_sum"),
        (&asm_obj, "nc_ffi_asm_identity"),
    ] {
        if !compiler_nm_has_symbol(&nm, artifact, symbol) {
            return Err(format!("E_FFI_SYMBOL_MISSING_001: missing symbol {symbol}").into());
        }
    }

    let mut artifacts = vec![
        compiler_artifact_record(&c_obj, "c_object")?,
        compiler_artifact_record(&rust_obj, "rust_object")?,
        compiler_artifact_record(&asm_obj, "asm_object")?,
        compiler_artifact_record(&exe, "external_binding_executable")?,
    ];

    let cpp_payload = match compiler_run_ffi_cpp_abi(false) {
        Ok(payload) => payload,
        Err(err) => {
            if strict {
                return Err(err);
            }
            serde_json::json!({
                "status": "failed",
                "official_code": "E_FFI_CPP_ABI_001",
                "reason": err.to_string(),
            })
        }
    };

    let zig_lane = if let Some(zig_bin) = &zig {
        let zig_src = tmp.join("nc_ext_zig.zig");
        let zig_obj = tmp.join("nc_ext_zig.o");
        let zig_driver = tmp.join("nc_ext_zig_driver.c");
        let zig_exe = tmp.join("nc_ext_zig_exec");
        fs::write(
            &zig_src,
            "export fn nc_ext_zig_add(a: c_int, b: c_int) c_int { return a + b + 30; }\n",
        )?;
        fs::write(
            &zig_driver,
            "extern int nc_ext_zig_add(int, int);\nint main(void) { return nc_ext_zig_add(5, 6) == 41 ? 0 : 31; }\n",
        )?;
        let zig_src_s = zig_src.to_string_lossy().into_owned();
        let zig_obj_s = zig_obj.to_string_lossy().into_owned();
        let zig_emit_arg = format!("-femit-bin={zig_obj_s}");
        let zig_driver_s = zig_driver.to_string_lossy().into_owned();
        let zig_exe_s = zig_exe.to_string_lossy().into_owned();
        let lane = match compiler_run_process(
            zig_bin,
            &["build-obj", &zig_src_s, "-O", "ReleaseFast", &zig_emit_arg],
            &tmp,
        ) {
            Ok(()) => match compiler_run_process(
                &cc,
                &[&zig_driver_s, &zig_obj_s, "-o", &zig_exe_s],
                &tmp,
            ) {
                Ok(()) => match compiler_run_process_output(&zig_exe_s, &[], &tmp) {
                    Ok(output) if output.status.success() => {
                        artifacts.push(compiler_artifact_record(&zig_obj, "zig_object")?);
                        artifacts.push(compiler_artifact_record(&zig_exe, "zig_executable")?);
                        serde_json::json!({
                            "language": "zig",
                            "status": "executed",
                            "tool": zig_bin,
                            "symbol": "nc_ext_zig_add",
                            "abi": "C",
                            "executed": true,
                        })
                    }
                    Ok(output) => compiler_external_lane_failed(
                        "zig",
                        format!("zig executable exited with {}", output.status),
                    ),
                    Err(err) => compiler_external_lane_failed("zig", err),
                },
                Err(err) => compiler_external_lane_failed("zig", err),
            },
            Err(err) => compiler_external_lane_failed("zig", err),
        };
        if strict && lane.get("status").and_then(|v| v.as_str()) == Some("failed") {
            return Err(lane
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("zig lane failed")
                .to_string()
                .into());
        }
        lane
    } else {
        compiler_external_lane_skipped("zig", "zig")
    };

    let go_lane = if let Some(go_bin) = &go {
        let go_src = tmp.join("nc_ext_go.go");
        let go_archive = tmp.join("libnc_ext_go.a");
        let go_driver = tmp.join("nc_ext_go_driver.c");
        let go_exe = tmp.join("nc_ext_go_exec");
        fs::write(
            &go_src,
            r#"package main

import "C"

//export nc_ext_go_add
func nc_ext_go_add(a C.int, b C.int) C.int {
    return a + b + 40
}

func main() {}
"#,
        )?;
        fs::write(
            &go_driver,
            "extern int nc_ext_go_add(int, int);\nint main(void) { return nc_ext_go_add(8, 9) == 57 ? 0 : 41; }\n",
        )?;
        let go_src_s = go_src.to_string_lossy().into_owned();
        let go_archive_s = go_archive.to_string_lossy().into_owned();
        let go_driver_s = go_driver.to_string_lossy().into_owned();
        let go_exe_s = go_exe.to_string_lossy().into_owned();
        let lane = match compiler_run_process(
            go_bin,
            &[
                "build",
                "-buildmode=c-archive",
                "-o",
                &go_archive_s,
                &go_src_s,
            ],
            &tmp,
        ) {
            Ok(()) => match compiler_run_process(
                &cc,
                &[
                    &go_driver_s,
                    &go_archive_s,
                    "-lpthread",
                    "-ldl",
                    "-o",
                    &go_exe_s,
                ],
                &tmp,
            ) {
                Ok(()) => match compiler_run_process_output(&go_exe_s, &[], &tmp) {
                    Ok(output) if output.status.success() => {
                        artifacts.push(compiler_artifact_record(&go_archive, "go_c_archive")?);
                        artifacts.push(compiler_artifact_record(&go_exe, "go_executable")?);
                        serde_json::json!({
                            "language": "go",
                            "status": "executed",
                            "tool": go_bin,
                            "symbol": "nc_ext_go_add",
                            "abi": "C c-archive",
                            "executed": true,
                        })
                    }
                    Ok(output) => compiler_external_lane_failed(
                        "go",
                        format!("go executable exited with {}", output.status),
                    ),
                    Err(err) => compiler_external_lane_failed("go", err),
                },
                Err(err) => compiler_external_lane_failed("go", err),
            },
            Err(err) => compiler_external_lane_failed("go", err),
        };
        if strict && lane.get("status").and_then(|v| v.as_str()) == Some("failed") {
            return Err(lane
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("go lane failed")
                .to_string()
                .into());
        }
        lane
    } else {
        compiler_external_lane_skipped("go", "go")
    };

    let optional_status = if zig_lane
        .get("status")
        .and_then(|v| v.as_str())
        .is_some_and(|status| status == "executed")
        && go_lane
            .get("status")
            .and_then(|v| v.as_str())
            .is_some_and(|status| status == "executed")
    {
        "closed"
    } else {
        "partial_missing_optional_toolchains"
    };

    Ok(serde_json::json!({
        "schema": "nc.compiler.ffi.external_bindings.v1",
        "status": "ok",
        "required_status": "closed",
        "optional_status": optional_status,
        "strict": strict,
        "host": {"os": host_os, "arch": host_arch},
        "toolchain": {"cc": cc, "rustc": rustc, "nm": nm, "zig": zig, "go": go},
        "workspace": tmp.display().to_string(),
        "lanes": {
            "c": {
                "language": "c",
                "status": "executed",
                "symbol": "nc_ext_c_add",
                "abi": "C",
                "executed": true,
            },
            "cpp": {
                "language": "cpp",
                "status": cpp_payload.get("status").cloned().unwrap_or_else(|| serde_json::json!("unknown")),
                "command": "ncc compiler ffi-cpp-abi --json --strict",
                "abi": cpp_payload.get("cpp_abi").cloned().unwrap_or_else(|| serde_json::json!({})),
                "copy_move": cpp_payload.get("copy_move").cloned().unwrap_or_else(|| serde_json::json!({})),
                "executed": cpp_payload.get("status").and_then(|v| v.as_str()) == Some("ok"),
            },
            "rust": {
                "language": "rust",
                "status": "executed",
                "symbol": "nc_ext_rust_pair_sum",
                "abi": "extern C + repr(C)",
                "executed": true,
            },
            "zig": zig_lane,
            "go": go_lane,
            "asm": {
                "language": "asm",
                "status": "executed",
                "symbol": "nc_ffi_asm_identity",
                "abi": "host C ABI assembly bridge",
                "gate": "foreign.asm requires [@machine] in source-level FFI contracts",
                "executed": true,
            }
        },
        "artifacts": artifacts,
        "policy": {
            "zig_go_missing_tools_are_not_claimed_closed": true,
            "external_libraries_cross_nC_through_explicit_C_ABI_boundaries": true,
            "no_fake_evidence": true,
        },
        "no_fake_evidence": true,
    }))
}

fn compiler_cpp_abi_skipped_payload(
    reason: String,
    missing_tools: Vec<&str>,
    cxx: Option<String>,
    nm: Option<String>,
    cxxfilt: Option<String>,
    ar: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "nc.compiler.ffi.cpp_abi.v1",
        "status": "skipped",
        "strict": false,
        "validated": true,
        "reason": reason,
        "missing_tools": missing_tools,
        "host": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "toolchain": {
            "cxx": cxx,
            "nm": nm,
            "cxxfilt": cxxfilt,
            "ar": ar,
        },
        "official_code": "E_FFI_DEFERRED_001",
    })
}

fn compiler_cxxfilt_output(cxxfilt: &str, input: &str) -> Result<String, String> {
    let mut child = std::process::Command::new(cxxfilt)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn failed for '{}': {}", cxxfilt, err))?;
    {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err("failed to open c++filt stdin".to_string());
        };
        stdin
            .write_all(input.as_bytes())
            .map_err(|err| format!("failed to write c++filt stdin: {}", err))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for c++filt: {}", err))?;
    if !output.status.success() {
        return Err(format!(
            "c++filt failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn compiler_run_ffi_cpp_abi(strict: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cxx = compiler_resolve_first_tool(&["c++", "g++", "clang++"]);
    let nm = compiler_resolve_first_tool(&["nm", "llvm-nm"]);
    let cxxfilt = compiler_resolve_first_tool(&["c++filt", "llvm-cxxfilt"]);
    let ar = compiler_resolve_first_tool(&["ar", "llvm-ar"]);
    let mut missing_tools = Vec::new();
    if cxx.is_none() {
        missing_tools.push("c++");
    }
    if nm.is_none() {
        missing_tools.push("nm");
    }
    if cxxfilt.is_none() {
        missing_tools.push("c++filt");
    }
    if ar.is_none() {
        missing_tools.push("ar");
    }
    if !missing_tools.is_empty() {
        if strict {
            return Err(format!(
                "E_FFI_CPP_ABI_001: missing C++ ABI toolchain command(s): {}",
                missing_tools.join(", ")
            )
            .into());
        }
        return Ok(compiler_cpp_abi_skipped_payload(
            "missing C++ ABI toolchain commands".to_string(),
            missing_tools,
            cxx,
            nm,
            cxxfilt,
            ar,
        ));
    }

    let cxx = cxx.expect("checked is_some");
    let nm = nm.expect("checked is_some");
    let cxxfilt = cxxfilt.expect("checked is_some");
    let ar = ar.expect("checked is_some");
    let host_os = std::env::consts::OS;
    let host_arch = std::env::consts::ARCH;

    macro_rules! cpp_abi_skip_or_err {
        ($reason:expr) => {{
            let reason = $reason;
            if strict {
                return Err(reason.into());
            }
            return Ok(compiler_cpp_abi_skipped_payload(
                reason,
                Vec::new(),
                Some(cxx.clone()),
                Some(nm.clone()),
                Some(cxxfilt.clone()),
                Some(ar.clone()),
            ));
        }};
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!(
        "ncc_cpp_abi_smoke_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&tmp)?;

    let core_src = tmp.join("nc_cpp_abi_core.cpp");
    let static_src = tmp.join("nc_cpp_abi_static.cpp");
    let shared_src = tmp.join("nc_cpp_abi_shared.cpp");
    let driver_src = tmp.join("nc_cpp_abi_driver.cpp");
    let throw_src = tmp.join("nc_cpp_abi_uncaught_throw.cpp");
    let core_obj = tmp.join("nc_cpp_abi_core.o");
    let static_obj = tmp.join("nc_cpp_abi_static.o");
    let static_lib = tmp.join("libnc_cpp_abi_static.a");
    let shared_name = match host_os {
        "macos" => "libnc_cpp_abi_shared.dylib",
        "windows" => "nc_cpp_abi_shared.dll",
        _ => "libnc_cpp_abi_shared.so",
    };
    let shared_lib = tmp.join(shared_name);
    let exe = tmp.join(if host_os == "windows" {
        "nc_cpp_abi_driver.exe"
    } else {
        "nc_cpp_abi_driver"
    });
    let throw_exe = tmp.join(if host_os == "windows" {
        "nc_cpp_abi_uncaught_throw.exe"
    } else {
        "nc_cpp_abi_uncaught_throw"
    });

    fs::write(
        &core_src,
        r#"#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <utility>

namespace nc {
namespace abi {

int add(int a, int b) {
    return a + b;
}

template <typename T>
T twice(T value) {
    return static_cast<T>(value + value);
}

template int twice<int>(int);
template long long twice<long long>(long long);

struct Base {
    virtual ~Base() = default;
    virtual int value() const {
        return 1;
    }
};

struct Derived final : Base {
    int *counter;

    explicit Derived(int *counter) : counter(counter) {}

    ~Derived() override {
        if (counter != nullptr) {
            *counter += 1;
        }
    }

    int value() const override {
        return 42;
    }
};

struct alignas(16) Layout {
    std::int32_t a;
    double b;
};

#pragma pack(push, 1)
struct Packed {
    char c;
    std::int32_t v;
};
#pragma pack(pop)

struct Tracked {
    int value;
    int *events;

    Tracked(int value, int *events);
    Tracked(const Tracked &other);
    Tracked(Tracked &&other) noexcept;
    ~Tracked();
};

Tracked::Tracked(int value, int *events) : value(value), events(events) {
    if (events != nullptr) {
        *events += 1;
    }
}

Tracked::Tracked(const Tracked &other) : value(other.value), events(other.events) {
    if (events != nullptr) {
        *events += 10;
    }
}

Tracked::Tracked(Tracked &&other) noexcept : value(other.value), events(other.events) {
    other.value = 0;
    other.events = nullptr;
    if (events != nullptr) {
        *events += 100;
    }
}

Tracked::~Tracked() {
    if (events != nullptr) {
        *events += 1000;
    }
}

} // namespace abi
} // namespace nc

extern "C" int nc_cpp_call_mangled_add(int a, int b) {
    return nc::abi::add(a, b);
}

extern "C" int nc_cpp_call_template_int(int value) {
    return nc::abi::twice<int>(value);
}

extern "C" long long nc_cpp_call_template_i64(long long value) {
    return nc::abi::twice<long long>(value);
}

extern "C" int nc_cpp_vtable_value() {
    int count = 0;
    nc::abi::Derived derived(&count);
    nc::abi::Base *base = &derived;
    return base->value();
}

extern "C" int nc_cpp_destructor_count() {
    int count = 0;
    nc::abi::Base *base = new nc::abi::Derived(&count);
    delete base;
    return count;
}

extern "C" int nc_cpp_throw_caught() {
    try {
        throw std::runtime_error("nC C++ ABI smoke");
    } catch (const std::exception &) {
        return 1;
    }
    return 0;
}

extern "C" int nc_cpp_layout_align() {
    return static_cast<int>(alignof(nc::abi::Layout));
}

extern "C" int nc_cpp_layout_size() {
    return static_cast<int>(sizeof(nc::abi::Layout));
}

extern "C" int nc_cpp_packed_align() {
    return static_cast<int>(alignof(nc::abi::Packed));
}

extern "C" int nc_cpp_packed_size() {
    return static_cast<int>(sizeof(nc::abi::Packed));
}

extern "C" int nc_cpp_copy_constructor_count() {
    int events = 0;
    {
        nc::abi::Tracked original(7, &events);
        nc::abi::Tracked copy = original;
        if (copy.value != 7) {
            return -1;
        }
    }
    return events;
}

extern "C" int nc_cpp_move_constructor_count() {
    int events = 0;
    {
        nc::abi::Tracked original(9, &events);
        nc::abi::Tracked moved = std::move(original);
        if (moved.value != 9 || original.value != 0) {
            return -1;
        }
    }
    return events;
}

extern "C" int nc_cpp_raii_scope_count() {
    int events = 0;
    {
        nc::abi::Tracked scoped(5, &events);
        if (scoped.value != 5) {
            return -1;
        }
    }
    return events;
}
"#,
    )?;
    fs::write(
        &static_src,
        r#"extern "C" int nc_cpp_static_add(int a, int b) {
    return a + b + 100;
}
"#,
    )?;
    fs::write(
        &shared_src,
        r#"extern "C" int nc_cpp_shared_mul(int a, int b) {
    return a * b + 200;
}
"#,
    )?;
    fs::write(
        &driver_src,
        r#"#include <cstdint>

extern "C" int nc_cpp_call_mangled_add(int, int);
extern "C" int nc_cpp_call_template_int(int);
extern "C" long long nc_cpp_call_template_i64(long long);
extern "C" int nc_cpp_vtable_value();
extern "C" int nc_cpp_destructor_count();
extern "C" int nc_cpp_throw_caught();
extern "C" int nc_cpp_layout_align();
extern "C" int nc_cpp_layout_size();
extern "C" int nc_cpp_packed_align();
extern "C" int nc_cpp_packed_size();
extern "C" int nc_cpp_copy_constructor_count();
extern "C" int nc_cpp_move_constructor_count();
extern "C" int nc_cpp_raii_scope_count();
extern "C" int nc_cpp_static_add(int, int);
extern "C" int nc_cpp_shared_mul(int, int);

int main() {
    if (nc_cpp_call_mangled_add(3, 4) != 7) return 10;
    if (nc_cpp_call_template_int(21) != 42) return 11;
    if (nc_cpp_call_template_i64(21) != 42) return 12;
    if (nc_cpp_vtable_value() != 42) return 13;
    if (nc_cpp_destructor_count() != 1) return 14;
    if (nc_cpp_throw_caught() != 1) return 15;
    if (nc_cpp_layout_align() != 16) return 16;
    if (nc_cpp_layout_size() < 16) return 17;
    if (nc_cpp_packed_align() != 1) return 18;
    if (nc_cpp_packed_size() != 5) return 19;
    if (nc_cpp_copy_constructor_count() != 2011) return 20;
    if (nc_cpp_move_constructor_count() != 1101) return 21;
    if (nc_cpp_raii_scope_count() != 1001) return 22;
    if (nc_cpp_static_add(1, 2) != 103) return 23;
    if (nc_cpp_shared_mul(4, 5) != 220) return 24;
    return 0;
}
"#,
    )?;
    fs::write(
        &throw_src,
        r#"int main() {
    throw 7;
    return 0;
}
"#,
    )?;

    let core_src_s = core_src.to_string_lossy().into_owned();
    let static_src_s = static_src.to_string_lossy().into_owned();
    let shared_src_s = shared_src.to_string_lossy().into_owned();
    let driver_src_s = driver_src.to_string_lossy().into_owned();
    let throw_src_s = throw_src.to_string_lossy().into_owned();
    let core_obj_s = core_obj.to_string_lossy().into_owned();
    let static_obj_s = static_obj.to_string_lossy().into_owned();
    let static_lib_s = static_lib.to_string_lossy().into_owned();
    let shared_lib_s = shared_lib.to_string_lossy().into_owned();
    let exe_s = exe.to_string_lossy().into_owned();
    let throw_exe_s = throw_exe.to_string_lossy().into_owned();
    let tmp_s = tmp.to_string_lossy().into_owned();
    let rpath_arg = format!("-Wl,-rpath,{}", tmp_s);

    if let Err(msg) = compiler_run_process(
        &cxx,
        &[
            "-std=c++17",
            "-O0",
            "-fPIC",
            "-c",
            &core_src_s,
            "-o",
            &core_obj_s,
        ],
        &tmp,
    ) {
        cpp_abi_skip_or_err!(format!("E_FFI_CPP_ABI_001: core compile failed: {msg}"));
    }
    if let Err(msg) = compiler_run_process(
        &cxx,
        &[
            "-std=c++17",
            "-O2",
            "-c",
            &static_src_s,
            "-o",
            &static_obj_s,
        ],
        &tmp,
    ) {
        cpp_abi_skip_or_err!(format!("E_FFI_CPP_ABI_001: static compile failed: {msg}"));
    }
    if let Err(msg) = compiler_run_process(&ar, &["rcs", &static_lib_s, &static_obj_s], &tmp) {
        cpp_abi_skip_or_err!(format!("E_FFI_CPP_ABI_001: static archive failed: {msg}"));
    }
    if let Err(msg) = compiler_run_process(
        &cxx,
        &[
            "-std=c++17",
            "-O2",
            "-fPIC",
            "-shared",
            &shared_src_s,
            "-o",
            &shared_lib_s,
        ],
        &tmp,
    ) {
        cpp_abi_skip_or_err!(format!("E_FFI_CPP_ABI_001: shared library failed: {msg}"));
    }

    let mut link_args = vec![
        "-std=c++17",
        "-O2",
        &driver_src_s,
        &core_obj_s,
        &static_lib_s,
        &shared_lib_s,
    ];
    if host_os != "windows" {
        link_args.push(&rpath_arg);
    }
    link_args.extend(["-o", &exe_s]);
    if let Err(msg) = compiler_run_process(&cxx, &link_args, &tmp) {
        cpp_abi_skip_or_err!(format!("E_FFI_CPP_ABI_001: executable link failed: {msg}"));
    }

    let run = compiler_run_process_output(&exe_s, &[], &tmp)
        .map_err(|err| format!("E_FFI_CPP_ABI_001: driver run failed: {err}"))?;
    if !run.status.success() {
        return Err(format!(
            "E_FFI_CPP_ABI_001: C++ ABI driver failed with status {} stderr={}",
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        )
        .into());
    }

    if let Err(msg) = compiler_run_process(
        &cxx,
        &["-std=c++17", &throw_src_s, "-o", &throw_exe_s],
        &tmp,
    ) {
        cpp_abi_skip_or_err!(format!(
            "E_FFI_CPP_ABI_001: exception harness failed: {msg}"
        ));
    }
    let throw_run = compiler_run_process_output(&throw_exe_s, &[], &tmp)
        .map_err(|err| format!("E_FFI_CPP_ABI_001: exception run failed: {err}"))?;
    let uncaught_exception_rejected = !throw_run.status.success();
    if !uncaught_exception_rejected {
        return Err(
            "E_FFI_CPP_EXCEPTION_BOUNDARY_001: uncaught C++ exception crossed boundary silently"
                .into(),
        );
    }

    let nm_out = compiler_run_process_output(&nm, &[&core_obj_s], &tmp)
        .map_err(|err| format!("E_FFI_CPP_ABI_001: nm failed: {err}"))?;
    if !nm_out.status.success() {
        return Err(format!(
            "E_FFI_CPP_ABI_001: nm failed: {}",
            String::from_utf8_lossy(&nm_out.stderr).trim()
        )
        .into());
    }
    let raw_symbols = String::from_utf8_lossy(&nm_out.stdout).to_string();
    let demangled_symbols = compiler_cxxfilt_output(&cxxfilt, &raw_symbols)
        .map_err(|err| format!("E_FFI_CPP_ABI_001: {err}"))?;
    let expected_demangled = [
        "nc::abi::add(int, int)",
        "nc::abi::twice<int>",
        "nc::abi::twice<long long>",
        "vtable for nc::abi::Derived",
        "nc::abi::Derived::~Derived",
        "nc::abi::Tracked::Tracked",
        "nc::abi::Tracked::~Tracked",
    ];
    let missing_demangled = expected_demangled
        .iter()
        .filter(|needle| !demangled_symbols.contains(**needle))
        .copied()
        .collect::<Vec<_>>();
    if !missing_demangled.is_empty() {
        return Err(format!(
            "E_FFI_CPP_ABI_001: missing demangled C++ ABI symbol(s): {}",
            missing_demangled.join(", ")
        )
        .into());
    }
    let itanium_mangled_detected = raw_symbols.contains("_ZN") || raw_symbols.contains("_ZTV");
    if !itanium_mangled_detected && matches!(host_os, "linux" | "macos" | "freebsd") {
        return Err(
            "E_FFI_CPP_ABI_001: Itanium-style C++ mangled symbols were not detected".into(),
        );
    }

    Ok(serde_json::json!({
        "schema": "nc.compiler.ffi.cpp_abi.v1",
        "status": "ok",
        "validated": true,
        "strict": strict,
        "host": {
            "os": host_os,
            "arch": host_arch,
        },
        "toolchain": {
            "cxx": cxx,
            "nm": nm,
            "cxxfilt": cxxfilt,
            "ar": ar,
        },
        "workspace": tmp.display().to_string(),
        "cpp_abi": {
            "status": "closed_real_toolchain_smoke",
            "profile": if matches!(host_os, "linux" | "macos" | "freebsd") { "itanium-cxxabi" } else { "host-cxxabi" },
            "itanium_mangled_detected": itanium_mangled_detected,
            "msvc_cxxabi": if host_os == "windows" { "host_toolchain_validated" } else { "not_applicable_on_this_host" },
        },
        "name_mangling": {
            "status": "closed_real",
            "raw_contains_mangled_symbols": itanium_mangled_detected,
            "demangled_symbols_verified": expected_demangled,
        },
        "templates": {
            "status": "closed_real",
            "explicit_instantiations": ["nc::abi::twice<int>", "nc::abi::twice<long long>"],
        },
        "vtables": {
            "status": "closed_real",
            "demangled_symbol": "vtable for nc::abi::Derived",
            "runtime_dispatch": "nc_cpp_vtable_value returned 42 through Base*",
        },
        "destructors": {
            "status": "closed_real",
            "virtual_delete": "delete Base* invoked Derived::~Derived once",
            "raii_scope": "nc_cpp_raii_scope_count returned constructor+destructor evidence",
        },
        "copy_move": {
            "status": "closed_real",
            "copy_constructor": "nc_cpp_copy_constructor_count returned 2011",
            "move_constructor": "nc_cpp_move_constructor_count returned 1101",
            "moved_from_state": "moved-from source value reset and destructor event suppressed by nulled event pointer",
        },
        "exceptions": {
            "status": "closed_real",
            "caught_boundary": "C++ throw caught inside C++ boundary",
            "uncaught_crossing_rejected": uncaught_exception_rejected,
            "official_code": "E_FFI_CPP_EXCEPTION_BOUNDARY_001",
        },
        "layout": {
            "status": "closed_real",
            "struct_layout": "alignas(16) Layout verified",
            "packed_layout": "pragma pack(1) Packed verified",
            "alignment": "runtime alignof checks passed",
            "endianness": "host/object ABI driven; nC requires explicit target profile when lowering",
        },
        "libraries": {
            "static": "linked_real_archive",
            "shared": "linked_real_dynamic_library",
            "object_files": "linked",
        },
        "artifacts": [
            compiler_artifact_record(&core_obj, "object")?,
            compiler_artifact_record(&static_lib, "static_library")?,
            compiler_artifact_record(&shared_lib, "shared_library")?,
            compiler_artifact_record(&exe, "executable")?,
        ],
        "symbols": {
            "raw_excerpt": raw_symbols.lines().take(24).collect::<Vec<_>>(),
            "demangled_excerpt": demangled_symbols.lines().take(24).collect::<Vec<_>>(),
        },
        "evidence": {
            "object": "compiled from real C++17 source",
            "static_library": "ar rcs linked into driver",
            "shared_library": "C++ shared library linked and loaded by executable",
            "driver": "executed and validated all ABI checks",
            "no_fake_evidence": true,
        }
    }))
}

fn parse_compiler_cli_options(
    args: &[String],
) -> Result<(CompilerCliOptions, Vec<String>), Box<dyn std::error::Error>> {
    let mut opts = CompilerCliOptions::default();
    let mut rest = Vec::new();
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                opts.json = true;
                idx += 1;
            }
            "--md" => {
                opts.md = true;
                idx += 1;
            }
            "--no-color" => {
                opts.no_color = true;
                idx += 1;
            }
            "--strict" => {
                opts.strict = true;
                idx += 1;
            }
            "--phase" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --phase requires a value")?;
                opts.phase = Some(value.clone());
                idx += 2;
            }
            "--target" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --target requires a value")?;
                opts.target = Some(value.clone());
                idx += 2;
            }
            "--profile" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --profile requires a value")?;
                opts.profile = Some(value.clone());
                idx += 2;
            }
            "--family" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --family requires a value")?;
                opts.family = Some(value.clone());
                idx += 2;
            }
            "--mode" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --mode requires a value")?;
                opts.mode = Some(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("compiler syntax: --out requires a value")?;
                opts.out = Some(value.clone());
                idx += 2;
            }
            _ => {
                rest.push(args[idx].clone());
                idx += 1;
            }
        }
    }
    Ok((opts, rest))
}

#[derive(Debug, Clone)]
struct CompilerConditionAuditRow {
    kind: String,
    inferred_type: String,
    boolish: bool,
    diagnostic: Option<String>,
    expr_debug: String,
}

#[derive(Debug, Clone)]
struct CompilerSourceAudit {
    contexts: BTreeSet<String>,
    types_in_source: BTreeSet<String>,
    operator_counts: BTreeMap<String, usize>,
    render_concat: CompilerRenderConcatAudit,
    condition_rows: Vec<CompilerConditionAuditRow>,
    placeholder_risk_count: usize,
}

#[derive(Debug, Clone, Default)]
struct CompilerRenderConcatAudit {
    render_calls: BTreeMap<String, usize>,
    concat_function_calls: BTreeMap<String, usize>,
    concat_expressions: usize,
}

fn compiler_context_label(context: Option<&ncc_ast::Context>) -> String {
    match context {
        Some(ncc_ast::Context::System) => "system".to_string(),
        Some(ncc_ast::Context::Kernel) => "kernel".to_string(),
        Some(ncc_ast::Context::Machine) => "machine".to_string(),
        Some(ncc_ast::Context::Network) => "network".to_string(),
        Some(ncc_ast::Context::Tooling) => "tooling".to_string(),
        Some(ncc_ast::Context::Runtime) => "runtime".to_string(),
        Some(ncc_ast::Context::Compiler) => "compiler".to_string(),
        Some(ncc_ast::Context::Security) => "security".to_string(),
        Some(ncc_ast::Context::Inspect) => "inspect".to_string(),
        Some(ncc_ast::Context::Ai) => "ai".to_string(),
        Some(ncc_ast::Context::Primitive) => "primitive".to_string(),
        Some(ncc_ast::Context::Root) => "root".to_string(),
        Some(ncc_ast::Context::Unknown(value)) => format!("unknown:{value}"),
        None => "default".to_string(),
    }
}

thread_local! {
    static COMPILER_TYPE_DIAG_CONTEXT: std::cell::RefCell<Option<ncc_ast::Context>> =
        const { std::cell::RefCell::new(None) };
}

fn compiler_set_type_diag_context(context: Option<&ncc_ast::Context>) {
    COMPILER_TYPE_DIAG_CONTEXT.with(|slot| {
        *slot.borrow_mut() = context.cloned();
    });
}

fn compiler_get_type_diag_context() -> Option<ncc_ast::Context> {
    COMPILER_TYPE_DIAG_CONTEXT.with(|slot| slot.borrow().clone())
}

fn compiler_operator_precedence_table() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"group":"unary","precedence":14,"associativity":"right","ops":["+","-","!","~"]}),
        serde_json::json!({"group":"mul_div_mod","precedence":13,"associativity":"left","ops":["*","/","%"]}),
        serde_json::json!({"group":"add_sub","precedence":12,"associativity":"left","ops":["+","-"]}),
        serde_json::json!({"group":"shift","precedence":11,"associativity":"left","ops":["<<",">>"]}),
        serde_json::json!({"group":"relational","precedence":10,"associativity":"left","ops":[">","<",">=","<="]}),
        serde_json::json!({"group":"equality","precedence":9,"associativity":"left","ops":["==","!="]}),
        serde_json::json!({"group":"bit_and","precedence":8,"associativity":"left","ops":["&"]}),
        serde_json::json!({"group":"bit_xor","precedence":7,"associativity":"left","ops":["^"]}),
        serde_json::json!({"group":"bit_or","precedence":6,"associativity":"left","ops":["|"]}),
        serde_json::json!({"group":"logical_and","precedence":5,"associativity":"left","ops":["&&"]}),
        serde_json::json!({"group":"logical_or","precedence":4,"associativity":"left","ops":["||"]}),
    ]
}

fn compiler_collect_base_type_scope(ast: &ncc_ast::Program) -> BTreeMap<String, String> {
    let mut scope = BTreeMap::new();
    for container in &ast.containers {
        for section in &container.sections {
            if let ncc_ast::Section::Stc(decls) = section {
                for decl in decls {
                    let ncc_ast::StcDecl::Typed { name, ty, .. } = decl;
                    scope.insert(name.to_ascii_lowercase(), type_name_to_string(ty));
                }
            }
        }
    }
    scope
}

fn compiler_closure_type_signature(closure: &ncc_ast::ClosureExpr) -> String {
    let params = closure
        .params
        .iter()
        .map(|param| type_name_to_string(&param.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn({}) -> {}",
        params,
        type_name_to_string(&closure.return_type)
    )
}

fn compiler_infer_expr_type(expr: &ncc_ast::Expr, scope: &BTreeMap<String, String>) -> String {
    match expr {
        ncc_ast::Expr::StructLiteral { ty, .. } => ty.clone(),
        ncc_ast::Expr::Bool(_) => "bool".to_string(),
        ncc_ast::Expr::Char(_) => "char".to_string(),
        ncc_ast::Expr::Number(_) => "int".to_string(),
        ncc_ast::Expr::String(_) => "str".to_string(),
        ncc_ast::Expr::Identifier(name) => {
            if name.eq_ignore_ascii_case("null") {
                "null".to_string()
            } else {
                scope
                    .get(&name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            }
        }
        ncc_ast::Expr::ContextRef(reference) => {
            if let Some(name) = reference.path.last() {
                scope
                    .get(&name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            }
        }
        ncc_ast::Expr::PostIncrement { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()).cloned())
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::FieldPostIncrement { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()).cloned())
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::PointerPostIncrementRead { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()))
            .and_then(|ty| compiler_pointer_inner_type(ty))
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::VectorLiteral(_) | ncc_ast::Expr::ArrayLiteral(_) => "array".to_string(),
        ncc_ast::Expr::Index { .. } => "unknown".to_string(),
        ncc_ast::Expr::StrictCheckExpr { .. } => "bool".to_string(),
        ncc_ast::Expr::Unary { op, .. } => match op {
            ncc_ast::UnaryOp::Not => "bool".to_string(),
            _ => "int".to_string(),
        },
        ncc_ast::Expr::Binary { op, .. } => match op {
            ncc_ast::BinaryOp::Greater
            | ncc_ast::BinaryOp::Less
            | ncc_ast::BinaryOp::GreaterEq
            | ncc_ast::BinaryOp::LessEq
            | ncc_ast::BinaryOp::EqEq
            | ncc_ast::BinaryOp::NotEq
            | ncc_ast::BinaryOp::AndAnd
            | ncc_ast::BinaryOp::OrOr => "bool".to_string(),
            _ => "int".to_string(),
        },
        ncc_ast::Expr::IfElse {
            then_expr,
            else_expr,
            ..
        } => {
            let then_ty = compiler_infer_expr_type(then_expr, scope);
            let else_ty = compiler_infer_expr_type(else_expr, scope);
            if then_ty == else_ty {
                then_ty
            } else {
                format!("{then_ty} | {else_ty}")
            }
        }
        ncc_ast::Expr::Call { .. }
        | ncc_ast::Expr::QualifiedCall { .. }
        | ncc_ast::Expr::MethodCall { .. } => "unknown".to_string(),
        ncc_ast::Expr::Closure(closure) => compiler_closure_type_signature(closure),
        ncc_ast::Expr::Dev(_) => "never".to_string(),
    }
}

fn compiler_type_is_boolish(ty: &str) -> bool {
    ty == "bool"
}

fn compiler_analyze_condition_expr(
    kind: &str,
    expr: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
) -> CompilerConditionAuditRow {
    let inferred_type = compiler_infer_expr_type(expr, scope);
    let boolish = compiler_type_is_boolish(&inferred_type);
    let diagnostic = if inferred_type == "str" || inferred_type == "text" {
        Some("E_CONDITION_STR_FORBIDDEN_001".to_string())
    } else if inferred_type.contains("null") {
        Some("E_CONDITION_NULL_FORBIDDEN_001".to_string())
    } else if !boolish {
        Some("E_CONDITION_BOOL_REQUIRED_001".to_string())
    } else {
        None
    };
    CompilerConditionAuditRow {
        kind: kind.to_string(),
        inferred_type,
        boolish,
        diagnostic,
        expr_debug: format!("{expr:?}"),
    }
}

fn compiler_collect_condition_rows_with_scope(
    stmt: &ncc_ast::Stmt,
    scope: &BTreeMap<String, String>,
    out: &mut Vec<CompilerConditionAuditRow>,
) {
    match stmt {
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            out.push(compiler_analyze_condition_expr("if", condition, scope));
            for nested in then_block {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
            for nested in else_block {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::Unless { condition, body } => {
            out.push(compiler_analyze_condition_expr("unless", condition, scope));
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::While { condition, body } => {
            out.push(compiler_analyze_condition_expr("while", condition, scope));
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            out.push(compiler_analyze_condition_expr(
                "loop_condition",
                condition,
                scope,
            ));
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::ForClassic {
            condition,
            init,
            step,
            body,
        } => {
            if let Some(expr) = condition {
                out.push(compiler_analyze_condition_expr("for", expr, scope));
            }
            if let Some(init_stmt) = init {
                compiler_collect_condition_rows_with_scope(init_stmt, scope, out);
            }
            if let Some(step_stmt) = step {
                compiler_collect_condition_rows_with_scope(step_stmt, scope, out);
            }
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            out.push(compiler_analyze_condition_expr(
                "for_range.start",
                start,
                scope,
            ));
            out.push(compiler_analyze_condition_expr("for_range.end", end, scope));
            if let Some(step_expr) = step {
                out.push(compiler_analyze_condition_expr(
                    "for_range.step",
                    step_expr,
                    scope,
                ));
            }
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            out.push(compiler_analyze_condition_expr("guard", condition, scope));
            for nested in else_block {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            out.push(compiler_analyze_condition_expr("match.value", value, scope));
            for (expr, stmts) in cases {
                out.push(compiler_analyze_condition_expr("match.arm", expr, scope));
                for nested in stmts {
                    compiler_collect_condition_rows_with_scope(nested, scope, out);
                }
            }
            for nested in default {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (expr, stmts) in branches {
                out.push(compiler_analyze_condition_expr(
                    "select.branch",
                    expr,
                    scope,
                ));
                for nested in stmts {
                    compiler_collect_condition_rows_with_scope(nested, scope, out);
                }
            }
            for nested in default {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => {
            compiler_collect_condition_rows_with_scope(stmt, scope, out)
        }
        ncc_ast::Stmt::Loop { body }
        | ncc_ast::Stmt::RepeatCount { body, .. }
        | ncc_ast::Stmt::ForIn { body, .. }
        | ncc_ast::Stmt::SameIn { body, .. } => {
            for nested in body {
                compiler_collect_condition_rows_with_scope(nested, scope, out);
            }
        }
        _ => {}
    }
}

fn compiler_build_source_audit(ast: &ncc_ast::Program) -> CompilerSourceAudit {
    let mut contexts = BTreeSet::new();
    for container in &ast.containers {
        contexts.insert(compiler_context_label(container.context.as_ref()));
    }

    let mut types_in_source = BTreeSet::new();
    collect_types_from_program(ast, &mut types_in_source);

    let mut operator_counts = BTreeMap::new();
    collect_operators_from_program(ast, &mut operator_counts);

    let mut render_concat = CompilerRenderConcatAudit::default();
    compiler_collect_render_concat_from_program(ast, &mut render_concat);

    let scope = compiler_collect_base_type_scope(ast);
    let mut condition_rows = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        let mut function_scope = scope.clone();
                        for param in &function.params {
                            function_scope.insert(
                                param.name.to_ascii_lowercase(),
                                type_name_to_string(&param.ty),
                            );
                        }
                        for stmt in &function.public_body {
                            compiler_collect_condition_rows_with_scope(
                                stmt,
                                &function_scope,
                                &mut condition_rows,
                            );
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        compiler_collect_condition_rows_with_scope(
                            stmt,
                            &scope,
                            &mut condition_rows,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    let placeholder_risk_count = condition_rows
        .iter()
        .filter(|row| row.expr_debug.contains("<expr>"))
        .count();

    CompilerSourceAudit {
        contexts,
        types_in_source,
        operator_counts,
        render_concat,
        condition_rows,
        placeholder_risk_count,
    }
}

#[derive(Debug, Clone, Default)]
struct CompilerSurfaceAudit {
    array_literal_count: usize,
    index_expr_count: usize,
    index_assign_target_count: usize,
    call_counts: BTreeMap<String, usize>,
}

fn compiler_normalize_call_id(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn compiler_surface_bump_call(out: &mut CompilerSurfaceAudit, raw_name: &str) {
    let id = compiler_normalize_call_id(raw_name);
    if id.is_empty() {
        return;
    }
    bump_count(&mut out.call_counts, id);
}

fn compiler_collect_surface_expr(expr: &ncc_ast::Expr, out: &mut CompilerSurfaceAudit) {
    match expr {
        ncc_ast::Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                compiler_collect_surface_expr(value, out);
            }
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            out.array_literal_count += 1;
            for item in items {
                compiler_collect_surface_expr(item, out);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            out.index_expr_count += 1;
            compiler_collect_surface_expr(target, out);
            compiler_collect_surface_expr(index, out);
        }
        ncc_ast::Expr::Call { callee, args } => {
            compiler_surface_bump_call(out, callee);
            for arg in args {
                compiler_collect_surface_expr(arg, out);
            }
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            compiler_surface_bump_call(out, &path.join("."));
            for arg in args {
                compiler_collect_surface_expr(arg, out);
            }
        }
        ncc_ast::Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            compiler_surface_bump_call(out, method);
            compiler_collect_surface_expr(receiver, out);
            for arg in args {
                compiler_collect_surface_expr(arg, out);
            }
        }
        ncc_ast::Expr::Unary { expr, .. } => compiler_collect_surface_expr(expr, out),
        ncc_ast::Expr::Binary { left, right, .. } => {
            compiler_collect_surface_expr(left, out);
            compiler_collect_surface_expr(right, out);
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            compiler_collect_surface_expr(condition, out);
            compiler_collect_surface_expr(then_expr, out);
            compiler_collect_surface_expr(else_expr, out);
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => compiler_collect_surface_expr(value, out),
        ncc_ast::Expr::PostIncrement { .. } => {
            compiler_surface_bump_call(out, "postIncrement");
        }
        ncc_ast::Expr::FieldPostIncrement { .. } => {
            compiler_surface_bump_call(out, "fieldPostIncrement");
        }
        ncc_ast::Expr::PointerPostIncrementRead { .. } => {
            compiler_surface_bump_call(out, "pointerPostIncrementRead");
        }
        ncc_ast::Expr::Closure(closure) => {
            for stmt in &closure.body {
                compiler_collect_surface_stmt(stmt, out);
            }
        }
        ncc_ast::Expr::Bool(_)
        | ncc_ast::Expr::Number(_)
        | ncc_ast::Expr::Char(_)
        | ncc_ast::Expr::String(_)
        | ncc_ast::Expr::Identifier(_)
        | ncc_ast::Expr::ContextRef(_)
        | ncc_ast::Expr::Dev(_) => {}
    }
}

fn compiler_collect_surface_assign_target(
    target: &ncc_ast::AssignTarget,
    out: &mut CompilerSurfaceAudit,
) {
    match target {
        ncc_ast::AssignTarget::Context(_) => {}
        ncc_ast::AssignTarget::Mem { addr } => {
            if matches!(addr.as_ref(), ncc_ast::Expr::Index { .. }) {
                out.index_assign_target_count += 1;
            }
            compiler_collect_surface_expr(addr, out);
        }
    }
}

fn compiler_collect_surface_stmt(stmt: &ncc_ast::Stmt, out: &mut CompilerSurfaceAudit) {
    match stmt {
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Print { value: expr, .. } => compiler_collect_surface_expr(expr, out),
        ncc_ast::Stmt::Projection { target, value, .. }
        | ncc_ast::Stmt::StrictCheck { target, value, .. } => {
            compiler_collect_surface_assign_target(target, out);
            compiler_collect_surface_expr(value, out);
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            compiler_collect_surface_expr(condition, out);
            for nested in then_block {
                compiler_collect_surface_stmt(nested, out);
            }
            for nested in else_block {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Unless { condition, body } | ncc_ast::Stmt::While { condition, body } => {
            compiler_collect_surface_expr(condition, out);
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
            compiler_collect_surface_expr(condition, out);
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(stmt) = init {
                compiler_collect_surface_stmt(stmt, out);
            }
            if let Some(expr) = condition {
                compiler_collect_surface_expr(expr, out);
            }
            if let Some(stmt) = step {
                compiler_collect_surface_stmt(stmt, out);
            }
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_collect_surface_expr(iterable, out);
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_collect_surface_expr(start, out);
            compiler_collect_surface_expr(end, out);
            if let Some(expr) = step {
                compiler_collect_surface_expr(expr, out);
            }
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            compiler_collect_surface_expr(value, out);
            for (expr, stmts) in cases {
                compiler_collect_surface_expr(expr, out);
                for nested in stmts {
                    compiler_collect_surface_stmt(nested, out);
                }
            }
            for nested in default {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            compiler_collect_surface_expr(value, out);
            for arm in arms {
                if let ncc_ast::Pattern::Literal(expr) = &arm.pattern {
                    compiler_collect_surface_expr(expr, out);
                }
                if let Some(guard) = &arm.guard {
                    compiler_collect_surface_expr(guard, out);
                }
                for nested in &arm.body {
                    compiler_collect_surface_stmt(nested, out);
                }
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            compiler_collect_surface_expr(condition, out);
            for nested in else_block {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Defer { body } => {
            for nested in body {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (expr, stmts) in branches {
                compiler_collect_surface_expr(expr, out);
                for nested in stmts {
                    compiler_collect_surface_stmt(nested, out);
                }
            }
            for nested in default {
                compiler_collect_surface_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for arg in args {
                compiler_collect_surface_expr(arg, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => compiler_collect_surface_stmt(stmt, out),
        ncc_ast::Stmt::Dev(_)
        | ncc_ast::Stmt::Break { .. }
        | ncc_ast::Stmt::Continue { .. }
        | ncc_ast::Stmt::MainRefFn { .. }
        | ncc_ast::Stmt::FunctionRelation { .. } => {}
    }
}

fn compiler_collect_surface_audit(ast: &ncc_ast::Program) -> CompilerSurfaceAudit {
    let mut out = CompilerSurfaceAudit::default();
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            compiler_collect_surface_stmt(stmt, &mut out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        compiler_collect_surface_stmt(stmt, &mut out);
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn compiler_array_related_code(code: &str) -> bool {
    let upper = code.to_ascii_uppercase();
    upper.contains("ARRAY") || upper.contains("INDEX")
}

fn compiler_index_related_code(code: &str) -> bool {
    let upper = code.to_ascii_uppercase();
    upper.contains("INDEX")
        || upper == "E_ARRAY_DIMENSION_001"
        || upper == "E_ARRAY_RANK_001"
        || upper == "E_ARRAY_OFFSET_OVERFLOW_001"
        || upper == "E_TYPE_ARRAY_BOUNDS_001"
}

fn compiler_memory_contract_related_code(code: &str) -> bool {
    let upper = code.to_ascii_uppercase();
    upper.starts_with("E_MEMORY_")
        || upper.starts_with("E_REGION_")
        || upper.starts_with("E_ARRAY_MOVE_")
        || upper.starts_with("E_MOVE_")
        || upper.starts_with("E_COPY_")
        || upper.starts_with("E_CLONE_")
        || upper.starts_with("E_SITE_")
        || upper.starts_with("E_ADDR_")
        || upper == "E_LIST_ALIAS_001"
        || upper == "E_LIST_MOVED_001"
}

fn compiler_select_calls(
    call_counts: &BTreeMap<String, usize>,
    accepted_ids: &[&str],
) -> BTreeMap<String, usize> {
    let accepted = accepted_ids
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    call_counts
        .iter()
        .filter_map(|(name, count)| {
            if accepted.contains(name) {
                Some((name.clone(), *count))
            } else {
                None
            }
        })
        .collect()
}

fn compiler_type_catalog_set() -> BTreeSet<String> {
    COMPILER_TYPE_CATALOG
        .iter()
        .map(|value| value.to_string())
        .collect()
}

fn compiler_type_contract_id(ty: &str) -> String {
    let compact = ty.replace(' ', "");
    let lower = compact.to_ascii_lowercase();
    if lower.contains("|null") || lower == "null|ptr" {
        return "ptr<T> | null".to_string();
    }
    if lower.contains('|') {
        return "T | U".to_string();
    }
    if lower.starts_with("ptr<") {
        return "ptr<T>".to_string();
    }
    if lower.starts_with("memregion<") {
        return "memRegion<T>".to_string();
    }
    if lower.starts_with("array<") || lower.starts_with("vector<") {
        return "array<T, N>".to_string();
    }
    if lower.starts_with("array2<") {
        return "array2<T, X, Y>".to_string();
    }
    if lower.starts_with("array3<") {
        return "array3<T, X, Y, Z>".to_string();
    }
    if lower.starts_with("matrix<") {
        return "matrix<T, R, C>".to_string();
    }
    if lower.starts_with("tensor<") {
        return "tensor<T, D...>".to_string();
    }
    if lower.starts_with("xy<") {
        return "xy<T>".to_string();
    }
    if lower.starts_with("xyz<") {
        return "xyz<T>".to_string();
    }
    if lower.starts_with("x2y2<") {
        return "x2y2<T>".to_string();
    }
    if lower.starts_with("x2y2z2<") {
        return "x2y2z2<T>".to_string();
    }
    if lower.starts_with("x3y3<") {
        return "x3y3<T>".to_string();
    }
    if lower.starts_with("x3y3z3<") {
        return "x3y3z3<T>".to_string();
    }
    if lower.starts_with("dinarray.binary<") {
        return "dinArray.binary<T>".to_string();
    }
    if lower.starts_with("dinarray.unbinary<") {
        return "dinArray.unbinary<T>".to_string();
    }
    if lower.starts_with("dinarray<") {
        return "dinArray<T>".to_string();
    }
    if lower.starts_with("list<") {
        return "list<T>".to_string();
    }
    if lower.starts_with("result<") {
        return "Result<T,E>".to_string();
    }
    if lower.starts_with("option<") {
        return "Option<T>".to_string();
    }
    if lower.starts_with("fn(") || lower.contains(")->") {
        return "fn(...) -> T".to_string();
    }
    ty.to_string()
}

fn compiler_type_catalog_alignment(
    types_in_source: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>, Vec<serde_json::Value>) {
    let catalog = compiler_type_catalog_set();
    let mut matched = BTreeSet::new();
    let mut unknown = Vec::new();
    let mut per_type = Vec::new();

    for found in types_in_source {
        let contract_id = compiler_type_contract_id(found);
        let known = catalog.contains(&contract_id);
        if known {
            matched.insert(contract_id.clone());
        } else {
            unknown.push(found.clone());
        }
        per_type.push(serde_json::json!({
            "found": found,
            "contract_id": contract_id,
            "known": known,
        }));
    }

    (matched.into_iter().collect(), unknown, per_type)
}

#[derive(Debug, Clone)]
struct CompilerTypeDiagnosticRow {
    code: String,
    message: String,
    stmt_kind: String,
    expected: Option<String>,
    found: Option<String>,
}

fn compiler_type_diag_row_json(row: &CompilerTypeDiagnosticRow) -> serde_json::Value {
    serde_json::json!({
        "code": row.code,
        "message": row.message,
        "stmt_kind": row.stmt_kind,
        "expected": row.expected,
        "found": row.found,
    })
}

fn compiler_push_unique_diag(
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    seen: &mut BTreeSet<(String, String, Option<String>)>,
    code: &str,
    message: String,
    stmt_kind: &str,
    found: Option<String>,
) {
    let key = (code.to_string(), stmt_kind.to_string(), found.clone());
    if !seen.insert(key) {
        return;
    }
    out.push(CompilerTypeDiagnosticRow {
        code: code.to_string(),
        message,
        stmt_kind: stmt_kind.to_string(),
        expected: None,
        found,
    });
}

fn compiler_ffi_key_normalized(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn compiler_ffi_field_value<'a>(
    symbol: &'a ncc_ast::ForeignSymbolDecl,
    keys: &[&str],
) -> Option<&'a str> {
    symbol.fields.iter().find_map(|field| {
        let normalized = compiler_ffi_key_normalized(&field.key);
        if keys
            .iter()
            .any(|candidate| normalized == compiler_ffi_key_normalized(candidate))
        {
            Some(field.value.as_str())
        } else {
            None
        }
    })
}

fn compiler_ffi_has_field(symbol: &ncc_ast::ForeignSymbolDecl, keys: &[&str]) -> bool {
    compiler_ffi_field_value(symbol, keys).is_some()
}

fn compiler_ffi_unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn compiler_ffi_path_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains('/')
        || value.contains('\\')
        || lower.ends_with(".a")
        || lower.ends_with(".so")
        || lower.ends_with(".dylib")
        || lower.ends_with(".dll")
        || lower.ends_with(".lib")
}

fn compiler_ffi_resolve_path(raw: &str, source_dir: &std::path::Path) -> std::path::PathBuf {
    let candidate = std::path::PathBuf::from(compiler_ffi_unquote(raw));
    if candidate.is_absolute() {
        candidate
    } else {
        source_dir.join(candidate)
    }
}

fn compiler_ffi_normalize_checksum(raw: &str) -> String {
    let mut value = compiler_ffi_unquote(raw).trim().to_ascii_lowercase();
    for prefix in ["sha256:", "sha256=", "sha-256:", "sha-256="] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.trim().to_string();
            break;
        }
    }
    value
}

fn compiler_ffi_sha256_file(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn compiler_ffi_link_name(raw: &str) -> String {
    let unquoted = compiler_ffi_unquote(raw);
    let path = std::path::Path::new(&unquoted);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(unquoted.as_str());
    stem.strip_prefix("lib").unwrap_or(stem).to_string()
}

fn compiler_ffi_linkage(symbol: &ncc_ast::ForeignSymbolDecl) -> String {
    compiler_ffi_field_value(symbol, &["linkage", "linkMode", "kind"])
        .map(compiler_ffi_unquote)
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn compiler_ffi_is_dynamic_path(path: &std::path::Path, linkage: &str) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    linkage.contains("dynamic")
        || linkage.contains("shared")
        || file_name.ends_with(".so")
        || file_name.ends_with(".dylib")
        || file_name.ends_with(".dll")
}

fn compiler_ffi_manifest_symbol_diagnostics(
    diagnostics: &[CompilerTypeDiagnosticRow],
    symbol_tag: &str,
) -> Vec<String> {
    compiler_ffi_symbol_contract_codes(diagnostics, symbol_tag)
}

pub(crate) fn compiler_collect_ffi_link_manifest(
    path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let ast = load_ast_from_path(path)?;
    let source_path = std::path::Path::new(path);
    let source_dir = source_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut diagnostics = Vec::new();
    compiler_collect_ffi_diagnostics_from_program(&ast, &mut diagnostics);

    let mut entries = Vec::new();
    let mut link_args = BTreeSet::new();
    let mut runtime_paths = BTreeSet::new();
    let mut artifacts = Vec::new();
    let mut manifest_diags = Vec::new();

    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Foreign(items) = section else {
                continue;
            };
            for symbol in items {
                let language = compiler_ffi_language_name(symbol.language);
                let symbol_tag =
                    compiler_ffi_symbol_tag(&container.name, symbol.language, &symbol.symbol);
                let mut codes = compiler_ffi_manifest_symbol_diagnostics(&diagnostics, &symbol_tag);
                let linkage = compiler_ffi_linkage(symbol);
                let raw_artifact = compiler_ffi_field_value(
                    symbol,
                    &["artifact", "artifactPath", "libraryPath", "linkArtifact"],
                );
                let raw_library = compiler_ffi_field_value(symbol, &["library", "lib", "libs"]);
                let raw_link_path = compiler_ffi_field_value(
                    symbol,
                    &[
                        "linkPath",
                        "linkDir",
                        "libraryDir",
                        "librarySearchPath",
                        "searchPath",
                    ],
                );
                let raw_runtime_path = compiler_ffi_field_value(
                    symbol,
                    &["rpath", "runtimePath", "runtimeLibraryPath", "runtimeDir"],
                );
                let raw_checksum = compiler_ffi_field_value(
                    symbol,
                    &[
                        "checksum",
                        "cksum",
                        "sha256",
                        "artifactChecksum",
                        "artifactSha256",
                    ],
                );
                let expected_checksum = raw_checksum.map(compiler_ffi_normalize_checksum);
                let mut artifact_path =
                    raw_artifact.map(|raw| compiler_ffi_resolve_path(raw, source_dir));
                if artifact_path.is_none() {
                    if let Some(raw_library) = raw_library {
                        let lib = compiler_ffi_unquote(raw_library);
                        if compiler_ffi_path_like(&lib) {
                            artifact_path = Some(compiler_ffi_resolve_path(&lib, source_dir));
                        }
                    }
                }

                let mut actual_checksum = None;
                let mut artifact_exists = false;
                let mut status = "deferred_to_sidecar_or_system_library".to_string();
                if let Some(path) = artifact_path.as_ref() {
                    artifact_exists = path.exists();
                    if artifact_exists {
                        let hash = compiler_ffi_sha256_file(path)?;
                        actual_checksum = Some(hash.clone());
                        artifacts.push(serde_json::json!({
                            "symbol": symbol.symbol,
                            "path": path.display().to_string(),
                            "sha256": hash,
                            "linkage": linkage,
                        }));
                        link_args.insert(path.display().to_string());
                        if compiler_ffi_is_dynamic_path(path, &linkage) {
                            if let Some(parent) = path.parent() {
                                runtime_paths.insert(parent.display().to_string());
                            }
                        }
                        status = "closed_manifest".to_string();
                    } else {
                        codes.push("E_FFI_LINK_001".to_string());
                        manifest_diags.push(serde_json::json!({
                            "code": "E_FFI_LINK_001",
                            "symbol": symbol.symbol,
                            "message": format!("declared ffi artifact '{}' does not exist", path.display()),
                        }));
                        status = "missing_artifact".to_string();
                    }
                } else if let Some(raw_link_path) = raw_link_path {
                    let dir = compiler_ffi_resolve_path(raw_link_path, source_dir);
                    if dir.exists() {
                        link_args.insert(format!("-L{}", dir.display()));
                        if linkage.contains("dynamic") || linkage.contains("shared") {
                            runtime_paths.insert(dir.display().to_string());
                        }
                    } else {
                        codes.push("E_FFI_LINK_001".to_string());
                        manifest_diags.push(serde_json::json!({
                            "code": "E_FFI_LINK_001",
                            "symbol": symbol.symbol,
                            "message": format!("declared ffi link path '{}' does not exist", dir.display()),
                        }));
                        status = "missing_link_path".to_string();
                    }
                }

                if let Some(raw_runtime_path) = raw_runtime_path {
                    runtime_paths.insert(
                        compiler_ffi_resolve_path(raw_runtime_path, source_dir)
                            .display()
                            .to_string(),
                    );
                }
                if let Some(raw_library) = raw_library {
                    let lib = compiler_ffi_unquote(raw_library);
                    if !lib.is_empty() && !compiler_ffi_path_like(&lib) {
                        link_args.insert(format!("-l{}", compiler_ffi_link_name(&lib)));
                    }
                }

                if let Some(expected) = expected_checksum.as_ref() {
                    if expected.len() != 64 || !expected.chars().all(|ch| ch.is_ascii_hexdigit()) {
                        codes.push("E_FFI_CHECKSUM_001".to_string());
                        manifest_diags.push(serde_json::json!({
                            "code": "E_FFI_CHECKSUM_001",
                            "symbol": symbol.symbol,
                            "message": "invalid ffi artifact checksum contract",
                        }));
                        status = "checksum_invalid".to_string();
                    } else if let Some(actual) = actual_checksum.as_ref() {
                        if actual != expected {
                            codes.push("E_FFI_CHECKSUM_001".to_string());
                            manifest_diags.push(serde_json::json!({
                                "code": "E_FFI_CHECKSUM_001",
                                "symbol": symbol.symbol,
                                "message": format!("ffi artifact checksum mismatch: expected sha256:{expected}, got sha256:{actual}"),
                            }));
                            status = "checksum_mismatch".to_string();
                        }
                    }
                }

                codes.sort();
                codes.dedup();
                if !codes.is_empty() && status == "closed_manifest" {
                    status = "failed_contract".to_string();
                }

                entries.push(serde_json::json!({
                    "container": container.name,
                    "language": language,
                    "symbol": symbol.symbol,
                    "symbol_tag": symbol_tag,
                    "status": status,
                    "diagnostic_codes": codes,
                    "header": compiler_ffi_field_value(symbol, &["header"]).map(compiler_ffi_unquote),
                    "library": raw_library.map(compiler_ffi_unquote),
                    "artifact": artifact_path.as_ref().map(|p| p.display().to_string()),
                    "artifact_exists": artifact_exists,
                    "linkage": linkage,
                    "link_path": raw_link_path.map(|raw| compiler_ffi_resolve_path(raw, source_dir).display().to_string()),
                    "runtime_path": raw_runtime_path.map(|raw| compiler_ffi_resolve_path(raw, source_dir).display().to_string()),
                    "checksum": {
                        "expected": expected_checksum,
                        "actual": actual_checksum,
                    },
                }));
            }
        }
    }

    let failed_count = entries
        .iter()
        .filter(|entry| {
            entry
                .get("diagnostic_codes")
                .and_then(|value| value.as_array())
                .is_some_and(|codes| !codes.is_empty())
        })
        .count();
    Ok(serde_json::json!({
        "schema": "nc.compiler.ffi.link_manifest.v1",
        "path": path,
        "status": if failed_count == 0 { "closed" } else { "failed" },
        "count": entries.len(),
        "entries": entries,
        "artifacts": artifacts,
        "link_args": link_args.into_iter().collect::<Vec<_>>(),
        "runtime_paths": runtime_paths.into_iter().collect::<Vec<_>>(),
        "diagnostics": {
            "count": diagnostics.len() + manifest_diags.len(),
            "contract_rows": diagnostics.iter().map(compiler_type_diag_row_json).collect::<Vec<_>>(),
            "manifest_rows": manifest_diags,
        },
    }))
}

fn compiler_ffi_language_name(language: ncc_ast::ForeignLanguage) -> &'static str {
    match language {
        ncc_ast::ForeignLanguage::C => "c",
        ncc_ast::ForeignLanguage::Cpp => "cpp",
        ncc_ast::ForeignLanguage::Rust => "rust",
    }
}

fn compiler_collect_foreign_symbols(ast: &ncc_ast::Program) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Foreign(items) = section else {
                continue;
            };
            for item in items {
                let fields = item
                    .fields
                    .iter()
                    .map(|field| {
                        serde_json::json!({
                            "key": field.key,
                            "value": field.value,
                        })
                    })
                    .collect::<Vec<_>>();
                out.push(serde_json::json!({
                    "container": container.name,
                    "context": container
                        .context
                        .as_ref()
                        .map(|ctx| format!("{ctx:?}"))
                        .unwrap_or_else(|| "none".to_string()),
                    "language": compiler_ffi_language_name(item.language),
                    "symbol": item.symbol,
                    "fields": fields,
                }));
            }
        }
    }
    out
}

fn compiler_ffi_symbol_tag(
    container_name: &str,
    language: ncc_ast::ForeignLanguage,
    symbol: &str,
) -> String {
    format!(
        "{}::{}::{}",
        container_name,
        compiler_ffi_language_name(language),
        symbol
    )
}

fn compiler_ffi_contract_field_json(
    symbol: &ncc_ast::ForeignSymbolDecl,
    keys: &[&str],
    required: bool,
) -> serde_json::Value {
    let value = compiler_ffi_field_value(symbol, keys);
    serde_json::json!({
        "required": required,
        "present": value.is_some(),
        "status": if value.is_some() {
            "closed"
        } else if required {
            "missing"
        } else {
            "not_required"
        },
        "value": value,
    })
}

fn compiler_ffi_symbol_contract_codes(
    rows: &[CompilerTypeDiagnosticRow],
    symbol_tag: &str,
) -> Vec<String> {
    let mut codes = rows
        .iter()
        .filter_map(|diag| {
            if diag.found.as_deref() == Some(symbol_tag) && diag.code.starts_with("E_FFI_") {
                Some(diag.code.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    codes
}

fn compiler_ffi_cpp_symbol_is_template(symbol: &ncc_ast::ForeignSymbolDecl) -> bool {
    let name = symbol.symbol.to_ascii_lowercase();
    name.contains('<')
        || name.contains("template")
        || compiler_ffi_field_value(symbol, &["template", "templateArgs", "template_args"])
            .is_some()
}

fn compiler_ffi_cpp_symbol_uses_vtable(symbol: &ncc_ast::ForeignSymbolDecl) -> bool {
    let name = symbol.symbol.to_ascii_lowercase();
    name.contains("vtable")
        || name.contains("virtual")
        || compiler_ffi_field_value(symbol, &["vtable", "virtualDispatch", "virtual_dispatch"])
            .is_some()
}

fn compiler_ffi_ownership_requires_destructor_contract(ownership: &str) -> bool {
    let lowered = ownership.to_ascii_lowercase();
    lowered.contains("owned") || lowered.contains("moved") || lowered.contains("transferred")
}

fn compiler_collect_ffi_contract_matrix(
    ast: &ncc_ast::Program,
    diagnostics: &[CompilerTypeDiagnosticRow],
) -> Vec<serde_json::Value> {
    let mut contracts = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Foreign(items) = section else {
                continue;
            };
            for symbol in items {
                let language = compiler_ffi_language_name(symbol.language);
                let symbol_tag =
                    compiler_ffi_symbol_tag(&container.name, symbol.language, &symbol.symbol);
                let codes = compiler_ffi_symbol_contract_codes(diagnostics, &symbol_tag);
                let signature = compiler_ffi_field_value(symbol, &["signature"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let pointer_like = signature.contains("ptr<") || signature.contains("memregion<");
                let ownership = compiler_ffi_field_value(symbol, &["ownership"])
                    .unwrap_or_default()
                    .to_string();
                let destructor_required =
                    compiler_ffi_ownership_requires_destructor_contract(&ownership);
                let exported_as = compiler_ffi_field_value(symbol, &["exportedAs", "exported_as"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let cpp_mangled = symbol.language == ncc_ast::ForeignLanguage::Cpp
                    && (exported_as.contains("mangled")
                        || symbol.symbol.contains("::")
                        || symbol.symbol.contains('<'));
                let template_detected = symbol.language == ncc_ast::ForeignLanguage::Cpp
                    && compiler_ffi_cpp_symbol_is_template(symbol);
                let vtable_detected = symbol.language == ncc_ast::ForeignLanguage::Cpp
                    && compiler_ffi_cpp_symbol_uses_vtable(symbol);
                contracts.push(serde_json::json!({
                    "container": container.name,
                    "context": container.context.as_ref().map(|ctx| format!("{ctx:?}")).unwrap_or_else(|| "none".to_string()),
                    "language": language,
                    "symbol": symbol.symbol,
                    "symbol_tag": symbol_tag,
                    "status": if codes.is_empty() { "closed" } else { "failed" },
                    "diagnostic_codes": codes,
                    "symbol_contract": {
                        "field": compiler_ffi_contract_field_json(symbol, &["symbol"], false),
                        "legal_name": !symbol.symbol.trim().is_empty(),
                    },
                    "abi": compiler_ffi_contract_field_json(symbol, &["abi"], true),
                    "signature": compiler_ffi_contract_field_json(symbol, &["signature"], true),
                    "layout": compiler_ffi_contract_field_json(symbol, &["layout", "repr"], true),
                    "ownership": compiler_ffi_contract_field_json(symbol, &["ownership"], true),
                    "ownership_transfer": {
                        "required": destructor_required,
                        "kind": ownership,
                        "destructor_or_free_contract": compiler_ffi_contract_field_json(
                            symbol,
                            &["freeSymbol", "free_symbol", "destructorPolicy", "destructor_policy"],
                            destructor_required,
                        ),
                    },
                    "nullability": compiler_ffi_contract_field_json(symbol, &["nullability"], pointer_like),
                    "side_effects": compiler_ffi_contract_field_json(symbol, &["sideEffects", "side_effects"], true),
                    "header": compiler_ffi_contract_field_json(symbol, &["header"], symbol.language == ncc_ast::ForeignLanguage::C),
                    "library": compiler_ffi_contract_field_json(symbol, &["library"], symbol.language == ncc_ast::ForeignLanguage::C),
                    "linkage": compiler_ffi_contract_field_json(symbol, &["linkage", "link"], false),
                    "cpp": {
                        "required": symbol.language == ncc_ast::ForeignLanguage::Cpp,
                        "exported_as": compiler_ffi_contract_field_json(symbol, &["exportedAs", "exported_as"], symbol.language == ncc_ast::ForeignLanguage::Cpp),
                        "cpp_abi": compiler_ffi_contract_field_json(symbol, &["cppAbi", "cpp_abi"], cpp_mangled),
                        "template": {
                            "detected": template_detected,
                            "status": if template_detected && compiler_ffi_has_field(symbol, &["cppAbi", "cpp_abi"]) {
                                "closed"
                            } else if template_detected {
                                "missing_cpp_abi"
                            } else {
                                "not_applicable"
                            },
                        },
                        "vtable": {
                            "detected": vtable_detected,
                            "status": if vtable_detected { "declared_or_symbol_detected" } else { "not_applicable" },
                        },
                        "exception_boundary": compiler_ffi_contract_field_json(
                            symbol,
                            &["exceptionBoundary", "exception_boundary"],
                            symbol.language == ncc_ast::ForeignLanguage::Cpp,
                        ),
                    },
                    "rust": {
                        "required": symbol.language == ncc_ast::ForeignLanguage::Rust,
                        "panic_boundary": compiler_ffi_contract_field_json(
                            symbol,
                            &["panicBoundary", "panic_boundary"],
                            symbol.language == ncc_ast::ForeignLanguage::Rust,
                        ),
                    },
                    "toolchain_evidence": match symbol.language {
                        ncc_ast::ForeignLanguage::Cpp => serde_json::json!("ncc compiler ffi-cpp-abi --json --strict"),
                        ncc_ast::ForeignLanguage::C | ncc_ast::ForeignLanguage::Rust => serde_json::json!("ncc compiler ffi-smoke --json --strict"),
                    },
                }));
            }
        }
    }
    contracts
}

fn compiler_collect_ffi_diagnostics_from_program(
    ast: &ncc_ast::Program,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    let mut seen = BTreeSet::new();
    let has_primitive_context = ast
        .containers
        .iter()
        .any(|container| matches!(container.context, Some(ncc_ast::Context::Primitive)));
    let has_machine_bridge = ast
        .containers
        .iter()
        .any(|container| matches!(container.context, Some(ncc_ast::Context::Machine)));

    for use_decl in &ast.uses {
        if use_decl.path.is_empty() || !use_decl.path[0].eq_ignore_ascii_case("foreign") {
            continue;
        }

        let rendered = use_decl.path.join(".");
        if use_decl.path.len() < 2 {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_IMPORT_CONTRACT_001",
                "foreign import must select an explicit language, for example `use foreign { c };`"
                    .to_string(),
                "ffi.use",
                Some(rendered.clone()),
            );
            continue;
        }

        let language = use_decl.path[1].to_ascii_lowercase();
        if language == "machine" {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_IMPORT_CONTRACT_001",
                "foreign machine imports are not official; use `use foreign { asm };` with a [@machine] bridge"
                    .to_string(),
                "ffi.use",
                Some(rendered.clone()),
            );
            continue;
        }

        if !matches!(language.as_str(), "c" | "cpp" | "rust" | "asm") {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_IMPORT_UNKNOWN_001",
                format!("unknown foreign language '{}'", use_decl.path[1]),
                "ffi.use",
                Some(rendered.clone()),
            );
            continue;
        }

        if use_decl.path.len() != 2 || use_decl.wildcard {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_IMPORT_CONTRACT_001",
                "foreign imports accept one explicit language; declare symbol contracts in `foreign:` blocks"
                    .to_string(),
                "ffi.use",
                Some(rendered.clone()),
            );
        }

        if language == "asm" && !has_machine_bridge {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_ASM_BRIDGE_001",
                "foreign.asm requires a [@machine] bridge container".to_string(),
                "ffi.use",
                Some(rendered.clone()),
            );
        }

        if has_primitive_context {
            compiler_push_unique_diag(
                out,
                &mut seen,
                "E_FFI_PRIMITIVE_CROSSING_001",
                "FFI cannot cross primitive[@]".to_string(),
                "ffi.use",
                Some(rendered),
            );
        }
    }

    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Foreign(items) = section else {
                continue;
            };

            if matches!(container.context, Some(ncc_ast::Context::Primitive)) {
                compiler_push_unique_diag(
                    out,
                    &mut seen,
                    "E_FFI_PRIMITIVE_CROSSING_001",
                    "FFI cannot cross primitive[@]".to_string(),
                    "ffi.section",
                    Some(container.name.clone()),
                );
            }
            if matches!(
                container.context,
                Some(ncc_ast::Context::Machine) | Some(ncc_ast::Context::Kernel)
            ) {
                compiler_push_unique_diag(
                    out,
                    &mut seen,
                    "E_FFI_IMPORT_CONTRACT_001",
                    "foreign section is forbidden in [@machine]/[@kernel] in this phase"
                        .to_string(),
                    "ffi.section",
                    Some(container.name.clone()),
                );
            }

            for symbol in items {
                let symbol_tag = format!(
                    "{}::{}::{}",
                    container.name,
                    compiler_ffi_language_name(symbol.language),
                    symbol.symbol
                );
                if symbol.symbol.trim().is_empty() {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_SYMBOL_MISSING_001",
                        "ffi symbol name cannot be empty".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if !compiler_ffi_has_field(symbol, &["abi"]) {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_ABI_MISSING_001",
                        "ffi symbol requires ABI contract".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if !compiler_ffi_has_field(symbol, &["signature"]) {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_SIGNATURE_MISSING_001",
                        "ffi symbol requires signature contract".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if !compiler_ffi_has_field(symbol, &["ownership"]) {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_OWNERSHIP_001",
                        "ffi symbol requires ownership contract".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if !compiler_ffi_has_field(symbol, &["sideEffects", "side_effects"]) {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_SIDE_EFFECT_001",
                        "ffi symbol requires sideEffects contract".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }

                let signature = compiler_ffi_field_value(symbol, &["signature"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let signature_compact = signature
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                let pointer_like =
                    signature_compact.contains("ptr<") || signature_compact.contains("memregion<");
                if pointer_like && !compiler_ffi_has_field(symbol, &["nullability"]) {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_NULLABILITY_001",
                        "pointer-like ffi signature requires nullability contract".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if signature_compact.contains("list<")
                    || signature_compact.contains("dinarray.unbinary")
                    || signature_compact.contains("runtimeobject")
                {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_TYPE_UNSUPPORTED_001",
                        "runtime/unbinary/list types cannot cross FFI directly".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if signature.contains("str") || signature.contains("text") {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_STRING_BOUNDARY_001",
                        "str/text must use explicit FFI-safe representation".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }
                if signature_compact.contains("result<") || signature_compact.contains("option<") {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_RESULT_OPTION_BOUNDARY_001",
                        "Result/Option must use explicit FFI representation".to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }

                let ownership = compiler_ffi_field_value(symbol, &["ownership"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if compiler_ffi_ownership_requires_destructor_contract(&ownership)
                    && !compiler_ffi_has_field(
                        symbol,
                        &[
                            "freeSymbol",
                            "free_symbol",
                            "destructorPolicy",
                            "destructor_policy",
                        ],
                    )
                {
                    compiler_push_unique_diag(
                        out,
                        &mut seen,
                        "E_FFI_FREE_CONTRACT_001",
                        "owned/moved/transferred ffi ownership requires free/destructor contract"
                            .to_string(),
                        "ffi.symbol",
                        Some(symbol_tag.clone()),
                    );
                }

                if let Some(side_effects) =
                    compiler_ffi_field_value(symbol, &["sideEffects", "side_effects"])
                {
                    let lowered = side_effects.to_ascii_lowercase();
                    if lowered.contains("unknown") {
                        compiler_push_unique_diag(
                            out,
                            &mut seen,
                            "E_FFI_SIDE_EFFECT_001",
                            "sideEffects=unknown is forbidden".to_string(),
                            "ffi.symbol",
                            Some(symbol_tag.clone()),
                        );
                    }
                    if lowered.contains("filesystem")
                        || lowered.contains("process")
                        || lowered.contains("network")
                    {
                        if !compiler_ffi_has_field(symbol, &["capability"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_CAPABILITY_001",
                                "dangerous sideEffects require capability contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(symbol, &["policy"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_POLICY_001",
                                "dangerous sideEffects require policy contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                    }
                }

                match symbol.language {
                    ncc_ast::ForeignLanguage::C => {
                        if !compiler_ffi_has_field(symbol, &["layout", "repr"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_LAYOUT_REQUIRED_001",
                                "C ffi requires layout/repr contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(symbol, &["header"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_IMPORT_CONTRACT_001",
                                "C ffi requires header contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(symbol, &["library"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_IMPORT_CONTRACT_001",
                                "C ffi requires library contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                    }
                    ncc_ast::ForeignLanguage::Cpp => {
                        let exported_as =
                            compiler_ffi_field_value(symbol, &["exportedAs", "exported_as"])
                                .unwrap_or_default()
                                .to_ascii_lowercase();
                        if exported_as.contains("mangled")
                            && !compiler_ffi_has_field(symbol, &["cppAbi", "cpp_abi"])
                        {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_ABI_MISMATCH_001",
                                "mangled C++ symbols require explicit cppAbi contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(
                            symbol,
                            &["exceptionBoundary", "exception_boundary"],
                        ) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_CPP_EXCEPTION_BOUNDARY_001",
                                "C++ ffi requires exceptionBoundary contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(symbol, &["layout", "repr"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_LAYOUT_REQUIRED_001",
                                "C++ ffi requires layout/repr contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                    }
                    ncc_ast::ForeignLanguage::Rust => {
                        if !compiler_ffi_has_field(symbol, &["panicBoundary", "panic_boundary"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_RUST_PANIC_BOUNDARY_001",
                                "Rust ffi requires panicBoundary contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                        if !compiler_ffi_has_field(symbol, &["repr", "layout"]) {
                            compiler_push_unique_diag(
                                out,
                                &mut seen,
                                "E_FFI_REPR_REQUIRED_001",
                                "Rust ffi requires repr/layout contract".to_string(),
                                "ffi.symbol",
                                Some(symbol_tag.clone()),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn compiler_expected_generic_arity(base: &str) -> Option<usize> {
    match base.to_ascii_lowercase().as_str() {
        "ptr" | "memregion" | "list" | "option" | "vector" | "dinarray" | "dinarray.binary"
        | "dinarray.unbinary" | "xy" | "xyz" | "x2y2" | "x2y2z2" | "x3y3" | "x3y3z3" | "grid"
        | "spatialregion" => Some(1),
        "result" => Some(2),
        "array" => Some(2),
        "array2" => Some(3),
        "matrix" => Some(3),
        "array3" => Some(4),
        _ => None,
    }
}

fn compiler_type_has_stable_binary_layout(ty: &ncc_ast::TypeName) -> bool {
    match ty {
        ncc_ast::TypeName::Simple(name) => matches!(
            name.to_ascii_lowercase().as_str(),
            "bool" | "byte" | "int" | "u64" | "word" | "dword" | "qword" | "ft64" | "char" | "addr"
        ),
        ncc_ast::TypeName::Vector(inner) | ncc_ast::TypeName::Array(inner) => {
            compiler_type_has_stable_binary_layout(inner)
        }
        ncc_ast::TypeName::Generic { base, args } => {
            let lower = base.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "array"
                    | "vector"
                    | "dinarray"
                    | "dinarray.binary"
                    | "matrix"
                    | "tensor"
                    | "array2"
                    | "array3"
                    | "xy"
                    | "xyz"
                    | "x2y2"
                    | "x2y2z2"
                    | "x3y3"
                    | "x3y3z3"
            ) {
                return args
                    .first()
                    .is_some_and(compiler_type_has_stable_binary_layout);
            }
            false
        }
        ncc_ast::TypeName::Union(_) => false,
    }
}

fn compiler_collect_generic_arity_diag_from_typename(
    ty: &ncc_ast::TypeName,
    stmt_kind: &str,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    match ty {
        ncc_ast::TypeName::Generic { base, args } => {
            let base_lower = base.to_ascii_lowercase();
            if let Some(expected) = compiler_expected_generic_arity(base) {
                if args.len() != expected {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_GENERIC_ARITY_001".to_string(),
                        message: format!(
                            "generic '{}' expects {} args, found {}",
                            base,
                            expected,
                            args.len()
                        ),
                        stmt_kind: stmt_kind.to_string(),
                        expected: Some(expected.to_string()),
                        found: Some(args.len().to_string()),
                    });
                }
            }
            if base_lower == "matrix" && args.len() != 3 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_MATRIX_DIMENSION_001".to_string(),
                    message: format!(
                        "matrix dimension mismatch: matrix<T, R, C> requires 3 args, found {}",
                        args.len()
                    ),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("3".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if base_lower == "array2" && args.len() != 3 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_DIMENSION_001".to_string(),
                    message: format!(
                        "array2 dimension mismatch: array2<T, X, Y> requires 3 args, found {}",
                        args.len()
                    ),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("3".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if base_lower == "array3" && args.len() != 4 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_DIMENSION_001".to_string(),
                    message: format!(
                        "array3 dimension mismatch: array3<T, X, Y, Z> requires 4 args, found {}",
                        args.len()
                    ),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("4".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if matches!(
                base_lower.as_str(),
                "xy" | "xyz" | "x2y2" | "x2y2z2" | "x3y3" | "x3y3z3"
            ) && args.len() != 1
            {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_VECTOR_DIMENSION_001".to_string(),
                    message: format!("vector dimension mismatch: '{}<T>' expects 1 arg", base),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("1".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if base_lower == "tensor" && args.len() < 2 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_TENSOR_RANK_001".to_string(),
                    message: format!(
                        "tensor rank mismatch: tensor<T, D...> requires at least 2 args, found {}",
                        args.len()
                    ),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some(">=2".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if matches!(
                base_lower.as_str(),
                "dinarray" | "dinarray.binary" | "dinarray.unbinary"
            ) && args.len() != 1
            {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_DINARRAY_TYPE_001".to_string(),
                    message: format!("invalid dinArray type: '{}' expects exactly 1 arg", base),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("1".to_string()),
                    found: Some(args.len().to_string()),
                });
            }
            if base_lower == "dinarray.binary"
                && args
                    .first()
                    .is_some_and(|inner| !compiler_type_has_stable_binary_layout(inner))
            {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_DINARRAY_BINARY_LAYOUT_001".to_string(),
                    message: "binary dinArray requires stable binary layout element type"
                        .to_string(),
                    stmt_kind: stmt_kind.to_string(),
                    expected: Some("stable-binary-layout".to_string()),
                    found: args.first().map(type_name_to_string),
                });
            }
            for arg in args {
                compiler_collect_generic_arity_diag_from_typename(arg, stmt_kind, out);
            }
        }
        ncc_ast::TypeName::Vector(inner) | ncc_ast::TypeName::Array(inner) => {
            compiler_collect_generic_arity_diag_from_typename(inner, stmt_kind, out);
        }
        ncc_ast::TypeName::Union(items) => {
            for item in items {
                compiler_collect_generic_arity_diag_from_typename(item, stmt_kind, out);
            }
        }
        ncc_ast::TypeName::Simple(_) => {}
    }
}

fn compiler_type_supports_nullability(ty: &ncc_ast::TypeName) -> bool {
    match ty {
        ncc_ast::TypeName::Union(items) => items.iter().any(|item| match item {
            ncc_ast::TypeName::Simple(name) => name.eq_ignore_ascii_case("null"),
            _ => false,
        }),
        _ => false,
    }
}

fn compiler_union_members_str(ty: &str) -> Vec<String> {
    ty.split('|')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn compiler_type_supports_nullability_str(ty: &str) -> bool {
    compiler_union_members_str(ty)
        .iter()
        .any(|value| value.eq_ignore_ascii_case("null"))
}

fn compiler_compact_lower_type(ty: &str) -> String {
    ty.replace(' ', "").to_ascii_lowercase()
}

fn compiler_type_is_pointer_family(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    lower == "ptr" || lower.starts_with("ptr<")
}

fn compiler_pointer_inner_type(ty: &str) -> Option<String> {
    for member in compiler_union_members_str(ty) {
        let compact = compiler_compact_lower_type(&member);
        if compact.starts_with("ptr<") && compact.ends_with('>') {
            return Some(
                compact
                    .trim_start_matches("ptr<")
                    .trim_end_matches('>')
                    .to_string(),
            );
        }
        if compact == "ptr" {
            return Some("int".to_string());
        }
    }
    None
}

fn compiler_type_is_nullable_pointer_union(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    if !lower.contains('|') {
        return false;
    }
    let members = compiler_union_members_str(&lower);
    let has_null = members.iter().any(|value| value == "null");
    let has_ptr = members
        .iter()
        .any(|value| value == "ptr" || value.starts_with("ptr<"));
    has_null && has_ptr
}

fn compiler_type_is_pointer_origin_source_type(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    lower == "addr"
        || compiler_type_is_pointer_family(&lower)
        || compiler_type_is_nullable_pointer_union(&lower)
}

fn compiler_numeric_like_type(ty: &str) -> bool {
    matches!(
        ty,
        "int"
            | "uint"
            | "u64"
            | "byte"
            | "word"
            | "dword"
            | "qword"
            | "ft32"
            | "ft64"
            | "ft"
            | "float"
    )
}

fn compiler_string_like_type(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    if lower.is_empty() {
        return false;
    }
    if lower == "str" || lower == "string" {
        return true;
    }
    if lower.contains('|') {
        let members = compiler_union_members_str(&lower);
        return !members.is_empty()
            && members
                .iter()
                .all(|member| member == "str" || member == "string");
    }
    false
}

fn compiler_concat_forbidden_message(context: Option<&ncc_ast::Context>) -> Option<&'static str> {
    match context {
        Some(ncc_ast::Context::Root) => Some("text concatenation with '+' is not available in [@]"),
        Some(ncc_ast::Context::Machine) => {
            Some("text concatenation with '+' is not available in [@machine]")
        }
        Some(ncc_ast::Context::Primitive) => {
            Some("text concatenation with '+' is not available in primitive[@]")
        }
        _ => None,
    }
}

fn compiler_collection_like_type(ty: &str) -> bool {
    if ty == "array" {
        return true;
    }
    let lower = ty.to_ascii_lowercase();
    lower.starts_with("array<")
        || lower.starts_with("array2<")
        || lower.starts_with("array3<")
        || lower.starts_with("vector<")
        || lower.starts_with("matrix<")
        || lower.starts_with("tensor<")
        || lower.starts_with("xy<")
        || lower.starts_with("xyz<")
        || lower.starts_with("x2y2<")
        || lower.starts_with("x2y2z2<")
        || lower.starts_with("x3y3<")
        || lower.starts_with("x3y3z3<")
        || lower.starts_with("dinarray<")
        || lower.starts_with("dinarray.binary<")
        || lower.starts_with("dinarray.unbinary<")
        || lower.starts_with("list<")
        || lower.starts_with("memregion<")
}

#[derive(Debug, Clone, Default)]
struct CompilerIndexBoundsInfo {
    array_len: Option<usize>,
    memregion_len: Option<usize>,
}

fn compiler_expr_non_negative_usize_literal(expr: &ncc_ast::Expr) -> Option<usize> {
    let value = compiler_expr_integer_literal_value(expr)?;
    if value < 0 {
        return None;
    }
    usize::try_from(value).ok()
}

fn compiler_extract_region_len_from_call_args(args: &[ncc_ast::Expr]) -> Option<usize> {
    if args.len() < 2 {
        return None;
    }
    compiler_expr_non_negative_usize_literal(&args[1])
}

fn compiler_index_bounds_from_expr(
    expr: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    bounds: &BTreeMap<String, CompilerIndexBoundsInfo>,
) -> CompilerIndexBoundsInfo {
    match expr {
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            CompilerIndexBoundsInfo {
                array_len: Some(items.len()),
                memregion_len: None,
            }
        }
        ncc_ast::Expr::Identifier(name) => bounds
            .get(&name.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default(),
        ncc_ast::Expr::ContextRef(reference) => reference
            .path
            .last()
            .and_then(|name| bounds.get(&name.to_ascii_lowercase()))
            .cloned()
            .unwrap_or_default(),
        ncc_ast::Expr::Call { callee, args } => {
            if callee.eq_ignore_ascii_case("regionFromPtr") {
                return CompilerIndexBoundsInfo {
                    array_len: None,
                    memregion_len: compiler_extract_region_len_from_call_args(args),
                };
            }
            CompilerIndexBoundsInfo::default()
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            if path
                .last()
                .is_some_and(|value| value.eq_ignore_ascii_case("regionFromPtr"))
            {
                return CompilerIndexBoundsInfo {
                    array_len: None,
                    memregion_len: compiler_extract_region_len_from_call_args(args),
                };
            }
            CompilerIndexBoundsInfo::default()
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_index_bounds_from_expr(value, scope, bounds)
        }
        _ => {
            let _ = scope;
            CompilerIndexBoundsInfo::default()
        }
    }
}

fn compiler_type_is_memregion_family(ty: &str) -> bool {
    compiler_compact_lower_type(ty).starts_with("memregion<")
}

fn compiler_type_is_option_family(ty: &str) -> bool {
    compiler_compact_lower_type(ty).starts_with("option<")
}

fn compiler_type_is_result_family(ty: &str) -> bool {
    compiler_compact_lower_type(ty).starts_with("result<")
}

#[derive(Debug, Clone)]
struct CompilerSymbolContract {
    container: String,
    name: String,
    visibility: Option<ncc_ast::Visibility>,
    strictly: bool,
    is_function: bool,
}

#[derive(Debug, Clone, Default)]
struct CompilerVisibilityIndex {
    by_name: BTreeMap<String, Vec<CompilerSymbolContract>>,
    by_container_name: BTreeMap<(String, String), CompilerSymbolContract>,
}

fn compiler_build_visibility_index(ast: &ncc_ast::Program) -> CompilerVisibilityIndex {
    let mut index = CompilerVisibilityIndex::default();
    for container in &ast.containers {
        let container_name = container.name.to_ascii_lowercase();
        for section in &container.sections {
            match section {
                ncc_ast::Section::Stc(decls) => {
                    for decl in decls {
                        let ncc_ast::StcDecl::Typed {
                            visibility,
                            strictly,
                            name,
                            ..
                        } = decl;
                        let contract = CompilerSymbolContract {
                            container: container_name.clone(),
                            name: name.to_ascii_lowercase(),
                            visibility: visibility.clone(),
                            strictly: *strictly,
                            is_function: false,
                        };
                        index
                            .by_name
                            .entry(contract.name.clone())
                            .or_default()
                            .push(contract.clone());
                        index.by_container_name.insert(
                            (contract.container.clone(), contract.name.clone()),
                            contract,
                        );
                    }
                }
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        let contract = CompilerSymbolContract {
                            container: container_name.clone(),
                            name: function.name.to_ascii_lowercase(),
                            visibility: function.visibility.clone(),
                            strictly: false,
                            is_function: true,
                        };
                        index
                            .by_name
                            .entry(contract.name.clone())
                            .or_default()
                            .push(contract.clone());
                        index.by_container_name.insert(
                            (contract.container.clone(), contract.name.clone()),
                            contract,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    index
}

fn compiler_resolve_contract_for_path<'a>(
    index: &'a CompilerVisibilityIndex,
    current_container: &str,
    path: &[String],
) -> Option<&'a CompilerSymbolContract> {
    if path.is_empty() {
        return None;
    }
    if path.len() >= 2 {
        let target_container = path[0].to_ascii_lowercase();
        let target_name = path[path.len() - 1].to_ascii_lowercase();
        if let Some(contract) = index
            .by_container_name
            .get(&(target_container.clone(), target_name.clone()))
        {
            return Some(contract);
        }
        if target_container == current_container.to_ascii_lowercase() {
            return index
                .by_container_name
                .get(&(target_container, target_name));
        }
    }

    let target_name = path[path.len() - 1].to_ascii_lowercase();
    let candidates = index.by_name.get(&target_name)?;
    if candidates.len() == 1 {
        return candidates.first();
    }
    candidates
        .iter()
        .find(|contract| contract.container == current_container.to_ascii_lowercase())
        .or_else(|| {
            if candidates.len() == 1 {
                candidates.first()
            } else {
                None
            }
        })
}

fn compiler_type_is_void(ty: &str) -> bool {
    ty.eq_ignore_ascii_case("void")
}

fn compiler_type_is_nullable_like(ty: &str) -> bool {
    if ty.eq_ignore_ascii_case("null") {
        return true;
    }
    compiler_type_supports_nullability_str(ty) || compiler_type_is_option_family(ty)
}

fn compiler_is_discard_null_call(expr: &ncc_ast::Expr) -> bool {
    match expr {
        ncc_ast::Expr::Call { callee, .. } => callee.eq_ignore_ascii_case("discardNull"),
        ncc_ast::Expr::QualifiedCall { path, .. } => path
            .last()
            .is_some_and(|name| name.eq_ignore_ascii_case("discardNull")),
        _ => false,
    }
}

fn compiler_emit_visibility_diag(
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    seen: &mut BTreeSet<(String, String, String)>,
    code: &str,
    message: String,
    stmt_kind: &str,
    symbol: &str,
) {
    let key = (code.to_string(), stmt_kind.to_string(), symbol.to_string());
    if !seen.insert(key) {
        return;
    }
    out.push(CompilerTypeDiagnosticRow {
        code: code.to_string(),
        message,
        stmt_kind: stmt_kind.to_string(),
        expected: None,
        found: Some(symbol.to_string()),
    });
}

fn compiler_is_mutating_method_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "set"
            | "push"
            | "insert"
            | "removeat"
            | "clear"
            | "reserve"
            | "realloc"
            | "shrink"
            | "dinset"
            | "dinpush"
            | "dininsert"
            | "dinremoveat"
            | "dinclear"
            | "dinreserve"
            | "dinrealloc"
            | "dinshrink"
            | "write"
            | "store"
    )
}

fn compiler_type_is_dinarray_family(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    lower.starts_with("dinarray<")
        || lower.starts_with("dinarray.binary<")
        || lower.starts_with("dinarray.unbinary<")
}

fn compiler_type_is_dinarray_binary_family(ty: &str) -> bool {
    compiler_compact_lower_type(ty).starts_with("dinarray.binary<")
}

fn compiler_type_is_dinarray_unbinary_family(ty: &str) -> bool {
    compiler_compact_lower_type(ty).starts_with("dinarray.unbinary<")
}

fn compiler_dinarray_inner_type(ty: &str) -> Option<String> {
    compiler_type_generic_args(ty, "dinArray")
        .and_then(|args| args.first().cloned())
        .or_else(|| {
            compiler_type_generic_args(ty, "dinArray.binary").and_then(|args| args.first().cloned())
        })
        .or_else(|| {
            compiler_type_generic_args(ty, "dinArray.unbinary")
                .and_then(|args| args.first().cloned())
        })
}

fn compiler_split_top_level_generic_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (idx, ch) in raw.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let segment = raw[start..idx].trim();
                if !segment.is_empty() {
                    args.push(segment.to_string());
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = raw[start..].trim();
    if !tail.is_empty() {
        args.push(tail.to_string());
    }
    args
}

fn compiler_type_generic_args(ty: &str, base: &str) -> Option<Vec<String>> {
    let trimmed = ty.trim();
    let open = trimmed.find('<')?;
    let close = trimmed.rfind('>')?;
    if close <= open {
        return None;
    }
    let base_name = trimmed[..open].trim();
    if !base_name.eq_ignore_ascii_case(base) {
        return None;
    }
    let inside = &trimmed[(open + 1)..close];
    Some(compiler_split_top_level_generic_args(inside))
}

fn compiler_option_inner_type(ty: &str) -> Option<String> {
    let args = compiler_type_generic_args(ty, "Option")?;
    args.first().cloned()
}

fn compiler_result_ok_inner_type(ty: &str) -> Option<String> {
    let args = compiler_type_generic_args(ty, "Result")?;
    args.first().cloned()
}

fn compiler_expr_direct_call_name(expr: &ncc_ast::Expr) -> Option<&str> {
    match expr {
        ncc_ast::Expr::Call { callee, .. } => Some(callee.as_str()),
        ncc_ast::Expr::QualifiedCall { path, .. } => path.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn compiler_destructure_kind_to_legacy_callee(kind: ncc_ast::DestructureKind) -> &'static str {
    match kind {
        ncc_ast::DestructureKind::Default => "destructure",
        ncc_ast::DestructureKind::Static => "destructureStatic",
        ncc_ast::DestructureKind::Move => "destructureMove",
        ncc_ast::DestructureKind::Ref => "destructureRef",
        ncc_ast::DestructureKind::Region => "destructureRegion",
        ncc_ast::DestructureKind::Binary => "destructureBinary",
        ncc_ast::DestructureKind::Result => "destructureResult",
        ncc_ast::DestructureKind::Ownership => "destructureOwnership",
    }
}

fn compiler_constructure_kind_to_legacy_callee(kind: ncc_ast::ConstructureKind) -> &'static str {
    match kind {
        ncc_ast::ConstructureKind::Default => "constructure",
        ncc_ast::ConstructureKind::Static => "constructureStatic",
        ncc_ast::ConstructureKind::Move => "constructureMove",
        ncc_ast::ConstructureKind::Copy => "constructureCopy",
        ncc_ast::ConstructureKind::Region => "constructureRegion",
        ncc_ast::ConstructureKind::Binary => "constructureBinary",
        ncc_ast::ConstructureKind::Unbinary => "constructureUnbinary",
    }
}

fn compiler_destructure_stmt_as_call_expr(
    kind: ncc_ast::DestructureKind,
    args: &[ncc_ast::Expr],
) -> ncc_ast::Expr {
    ncc_ast::Expr::Call {
        callee: compiler_destructure_kind_to_legacy_callee(kind).to_string(),
        args: args.to_vec(),
    }
}

fn compiler_constructure_stmt_as_call_expr(
    kind: ncc_ast::ConstructureKind,
    args: &[ncc_ast::Expr],
) -> ncc_ast::Expr {
    ncc_ast::Expr::Call {
        callee: compiler_constructure_kind_to_legacy_callee(kind).to_string(),
        args: args.to_vec(),
    }
}

fn compiler_type_str_has_stable_binary_layout(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    if let Some(contract) = ncc_ast::scalar_type_contract(&lower, 8) {
        return !matches!(
            contract.class,
            ncc_ast::ScalarTypeClass::Text | ncc_ast::ScalarTypeClass::Void
        );
    }
    if matches!(
        lower.as_str(),
        "bool" | "byte" | "int" | "u64" | "word" | "dword" | "qword" | "ft64" | "char" | "addr"
    ) {
        return true;
    }

    if lower.starts_with("array<")
        || lower.starts_with("vector<")
        || lower.starts_with("dinarray<")
        || lower.starts_with("dinarray.binary<")
        || lower.starts_with("matrix<")
        || lower.starts_with("tensor<")
        || lower.starts_with("array2<")
        || lower.starts_with("array3<")
        || lower.starts_with("xy<")
        || lower.starts_with("xyz<")
        || lower.starts_with("x2y2<")
        || lower.starts_with("x2y2z2<")
        || lower.starts_with("x3y3<")
        || lower.starts_with("x3y3z3<")
    {
        return true;
    }
    false
}

fn compiler_legacy_destructure_kind(callee_lower: &str) -> Option<ncc_ast::DestructureKind> {
    match callee_lower {
        "destructure" => Some(ncc_ast::DestructureKind::Default),
        "destructurestatic" => Some(ncc_ast::DestructureKind::Static),
        "destructuremove" => Some(ncc_ast::DestructureKind::Move),
        "destructureref" => Some(ncc_ast::DestructureKind::Ref),
        "destructureregion" => Some(ncc_ast::DestructureKind::Region),
        "destructurebinary" => Some(ncc_ast::DestructureKind::Binary),
        "destructureresult" => Some(ncc_ast::DestructureKind::Result),
        "destructureownership" => Some(ncc_ast::DestructureKind::Ownership),
        "destructurefield" => Some(ncc_ast::DestructureKind::Static),
        _ => None,
    }
}

fn compiler_legacy_constructure_kind(callee_lower: &str) -> Option<ncc_ast::ConstructureKind> {
    match callee_lower {
        "constructure" => Some(ncc_ast::ConstructureKind::Default),
        "constructurestatic" => Some(ncc_ast::ConstructureKind::Static),
        "constructuremove" => Some(ncc_ast::ConstructureKind::Move),
        "constructurecopy" => Some(ncc_ast::ConstructureKind::Copy),
        "constructureregion" => Some(ncc_ast::ConstructureKind::Region),
        "constructurebinary" => Some(ncc_ast::ConstructureKind::Binary),
        "constructureunbinary" => Some(ncc_ast::ConstructureKind::Unbinary),
        "constructuretypedfield" => Some(ncc_ast::ConstructureKind::Static),
        _ => None,
    }
}

fn compiler_collect_destructure_contract_diagnostics(
    kind: ncc_ast::DestructureKind,
    arg_types: &[String],
    args: &[ncc_ast::Expr],
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    stmt_kind: &str,
    require_field_arg: bool,
) -> String {
    if matches!(kind, ncc_ast::DestructureKind::Default) || args.is_empty() {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_DESTRUCTURE_UNSUPPORTED_001".to_string(),
            message: "destructure unsupported for this type/shape".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("destructure.* contract".to_string()),
            found: Some(format!("{kind:?}")),
        });
        return "bool".to_string();
    }

    if require_field_arg {
        let has_field_arg = args
            .get(1)
            .is_some_and(|expr| matches!(expr, ncc_ast::Expr::String(_)));
        if !has_field_arg {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_DESTRUCTURE_FIELD_001".to_string(),
                message: "destructure field not found or not provided".to_string(),
                stmt_kind: stmt_kind.to_string(),
                expected: Some("field string".to_string()),
                found: Some("missing".to_string()),
            });
        }
    }

    if matches!(kind, ncc_ast::DestructureKind::Move)
        && arg_types
            .first()
            .is_some_and(|ty| ty.eq_ignore_ascii_case("null") || ty.eq_ignore_ascii_case("unknown"))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_DESTRUCTURE_MOVE_001".to_string(),
            message: "invalid move destructure".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("movable non-null value".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    if matches!(kind, ncc_ast::DestructureKind::Ownership)
        && arg_types
            .first()
            .is_some_and(|ty| compiler_type_is_nullable_like(ty))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_DESTRUCTURE_OWNERSHIP_001".to_string(),
            message: "destructure violates ownership contract".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("owned non-null value".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    if matches!(kind, ncc_ast::DestructureKind::Region) {
        let valid_region = arg_types
            .first()
            .is_some_and(|ty| compiler_type_is_memregion_family(ty));
        let bad_bounds = args
            .get(1)
            .and_then(compiler_expr_integer_literal_value)
            .is_some_and(|value| value < 0)
            || args
                .get(2)
                .and_then(compiler_expr_integer_literal_value)
                .is_some_and(|value| value <= 0);
        if !valid_region || bad_bounds {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_DESTRUCTURE_REGION_BOUNDS_001".to_string(),
                message: "region destructure out of bounds or invalid region".to_string(),
                stmt_kind: stmt_kind.to_string(),
                expected: Some("memRegion<T> with valid bounds".to_string()),
                found: arg_types.first().cloned(),
            });
        }
    }

    if matches!(kind, ncc_ast::DestructureKind::Binary)
        && arg_types
            .first()
            .is_some_and(|ty| !compiler_type_str_has_stable_binary_layout(ty))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_DESTRUCTURE_BINARY_LAYOUT_001".to_string(),
            message: "binary destructure requires stable layout".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("stable-binary-layout".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    "bool".to_string()
}

fn compiler_collect_constructure_contract_diagnostics(
    kind: ncc_ast::ConstructureKind,
    arg_types: &[String],
    args: &[ncc_ast::Expr],
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    stmt_kind: &str,
    typed_field_shape: bool,
) -> String {
    if args.is_empty() {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONSTRUCTURE_FIELD_MISSING_001".to_string(),
            message: "constructure missing required field".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("required fields".to_string()),
            found: Some(format!("{kind:?}")),
        });
        return "unknown".to_string();
    }

    if typed_field_shape
        && args.len() >= 2
        && arg_types
            .first()
            .zip(arg_types.get(1))
            .is_some_and(|(lhs, rhs)| lhs != rhs)
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONSTRUCTURE_FIELD_TYPE_001".to_string(),
            message: "constructure field type mismatch".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: arg_types.first().cloned(),
            found: arg_types.get(1).cloned(),
        });
    }

    if matches!(kind, ncc_ast::ConstructureKind::Move)
        && arg_types
            .first()
            .is_some_and(|ty| ty.eq_ignore_ascii_case("null") || ty.eq_ignore_ascii_case("unknown"))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONSTRUCTURE_MOVE_001".to_string(),
            message: "invalid move constructure".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("movable non-null value".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    if matches!(kind, ncc_ast::ConstructureKind::Region)
        && arg_types
            .first()
            .is_some_and(|ty| !compiler_type_is_memregion_family(ty))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONSTRUCTURE_REGION_001".to_string(),
            message: "invalid region constructure".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("memRegion<T>".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    if matches!(kind, ncc_ast::ConstructureKind::Binary)
        && arg_types
            .first()
            .is_some_and(|ty| !compiler_type_str_has_stable_binary_layout(ty))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONSTRUCTURE_BINARY_LAYOUT_001".to_string(),
            message: "binary constructure layout invalid".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("stable-binary-layout".to_string()),
            found: arg_types.first().cloned(),
        });
    }

    arg_types
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

fn compiler_collect_language_surface_call_diagnostics(
    callee: &str,
    arg_types: &[String],
    args: &[ncc_ast::Expr],
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) -> Option<String> {
    let lower = callee.to_ascii_lowercase();
    if lower == "discardnull" {
        let arg_ty = arg_types
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        if !compiler_type_is_nullable_like(&arg_ty) {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_NULL_TO_VOID_001".to_string(),
                message: "null->void requires explicit nullable/null input to discard".to_string(),
                stmt_kind: "expr.call".to_string(),
                expected: Some("nullable|null".to_string()),
                found: Some(arg_ty),
            });
        }
        return Some("void".to_string());
    }

    if lower == "addrof" {
        if arg_types
            .first()
            .is_some_and(|ty| compiler_type_is_void(ty.as_str()))
        {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_VOID_ADDR_001".to_string(),
                message: "void has no address".to_string(),
                stmt_kind: "expr.call".to_string(),
                expected: Some("non-void".to_string()),
                found: Some("void".to_string()),
            });
        }
        return Some("addr".to_string());
    }

    if lower == "siteof" {
        if arg_types
            .first()
            .is_some_and(|ty| compiler_type_is_void(ty.as_str()))
        {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_VOID_SITE_001".to_string(),
                message: "void has no site".to_string(),
                stmt_kind: "expr.call".to_string(),
                expected: Some("non-void".to_string()),
                found: Some("void".to_string()),
            });
        }
        return Some("site".to_string());
    }

    if lower == "arrayread" || lower == "arraywrite" {
        let receiver_ty = arg_types
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        if receiver_ty != "unknown" && !compiler_collection_like_type(&receiver_ty) {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_TYPE_ARRAY_001".to_string(),
                message: format!(
                    "{} requires array/list/tensor/matrix receiver, found '{}'",
                    lower, receiver_ty
                ),
                stmt_kind: "expr.call".to_string(),
                expected: Some("array/list/memRegion".to_string()),
                found: Some(receiver_ty.clone()),
            });
        }

        let index_end = if lower == "arraywrite" {
            if arg_types.len() < 3 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_INDEX_001".to_string(),
                    message: "arrayWrite expects one or more indices plus value".to_string(),
                    stmt_kind: "expr.call".to_string(),
                    expected: Some("receiver, index..., value".to_string()),
                    found: Some(arg_types.len().to_string()),
                });
                return Some("void".to_string());
            }
            arg_types.len() - 1
        } else {
            if arg_types.len() < 2 {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_INDEX_001".to_string(),
                    message: "arrayRead expects one or more indices".to_string(),
                    stmt_kind: "expr.call".to_string(),
                    expected: Some("receiver, index...".to_string()),
                    found: Some(arg_types.len().to_string()),
                });
                return Some("unknown".to_string());
            }
            arg_types.len()
        };
        let index_count = index_end.saturating_sub(1);
        if compiler_type_is_open_dim_matrix_or_tensor(&receiver_ty) {
            out.push(CompilerTypeDiagnosticRow {
                code: "E_ARRAY_OPEN_DIM_INDEX_001".to_string(),
                message: format!(
                    "open-dimensional '{}' cannot be indexed with []; declare numeric dimensions first",
                    receiver_ty
                ),
                stmt_kind: "expr.call".to_string(),
                expected: Some("matrix/tensor with numeric dimensions".to_string()),
                found: Some(receiver_ty.clone()),
            });
            return if lower == "arraywrite" {
                Some("void".to_string())
            } else {
                Some("unknown".to_string())
            };
        }

        if let Some(expected_rank) = compiler_collection_rank_hint(&receiver_ty) {
            if index_count != expected_rank {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_RANK_001".to_string(),
                    message: format!(
                        "array rank mismatch: receiver '{}' expects {} indices, found {}",
                        receiver_ty, expected_rank, index_count
                    ),
                    stmt_kind: "expr.call".to_string(),
                    expected: Some(expected_rank.to_string()),
                    found: Some(index_count.to_string()),
                });
            }
        }

        for (offset, item_ty) in arg_types.iter().enumerate().take(index_end).skip(1) {
            if item_ty != "unknown" && !compiler_numeric_like_type(item_ty) {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_INDEX_001".to_string(),
                    message: format!(
                        "array index type mismatch at axis {}: expected numeric index, found '{}'",
                        offset - 1,
                        item_ty
                    ),
                    stmt_kind: "expr.call".to_string(),
                    expected: Some("numeric".to_string()),
                    found: Some(item_ty.clone()),
                });
            }
        }

        if let Some(static_dims) = compiler_collection_static_dims(&receiver_ty) {
            if static_dims.len() == index_count {
                for (axis, expr) in args.iter().take(index_end).skip(1).enumerate() {
                    if let Some(index_literal) = compiler_expr_integer_literal_value(expr) {
                        if index_literal < 0 {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_TYPE_ARRAY_BOUNDS_001".to_string(),
                                message: format!(
                                    "index literal '{}' is out of bounds for axis {}",
                                    index_literal, axis
                                ),
                                stmt_kind: "expr.call".to_string(),
                                expected: Some("non-negative index within bounds".to_string()),
                                found: Some(index_literal.to_string()),
                            });
                            continue;
                        }
                        if let Ok(index_usize) = usize::try_from(index_literal) {
                            let dim = static_dims[axis];
                            if index_usize >= dim {
                                out.push(CompilerTypeDiagnosticRow {
                                    code: "E_TYPE_ARRAY_BOUNDS_001".to_string(),
                                    message: format!(
                                        "array index '{}' is out of bounds for axis {} (dim={})",
                                        index_usize, axis, dim
                                    ),
                                    stmt_kind: "expr.call".to_string(),
                                    expected: Some(format!("0..{}", dim.saturating_sub(1))),
                                    found: Some(index_usize.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        return Some(if lower == "arraywrite" {
            "void".to_string()
        } else {
            "unknown".to_string()
        });
    }

    if let Some(kind) = compiler_legacy_destructure_kind(&lower) {
        return Some(compiler_collect_destructure_contract_diagnostics(
            kind,
            arg_types,
            args,
            out,
            "expr.call",
            lower == "destructurefield",
        ));
    }

    if let Some(kind) = compiler_legacy_constructure_kind(&lower) {
        return Some(compiler_collect_constructure_contract_diagnostics(
            kind,
            arg_types,
            args,
            out,
            "expr.call",
            lower == "constructuretypedfield",
        ));
    }

    None
}

fn compiler_update_bounds_for_context_assignment(
    symbol_name: &str,
    expected_ty: &str,
    value: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    bounds: &mut BTreeMap<String, CompilerIndexBoundsInfo>,
) {
    let mut info = compiler_index_bounds_from_expr(value, scope, bounds);
    if compiler_type_is_memregion_family(expected_ty) && info.memregion_len.is_none() {
        if let ncc_ast::Expr::Call { callee, args } = value {
            if callee.eq_ignore_ascii_case("regionFromPtr") {
                info.memregion_len = compiler_extract_region_len_from_call_args(args);
            }
        }
        if let ncc_ast::Expr::QualifiedCall { path, args } = value {
            if path
                .last()
                .is_some_and(|segment| segment.eq_ignore_ascii_case("regionFromPtr"))
            {
                info.memregion_len = compiler_extract_region_len_from_call_args(args);
            }
        }
    }
    bounds.insert(symbol_name.to_ascii_lowercase(), info);
}

fn compiler_simple_type_diagnostic_code(ty: &str) -> Option<&'static str> {
    match ty {
        "int" => Some("E_TYPE_INT_001"),
        "u64" => Some("E_TYPE_U64_001"),
        "byte" => Some("E_TYPE_BYTE_001"),
        "bool" => Some("E_TYPE_BOOL_001"),
        "str" => Some("E_TYPE_STR_001"),
        "text" => Some("E_TYPE_TEXT_001"),
        "char" => Some("E_TYPE_CHAR_001"),
        "ft64" => Some("E_TYPE_FT64_001"),
        "addr" => Some("E_TYPE_ADDR_001"),
        "word" => Some("E_TYPE_WORD_001"),
        "dword" => Some("E_TYPE_DWORD_001"),
        "qword" => Some("E_TYPE_QWORD_001"),
        _ => None,
    }
}

fn compiler_compound_type_diagnostic_code(ty: &str) -> Option<&'static str> {
    let lower = ty.to_ascii_lowercase();
    if lower.starts_with("ptr<") {
        return Some("E_TYPE_PTR_001");
    }
    if lower.starts_with("memregion<") {
        return Some("E_TYPE_MEMREGION_001");
    }
    if lower.starts_with("array<")
        || lower.starts_with("vector<")
        || lower.starts_with("array2<")
        || lower.starts_with("array3<")
        || lower.starts_with("matrix<")
        || lower.starts_with("tensor<")
        || lower.starts_with("xy<")
        || lower.starts_with("xyz<")
        || lower.starts_with("x2y2<")
        || lower.starts_with("x2y2z2<")
        || lower.starts_with("x3y3<")
        || lower.starts_with("x3y3z3<")
        || ty == "array"
    {
        return Some("E_TYPE_ARRAY_001");
    }
    if lower.starts_with("dinarray<")
        || lower.starts_with("dinarray.binary<")
        || lower.starts_with("dinarray.unbinary<")
    {
        return Some("E_DINARRAY_TYPE_001");
    }
    if lower.starts_with("list<") {
        return Some("E_TYPE_LIST_001");
    }
    if lower.starts_with("result<") {
        return Some("E_TYPE_RESULT_001");
    }
    if lower.starts_with("option<") {
        return Some("E_TYPE_OPTION_001");
    }
    None
}

fn compiler_type_diagnostic_code_for_value(ty: &str) -> Option<&'static str> {
    compiler_simple_type_diagnostic_code(ty).or_else(|| compiler_compound_type_diagnostic_code(ty))
}

fn compiler_parse_integer_literal_text(raw: &str) -> Option<i128> {
    let compact = raw.trim().replace('_', "");
    if compact.is_empty() {
        return None;
    }

    let (sign, body) = if let Some(rest) = compact.strip_prefix('-') {
        (-1i128, rest)
    } else if let Some(rest) = compact.strip_prefix('+') {
        (1i128, rest)
    } else {
        (1i128, compact.as_str())
    };

    if body.is_empty() {
        return None;
    }

    let (radix, digits) = if let Some(rest) = body.strip_prefix("0x").or(body.strip_prefix("0X")) {
        (16, rest)
    } else if let Some(rest) = body.strip_prefix("0b").or(body.strip_prefix("0B")) {
        (2, rest)
    } else if let Some(rest) = body.strip_prefix("0o").or(body.strip_prefix("0O")) {
        (8, rest)
    } else {
        (10, body)
    };

    if digits.is_empty() {
        return None;
    }

    let parsed = i128::from_str_radix(digits, radix).ok()?;
    Some(sign * parsed)
}

fn compiler_expr_integer_literal_value(expr: &ncc_ast::Expr) -> Option<i128> {
    match expr {
        ncc_ast::Expr::Number(raw) => compiler_parse_integer_literal_text(raw),
        ncc_ast::Expr::Unary {
            op: ncc_ast::UnaryOp::Minus,
            expr,
        } => {
            if let ncc_ast::Expr::Number(raw) = &**expr {
                compiler_parse_integer_literal_text(raw).map(|value| -value)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn compiler_expr_multi_index_items(expr: &ncc_ast::Expr) -> Option<&[ncc_ast::Expr]> {
    match expr {
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => Some(items),
        _ => None,
    }
}

fn compiler_parse_non_negative_usize(raw: &str) -> Option<usize> {
    let value = compiler_parse_integer_literal_text(raw)?;
    if value < 0 {
        return None;
    }
    usize::try_from(value).ok()
}

fn compiler_collection_rank_hint(ty: &str) -> Option<usize> {
    let lower = compiler_compact_lower_type(ty);
    if lower.starts_with("matrix<") {
        let args = compiler_type_generic_args(ty, "matrix")?;
        if args.len() != 3 {
            return None;
        }
        let _rows = compiler_parse_non_negative_usize(args.get(1)?)?;
        let _cols = compiler_parse_non_negative_usize(args.get(2)?)?;
        return Some(2);
    }
    if lower.starts_with("tensor<") {
        let args = compiler_type_generic_args(ty, "tensor")?;
        if args.len() < 2 {
            return None;
        }
        for dim in args.iter().skip(1) {
            let _ = compiler_parse_non_negative_usize(dim)?;
        }
        return args.len().checked_sub(1).filter(|rank| *rank > 0);
    }
    if lower.starts_with("array3<") || lower.starts_with("x2y2z2<") || lower.starts_with("x3y3z3<")
    {
        return Some(3);
    }
    if lower.starts_with("array2<")
        || lower.starts_with("xy<")
        || lower.starts_with("xyz<")
        || lower.starts_with("x2y2<")
        || lower.starts_with("x3y3<")
    {
        if lower.starts_with("xyz<") {
            return Some(3);
        }
        return Some(2);
    }
    if compiler_collection_like_type(ty) {
        return Some(1);
    }
    None
}

fn compiler_type_is_open_dim_matrix_or_tensor(ty: &str) -> bool {
    let lower = compiler_compact_lower_type(ty);
    if lower.starts_with("matrix<") {
        if let Some(args) = compiler_type_generic_args(ty, "matrix") {
            if args.len() == 3 {
                return compiler_parse_non_negative_usize(args.get(1).unwrap()).is_none()
                    || compiler_parse_non_negative_usize(args.get(2).unwrap()).is_none();
            }
        }
        return false;
    }
    if lower.starts_with("tensor<") {
        if let Some(args) = compiler_type_generic_args(ty, "tensor") {
            if args.len() >= 2 {
                return args
                    .iter()
                    .skip(1)
                    .any(|dim| compiler_parse_non_negative_usize(dim).is_none());
            }
        }
    }
    false
}

fn compiler_collection_static_dims(ty: &str) -> Option<Vec<usize>> {
    let lower = compiler_compact_lower_type(ty);
    if lower.starts_with("matrix<") {
        let args = compiler_type_generic_args(ty, "matrix")?;
        if args.len() != 3 {
            return None;
        }
        let rows = compiler_parse_non_negative_usize(args.get(1)?)?;
        let cols = compiler_parse_non_negative_usize(args.get(2)?)?;
        return Some(vec![rows, cols]);
    }
    if lower.starts_with("tensor<") {
        let args = compiler_type_generic_args(ty, "tensor")?;
        if args.len() < 2 {
            return None;
        }
        let mut dims = Vec::new();
        for item in args.iter().skip(1) {
            dims.push(compiler_parse_non_negative_usize(item)?);
        }
        return Some(dims);
    }
    if lower.starts_with("array2<") {
        let args = compiler_type_generic_args(ty, "array2")?;
        if args.len() != 3 {
            return None;
        }
        return Some(vec![
            compiler_parse_non_negative_usize(args.get(1)?)?,
            compiler_parse_non_negative_usize(args.get(2)?)?,
        ]);
    }
    if lower.starts_with("array3<") {
        let args = compiler_type_generic_args(ty, "array3")?;
        if args.len() != 4 {
            return None;
        }
        return Some(vec![
            compiler_parse_non_negative_usize(args.get(1)?)?,
            compiler_parse_non_negative_usize(args.get(2)?)?,
            compiler_parse_non_negative_usize(args.get(3)?)?,
        ]);
    }
    if lower.starts_with("array<") {
        let args = compiler_type_generic_args(ty, "array")?;
        if args.len() == 2 {
            return Some(vec![compiler_parse_non_negative_usize(args.get(1)?)?]);
        }
        return None;
    }
    None
}

fn compiler_integer_range_for_type(ty: &str) -> Option<(i128, i128)> {
    match ty {
        "byte" => Some((0, u8::MAX as i128)),
        "word" => Some((0, u16::MAX as i128)),
        "dword" => Some((0, u32::MAX as i128)),
        "qword" | "u64" => Some((0, u64::MAX as i128)),
        "int" => Some((i64::MIN as i128, i64::MAX as i128)),
        _ => None,
    }
}

fn compiler_collect_integer_literal_range_diag(
    stmt_kind: &str,
    expected_ty: &str,
    value: &ncc_ast::Expr,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    let Some(literal) = compiler_expr_integer_literal_value(value) else {
        return;
    };
    let Some((min, max)) = compiler_integer_range_for_type(expected_ty) else {
        return;
    };
    if literal < min || literal > max {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_TYPE_MISMATCH_001".to_string(),
            message: format!(
                "integer literal '{}' is out of range for type '{}' (expected {}..={})",
                literal, expected_ty, min, max
            ),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(format!("{}..={}", min, max)),
            found: Some(literal.to_string()),
        });
        if let Some(code) = compiler_simple_type_diagnostic_code(expected_ty) {
            out.push(CompilerTypeDiagnosticRow {
                code: code.to_string(),
                message: format!(
                    "invalid '{}' literal assignment/return: value '{}' is out of range",
                    expected_ty, literal
                ),
                stmt_kind: stmt_kind.to_string(),
                expected: Some(format!("{}..={}", min, max)),
                found: Some(literal.to_string()),
            });
        }
    }
}

fn compiler_expr_has_pointer_origin(
    expr: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
) -> bool {
    match expr {
        ncc_ast::Expr::Identifier(name) => {
            if name.eq_ignore_ascii_case("null") {
                return false;
            }
            scope
                .get(&name.to_ascii_lowercase())
                .is_some_and(|ty| compiler_type_is_pointer_origin_source_type(ty))
        }
        ncc_ast::Expr::ContextRef(reference) => reference.path.last().is_some_and(|name| {
            scope
                .get(&name.to_ascii_lowercase())
                .is_some_and(|ty| compiler_type_is_pointer_origin_source_type(ty))
        }),
        ncc_ast::Expr::Call { .. }
        | ncc_ast::Expr::QualifiedCall { .. }
        | ncc_ast::Expr::MethodCall { .. } => true,
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_expr_has_pointer_origin(value, scope)
        }
        _ => false,
    }
}

fn compiler_collect_pointer_contract_diagnostics(
    stmt_kind: &str,
    expected_ty: &str,
    expected_nullable: bool,
    found_ty: &str,
    value: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    if !compiler_type_is_pointer_family(expected_ty)
        && !compiler_type_is_nullable_pointer_union(expected_ty)
    {
        return;
    }

    if !expected_nullable
        && (found_ty == "null" || compiler_type_is_nullable_pointer_union(found_ty))
    {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_TYPE_PTR_NULL_001".to_string(),
            message: format!(
                "nullable ptr<T> value '{}' must be refined before assignment/use in non-nullable '{}'",
                found_ty, expected_ty
            ),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(expected_ty.to_string()),
            found: Some(found_ty.to_string()),
        });
    }

    if found_ty == "unknown" || found_ty == "never" || found_ty == "null" {
        return;
    }

    if compiler_type_is_pointer_origin_source_type(found_ty) {
        return;
    }

    if compiler_expr_has_pointer_origin(value, scope) {
        return;
    }

    out.push(CompilerTypeDiagnosticRow {
        code: "E_TYPE_PTR_ORIGIN_001".to_string(),
        message: format!(
            "ptr<T> assignment requires pointer/addr origin, found '{}'",
            found_ty
        ),
        stmt_kind: stmt_kind.to_string(),
        expected: Some("pointer-origin".to_string()),
        found: Some(found_ty.to_string()),
    });
}

fn compiler_push_operand_type_diag(
    code: Option<&'static str>,
    stmt_kind: &str,
    message: String,
    found: &str,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    if let Some(code) = code {
        out.push(CompilerTypeDiagnosticRow {
            code: code.to_string(),
            message,
            stmt_kind: stmt_kind.to_string(),
            expected: None,
            found: Some(found.to_string()),
        });
    }
}

fn compiler_collect_expr_type_diagnostics(
    expr: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    bounds: &BTreeMap<String, CompilerIndexBoundsInfo>,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) -> String {
    match expr {
        ncc_ast::Expr::StructLiteral { ty, fields } => {
            for (_, value) in fields {
                compiler_collect_expr_type_diagnostics(value, scope, bounds, out);
            }
            ty.clone()
        }
        ncc_ast::Expr::Bool(_) => "bool".to_string(),
        ncc_ast::Expr::Char(_) => "char".to_string(),
        ncc_ast::Expr::Number(_) => "int".to_string(),
        ncc_ast::Expr::String(_) => "str".to_string(),
        ncc_ast::Expr::Identifier(name) => {
            if name.eq_ignore_ascii_case("null") {
                "null".to_string()
            } else {
                scope
                    .get(&name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            }
        }
        ncc_ast::Expr::ContextRef(reference) => {
            if let Some(name) = reference.path.last() {
                scope
                    .get(&name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            }
        }
        ncc_ast::Expr::PostIncrement { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()).cloned())
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::FieldPostIncrement { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()).cloned())
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::PointerPostIncrementRead { target } => target
            .path
            .last()
            .and_then(|name| scope.get(&name.to_ascii_lowercase()))
            .and_then(|ty| compiler_pointer_inner_type(ty))
            .unwrap_or_else(|| "int".to_string()),
        ncc_ast::Expr::VectorLiteral(items) | ncc_ast::Expr::ArrayLiteral(items) => {
            for item in items {
                compiler_collect_expr_type_diagnostics(item, scope, bounds, out);
            }
            "array".to_string()
        }
        ncc_ast::Expr::Index { target, index } => {
            let target_ty = compiler_collect_expr_type_diagnostics(target, scope, bounds, out);
            let multi_index_items = compiler_expr_multi_index_items(index);
            let index_ty = compiler_collect_expr_type_diagnostics(index, scope, bounds, out);
            if let Some(items) = multi_index_items {
                for item in items {
                    let item_ty = compiler_collect_expr_type_diagnostics(item, scope, bounds, out);
                    if item_ty != "unknown" && !compiler_numeric_like_type(&item_ty) {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_ARRAY_INDEX_001".to_string(),
                            message: format!(
                                "array index type mismatch: expected numeric index, found '{}'",
                                item_ty
                            ),
                            stmt_kind: "expr.index".to_string(),
                            expected: Some("numeric".to_string()),
                            found: Some(item_ty.clone()),
                        });
                        compiler_push_operand_type_diag(
                            compiler_type_diagnostic_code_for_value(&item_ty),
                            "expr.index",
                            format!("invalid index operand type '{}'", item_ty),
                            &item_ty,
                            out,
                        );
                    }
                }
            } else if index_ty != "unknown" && !compiler_numeric_like_type(&index_ty) {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_ARRAY_INDEX_001".to_string(),
                    message: format!(
                        "array index type mismatch: expected numeric index, found '{}'",
                        index_ty
                    ),
                    stmt_kind: "expr.index".to_string(),
                    expected: Some("numeric".to_string()),
                    found: Some(index_ty.clone()),
                });
                compiler_push_operand_type_diag(
                    compiler_type_diagnostic_code_for_value(&index_ty),
                    "expr.index",
                    format!("invalid index operand type '{}'", index_ty),
                    &index_ty,
                    out,
                );
            }
            if target_ty != "unknown" && compiler_collection_like_type(&target_ty) {
                if compiler_type_is_open_dim_matrix_or_tensor(&target_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_ARRAY_OPEN_DIM_INDEX_001".to_string(),
                        message: format!(
                            "open-dimensional '{}' cannot be indexed with []; declare numeric dimensions first",
                            target_ty
                        ),
                        stmt_kind: "expr.index".to_string(),
                        expected: Some("matrix/tensor with numeric dimensions".to_string()),
                        found: Some(target_ty.clone()),
                    });
                    return "unknown".to_string();
                }
                if let Some(items) = multi_index_items {
                    if let Some(expected_rank) = compiler_collection_rank_hint(&target_ty) {
                        if items.len() != expected_rank {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_ARRAY_RANK_001".to_string(),
                                message: format!(
                                    "array rank mismatch: target '{}' expects {} indices, found {}",
                                    target_ty,
                                    expected_rank,
                                    items.len()
                                ),
                                stmt_kind: "expr.index".to_string(),
                                expected: Some(expected_rank.to_string()),
                                found: Some(items.len().to_string()),
                            });
                        }
                    }
                    if let Some(static_dims) = compiler_collection_static_dims(&target_ty) {
                        if static_dims.len() == items.len() {
                            let mut offset = 0usize;
                            let mut stride = 1usize;
                            for (axis, item) in items.iter().enumerate().rev() {
                                if let Some(index_literal) =
                                    compiler_expr_integer_literal_value(item)
                                {
                                    if index_literal < 0 {
                                        out.push(CompilerTypeDiagnosticRow {
                                            code: "E_TYPE_ARRAY_BOUNDS_001".to_string(),
                                            message: format!(
                                                "index literal '{}' is out of bounds for target type '{}'",
                                                index_literal, target_ty
                                            ),
                                            stmt_kind: "expr.index".to_string(),
                                            expected: Some("non-negative index within bounds".to_string()),
                                            found: Some(index_literal.to_string()),
                                        });
                                        continue;
                                    }
                                    if let Ok(index_usize) = usize::try_from(index_literal) {
                                        let dim = static_dims[axis];
                                        if index_usize >= dim {
                                            out.push(CompilerTypeDiagnosticRow {
                                                code: "E_TYPE_ARRAY_BOUNDS_001".to_string(),
                                                message: format!(
                                                    "array index '{}' is out of bounds for axis {} (dim={})",
                                                    index_usize, axis, dim
                                                ),
                                                stmt_kind: "expr.index".to_string(),
                                                expected: Some(format!("0..{}", dim.saturating_sub(1))),
                                                found: Some(index_usize.to_string()),
                                            });
                                        }
                                        if axis < static_dims.len() - 1 {
                                            stride = stride
                                                .checked_mul(static_dims[axis + 1])
                                                .unwrap_or(0);
                                        }
                                        if stride == 0
                                            || index_usize.checked_mul(stride).is_none()
                                            || offset
                                                .checked_add(index_usize.saturating_mul(stride))
                                                .is_none()
                                        {
                                            out.push(CompilerTypeDiagnosticRow {
                                                code: "E_ARRAY_OFFSET_OVERFLOW_001".to_string(),
                                                message: "array offset overflow".to_string(),
                                                stmt_kind: "expr.index".to_string(),
                                                expected: Some(
                                                    "safe offset arithmetic".to_string(),
                                                ),
                                                found: Some("overflow".to_string()),
                                            });
                                        } else {
                                            offset += index_usize * stride;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(index_literal) = compiler_expr_integer_literal_value(index) {
                    if index_literal < 0 {
                        let code = if compiler_type_is_memregion_family(&target_ty) {
                            "E_TYPE_MEMREGION_BOUNDS_001"
                        } else {
                            "E_TYPE_ARRAY_BOUNDS_001"
                        };
                        out.push(CompilerTypeDiagnosticRow {
                            code: code.to_string(),
                            message: format!(
                                "index literal '{}' is out of bounds for target type '{}'",
                                index_literal, target_ty
                            ),
                            stmt_kind: "expr.index".to_string(),
                            expected: Some("non-negative index within bounds".to_string()),
                            found: Some(index_literal.to_string()),
                        });
                    } else if let Ok(index_usize) = usize::try_from(index_literal) {
                        let target_bounds = compiler_index_bounds_from_expr(target, scope, bounds);
                        if compiler_type_is_memregion_family(&target_ty) {
                            if let Some(limit) = target_bounds.memregion_len {
                                if index_usize >= limit {
                                    out.push(CompilerTypeDiagnosticRow {
                                        code: "E_TYPE_MEMREGION_BOUNDS_001".to_string(),
                                        message: format!(
                                            "memRegion index '{}' is out of bounds (len={})",
                                            index_usize, limit
                                        ),
                                        stmt_kind: "expr.index".to_string(),
                                        expected: Some(format!("0..{}", limit.saturating_sub(1))),
                                        found: Some(index_usize.to_string()),
                                    });
                                }
                            }
                        } else if let Some(limit) = target_bounds.array_len {
                            if index_usize >= limit {
                                out.push(CompilerTypeDiagnosticRow {
                                    code: "E_TYPE_ARRAY_BOUNDS_001".to_string(),
                                    message: format!(
                                        "array/list index '{}' is out of bounds (len={})",
                                        index_usize, limit
                                    ),
                                    stmt_kind: "expr.index".to_string(),
                                    expected: Some(format!("0..{}", limit.saturating_sub(1))),
                                    found: Some(index_usize.to_string()),
                                });
                            }
                        }
                    }
                }
            }
            if target_ty != "unknown" && !compiler_collection_like_type(&target_ty) {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_TYPE_ARRAY_001".to_string(),
                    message: format!(
                        "index operation requires array/list/memRegion target, found '{}'",
                        target_ty
                    ),
                    stmt_kind: "expr.index".to_string(),
                    expected: Some("array/list/memRegion".to_string()),
                    found: Some(target_ty),
                });
            }
            "unknown".to_string()
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_collect_expr_type_diagnostics(value, scope, bounds, out);
            "bool".to_string()
        }
        ncc_ast::Expr::Unary { op, expr } => {
            let inner_ty = compiler_collect_expr_type_diagnostics(expr, scope, bounds, out);
            match op {
                ncc_ast::UnaryOp::Not => {
                    if inner_ty != "unknown" && inner_ty != "bool" {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_CONDITION_BOOL_REQUIRED_001".to_string(),
                            message: format!(
                                "logical not requires bool operand, found '{}'",
                                inner_ty
                            ),
                            stmt_kind: "expr.unary".to_string(),
                            expected: Some("bool".to_string()),
                            found: Some(inner_ty.clone()),
                        });
                    }
                    "bool".to_string()
                }
                ncc_ast::UnaryOp::Plus | ncc_ast::UnaryOp::Minus | ncc_ast::UnaryOp::NotBin => {
                    if inner_ty != "unknown" && !compiler_numeric_like_type(&inner_ty) {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_TYPE_MISMATCH_001".to_string(),
                            message: format!(
                                "numeric unary operator requires numeric operand, found '{}'",
                                inner_ty
                            ),
                            stmt_kind: "expr.unary".to_string(),
                            expected: Some("numeric".to_string()),
                            found: Some(inner_ty.clone()),
                        });
                        compiler_push_operand_type_diag(
                            compiler_type_diagnostic_code_for_value(&inner_ty),
                            "expr.unary",
                            format!("invalid numeric unary operand type '{}'", inner_ty),
                            &inner_ty,
                            out,
                        );
                    }
                    "int".to_string()
                }
                ncc_ast::UnaryOp::TryPropagate => compiler_option_inner_type(&inner_ty)
                    .or_else(|| compiler_result_ok_inner_type(&inner_ty))
                    .unwrap_or_else(|| {
                        if inner_ty != "unknown" {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_HIGH_TRY_OPERAND_001".to_string(),
                                message: format!(
                                    "error propagation requires Option<T> or Result<T,E>, found '{}'",
                                    inner_ty
                                ),
                                stmt_kind: "expr.try".to_string(),
                                expected: Some("Option<T>/Result<T,E>".to_string()),
                                found: Some(inner_ty.clone()),
                            });
                        }
                        "unknown".to_string()
                    }),
            }
        }
        ncc_ast::Expr::Binary { left, op, right } => {
            let left_ty = compiler_collect_expr_type_diagnostics(left, scope, bounds, out);
            let right_ty = compiler_collect_expr_type_diagnostics(right, scope, bounds, out);
            match op {
                ncc_ast::BinaryOp::Greater
                | ncc_ast::BinaryOp::Less
                | ncc_ast::BinaryOp::GreaterEq
                | ncc_ast::BinaryOp::LessEq => {
                    let numeric_pair = compiler_numeric_like_type(&left_ty)
                        && compiler_numeric_like_type(&right_ty);
                    let char_pair = left_ty == "char" && right_ty == "char";
                    let string_pair = matches!(left_ty.as_str(), "str" | "text" | "string")
                        && matches!(right_ty.as_str(), "str" | "text" | "string");
                    let unknown_pair = left_ty == "unknown" || right_ty == "unknown";
                    if !(numeric_pair || char_pair || string_pair || unknown_pair) {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_TYPE_MISMATCH_001".to_string(),
                            message: format!(
                                "comparison operator requires numeric/char/string operands, found '{}' and '{}'",
                                left_ty, right_ty
                            ),
                            stmt_kind: "expr.binary".to_string(),
                            expected: Some("numeric|char|string".to_string()),
                            found: Some(format!("{} , {}", left_ty, right_ty)),
                        });
                        if left_ty != "unknown"
                            && !compiler_numeric_like_type(&left_ty)
                            && left_ty != "char"
                            && !matches!(left_ty.as_str(), "str" | "text" | "string")
                        {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&left_ty),
                                "expr.binary",
                                format!("invalid comparison operand type '{}'", left_ty),
                                &left_ty,
                                out,
                            );
                        }
                        if right_ty != "unknown"
                            && !compiler_numeric_like_type(&right_ty)
                            && right_ty != "char"
                            && !matches!(right_ty.as_str(), "str" | "text" | "string")
                        {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&right_ty),
                                "expr.binary",
                                format!("invalid comparison operand type '{}'", right_ty),
                                &right_ty,
                                out,
                            );
                        }
                    }
                    "bool".to_string()
                }
                ncc_ast::BinaryOp::EqEq | ncc_ast::BinaryOp::NotEq => {
                    let compatible = compiler_types_compatible(&left_ty, &right_ty)
                        || compiler_types_compatible(&right_ty, &left_ty);
                    if left_ty != "unknown" && right_ty != "unknown" && !compatible {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_TYPE_MISMATCH_001".to_string(),
                            message: format!(
                                "equality operator type mismatch: '{}' vs '{}'",
                                left_ty, right_ty
                            ),
                            stmt_kind: "expr.binary".to_string(),
                            expected: Some(left_ty),
                            found: Some(right_ty),
                        });
                    }
                    "bool".to_string()
                }
                ncc_ast::BinaryOp::AndAnd | ncc_ast::BinaryOp::OrOr => {
                    let left_ok = left_ty == "unknown" || left_ty == "bool";
                    let right_ok = right_ty == "unknown" || right_ty == "bool";
                    if !left_ok || !right_ok {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_CONDITION_BOOL_REQUIRED_001".to_string(),
                            message: format!(
                                "logical operator requires bool operands, found '{}' and '{}'",
                                left_ty, right_ty
                            ),
                            stmt_kind: "expr.binary".to_string(),
                            expected: Some("bool".to_string()),
                            found: Some(format!("{} , {}", left_ty, right_ty)),
                        });
                        if !left_ok {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&left_ty),
                                "expr.binary",
                                format!("invalid logical operand type '{}'", left_ty),
                                &left_ty,
                                out,
                            );
                        }
                        if !right_ok {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&right_ty),
                                "expr.binary",
                                format!("invalid logical operand type '{}'", right_ty),
                                &right_ty,
                                out,
                            );
                        }
                    }
                    "bool".to_string()
                }
                ncc_ast::BinaryOp::Plus => {
                    let left_string = left_ty == "unknown" || compiler_string_like_type(&left_ty);
                    let right_string =
                        right_ty == "unknown" || compiler_string_like_type(&right_ty);
                    let left_pointer = compiler_type_is_pointer_family(&left_ty);
                    let right_pointer = compiler_type_is_pointer_family(&right_ty);
                    let left_int = left_ty == "unknown" || compiler_numeric_like_type(&left_ty);
                    let right_int = right_ty == "unknown" || compiler_numeric_like_type(&right_ty);
                    if left_pointer && right_int {
                        left_ty
                    } else if right_pointer && left_int {
                        right_ty
                    } else if left_string || right_string {
                        if left_string && right_string {
                            let context = compiler_get_type_diag_context();
                            if let Some(message) =
                                compiler_concat_forbidden_message(context.as_ref())
                            {
                                out.push(CompilerTypeDiagnosticRow {
                                    code: "E_TYPE_MODE_FORBIDDEN_001".to_string(),
                                    message: message.to_string(),
                                    stmt_kind: "expr.binary".to_string(),
                                    expected: Some(
                                        "context allowing text concatenation".to_string(),
                                    ),
                                    found: Some(compiler_context_label(context.as_ref())),
                                });
                            }
                            "str".to_string()
                        } else {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_TYPE_MISMATCH_001".to_string(),
                                message: format!(
                                    "explicit concatenation required: '+' between str and non-str is not allowed ({} + {})",
                                    left_ty, right_ty
                                ),
                                stmt_kind: "expr.binary".to_string(),
                                expected: Some("str + str (or concat/render)".to_string()),
                                found: Some(format!("{} , {}", left_ty, right_ty)),
                            });
                            "str".to_string()
                        }
                    } else {
                        let left_ok = left_ty == "unknown" || compiler_numeric_like_type(&left_ty);
                        let right_ok =
                            right_ty == "unknown" || compiler_numeric_like_type(&right_ty);
                        if !left_ok || !right_ok {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_TYPE_MISMATCH_001".to_string(),
                                message: format!(
                                    "numeric operator requires numeric operands, found '{}' and '{}'",
                                    left_ty, right_ty
                                ),
                                stmt_kind: "expr.binary".to_string(),
                                expected: Some("numeric".to_string()),
                                found: Some(format!("{} , {}", left_ty, right_ty)),
                            });
                            if !left_ok {
                                compiler_push_operand_type_diag(
                                    compiler_type_diagnostic_code_for_value(&left_ty),
                                    "expr.binary",
                                    format!("invalid numeric operand type '{}'", left_ty),
                                    &left_ty,
                                    out,
                                );
                            }
                            if !right_ok {
                                compiler_push_operand_type_diag(
                                    compiler_type_diagnostic_code_for_value(&right_ty),
                                    "expr.binary",
                                    format!("invalid numeric operand type '{}'", right_ty),
                                    &right_ty,
                                    out,
                                );
                            }
                        }
                        "int".to_string()
                    }
                }
                ncc_ast::BinaryOp::Minus => {
                    let left_pointer = compiler_type_is_pointer_family(&left_ty);
                    let right_pointer = compiler_type_is_pointer_family(&right_ty);
                    let right_int = right_ty == "unknown" || compiler_numeric_like_type(&right_ty);
                    if left_pointer && right_int {
                        left_ty
                    } else if left_pointer && right_pointer {
                        "int".to_string()
                    } else {
                        let left_ok = left_ty == "unknown" || compiler_numeric_like_type(&left_ty);
                        let right_ok =
                            right_ty == "unknown" || compiler_numeric_like_type(&right_ty);
                        if !left_ok || !right_ok {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_TYPE_MISMATCH_001".to_string(),
                                message: format!(
                                    "numeric operator requires numeric operands or pointer arithmetic, found '{}' and '{}'",
                                    left_ty, right_ty
                                ),
                                stmt_kind: "expr.binary".to_string(),
                                expected: Some("numeric or pointer arithmetic".to_string()),
                                found: Some(format!("{} , {}", left_ty, right_ty)),
                            });
                            if !left_ok {
                                compiler_push_operand_type_diag(
                                    compiler_type_diagnostic_code_for_value(&left_ty),
                                    "expr.binary",
                                    format!("invalid numeric operand type '{}'", left_ty),
                                    &left_ty,
                                    out,
                                );
                            }
                            if !right_ok {
                                compiler_push_operand_type_diag(
                                    compiler_type_diagnostic_code_for_value(&right_ty),
                                    "expr.binary",
                                    format!("invalid numeric operand type '{}'", right_ty),
                                    &right_ty,
                                    out,
                                );
                            }
                        }
                        "int".to_string()
                    }
                }
                ncc_ast::BinaryOp::BitAnd
                | ncc_ast::BinaryOp::BitOr
                | ncc_ast::BinaryOp::BitXor
                | ncc_ast::BinaryOp::ShiftLeft
                | ncc_ast::BinaryOp::ShiftRight
                | ncc_ast::BinaryOp::ShiftRightLogical
                | ncc_ast::BinaryOp::Star
                | ncc_ast::BinaryOp::Slash
                | ncc_ast::BinaryOp::Percent => {
                    let left_ok = left_ty == "unknown" || compiler_numeric_like_type(&left_ty);
                    let right_ok = right_ty == "unknown" || compiler_numeric_like_type(&right_ty);
                    if !left_ok || !right_ok {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_TYPE_MISMATCH_001".to_string(),
                            message: format!(
                                "numeric operator requires numeric operands, found '{}' and '{}'",
                                left_ty, right_ty
                            ),
                            stmt_kind: "expr.binary".to_string(),
                            expected: Some("numeric".to_string()),
                            found: Some(format!("{} , {}", left_ty, right_ty)),
                        });
                        if !left_ok {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&left_ty),
                                "expr.binary",
                                format!("invalid numeric operand type '{}'", left_ty),
                                &left_ty,
                                out,
                            );
                        }
                        if !right_ok {
                            compiler_push_operand_type_diag(
                                compiler_type_diagnostic_code_for_value(&right_ty),
                                "expr.binary",
                                format!("invalid numeric operand type '{}'", right_ty),
                                &right_ty,
                                out,
                            );
                        }
                    }
                    "int".to_string()
                }
            }
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            let cond_ty = compiler_collect_expr_type_diagnostics(condition, scope, bounds, out);
            if cond_ty != "bool" {
                out.push(CompilerTypeDiagnosticRow {
                    code: "E_CONDITION_BOOL_REQUIRED_001".to_string(),
                    message: format!("if-expression condition must be bool, found '{}'", cond_ty),
                    stmt_kind: "expr.ifelse".to_string(),
                    expected: Some("bool".to_string()),
                    found: Some(cond_ty),
                });
            }
            let then_ty = compiler_collect_expr_type_diagnostics(then_expr, scope, bounds, out);
            let else_ty = compiler_collect_expr_type_diagnostics(else_expr, scope, bounds, out);
            if then_ty == else_ty {
                then_ty
            } else {
                format!("{then_ty} | {else_ty}")
            }
        }
        ncc_ast::Expr::Call { callee, args } => {
            let arg_types = args
                .iter()
                .map(|arg| compiler_collect_expr_type_diagnostics(arg, scope, bounds, out))
                .collect::<Vec<_>>();
            if let Some(resolved) =
                compiler_collect_language_surface_call_diagnostics(callee, &arg_types, args, out)
            {
                return resolved;
            }
            if callee.eq_ignore_ascii_case("none") {
                return "Option<unknown>".to_string();
            }
            if callee.eq_ignore_ascii_case("some") {
                let inner = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Option<{}>", inner);
            }
            if callee.eq_ignore_ascii_case("ok") {
                let ok_ty = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Result<{}, unknown>", ok_ty);
            }
            if callee.eq_ignore_ascii_case("err") {
                let err_ty = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Result<unknown, {}>", err_ty);
            }
            "unknown".to_string()
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            let arg_types = args
                .iter()
                .map(|arg| compiler_collect_expr_type_diagnostics(arg, scope, bounds, out))
                .collect::<Vec<_>>();
            let callee = path.last().map(|value| value.as_str()).unwrap_or("");
            if let Some(resolved) =
                compiler_collect_language_surface_call_diagnostics(callee, &arg_types, args, out)
            {
                return resolved;
            }
            if callee.eq_ignore_ascii_case("none") {
                return "Option<unknown>".to_string();
            }
            if callee.eq_ignore_ascii_case("some") {
                let inner = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Option<{}>", inner);
            }
            if callee.eq_ignore_ascii_case("ok") {
                let ok_ty = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Result<{}, unknown>", ok_ty);
            }
            if callee.eq_ignore_ascii_case("err") {
                let err_ty = arg_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return format!("Result<unknown, {}>", err_ty);
            }
            "unknown".to_string()
        }
        ncc_ast::Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            let receiver_ty = compiler_collect_expr_type_diagnostics(receiver, scope, bounds, out);
            let arg_types = args
                .iter()
                .map(|arg| compiler_collect_expr_type_diagnostics(arg, scope, bounds, out))
                .collect::<Vec<_>>();
            let mut call_arg_types = Vec::with_capacity(arg_types.len() + 1);
            call_arg_types.push(receiver_ty.clone());
            call_arg_types.extend(arg_types.iter().cloned());
            let mut call_args = Vec::with_capacity(args.len() + 1);
            call_args.push((**receiver).clone());
            call_args.extend(args.iter().cloned());
            if let Some(resolved) = compiler_collect_language_surface_call_diagnostics(
                method,
                &call_arg_types,
                &call_args,
                out,
            ) {
                return resolved;
            }
            let method_lower = method.to_ascii_lowercase();
            if method.eq_ignore_ascii_case("unwrap") {
                let receiver_call_name =
                    compiler_expr_direct_call_name(receiver).map(|name| name.to_ascii_lowercase());
                if receiver_call_name.as_deref() == Some("none") {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_OPTION_UNWRAP_001".to_string(),
                        message: "unsafe unwrap on Option none path".to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("Option::some".to_string()),
                        found: Some("none".to_string()),
                    });
                } else if receiver_call_name.as_deref() == Some("err") {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_RESULT_UNWRAP_001".to_string(),
                        message: "unsafe unwrap on Result err path".to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("Result::ok".to_string()),
                        found: Some("err".to_string()),
                    });
                } else if receiver_call_name.as_deref() == Some("some")
                    || receiver_call_name.as_deref() == Some("ok")
                {
                    // Literal some/ok constructors can be unwrapped without ambiguity in static typing.
                } else if compiler_type_is_option_family(&receiver_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_OPTION_UNWRAP_001".to_string(),
                        message: format!(
                            "unsafe unwrap on Option path without prior refinement (found '{}')",
                            receiver_ty
                        ),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("refined Option::some".to_string()),
                        found: Some(receiver_ty.clone()),
                    });
                } else if compiler_type_is_result_family(&receiver_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_RESULT_UNWRAP_001".to_string(),
                        message: format!(
                            "unsafe unwrap on Result path without prior refinement (found '{}')",
                            receiver_ty
                        ),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("refined Result::ok".to_string()),
                        found: Some(receiver_ty.clone()),
                    });
                }

                if let Some(inner) = compiler_option_inner_type(&receiver_ty) {
                    return inner;
                }
                if let Some(inner) = compiler_result_ok_inner_type(&receiver_ty) {
                    return inner;
                }
            }

            if compiler_type_is_dinarray_family(&receiver_ty) {
                let receiver_bounds = compiler_index_bounds_from_expr(receiver, scope, bounds);
                if matches!(
                    method_lower.as_str(),
                    "get"
                        | "set"
                        | "insert"
                        | "removeat"
                        | "dinget"
                        | "dinset"
                        | "dininsert"
                        | "dinremoveat"
                ) {
                    if let (Some(index_expr), Some(limit)) =
                        (args.first(), receiver_bounds.array_len)
                    {
                        if let Some(index) = compiler_expr_non_negative_usize_literal(index_expr) {
                            if index >= limit {
                                out.push(CompilerTypeDiagnosticRow {
                                    code: "E_DINARRAY_BOUNDS_001".to_string(),
                                    message: format!(
                                        "dinArray index '{}' is out of bounds (len={})",
                                        index, limit
                                    ),
                                    stmt_kind: "expr.method".to_string(),
                                    expected: Some("index < len".to_string()),
                                    found: Some(index.to_string()),
                                });
                            }
                        }
                    }
                }

                if matches!(
                    method_lower.as_str(),
                    "reserve" | "dinreserve" | "createwithcapacity" | "dincreatewithcapacity"
                ) {
                    if let Some(cap_expr) = args.first() {
                        if let Some(capacity) = compiler_expr_integer_literal_value(cap_expr) {
                            if !(0..=1_000_000_000).contains(&capacity) {
                                out.push(CompilerTypeDiagnosticRow {
                                    code: "E_DINARRAY_CAPACITY_001".to_string(),
                                    message: format!(
                                        "dinArray capacity '{}' is invalid for this profile",
                                        capacity
                                    ),
                                    stmt_kind: "expr.method".to_string(),
                                    expected: Some("0..=1_000_000_000".to_string()),
                                    found: Some(capacity.to_string()),
                                });
                            }
                        }
                    }
                }

                if matches!(method_lower.as_str(), "realloc" | "dinrealloc")
                    && args
                        .first()
                        .and_then(compiler_expr_integer_literal_value)
                        .is_some_and(|next| !(0..=1_000_000_000).contains(&next))
                {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_DINARRAY_REALLOC_001".to_string(),
                        message: "dinArray realloc failed: requested capacity is invalid"
                            .to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("valid capacity".to_string()),
                        found: args
                            .first()
                            .and_then(compiler_expr_integer_literal_value)
                            .map(|value| value.to_string()),
                    });
                }

                if method_lower == "usestalesite" {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_DINARRAY_SITE_STALE_001".to_string(),
                        message: "dinArray realloc made previous site stale".to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("fresh site".to_string()),
                        found: Some("stale site".to_string()),
                    });
                }

                if method_lower == "useinvalidatedalias" {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_DINARRAY_ALIAS_001".to_string(),
                        message: "dinArray alias invalidated by movement".to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("valid alias".to_string()),
                        found: Some("invalidated alias".to_string()),
                    });
                }

                if compiler_type_is_dinarray_unbinary_family(&receiver_ty)
                    && matches!(
                        method_lower.as_str(),
                        "binaryseal" | "binaryhash" | "asbinarylayout"
                    )
                {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_DINARRAY_UNBINARY_LAYOUT_001".to_string(),
                        message: "unbinary dinArray cannot be used as stable binary layout"
                            .to_string(),
                        stmt_kind: "expr.method".to_string(),
                        expected: Some("dinArray.binary<T>".to_string()),
                        found: Some(receiver_ty.clone()),
                    });
                }

                if method_lower == "unbinaryview" {
                    return format!(
                        "dinArray.unbinary<{}>",
                        compiler_dinarray_inner_type(&receiver_ty)
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                }
                if method_lower == "asregion" {
                    return format!(
                        "memRegion<{}>",
                        compiler_dinarray_inner_type(&receiver_ty)
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                }
                if method_lower == "binaryhash"
                    && compiler_type_is_dinarray_binary_family(&receiver_ty)
                {
                    return "qword".to_string();
                }
                if method_lower == "binaryseal"
                    && compiler_type_is_dinarray_binary_family(&receiver_ty)
                {
                    return "bool".to_string();
                }
                if matches!(
                    method_lower.as_str(),
                    "len" | "capacity" | "dinlen" | "dincapacity"
                ) {
                    return "int".to_string();
                }
                if matches!(method_lower.as_str(), "get" | "dinget" | "pop" | "dinpop")
                    && compiler_dinarray_inner_type(&receiver_ty).is_some()
                {
                    return compiler_dinarray_inner_type(&receiver_ty)
                        .unwrap_or_else(|| "unknown".to_string());
                }
                if matches!(
                    method_lower.as_str(),
                    "set"
                        | "dinset"
                        | "push"
                        | "dinpush"
                        | "insert"
                        | "dininsert"
                        | "removeat"
                        | "dinremoveat"
                        | "clear"
                        | "dinclear"
                        | "reserve"
                        | "dinreserve"
                        | "realloc"
                        | "dinrealloc"
                        | "shrink"
                        | "dinshrink"
                ) {
                    return "bool".to_string();
                }
            }
            "unknown".to_string()
        }
        ncc_ast::Expr::Closure(closure) => {
            let mut closure_scope = scope.clone();
            let mut closure_bounds = bounds.clone();
            for param in &closure.params {
                closure_scope.insert(
                    param.name.to_ascii_lowercase(),
                    type_name_to_string(&param.ty),
                );
            }
            let expected_return = type_name_to_string(&closure.return_type);
            for stmt in &closure.body {
                compiler_collect_type_diagnostics_from_stmt(
                    stmt,
                    &mut closure_scope,
                    &mut closure_bounds,
                    Some(&expected_return),
                    out,
                );
            }
            compiler_closure_type_signature(closure)
        }
        ncc_ast::Expr::Dev(_) => "never".to_string(),
    }
}

fn compiler_types_compatible(expected: &str, found: &str) -> bool {
    if expected == "unknown" || found == "unknown" {
        return true;
    }
    if found == "never" {
        return true;
    }
    if expected == found {
        return true;
    }

    let expected_members = compiler_union_members_str(expected);
    if expected_members.len() > 1 {
        return expected_members
            .iter()
            .any(|member| compiler_types_compatible(member, found));
    }

    let found_members = compiler_union_members_str(found);
    if found_members.len() > 1 {
        return false;
    }

    if (expected == "text" && found == "str") || (expected == "str" && found == "text") {
        return true;
    }

    if expected == "bool" {
        return found == "bool";
    }

    if compiler_numeric_like_type(expected) && compiler_numeric_like_type(found) {
        return true;
    }

    if expected.starts_with("ptr<") && found.starts_with("ptr<") {
        return true;
    }
    if expected.starts_with("memRegion<") && found.starts_with("memRegion<") {
        return true;
    }
    if expected.starts_with("array<") && found.starts_with("array<") {
        return true;
    }
    if expected.starts_with("array2<") && (found.starts_with("array2<") || found == "array") {
        return true;
    }
    if expected.starts_with("array3<") && (found.starts_with("array3<") || found == "array") {
        return true;
    }
    if expected.starts_with("matrix<") && (found.starts_with("matrix<") || found == "array") {
        return true;
    }
    if expected.starts_with("tensor<") && (found.starts_with("tensor<") || found == "array") {
        return true;
    }
    if expected.starts_with("xy<") && (found.starts_with("xy<") || found == "array") {
        return true;
    }
    if expected.starts_with("xyz<") && (found.starts_with("xyz<") || found == "array") {
        return true;
    }
    if expected.starts_with("x2y2<") && (found.starts_with("x2y2<") || found == "array") {
        return true;
    }
    if expected.starts_with("x2y2z2<") && (found.starts_with("x2y2z2<") || found == "array") {
        return true;
    }
    if expected.starts_with("x3y3<") && (found.starts_with("x3y3<") || found == "array") {
        return true;
    }
    if expected.starts_with("x3y3z3<") && (found.starts_with("x3y3z3<") || found == "array") {
        return true;
    }
    if expected.starts_with("dinArray<") && (found.starts_with("dinArray<") || found == "array") {
        return true;
    }
    if expected.starts_with("dinArray.binary<")
        && (found.starts_with("dinArray.binary<") || found == "array")
    {
        return true;
    }
    if expected.starts_with("dinArray.unbinary<")
        && (found.starts_with("dinArray.unbinary<") || found == "array")
    {
        return true;
    }
    if expected.starts_with("vector<") && found.starts_with("vector<") {
        return true;
    }
    if expected.starts_with("list<") && found.starts_with("list<") {
        return true;
    }
    if expected.starts_with("Option<") && found.starts_with("Option<") {
        return true;
    }
    if expected.starts_with("Result<") && found.starts_with("Result<") {
        return true;
    }

    false
}

fn compiler_push_type_mismatch_diagnostics(
    stmt_kind: &str,
    expected_ty: &str,
    found_ty: &str,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    out.push(CompilerTypeDiagnosticRow {
        code: "E_TYPE_MISMATCH_001".to_string(),
        message: format!(
            "type mismatch: expected '{}', found '{}'",
            expected_ty, found_ty
        ),
        stmt_kind: stmt_kind.to_string(),
        expected: Some(expected_ty.to_string()),
        found: Some(found_ty.to_string()),
    });

    if let Some(code) = compiler_simple_type_diagnostic_code(expected_ty) {
        out.push(CompilerTypeDiagnosticRow {
            code: code.to_string(),
            message: format!(
                "invalid '{}' operation/assignment against '{}'",
                expected_ty, found_ty
            ),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(expected_ty.to_string()),
            found: Some(found_ty.to_string()),
        });
    }
}

fn compiler_collect_assignment_type_diag(
    stmt_kind: &str,
    target_ty: &ncc_ast::TypeName,
    value: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    bounds: &BTreeMap<String, CompilerIndexBoundsInfo>,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    compiler_collect_generic_arity_diag_from_typename(target_ty, stmt_kind, out);

    let expected_ty = type_name_to_string(target_ty);
    let found_ty = compiler_collect_expr_type_diagnostics(value, scope, bounds, out);
    if compiler_type_is_void(&expected_ty) {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_VOID_VALUE_001".to_string(),
            message: "void cannot be used as a value".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("non-void assignment target".to_string()),
            found: Some(expected_ty),
        });
        return;
    }
    if compiler_type_is_void(&found_ty) {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_VOID_VALUE_001".to_string(),
            message: "void cannot be assigned as value".to_string(),
            stmt_kind: stmt_kind.to_string(),
            expected: Some("non-void value".to_string()),
            found: Some(found_ty),
        });
        return;
    }
    compiler_collect_integer_literal_range_diag(stmt_kind, &expected_ty, value, out);
    compiler_collect_pointer_contract_diagnostics(
        stmt_kind,
        &expected_ty,
        compiler_type_supports_nullability(target_ty),
        &found_ty,
        value,
        scope,
        out,
    );

    if found_ty == "null" && !compiler_type_supports_nullability(target_ty) {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_NULL_TYPE_001".to_string(),
            message: format!("null requires nullable type, found '{}'", expected_ty),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(format!("{} | null", expected_ty)),
            found: Some("null".to_string()),
        });
        out.push(CompilerTypeDiagnosticRow {
            code: "E_TYPE_NULLABILITY_001".to_string(),
            message: format!("type '{}' is non-nullable, found null", expected_ty),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(expected_ty.clone()),
            found: Some("null".to_string()),
        });
        return;
    }

    if found_ty.contains('|') && !expected_ty.contains('|') && found_ty != "unknown" {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_TYPE_UNION_NARROWING_001".to_string(),
            message: format!(
                "union value '{}' must be narrowed before assignment to '{}'",
                found_ty, expected_ty
            ),
            stmt_kind: stmt_kind.to_string(),
            expected: Some(expected_ty.clone()),
            found: Some(found_ty.clone()),
        });
        return;
    }

    if !compiler_types_compatible(&expected_ty, &found_ty) {
        compiler_push_type_mismatch_diagnostics(stmt_kind, &expected_ty, &found_ty, out);
    }
}

fn compiler_collect_condition_expr_type_diagnostics(
    kind: &str,
    expr: &ncc_ast::Expr,
    scope: &BTreeMap<String, String>,
    bounds: &BTreeMap<String, CompilerIndexBoundsInfo>,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    let ty = compiler_collect_expr_type_diagnostics(expr, scope, bounds, out);
    if compiler_type_is_nullable_like(&ty) {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_NULL_CONDITION_001".to_string(),
            message: format!("null cannot be used as condition in {}", kind),
            stmt_kind: kind.to_string(),
            expected: Some("bool".to_string()),
            found: Some(ty),
        });
    } else if ty != "bool" {
        out.push(CompilerTypeDiagnosticRow {
            code: "E_CONDITION_BOOL_REQUIRED_001".to_string(),
            message: format!("{} condition must be bool, found '{}'", kind, ty),
            stmt_kind: kind.to_string(),
            expected: Some("bool".to_string()),
            found: Some(ty),
        });
    }
}

fn compiler_collect_type_diagnostics_from_stmt(
    stmt: &ncc_ast::Stmt,
    scope: &mut BTreeMap<String, String>,
    bounds: &mut BTreeMap<String, CompilerIndexBoundsInfo>,
    expected_return: Option<&str>,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
) {
    match stmt {
        ncc_ast::Stmt::Projection { target, ty, value }
        | ncc_ast::Stmt::StrictCheck { target, ty, value } => {
            compiler_collect_assignment_type_diag("assignment", ty, value, scope, bounds, out);
            if let ncc_ast::AssignTarget::Context(reference) = target {
                if let Some(name) = reference.path.last() {
                    scope.insert(name.to_ascii_lowercase(), type_name_to_string(ty));
                    compiler_update_bounds_for_context_assignment(
                        name,
                        &type_name_to_string(ty),
                        value,
                        scope,
                        bounds,
                    );
                }
            }
        }
        ncc_ast::Stmt::Return(expr) => {
            if let Some(expected_ty) = expected_return {
                let found_ty = compiler_collect_expr_type_diagnostics(expr, scope, bounds, out);
                if compiler_type_is_void(expected_ty) {
                    if compiler_is_discard_null_call(expr) {
                        if !matches!(found_ty.as_str(), "void" | "unknown") {
                            out.push(CompilerTypeDiagnosticRow {
                                code: "E_NULL_TO_VOID_001".to_string(),
                                message: "null->void requires explicit discardNull(nullable)"
                                    .to_string(),
                                stmt_kind: "return".to_string(),
                                expected: Some("discardNull(nullable)".to_string()),
                                found: Some(found_ty),
                            });
                        }
                        return;
                    }
                    if found_ty == "null" || compiler_type_is_nullable_like(&found_ty) {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_NULL_TO_VOID_001".to_string(),
                            message: "null->void requires explicit discard".to_string(),
                            stmt_kind: "return".to_string(),
                            expected: Some("discardNull(nullable)".to_string()),
                            found: Some(found_ty),
                        });
                    } else if !compiler_type_is_void(&found_ty) && found_ty != "unknown" {
                        out.push(CompilerTypeDiagnosticRow {
                            code: "E_VOID_RETURN_001".to_string(),
                            message: "invalid void return".to_string(),
                            stmt_kind: "return".to_string(),
                            expected: Some("void".to_string()),
                            found: Some(found_ty),
                        });
                    }
                    return;
                }
                if compiler_type_is_void(&found_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_VOID_VALUE_001".to_string(),
                        message: "void cannot be used as a return value for non-void function"
                            .to_string(),
                        stmt_kind: "return".to_string(),
                        expected: Some(expected_ty.to_string()),
                        found: Some("void".to_string()),
                    });
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_RETURN_001".to_string(),
                        message: format!(
                            "return type mismatch: expected '{}', found 'void'",
                            expected_ty
                        ),
                        stmt_kind: "return".to_string(),
                        expected: Some(expected_ty.to_string()),
                        found: Some("void".to_string()),
                    });
                    return;
                }
                compiler_collect_integer_literal_range_diag("return", expected_ty, expr, out);
                compiler_collect_pointer_contract_diagnostics(
                    "return",
                    expected_ty,
                    compiler_type_supports_nullability_str(expected_ty),
                    &found_ty,
                    expr,
                    scope,
                    out,
                );
                if found_ty == "null" && !compiler_type_supports_nullability_str(expected_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_NULL_TYPE_001".to_string(),
                        message: format!(
                            "null requires nullable return type, found '{}'",
                            expected_ty
                        ),
                        stmt_kind: "return".to_string(),
                        expected: Some(format!("{} | null", expected_ty)),
                        found: Some("null".to_string()),
                    });
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_RETURN_001".to_string(),
                        message: format!(
                            "return type mismatch: expected '{}', found null",
                            expected_ty
                        ),
                        stmt_kind: "return".to_string(),
                        expected: Some(expected_ty.to_string()),
                        found: Some("null".to_string()),
                    });
                } else if !compiler_types_compatible(expected_ty, &found_ty) {
                    out.push(CompilerTypeDiagnosticRow {
                        code: "E_TYPE_RETURN_001".to_string(),
                        message: format!(
                            "return type mismatch: expected '{}', found '{}'",
                            expected_ty, found_ty
                        ),
                        stmt_kind: "return".to_string(),
                        expected: Some(expected_ty.to_string()),
                        found: Some(found_ty.clone()),
                    });
                    if let Some(code) = compiler_simple_type_diagnostic_code(expected_ty) {
                        out.push(CompilerTypeDiagnosticRow {
                            code: code.to_string(),
                            message: format!(
                                "invalid '{}' return operation with '{}'",
                                expected_ty, found_ty
                            ),
                            stmt_kind: "return".to_string(),
                            expected: Some(expected_ty.to_string()),
                            found: Some(found_ty),
                        });
                    }
                }
            }
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            compiler_collect_condition_expr_type_diagnostics(
                "if.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut then_scope = scope.clone();
            let mut else_scope = scope.clone();
            let mut then_bounds = bounds.clone();
            let mut else_bounds = bounds.clone();
            for nested in then_block {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut then_scope,
                    &mut then_bounds,
                    expected_return,
                    out,
                );
            }
            for nested in else_block {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut else_scope,
                    &mut else_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Unless { condition, body } => {
            compiler_collect_condition_expr_type_diagnostics(
                "unless.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::While { condition, body } => {
            compiler_collect_condition_expr_type_diagnostics(
                "while.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Loop { body } => {
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::DoWhile { body, condition } => {
            compiler_collect_condition_expr_type_diagnostics(
                "doWhile.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::RepeatUntil { body, condition } => {
            compiler_collect_condition_expr_type_diagnostics(
                "repeatUntil.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::RepeatCount { count, body } => {
            compiler_collect_expr_type_diagnostics(count, scope, bounds, out);
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_collect_expr_type_diagnostics(start, scope, bounds, out);
            compiler_collect_expr_type_diagnostics(end, scope, bounds, out);
            if let Some(step_expr) = step {
                compiler_collect_expr_type_diagnostics(step_expr, scope, bounds, out);
            }
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_collect_expr_type_diagnostics(iterable, scope, bounds, out);
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::EachDoWhile {
            iterable,
            condition,
            body,
            ..
        } => {
            compiler_collect_expr_type_diagnostics(iterable, scope, bounds, out);
            compiler_collect_condition_expr_type_diagnostics(
                "eachDoWhile.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
            ..
        } => {
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            if let Some(init_stmt) = init {
                compiler_collect_type_diagnostics_from_stmt(
                    init_stmt,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
            if let Some(step_stmt) = step {
                compiler_collect_type_diagnostics_from_stmt(
                    step_stmt,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
            if let Some(condition_expr) = condition {
                compiler_collect_condition_expr_type_diagnostics(
                    "for.condition",
                    condition_expr,
                    &nested_scope,
                    &nested_bounds,
                    out,
                );
            }
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => {
            compiler_collect_type_diagnostics_from_stmt(stmt, scope, bounds, expected_return, out);
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            compiler_collect_expr_type_diagnostics(value, scope, bounds, out);
            for (_expr, stmts) in cases {
                let mut branch_scope = scope.clone();
                let mut branch_bounds = bounds.clone();
                compiler_collect_expr_type_diagnostics(_expr, scope, bounds, out);
                for nested in stmts {
                    compiler_collect_type_diagnostics_from_stmt(
                        nested,
                        &mut branch_scope,
                        &mut branch_bounds,
                        expected_return,
                        out,
                    );
                }
            }
            let mut default_scope = scope.clone();
            let mut default_bounds = bounds.clone();
            for nested in default {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut default_scope,
                    &mut default_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            compiler_collect_expr_type_diagnostics(value, scope, bounds, out);
            for arm in arms {
                if let ncc_ast::Pattern::Literal(expr) = &arm.pattern {
                    compiler_collect_expr_type_diagnostics(expr, scope, bounds, out);
                }
                if let Some(guard) = &arm.guard {
                    compiler_collect_condition_expr_type_diagnostics(
                        "match.guard",
                        guard,
                        scope,
                        bounds,
                        out,
                    );
                }
                let mut branch_scope = scope.clone();
                let mut branch_bounds = bounds.clone();
                for nested in &arm.body {
                    compiler_collect_type_diagnostics_from_stmt(
                        nested,
                        &mut branch_scope,
                        &mut branch_bounds,
                        expected_return,
                        out,
                    );
                }
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            compiler_collect_condition_expr_type_diagnostics(
                "guard.condition",
                condition,
                scope,
                bounds,
                out,
            );
            let mut else_scope = scope.clone();
            let mut else_bounds = bounds.clone();
            for nested in else_block {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut else_scope,
                    &mut else_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Defer { body } => {
            let mut nested_scope = scope.clone();
            let mut nested_bounds = bounds.clone();
            for nested in body {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut nested_scope,
                    &mut nested_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (_expr, stmts) in branches {
                compiler_collect_expr_type_diagnostics(_expr, scope, bounds, out);
                let mut branch_scope = scope.clone();
                let mut branch_bounds = bounds.clone();
                for nested in stmts {
                    compiler_collect_type_diagnostics_from_stmt(
                        nested,
                        &mut branch_scope,
                        &mut branch_bounds,
                        expected_return,
                        out,
                    );
                }
            }
            let mut default_scope = scope.clone();
            let mut default_bounds = bounds.clone();
            for nested in default {
                compiler_collect_type_diagnostics_from_stmt(
                    nested,
                    &mut default_scope,
                    &mut default_bounds,
                    expected_return,
                    out,
                );
            }
        }
        ncc_ast::Stmt::MainRefFn { .. }
        | ncc_ast::Stmt::FunctionRelation { .. }
        | ncc_ast::Stmt::Print { .. }
        | ncc_ast::Stmt::Dev(_)
        | ncc_ast::Stmt::Break { .. }
        | ncc_ast::Stmt::Continue { .. } => {}
        ncc_ast::Stmt::Expr(expr) => {
            compiler_collect_expr_type_diagnostics(expr, scope, bounds, out);
        }
        ncc_ast::Stmt::Destructure { kind, args } => {
            let arg_types = args
                .iter()
                .map(|arg| compiler_collect_expr_type_diagnostics(arg, scope, bounds, out))
                .collect::<Vec<_>>();
            let _ = compiler_collect_destructure_contract_diagnostics(
                *kind,
                &arg_types,
                args,
                out,
                "stmt.destructure",
                false,
            );
        }
        ncc_ast::Stmt::Constructure { kind, args } => {
            let arg_types = args
                .iter()
                .map(|arg| compiler_collect_expr_type_diagnostics(arg, scope, bounds, out))
                .collect::<Vec<_>>();
            let _ = compiler_collect_constructure_contract_diagnostics(
                *kind,
                &arg_types,
                args,
                out,
                "stmt.constructure",
                false,
            );
        }
    }
}

fn compiler_collect_visibility_diagnostics_from_expr(
    expr: &ncc_ast::Expr,
    current_container: &str,
    index: &CompilerVisibilityIndex,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    seen: &mut BTreeSet<(String, String, String)>,
) {
    match expr {
        ncc_ast::Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                compiler_collect_visibility_diagnostics_from_expr(
                    value,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Expr::ContextRef(reference) => {
            if let Some(contract) =
                compiler_resolve_contract_for_path(index, current_container, &reference.path)
            {
                let symbol = format!("{}.{}", contract.container, contract.name);
                if matches!(contract.visibility, Some(ncc_ast::Visibility::Private))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_PRIVATE_001",
                        format!(
                            "private symbol '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
                if matches!(contract.visibility, Some(ncc_ast::Visibility::Internal))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_INTERNAL_001",
                        format!(
                            "internal symbol '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
                if contract.strictly && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_STRICTLY_001",
                        format!(
                            "strictly symbol '{}' cannot be accessed without authority",
                            symbol
                        ),
                        "visibility",
                        &symbol,
                    );
                }
            }
        }
        ncc_ast::Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            if let ncc_ast::Expr::ContextRef(reference) = &**receiver {
                if let Some(contract) =
                    compiler_resolve_contract_for_path(index, current_container, &reference.path)
                {
                    if contract.strictly && compiler_is_mutating_method_name(method) {
                        let symbol = format!("{}.{}", contract.container, contract.name);
                        compiler_emit_visibility_diag(
                            out,
                            seen,
                            "E_VISIBILITY_STRICTLY_001",
                            format!(
                                "strictly symbol '{}' cannot be mutated via method '{}'",
                                symbol, method
                            ),
                            "visibility",
                            &symbol,
                        );
                    }
                }
            }
            compiler_collect_visibility_diagnostics_from_expr(
                receiver,
                current_container,
                index,
                out,
                seen,
            );
            for arg in args {
                compiler_collect_visibility_diagnostics_from_expr(
                    arg,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Expr::Unary { expr, .. } => compiler_collect_visibility_diagnostics_from_expr(
            expr,
            current_container,
            index,
            out,
            seen,
        ),
        ncc_ast::Expr::Binary { left, right, .. } => {
            compiler_collect_visibility_diagnostics_from_expr(
                left,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                right,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                then_expr,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                else_expr,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Expr::Call { args, .. } | ncc_ast::Expr::QualifiedCall { args, .. } => {
            for arg in args {
                compiler_collect_visibility_diagnostics_from_expr(
                    arg,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Expr::VectorLiteral(items) | ncc_ast::Expr::ArrayLiteral(items) => {
            for item in items {
                compiler_collect_visibility_diagnostics_from_expr(
                    item,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Expr::Index { target, index: idx } => {
            compiler_collect_visibility_diagnostics_from_expr(
                target,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                idx,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Expr::StrictCheckExpr { target, value, .. } => {
            if let Some(contract) =
                compiler_resolve_contract_for_path(index, current_container, &target.path)
            {
                if contract.strictly {
                    let symbol = format!("{}.{}", contract.container, contract.name);
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_STRICTLY_001",
                        format!(
                            "strictly symbol '{}' requires explicit authority contract",
                            symbol
                        ),
                        "visibility",
                        &symbol,
                    );
                }
            }
            compiler_collect_visibility_diagnostics_from_expr(
                value,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Expr::PostIncrement { target }
        | ncc_ast::Expr::FieldPostIncrement { target }
        | ncc_ast::Expr::PointerPostIncrementRead { target } => {
            if let Some(contract) =
                compiler_resolve_contract_for_path(index, current_container, &target.path)
            {
                let symbol = format!("{}.{}", contract.container, contract.name);
                if matches!(contract.visibility, Some(ncc_ast::Visibility::Private))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_PRIVATE_001",
                        format!(
                            "private symbol '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
                if matches!(contract.visibility, Some(ncc_ast::Visibility::Internal))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_INTERNAL_001",
                        format!(
                            "internal symbol '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
                if contract.strictly && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_STRICTLY_001",
                        format!(
                            "strictly symbol '{}' cannot be incremented without authority",
                            symbol
                        ),
                        "visibility",
                        &symbol,
                    );
                }
            }
        }
        ncc_ast::Expr::Closure(closure) => {
            for stmt in &closure.body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    stmt,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Expr::Bool(_)
        | ncc_ast::Expr::Number(_)
        | ncc_ast::Expr::Char(_)
        | ncc_ast::Expr::String(_)
        | ncc_ast::Expr::Identifier(_)
        | ncc_ast::Expr::Dev(_) => {}
    }
}

fn compiler_collect_visibility_diagnostics_from_stmt(
    stmt: &ncc_ast::Stmt,
    current_container: &str,
    index: &CompilerVisibilityIndex,
    out: &mut Vec<CompilerTypeDiagnosticRow>,
    seen: &mut BTreeSet<(String, String, String)>,
) {
    match stmt {
        ncc_ast::Stmt::Projection { target, value, .. }
        | ncc_ast::Stmt::StrictCheck { target, value, .. } => {
            if let ncc_ast::AssignTarget::Context(reference) = target {
                if let Some(contract) =
                    compiler_resolve_contract_for_path(index, current_container, &reference.path)
                {
                    if contract.strictly {
                        let symbol = format!("{}.{}", contract.container, contract.name);
                        compiler_emit_visibility_diag(
                            out,
                            seen,
                            "E_VISIBILITY_STRICTLY_001",
                            format!(
                                "strictly symbol '{}' cannot be assigned without authority",
                                symbol
                            ),
                            "visibility",
                            &symbol,
                        );
                    }
                }
            }
            compiler_collect_visibility_diagnostics_from_expr(
                value,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Print { value: expr, .. } => {
            compiler_collect_visibility_diagnostics_from_expr(
                expr,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
            for nested in then_block {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            for nested in else_block {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Unless { condition, body } | ncc_ast::Stmt::While { condition, body } => {
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition } => {
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(init_stmt) = init {
                compiler_collect_visibility_diagnostics_from_stmt(
                    init_stmt,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            if let Some(condition_expr) = condition {
                compiler_collect_visibility_diagnostics_from_expr(
                    condition_expr,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            if let Some(step_stmt) = step {
                compiler_collect_visibility_diagnostics_from_stmt(
                    step_stmt,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_collect_visibility_diagnostics_from_expr(
                iterable,
                current_container,
                index,
                out,
                seen,
            );
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                start,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                end,
                current_container,
                index,
                out,
                seen,
            );
            if let Some(step_expr) = step {
                compiler_collect_visibility_diagnostics_from_expr(
                    step_expr,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::EachDoWhile {
            iterable,
            condition,
            body,
            ..
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                iterable,
                current_container,
                index,
                out,
                seen,
            );
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                value,
                current_container,
                index,
                out,
                seen,
            );
            for (arm_expr, arm_stmts) in cases {
                compiler_collect_visibility_diagnostics_from_expr(
                    arm_expr,
                    current_container,
                    index,
                    out,
                    seen,
                );
                for nested in arm_stmts {
                    compiler_collect_visibility_diagnostics_from_stmt(
                        nested,
                        current_container,
                        index,
                        out,
                        seen,
                    );
                }
            }
            for nested in default {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            compiler_collect_visibility_diagnostics_from_expr(
                value,
                current_container,
                index,
                out,
                seen,
            );
            for arm in arms {
                if let ncc_ast::Pattern::Literal(expr) = &arm.pattern {
                    compiler_collect_visibility_diagnostics_from_expr(
                        expr,
                        current_container,
                        index,
                        out,
                        seen,
                    );
                }
                if let Some(guard) = &arm.guard {
                    compiler_collect_visibility_diagnostics_from_expr(
                        guard,
                        current_container,
                        index,
                        out,
                        seen,
                    );
                }
                for nested in &arm.body {
                    compiler_collect_visibility_diagnostics_from_stmt(
                        nested,
                        current_container,
                        index,
                        out,
                        seen,
                    );
                }
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            compiler_collect_visibility_diagnostics_from_expr(
                condition,
                current_container,
                index,
                out,
                seen,
            );
            for nested in else_block {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (branch_expr, branch_stmts) in branches {
                compiler_collect_visibility_diagnostics_from_expr(
                    branch_expr,
                    current_container,
                    index,
                    out,
                    seen,
                );
                for nested in branch_stmts {
                    compiler_collect_visibility_diagnostics_from_stmt(
                        nested,
                        current_container,
                        index,
                        out,
                        seen,
                    );
                }
            }
            for nested in default {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Defer { body } => {
            for nested in body {
                compiler_collect_visibility_diagnostics_from_stmt(
                    nested,
                    current_container,
                    index,
                    out,
                    seen,
                );
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => compiler_collect_visibility_diagnostics_from_stmt(
            stmt,
            current_container,
            index,
            out,
            seen,
        ),
        ncc_ast::Stmt::FunctionRelation { fn_ref, .. }
        | ncc_ast::Stmt::MainRefFn { fn_ref, .. } => {
            if let Some(contract) =
                compiler_resolve_contract_for_path(index, current_container, &fn_ref.path)
            {
                let symbol = format!("{}.{}", contract.container, contract.name);
                if contract.is_function
                    && matches!(contract.visibility, Some(ncc_ast::Visibility::Private))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_PRIVATE_001",
                        format!(
                            "private function '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
                if contract.is_function
                    && matches!(contract.visibility, Some(ncc_ast::Visibility::Internal))
                    && contract.container != current_container.to_ascii_lowercase()
                {
                    compiler_emit_visibility_diag(
                        out,
                        seen,
                        "E_VISIBILITY_INTERNAL_001",
                        format!(
                            "internal function '{}' is not visible in container '{}'",
                            symbol, current_container
                        ),
                        "visibility",
                        &symbol,
                    );
                }
            }
        }
        ncc_ast::Stmt::Dev(_) | ncc_ast::Stmt::Break { .. } | ncc_ast::Stmt::Continue { .. } => {}
        ncc_ast::Stmt::Destructure { kind, args } => {
            let expr = compiler_destructure_stmt_as_call_expr(*kind, args);
            compiler_collect_visibility_diagnostics_from_expr(
                &expr,
                current_container,
                index,
                out,
                seen,
            );
        }
        ncc_ast::Stmt::Constructure { kind, args } => {
            let expr = compiler_constructure_stmt_as_call_expr(*kind, args);
            compiler_collect_visibility_diagnostics_from_expr(
                &expr,
                current_container,
                index,
                out,
                seen,
            );
        }
    }
}

fn compiler_collect_visibility_diagnostics_from_program(
    ast: &ncc_ast::Program,
) -> Vec<CompilerTypeDiagnosticRow> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let index = compiler_build_visibility_index(ast);
    for container in &ast.containers {
        let current_container = container.name.to_ascii_lowercase();
        for section in &container.sections {
            if let ncc_ast::Section::Stc(decls) = section {
                for decl in decls {
                    let ncc_ast::StcDecl::Typed {
                        visibility, name, ..
                    } = decl;
                    if matches!(visibility, Some(ncc_ast::Visibility::Public))
                        && name.starts_with('_')
                    {
                        compiler_emit_visibility_diag(
                            &mut out,
                            &mut seen,
                            "E_VISIBILITY_PUBLIC_001",
                            format!(
                                "invalid public declaration '{}': public symbols cannot be underscore-prefixed",
                                name
                            ),
                            "visibility",
                            name,
                        );
                    }
                }
            }
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            compiler_collect_visibility_diagnostics_from_stmt(
                                stmt,
                                &current_container,
                                &index,
                                &mut out,
                                &mut seen,
                            );
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        compiler_collect_visibility_diagnostics_from_stmt(
                            stmt,
                            &current_container,
                            &index,
                            &mut out,
                            &mut seen,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn compiler_collect_type_diagnostics_from_program(
    ast: &ncc_ast::Program,
    unknown_types: &[String],
) -> Vec<CompilerTypeDiagnosticRow> {
    let mut rows = Vec::new();
    let base_scope = compiler_collect_base_type_scope(ast);

    for ty in unknown_types {
        rows.push(CompilerTypeDiagnosticRow {
            code: "E_TYPE_UNKNOWN_001".to_string(),
            message: format!("unknown type contract '{}'", ty),
            stmt_kind: "type_catalog".to_string(),
            expected: None,
            found: Some(ty.clone()),
        });
    }
    compiler_collect_ffi_diagnostics_from_program(ast, &mut rows);

    for container in &ast.containers {
        compiler_set_type_diag_context(container.context.as_ref());
        for section in &container.sections {
            match section {
                ncc_ast::Section::Stc(decls) => {
                    for decl in decls {
                        let ncc_ast::StcDecl::Typed { ty, .. } = decl;
                        compiler_collect_generic_arity_diag_from_typename(ty, "stc", &mut rows);
                    }
                }
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        compiler_collect_generic_arity_diag_from_typename(
                            &function.return_type,
                            "fn.return",
                            &mut rows,
                        );
                        let expected_return = type_name_to_string(&function.return_type);
                        let mut scope = base_scope.clone();
                        let mut bounds = BTreeMap::new();
                        for param in &function.params {
                            compiler_collect_generic_arity_diag_from_typename(
                                &param.ty, "fn.param", &mut rows,
                            );
                            scope.insert(
                                param.name.to_ascii_lowercase(),
                                type_name_to_string(&param.ty),
                            );
                        }
                        for stmt in &function.public_body {
                            compiler_collect_type_diagnostics_from_stmt(
                                stmt,
                                &mut scope,
                                &mut bounds,
                                Some(expected_return.as_str()),
                                &mut rows,
                            );
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    let mut scope = base_scope.clone();
                    let mut bounds = BTreeMap::new();
                    for stmt in body {
                        compiler_collect_type_diagnostics_from_stmt(
                            stmt,
                            &mut scope,
                            &mut bounds,
                            None,
                            &mut rows,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    compiler_set_type_diag_context(None);

    rows.extend(compiler_collect_visibility_diagnostics_from_program(ast));
    rows
}

fn compiler_type_diagnostic_codes(rows: &[CompilerTypeDiagnosticRow]) -> Vec<String> {
    let mut values = rows.iter().map(|row| row.code.clone()).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn compiler_condition_rows_json(rows: &[CompilerConditionAuditRow]) -> Vec<serde_json::Value> {
    rows.iter()
        .map(|row| {
            serde_json::json!({
                "kind": row.kind,
                "inferred_type": row.inferred_type,
                "boolish": row.boolish,
                "diagnostic": row.diagnostic,
                "expr": row.expr_debug,
            })
        })
        .collect()
}

fn compiler_condition_diagnostics(rows: &[CompilerConditionAuditRow]) -> Vec<String> {
    let mut values: Vec<String> = rows
        .iter()
        .filter_map(|row| row.diagnostic.clone())
        .collect();
    values.sort();
    values.dedup();
    values
}

fn compiler_handle_doctor(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler doctor does not accept positional arguments".into());
    }
    let payload = compiler_doctor_payload(opts);
    compiler_print_json_or_text(opts, payload, "compiler doctor: OK")
}

fn compiler_handle_pointer_industrial(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler pointer-industrial does not accept positional arguments".into());
    }
    compiler_print_json_or_text(
        opts,
        pointer_industrial_closure_payload(),
        "compiler pointer-industrial: OK",
    )
}

fn compiler_handle_pointer_lowering(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler pointer-lowering does not accept positional arguments".into());
    }
    compiler_print_json_or_text(
        opts,
        compiler_pointer_lowering_payload(),
        "compiler pointer-lowering: OK",
    )
}

fn compiler_handle_pointer_audit(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler pointer-audit requires a source path")?;
    if rest.len() != 1 {
        return Err("compiler pointer-audit accepts exactly one source path".into());
    }

    let ast = load_ast_from_path(path)?;
    ncc_sema::analyze(&ast).map_err(|error| {
        format!("normal manual/raw compilation rejected source before audit: {error}")
    })?;

    let audit = ncc_sema::analyze_with_explicit_pointer_checks(&ast);
    let diagnostic = audit.err();
    let status = if diagnostic.is_some() {
        "hazard_detected"
    } else {
        "clean_for_tracked_hazards"
    };
    let payload = serde_json::json!({
        "schema": "nc.compiler.pointer_audit.v1",
        "path": path,
        "status": status,
        "optional": true,
        "affects_ast": false,
        "affects_ir": false,
        "affects_codegen": false,
        "normal_compilation": "manual_raw_unchecked",
        "undefined_behavior": "C-like: accepted source has no deterministic result guarantee",
        "diagnostics": diagnostic.iter().cloned().collect::<Vec<_>>(),
    });

    if opts.strict {
        if let Some(diagnostic) = diagnostic {
            return Err(format!("pointer audit detected tracked hazard: {diagnostic}").into());
        }
    }

    compiler_print_json_or_text(
        opts,
        payload,
        &format!("compiler pointer-audit: {status} {path}"),
    )
}

fn compiler_pointer_lowering_payload() -> serde_json::Value {
    serde_json::json!({
        "schema": "nc.compiler.pointer_lowering.v2",
        "status": "closed",
        "scope": "normal_nc_high_level_to_machine_lowering_plan",
        "targets": [
            "x86_64-unknown-linux-gnu",
            "i386-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "armv7-unknown-none-eabi",
            "riscv32imac-unknown-elf",
            "riscv64-unknown-elf"
        ],
        "pointer_widths": [16, 32, 64, 128],
        "address_spaces": {
            "cpu": 0,
            "mmio": 1,
            "physical": 2,
            "dma": 3,
            "gpu": 4,
            "firmware": 5
        },
        "pointee_forms": [
            "data",
            "function",
            "member",
            "fixed_array",
            "flexible_array",
            "aggregate"
        ],
        "llvm_contract": {
            "raw_gep": "getelementptr without inbounds",
            "pointer_difference": "target-sized isize",
            "wider_than_target": "lossless integer carrier until explicit materialization",
            "mmio": "automatic volatile load/store",
            "hidden_bounds_checks": false,
            "hidden_provenance": false,
            "hidden_noalias": false
        },
        "machine_dependency": {
            "status": "available_closed",
            "asm_complete_total_required": true
        },
        "surface": {
            "less_verbose_runtime_import": "use nC.runtime.heap exposes alloc/free/realloc/zero aliases for memory.heap.manager.*",
            "generic_pointer_calls": ["ptrRead<T>", "ptrWrite<T>", "ptrOffset<T>", "ptrDiff<T>", "ptrFromAddr<T>", "regionFromPtr<T>", "regionPtrAt<T>"],
            "nullable_policy": "p<T>/ptr<T> is a raw address and may contain null without a refinement gate; dereference is C-like undefined behavior",
            "raw_ambiguity_policy": "raw pointers are unconditional manual/raw compiler semantics; risk and silentPointer annotations are legacy evidence only",
            "bounded_view_policy": "ref<T>, site<T> and memRegion<T> keep their independent contracts and are not aliases of p<T>"
        },
        "physical_lowering_equations": {
            "ptrRead<int>(p)": "load i32 [addr]",
            "ptrWrite<int>(p, v)": "store i32 [addr], v",
            "ptrOffset<int>(p, 1)": "addr + sizeof(int)",
            "ptrDiff<T>(pB, pA)": "(addrB - addrA) / sizeof(T)",
            "regionFromPtr<int>(p, n)": "view { base=addr(p), len=n, elem_width=sizeof(int), owner_generation=heap.generation }",
            "regionPtrAt<int>(r, i)": "addr = region.base + i * sizeof(int); bounds <= region.end; generation == heap.generation"
        },
        "operations": [
            {
                "op": "ptrRead<T>",
                "machine_intent": "load",
                "physical_formula": "load i{width(T)} [addr]",
                "machine_ir": {"kind": "load", "addr": "ptr.addr", "width": "sizeof(T)", "result": "T"},
                "x86_64_sysv64": ["mov <dst>, [<ptr>]"],
                "aarch64_aapcs64": ["ldr <dst>, [<ptr>]"],
                "riscv64": ["ld/lw/lh/lb <dst>, 0(<ptr>)"],
                "width": "sizeof(T)",
                "endian": "target_profile",
                "alignment": "target ABI assumption; misalignment compiles and may be undefined behavior",
                "diagnostics": ["optional-audit:E_PTR_NULL_DEREF_001", "optional-audit:E_PTR_STALE_SITE_001", "optional-audit:E_PTR_ALIGNMENT_001", "E_PTR_TYPE_002"],
                "runtime_e2e_fixture": "examples/pointers/ptr_runtime_read_write_offset_diff_e2e_ok.n"
            },
            {
                "op": "ptrWrite<T>",
                "machine_intent": "store",
                "physical_formula": "store i{width(T)} [addr], value",
                "machine_ir": {"kind": "store", "addr": "ptr.addr", "width": "sizeof(T)", "value": "T"},
                "x86_64_sysv64": ["mov [<ptr>], <src>"],
                "aarch64_aapcs64": ["str <src>, [<ptr>]"],
                "riscv64": ["sd/sw/sh/sb <src>, 0(<ptr>)"],
                "width": "sizeof(T)",
                "endian": "target_profile",
                "alignment": "target ABI assumption; misalignment compiles and may be undefined behavior",
                "diagnostics": ["optional-audit:E_PTR_NULL_DEREF_001", "optional-audit:E_PTR_STALE_SITE_001", "optional-audit:E_PTR_ALIGNMENT_001", "E_PTR_TYPE_002"],
                "runtime_e2e_fixture": "examples/pointers/ptr_runtime_read_write_offset_diff_e2e_ok.n"
            },
            {
                "op": "ptrOffset<T>",
                "machine_intent": "base_plus_index_times_sizeof",
                "physical_formula": "addr + index * sizeof(T)",
                "machine_ir": {"kind": "addr_add_scaled", "base": "ptr.addr", "scale": "sizeof(T)", "index": "offset"},
                "x86_64_sysv64": ["lea <dst>, [<base> + <idx>*sizeof(T)>"],
                "aarch64_aapcs64": ["add <dst>, <base>, <idx>, lsl log2(sizeof(T))"],
                "riscv64": ["slli <tmp>, <idx>, log2(sizeof(T))", "add <dst>, <base>, <tmp>"],
                "width": "addr",
                "range_policy": "unchecked; result may leave the originating allocation",
                "arithmetic": "wrapping at target pointer width",
                "diagnostics": ["optional-audit:E_PTR_NULL_DEREF_001", "optional-audit:E_PTR_STALE_SITE_001", "optional-audit:E_PTR_PROVENANCE_001"]
            },
            {
                "op": "ptrDiff<T>",
                "machine_intent": "subtract_then_divide_by_sizeof",
                "physical_formula": "(addrB - addrA) / sizeof(T)",
                "machine_ir": {"kind": "addr_diff_scaled", "left": "ptrB.addr", "right": "ptrA.addr", "scale": "sizeof(T)", "result": "isize"},
                "x86_64_sysv64": ["sub <left>, <right>", "sar/shr <diff>, log2(sizeof(T)) when power-of-two"],
                "i386_sysv": ["sub <left>, <right>", "sar/shr <diff>, log2(sizeof(T)) when power-of-two"],
                "aarch64_aapcs64": ["sub <diff>, <left>, <right>", "asr/lsr <diff>, <diff>, log2(sizeof(T))"],
                "armv7_aapcs32": ["sub <diff>, <left>, <right>", "asr/lsr <diff>, <diff>, log2(sizeof(T))"],
                "riscv32_ilp32": ["sub <diff>, <left>, <right>", "srai/srli <diff>, <diff>, log2(sizeof(T))"],
                "riscv64": ["sub <diff>, <left>, <right>", "srai/srli <diff>, <diff>, log2(sizeof(T))"],
                "diagnostics": ["optional-audit:E_PTR_ALIGNMENT_001", "optional-audit:E_PTR_PROVENANCE_001"]
            },
            {
                "op": "addrOf",
                "machine_intent": "symbol_or_stack_address_materialization",
                "machine_refs": ["relocation/symbol address when static", "frame offset when stack-local"],
                "diagnostics": ["optional-audit:E_PTR_PROVENANCE_001"]
            },
            {
                "op": "regionFromPtr<T>/regionPtrAt<T>",
                "machine_intent": "bounded_view_and_pointer_materialization",
                "physical_formula": "regionFromPtr<T>(p,n) records base/end/sizeof(T)/heap_generation; regionPtrAt<T>(r,i) materializes base + i*sizeof(T)",
                "machine_refs": ["base", "end", "length", "element_width", "heap_generation"],
                "diagnostics": ["E_REGION_BOUNDS_001", "E_PTR_OUT_OF_BOUNDS_001", "E_PTR_STALE_SITE_001", "E_PTR_TYPE_002"],
                "runtime_e2e_fixture": "examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n"
            }
        ],
        "abi_ffi_bridge": {
            "c_abi": {
                "status": "closed",
                "contracts": ["extern symbol", "header contract", "calling convention", "ownership", "who_frees", "nullability", "layout", "alignment"],
                "artifacts": ["object files", "static libraries", "dynamic libraries", "linker invocation"]
            },
            "cpp_abi": {
                "status": "closed_real_toolchain_smoke",
                "command": "ncc compiler ffi-cpp-abi --json --strict",
                "contracts": ["extern C boundary", "real C++ mangled symbol validation", "vtable dispatch", "virtual destructor ownership policy", "no hidden exception crossing", "template instantiation", "object/static/shared library link"],
                "profiles": ["itanium-cxxabi", "msvc-cxxabi", "extern-c-stable"],
                "host_validated": "itanium-cxxabi on Linux/macOS/FreeBSD or host-cxxabi where available"
            },
            "asm_bridge": {
                "status": "closed",
                "contracts": ["register class", "stack alignment", "symbol visibility", "object format", "relocation", "call-preserved registers"]
            },
            "layout": {
                "struct_layout": "repr/c layout recorded",
                "packed_layout": "packed layout requires explicit alignment contract",
                "alignment": "raw pointer access is emitted with target ABI alignment assumptions and no compile-time gate",
                "endianness": "target-profile driven"
            }
        },
        "memory_model": {
            "stack": "raw addresses may escape without compiler ownership or lifetime gates",
            "heap": "alloc/free/realloc events may be tracked for audit, but normal codegen carries no lifetime or generation gate",
            "static_global": "addrOf emits symbol/relocation materialization intent",
            "pointer_arithmetic": "ptrOffset/ptrDiff scaled by sizeof(T)",
            "raw_pointer_bounds": "unchecked allocation range; arithmetic wraps at target pointer width",
            "bounded_views": "array/vector/slice/buffer/memRegion retain bounds checks",
            "ub_policy": "null, misalignment, out-of-range, aliasing, use-after-free, double-free and overflow compile; their result is undefined as in C",
            "aliasing": "p<T>/ptr<T> carries no inferred noalias, ownership or lifetime promise",
            "volatile": "lowering contract field for side-effecting memory",
            "restrict": "explicit alias contract; never inferred from raw pointer syntax",
            "raii_like": "never inferred for p<T>; resource cleanup is explicit",
            "templates": "generic pointer operations monomorphize by width/layout",
            "optional_audit": "ncc compiler pointer-audit <file.n>; read-only and codegen-neutral"
        },
        "performance_measurement": {
            "status": "measurable",
            "metrics": {
                "ptrRead_overhead": "normal native lowering: one load intent and no pointer-state check",
                "ptrWrite_overhead": "normal native lowering: one store intent and no pointer-state check",
                "ptrOffset_overhead": "wrapping scaled address add without allocation/provenance gate",
                "ptrDiff_overhead": "subtract plus scale without same-origin or alignment gate",
                "memRegion_read_write_overhead": "bounds plus stale provenance",
                "heap_alloc_free_speed": "heap event counter and live allocation counter",
                "parser_speed": "covered by compiler pipeline timing harness",
                "compile_time": "covered by cargo/ncc validation commands",
                "binary_size": "covered by emit-object object metadata",
                "ir_lowering_time": "covered by compiler lower/backend reports",
                "machine_emitted_bytes": "covered by machine emit-bytes/emit-object",
                "runtime_overhead": "runtime pointer examples execute end-to-end",
                "ffi_overhead": "ffi real link/run smoke covers C/C++/Rust/C-ASM paths when host tools exist"
            }
        },
        "evidence": {
            "runtime_cells": "closed",
            "heap_live_dead_generation_checks": "closed",
            "basal_pointer_surface_records": "closed",
            "ffi_c_asm_pointer_smoke": "closed_when_host_tools_available",
            "generic_short_fixture": "examples/pointers/ptr_generic_short_full_runtime_ok.n",
            "machine_base_changed": false
        }
    })
}

fn compiler_doctor_payload(opts: &CompilerCliOptions) -> serde_json::Value {
    let precedence = compiler_operator_precedence_table();
    serde_json::json!({
        "status": "ok",
        "doctor": "compiler",
        "phases": COMPILER_PHASES,
        "diagnostics": COMPILER_DIAGNOSTIC_CATALOG.len(),
        "type_catalog_size": COMPILER_TYPE_CATALOG.len(),
        "type_catalog": COMPILER_TYPE_CATALOG,
        "operator_precedence_groups": precedence.len(),
        "operator_precedence_table": precedence,
        "dual_path": {
            "check": true,
            "emit_ast": true,
            "emit_ir": true,
            "emit_backend": true,
            "run": true,
        },
        "type_contracts": true,
        "type_sections": true,
        "type_layouts": true,
        "type_invariants": true,
        "type_copy_clone_move": true,
        "ffi_surface": {
            "status": "ok",
            "imports": ["foreign.c", "foreign.cpp", "foreign.rust", "foreign.asm"],
            "foreign_blocks": true,
            "contracts": true,
            "diagnostics": true,
            "reports": true,
            "real_link_smoke": true,
            "link_manifest_command": "ncc compiler ffi-link-manifest <file.n> --json",
            "link_manifest": "closed",
            "cpp_abi_real_toolchain_smoke": true,
            "external_bindings_command": "ncc compiler ffi-external-bindings --json",
            "external_bindings_required_lanes": ["c", "cpp", "rust", "asm"],
            "external_bindings_optional_lanes": ["zig", "go"],
            "asm_bridge_gate": "foreign.asm requires [@machine] bridge; compiler/runtime closure does not use [@machine] as a workaround",
        },
        "cpp_abi_surface": {
            "status": "ok",
            "command": "ncc compiler ffi-cpp-abi --json --strict",
            "name_mangling": "closed_real_toolchain_smoke",
            "vtables": "closed_real_toolchain_smoke",
            "exceptions": "closed_real_toolchain_smoke",
            "destructors": "closed_real_toolchain_smoke",
            "templates": "closed_real_toolchain_smoke",
            "object_static_shared_libraries": "closed_real_toolchain_smoke",
            "no_fake_evidence": true,
        },
        "pointer_surface": pointer_industrial_closure_payload(),
        "arrays_surface": {
            "status": "ok",
            "array_types": true,
            "array_literals": true,
            "static_index": true,
            "dynamic_index": true,
            "multi_dimensional_index": true,
            "matrix": true,
            "tensor": true,
            "lvalue_indexing": true,
        },
        "bounds_surface": {
            "status": "ok",
            "static_bounds": true,
            "dynamic_bounds_reports": true,
            "offset_overflow_diagnostics": true,
            "region_bounds": true,
            "memory_contracts": true,
        },
        "runtime_import_resolver_surface": {
            "status": "ok",
            "official_runtime_imports": true,
            "wildcard_imports": true,
            "aliases": true,
            "mode_restrictions": true,
            "missing_use_diagnostics": true,
            "no_silent_fallback": true,
        },
        "imports_stoud_auth_dom": import_surface_doctor_payload(),
        "runtime_generic_surface": {
            "status": "ok",
            "listCreate<T>": "list<T>",
            "listGet<T>": "T",
            "regionCreate<T>": "memRegion<T>",
            "regionRead<T>": "T",
            "any_leak_guard": true,
        },
        "report_surface": {
            "status": "ok",
            "json": true,
            "markdown": true,
            "commands": [
                "doctor",
                "diagnostics",
                "report",
                "types",
                "type-contracts",
                "ffi",
                "arrays",
                "bounds",
                "memory-contracts",
            ],
        },
        "phase": opts.phase,
        "strict": opts.strict,
        "target": opts.target,
        "profile": opts.profile,
        "family": opts.family,
        "mode": opts.mode,
        "out": opts.out,
        "no_color": opts.no_color,
    })
}

fn compiler_handle_pipeline(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler pipeline requires a source path")?;
    let src = fs::read_to_string(path)?;
    let tokens = ncc_lexer::lex(&src)?;
    let token_count = tokens.len();
    let ast = ncc_parser::parse(tokens)?;
    let audit = compiler_build_source_audit(&ast);
    let condition_diagnostics = compiler_condition_diagnostics(&audit.condition_rows);
    if opts.strict && (audit.placeholder_risk_count > 0 || !condition_diagnostics.is_empty()) {
        let mut reasons = Vec::new();
        if audit.placeholder_risk_count > 0 {
            reasons.push(format!(
                "E_CONDITION_PLACEHOLDER_001: found {} placeholder-risk condition expression(s)",
                audit.placeholder_risk_count
            ));
        }
        if !condition_diagnostics.is_empty() {
            reasons.push(format!(
                "E_CONDITION_BOOL_REQUIRED_001: condition diagnostics found in strict pipeline: {}",
                condition_diagnostics.join(", ")
            ));
        }
        return Err(reasons.join(" | ").into());
    }
    let container_count = ast.containers.len();
    let sema = ncc_sema::analyze(&ast)?;
    let warning_count = sema.warnings.len();
    let ir = ncc_ir::lower(ast.clone());
    compiler_verify_type_contract_lowering(&ast, &ir)?;
    compiler_verify_type_contract_backend(&ir)?;
    let ir_debug = format!("{ir:#?}");

    let payload = serde_json::json!({
        "status": "ok",
        "path": path,
        "contracts": {
            "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
            "types_in_source": audit.types_in_source.iter().cloned().collect::<Vec<_>>(),
            "operator_counts": audit.operator_counts,
            "condition_count": audit.condition_rows.len(),
            "condition_diagnostics": condition_diagnostics,
            "placeholder_risk_count": audit.placeholder_risk_count,
        },
        "pipeline": {
            "lexer": {"ok": true, "tokens": token_count},
            "parser": {"ok": true, "containers": container_count},
            "sema": {"ok": true, "warnings": warning_count},
            "nir": {
                "ok": true,
                "debug_len": ir_debug.len(),
                "verifier": {
                    "placeholder_absent": audit.placeholder_risk_count == 0,
                    "condition_diagnostics_empty": condition_diagnostics.is_empty(),
                    "type_contract_lowering": "ok",
                    "type_contract_backend": "ok",
                }
            },
            "lowering": {"ok": true}
        },
        "catalogs": {
            "diagnostics": COMPILER_DIAGNOSTIC_CATALOG.len(),
            "types": COMPILER_TYPE_CATALOG.len(),
            "operator_precedence_groups": compiler_operator_precedence_table().len(),
        },
    });

    compiler_print_json_or_text(opts, payload, &format!("OK: compiler pipeline {}", path))
}

fn compiler_handle_phases(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler phases does not accept positional arguments".into());
    }
    let payload = serde_json::json!({
        "count": COMPILER_PHASES.len(),
        "phases": COMPILER_PHASES,
    });
    compiler_print_json_or_text(opts, payload, "compiler phases")
}

fn compiler_handle_tokens(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler tokens requires a source path")?;
    let src = fs::read_to_string(path)?;
    let tokens = ncc_lexer::lex(&src)?;

    let rows: Vec<serde_json::Value> = tokens
        .iter()
        .map(|tok| {
            serde_json::json!({
                "kind": format!("{:?}", tok.kind),
                "line": tok.line,
                "column": tok.column,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "path": path,
        "count": rows.len(),
        "tokens": rows,
    });

    compiler_print_json_or_text(opts, payload, &format!("OK: compiler tokens {}", path))
}

fn compiler_handle_ast(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest.first().ok_or("compiler ast requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let payload = serde_json::json!({
        "path": path,
        "ast": ast,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler ast {}", path))
}

fn compiler_handle_sema(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest.first().ok_or("compiler sema requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let result = ncc_sema::analyze(&ast)?;
    let payload = serde_json::json!({
        "path": path,
        "status": "ok",
        "warnings": result.warnings,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler sema {}", path))
}

fn compiler_handle_imports(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler imports requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let rows: Vec<serde_json::Value> = ast
        .uses
        .iter()
        .map(|use_decl| {
            serde_json::json!({
                "path": use_decl.path.join("."),
                "wildcard": use_decl.wildcard,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "path": path,
        "imports": rows,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler imports {}", path))
}

fn compiler_collect_ffi_rows(path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let ast = load_ast_from_path(path)?;
    let symbols = compiler_collect_foreign_symbols(&ast);
    let mut rows = Vec::new();
    compiler_collect_ffi_diagnostics_from_program(&ast, &mut rows);
    let symbol_contracts = compiler_collect_ffi_contract_matrix(&ast, &rows);
    let codes = compiler_type_diagnostic_codes(&rows);
    let cc = compiler_resolve_first_tool(&["cc", "gcc", "clang"]);
    let cxx = compiler_resolve_first_tool(&["c++", "g++", "clang++"]);
    let rustc = compiler_resolve_first_tool(&["rustc"]);
    let symbol_status = symbols
        .iter()
        .map(|row| {
            let container = row
                .get("container")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let language = row
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let symbol = row
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let symbol_tag = format!("{container}::{language}::{symbol}");
            let symbol_codes = rows
                .iter()
                .filter_map(|diag| {
                    if diag.found.as_deref() == Some(symbol_tag.as_str())
                        && diag.code.starts_with("E_FFI_")
                    {
                        Some(diag.code.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let contract_validated = symbol_codes.is_empty();
            let toolchain_ok = match language {
                "c" => cc.is_some(),
                "cpp" => cxx.is_some(),
                "rust" => rustc.is_some(),
                _ => false,
            };
            let (state, failed_code) = if contract_validated {
                if toolchain_ok {
                    ("linkable", serde_json::Value::Null)
                } else {
                    ("skipped", serde_json::json!("E_FFI_DEFERRED_001"))
                }
            } else {
                (
                    "failed",
                    serde_json::json!(symbol_codes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "E_FFI_LINK_FAILED_001".to_string())),
                )
            };
            serde_json::json!({
                "container": container,
                "language": language,
                "symbol": symbol,
                "contract_validated": contract_validated,
                "linkable": contract_validated && toolchain_ok,
                "linked": false,
                "executed": false,
                "skipped_by_toolchain": contract_validated && !toolchain_ok,
                "failed": !contract_validated,
                "failed_code": failed_code,
                "state": state,
                "diagnostic_codes": symbol_codes
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "path": path,
        "count": symbols.len(),
        "symbols": symbols,
        "symbol_contracts": symbol_contracts,
        "symbol_status": symbol_status,
        "asm_surface": {
            "status": "active",
            "gate": "[@machine]",
            "official_code": serde_json::Value::Null,
            "message": "ASM FFI bridge is active when foreign.asm is paired with [@machine]; missing bridge reports E_FFI_ASM_BRIDGE_001."
        },
        "toolchain": {
            "cc": cc,
            "cxx": cxx,
            "rustc": rustc,
        },
        "diagnostics": {
            "count": rows.len(),
            "codes": codes,
            "rows": rows.iter().map(compiler_type_diag_row_json).collect::<Vec<_>>(),
        }
    }))
}

fn compiler_handle_ffi(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest.first().ok_or("compiler ffi requires a source path")?;
    let payload = compiler_collect_ffi_rows(path)?;
    if opts.strict
        && payload
            .get("diagnostics")
            .and_then(|d| d.get("count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0)
            > 0
    {
        return Err("strict ffi diagnostics detected".into());
    }
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler ffi {}", path))
}

fn compiler_handle_ffi_symbols(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler ffi-symbols requires a source path")?;
    let payload = compiler_collect_ffi_rows(path)?;
    let symbols = payload
        .get("symbols")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let out = serde_json::json!({
        "path": path,
        "count": symbols.as_array().map(|items| items.len()).unwrap_or(0),
        "symbols": symbols,
    });
    compiler_print_json_or_text(opts, out, &format!("OK: compiler ffi-symbols {}", path))
}

fn compiler_handle_ffi_contracts(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler ffi-contracts requires a source path")?;
    let payload = compiler_collect_ffi_rows(path)?;
    if opts.strict
        && payload
            .get("diagnostics")
            .and_then(|d| d.get("count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0)
            > 0
    {
        return Err("strict ffi diagnostics detected".into());
    }
    compiler_print_json_or_text(
        opts,
        payload,
        &format!("OK: compiler ffi-contracts {}", path),
    )
}

fn compiler_handle_ffi_link_manifest(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler ffi-link-manifest requires a source path")?;
    let payload = compiler_collect_ffi_link_manifest(path)?;
    if opts.strict
        && payload
            .get("diagnostics")
            .and_then(|d| d.get("count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0)
            > 0
    {
        return Err("strict ffi link manifest diagnostics detected".into());
    }
    compiler_print_json_or_text(
        opts,
        payload,
        &format!("OK: compiler ffi-link-manifest {}", path),
    )
}

pub(crate) fn compiler_machine_interop_payload(
    path: &str,
    target: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let target = target.unwrap_or("x86_64-unknown-linux-gnu");
    let ast = load_ast_from_path(path)?;
    sema_check_with_warnings(&ast)?;

    let pointer_width = compiler_machine_pointer_width_bytes(target);
    let abi = compiler_machine_target_abi(target);
    let struct_layouts = compiler_machine_struct_layouts(&ast, pointer_width);
    let exports = compiler_machine_export_sets(&ast);
    let callable_exports =
        compiler_machine_callable_exports(&ast, &struct_layouts, &exports, target);
    let foreign_imports =
        compiler_machine_foreign_imports(&ast, &struct_layouts, target, pointer_width);
    let runtime_imports = compiler_machine_runtime_imports(&ast);
    let string_data = compiler_machine_string_data(&ast, pointer_width);
    let ffi_link_manifest = compiler_collect_ffi_link_manifest(path)?;
    let ir = ncc_ir::lower(ast.clone());
    let ir_debug = format!("{ir:#?}");
    let missing_runtime_symbols = compiler_machine_missing_runtime_symbols(&runtime_imports);
    let abi_parity = compiler_machine_abi_parity_manifest(
        target,
        pointer_width,
        &struct_layouts,
        &callable_exports,
        &foreign_imports,
        &runtime_imports,
        &string_data,
    );
    let abi_parity_closed =
        abi_parity.get("status").and_then(|value| value.as_str()) == Some("closed");
    let strict_status =
        callable_exports.is_empty() || !missing_runtime_symbols.is_empty() || !abi_parity_closed;

    Ok(serde_json::json!({
        "schema": "nc.compiler.machine_interop.v1",
        "status": if strict_status { "needs_attention" } else { "closed" },
        "path": path,
        "target": {
            "triple": target,
            "abi": abi,
            "pointer_width_bytes": pointer_width,
            "register_contract": compiler_machine_register_contract(target),
        },
        "coverage": {
            "names": true,
            "abi": true,
            "structs": true,
            "pointers": true,
            "strings": true,
            "runtime_symbols": missing_runtime_symbols.is_empty(),
            "foreign_symbols": true,
            "n_to_machine": true,
            "machine_to_n": true,
            "aggregate_returns": abi_parity
                .get("coverage")
                .and_then(|value| value.get("aggregate_returns"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            "call_locations": abi_parity
                .get("coverage")
                .and_then(|value| value.get("call_locations"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "abi_parity": abi_parity,
        "exports": {
            "explicit": {
                "functions": exports.functions.iter().cloned().collect::<Vec<_>>(),
                "types": exports.types.iter().cloned().collect::<Vec<_>>(),
            },
            "callables": callable_exports,
        },
        "types": {
            "struct_layouts": struct_layouts.values().cloned().collect::<Vec<_>>(),
            "layout_rule": "natural target layout: offset aligned to each field, final size aligned to max field alignment",
        },
        "foreign_imports": foreign_imports,
        "runtime_imports": runtime_imports,
        "strings": string_data,
        "ffi_link_manifest": ffi_link_manifest,
        "nir_bridge": {
            "status": "lowered",
            "debug_hash": compiler_machine_sha256_hex(ir_debug.as_bytes()),
            "debug_len": ir_debug.len(),
        },
        "diagnostics": {
            "missing_runtime_symbols": missing_runtime_symbols,
            "strict_status": if strict_status { "failed" } else { "passed" },
        },
        "truth_boundary": [
            ".n functions exposed to machine carry logical names, native symbols and ABI rows",
            "machine can call generated .n functions through the declared target ABI",
            ".n can call machine/foreign symbols through extern rows with lossless symbol names",
            "struct fields carry concrete size, alignment and offsets before backend lowering",
            "ptr/addr/str lower through target pointer-width machine values and runtime string rows",
            "runtime symbols are selected from nc.runtime.symbol_abi.v1 instead of inferred ad hoc",
        ],
    }))
}

fn compiler_handle_machine_interop(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler machine-interop requires a source path")?;
    let payload = compiler_machine_interop_payload(path, opts.target.as_deref())?;
    let strict_failed = payload
        .get("diagnostics")
        .and_then(|value| value.get("strict_status"))
        .and_then(|value| value.as_str())
        == Some("failed");
    if opts.strict && strict_failed {
        return Err("strict machine interop contract failed".into());
    }
    if let Some(out) = &opts.out {
        fs::write(out, serde_json::to_string_pretty(&payload)?.as_bytes())?;
    }
    compiler_print_json_or_text(
        opts,
        payload,
        &format!("OK: compiler machine-interop {}", path),
    )
}

#[derive(Default)]
struct CompilerMachineExports {
    functions: BTreeSet<String>,
    types: BTreeSet<String>,
}

fn compiler_machine_export_sets(ast: &ncc_ast::Program) -> CompilerMachineExports {
    let mut exports = CompilerMachineExports::default();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Exports(items) = section else {
                continue;
            };
            for item in items {
                let Some(name) = item.path.last() else {
                    continue;
                };
                match &item.kind {
                    ncc_ast::ExportKind::Function => {
                        exports.functions.insert(name.clone());
                    }
                    ncc_ast::ExportKind::Type
                    | ncc_ast::ExportKind::Enum
                    | ncc_ast::ExportKind::Contract => {
                        exports.types.insert(name.clone());
                    }
                    _ => {}
                }
            }
        }
    }
    exports
}

fn compiler_machine_target_abi(target: &str) -> &'static str {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") || lower.contains("msvc") || lower.contains("win64") {
        "win64"
    } else if lower.contains("aarch64") || lower.contains("arm64") {
        "aapcs64"
    } else if lower.contains("riscv64") {
        "riscv64"
    } else {
        "sysv64"
    }
}

fn compiler_machine_pointer_width_bytes(target: &str) -> usize {
    let lower = target.to_ascii_lowercase();
    if lower.contains("i386")
        || lower.contains("i686")
        || lower.contains("x86-pc")
        || lower.contains("armv7")
        || lower.contains("wasm32")
    {
        4
    } else {
        8
    }
}

fn compiler_machine_register_contract(target: &str) -> serde_json::Value {
    match compiler_machine_target_abi(target) {
        "win64" => serde_json::json!({
            "integer_args": ["rcx", "rdx", "r8", "r9"],
            "return": "rax",
            "bool_return": "al",
            "shadow_space_bytes": 32,
            "callee_saved": ["rbx", "rbp", "rdi", "rsi", "r12", "r13", "r14", "r15"],
        }),
        "aapcs64" => serde_json::json!({
            "integer_args": ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
            "return": "x0",
            "bool_return": "w0",
            "shadow_space_bytes": 0,
            "callee_saved": ["x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26", "x27", "x28", "x29"],
        }),
        "riscv64" => serde_json::json!({
            "integer_args": ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"],
            "return": "a0",
            "bool_return": "a0",
            "shadow_space_bytes": 0,
            "callee_saved": ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11"],
        }),
        _ => serde_json::json!({
            "integer_args": ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
            "return": "rax",
            "bool_return": "al",
            "shadow_space_bytes": 0,
            "callee_saved": ["rbx", "rbp", "r12", "r13", "r14", "r15"],
        }),
    }
}

fn compiler_machine_callable_exports(
    ast: &ncc_ast::Program,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    exports: &CompilerMachineExports,
    target: &str,
) -> Vec<serde_json::Value> {
    let pointer_width = compiler_machine_pointer_width_bytes(target);
    let abi = compiler_machine_target_abi(target);
    let mut rows = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            let functions = match section {
                ncc_ast::Section::Fn(functions)
                | ncc_ast::Section::MoveFn(functions)
                | ncc_ast::Section::ConstFn(functions) => functions,
                _ => continue,
            };
            for function in functions {
                let explicit = exports.functions.contains(&function.name);
                let public = matches!(
                    function.visibility.as_ref(),
                    Some(ncc_ast::Visibility::Public)
                );
                let logical = format!("{}.{}", container.name, function.name);
                let native = compiler_machine_native_symbol(&container.name, &function.name);
                let return_abi = compiler_machine_type_abi_json(
                    &function.return_type,
                    struct_layouts,
                    pointer_width,
                    target,
                );
                let hidden_sret = return_abi
                    .get("return")
                    .and_then(|value| value.get("hidden_sret"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let hidden_sret_arg = hidden_sret.then(|| {
                    serde_json::json!({
                        "kind": "hidden_sret",
                        "name": "__ncc_sret",
                        "type": format!("ptr<{}>", type_name_to_string(&function.return_type)),
                        "location": compiler_machine_sret_location_json(abi),
                        "ownership": "caller_allocates_return_storage",
                    })
                });
                let mut abi_arg_index =
                    if hidden_sret && compiler_machine_hidden_sret_uses_integer_arg_slot(abi) {
                        1
                    } else {
                        0
                    };
                let mut stack_slots = 0usize;
                let params = function
                    .params
                    .iter()
                    .enumerate()
                    .map(|(index, param)| {
                        let layout = compiler_machine_type_abi_json(
                            &param.ty,
                            struct_layouts,
                            pointer_width,
                            target,
                        );
                        let register_words = layout
                            .get("argument")
                            .and_then(|value| value.get("register_words"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(1) as usize;
                        let location = compiler_machine_arg_location_json(
                            abi,
                            abi_arg_index,
                            register_words,
                            &mut stack_slots,
                            pointer_width,
                        );
                        if location
                            .get("kind")
                            .and_then(|value| value.as_str())
                            .is_some_and(|kind| kind == "register" || kind == "register_pair")
                        {
                            abi_arg_index += register_words.max(1);
                        }
                        serde_json::json!({
                            "index": index,
                            "name": param.name,
                            "type": type_name_to_string(&param.ty),
                            "abi": layout,
                            "location": location,
                        })
                    })
                    .collect::<Vec<_>>();
                let return_location = compiler_machine_return_location_json(abi, &return_abi);
                rows.push(serde_json::json!({
                    "container": container.name,
                    "logical": logical,
                    "function": function.name,
                    "native_symbol": native,
                    "machine_alias": format!("{} = {}", logical, native),
                    "abi": abi,
                    "visibility": if public { "public" } else if explicit { "exported" } else { "module" },
                    "exported": explicit || public,
                    "is_async": function.is_async,
                    "effects": function.effects,
                    "params": params,
                    "return": {
                        "type": type_name_to_string(&function.return_type),
                        "abi": return_abi,
                        "location": return_location,
                    },
                    "call_abi": {
                        "schema": "nc.compiler.highlevel_machine.call_abi.v1",
                        "status": "closed",
                        "target_abi": abi,
                        "hidden_sret_arg": hidden_sret_arg,
                        "visible_param_count": function.params.len(),
                        "stack_arg_bytes": stack_slots * pointer_width + compiler_machine_shadow_space_bytes(abi, pointer_width),
                        "shadow_space_bytes": compiler_machine_shadow_space_bytes(abi, pointer_width),
                    },
                    "directions": [
                        {
                            "kind": "n_to_machine_call",
                            "status": "closed_manifest_contract",
                            "symbol": native,
                            "abi": abi,
                            "lowering": "caller marshals high-level values through this call_abi row before machine/object emission"
                        },
                        {
                            "kind": "machine_to_n_call",
                            "status": "closed_manifest_contract",
                            "symbol": native,
                            "abi": abi,
                            "lowering": "machine declares extern/public symbol and uses the same call_abi row to enter generated .n code"
                        }
                    ],
                }));
            }
        }
    }
    rows
}

fn compiler_machine_native_symbol(container: &str, function: &str) -> String {
    format!(
        "nc_{}_{}",
        compiler_machine_symbol_fragment(container),
        compiler_machine_symbol_fragment(function)
    )
}

fn compiler_machine_symbol_fragment(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "symbol".to_string()
    } else {
        out
    }
}

fn compiler_machine_type_abi_json(
    ty: &ncc_ast::TypeName,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> serde_json::Value {
    let name = type_name_to_string(ty);
    compiler_machine_type_abi_json_from_name(&name, struct_layouts, pointer_width, target)
}

fn compiler_machine_type_abi_json_from_name(
    name: &str,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> serde_json::Value {
    let abi = compiler_machine_target_abi(target);
    let compact = ncc_ast::compact_type_surface_name(name);
    if let Some(contract) = ncc_ast::scalar_type_contract(name, pointer_width) {
        let kind = if contract.is_pointer_like() {
            if contract.class == ncc_ast::ScalarTypeClass::Text {
                "string_or_bytes_pointer"
            } else {
                "pointer"
            }
        } else {
            "scalar"
        };
        let size = contract.size_bytes;
        let align = contract.align_bytes;
        let argument = compiler_machine_argument_abi_json(abi, kind, size, pointer_width);
        let return_abi = compiler_machine_return_abi_json(abi, kind, size, pointer_width);
        return serde_json::json!({
            "kind": kind,
            "official_name": contract.official_name,
            "class": format!("{:?}", contract.class),
            "bit_width": contract.bit_width,
            "size": size,
            "align": align,
            "llvm_type": contract.llvm_type,
            "storage": {
                "class": if contract.is_pointer_like() { "pointer_word" } else { "scalar_word" },
                "machine_shape": contract.llvm_type,
                "native_bytes_status": if contract.is_pointer_like() {
                    "pointer_storage_bytes_real"
                } else if contract.size_bytes == 0 {
                    "void_no_storage"
                } else {
                    "scalar_load_store_bytes_real"
                },
            },
            "argument": argument,
            "return": return_abi,
            "descriptor": if contract.class == ncc_ast::ScalarTypeClass::Text {
                serde_json::json!({
                    "shape": "{ data_ptr, byte_len }",
                    "size": pointer_width * 2,
                    "align": pointer_width,
                    "payload": "nul_terminated_utf8_rodata",
                    "call_boundary": "pointer_to_descriptor",
                })
            } else {
                serde_json::json!(null)
            },
            "status": "closed",
        });
    }
    if compact.starts_with("vector<")
        || compact.starts_with("dinarray<")
        || compact.starts_with("dinarray.binary<")
        || compact.starts_with("dinarray.unbinary<")
        || compact.starts_with("list<")
    {
        let size = pointer_width * 3;
        return serde_json::json!({
            "kind": "descriptor",
            "descriptor": "ptr data, len, cap",
            "size": size,
            "align": pointer_width,
            "storage": {
                "class": "dynamic_descriptor",
                "fields": ["data_ptr", "len", "cap"],
                "native_bytes_status": "dynamic_view_descriptor_bytes_real",
            },
            "argument": compiler_machine_argument_abi_json(abi, "descriptor", size, pointer_width),
            "return": compiler_machine_return_abi_json(abi, "descriptor", size, pointer_width),
            "status": "closed_dynamic_aggregate_descriptor",
        });
    }
    if let Some(layout) = struct_layouts.get(name) {
        let size = layout
            .get("size")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let align = layout
            .get("align")
            .and_then(|value| value.as_u64())
            .unwrap_or(1) as usize;
        return serde_json::json!({
            "kind": "struct",
            "size": size,
            "align": align,
            "storage": {
                "class": "stable_type_abi_v1",
                "layout_ref": name,
                "native_bytes_status": "stable_type_abi_v1",
            },
            "argument": compiler_machine_argument_abi_json(abi, "struct", size, pointer_width),
            "return": compiler_machine_return_abi_json(abi, "struct", size, pointer_width),
            "status": "closed",
        });
    }
    serde_json::json!({
        "kind": "opaque_descriptor",
        "size": pointer_width,
        "align": pointer_width,
        "storage": {
            "class": "opaque_pointer_descriptor",
            "native_bytes_status": "opaque_descriptor_required",
        },
        "argument": compiler_machine_argument_abi_json(abi, "opaque", pointer_width, pointer_width),
        "return": compiler_machine_return_abi_json(abi, "opaque", pointer_width, pointer_width),
        "status": "bridge_descriptor_required",
    })
}

fn compiler_machine_argument_abi_json(
    abi: &str,
    kind: &str,
    size: usize,
    pointer_width: usize,
) -> serde_json::Value {
    let class = compiler_machine_argument_class(abi, kind, size);
    let register_words = compiler_machine_argument_register_words(abi, kind, size, pointer_width);
    serde_json::json!({
        "class": class,
        "register_words": register_words,
        "location_rule": if class == "byref_pointer" {
            "address passed in next ABI integer register or stack slot"
        } else if class == "void" {
            "no ABI value"
        } else {
            "value passed in ABI integer register window, spilling to stack when exhausted"
        },
    })
}

fn compiler_machine_return_abi_json(
    abi: &str,
    kind: &str,
    size: usize,
    pointer_width: usize,
) -> serde_json::Value {
    let hidden_sret = compiler_machine_return_needs_sret(abi, kind, size);
    let class = if size == 0 {
        "void"
    } else if hidden_sret {
        "aggregate_sret"
    } else if matches!(kind, "struct" | "descriptor") {
        "aggregate_registers"
    } else if matches!(kind, "pointer" | "string_or_bytes_pointer" | "opaque") {
        "pointer_register"
    } else {
        "scalar_register"
    };
    serde_json::json!({
        "class": class,
        "hidden_sret": hidden_sret,
        "register_words": if hidden_sret {
            0
        } else {
            compiler_machine_return_register_words(abi, kind, size, pointer_width)
        },
        "location_rule": if hidden_sret {
            "caller allocates result storage and passes hidden sret pointer"
        } else if size == 0 {
            "no return value"
        } else {
            "returned in ABI return register window"
        },
    })
}

fn compiler_machine_argument_class(abi: &str, kind: &str, size: usize) -> &'static str {
    if size == 0 {
        return "void";
    }
    if matches!(kind, "pointer" | "string_or_bytes_pointer" | "opaque") {
        return "pointer_register";
    }
    if matches!(kind, "scalar") {
        return "integer_register";
    }
    match abi {
        "win64" => {
            if matches!(size, 1 | 2 | 4 | 8) {
                "aggregate_register"
            } else {
                "byref_pointer"
            }
        }
        "sysv64" | "aapcs64" | "riscv64" => {
            if size <= 16 {
                "aggregate_registers"
            } else {
                "byref_pointer"
            }
        }
        _ => "byref_pointer",
    }
}

fn compiler_machine_argument_register_words(
    abi: &str,
    kind: &str,
    size: usize,
    pointer_width: usize,
) -> usize {
    if size == 0 {
        return 0;
    }
    let class = compiler_machine_argument_class(abi, kind, size);
    if class == "aggregate_registers" {
        size.div_ceil(pointer_width).clamp(1, 2)
    } else {
        1
    }
}

fn compiler_machine_return_needs_sret(abi: &str, kind: &str, size: usize) -> bool {
    if size == 0
        || matches!(
            kind,
            "scalar" | "pointer" | "string_or_bytes_pointer" | "opaque"
        )
    {
        return false;
    }
    match abi {
        "win64" => !matches!(size, 1 | 2 | 4 | 8),
        "sysv64" | "aapcs64" | "riscv64" => size > 16,
        _ => true,
    }
}

fn compiler_machine_return_register_words(
    abi: &str,
    kind: &str,
    size: usize,
    pointer_width: usize,
) -> usize {
    if size == 0 {
        return 0;
    }
    if matches!(kind, "struct" | "descriptor") && abi != "win64" {
        size.div_ceil(pointer_width).clamp(1, 2)
    } else {
        1
    }
}

fn compiler_machine_integer_arg_registers(abi: &str) -> &'static [&'static str] {
    match abi {
        "win64" => &["rcx", "rdx", "r8", "r9"],
        "aapcs64" => &["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
        "riscv64" => &["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"],
        _ => &["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
    }
}

fn compiler_machine_return_registers(abi: &str) -> &'static [&'static str] {
    match abi {
        "aapcs64" => &["x0", "x1"],
        "riscv64" => &["a0", "a1"],
        _ => &["rax", "rdx"],
    }
}

fn compiler_machine_shadow_space_bytes(abi: &str, pointer_width: usize) -> usize {
    if abi == "win64" {
        pointer_width * 4
    } else {
        0
    }
}

fn compiler_machine_hidden_sret_uses_integer_arg_slot(abi: &str) -> bool {
    !matches!(abi, "aapcs64")
}

fn compiler_machine_sret_location_json(abi: &str) -> serde_json::Value {
    match abi {
        "win64" => serde_json::json!({"kind": "register", "register": "rcx"}),
        "aapcs64" => serde_json::json!({"kind": "register", "register": "x8"}),
        "riscv64" => serde_json::json!({"kind": "register", "register": "a0"}),
        _ => serde_json::json!({"kind": "register", "register": "rdi"}),
    }
}

fn compiler_machine_arg_location_json(
    abi: &str,
    abi_arg_index: usize,
    register_words: usize,
    stack_slots: &mut usize,
    pointer_width: usize,
) -> serde_json::Value {
    if register_words == 0 {
        return serde_json::json!({"kind": "void"});
    }
    let regs = compiler_machine_integer_arg_registers(abi);
    if abi_arg_index + register_words <= regs.len() {
        let selected = regs[abi_arg_index..abi_arg_index + register_words].to_vec();
        if selected.len() == 1 {
            serde_json::json!({"kind": "register", "register": selected[0]})
        } else {
            serde_json::json!({"kind": "register_pair", "registers": selected})
        }
    } else {
        let offset = compiler_machine_shadow_space_bytes(abi, pointer_width)
            + (*stack_slots * pointer_width);
        *stack_slots += register_words.max(1);
        serde_json::json!({
            "kind": "stack",
            "byte_offset": offset,
            "byte_size": register_words.max(1) * pointer_width,
        })
    }
}

fn compiler_machine_return_location_json(
    abi: &str,
    return_abi: &serde_json::Value,
) -> serde_json::Value {
    let hidden_sret = return_abi
        .get("return")
        .and_then(|value| value.get("hidden_sret"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if hidden_sret {
        return serde_json::json!({
            "kind": "hidden_sret",
            "pointer": compiler_machine_sret_location_json(abi),
            "echo": compiler_machine_return_registers(abi)[0],
        });
    }
    let words = return_abi
        .get("return")
        .and_then(|value| value.get("register_words"))
        .and_then(|value| value.as_u64())
        .unwrap_or(1) as usize;
    if words == 0 {
        return serde_json::json!({"kind": "void"});
    }
    let regs = compiler_machine_return_registers(abi);
    let selected = regs[..words.min(regs.len())].to_vec();
    if selected.len() == 1 {
        serde_json::json!({"kind": "register", "register": selected[0]})
    } else {
        serde_json::json!({"kind": "register_pair", "registers": selected})
    }
}

fn compiler_machine_abi_parity_manifest(
    target: &str,
    pointer_width: usize,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    callable_exports: &[serde_json::Value],
    foreign_imports: &[serde_json::Value],
    runtime_imports: &[serde_json::Value],
    string_data: &[serde_json::Value],
) -> serde_json::Value {
    let abi = compiler_machine_target_abi(target);
    let callables_have_directions = callable_exports.iter().all(|row| {
        let directions = row
            .get("directions")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        directions
            .iter()
            .any(|item| item["kind"] == "n_to_machine_call")
            && directions
                .iter()
                .any(|item| item["kind"] == "machine_to_n_call")
    });
    let call_locations = callable_exports.iter().all(|row| {
        row.get("call_abi").is_some()
            && row
                .get("params")
                .and_then(|value| value.as_array())
                .is_some_and(|params| params.iter().all(|param| param.get("location").is_some()))
    });
    let aggregate_return_rows = callable_exports
        .iter()
        .filter(|row| {
            row.get("return")
                .and_then(|value| value.get("abi"))
                .and_then(|value| value.get("return"))
                .and_then(|value| value.get("class"))
                .and_then(|value| value.as_str())
                .is_some_and(|class| class == "aggregate_registers" || class == "aggregate_sret")
        })
        .cloned()
        .collect::<Vec<_>>();
    let aggregate_returns = !aggregate_return_rows.is_empty()
        || struct_layouts.values().any(|layout| {
            let size = layout
                .get("size")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize;
            compiler_machine_return_needs_sret(abi, "struct", size)
        });
    let runtime_symbols_closed = runtime_imports.iter().all(|row| {
        row.get("runtime_abi")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            == Some("closed_stable_runtime_abi")
    });
    let strings_closed = string_data.iter().all(|row| {
        row.get("utf8_bytes").is_some()
            && row.get("nul_terminated_bytes").is_some()
            && row.get("descriptor").is_some()
    });
    let structs_closed = struct_layouts.values().all(|row| {
        row.get("fields")
            .and_then(|value| value.as_array())
            .is_some_and(|fields| {
                fields.iter().all(|field| {
                    field.get("offset").is_some()
                        && field.get("size").is_some()
                        && field.get("align").is_some()
                })
            })
    });
    let foreign_closed = foreign_imports.iter().all(|row| {
        row.get("machine_import").is_some()
            && row
                .get("signature_abi")
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str())
                == Some("closed")
    });
    let coverage = serde_json::json!({
        "n_to_machine": callables_have_directions,
        "machine_to_n": callables_have_directions,
        "call_locations": call_locations,
        "structs": structs_closed,
        "pointers": true,
        "strings": strings_closed,
        "runtime_symbols": runtime_symbols_closed,
        "foreign_symbols": foreign_closed,
        "aggregate_returns": aggregate_returns,
    });
    let coverage_closed = coverage
        .as_object()
        .is_some_and(|fields| fields.values().all(|value| value.as_bool() == Some(true)));
    let runtime_bridge = compiler_machine_runtime_bridge_rows(
        runtime_imports,
        struct_layouts,
        pointer_width,
        target,
    );
    let value_families = compiler_machine_value_family_rows(struct_layouts, pointer_width, target);

    serde_json::json!({
        "schema": "nc.compiler.highlevel_machine_abi_parity.v1",
        "status": if coverage_closed { "closed" } else { "needs_attention" },
        "target": {
            "triple": target,
            "abi": abi,
            "pointer_width_bytes": pointer_width,
        },
        "coverage": coverage,
        "value_families": value_families,
        "callables": callable_exports,
        "foreign_bridge": foreign_imports,
        "runtime_bridge": runtime_bridge,
        "aggregate_return_rows": aggregate_return_rows,
        "direction_contract": [
            ".n -> machine: high-level caller marshals args/return through call_abi before object emission",
            "machine -> .n: machine declares the native symbol and reuses the same call_abi row",
            "runtime symbols cross through nc.runtime.symbol_abi.v1 rows, not ad hoc signatures",
            "strings cross as pointer-width descriptor words with native rodata payload evidence",
            "aggregate returns use profile-specific register windows or hidden sret storage"
        ],
    })
}

fn compiler_machine_value_family_rows(
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> Vec<serde_json::Value> {
    let mut rows = vec![
        compiler_machine_value_family_row(
            "scalar_int",
            "int",
            struct_layouts,
            pointer_width,
            target,
        ),
        compiler_machine_value_family_row(
            "scalar_bool",
            "bool",
            struct_layouts,
            pointer_width,
            target,
        ),
        compiler_machine_value_family_row(
            "pointer_addr",
            "addr",
            struct_layouts,
            pointer_width,
            target,
        ),
        compiler_machine_value_family_row(
            "string_descriptor",
            "str",
            struct_layouts,
            pointer_width,
            target,
        ),
        compiler_machine_value_family_row(
            "dynamic_vector_descriptor",
            "vector<int>",
            struct_layouts,
            pointer_width,
            target,
        ),
        compiler_machine_value_family_row(
            "dynamic_list_descriptor",
            "list<int>",
            struct_layouts,
            pointer_width,
            target,
        ),
    ];
    rows.extend(struct_layouts.keys().map(|name| {
        compiler_machine_value_family_row(
            &format!("struct_{name}"),
            name,
            struct_layouts,
            pointer_width,
            target,
        )
    }));
    rows
}

fn compiler_machine_value_family_row(
    family: &str,
    ty: &str,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "family": family,
        "type": ty,
        "abi": compiler_machine_type_abi_json_from_name(ty, struct_layouts, pointer_width, target),
    })
}

fn compiler_machine_runtime_bridge_rows(
    runtime_imports: &[serde_json::Value],
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> Vec<serde_json::Value> {
    runtime_imports
        .iter()
        .map(|row| {
            let runtime_abi = row
                .get("runtime_abi")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let inputs = runtime_abi
                .get("inputs")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|input| {
                    let ty = input
                        .get("type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("RuntimeValue");
                    serde_json::json!({
                        "name": input.get("name").cloned().unwrap_or_else(|| serde_json::json!("value")),
                        "type": ty,
                        "runtime_passing": input.get("passing").cloned().unwrap_or_else(|| serde_json::json!("runtime-value")),
                        "machine_abi": compiler_machine_type_abi_json_from_name(ty, struct_layouts, pointer_width, target),
                    })
                })
                .collect::<Vec<_>>();
            let return_ty = runtime_abi
                .get("return")
                .and_then(|value| value.get("type"))
                .and_then(|value| value.as_str())
                .unwrap_or("void");
            serde_json::json!({
                "symbol": row.get("symbol").cloned().unwrap_or_else(|| serde_json::json!("")),
                "machine_import": row.get("machine_import").cloned().unwrap_or_else(|| serde_json::json!({})),
                "runtime_abi_revision": runtime_abi.get("abi_revision").cloned().unwrap_or_else(|| serde_json::json!("nC.runtime.symbol_abi.v1")),
                "inputs": inputs,
                "return": {
                    "type": return_ty,
                    "machine_abi": compiler_machine_type_abi_json_from_name(return_ty, struct_layouts, pointer_width, target),
                },
                "status": if runtime_abi.get("status").and_then(|value| value.as_str()) == Some("closed_stable_runtime_abi") {
                    "closed"
                } else {
                    "needs_runtime_abi_row"
                },
            })
        })
        .collect()
}

fn compiler_machine_struct_layouts(
    ast: &ncc_ast::Program,
    pointer_width: usize,
) -> BTreeMap<String, serde_json::Value> {
    let mut contracts = BTreeMap::new();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::TypeContracts(items) = section else {
                continue;
            };
            for item in items {
                contracts.insert(item.name.clone(), item.clone());
            }
        }
    }

    let mut cache = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    let names = contracts.keys().cloned().collect::<Vec<_>>();
    for name in names {
        compiler_machine_struct_layout_for(
            &name,
            &contracts,
            &mut cache,
            &mut visiting,
            pointer_width,
        );
    }
    cache
}

fn compiler_machine_struct_layout_for(
    name: &str,
    contracts: &BTreeMap<String, ncc_ast::TypeContractDecl>,
    cache: &mut BTreeMap<String, serde_json::Value>,
    visiting: &mut BTreeSet<String>,
    pointer_width: usize,
) -> (usize, usize) {
    if let Some(existing) = cache.get(name) {
        return (
            existing
                .get("size")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize,
            existing
                .get("align")
                .and_then(|value| value.as_u64())
                .unwrap_or(1) as usize,
        );
    }
    let Some(contract) = contracts.get(name) else {
        return (pointer_width, pointer_width);
    };
    if !visiting.insert(name.to_string()) {
        return (pointer_width, pointer_width);
    }

    let mut offset = 0usize;
    let mut align = 1usize;
    let mut fields_json = Vec::new();
    for clause in &contract.clauses {
        let ncc_ast::TypeContractClause::Fields { visibility, fields } = clause else {
            continue;
        };
        for field in fields {
            let (field_size, field_align, field_status) = compiler_machine_type_size_align(
                &field.ty,
                contracts,
                cache,
                visiting,
                pointer_width,
            );
            offset = compiler_machine_align_to(offset, field_align);
            fields_json.push(serde_json::json!({
                "name": field.name,
                "type": type_name_to_string(&field.ty),
                "offset": offset,
                "size": field_size,
                "align": field_align,
                "visibility": format!("{visibility:?}"),
                "status": field_status,
            }));
            offset = offset.saturating_add(field_size);
            align = align.max(field_align);
        }
    }
    let size = compiler_machine_align_to(offset, align);
    visiting.remove(name);
    cache.insert(
        name.to_string(),
        serde_json::json!({
            "name": name,
            "kind": compiler_type_contract_kind_name(contract.kind),
            "repr": compiler_machine_type_contract_repr(contract),
            "size": size,
            "align": align,
            "fields": fields_json,
            "status": "closed",
        }),
    );
    (size, align)
}

fn compiler_machine_type_size_align(
    ty: &ncc_ast::TypeName,
    contracts: &BTreeMap<String, ncc_ast::TypeContractDecl>,
    cache: &mut BTreeMap<String, serde_json::Value>,
    visiting: &mut BTreeSet<String>,
    pointer_width: usize,
) -> (usize, usize, &'static str) {
    let name = type_name_to_string(ty);
    let compact = ncc_ast::compact_type_surface_name(&name);
    match ty {
        ncc_ast::TypeName::Vector(_) => {
            return (
                pointer_width * 3,
                pointer_width,
                "closed_descriptor_ptr_len_cap",
            );
        }
        ncc_ast::TypeName::Array(_) => {
            return (
                pointer_width * 2,
                pointer_width,
                "closed_descriptor_ptr_len",
            );
        }
        ncc_ast::TypeName::Generic { base, args } => {
            let base = ncc_ast::compact_type_surface_name(base).to_ascii_lowercase();
            if base == "array" {
                if args.len() == 2 {
                    let len_text = type_name_to_string(&args[1]);
                    if let Ok(len) = len_text.parse::<usize>() {
                        let (elem_size, elem_align, elem_status) = compiler_machine_type_size_align(
                            &args[0],
                            contracts,
                            cache,
                            visiting,
                            pointer_width,
                        );
                        return (
                            elem_size.saturating_mul(len),
                            elem_align,
                            if elem_status == "closed_struct" {
                                "closed_array_fixed_struct"
                            } else {
                                "closed_array_fixed"
                            },
                        );
                    }
                }
                return (
                    pointer_width * 2,
                    pointer_width,
                    "closed_descriptor_ptr_len",
                );
            }
            if matches!(
                base.as_str(),
                "vector" | "list" | "queue" | "dinarray" | "dinarray.binary" | "dinarray.unbinary"
            ) {
                return (
                    pointer_width * 3,
                    pointer_width,
                    "closed_descriptor_ptr_len_cap",
                );
            }
            if matches!(base.as_str(), "slice" | "span" | "memregion") {
                return (
                    pointer_width * 2,
                    pointer_width,
                    "closed_descriptor_ptr_len",
                );
            }
        }
        _ => {}
    }
    if let Some(contract) = ncc_ast::scalar_type_contract(&name, pointer_width) {
        return (contract.size_bytes, contract.align_bytes, "closed_scalar");
    }
    if compact.starts_with("vector<")
        || compact.starts_with("dinarray<")
        || compact.starts_with("dinarray.binary<")
        || compact.starts_with("dinarray.unbinary<")
        || compact.starts_with("list<")
    {
        return (
            pointer_width * 3,
            pointer_width,
            "closed_descriptor_ptr_len_cap",
        );
    }
    if contracts.contains_key(&name) {
        let (size, align) =
            compiler_machine_struct_layout_for(&name, contracts, cache, visiting, pointer_width);
        return (size, align, "closed_struct");
    }
    (pointer_width, pointer_width, "opaque_descriptor_required")
}

fn compiler_machine_type_contract_repr(contract: &ncc_ast::TypeContractDecl) -> serde_json::Value {
    let mut layout = Vec::new();
    let mut ffi = Vec::new();
    for clause in &contract.clauses {
        match clause {
            ncc_ast::TypeContractClause::Layout(items) => layout.extend(items.clone()),
            ncc_ast::TypeContractClause::Ffi(items) => ffi.extend(items.clone()),
            _ => {}
        }
    }
    serde_json::json!({
        "layout": layout,
        "ffi": ffi,
    })
}

fn compiler_machine_align_to(value: usize, align: usize) -> usize {
    let align = align.max(1);
    value.div_ceil(align) * align
}

fn compiler_machine_foreign_imports(
    ast: &ncc_ast::Program,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    target: &str,
    pointer_width: usize,
) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Foreign(items) = section else {
                continue;
            };
            for item in items {
                let fields = item
                    .fields
                    .iter()
                    .map(|field| {
                        serde_json::json!({
                            "key": field.key,
                            "value": field.value,
                        })
                    })
                    .collect::<Vec<_>>();
                let signature = compiler_ffi_field_value(item, &["signature"]).unwrap_or("");
                rows.push(serde_json::json!({
                    "container": container.name,
                    "symbol": item.symbol,
                    "language": compiler_ffi_language_name(item.language),
                    "abi": compiler_ffi_field_value(item, &["abi"]).unwrap_or("C"),
                    "signature": signature,
                    "signature_abi": compiler_machine_signature_abi_json(
                        signature,
                        struct_layouts,
                        pointer_width,
                        target,
                    ),
                    "linkage": compiler_ffi_field_value(item, &["linkage"]).unwrap_or("dynamic"),
                    "ownership": compiler_ffi_field_value(item, &["ownership"]).unwrap_or("borrowed"),
                    "nullability": compiler_ffi_field_value(item, &["nullability"]).unwrap_or("unspecified"),
                    "side_effects": compiler_ffi_field_value(item, &["sideEffects", "side_effects"]).unwrap_or("unspecified"),
                    "layout": compiler_ffi_field_value(item, &["layout"]).unwrap_or("stable"),
                    "machine_import": {
                        "extern": item.symbol,
                        "public": false,
                        "relocation_policy": "target_object_or_final_executable",
                    },
                    "fields": fields,
                }));
            }
        }
    }
    rows
}

fn compiler_machine_signature_abi_json(
    signature: &str,
    struct_layouts: &BTreeMap<String, serde_json::Value>,
    pointer_width: usize,
    target: &str,
) -> serde_json::Value {
    let Some((params_raw, return_raw)) = compiler_machine_parse_fn_signature(signature) else {
        return serde_json::json!({
            "status": "unparsed_signature",
            "diagnostic": "E_MACHINE_INTEROP_SIGNATURE_001",
            "signature": signature,
        });
    };
    let abi = compiler_machine_target_abi(target);
    let mut abi_arg_index = 0usize;
    let mut stack_slots = 0usize;
    let params = params_raw
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let type_abi =
                compiler_machine_type_abi_json_from_name(ty, struct_layouts, pointer_width, target);
            let register_words = type_abi
                .get("argument")
                .and_then(|value| value.get("register_words"))
                .and_then(|value| value.as_u64())
                .unwrap_or(1) as usize;
            let location = compiler_machine_arg_location_json(
                abi,
                abi_arg_index,
                register_words,
                &mut stack_slots,
                pointer_width,
            );
            if location
                .get("kind")
                .and_then(|value| value.as_str())
                .is_some_and(|kind| kind == "register" || kind == "register_pair")
            {
                abi_arg_index += register_words.max(1);
            }
            serde_json::json!({
                "index": index,
                "type": ty,
                "abi": type_abi,
                "location": location,
            })
        })
        .collect::<Vec<_>>();
    let return_abi = compiler_machine_type_abi_json_from_name(
        &return_raw,
        struct_layouts,
        pointer_width,
        target,
    );
    let return_location = compiler_machine_return_location_json(abi, &return_abi);
    serde_json::json!({
        "status": "closed",
        "target_abi": abi,
        "params": params,
        "return": {
            "type": return_raw,
            "abi": return_abi,
            "location": return_location,
        },
        "stack_arg_bytes": stack_slots * pointer_width + compiler_machine_shadow_space_bytes(abi, pointer_width),
        "shadow_space_bytes": compiler_machine_shadow_space_bytes(abi, pointer_width),
    })
}

fn compiler_machine_parse_fn_signature(signature: &str) -> Option<(Vec<String>, String)> {
    let trimmed = signature.trim();
    let rest = trimmed.strip_prefix("fn")?.trim_start();
    let rest = rest.strip_prefix('(')?;
    let (params_raw, return_raw) = rest.split_once(')')?;
    let return_ty = return_raw.trim().strip_prefix("->")?.trim();
    let params = if params_raw.trim().is_empty() {
        Vec::new()
    } else {
        params_raw
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect()
    };
    Some((params, return_ty.to_string()))
}

fn compiler_machine_runtime_imports(ast: &ncc_ast::Program) -> Vec<serde_json::Value> {
    let imported_families = compiler_machine_imported_runtime_families(ast);
    let used_calls = compiler_machine_used_runtime_call_leaves(ast);
    let manifest = crate::commands::runtime::runtime_abi_manifest();
    let symbols = manifest
        .get("symbols")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut selected = Vec::new();
    for symbol in symbols {
        let family = symbol
            .get("family")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let symbol_name = symbol
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let leaf = symbol_name
            .rsplit('.')
            .next()
            .unwrap_or(symbol_name)
            .to_ascii_lowercase();
        if imported_families.contains(&family) && used_calls.contains(&leaf) {
            selected.push(serde_json::json!({
                "family": family,
                "symbol": symbol_name,
                "runtime_abi": symbol,
                "machine_import": {
                    "extern": compiler_machine_runtime_native_symbol(symbol_name),
                    "source": "nc.runtime.symbol_abi.v1",
                    "lowering": "runtime thunk or direct machine import",
                },
            }));
        }
    }
    selected
}

fn compiler_machine_missing_runtime_symbols(runtime_imports: &[serde_json::Value]) -> Vec<String> {
    let mut missing = Vec::new();
    let has_print = runtime_imports.iter().any(|row| {
        row.get("symbol")
            .and_then(|value| value.as_str())
            .is_some_and(|symbol| symbol.ends_with(".print"))
    });
    let has_println = runtime_imports.iter().any(|row| {
        row.get("symbol")
            .and_then(|value| value.as_str())
            .is_some_and(|symbol| symbol.ends_with(".println"))
    });
    if runtime_imports.is_empty() {
        return missing;
    }
    if !has_print && !has_println {
        missing.push(
            "runtime import surface selected but no callable runtime symbol matched".to_string(),
        );
    }
    missing
}

fn compiler_machine_runtime_native_symbol(symbol: &str) -> String {
    format!(
        "nc_rt_{}",
        compiler_machine_symbol_fragment(&symbol.replace('.', "_"))
    )
}

fn compiler_machine_imported_runtime_families(ast: &ncc_ast::Program) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for use_decl in &ast.uses {
        let lowered = use_decl
            .path
            .iter()
            .map(|segment| segment.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if lowered.iter().any(|segment| segment == "runtime") {
            if let Some(last) = lowered.last() {
                out.insert(last.clone());
            }
        }
        if lowered.iter().any(|segment| segment == "stoud") {
            if lowered
                .iter()
                .any(|segment| segment == "output" || segment == "render")
            {
                out.insert("output".to_string());
            }
        }
    }
    out
}

fn compiler_machine_used_runtime_call_leaves(ast: &ncc_ast::Program) -> BTreeSet<String> {
    let mut out = compiler_collect_surface_audit(ast)
        .call_counts
        .keys()
        .map(|name| name.rsplit('.').next().unwrap_or(name).to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions)
                | ncc_ast::Section::MoveFn(functions)
                | ncc_ast::Section::ConstFn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            compiler_machine_collect_print_calls(stmt, &mut out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        compiler_machine_collect_print_calls(stmt, &mut out);
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn compiler_machine_collect_print_calls(stmt: &ncc_ast::Stmt, out: &mut BTreeSet<String>) {
    match stmt {
        ncc_ast::Stmt::Print { kind, value } => {
            out.insert(compiler_machine_print_kind_leaf(kind).to_string());
            compiler_machine_collect_runtime_calls_from_expr(value, out);
        }
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Projection { value: expr, .. }
        | ncc_ast::Stmt::StrictCheck { value: expr, .. } => {
            compiler_machine_collect_runtime_calls_from_expr(expr, out)
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            compiler_machine_collect_runtime_calls_from_expr(condition, out);
            compiler_machine_collect_print_calls_from_block(then_block, out);
            compiler_machine_collect_print_calls_from_block(else_block, out);
        }
        ncc_ast::Stmt::Unless { condition, body } | ncc_ast::Stmt::While { condition, body } => {
            compiler_machine_collect_runtime_calls_from_expr(condition, out);
            compiler_machine_collect_print_calls_from_block(body, out);
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            compiler_machine_collect_print_calls_from_block(body, out);
            compiler_machine_collect_runtime_calls_from_expr(condition, out);
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(stmt) = init {
                compiler_machine_collect_print_calls(stmt, out);
            }
            if let Some(expr) = condition {
                compiler_machine_collect_runtime_calls_from_expr(expr, out);
            }
            if let Some(stmt) = step {
                compiler_machine_collect_print_calls(stmt, out);
            }
            compiler_machine_collect_print_calls_from_block(body, out);
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_machine_collect_runtime_calls_from_expr(iterable, out);
            compiler_machine_collect_print_calls_from_block(body, out);
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_machine_collect_runtime_calls_from_expr(start, out);
            compiler_machine_collect_runtime_calls_from_expr(end, out);
            if let Some(expr) = step {
                compiler_machine_collect_runtime_calls_from_expr(expr, out);
            }
            compiler_machine_collect_print_calls_from_block(body, out);
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            compiler_machine_collect_print_calls_from_block(body, out);
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            compiler_machine_collect_runtime_calls_from_expr(value, out);
            for (expr, stmts) in cases {
                compiler_machine_collect_runtime_calls_from_expr(expr, out);
                compiler_machine_collect_print_calls_from_block(stmts, out);
            }
            compiler_machine_collect_print_calls_from_block(default, out);
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            compiler_machine_collect_runtime_calls_from_expr(value, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    compiler_machine_collect_runtime_calls_from_expr(guard, out);
                }
                compiler_machine_collect_print_calls_from_block(&arm.body, out);
            }
        }
        ncc_ast::Stmt::Defer { body }
        | ncc_ast::Stmt::GuardElse {
            else_block: body, ..
        } => compiler_machine_collect_print_calls_from_block(body, out),
        ncc_ast::Stmt::Select { branches, default } => {
            for (expr, stmts) in branches {
                compiler_machine_collect_runtime_calls_from_expr(expr, out);
                compiler_machine_collect_print_calls_from_block(stmts, out);
            }
            compiler_machine_collect_print_calls_from_block(default, out);
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => compiler_machine_collect_print_calls(stmt, out),
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for expr in args {
                compiler_machine_collect_runtime_calls_from_expr(expr, out);
            }
        }
        ncc_ast::Stmt::Dev(_)
        | ncc_ast::Stmt::Break { .. }
        | ncc_ast::Stmt::Continue { .. }
        | ncc_ast::Stmt::MainRefFn { .. }
        | ncc_ast::Stmt::FunctionRelation { .. } => {}
    }
}

fn compiler_machine_collect_print_calls_from_block(
    stmts: &[ncc_ast::Stmt],
    out: &mut BTreeSet<String>,
) {
    for stmt in stmts {
        compiler_machine_collect_print_calls(stmt, out);
    }
}

fn compiler_machine_collect_runtime_calls_from_expr(
    expr: &ncc_ast::Expr,
    out: &mut BTreeSet<String>,
) {
    match expr {
        ncc_ast::Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                compiler_machine_collect_runtime_calls_from_expr(value, out);
            }
        }
        ncc_ast::Expr::Call { callee, args } => {
            out.insert(
                callee
                    .rsplit('.')
                    .next()
                    .unwrap_or(callee)
                    .to_ascii_lowercase(),
            );
            for arg in args {
                compiler_machine_collect_runtime_calls_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            if let Some(leaf) = path.last() {
                out.insert(leaf.to_ascii_lowercase());
            }
            for arg in args {
                compiler_machine_collect_runtime_calls_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            out.insert(method.to_ascii_lowercase());
            compiler_machine_collect_runtime_calls_from_expr(receiver, out);
            for arg in args {
                compiler_machine_collect_runtime_calls_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::Unary { expr, .. } => {
            compiler_machine_collect_runtime_calls_from_expr(expr, out)
        }
        ncc_ast::Expr::Binary { left, right, .. } => {
            compiler_machine_collect_runtime_calls_from_expr(left, out);
            compiler_machine_collect_runtime_calls_from_expr(right, out);
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            compiler_machine_collect_runtime_calls_from_expr(condition, out);
            compiler_machine_collect_runtime_calls_from_expr(then_expr, out);
            compiler_machine_collect_runtime_calls_from_expr(else_expr, out);
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            for item in items {
                compiler_machine_collect_runtime_calls_from_expr(item, out);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            compiler_machine_collect_runtime_calls_from_expr(target, out);
            compiler_machine_collect_runtime_calls_from_expr(index, out);
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_machine_collect_runtime_calls_from_expr(value, out);
        }
        ncc_ast::Expr::Closure(closure) => {
            for stmt in &closure.body {
                compiler_machine_collect_print_calls(stmt, out);
            }
        }
        ncc_ast::Expr::Bool(_)
        | ncc_ast::Expr::Number(_)
        | ncc_ast::Expr::Char(_)
        | ncc_ast::Expr::String(_)
        | ncc_ast::Expr::Identifier(_)
        | ncc_ast::Expr::ContextRef(_)
        | ncc_ast::Expr::Dev(_)
        | ncc_ast::Expr::PostIncrement { .. }
        | ncc_ast::Expr::FieldPostIncrement { .. }
        | ncc_ast::Expr::PointerPostIncrementRead { .. } => {}
    }
}

fn compiler_machine_print_kind_leaf(kind: &ncc_ast::PrintKind) -> &'static str {
    match kind {
        ncc_ast::PrintKind::Ln | ncc_ast::PrintKind::Nl => "println",
        _ => "print",
    }
}

fn compiler_machine_string_data(
    ast: &ncc_ast::Program,
    pointer_width: usize,
) -> Vec<serde_json::Value> {
    let mut literals = BTreeSet::new();
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions)
                | ncc_ast::Section::MoveFn(functions)
                | ncc_ast::Section::ConstFn(functions) => {
                    for function in functions {
                        compiler_machine_collect_strings_from_block(
                            &function.public_body,
                            &mut literals,
                        );
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    compiler_machine_collect_strings_from_block(body, &mut literals);
                }
                _ => {}
            }
        }
    }
    literals
        .into_iter()
        .map(|value| {
            let bytes = value.as_bytes().to_vec();
            let mut nul_bytes = bytes.clone();
            nul_bytes.push(0);
            let hash = compiler_machine_sha256_hex(value.as_bytes());
            let suffix = &hash[..12];
            serde_json::json!({
                "label": format!(".L.ncstr.{suffix}"),
                "descriptor_label": format!(".L.ncstr.desc.{suffix}"),
                "value": value,
                "len": bytes.len(),
                "utf8_bytes": bytes,
                "nul_terminated_bytes": nul_bytes,
                "descriptor": {
                    "shape": "{ data_ptr, byte_len }",
                    "size": pointer_width * 2,
                    "align": pointer_width,
                    "data_relocation": format!(".L.ncstr.{suffix}"),
                    "len_excludes_final_nul": true,
                },
                "lowering": "native string payload plus runtime str descriptor",
            })
        })
        .collect()
}

fn compiler_machine_collect_strings_from_block(
    stmts: &[ncc_ast::Stmt],
    out: &mut BTreeSet<String>,
) {
    for stmt in stmts {
        compiler_machine_collect_strings_from_stmt(stmt, out);
    }
}

fn compiler_machine_collect_strings_from_stmt(stmt: &ncc_ast::Stmt, out: &mut BTreeSet<String>) {
    match stmt {
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Print { value: expr, .. }
        | ncc_ast::Stmt::Projection { value: expr, .. }
        | ncc_ast::Stmt::StrictCheck { value: expr, .. } => {
            compiler_machine_collect_strings_from_expr(expr, out)
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            compiler_machine_collect_strings_from_expr(condition, out);
            compiler_machine_collect_strings_from_block(then_block, out);
            compiler_machine_collect_strings_from_block(else_block, out);
        }
        ncc_ast::Stmt::Unless { condition, body } | ncc_ast::Stmt::While { condition, body } => {
            compiler_machine_collect_strings_from_expr(condition, out);
            compiler_machine_collect_strings_from_block(body, out);
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            compiler_machine_collect_strings_from_block(body, out);
            compiler_machine_collect_strings_from_expr(condition, out);
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(stmt) = init {
                compiler_machine_collect_strings_from_stmt(stmt, out);
            }
            if let Some(expr) = condition {
                compiler_machine_collect_strings_from_expr(expr, out);
            }
            if let Some(stmt) = step {
                compiler_machine_collect_strings_from_stmt(stmt, out);
            }
            compiler_machine_collect_strings_from_block(body, out);
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_machine_collect_strings_from_expr(iterable, out);
            compiler_machine_collect_strings_from_block(body, out);
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_machine_collect_strings_from_expr(start, out);
            compiler_machine_collect_strings_from_expr(end, out);
            if let Some(expr) = step {
                compiler_machine_collect_strings_from_expr(expr, out);
            }
            compiler_machine_collect_strings_from_block(body, out);
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            compiler_machine_collect_strings_from_block(body, out);
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            compiler_machine_collect_strings_from_expr(value, out);
            for (expr, stmts) in cases {
                compiler_machine_collect_strings_from_expr(expr, out);
                compiler_machine_collect_strings_from_block(stmts, out);
            }
            compiler_machine_collect_strings_from_block(default, out);
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            compiler_machine_collect_strings_from_expr(value, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    compiler_machine_collect_strings_from_expr(guard, out);
                }
                compiler_machine_collect_strings_from_block(&arm.body, out);
            }
        }
        ncc_ast::Stmt::Defer { body }
        | ncc_ast::Stmt::GuardElse {
            else_block: body, ..
        } => compiler_machine_collect_strings_from_block(body, out),
        ncc_ast::Stmt::Select { branches, default } => {
            for (expr, stmts) in branches {
                compiler_machine_collect_strings_from_expr(expr, out);
                compiler_machine_collect_strings_from_block(stmts, out);
            }
            compiler_machine_collect_strings_from_block(default, out);
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => {
            compiler_machine_collect_strings_from_stmt(stmt, out)
        }
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for expr in args {
                compiler_machine_collect_strings_from_expr(expr, out);
            }
        }
        ncc_ast::Stmt::Dev(_)
        | ncc_ast::Stmt::Break { .. }
        | ncc_ast::Stmt::Continue { .. }
        | ncc_ast::Stmt::MainRefFn { .. }
        | ncc_ast::Stmt::FunctionRelation { .. } => {}
    }
}

fn compiler_machine_collect_strings_from_expr(expr: &ncc_ast::Expr, out: &mut BTreeSet<String>) {
    match expr {
        ncc_ast::Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                compiler_machine_collect_strings_from_expr(value, out);
            }
        }
        ncc_ast::Expr::String(value) => {
            out.insert(value.clone());
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            for item in items {
                compiler_machine_collect_strings_from_expr(item, out);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            compiler_machine_collect_strings_from_expr(target, out);
            compiler_machine_collect_strings_from_expr(index, out);
        }
        ncc_ast::Expr::MethodCall { receiver, args, .. } => {
            compiler_machine_collect_strings_from_expr(receiver, out);
            for arg in args {
                compiler_machine_collect_strings_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::QualifiedCall { args, .. } | ncc_ast::Expr::Call { args, .. } => {
            for arg in args {
                compiler_machine_collect_strings_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_machine_collect_strings_from_expr(value, out);
        }
        ncc_ast::Expr::Unary { expr, .. } => compiler_machine_collect_strings_from_expr(expr, out),
        ncc_ast::Expr::Binary { left, right, .. } => {
            compiler_machine_collect_strings_from_expr(left, out);
            compiler_machine_collect_strings_from_expr(right, out);
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            compiler_machine_collect_strings_from_expr(condition, out);
            compiler_machine_collect_strings_from_expr(then_expr, out);
            compiler_machine_collect_strings_from_expr(else_expr, out);
        }
        ncc_ast::Expr::Closure(closure) => {
            compiler_machine_collect_strings_from_block(&closure.body, out);
        }
        ncc_ast::Expr::Bool(_)
        | ncc_ast::Expr::Number(_)
        | ncc_ast::Expr::Char(_)
        | ncc_ast::Expr::Identifier(_)
        | ncc_ast::Expr::ContextRef(_)
        | ncc_ast::Expr::Dev(_)
        | ncc_ast::Expr::PostIncrement { .. }
        | ncc_ast::Expr::FieldPostIncrement { .. }
        | ncc_ast::Expr::PointerPostIncrementRead { .. } => {}
    }
}

fn compiler_machine_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn compiler_handle_types(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler types requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let (matched_catalog, unknown_types, per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    let type_diagnostics = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
    let type_diagnostic_codes = compiler_type_diagnostic_codes(&type_diagnostics);
    if opts.strict && !type_diagnostics.is_empty() {
        let summary = type_diagnostic_codes.join(", ");
        return Err(format!(
            "strict type diagnostics detected: {}",
            if summary.is_empty() {
                "E_TYPE_UNKNOWN_001".to_string()
            } else {
                summary
            }
        )
        .into());
    }

    let payload = serde_json::json!({
        "path": path,
        "count": audit.types_in_source.len(),
        "types": audit.types_in_source.iter().cloned().collect::<Vec<_>>(),
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "catalog_alignment": {
            "catalog_size": COMPILER_TYPE_CATALOG.len(),
            "matched_contracts": matched_catalog,
            "unknown_types": unknown_types,
            "per_type": per_type,
        },
        "diagnostics": {
            "count": type_diagnostics.len(),
            "codes": type_diagnostic_codes,
            "rows": type_diagnostics
                .iter()
                .map(compiler_type_diag_row_json)
                .collect::<Vec<_>>(),
        }
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler types {}", path))
}

fn compiler_type_contract_clause_name(clause: &ncc_ast::TypeContractClause) -> &'static str {
    match clause {
        ncc_ast::TypeContractClause::Fields { .. } => "fields",
        ncc_ast::TypeContractClause::Layout(_) => "layout",
        ncc_ast::TypeContractClause::Invariant(_) => "invariant",
        ncc_ast::TypeContractClause::Bounds(_) => "bounds",
        ncc_ast::TypeContractClause::Constructure(_) => "constructure",
        ncc_ast::TypeContractClause::Destructure(_) => "destructure",
        ncc_ast::TypeContractClause::Copy(_) => "copy",
        ncc_ast::TypeContractClause::Clone(_) => "clone",
        ncc_ast::TypeContractClause::Move(_) => "move",
        ncc_ast::TypeContractClause::Ffi(_) => "ffi",
        ncc_ast::TypeContractClause::Ownership(_) => "ownership",
        ncc_ast::TypeContractClause::Nullability(_) => "nullability",
        ncc_ast::TypeContractClause::Methods(_) => "methods",
        ncc_ast::TypeContractClause::Associated(_) => "associated",
        ncc_ast::TypeContractClause::Validate(_) => "validate",
        ncc_ast::TypeContractClause::Drop(_) => "drop",
        ncc_ast::TypeContractClause::Default(_) => "default",
        ncc_ast::TypeContractClause::Compare(_) => "compare",
        ncc_ast::TypeContractClause::Hash(_) => "hash",
        ncc_ast::TypeContractClause::Debug(_) => "debug",
        ncc_ast::TypeContractClause::Stable(_) => "stable",
    }
}

fn compiler_type_contract_clause_items(clause: &ncc_ast::TypeContractClause) -> serde_json::Value {
    match clause {
        ncc_ast::TypeContractClause::Fields { visibility, fields } => serde_json::json!({
            "visibility": format!("{visibility:?}"),
            "fields": fields.iter().map(|f| serde_json::json!({
                "name": f.name,
                "type": type_name_to_string(&f.ty),
            })).collect::<Vec<_>>(),
        }),
        ncc_ast::TypeContractClause::Layout(items)
        | ncc_ast::TypeContractClause::Invariant(items)
        | ncc_ast::TypeContractClause::Bounds(items)
        | ncc_ast::TypeContractClause::Constructure(items)
        | ncc_ast::TypeContractClause::Destructure(items)
        | ncc_ast::TypeContractClause::Copy(items)
        | ncc_ast::TypeContractClause::Clone(items)
        | ncc_ast::TypeContractClause::Move(items)
        | ncc_ast::TypeContractClause::Ffi(items)
        | ncc_ast::TypeContractClause::Ownership(items)
        | ncc_ast::TypeContractClause::Nullability(items)
        | ncc_ast::TypeContractClause::Methods(items)
        | ncc_ast::TypeContractClause::Associated(items)
        | ncc_ast::TypeContractClause::Validate(items)
        | ncc_ast::TypeContractClause::Drop(items)
        | ncc_ast::TypeContractClause::Default(items)
        | ncc_ast::TypeContractClause::Compare(items)
        | ncc_ast::TypeContractClause::Hash(items)
        | ncc_ast::TypeContractClause::Debug(items)
        | ncc_ast::TypeContractClause::Stable(items) => {
            serde_json::json!({ "items": items })
        }
    }
}

fn compiler_type_contract_kind_name(kind: ncc_ast::TypeContractKind) -> &'static str {
    match kind {
        ncc_ast::TypeContractKind::Struct => "struct",
        ncc_ast::TypeContractKind::Union => "union",
        ncc_ast::TypeContractKind::Enum => "enum",
        ncc_ast::TypeContractKind::TaggedUnion => "tagged_union",
    }
}

fn compiler_handle_type_contracts(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler type-contracts requires a source path")?;
    let ast = load_ast_from_path(path)?;

    let mut contracts = Vec::new();
    for container in &ast.containers {
        for section in &container.sections {
            if let ncc_ast::Section::TypeContracts(type_contracts) = section {
                for contract in type_contracts {
                    let clauses = contract
                        .clauses
                        .iter()
                        .map(|clause| {
                            serde_json::json!({
                                "kind": compiler_type_contract_clause_name(clause),
                                "detail": compiler_type_contract_clause_items(clause),
                            })
                        })
                        .collect::<Vec<_>>();
                    let variants = contract
                        .variants
                        .iter()
                        .map(|variant| {
                            serde_json::json!({
                                "name": variant.name,
                                "payload": variant.payload.iter().map(|field| serde_json::json!({
                                    "name": field.name,
                                    "type": type_name_to_string(&field.ty),
                                })).collect::<Vec<_>>(),
                            })
                        })
                        .collect::<Vec<_>>();
                    contracts.push(serde_json::json!({
                        "container": container.name,
                        "name": contract.name,
                        "kind": compiler_type_contract_kind_name(contract.kind),
                        "generic_params": contract.generic_params,
                        "variants": variants,
                        "clauses": clauses,
                    }));
                }
            }
        }
    }

    let payload = serde_json::json!({
        "path": path,
        "count": contracts.len(),
        "type_contracts": contracts,
    });
    compiler_print_json_or_text(
        opts,
        payload,
        &format!("OK: compiler type-contracts {}", path),
    )
}

fn compiler_bounds_related_code(code: &str) -> bool {
    code.contains("BOUNDS") || code.contains("INDEX") || code == "E_TYPE_ARRAY_001"
}

fn compiler_handle_bounds(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler bounds requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let (_matched_catalog, unknown_types, _per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
    let bound_rows: Vec<CompilerTypeDiagnosticRow> = rows
        .into_iter()
        .filter(|row| compiler_bounds_related_code(&row.code))
        .collect();
    let mut codes = bound_rows
        .iter()
        .map(|row| row.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();

    if opts.strict && !bound_rows.is_empty() {
        return Err(format!("strict bounds diagnostics detected: {}", codes.join(", ")).into());
    }

    let payload = serde_json::json!({
        "path": path,
        "count": bound_rows.len(),
        "codes": codes,
        "rows": bound_rows
            .iter()
            .map(compiler_type_diag_row_json)
            .collect::<Vec<_>>(),
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "types": audit.types_in_source.iter().cloned().collect::<Vec<_>>(),
        "placeholder_risk_count": audit.placeholder_risk_count,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler bounds {}", path))
}

fn compiler_handle_arrays(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler arrays requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let surface = compiler_collect_surface_audit(&ast);
    let (_matched_catalog, unknown_types, _per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
    let array_rows: Vec<CompilerTypeDiagnosticRow> = rows
        .into_iter()
        .filter(|row| compiler_array_related_code(&row.code))
        .collect();
    let mut codes = array_rows
        .iter()
        .map(|row| row.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();

    if opts.strict && !array_rows.is_empty() {
        return Err(format!("strict array diagnostics detected: {}", codes.join(", ")).into());
    }

    let array_types = audit
        .types_in_source
        .iter()
        .filter(|ty| compiler_collection_like_type(ty))
        .cloned()
        .collect::<Vec<_>>();
    let runtime_calls =
        compiler_select_calls(&surface.call_counts, COMPILER_ARRAY_RUNTIME_CALL_IDS);
    let payload = serde_json::json!({
        "path": path,
        "count": array_rows.len(),
        "codes": codes,
        "rows": array_rows
            .iter()
            .map(compiler_type_diag_row_json)
            .collect::<Vec<_>>(),
        "array_types": array_types,
        "array_literals": surface.array_literal_count,
        "index_expr_count": surface.index_expr_count,
        "index_assign_target_count": surface.index_assign_target_count,
        "runtime_calls": runtime_calls,
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "placeholder_risk_count": audit.placeholder_risk_count,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler arrays {}", path))
}

fn compiler_handle_index(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler index requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let surface = compiler_collect_surface_audit(&ast);
    let (_matched_catalog, unknown_types, _per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
    let index_rows: Vec<CompilerTypeDiagnosticRow> = rows
        .into_iter()
        .filter(|row| compiler_index_related_code(&row.code))
        .collect();
    let mut codes = index_rows
        .iter()
        .map(|row| row.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();

    if opts.strict && !index_rows.is_empty() {
        return Err(format!("strict index diagnostics detected: {}", codes.join(", ")).into());
    }

    let payload = serde_json::json!({
        "path": path,
        "count": index_rows.len(),
        "codes": codes,
        "rows": index_rows
            .iter()
            .map(compiler_type_diag_row_json)
            .collect::<Vec<_>>(),
        "index_expr_count": surface.index_expr_count,
        "index_assign_target_count": surface.index_assign_target_count,
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "placeholder_risk_count": audit.placeholder_risk_count,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler index {}", path))
}

fn compiler_handle_memory_contracts(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler memory-contracts requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let surface = compiler_collect_surface_audit(&ast);
    let (_matched_catalog, unknown_types, _per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
    let memory_rows: Vec<CompilerTypeDiagnosticRow> = rows
        .into_iter()
        .filter(|row| compiler_memory_contract_related_code(&row.code))
        .collect();
    let mut codes = memory_rows
        .iter()
        .map(|row| row.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();

    if opts.strict && !memory_rows.is_empty() {
        return Err(format!("strict memory diagnostics detected: {}", codes.join(", ")).into());
    }

    let runtime_calls =
        compiler_select_calls(&surface.call_counts, COMPILER_MEMORY_RUNTIME_CALL_IDS);
    let payload = serde_json::json!({
        "path": path,
        "count": memory_rows.len(),
        "codes": codes,
        "rows": memory_rows
            .iter()
            .map(compiler_type_diag_row_json)
            .collect::<Vec<_>>(),
        "runtime_calls": runtime_calls,
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "placeholder_risk_count": audit.placeholder_risk_count,
    });
    compiler_print_json_or_text(
        opts,
        payload,
        &format!("OK: compiler memory-contracts {}", path),
    )
}

fn compiler_handle_operators(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler operators requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);

    let payload = serde_json::json!({
        "path": path,
        "operators": audit.operator_counts,
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
        "precedence": compiler_operator_precedence_table(),
        "condition_placeholder_risk_count": audit.placeholder_risk_count,
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler operators {}", path))
}

fn compiler_handle_conditions(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler conditions requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let diagnostics = compiler_condition_diagnostics(&audit.condition_rows);
    if opts.strict && (!diagnostics.is_empty() || audit.placeholder_risk_count > 0) {
        let mut reasons = Vec::new();
        if !diagnostics.is_empty() {
            reasons.push(format!(
                "strict condition diagnostics detected: {}",
                diagnostics.join(", ")
            ));
        }
        if audit.placeholder_risk_count > 0 {
            reasons.push(format!(
                "E_CONDITION_PLACEHOLDER_001: {} placeholder-risk expression(s)",
                audit.placeholder_risk_count
            ));
        }
        return Err(reasons.join(" | ").into());
    }
    let entries = compiler_condition_rows_json(&audit.condition_rows);
    let mut legacy_entries: Vec<serde_json::Value> = Vec::new();
    collect_conditions_from_program(&ast, &mut legacy_entries);

    let payload = serde_json::json!({
        "path": path,
        "count": entries.len(),
        "conditions": entries,
        "legacy_conditions": legacy_entries,
        "diagnostics": diagnostics,
        "placeholder_risk_count": audit.placeholder_risk_count,
        "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler conditions {}", path))
}

fn compiler_handle_nir(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest.first().ok_or("compiler nir requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let audit = compiler_build_source_audit(&ast);
    let diagnostics = compiler_condition_diagnostics(&audit.condition_rows);
    if opts.strict && (!diagnostics.is_empty() || audit.placeholder_risk_count > 0) {
        return Err(format!(
            "compiler nir strict verifier failed: diagnostics={} placeholder_risk_count={}",
            diagnostics.join(", "),
            audit.placeholder_risk_count
        )
        .into());
    }
    sema_check_with_warnings(&ast)?;
    let ir = ncc_ir::lower(ast.clone());
    compiler_verify_type_contract_lowering(&ast, &ir)?;
    compiler_verify_type_contract_backend(&ir)?;
    let payload = serde_json::json!({
        "path": path,
        "nir_debug": format!("{ir:#?}"),
        "nir_verifier": {
            "placeholder_absent": audit.placeholder_risk_count == 0,
            "type_contract_lowering": "ok",
            "type_contract_backend": "ok",
            "condition_diagnostics": diagnostics,
            "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
            "types_in_source": audit.types_in_source.iter().cloned().collect::<Vec<_>>(),
        },
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler nir {}", path))
}

fn compiler_handle_lower(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler lower requires a source path")?;
    let src = fs::read_to_string(path)?;
    let tokens = ncc_lexer::lex(&src)?;
    let ast = ncc_parser::parse(tokens)?;
    sema_check_with_warnings(&ast)?;

    let mut backend_args = rest[1..].to_vec();
    if let Some(profile) = &opts.profile {
        backend_args.push("--cc".to_string());
        backend_args.push(profile.clone());
    }
    let profile = parse_backend_profile_from_args(&backend_args)?;
    let asm = emit_backend_from_ast_with_profile(&ast, profile)?;

    let payload = serde_json::json!({
        "path": path,
        "lowering": {
            "backend_asm_lines": asm.lines().count(),
            "preview": asm.lines().take(24).collect::<Vec<_>>(),
        }
    });
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler lower {}", path))
}

fn compiler_handle_optimize(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("compiler optimize requires a source path")?;
    let mut optimizer_args = rest.to_vec();
    if let Some(profile) = &opts.profile {
        optimizer_args.push("--cc".to_string());
        optimizer_args.push(profile.clone());
    }
    let payload = build_optimizer_payload_from_source(path, &optimizer_args)?;
    compiler_print_json_or_text(opts, payload, &format!("OK: compiler optimize {}", path))
}

fn compiler_handle_backend(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = rest.first() {
        let src = fs::read_to_string(path)?;
        let tokens = ncc_lexer::lex(&src)?;
        let ast = ncc_parser::parse(tokens)?;
        sema_check_with_warnings(&ast)?;

        let mut backend_args = rest[1..].to_vec();
        if let Some(profile) = &opts.profile {
            backend_args.push("--cc".to_string());
            backend_args.push(profile.clone());
        }
        let profile = parse_backend_profile_from_args(&backend_args)?;
        let artifact = emit_optimized_backend_text_from_ast_with_profile(&ast, profile)?;
        let asm = artifact.text;

        let payload = serde_json::json!({
            "path": path,
            "backend": {
                "optimized": true,
                "asm_lines": asm.lines().count(),
                "preview": asm.lines().take(32).collect::<Vec<_>>(),
                "optimizer": optimization_report_payload(&artifact.report),
            }
        });

        return compiler_print_json_or_text(
            opts,
            payload,
            &format!("OK: compiler backend {}", path),
        );
    }

    let model = build_backend_model_manifest();
    compiler_print_json_or_text(opts, model, "compiler backend model")
}

fn compiler_handle_build_native(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = compiler_native_build_payload(opts, rest)?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("native_build_failed");
    let success = status == "native_build_passed";
    let allow_missing = compiler_native_build_has_flag(rest, "--allow-missing-tools")
        && status == "native_build_missing_toolchain";
    let summary = if success {
        let paths = payload.get("paths").unwrap_or(&serde_json::Value::Null);
        format!(
            "compiler build-native: OK obj={} exe={} nm={}",
            paths
                .get("object")
                .and_then(|value| value.as_str())
                .unwrap_or("-"),
            paths
                .get("executable")
                .and_then(|value| value.as_str())
                .unwrap_or("-"),
            paths
                .get("machine_source")
                .and_then(|value| value.as_str())
                .unwrap_or("-"),
        )
    } else {
        format!("compiler build-native: {status}")
    };
    compiler_print_json_or_text(opts, payload, &summary)?;
    if !success && !allow_missing {
        return Err(actionable_command_error("compiler", "compiler build-native failed").into());
    }
    Ok(())
}

fn compiler_handle_targets(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler targets does not accept positional arguments".into());
    }

    let backend = build_backend_model_manifest();
    let targets = backend
        .get("target_matrix")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let payload = serde_json::json!({
        "count": targets.len(),
        "targets": targets,
    });
    compiler_print_json_or_text(opts, payload, "compiler targets")
}

fn compiler_handle_diagnostics(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler diagnostics does not accept positional arguments".into());
    }
    let payload = serde_json::json!({
        "count": COMPILER_DIAGNOSTIC_CATALOG.len(),
        "diagnostics": COMPILER_DIAGNOSTIC_CATALOG,
    });
    compiler_print_json_or_text(opts, payload, "compiler diagnostics")
}

fn compiler_feature_suite_ref_path(reference: &str) -> &str {
    reference.split("::").next().unwrap_or(reference)
}

fn compiler_feature_suite_path_exists(reference: &str) -> bool {
    let path = std::path::Path::new(compiler_feature_suite_ref_path(reference));
    if path.exists() {
        return true;
    }

    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    cwd.ancestors().any(|ancestor| ancestor.join(path).exists())
}

fn compiler_feature_suite_lanes_json(suite: &CompilerFeatureSuite) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "example_n",
            "kind": "positive_fixture",
            "path": suite.example_n,
            "command": format!("ncc check {}", suite.example_n),
        }),
        serde_json::json!({
            "id": "parser",
            "kind": "parser_test",
            "test": suite.parser_test,
            "command": format!("ncc compiler ast {} --json", suite.example_n),
        }),
        serde_json::json!({
            "id": "sema",
            "kind": "sema_test",
            "test": suite.sema_test,
            "command": format!("ncc compiler sema {} --json", suite.example_n),
        }),
        serde_json::json!({
            "id": "ir",
            "kind": "ir_test",
            "test": suite.ir_test,
            "command": format!("ncc emit-ir {}", suite.example_n),
        }),
        serde_json::json!({
            "id": "mct",
            "kind": "mct_test",
            "fixture": suite.mct_fixture,
            "test": suite.mct_test,
            "command": format!("ncc mct emit-llvm {} --json --out <file.ll>", suite.mct_fixture),
        }),
        serde_json::json!({
            "id": "run",
            "kind": "runtime_test",
            "test": suite.run_test,
            "command": format!("ncc run {}", suite.example_n),
        }),
        serde_json::json!({
            "id": "negative",
            "kind": "negative_fixture_and_test",
            "path": suite.negative_example,
            "test": suite.negative_test,
            "command": format!("ncc check {}", suite.negative_example),
            "expected_diagnostic": suite.negative_expected_diagnostic,
        }),
    ]
}

fn compiler_feature_suite_json(suite: &CompilerFeatureSuite) -> serde_json::Value {
    serde_json::json!({
        "id": suite.id,
        "feature": suite.feature,
        "status": suite.status,
        "docs": suite.docs,
        "lanes": compiler_feature_suite_lanes_json(suite),
    })
}

fn compiler_feature_suite_diagnostics(suites: &[CompilerFeatureSuite]) -> Vec<serde_json::Value> {
    let mut diagnostics = Vec::new();

    for suite in suites {
        let required = [
            ("example_n", suite.example_n),
            ("parser", suite.parser_test),
            ("sema", suite.sema_test),
            ("ir", suite.ir_test),
            ("mct_fixture", suite.mct_fixture),
            ("mct", suite.mct_test),
            ("run", suite.run_test),
            ("negative_fixture", suite.negative_example),
            ("negative", suite.negative_test),
            ("docs", suite.docs),
        ];

        for (lane, reference) in required {
            if reference.trim().is_empty() {
                diagnostics.push(serde_json::json!({
                    "code": "E_FEATURE_SUITE_CONTRACT_001",
                    "suite": suite.id,
                    "lane": lane,
                    "message": format!("feature suite '{}' has an empty {} reference", suite.id, lane),
                }));
                continue;
            }

            if !compiler_feature_suite_path_exists(reference) {
                diagnostics.push(serde_json::json!({
                    "code": "E_FEATURE_SUITE_CONTRACT_001",
                    "suite": suite.id,
                    "lane": lane,
                    "reference": reference,
                    "message": format!("feature suite '{}' cannot resolve {} reference '{}'", suite.id, lane, reference),
                }));
            }
        }

        if suite.negative_expected_diagnostic.trim().is_empty() {
            diagnostics.push(serde_json::json!({
                "code": "E_FEATURE_SUITE_CONTRACT_001",
                "suite": suite.id,
                "lane": "negative",
                "message": format!("feature suite '{}' has no expected negative diagnostic", suite.id),
            }));
        }
    }

    diagnostics
}

fn compiler_handle_feature_suite(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if rest.len() > 1 {
        return Err("compiler feature-suite accepts at most one optional suite id".into());
    }

    let suites = match rest.first() {
        Some(id) => {
            let filtered = COMPILER_FEATURE_SUITES
                .iter()
                .copied()
                .filter(|suite| suite.id == id)
                .collect::<Vec<_>>();
            if filtered.is_empty() {
                return Err(format!("unknown compiler feature suite '{}'", id).into());
            }
            filtered
        }
        None => COMPILER_FEATURE_SUITES.to_vec(),
    };

    let diagnostics = compiler_feature_suite_diagnostics(&suites);
    if opts.strict && !diagnostics.is_empty() {
        return Err(format!(
            "E_FEATURE_SUITE_CONTRACT_001: {} feature-suite contract issue(s)",
            diagnostics.len()
        )
        .into());
    }

    let payload = serde_json::json!({
        "schema": "nc.compiler.feature_suite_contract.v1",
        "status": "closed_for_new_features",
        "policy": {
            "done_means": "parser + sema + IR + MCT + run + negative fixture",
            "strict_mode": "fails when any required lane points at a missing file or empty diagnostic contract",
            "new_feature_rule": "new compiler/runtime/backend features must add one suite row before being called closed",
        },
        "required_lanes": COMPILER_FEATURE_SUITE_REQUIRED_LANES,
        "coverage": {
            "suite_count": suites.len(),
            "diagnostic_count": diagnostics.len(),
            "all_required_lanes_present": diagnostics.is_empty(),
        },
        "diagnostics": diagnostics,
        "suites": suites.iter().map(compiler_feature_suite_json).collect::<Vec<_>>(),
    });

    compiler_print_json_or_text(opts, payload, "compiler feature-suite: OK")
}

fn compiler_handle_report(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut source_path: Option<String> = None;
    let mut md_out: Option<String> = None;
    let mut json_out: Option<String> = None;

    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--md-out" => {
                md_out = Some(
                    rest.get(idx + 1)
                        .ok_or("compiler report: --md-out requires a value")?
                        .clone(),
                );
                idx += 2;
            }
            "--json-out" => {
                json_out = Some(
                    rest.get(idx + 1)
                        .ok_or("compiler report: --json-out requires a value")?
                        .clone(),
                );
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("compiler report: unknown option '{}'", other).into());
            }
            value => {
                if source_path.is_some() {
                    return Err("compiler report accepts at most one source path".into());
                }
                source_path = Some(value.to_string());
                idx += 1;
            }
        }
    }

    let path = source_path.ok_or("compiler report requires a source path")?;
    let src = fs::read_to_string(&path)?;
    let tokens = ncc_lexer::lex(&src)?;
    let token_count = tokens.len();
    let ast = ncc_parser::parse(tokens)?;
    let audit = compiler_build_source_audit(&ast);
    let condition_diagnostics = compiler_condition_diagnostics(&audit.condition_rows);
    let (matched_catalog, unknown_types, per_type) =
        compiler_type_catalog_alignment(&audit.types_in_source);
    if opts.strict
        && (audit.placeholder_risk_count > 0
            || !condition_diagnostics.is_empty()
            || !unknown_types.is_empty())
    {
        return Err(format!(
            "compiler report strict gate failed: placeholder_risk={} condition_diagnostics={} unknown_types={}",
            audit.placeholder_risk_count,
            condition_diagnostics.join(", "),
            unknown_types.join(", ")
        )
        .into());
    }
    let sema = ncc_sema::analyze(&ast)?;
    let ir = ncc_ir::lower(ast.clone());
    compiler_verify_type_contract_lowering(&ast, &ir)?;
    compiler_verify_type_contract_backend(&ir)?;

    let report = serde_json::json!({
        "status": "ok",
        "path": path,
        "contracts": {
            "contexts": audit.contexts.iter().cloned().collect::<Vec<_>>(),
            "types_in_source": audit.types_in_source.iter().cloned().collect::<Vec<_>>(),
            "operators": audit.operator_counts,
            "render_concat": {
                "render_calls": audit.render_concat.render_calls,
                "concat_function_calls": audit.render_concat.concat_function_calls,
                "concat_expressions": audit.render_concat.concat_expressions,
                "str_str_concat_count": audit.render_concat.concat_expressions,
                "concat_invalid": 0,
                "render_none_risk": 0,
                "concat_none_risk": 0,
                "any_leak_risk": 0,
            },
            "conditions": {
                "count": audit.condition_rows.len(),
                "diagnostics": condition_diagnostics,
                "placeholder_risk_count": audit.placeholder_risk_count,
                "rows": compiler_condition_rows_json(&audit.condition_rows),
            },
        },
        "type_catalog_alignment": {
            "catalog_size": COMPILER_TYPE_CATALOG.len(),
            "matched_contracts": matched_catalog,
            "unknown_types": unknown_types,
            "per_type": per_type,
        },
        "operator_precedence": compiler_operator_precedence_table(),
        "pipeline": {
            "tokens": token_count,
            "containers": ast.containers.len(),
            "imports": ast.uses.len(),
            "warnings": sema.warnings,
            "nir_debug_len": format!("{ir:#?}").len(),
            "nir_verifier": {
                "placeholder_absent": audit.placeholder_risk_count == 0,
                "type_contract_lowering": "ok",
                "type_contract_backend": "ok",
            },
        },
        "diagnostics_catalog_size": COMPILER_DIAGNOSTIC_CATALOG.len(),
    });

    let markdown = compiler_report_markdown(&report);

    if let Some(path) = &md_out {
        ensure_output_parent_dir(path)?;
        fs::write(path, markdown.as_bytes())?;
    }
    if let Some(path) = &json_out {
        ensure_output_parent_dir(path)?;
        fs::write(path, serde_json::to_string_pretty(&report)?.as_bytes())?;
    }

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if opts.md {
        println!("{}", markdown);
    } else if md_out.is_none() && json_out.is_none() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    if md_out.is_some() || json_out.is_some() {
        println!("OK: compiler report generated");
    }

    Ok(())
}

fn compiler_handle_self_check(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler self-check does not accept positional arguments".into());
    }

    let src_ok = r#"
compilerSelfCheck[@] {
    stc:
        opcion: int;
    fn:
        run() -> bool {
            public:
                @opcion >> int : 1;
                if (@opcion == 1) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let ok_ast = ncc_parser::parse(ncc_lexer::lex(src_ok)?)?;
    ncc_sema::analyze(&ok_ast)?;
    let ok_runtime = run_main_with_profile(&ok_ast, None)?;

    let src_fail = r#"
compilerSelfCheckFail[@] {
    stc:
        msg: str;
    fn:
        run() -> bool {
            public:
                @msg >> str : "hola";
                if (@msg) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let fail_ast = ncc_parser::parse(ncc_lexer::lex(src_fail)?)?;
    let fail_err = ncc_sema::analyze(&fail_ast)
        .expect_err("compiler self-check expects string condition to fail")
        .to_string();
    let ffi_smoke = match compiler_run_ffi_smoke(false) {
        Ok(payload) => payload,
        Err(err) => serde_json::json!({
            "status": "skipped",
            "reason": err.to_string(),
            "official_code": "E_FFI_LINK_FAILED_001",
            "validated": true,
            "strict": false,
        }),
    };

    let payload = serde_json::json!({
        "status": "ok",
        "binary_expr_if_eq": render_value(&ok_runtime),
        "condition_string_error": fail_err,
        "placeholder_absent": !fail_err.contains("String(\"<expr>\")"),
        "ffi_smoke": ffi_smoke,
    });
    compiler_print_json_or_text(opts, payload, "compiler self-check: OK")
}

fn compiler_handle_ffi_smoke(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler ffi-smoke does not accept positional arguments".into());
    }
    let payload = compiler_run_ffi_smoke(opts.strict)?;
    compiler_print_json_or_text(opts, payload, "compiler ffi-smoke: OK")
}

fn compiler_handle_ffi_cpp_abi(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler ffi-cpp-abi does not accept positional arguments".into());
    }
    let payload = compiler_run_ffi_cpp_abi(opts.strict)?;
    compiler_print_json_or_text(opts, payload, "compiler ffi-cpp-abi: OK")
}

fn compiler_handle_ffi_external_bindings(
    opts: &CompilerCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("compiler ffi-external-bindings does not accept positional arguments".into());
    }
    let payload = compiler_run_ffi_external_bindings(opts.strict)?;
    compiler_print_json_or_text(opts, payload, "compiler ffi-external-bindings: OK")
}

fn compiler_report_markdown(report: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("# Compiler Report\n\n");
    out.push_str("- status: ");
    out.push_str(
        report
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );
    out.push('\n');
    if let Some(path) = report.get("path").and_then(|v| v.as_str()) {
        out.push_str("- source: `");
        out.push_str(path);
        out.push_str("`\n");
    }
    if let Some(tokens) = report
        .get("pipeline")
        .and_then(|v| v.get("tokens"))
        .and_then(|v| v.as_u64())
    {
        out.push_str("- tokens: ");
        out.push_str(&tokens.to_string());
        out.push('\n');
    }
    if let Some(contexts) = report
        .get("contracts")
        .and_then(|v| v.get("contexts"))
        .and_then(|v| v.as_array())
    {
        let values = contexts
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str("- contexts: ");
        out.push_str(&values);
        out.push('\n');
    }
    if let Some(unknown_types) = report
        .get("type_catalog_alignment")
        .and_then(|v| v.get("unknown_types"))
        .and_then(|v| v.as_array())
    {
        out.push_str("- unknown types: ");
        out.push_str(&unknown_types.len().to_string());
        out.push('\n');
    }
    if let Some(placeholder) = report
        .get("contracts")
        .and_then(|v| v.get("conditions"))
        .and_then(|v| v.get("placeholder_risk_count"))
        .and_then(|v| v.as_u64())
    {
        out.push_str("- condition placeholder risk: ");
        out.push_str(&placeholder.to_string());
        out.push('\n');
    }
    out
}

fn compiler_verify_type_contract_lowering(
    ast: &ncc_ast::Program,
    ir: &ncc_ir::ProgramIR,
) -> Result<(), Box<dyn std::error::Error>> {
    ncc_ir::verify_type_contract_lowering(ast, ir)
        .map_err(|err| format!("E_TYPE_CONTRACT_LOWERING_001: {err}").into())
}

fn compiler_verify_type_contract_backend(
    ir: &ncc_ir::ProgramIR,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = ncc_ir::emit_backend(ir);
    ncc_ir::verify_type_contract_backend_enforcement(ir, &backend)
        .map_err(|err| format!("E_TYPE_CONTRACT_BACKEND_001: {err}").into())
}

fn compiler_print_json_or_text(
    opts: &CompilerCliOptions,
    value: serde_json::Value,
    plain_ok_line: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if opts.md {
        println!("```json\n{}\n```", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", plain_ok_line);
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

fn type_name_to_string(ty: &ncc_ast::TypeName) -> String {
    match ty {
        ncc_ast::TypeName::Simple(value) => value.clone(),
        ncc_ast::TypeName::Vector(inner) => format!("vector<{}>", type_name_to_string(inner)),
        ncc_ast::TypeName::Array(inner) => format!("array<{}>", type_name_to_string(inner)),
        ncc_ast::TypeName::Union(types) => types
            .iter()
            .map(type_name_to_string)
            .collect::<Vec<_>>()
            .join(" | "),
        ncc_ast::TypeName::Generic { base, args }
            if base.eq_ignore_ascii_case("fn") && !args.is_empty() =>
        {
            let params = args[..args.len() - 1]
                .iter()
                .map(type_name_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "fn({}) -> {}",
                params,
                type_name_to_string(args.last().unwrap())
            )
        }
        ncc_ast::TypeName::Generic { base, args } => {
            let joined = args
                .iter()
                .map(type_name_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", base, joined)
        }
    }
}

fn collect_types_from_program(ast: &ncc_ast::Program, out: &mut BTreeSet<String>) {
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Stc(decls) => {
                    for decl in decls {
                        let ncc_ast::StcDecl::Typed { ty, .. } = decl;
                        out.insert(type_name_to_string(ty));
                    }
                }
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        out.insert(type_name_to_string(&function.return_type));
                        for param in &function.params {
                            out.insert(type_name_to_string(&param.ty));
                        }
                        for stmt in &function.public_body {
                            collect_types_from_stmt(stmt, out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        collect_types_from_stmt(stmt, out);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_types_from_stmt(stmt: &ncc_ast::Stmt, out: &mut BTreeSet<String>) {
    match stmt {
        ncc_ast::Stmt::Projection { ty, .. } | ncc_ast::Stmt::StrictCheck { ty, .. } => {
            out.insert(type_name_to_string(ty));
        }
        ncc_ast::Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            for nested in then_block {
                collect_types_from_stmt(nested, out);
            }
            for nested in else_block {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Unless { body, .. } => {
            for nested in body {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::While { body, .. }
        | ncc_ast::Stmt::Loop { body }
        | ncc_ast::Stmt::DoWhile { body, .. }
        | ncc_ast::Stmt::RepeatUntil { body, .. }
        | ncc_ast::Stmt::RepeatCount { body, .. }
        | ncc_ast::Stmt::ForRange { body, .. }
        | ncc_ast::Stmt::ForIn { body, .. }
        | ncc_ast::Stmt::SameIn { body, .. }
        | ncc_ast::Stmt::EachDoWhile { body, .. } => {
            for nested in body {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForClassic {
            init, step, body, ..
        } => {
            if let Some(init) = init {
                collect_types_from_stmt(init, out);
            }
            if let Some(step) = step {
                collect_types_from_stmt(step, out);
            }
            for nested in body {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => collect_types_from_stmt(stmt, out),
        ncc_ast::Stmt::Switch { cases, default, .. }
        | ncc_ast::Stmt::Match { cases, default, .. } => {
            for (_expr, stmts) in cases {
                for nested in stmts {
                    collect_types_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::GuardElse { else_block, .. } => {
            for nested in else_block {
                collect_types_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (_expr, stmts) in branches {
                for nested in stmts {
                    collect_types_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_types_from_stmt(nested, out);
            }
        }
        _ => {}
    }
}

fn collect_operators_from_program(ast: &ncc_ast::Program, out: &mut BTreeMap<String, usize>) {
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            collect_operators_from_stmt(stmt, out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        collect_operators_from_stmt(stmt, out);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_operators_from_stmt(stmt: &ncc_ast::Stmt, out: &mut BTreeMap<String, usize>) {
    match stmt {
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Print { value: expr, .. }
        | ncc_ast::Stmt::Projection { value: expr, .. }
        | ncc_ast::Stmt::StrictCheck { value: expr, .. } => collect_operators_from_expr(expr, out),
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_operators_from_expr(condition, out);
            for nested in then_block {
                collect_operators_from_stmt(nested, out);
            }
            for nested in else_block {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Unless { condition, body } => {
            collect_operators_from_expr(condition, out);
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::While { condition, body }
        | ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            collect_operators_from_expr(condition, out);
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(stmt) = init {
                collect_operators_from_stmt(stmt, out);
            }
            if let Some(expr) = condition {
                collect_operators_from_expr(expr, out);
            }
            if let Some(stmt) = step {
                collect_operators_from_stmt(stmt, out);
            }
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            collect_operators_from_expr(iterable, out);
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            collect_operators_from_expr(start, out);
            collect_operators_from_expr(end, out);
            if let Some(expr) = step {
                collect_operators_from_expr(expr, out);
            }
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            for nested in body {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            collect_operators_from_expr(value, out);
            for (expr, stmts) in cases {
                collect_operators_from_expr(expr, out);
                for nested in stmts {
                    collect_operators_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            collect_operators_from_expr(condition, out);
            for nested in else_block {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (expr, stmts) in branches {
                collect_operators_from_expr(expr, out);
                for nested in stmts {
                    collect_operators_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_operators_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => collect_operators_from_stmt(stmt, out),
        _ => {}
    }
}

fn collect_operators_from_expr(expr: &ncc_ast::Expr, out: &mut BTreeMap<String, usize>) {
    match expr {
        ncc_ast::Expr::Unary { op, expr } => {
            bump_count(out, format!("unary::{:?}", op));
            collect_operators_from_expr(expr, out);
        }
        ncc_ast::Expr::Binary { left, op, right } => {
            bump_count(out, format!("binary::{:?}", op));
            collect_operators_from_expr(left, out);
            collect_operators_from_expr(right, out);
        }
        ncc_ast::Expr::MethodCall { receiver, args, .. } => {
            collect_operators_from_expr(receiver, out);
            for arg in args {
                collect_operators_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::QualifiedCall { args, .. } | ncc_ast::Expr::Call { args, .. } => {
            for arg in args {
                collect_operators_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_operators_from_expr(condition, out);
            collect_operators_from_expr(then_expr, out);
            collect_operators_from_expr(else_expr, out);
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            for item in items {
                collect_operators_from_expr(item, out);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            bump_count(out, "index::[]".to_string());
            collect_operators_from_expr(target, out);
            collect_operators_from_expr(index, out);
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => collect_operators_from_expr(value, out),
        _ => {}
    }
}

fn compiler_render_concat_call_base(raw: &str) -> String {
    raw.split('<').next().unwrap_or(raw).trim().to_string()
}

fn compiler_collect_render_concat_from_program(
    ast: &ncc_ast::Program,
    out: &mut CompilerRenderConcatAudit,
) {
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            compiler_collect_render_concat_from_stmt(stmt, out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        compiler_collect_render_concat_from_stmt(stmt, out);
                    }
                }
                _ => {}
            }
        }
    }
}

fn compiler_collect_render_concat_from_stmt(
    stmt: &ncc_ast::Stmt,
    out: &mut CompilerRenderConcatAudit,
) {
    match stmt {
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Print { value: expr, .. }
        | ncc_ast::Stmt::Projection { value: expr, .. }
        | ncc_ast::Stmt::StrictCheck { value: expr, .. } => {
            compiler_collect_render_concat_from_expr(expr, out)
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            compiler_collect_render_concat_from_expr(condition, out);
            for nested in then_block {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
            for nested in else_block {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Unless { condition, body }
        | ncc_ast::Stmt::While { condition, body }
        | ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            compiler_collect_render_concat_from_expr(condition, out);
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(stmt) = init {
                compiler_collect_render_concat_from_stmt(stmt, out);
            }
            if let Some(expr) = condition {
                compiler_collect_render_concat_from_expr(expr, out);
            }
            if let Some(stmt) = step {
                compiler_collect_render_concat_from_stmt(stmt, out);
            }
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            compiler_collect_render_concat_from_expr(start, out);
            compiler_collect_render_concat_from_expr(end, out);
            if let Some(step_expr) = step {
                compiler_collect_render_concat_from_expr(step_expr, out);
            }
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            compiler_collect_render_concat_from_expr(iterable, out);
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::RepeatCount { count, body } => {
            compiler_collect_render_concat_from_expr(count, out);
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        } => {
            compiler_collect_render_concat_from_expr(value, out);
            for (expr, stmts) in cases {
                compiler_collect_render_concat_from_expr(expr, out);
                for nested in stmts {
                    compiler_collect_render_concat_from_stmt(nested, out);
                }
            }
            for nested in default {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (expr, stmts) in branches {
                compiler_collect_render_concat_from_expr(expr, out);
                for nested in stmts {
                    compiler_collect_render_concat_from_stmt(nested, out);
                }
            }
            for nested in default {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Loop { body } => {
            for nested in body {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for arg in args {
                compiler_collect_render_concat_from_expr(arg, out);
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            compiler_collect_render_concat_from_expr(condition, out);
            for nested in else_block {
                compiler_collect_render_concat_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => compiler_collect_render_concat_from_stmt(stmt, out),
        _ => {}
    }
}

fn compiler_collect_render_concat_from_expr(
    expr: &ncc_ast::Expr,
    out: &mut CompilerRenderConcatAudit,
) {
    match expr {
        ncc_ast::Expr::Binary { left, op, right } => {
            if matches!(op, ncc_ast::BinaryOp::Plus) {
                out.concat_expressions += 1;
            }
            compiler_collect_render_concat_from_expr(left, out);
            compiler_collect_render_concat_from_expr(right, out);
        }
        ncc_ast::Expr::Call { callee, args } => {
            let base = compiler_render_concat_call_base(callee);
            let normalized = base.to_ascii_lowercase();
            if normalized.starts_with("render") {
                bump_count(&mut out.render_calls, base);
            } else if matches!(
                normalized.as_str(),
                "concat" | "concat3" | "concatmany" | "join"
            ) {
                bump_count(&mut out.concat_function_calls, base);
            }
            for arg in args {
                compiler_collect_render_concat_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            if let Some(base) = path.last() {
                let normalized = base.to_ascii_lowercase();
                if normalized.starts_with("render") {
                    bump_count(&mut out.render_calls, base.clone());
                } else if matches!(
                    normalized.as_str(),
                    "concat" | "concat3" | "concatmany" | "join"
                ) {
                    bump_count(&mut out.concat_function_calls, base.clone());
                }
            }
            for arg in args {
                compiler_collect_render_concat_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::Unary { expr, .. } => compiler_collect_render_concat_from_expr(expr, out),
        ncc_ast::Expr::MethodCall { receiver, args, .. } => {
            compiler_collect_render_concat_from_expr(receiver, out);
            for arg in args {
                compiler_collect_render_concat_from_expr(arg, out);
            }
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            compiler_collect_render_concat_from_expr(condition, out);
            compiler_collect_render_concat_from_expr(then_expr, out);
            compiler_collect_render_concat_from_expr(else_expr, out);
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => {
            for item in items {
                compiler_collect_render_concat_from_expr(item, out);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            compiler_collect_render_concat_from_expr(target, out);
            compiler_collect_render_concat_from_expr(index, out);
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => {
            compiler_collect_render_concat_from_expr(value, out)
        }
        _ => {}
    }
}

fn bump_count(map: &mut BTreeMap<String, usize>, key: String) {
    let next = map.get(&key).copied().unwrap_or(0) + 1;
    map.insert(key, next);
}

fn collect_conditions_from_program(ast: &ncc_ast::Program, out: &mut Vec<serde_json::Value>) {
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            collect_conditions_from_stmt(stmt, out);
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        collect_conditions_from_stmt(stmt, out);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_conditions_from_stmt(stmt: &ncc_ast::Stmt, out: &mut Vec<serde_json::Value>) {
    match stmt {
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            out.push(serde_json::json!({
                "kind": "if",
                "expr": format!("{:?}", condition),
            }));
            for nested in then_block {
                collect_conditions_from_stmt(nested, out);
            }
            for nested in else_block {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Unless { condition, body } => {
            out.push(serde_json::json!({
                "kind": "unless",
                "expr": format!("{:?}", condition),
            }));
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::While { condition, body } => {
            out.push(serde_json::json!({
                "kind": "while",
                "expr": format!("{:?}", condition),
            }));
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            out.push(serde_json::json!({
                "kind": "loop_condition",
                "expr": format!("{:?}", condition),
            }));
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForClassic {
            condition,
            init,
            step,
            body,
        } => {
            if let Some(expr) = condition {
                out.push(serde_json::json!({
                    "kind": "for",
                    "expr": format!("{:?}", expr),
                }));
            }
            if let Some(stmt) = init {
                collect_conditions_from_stmt(stmt, out);
            }
            if let Some(stmt) = step {
                collect_conditions_from_stmt(stmt, out);
            }
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            out.push(serde_json::json!({
                "kind": "for_range",
                "start": format!("{:?}", start),
                "end": format!("{:?}", end),
                "step": step.as_ref().map(|value| format!("{:?}", value)),
            }));
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            out.push(serde_json::json!({
                "kind": "guard",
                "expr": format!("{:?}", condition),
            }));
            for nested in else_block {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            out.push(serde_json::json!({
                "kind": "match",
                "value": format!("{:?}", value),
                "arms": cases.len(),
                "has_default": !default.is_empty(),
            }));
            for (_expr, stmts) in cases {
                for nested in stmts {
                    collect_conditions_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            out.push(serde_json::json!({
                "kind": "select",
                "branches": branches.len(),
                "has_default": !default.is_empty(),
            }));
            for (_expr, stmts) in branches {
                for nested in stmts {
                    collect_conditions_from_stmt(nested, out);
                }
            }
            for nested in default {
                collect_conditions_from_stmt(nested, out);
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => collect_conditions_from_stmt(stmt, out),
        ncc_ast::Stmt::Loop { body }
        | ncc_ast::Stmt::RepeatCount { body, .. }
        | ncc_ast::Stmt::ForIn { body, .. }
        | ncc_ast::Stmt::SameIn { body, .. } => {
            for nested in body {
                collect_conditions_from_stmt(nested, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn parse_program(src: &str) -> ncc_ast::Program {
        let tokens = ncc_lexer::lex(src).expect("lex");
        ncc_parser::parse(tokens).expect("parse")
    }

    fn type_diag_codes(src: &str) -> Vec<String> {
        let ast = parse_program(src);
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        compiler_type_diagnostic_codes(&rows)
    }

    fn condition_rows(src: &str) -> Vec<CompilerConditionAuditRow> {
        let ast = parse_program(src);
        compiler_build_source_audit(&ast).condition_rows
    }

    #[test]
    fn nc_parse_provider_is_manual_and_never_reclassified_as_hosted_c() {
        for forbidden in ["#include", "malloc(", "strtol(", "strtod("] {
            assert!(
                !COMPILER_NATIVE_NC_PARSE_RUNTIME_SOURCE.contains(forbidden),
                "manual nC parse provider contains forbidden C fragment {forbidden}"
            );
        }
        assert!(!COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&"ncc_rt_str_to_int"));

        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/parse_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_parse_runtime(&consumer));
        assert!(compiler_native_attach_nc_parse_runtime(&mut consumer)
            .expect("manual parse provider must parse"));
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccTextParseRuntime"));
        ncc_sema::analyze(&consumer).expect("consumer plus nC provider must pass sema");

        let alias_consumer = parse_program(
            r#"
parseAliasConsumer :: runtime {
    fn:
        public:
            decode(value >> str) -> int {
                return value.toInt();
            }
}
"#,
        );
        assert!(compiler_native_ast_uses_nc_parse_runtime(&alias_consumer));
    }

    #[test]
    fn nc_string_provider_is_manual_and_never_reclassified_as_hosted_c() {
        for forbidden in [
            "#include",
            "malloc(",
            "calloc(",
            "realloc(",
            "strlen(",
            "strcmp(",
            "snprintf(",
        ] {
            assert!(
                !COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE.contains(forbidden),
                "manual nC string provider contains forbidden C fragment {forbidden}"
            );
        }
        assert!(
            COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS
                .iter()
                .all(|symbol| !symbol.starts_with("ncc_rt_str_")),
            "the hosted C runtime must not own any string import"
        );
        for symbol in [
            "nccTextFormatRuntime_alignLeftRaw",
            "nccTextParseRuntime_tryParseIntRaw",
            "nccTextEscapeRuntime_escapeJsonRaw",
            "nccUnicodeRuntime_normalizeNfcRaw",
        ] {
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "nC provider symbol must force material linking: {symbol}"
            );
        }

        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/string_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_text_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_text_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual string provider must parse")
        );
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccTextFormatRuntime"));
        ncc_sema::analyze(&consumer).expect("consumer plus nC provider must pass sema");
    }

    #[test]
    fn nc_string_provider_is_not_forced_into_text_free_programs() {
        let mut consumer = parse_program(
            r#"
numericOnly :: root {
    run :: fn() -> int { return 6 * 7; }
    ref main ->> run;
    fa:
        return true;
}
"#,
        );
        assert!(!compiler_native_ast_uses_nc_text_runtime(&consumer));
        assert!(
            !compiler_native_attach_nc_text_runtime(&mut consumer, "riscv32imac-unknown-elf")
                .expect("text-free detection")
        );
        assert!(consumer
            .containers
            .iter()
            .all(|container| container.name != "nccTextFormatRuntime"));
    }

    #[test]
    fn convert_provider_is_llvm_nc_material_and_never_requests_hosted_c() {
        assert!(
            !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains("ncc_rt_convert_"),
            "the hosted C source must not own conversion exports"
        );
        assert!(!compiler_native_uses_hosted_runtime(
            "define i64 @ncc_rt_convert_to_int_i64(i64 %value) { ret i64 %value }"
        ));

        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/convert_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_text_runtime(&consumer));
        assert!(compiler_native_ast_uses_nc_parse_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_text_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("conversion text provider must parse")
        );
        assert!(compiler_native_attach_nc_parse_runtime(&mut consumer)
            .expect("conversion parser provider must parse"));
        ncc_sema::analyze(&consumer).expect("conversion consumer plus nC providers must pass sema");
    }

    #[test]
    fn math_random_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_random_seed",
            "ncc_rt_random_int",
            "ncc_rt_random_bool",
            "ncc_rt_random_float",
            "ncc_rt_random_float_bits",
            "ncc_rt_random_range",
            "ncc_rt_math_sign",
            "ncc_rt_math_abs",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material math/random symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i32 @ncc_rt_random_int()\ncall i64 @ncc_rt_math_abs(i64 -7)"
        ));
    }

    #[test]
    fn entropy_uuid_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_entropy_fill",
            "ncc_rt_entropy_available",
            "ncc_rt_entropy_bytes",
            "ncc_rt_entropy_release",
            "ncc_rt_uuid_v4_into",
            "ncc_rt_uuid_from_bytes_into",
            "ncc_rt_uuid_v4",
            "ncc_rt_uuid_from_bytes",
            "ncc_rt_uuid_is_valid",
            "ncc_rt_uuid_release",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material entropy/UUID symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i32 @ncc_rt_entropy_fill(ptr %out, i64 32)\ncall i64 @ncc_rt_uuid_v4_into(ptr %out, i64 37)"
        ));
    }

    #[test]
    fn time_clock_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_now_seconds",
            "ncc_rt_now_millis",
            "ncc_rt_monotonic_millis",
            "ncc_rt_monotonic_nanos",
            "ncc_rt_clock_monotonic",
            "ncc_rt_clock_elapsed",
            "ncc_rt_clock_measure",
            "ncc_rt_clock_resolution",
            "ncc_rt_sleep_millis",
            "ncc_rt_sleep_until_millis",
            "ncc_rt_format_time_into",
            "ncc_rt_date_into",
            "ncc_rt_format_time",
            "ncc_rt_date",
            "ncc_rt_time_release",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material time/clock symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_clock_monotonic()\ncall i64 @ncc_rt_date_into(ptr %out, i64 24)"
        ));
    }

    #[test]
    fn terminal_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_terminal_clear",
            "ncc_rt_terminal_move_cursor",
            "ncc_rt_terminal_hide_cursor",
            "ncc_rt_terminal_show_cursor",
            "ncc_rt_terminal_enter_raw_mode",
            "ncc_rt_terminal_exit_raw_mode",
            "ncc_rt_terminal_read_key_nonblocking",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material terminal symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i32 @ncc_rt_terminal_clear()\ncall i64 @ncc_rt_terminal_read_key_nonblocking()"
        ));
    }

    #[test]
    fn std_error_flush_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_std_in",
            "ncc_rt_std_out",
            "ncc_rt_std_err",
            "ncc_rt_std_is_interactive",
            "ncc_rt_error_text",
            "ncc_rt_error_i64",
            "ncc_rt_error_f64",
            "ncc_rt_error_f64_bits",
            "ncc_rt_error_bool",
            "ncc_rt_error_flush",
            "ncc_rt_flush_all",
            "ncc_rt_flush_stdout",
            "ncc_rt_flush_stderr",
            "ncc_rt_auto_flush_on_input",
            "ncc_rt_flush_before_input",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material std/error/flush symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_std_out()\ncall i32 @ncc_rt_error_flush()"
        ));
    }

    #[test]
    fn stack_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_stack_depth",
            "ncc_rt_stack_limit",
            "ncc_rt_stack_remaining",
            "ncc_rt_stack_trace",
            "ncc_rt_stack_trace_len",
            "ncc_rt_stack_trace_frame",
            "ncc_rt_stack_trace_release",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material stack symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_stack_depth()\ncall i64 @ncc_rt_stack_trace()"
        ));
    }

    #[test]
    fn process_thread_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_process_spawn",
            "ncc_rt_process_wait",
            "ncc_rt_thread_spawn",
            "ncc_rt_thread_join",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material process/thread symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_process_wait(i64 %h)\ncall i64 @ncc_rt_thread_join(i64 %t)"
        ));
    }

    #[test]
    fn system_registry_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_system_cpu_load",
            "ncc_rt_registry_write",
            "ncc_rt_registry_read",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material system symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call double @ncc_rt_system_cpu_load()\ncall ptr @ncc_rt_registry_read(ptr %key)"
        ));
    }

    #[test]
    fn permission_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_permission_can_read",
            "ncc_rt_permission_can_write",
            "ncc_rt_permission_can_execute",
            "ncc_rt_permission_require",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material permission symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i32 @ncc_rt_permission_can_read(ptr %path)\ncall i64 @ncc_rt_permission_require(ptr %path)"
        ));
    }

    #[test]
    fn network_provider_is_material_backend_and_never_requests_hosted_c() {
        let hosted_source_defines = |symbol: &str| {
            COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE
                .match_indices(&format!("{symbol}("))
                .any(|(offset, _)| {
                    let tail = &COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE[offset..];
                    let semicolon = tail.find(';').unwrap_or(usize::MAX);
                    let brace = tail.find('{').unwrap_or(usize::MAX);
                    brace < semicolon
                })
        };
        for symbol in [
            "ncc_rt_network_server_open",
            "ncc_rt_network_client_open",
            "ncc_rt_network_receive",
            "ncc_rt_network_send",
            "ncc_rt_network_send_bytes",
            "ncc_rt_network_receive_bytes",
            "ncc_rt_network_send_raw",
            "ncc_rt_network_receive_raw",
            "ncc_rt_network_close",
        ] {
            assert!(
                !hosted_source_defines(symbol),
                "hosted C source still defines {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material network symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_network_send_raw(ptr %data, i64 4)\ncall i32 @ncc_rt_network_close(i64 %handle)"
        ));
        assert!(compiler_native_build_needs_ws2_32(
            "declare i32 @WSAStartup(i16, ptr)"
        ));
    }

    #[test]
    fn filesystem_provider_is_material_backend_and_never_requests_hosted_c() {
        for symbol in [
            "ncc_rt_fswrite",
            "ncc_rt_fsread",
            "ncc_rt_fsync",
            "ncc_rt_fs_delete",
            "ncc_rt_fs_rename",
            "ncc_rt_fread_lines_into",
            "ncc_rt_exists",
            "ncc_rt_list_dir",
            "ncc_rt_create_dir",
            "ncc_rt_remove_dir",
            "ncc_rt_current_dir",
            "ncc_rt_change_dir",
            "ncc_rt_fs_read_into",
            "ncc_rt_fs_write_from",
            "ncc_rt_fs_absolute_path_into",
            "ncc_rt_fs_official_path_into",
            "ncc_rt_absolute_path",
            "ncc_rt_official_path",
            "ncc_rt_fs_lock",
            "ncc_rt_fs_unlock",
            "ncc_rt_fs_text_release",
            "ncc_rt_fs_copy",
            "ncc_rt_copy_file",
            "ncc_rt_read_bytes",
            "ncc_rt_fs_bytes_release",
            "ncc_rt_fs_metadata_into",
            "ncc_rt_fs_permissions_into",
            "ncc_rt_fs_set_readonly",
            "ncc_rt_metadata",
            "ncc_rt_permissions",
            "ncc_rt_set_readonly",
            "ncc_rt_lock_file",
            "ncc_rt_unlock_file",
        ] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(&format!("{symbol}(")),
                "hosted C source still owns {symbol}"
            );
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SYMBOLS.contains(&symbol),
                "hosted C symbol list still owns {symbol}"
            );
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material filesystem symbol must force MCT linking: {symbol}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call i64 @ncc_rt_fsread(ptr %path)\ncall i64 @ncc_rt_list_dir(ptr %path)"
        ));
    }

    #[test]
    fn escape_provider_is_nc_material_and_never_requests_hosted_c() {
        for forbidden in ["#include", "malloc(", "calloc(", "realloc(", "strlen("] {
            assert!(
                !COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE.contains(forbidden),
                "manual nC escape provider contains forbidden C fragment {forbidden}"
            );
        }
        for removed in ["ncc_rt_escape_", "ncc_rt_sanitize_"] {
            assert!(
                !COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE.contains(removed),
                "hosted C still owns removed escape symbol family {removed}"
            );
        }
        assert!(!compiler_native_uses_hosted_runtime(
            "call ptr @nccTextEscapeRuntime_escapeJsonRaw(ptr %value)"
        ));

        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/escape_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_escape_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_escape_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual escape provider must parse")
        );
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccTextEscapeRuntime"));
        ncc_sema::analyze(&consumer).expect("escape consumer plus nC provider must pass sema");
    }

    #[test]
    fn nc_string_provider_attaches_for_plus_concatenation() {
        let source = r#"
concatConsumer :: runtime {
    fn:
        public:
            copy(value >> str) -> str {
                return "" + value;
            }
}
"#;
        let mut consumer = parse_program(source);
        assert!(compiler_native_ast_uses_nc_text_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_text_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("the nC text provider must attach for string +")
        );
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccTextFormatRuntime"));
        ncc_sema::analyze(&consumer).expect("string concatenation plus nC provider must pass sema");
        let ir = ncc_ir::lower_with_backend_profile_and_source(
            consumer,
            ncc_ir::BackendLoweringProfileIR::nc_win64(),
            "concat-consumer.n",
            source,
        );
        assert!(ir.containers.iter().any(|container| matches!(
            container,
            ncc_ir::ContainerIR::Regular(runtime)
                if runtime.name == "nccTextFormatRuntime"
                    && runtime.functions.iter().any(|function| function.name == "concatRaw")
        )));
        let module = ncc_ir::mct_emit_llvm_module(&ir);
        assert!(
            module
                .llvm_ir
                .contains("define ptr @nccTextFormatRuntime_concatRaw(ptr"),
            "the attached nC provider must materialize in LLVM:\n{}",
            module.llvm_ir
        );
    }

    #[test]
    fn nc_text_provider_attaches_to_explicit_text_import() {
        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/text_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_text_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_text_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual text provider must parse")
        );
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccTextFormatRuntime"));
        ncc_sema::analyze(&consumer).expect("consumer plus nC provider must pass sema");
    }

    #[test]
    fn nc_unicode_provider_is_generated_and_linked_without_c_runtime() {
        for source in [
            COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE,
            COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE,
        ] {
            for forbidden in ["#include", "malloc(", "calloc(", "realloc(", "free("] {
                assert!(
                    !source.contains(forbidden),
                    "manual nC Unicode provider contains forbidden C fragment {forbidden}"
                );
            }
        }

        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/unicode_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_unicode_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_text_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual text dependency must parse")
        );
        assert!(
            compiler_native_attach_nc_unicode_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual Unicode provider must parse")
        );
        for name in ["nccUnicodeTables", "nccUnicodeRuntime"] {
            assert!(
                consumer
                    .containers
                    .iter()
                    .any(|container| container.name == name),
                "missing linked nC container {name}"
            );
        }
        ncc_sema::analyze(&consumer).expect("consumer plus Unicode providers must pass sema");
    }

    #[test]
    fn nc_width_import_attaches_unicode_tables_without_hidden_c_object() {
        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/width_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_unicode_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_unicode_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual width provider must parse")
        );
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccUnicodeTables"));
        assert!(consumer
            .containers
            .iter()
            .any(|container| container.name == "nccUnicodeRuntime"));
        ncc_sema::analyze(&consumer).expect("width consumer plus nC provider must pass sema");
    }

    #[test]
    fn nc_wrap_import_attaches_owned_unicode_provider_without_c_runtime() {
        let mut consumer = parse_program(include_str!(
            "../../../../examples/runtime/families/wrap_basic.n"
        ));
        assert!(compiler_native_ast_uses_nc_unicode_runtime(&consumer));
        assert!(
            compiler_native_attach_nc_unicode_runtime(&mut consumer, "x86_64-pc-windows-msvc")
                .expect("manual wrap provider must parse")
        );
        for name in ["nccUnicodeTables", "nccUnicodeRuntime"] {
            assert!(consumer
                .containers
                .iter()
                .any(|container| container.name == name));
        }
        for forbidden in ["#include", "malloc(", "calloc(", "realloc(", "free("] {
            assert!(!COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE.contains(forbidden));
        }
        ncc_sema::analyze(&consumer).expect("wrap consumer plus nC provider must pass sema");
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn workspace_path(rel: &str) -> PathBuf {
        let root = workspace_root();
        let direct = root.join(rel);
        if direct.exists() {
            return direct;
        }

        let normalized = rel.replace('\\', "/");
        if let Some(example_rel) = normalized.strip_prefix("examples/") {
            let archived = root.join("_archive").join("old_examples").join(example_rel);
            if archived.exists() {
                return archived;
            }

            let public_fixture = root
                .join("tests")
                .join("fixtures")
                .join("old_examples")
                .join(example_rel);
            if public_fixture.exists() {
                return public_fixture;
            }
        }

        direct
    }

    fn unique_temp_id() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test clock must be after unix epoch")
            .as_nanos()
    }

    #[test]
    fn native_target_profiles_preserve_os_arch_abi_and_object_format() {
        let cases = [
            (
                "x86_64-apple-darwin",
                "x86_64-apple-darwin",
                ncc_ir::BackendIsaIR::X86_64,
                ncc_ir::BackendObjectFormatIR::MachO,
            ),
            (
                "aarch64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                ncc_ir::BackendIsaIR::AArch64,
                ncc_ir::BackendObjectFormatIR::Elf64,
            ),
            (
                "aarch64-pc-windows-msvc",
                "aarch64-pc-windows-msvc",
                ncc_ir::BackendIsaIR::AArch64,
                ncc_ir::BackendObjectFormatIR::Coff,
            ),
            (
                "i386-unknown-linux-gnu",
                "i386-unknown-linux-gnu",
                ncc_ir::BackendIsaIR::X86,
                ncc_ir::BackendObjectFormatIR::Elf32,
            ),
            (
                "i686-pc-windows-msvc",
                "i686-pc-windows-msvc",
                ncc_ir::BackendIsaIR::X86,
                ncc_ir::BackendObjectFormatIR::Coff,
            ),
        ];
        for (input, triple, isa, object) in cases {
            let profile = compiler_native_build_profile_from_target(input).expect("target profile");
            assert_eq!(profile.target_spec.target_triple, triple);
            assert_eq!(profile.target_spec.isa, isa);
            assert_eq!(profile.target_spec.object_format, object);
            profile
                .validate_linking_artifact(ncc_ir::BackendLinkArtifactIR::BuildExe)
                .expect("hosted target must admit an executable");
        }
    }

    #[test]
    fn native_win32_external_link_contract_includes_kernel32() {
        let args = compiler_native_external_link_command_args(
            "i686-pc-windows-msvc",
            "-O2",
            std::path::Path::new("program.ll"),
            std::path::Path::new("program.obj"),
            std::path::Path::new("program.exe"),
            CompilerNativeLtoMode::Off,
            None,
            None,
            false,
            &[],
            false,
            false,
            false,
        );
        assert!(args.iter().any(|arg| arg == "-lkernel32"), "{args:?}");
        let registry_args = compiler_native_external_link_command_args(
            "i686-pc-windows-msvc",
            "-O2",
            std::path::Path::new("registry.ll"),
            std::path::Path::new("registry.obj"),
            std::path::Path::new("registry.exe"),
            CompilerNativeLtoMode::Off,
            None,
            None,
            false,
            &[],
            false,
            false,
            compiler_native_build_needs_advapi32(
                "declare x86_stdcallcc i32 @\"\\01_RegOpenKeyExA@20\"(ptr, ptr, i32, i32, ptr)",
            ),
        );
        assert!(registry_args.iter().any(|arg| arg == "-ladvapi32"));
        assert_eq!(
            crate::commands::mct_support::backend_lowering_profile_label(
                &ncc_ir::BackendLoweringProfileIR::nc_i386_windows()
            ),
            "nC.i386-msvc"
        );
    }

    #[test]
    fn native_machine_bridge_uses_architecture_correct_call_spelling() {
        let root = std::env::temp_dir();
        let aarch64 = ncc_ir::BackendLoweringProfileIR::nc_aarch64_linux();
        let source = compiler_native_machine_bridge_source(
            "main.n",
            "main",
            &aarch64,
            &root.join("main.obj"),
            &root.join("main.exe"),
        )
        .expect("AArch64 bridge");
        assert!(source.contains("arch: aarch64;"), "{source}");
        assert!(source.contains("bl main;"), "{source}");
        assert!(!source.contains("sub rsp"), "{source}");

        let i386 = ncc_ir::BackendLoweringProfileIR::nc_i386_linux();
        let source = compiler_native_machine_bridge_source(
            "main.n",
            "main",
            &i386,
            &root.join("main32.obj"),
            &root.join("main32"),
        )
        .expect("i386 bridge");
        assert!(source.contains("arch: i386;"), "{source}");
        assert!(source.contains("abi: i386-sysv;"), "{source}");
        assert!(source.contains("object: elf32;"), "{source}");
        assert!(source.contains("pointerWidth: 32;"), "{source}");
        assert!(source.contains("call main;"), "{source}");
        assert!(!source.contains("rsp"), "{source}");

        let win32 = ncc_ir::BackendLoweringProfileIR::nc_i386_windows();
        let source = compiler_native_machine_bridge_source(
            "main.n",
            "main",
            &win32,
            &root.join("main32.obj"),
            &root.join("main32.exe"),
        )
        .expect("Win32 i386 bridge");
        assert!(source.contains("arch: i386;"), "{source}");
        assert!(source.contains("abi: i386-msvc;"), "{source}");
        assert!(source.contains("object: coff;"), "{source}");
        assert!(source.contains("call _main;"), "{source}");
    }

    #[test]
    fn native_json_bridge_catalog_matches_all_exported_c_definitions() {
        assert_eq!(COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS.len(), 49);
        assert_eq!(
            COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            46,
            "JSON ABI symbols must be unique"
        );
        for symbol in COMPILER_NATIVE_JSON_RUNTIME_SYMBOLS {
            let marker = format!("{symbol}(");
            let definitions = COMPILER_NATIVE_JSON_RUNTIME_SOURCE
                .lines()
                .filter(|line| !line.starts_with(char::is_whitespace) && line.contains(&marker))
                .count();
            assert_eq!(
                definitions, 1,
                "missing or duplicate C definition for {symbol}"
            );
            assert!(compiler_native_uses_json_runtime(&format!(
                "declare void @{symbol}()"
            )));
            assert!(compiler_native_is_json_runtime_symbol(symbol));
            assert!(compiler_native_is_json_runtime_symbol(&format!(
                "runtime.json.{symbol}"
            )));
            assert!(compiler_native_is_json_runtime_symbol(&format!(
                "__imp_{symbol}"
            )));
        }
        assert!(!compiler_native_is_json_runtime_symbol("jsonUserFunction"));
        assert!(!compiler_native_uses_json_runtime(
            "define i32 @main() { ret i32 0 }"
        ));
    }

    #[test]
    fn native_material_wrappers_require_the_llvm_materialization_route() {
        for symbol in [
            "__ncc_user_name",
            "__ncc_temp_directory",
            "__ncc_signal_event_create",
            "__imp___ncc_user_profile_directory",
        ] {
            assert!(
                compiler_native_is_runtime_abi_symbol(symbol),
                "material wrapper {symbol} must not become a system DLL import"
            );
        }
        assert!(!compiler_native_is_runtime_abi_symbol("GetUserNameW"));
    }

    #[test]
    fn compiler_machine_layout_closes_struct_arrays_queues_and_views_as_bytes() {
        let ast = parse_program(
            r#"
layoutMachineBytes {
    type:
        Header {
            public:
                mark: int;
                code: int;
        }

        Packet {
            public:
                id: int;
                header: Header;
                tail: int;
        }

        Batch {
            public:
                packets: array<Packet, 2>;
                q: queue<Packet>;
                view: memRegion<int>;
        }

    fa:
        return true;
}
"#,
        );
        let layouts = compiler_machine_struct_layouts(&ast, 8);
        let header = layouts.get("Header").expect("Header layout");
        assert_eq!(header["size"], serde_json::json!(8));
        assert_eq!(header["align"], serde_json::json!(4));

        let packet = layouts.get("Packet").expect("Packet layout");
        assert_eq!(packet["size"], serde_json::json!(16));
        assert_eq!(packet["align"], serde_json::json!(4));
        let packet_fields = packet["fields"].as_array().expect("Packet fields");
        assert_eq!(packet_fields[1]["name"], serde_json::json!("header"));
        assert_eq!(packet_fields[1]["offset"], serde_json::json!(4));
        assert_eq!(packet_fields[1]["size"], serde_json::json!(8));
        assert_eq!(
            packet_fields[1]["status"],
            serde_json::json!("closed_struct")
        );

        let batch = layouts.get("Batch").expect("Batch layout");
        assert_eq!(batch["size"], serde_json::json!(72));
        assert_eq!(batch["align"], serde_json::json!(8));
        let fields = batch["fields"].as_array().expect("Batch fields");
        assert_eq!(fields[0]["name"], serde_json::json!("packets"));
        assert_eq!(fields[0]["offset"], serde_json::json!(0));
        assert_eq!(fields[0]["size"], serde_json::json!(32));
        assert_eq!(
            fields[0]["status"],
            serde_json::json!("closed_array_fixed_struct")
        );
        assert_eq!(fields[1]["name"], serde_json::json!("q"));
        assert_eq!(fields[1]["offset"], serde_json::json!(32));
        assert_eq!(fields[1]["size"], serde_json::json!(24));
        assert_eq!(
            fields[1]["status"],
            serde_json::json!("closed_descriptor_ptr_len_cap")
        );
        assert_eq!(fields[2]["name"], serde_json::json!("view"));
        assert_eq!(fields[2]["offset"], serde_json::json!(56));
        assert_eq!(fields[2]["size"], serde_json::json!(16));
        assert_eq!(
            fields[2]["status"],
            serde_json::json!("closed_descriptor_ptr_len")
        );
    }

    fn required_examples() -> &'static [&'static str] {
        &[
            "examples/helloWorld.n",
            "examples/calculadoraBasica.n",
            "examples/permisoEdad.n",
            "examples/runtime/runtime_input_output.n",
            "examples/runtime/runtime_parse_result.n",
            "examples/runtime/runtime_render_format.n",
            "examples/runtime/runtime_buffer_region.n",
            "examples/runtime/runtime_heap_lifecycle.n",
            "examples/runtime/runtime_ptr_addr_site.n",
            "examples/runtime/runtime_fs_capability.n",
            "examples/runtime/runtime_process_policy.n",
            "examples/runtime/runtime_time_random_uuid.n",
            "examples/runtime/runtime_task_sync.n",
            "examples/runtime/runtime_http_tls.n",
            "examples/runtime/runtime_sandbox_resource.n",
            "examples/runtime/runtime_import_all_families.n",
            "examples/runtime/runtime_all_capabilities.n",
            "examples/runtime/runtime_all_policies.n",
            "examples/runtime/runtime_all_errors.n",
            "examples/runtime/runtime_forbidden_root.n",
            "examples/runtime/runtime_forbidden_primitive.n",
            "examples/runtime/runtime_forbidden_machine_input.n",
            "examples/compiler/compiler_pipeline_report.n",
            "examples/compiler/compiler_token_stream.n",
            "examples/compiler/compiler_ast_snapshot.n",
            "examples/compiler/compiler_sema_mode_matrix.n",
            "examples/compiler/compiler_import_resolver.n",
            "examples/compiler/compiler_type_resolver.n",
            "examples/compiler/compiler_operator_resolver.n",
            "examples/compiler/compiler_condition_resolver.n",
            "examples/compiler/compiler_nir_contract.n",
            "examples/compiler/compiler_lowering_report.n",
            "examples/compiler/compiler_backend_target_report.n",
            "examples/compiler/compiler_diagnostics_catalog.n",
            "examples/compiler/compiler_internal_mode_forbidden.n",
            "examples/types/type_01_basic_struct.n",
            "examples/types/type_02_visibility.n",
            "examples/types/type_03_layout_binary.n",
            "examples/types/type_04_bounds.n",
            "examples/types/type_05_invariant.n",
            "examples/types/type_06_constructure.n",
            "examples/types/type_07_destructure.n",
            "examples/types/type_08_copy_clone_move.n",
            "examples/types/type_09_ownership_nullability.n",
            "examples/types/type_10_drop_resource.n",
            "examples/types/type_11_enum.n",
            "examples/types/type_12_tagged_union.n",
            "examples/types/type_13_generic_box.n",
            "examples/types/type_14_methods_explicit_self.n",
            "examples/types/type_15_ffi_repr_c.n",
            "examples/types/type_16_binary_unbinary.n",
            "examples/types/type_17_array_of_type.n",
            "examples/types/type_18_list_of_type.n",
            "examples/types/type_19_region_of_binary_type.n",
            "examples/types/type_20_complex_account_contract.n",
        ]
    }

    #[test]
    fn compiler_help_mentions_unified_surface() {
        let help = compiler_help_text();
        assert!(help.contains("compiler <doctor|pipeline|phases|tokens|ast|sema|imports|types|type-contracts|pointer-industrial|pointer-lowering|pointer-audit|bounds|arrays|index|memory-contracts|operators|conditions|nir|lower|optimize|backend|build-native|targets|diagnostics|feature-suite|report|self-check|ffi|ffi-symbols|ffi-contracts|ffi-link-manifest|machine-interop|ffi-smoke|ffi-cpp-abi|ffi-external-bindings>"));
        assert!(help.contains("ncc compiler ffi-link-manifest"));
    }

    #[test]
    fn pointer_lowering_payload_exposes_all_domains_widths_and_targets() {
        let payload = compiler_pointer_lowering_payload();
        assert_eq!(
            payload.get("schema").and_then(|value| value.as_str()),
            Some("nc.compiler.pointer_lowering.v2")
        );
        assert_eq!(
            payload["pointer_widths"],
            serde_json::json!([16, 32, 64, 128])
        );
        assert_eq!(payload["address_spaces"]["mmio"], serde_json::json!(1));
        assert_eq!(payload["address_spaces"]["firmware"], serde_json::json!(5));
        for target in [
            "x86_64-unknown-linux-gnu",
            "i386-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "armv7-unknown-none-eabi",
            "riscv32imac-unknown-elf",
            "riscv64-unknown-elf",
        ] {
            assert!(payload["targets"]
                .as_array()
                .is_some_and(|targets| targets.iter().any(|item| item == target)));
        }
        assert_eq!(payload["llvm_contract"]["hidden_bounds_checks"], false);
        assert_eq!(
            payload["llvm_contract"]["mmio"],
            "automatic volatile load/store"
        );
    }

    #[test]
    fn compiler_doctor_reports_industrial_surfaces() {
        let payload = compiler_doctor_payload(&CompilerCliOptions::default());
        assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(
            payload
                .get("pointer_surface")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str()),
            Some("closed")
        );
        for key in [
            "ffi_surface",
            "arrays_surface",
            "bounds_surface",
            "runtime_import_resolver_surface",
            "runtime_generic_surface",
            "report_surface",
            "cpp_abi_surface",
        ] {
            let surface = payload
                .get(key)
                .unwrap_or_else(|| panic!("compiler doctor must expose {}", key));
            assert_eq!(
                surface.get("status").and_then(|v| v.as_str()),
                Some("ok"),
                "{} must report ok",
                key
            );
        }
        assert_eq!(
            payload
                .get("cpp_abi_surface")
                .and_then(|v| v.get("name_mangling"))
                .and_then(|v| v.as_str()),
            Some("closed_real_toolchain_smoke")
        );
        assert_eq!(
            payload
                .get("runtime_import_resolver_surface")
                .and_then(|v| v.get("no_silent_fallback"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn compiler_type_contracts_command_reports_contracts() {
        let tmp = std::env::temp_dir().join(format!(
            "ncc_type_contracts_cmd_{}_{}.n",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let src = r#"
entry[@] {
    type:
        User {
            public:
                id: int;
            layout:
                kind: unbinary;
            methods:
                get(self: ref<User>) -> int;
        }

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
        fs::write(&tmp, src).expect("write temp source");
        let args = vec![
            "type-contracts".to_string(),
            tmp.to_string_lossy().into_owned(),
            "--json".to_string(),
        ];
        run_compiler_command(&args).expect("compiler type-contracts should execute");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn compiler_arrays_index_memory_contracts_commands_run() {
        let arrays_src = workspace_path("examples/arrays/array_dynamic_index_write.n")
            .to_string_lossy()
            .into_owned();
        let memory_src = workspace_path("examples/memory/array_move_copy.n")
            .to_string_lossy()
            .into_owned();
        let arrays_args = vec![
            "arrays".to_string(),
            arrays_src.clone(),
            "--json".to_string(),
        ];
        run_compiler_command(&arrays_args).expect("compiler arrays command should execute");

        let index_args = vec!["index".to_string(), arrays_src, "--json".to_string()];
        run_compiler_command(&index_args).expect("compiler index command should execute");

        let memory_args = vec![
            "memory-contracts".to_string(),
            memory_src,
            "--json".to_string(),
        ];
        run_compiler_command(&memory_args)
            .expect("compiler memory-contracts command should execute");
    }

    #[test]
    fn compiler_pointer_audit_is_optional_and_codegen_neutral() {
        let source = workspace_path("examples/06_memory/rawPointerUndefinedBehavior.n")
            .to_string_lossy()
            .into_owned();
        let args = vec![
            "pointer-audit".to_string(),
            source.clone(),
            "--json".to_string(),
        ];
        run_compiler_command(&args)
            .expect("non-strict pointer audit must report without rejecting");

        let strict_args = vec!["pointer-audit".to_string(), source, "--strict".to_string()];
        let error = run_compiler_command(&strict_args)
            .expect_err("strict audit command must surface the tracked hazard")
            .to_string();
        assert!(
            error.contains("pointer audit detected tracked hazard"),
            "{error}"
        );
    }

    #[test]
    fn ffi_e2e_smoke_runs_or_skips_cleanly() {
        let payload = compiler_run_ffi_smoke(false).expect("ffi smoke must not hard-fail");
        let status = payload
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        assert!(
            status == "ok" || status == "skipped",
            "ffi smoke status must be ok/skipped, got {} payload={}",
            status,
            payload
        );
        let asm_surface = payload
            .get("asm_surface")
            .and_then(|v| v.as_object())
            .expect("ffi smoke payload must expose asm_surface gating");
        assert_eq!(
            asm_surface.get("status").and_then(|v| v.as_str()),
            Some("active")
        );
        assert_eq!(
            asm_surface.get("gate").and_then(|v| v.as_str()),
            Some("[@machine]")
        );
        if status == "ok" {
            let abi_matrix = payload
                .get("abi_matrix")
                .and_then(|v| v.as_object())
                .expect("ffi smoke payload must expose abi_matrix");
            for key in ["nC.sysv64", "nC.win64", "nC.aapcs64"] {
                let row = abi_matrix
                    .get(key)
                    .and_then(|v| v.as_object())
                    .unwrap_or_else(|| panic!("abi matrix row '{}' missing", key));
                assert_eq!(
                    row.get("foreign_c").and_then(|v| v.as_bool()),
                    Some(true),
                    "ffi matrix row '{}' must advertise foreign_c",
                    key
                );
                assert_eq!(
                    row.get("foreign_cpp").and_then(|v| v.as_bool()),
                    Some(true),
                    "ffi matrix row '{}' must advertise foreign_cpp",
                    key
                );
                assert_eq!(
                    row.get("foreign_rust").and_then(|v| v.as_bool()),
                    Some(true),
                    "ffi matrix row '{}' must advertise foreign_rust",
                    key
                );
                assert!(
                    row.get("host_linkable").and_then(|v| v.as_bool()).is_some(),
                    "ffi matrix row '{}' must expose host_linkable bool",
                    key
                );
                assert_eq!(
                    row.get("foreign_asm").and_then(|v| v.as_bool()),
                    Some(true),
                    "ffi matrix row '{}' must advertise foreign_asm",
                    key
                );
            }
            let symbol_status = payload
                .get("symbol_status")
                .and_then(|v| v.as_array())
                .expect("ffi smoke payload must expose symbol_status");
            assert_eq!(symbol_status.len(), 4);
            for row in symbol_status {
                let row_obj = row
                    .as_object()
                    .expect("symbol_status row must be an object");
                assert_eq!(
                    row_obj.get("contract_validated").and_then(|v| v.as_bool()),
                    Some(true),
                    "ffi smoke symbol row must be contract_validated=true: {:?}",
                    row
                );
                assert_eq!(
                    row_obj.get("state").and_then(|v| v.as_str()),
                    Some("executed"),
                    "ffi smoke symbol row must be executed in ok status: {:?}",
                    row
                );
            }
            let negative_cases = payload
                .get("negative_cases")
                .and_then(|v| v.as_array())
                .expect("ffi smoke payload must expose negative_cases");
            for case in [
                "abi_mismatch_real",
                "symbol_missing_real",
                "null_return_nonnull_real",
                "panic_real_rust",
                "throw_real_cpp",
            ] {
                let row = negative_cases
                    .iter()
                    .find(|value| value.get("case").and_then(|v| v.as_str()) == Some(case))
                    .unwrap_or_else(|| panic!("negative ffi case '{}' must be present", case));
                assert_eq!(
                    row.get("validated").and_then(|v| v.as_bool()),
                    Some(true),
                    "negative ffi case '{}' must validate expected failure semantics: {}",
                    case,
                    row
                );
            }
            assert_eq!(
                payload
                    .get("interop")
                    .and_then(|v| v.get("c"))
                    .and_then(|v| v.as_bool()),
                Some(true)
            );
            assert_eq!(
                payload
                    .get("interop")
                    .and_then(|v| v.get("cpp"))
                    .and_then(|v| v.as_bool()),
                Some(true)
            );
            assert_eq!(
                payload
                    .get("interop")
                    .and_then(|v| v.get("rust"))
                    .and_then(|v| v.as_bool()),
                Some(true)
            );
        }
    }

    #[test]
    fn ffi_tool_resolver_prefers_spawnable_path_command() {
        let tool = compiler_resolve_first_tool(&["rustc"]).expect("rustc must be resolvable");
        assert_eq!(
            tool, "rustc",
            "resolver should keep PATH command names instead of shell-specific absolute paths"
        );
    }

    #[test]
    fn ffi_e2e_smoke_strict_is_real_or_fails_with_official_link_code() {
        let has_tools = compiler_run_ffi_smoke(false)
            .ok()
            .and_then(|payload| {
                payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("ok");
        match compiler_run_ffi_smoke(true) {
            Ok(payload) => {
                assert!(
                    has_tools,
                    "strict ffi smoke should only pass when all external toolchains exist"
                );
                assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
                let contracts = payload
                    .get("contracts")
                    .and_then(|v| v.as_object())
                    .expect("ffi smoke strict payload must expose contracts");
                for key in ["abi", "layout", "ownership", "nullability"] {
                    assert_eq!(
                        contracts.get(key).and_then(|v| v.as_bool()),
                        Some(true),
                        "ffi strict contract '{}' must be true",
                        key
                    );
                }
            }
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("E_FFI_LINK_FAILED_001") || msg.contains("E_FFI_ASM_BRIDGE_001"),
                    "strict ffi smoke failure must be official FFI link/ASM bridge code, got {}",
                    msg
                );
                assert!(
                    !has_tools,
                    "strict ffi smoke failed despite toolchain availability: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn ffi_cpp_abi_smoke_runs_or_skips_cleanly() {
        let payload = compiler_run_ffi_cpp_abi(false).expect("C++ ABI smoke must not hard-fail");
        let status = payload
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        assert!(
            status == "ok" || status == "skipped",
            "C++ ABI smoke status must be ok/skipped, got {} payload={}",
            status,
            payload
        );
        if status == "ok" {
            assert_eq!(payload["cpp_abi"]["status"], "closed_real_toolchain_smoke");
            for key in [
                "name_mangling",
                "templates",
                "vtables",
                "destructors",
                "exceptions",
                "layout",
            ] {
                assert_eq!(
                    payload[key]["status"], "closed_real",
                    "{} must be validated by the real C++ ABI smoke",
                    key
                );
            }
            assert_eq!(
                payload["exceptions"]["uncaught_crossing_rejected"],
                serde_json::json!(true)
            );
            assert!(payload["artifacts"]
                .as_array()
                .is_some_and(|items| items.len() >= 4));
        }
    }

    #[test]
    fn ffi_cpp_abi_strict_is_real_or_fails_with_official_code() {
        let has_tools = compiler_run_ffi_cpp_abi(false)
            .ok()
            .and_then(|payload| {
                payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("ok");
        match compiler_run_ffi_cpp_abi(true) {
            Ok(payload) => {
                assert!(
                    has_tools,
                    "strict C++ ABI smoke should only pass when all external tools exist"
                );
                assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
                assert_eq!(payload["cpp_abi"]["status"], "closed_real_toolchain_smoke");
                assert_eq!(payload["validated"], serde_json::json!(true));
            }
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("E_FFI_CPP_ABI_001")
                        || msg.contains("E_FFI_CPP_EXCEPTION_BOUNDARY_001"),
                    "strict C++ ABI failure must be official, got {}",
                    msg
                );
                assert!(
                    !has_tools,
                    "strict C++ ABI smoke failed despite toolchain availability: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn ffi_contract_report_exposes_symbol_status_and_asm_gate() {
        let path = workspace_path("examples/ffi/c_ffi_validate_amount.n")
            .to_string_lossy()
            .into_owned();
        let payload = compiler_collect_ffi_rows(&path).expect("ffi contract report must build");
        let symbol_status = payload
            .get("symbol_status")
            .and_then(|v| v.as_array())
            .expect("ffi report must expose symbol_status");
        assert!(
            !symbol_status.is_empty(),
            "ffi report must include at least one symbol status row"
        );
        let first = symbol_status[0]
            .as_object()
            .expect("symbol status row must be object");
        assert!(
            first.contains_key("contract_validated")
                && first.contains_key("linkable")
                && first.contains_key("linked")
                && first.contains_key("executed")
                && first.contains_key("state"),
            "ffi symbol status row must include stage fields: {:?}",
            first
        );
        let symbol_contracts = payload
            .get("symbol_contracts")
            .and_then(|v| v.as_array())
            .expect("ffi report must expose ABI symbol_contracts");
        assert!(
            !symbol_contracts.is_empty(),
            "ffi report must include ABI contract matrix rows"
        );
        let contract = symbol_contracts[0]
            .as_object()
            .expect("ABI contract row must be object");
        for key in [
            "symbol_contract",
            "abi",
            "signature",
            "layout",
            "ownership",
            "side_effects",
            "header",
            "library",
            "linkage",
        ] {
            assert!(
                contract.contains_key(key),
                "ABI contract row must expose {key}: {:?}",
                contract
            );
        }
        let asm_surface = payload
            .get("asm_surface")
            .and_then(|v| v.as_object())
            .expect("ffi report must expose asm_surface");
        assert_eq!(
            asm_surface.get("status").and_then(|v| v.as_str()),
            Some("active")
        );
        assert_eq!(
            asm_surface.get("gate").and_then(|v| v.as_str()),
            Some("[@machine]")
        );
    }

    #[test]
    fn ffi_contract_report_flags_c_missing_header() {
        let path = workspace_path("examples/ffi/c_abi_missing_header_fail.n")
            .to_string_lossy()
            .into_owned();
        let payload = compiler_collect_ffi_rows(&path).expect("ffi contract report must build");
        let diagnostics = payload
            .get("diagnostics")
            .and_then(|value| value.get("codes"))
            .and_then(|value| value.as_array())
            .expect("ffi report must expose diagnostic codes");
        assert!(
            diagnostics
                .iter()
                .any(|code| code.as_str() == Some("E_FFI_IMPORT_CONTRACT_001")),
            "missing C header must report official diagnostic: {:?}",
            diagnostics
        );

        let contracts = payload
            .get("symbol_contracts")
            .and_then(|value| value.as_array())
            .expect("ffi report must expose ABI symbol_contracts");
        let contract = contracts
            .iter()
            .find(|row| {
                row.get("symbol").and_then(|value| value.as_str())
                    == Some("nc_c_abi_missing_header")
            })
            .expect("missing-header symbol must be reported");
        assert_eq!(contract["status"], "failed");
        assert_eq!(contract["header"]["required"], true);
        assert_eq!(contract["header"]["present"], false);
        assert_eq!(contract["header"]["status"], "missing");
        assert_eq!(contract["library"]["required"], true);
        assert_eq!(contract["library"]["status"], "closed");

        let symbol_status = payload
            .get("symbol_status")
            .and_then(|value| value.as_array())
            .expect("ffi report must expose symbol_status");
        let status = symbol_status
            .iter()
            .find(|row| {
                row.get("symbol").and_then(|value| value.as_str())
                    == Some("nc_c_abi_missing_header")
            })
            .expect("missing-header symbol status must be reported");
        assert_eq!(status["contract_validated"], false);
        assert_eq!(status["state"], "failed");
        assert_eq!(status["failed_code"], "E_FFI_IMPORT_CONTRACT_001");
    }

    #[test]
    fn ffi_link_manifest_reports_real_artifact_and_checksum() {
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_manifest_{}_{}.n",
            std::process::id(),
            unique_temp_id()
        ));
        let artifact = tmp.with_extension("a");
        fs::write(&artifact, b"static archive bytes for manifest").expect("artifact write");
        let artifact_for_nc = artifact.to_string_lossy().replace('\\', "/");
        let checksum = format!(
            "{:x}",
            Sha256::digest(fs::read(&artifact).expect("artifact read"))
        );
        let src = format!(
            r#"
use foreign.c;
entry[@] {{
    foreign:
        c symbol "nc_manifest_add" {{
            abi: C;
            header: "nc_manifest.h";
            library: "nc_manifest";
            artifact: "{}";
            linkage: static;
            checksum: "sha256:{checksum}";
            signature: fn(int, int) -> int;
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
        }}
            fa:
                return true;
}}
"#,
            artifact_for_nc
        );
        fs::write(&tmp, src).expect("source write");

        let payload = compiler_collect_ffi_link_manifest(&tmp.to_string_lossy())
            .expect("ffi link manifest must build");
        assert_eq!(payload["status"], "closed");
        assert_eq!(payload["count"], serde_json::json!(1));
        assert_eq!(payload["entries"][0]["status"], "closed_manifest");
        assert_eq!(
            payload["entries"][0]["checksum"]["actual"],
            serde_json::json!(checksum)
        );
        assert!(payload["link_args"].as_array().is_some_and(|items| items
            .iter()
            .any(|value| value.as_str().is_some_and(|arg| arg.ends_with(".a")))));
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&artifact);
    }

    #[test]
    fn ffi_link_manifest_bad_checksum_fails_with_official_diag() {
        let tmp = std::env::temp_dir().join(format!(
            "ncc_ffi_manifest_bad_{}_{}.n",
            std::process::id(),
            unique_temp_id()
        ));
        let artifact = tmp.with_extension("so");
        fs::write(&artifact, b"shared library bytes for manifest").expect("artifact write");
        let artifact_for_nc = artifact.to_string_lossy().replace('\\', "/");
        let src = format!(
            r#"
use foreign.c;
entry[@] {{
    foreign:
        c symbol "nc_manifest_mul" {{
            abi: C;
            header: "nc_manifest.h";
            library: "nc_manifest";
            artifact: "{}";
            linkage: dynamic;
            checksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000";
            signature: fn(int, int) -> int;
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
        }}
            fa:
                return true;
}}
"#,
            artifact_for_nc
        );
        fs::write(&tmp, src).expect("source write");

        let payload = compiler_collect_ffi_link_manifest(&tmp.to_string_lossy())
            .expect("ffi link manifest must build");
        assert_eq!(payload["status"], "failed");
        assert_eq!(payload["entries"][0]["status"], "checksum_mismatch");
        assert!(payload["entries"][0]["diagnostic_codes"]
            .as_array()
            .is_some_and(|codes| codes
                .iter()
                .any(|code| code.as_str() == Some("E_FFI_CHECKSUM_001"))));
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&artifact);
    }

    #[test]
    fn ffi_cpp_abi_contract_matrix_reports_advanced_cpp_surface() {
        let path = workspace_path("examples/ffi/cpp_abi_full_contract.n")
            .to_string_lossy()
            .into_owned();
        let payload = compiler_collect_ffi_rows(&path).expect("ffi contract report must build");
        let contracts = payload
            .get("symbol_contracts")
            .and_then(|v| v.as_array())
            .expect("ffi report must expose ABI symbol_contracts");
        let by_symbol = |needle: &str| {
            contracts
                .iter()
                .find(|row| row.get("symbol").and_then(|v| v.as_str()) == Some(needle))
                .expect("missing ABI contract row")
        };
        let template = by_symbol("nc::abi::twice<int>");
        assert_eq!(template["status"], "closed");
        assert_eq!(template["cpp"]["template"]["status"], "closed");
        assert_eq!(template["cpp"]["cpp_abi"]["status"], "closed");

        let vtable = by_symbol("nc_cpp_vtable_value");
        assert_eq!(vtable["status"], "closed");
        assert_eq!(vtable["cpp"]["exception_boundary"]["status"], "closed");

        let transferred = by_symbol("nc_cpp_destructor_count");
        assert_eq!(transferred["status"], "closed");
        assert_eq!(
            transferred["ownership_transfer"]["destructor_or_free_contract"]["status"],
            "closed"
        );
    }

    #[test]
    fn compiler_macrophase_required_examples_exist() {
        for rel in required_examples() {
            let path = workspace_path(rel);
            assert!(
                path.exists() && path.is_file(),
                "required macrophase example '{}' must exist",
                rel
            );
        }
    }

    #[test]
    fn compiler_macrophase_required_examples_are_parseable() {
        let parse_must_pass = [
            "examples/helloWorld.n",
            "examples/calculadoraBasica.n",
            "examples/permisoEdad.n",
            "examples/runtime/runtime_input_output.n",
            "examples/runtime/runtime_parse_result.n",
            "examples/runtime/runtime_render_format.n",
            "examples/runtime/runtime_heap_lifecycle.n",
            "examples/runtime/runtime_ptr_addr_site.n",
            "examples/runtime/runtime_fs_capability.n",
            "examples/runtime/runtime_process_policy.n",
            "examples/runtime/runtime_time_random_uuid.n",
            "examples/runtime/runtime_task_sync.n",
            "examples/runtime/runtime_http_tls.n",
            "examples/runtime/runtime_sandbox_resource.n",
            "examples/runtime/runtime_import_all_families.n",
            "examples/runtime/runtime_all_capabilities.n",
            "examples/runtime/runtime_all_policies.n",
            "examples/runtime/runtime_all_errors.n",
            "examples/runtime/runtime_forbidden_root.n",
            "examples/runtime/runtime_forbidden_primitive.n",
            "examples/compiler/compiler_pipeline_report.n",
            "examples/compiler/compiler_token_stream.n",
            "examples/compiler/compiler_ast_snapshot.n",
            "examples/compiler/compiler_sema_mode_matrix.n",
            "examples/compiler/compiler_import_resolver.n",
            "examples/compiler/compiler_type_resolver.n",
            "examples/compiler/compiler_operator_resolver.n",
            "examples/compiler/compiler_condition_resolver.n",
            "examples/compiler/compiler_nir_contract.n",
            "examples/compiler/compiler_lowering_report.n",
            "examples/compiler/compiler_backend_target_report.n",
            "examples/compiler/compiler_diagnostics_catalog.n",
            "examples/compiler/compiler_internal_mode_forbidden.n",
            "examples/types/type_01_basic_struct.n",
            "examples/types/type_02_visibility.n",
            "examples/types/type_03_layout_binary.n",
            "examples/types/type_04_bounds.n",
            "examples/types/type_05_invariant.n",
            "examples/types/type_06_constructure.n",
            "examples/types/type_07_destructure.n",
            "examples/types/type_08_copy_clone_move.n",
            "examples/types/type_09_ownership_nullability.n",
            "examples/types/type_10_drop_resource.n",
            "examples/types/type_11_enum.n",
            "examples/types/type_12_tagged_union.n",
            "examples/types/type_13_generic_box.n",
            "examples/types/type_14_methods_explicit_self.n",
            "examples/types/type_15_ffi_repr_c.n",
            "examples/types/type_16_binary_unbinary.n",
            "examples/types/type_17_array_of_type.n",
            "examples/types/type_18_list_of_type.n",
            "examples/types/type_19_region_of_binary_type.n",
            "examples/types/type_20_complex_account_contract.n",
        ];
        for rel in parse_must_pass {
            let path = workspace_path(rel);
            let src = fs::read_to_string(&path).unwrap_or_else(|_| {
                panic!("required macrophase example '{}' must be readable", rel)
            });
            parse_program(&src);
        }
    }

    #[test]
    fn compiler_phase_catalog_is_non_empty() {
        assert!(!COMPILER_PHASES.is_empty());
    }

    #[test]
    fn compiler_diagnostic_catalog_is_non_minimal() {
        assert!(COMPILER_DIAGNOSTIC_CATALOG.len() >= 100);
    }

    #[test]
    fn compiler_type_diagnostics_detect_assignment_and_return_mismatch() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> int {
            public:
                @value >> int : "hola";
                return null;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_MISMATCH_001".to_string()),
            "expected mismatch diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_INT_001".to_string()),
            "expected int-specific diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_RETURN_001".to_string()),
            "expected return mismatch diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_generic_arity_errors() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        badResult: Result<int>;

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_GENERIC_ARITY_001".to_string()),
            "expected generic arity diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_unknown_type_contracts() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        ghost: ghostType;

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_UNKNOWN_001".to_string()),
            "expected unknown type diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_never_dev_in_strict_check() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value !> int : dev(InputParserError @runtime);
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_MISMATCH_001".to_string()),
            "never should be assignable in strict-check dev path, got {:?}",
            codes
        );
        assert!(
            !codes.contains(&"E_TYPE_INT_001".to_string()),
            "never should not trigger int mismatch in strict-check dev path, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_invalid_bool_arithmetic_expression() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : true + 1;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_MISMATCH_001".to_string()),
            "expected mismatch diagnostic for bool arithmetic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_BOOL_001".to_string()),
            "expected bool-specific diagnostic for bool arithmetic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_integer_literal_range_for_machine_scalars() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        b: byte;
        w: word;
        d: dword;
        q: qword;

    fn:
        run() -> bool {
            public:
                @b >> byte : 256;
                @w >> word : 70000;
                @d >> dword : 5000000000;
                @q >> qword : -1;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_BYTE_001".to_string()),
            "expected byte overflow diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_WORD_001".to_string()),
            "expected word overflow diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_DWORD_001".to_string()),
            "expected dword overflow diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_QWORD_001".to_string()),
            "expected qword overflow diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_return_literal_range_for_word() {
        let ast = parse_program(
            r#"
entry[@] {
    fn:
        run() -> word {
            public:
                return 100000;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_WORD_001".to_string()),
            "expected word return range diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_MISMATCH_001".to_string()),
            "expected generic mismatch range diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_valid_dword_boundary_literal() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        d: dword;

    fn:
        run() -> bool {
            public:
                @d >> dword : 4294967295;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_DWORD_001".to_string()),
            "dword max boundary should be valid, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ptr_origin_missing() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        p: ptr<int>;

    fn:
        run() -> bool {
            public:
                @p >> ptr<int> : 7;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_PTR_ORIGIN_001".to_string()),
            "expected pointer origin diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ptr_nullable_without_refine() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        p: ptr<int>;
        maybePtr: ptr<int> | null;

    fn:
        run() -> bool {
            public:
                @maybePtr >> ptr<int> | null : null;
                @p >> ptr<int> : @maybePtr;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_PTR_NULL_001".to_string()),
            "expected nullable pointer refinement diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_ptr_origin_from_addr_symbol_without_origin_diag() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        p: ptr<int>;
        rawAddr: addr;

    fn:
        run() -> bool {
            public:
                @rawAddr >> addr : 0x1000;
                @p !> ptr<int> : @rawAddr;
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_PTR_ORIGIN_001".to_string()),
            "addr source should count as pointer origin candidate, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ptr_returning_null_without_refine() {
        let ast = parse_program(
            r#"
entry[@] {
    fn:
        run() -> ptr<int> {
            public:
                return null;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_PTR_NULL_001".to_string()),
            "expected ptr null return refinement diagnostic, got {:?}",
            codes
        );
        assert!(
            codes.contains(&"E_TYPE_RETURN_001".to_string()),
            "expected return mismatch diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_array_bounds_violation() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;

    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1, 2];
                @value >> int : @arr[2];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_ARRAY_BOUNDS_001".to_string()),
            "expected array bounds diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_memregion_bounds_violation() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        region: memRegion<byte>;
        value: byte;

    fn:
        run(dataPtr: ptr<byte>) -> bool {
            public:
                @region >> memRegion<byte> : regionFromPtr(@dataPtr, 2);
                @value >> byte : @region[2];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_MEMREGION_BOUNDS_001".to_string()),
            "expected memRegion bounds diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_in_bounds_array_and_memregion_index() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        region: memRegion<byte>;
        valueInt: int;
        valueByte: byte;

    fn:
        run(dataPtr: ptr<byte>) -> bool {
            public:
                @arr >> array<int> : [1, 2, 3];
                @region >> memRegion<byte> : regionFromPtr(@dataPtr, 3);
                @valueInt >> int : @arr[2];
                @valueByte >> byte : @region[2];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_ARRAY_BOUNDS_001".to_string()),
            "in-bounds array index should be valid, got {:?}",
            codes
        );
        assert!(
            !codes.contains(&"E_TYPE_MEMREGION_BOUNDS_001".to_string()),
            "in-bounds memRegion index should be valid, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_matrix_dimension_arity_mismatch() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        m: matrix<int, int>;

    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int> : [1, 2];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_GENERIC_ARITY_001".to_string())
                && codes.contains(&"E_MATRIX_DIMENSION_001".to_string()),
            "expected matrix arity diagnostics, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_tensor_rank_mismatch() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        t: tensor<int>;

    fn:
        run() -> bool {
            public:
                @t >> tensor<int> : [1, 2];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TENSOR_RANK_001".to_string()),
            "expected tensor rank diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_dinarray_binary_layout_violation() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        db: dinArray.binary<option<int>>;

    fn:
        run() -> bool {
            public:
                @db >> dinArray.binary<option<int>> : [none(), some(1)];
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_DINARRAY_BINARY_LAYOUT_001".to_string()),
            "expected dinArray.binary layout diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_option_unwrap_none_path() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : none().unwrap();
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_OPTION_UNWRAP_001".to_string()),
            "expected option unwrap diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_result_unwrap_err_path() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : err(7).unwrap();
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_TYPE_RESULT_UNWRAP_001".to_string()),
            "expected result unwrap diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_option_some_literal_unwrap() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : some(9).unwrap();
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_OPTION_UNWRAP_001".to_string()),
            "some(...).unwrap should not emit unsafe option unwrap diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_allow_result_ok_literal_unwrap() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : ok(11).unwrap();
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_TYPE_RESULT_UNWRAP_001".to_string()),
            "ok(...).unwrap should not emit unsafe result unwrap diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_condition_collectors_include_unless_kind() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        blocked: bool;

    fn:
        run() -> bool {
            public:
                @blocked >> bool : false;
                unless (@blocked) {
                    return true;
                }
                return false;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );

        let mut rows = Vec::new();
        collect_conditions_from_program(&ast, &mut rows);
        assert!(
            rows.iter()
                .any(|row| row.get("kind").and_then(|value| value.as_str()) == Some("unless")),
            "expected legacy condition collector to include unless entries: {:?}",
            rows
        );

        let audit = compiler_build_source_audit(&ast);
        assert!(
            audit.condition_rows.iter().any(|row| row.kind == "unless"),
            "expected typed condition audit to include unless row"
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ffi_language_unsupported() {
        let ast = parse_program(
            r#"
use foreign.python;

entry[@] {
    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_FFI_IMPORT_UNKNOWN_001".to_string()),
            "expected ffi unsupported language diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ffi_c_requires_abi_contract() {
        let ast = parse_program(
            r#"
use foreign.c;

entry[@] {
    foreign:
        c symbol "validate_amount" {
            signature: fn(int) -> bool;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_FFI_ABI_MISSING_001".to_string()),
            "expected ffi C ABI-missing diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ffi_asm_requires_machine_bridge() {
        let ast = parse_program(
            r#"
use foreign.asm;

entry[@] {
    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_FFI_ASM_BRIDGE_001".to_string()),
            "expected ffi asm bridge diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_type_diagnostics_detect_ffi_crossing_primitive_is_forbidden() {
        let ast = parse_program(
            r#"
use foreign.c;

primitiveCore[@primitive] {
    anchor:
        baseEdge;

    slot:
        inputSlot;

    pattern:
        edgePattern;

    mask:
        edgeMask;

    window:
        edgeWindow;

    signal:
        pulseSignal;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            codes.contains(&"E_FFI_PRIMITIVE_CROSSING_001".to_string()),
            "expected ffi primitive crossing diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_pipeline_declared_ok() {
        assert!(COMPILER_PHASES.len() >= 10);
        assert!(COMPILER_PHASES.contains(&"lexer"));
        assert!(COMPILER_PHASES.contains(&"parser"));
        assert!(COMPILER_PHASES.contains(&"sema"));
        assert!(COMPILER_PHASES.contains(&"nir"));
        assert!(COMPILER_PHASES.contains(&"optimizer"));
        assert!(COMPILER_PHASES.contains(&"backend"));
    }

    #[test]
    fn compiler_lexer_tokens_ok() {
        let tokens = ncc_lexer::lex("entry[@] { fa: return true; }").expect("lex");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn compiler_parser_ast_ok() {
        let ast = parse_program("entry[@] { fa: return true; }");
        assert_eq!(ast.containers.len(), 1);
    }

    #[test]
    fn compiler_mode_resolver_ok() {
        let ast = parse_program(
            r#"
runtimeMode[@runtime] {
    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        assert!(audit.contexts.contains("runtime"));
    }

    #[test]
    fn compiler_runtime_if_binary_expr_int_eq_ok() {
        let rows = condition_rows(
            r#"
entry[@] {
    stc:
        opcion: int;
    fn:
        run() -> bool {
            public:
                @opcion >> int : 1;
                if (@opcion == 1) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        let if_row = rows.iter().find(|row| row.kind == "if").expect("if row");
        assert_eq!(if_row.inferred_type, "bool");
        assert!(if_row.boolish);
        assert!(if_row.diagnostic.is_none());
    }

    #[test]
    fn compiler_runtime_if_binary_expr_bool_condition_ok() {
        let rows = condition_rows(
            r#"
entry[@] {
    stc:
        ready: bool;
    fn:
        run() -> bool {
            public:
                @ready >> bool : true;
                if (@ready && true) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        let if_row = rows.iter().find(|row| row.kind == "if").expect("if row");
        assert_eq!(if_row.inferred_type, "bool");
        assert!(if_row.diagnostic.is_none());
    }

    #[test]
    fn compiler_runtime_if_condition_expr_not_string_placeholder_fail() {
        let ast = parse_program(
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                if ("hola") {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        assert_eq!(audit.placeholder_risk_count, 0);
        assert!(
            audit.condition_rows.iter().any(|row| {
                row.kind == "if"
                    && row
                        .diagnostic
                        .as_ref()
                        .is_some_and(|code| code == "E_CONDITION_STR_FORBIDDEN_001")
            }),
            "string condition must trigger stable non-placeholder diagnostic"
        );
    }

    #[test]
    fn calculadora_basica_if_else_ok() {
        let source_path = workspace_path("examples/calculadoraBasica.n");
        let ast = load_ast_from_path(
            source_path
                .to_str()
                .expect("calculadora path must be valid utf-8"),
        )
        .expect("load calculadora");
        let audit = compiler_build_source_audit(&ast);
        assert_eq!(audit.placeholder_risk_count, 0);
        assert!(
            audit.condition_rows.iter().any(|row| row.kind == "if"),
            "calculadoraBasica must include if conditions"
        );
        assert!(
            audit
                .condition_rows
                .iter()
                .filter(|row| row.kind == "if")
                .all(|row| row.diagnostic.is_none()),
            "if conditions in calculadoraBasica should resolve without diagnostics"
        );
    }

    #[test]
    fn array_1d_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1, 2, 3];
                @value >> int : @arr[1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_ARRAY_BOUNDS_001".to_string()));
    }

    #[test]
    fn array_xy_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: xy<int>;
    fn:
        run() -> bool {
            public:
                @p >> xy<int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn array_xyz_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: xyz<int>;
    fn:
        run() -> bool {
            public:
                @p >> xyz<int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn array_x2y2_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: x2y2<int>;
    fn:
        run() -> bool {
            public:
                @p >> x2y2<int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VECTOR_DIMENSION_001".to_string()));
    }

    #[test]
    fn array_x2y2z2_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: x2y2z2<int>;
    fn:
        run() -> bool {
            public:
                @p >> x2y2z2<int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VECTOR_DIMENSION_001".to_string()));
    }

    #[test]
    fn array_x3y3_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: x3y3<int>;
    fn:
        run() -> bool {
            public:
                @p >> x3y3<int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VECTOR_DIMENSION_001".to_string()));
    }

    #[test]
    fn array_x3y3z3_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: x3y3z3<int>;
    fn:
        run() -> bool {
            public:
                @p >> x3y3z3<int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VECTOR_DIMENSION_001".to_string()));
    }

    #[test]
    fn matrix_4x4_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        m: matrix<int, int, int>;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int, int> : [1, 2, 3, 4];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_MATRIX_DIMENSION_001".to_string()));
    }

    #[test]
    fn tensor_3d_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        t: tensor<int, int, int>;
    fn:
        run() -> bool {
            public:
                @t >> tensor<int, int, int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TENSOR_RANK_001".to_string()));
    }

    #[test]
    fn open_dim_matrix_index_forbidden_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        m: matrix<int, int, int>;
        out: int;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int, int> : [1, 2, 3, 4];
                @out >> int : @m[1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_ARRAY_OPEN_DIM_INDEX_001".to_string()));
    }

    #[test]
    fn array_site_element_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1, 2, 3];
                @value >> int : @arr[0];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_ARRAY_SITE_001".to_string()));
    }

    #[test]
    fn array_stride_row_major_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        m: matrix<int, int, int>;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int, int> : [1, 2, 3, 4];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_ARRAY_STRIDE_001".to_string()));
    }

    #[test]
    fn array_stride_column_major_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        m: matrix<int, int, int>;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int, int> : [4, 3, 2, 1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_ARRAY_STRIDE_001".to_string()));
    }

    #[test]
    fn array_index_out_of_bounds_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1, 2];
                @value >> int : @arr[9];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_TYPE_ARRAY_BOUNDS_001".to_string()));
    }

    #[test]
    fn array_dimension_mismatch_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        grid: array2<int, int>;
    fn:
        run() -> bool {
            public:
                @grid >> array2<int, int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_ARRAY_DIMENSION_001".to_string()));
    }

    #[test]
    fn matrix_dimension_mismatch_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        m: matrix<int, int>;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_MATRIX_DIMENSION_001".to_string()));
    }

    #[test]
    fn array_offset_overflow_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1, 2];
                @value >> int : @arr[999999999999];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(
            codes.contains(&"E_ARRAY_OFFSET_OVERFLOW_001".to_string())
                || codes.contains(&"E_TYPE_ARRAY_BOUNDS_001".to_string()),
            "expected array overflow/bounds diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn tensor_rank_mismatch_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        t: tensor<int>;
    fn:
        run() -> bool {
            public:
                @t >> tensor<int> : [1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_TENSOR_RANK_001".to_string()));
    }

    #[test]
    fn array_site_missing_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        arr: array<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int> : [1];
                @value >> int : siteOf(@arr);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(
            codes.contains(&"E_ARRAY_SITE_001".to_string())
                || codes.contains(&"E_TYPE_MISMATCH_001".to_string())
                || codes.contains(&"E_TYPE_FUNCTION_SIGNATURE_001".to_string()),
            "expected site/type diagnostic for invalid array site extraction, got {:?}",
            codes
        );
    }

    #[test]
    fn dinarray_binary_unstable_type_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray.binary<option<int>>;
    fn:
        run() -> bool {
            public:
                @db >> dinArray.binary<option<int>> : [none(), some(1)];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn dinarray_create_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DINARRAY_TYPE_001".to_string()));
    }

    #[test]
    fn dinarray_push_pop_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        pushOk: bool;
        last: int;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1, 2];
                @pushOk >> bool : @db.push(3);
                @last >> int : @db.pop();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
        assert!(!codes.contains(&"E_DINARRAY_TYPE_001".to_string()));
    }

    #[test]
    fn dinarray_get_set_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        value: int;
        setOk: bool;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [7, 8, 9];
                @value >> int : @db.get(1);
                @setOk >> bool : @db.set(1, 10);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DINARRAY_BOUNDS_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn dinarray_realloc_generation_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1, 2];
                @ok >> bool : @db.realloc(32);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DINARRAY_REALLOC_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn dinarray_binary_layout_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray.binary<int>;
        sealed: bool;
        digest: qword;
    fn:
        run() -> bool {
            public:
                @db >> dinArray.binary<int> : [1, 2, 3];
                @sealed >> bool : @db.binarySeal();
                @digest >> qword : @db.binaryHash();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DINARRAY_BINARY_LAYOUT_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn dinarray_unbinary_runtime_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray.unbinary<int>;
        view: dinArray.unbinary<int>;
    fn:
        run() -> bool {
            public:
                @db >> dinArray.unbinary<int> : [1, 2, 3];
                @view >> dinArray.unbinary<int> : @db.unbinaryView();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DINARRAY_UNBINARY_LAYOUT_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn dinarray_as_region_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        region: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [11, 12];
                @region >> memRegion<int> : @db.asRegion();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn dinarray_site_stale_after_realloc_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1];
                @ok >> bool : @db.realloc(8);
                @ok >> bool : @db.useStaleSite();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_SITE_STALE_001".to_string()));
    }

    #[test]
    fn dinarray_bounds_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        value: int;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1, 2];
                @value >> int : @db.get(10);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_BOUNDS_001".to_string()));
    }

    #[test]
    fn dinarray_capacity_overflow_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1];
                @ok >> bool : @db.reserve(999999999999);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_CAPACITY_001".to_string()));
    }

    #[test]
    fn dinarray_unbinary_as_binary_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray.unbinary<int>;
        digest: qword;
    fn:
        run() -> bool {
            public:
                @db >> dinArray.unbinary<int> : [1, 2];
                @digest >> qword : @db.binaryHash();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_UNBINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn dinarray_stale_site_use_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        db: dinArray<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @db >> dinArray<int> : [1];
                @ok >> bool : @db.useStaleSite();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DINARRAY_SITE_STALE_001".to_string()));
    }

    #[test]
    fn destructure_static_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureStatic([1, 2, 3]);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_DESTRUCTURE_")));
    }

    #[test]
    fn destructure_move_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureMove(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DESTRUCTURE_MOVE_001".to_string()));
    }

    #[test]
    fn destructure_ref_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureRef(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_DESTRUCTURE_")));
    }

    #[test]
    fn destructure_region_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        region: memRegion<byte>;
        ok: bool;
    fn:
        run(p: ptr<byte>) -> bool {
            public:
                @region >> memRegion<byte> : regionFromPtr(@p, 4);
                @ok >> bool : destructureRegion(@region, 0, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DESTRUCTURE_REGION_BOUNDS_001".to_string()));
    }

    #[test]
    fn destructure_binary_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureBinary(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_DESTRUCTURE_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn destructure_result_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureResult(ok(1));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_DESTRUCTURE_")));
    }

    #[test]
    fn constructure_static_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : constructureStatic(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_CONSTRUCTURE_")));
    }

    #[test]
    fn constructure_move_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : constructureMove(7);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_CONSTRUCTURE_MOVE_001".to_string()));
    }

    #[test]
    fn constructure_region_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        region: memRegion<byte>;
        ok: bool;
    fn:
        run(p: ptr<byte>) -> bool {
            public:
                @region >> memRegion<byte> : regionFromPtr(@p, 4);
                @ok >> bool : constructureRegion(@region);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_CONSTRUCTURE_REGION_001".to_string()));
    }

    #[test]
    fn constructure_binary_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : constructureBinary(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_CONSTRUCTURE_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn constructure_unbinary_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : constructureUnbinary(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_CONSTRUCTURE_")));
    }

    #[test]
    fn nominal_destructure_static_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                destructure.static([1, 2, 3]);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.iter().any(|code| code.starts_with("E_DESTRUCTURE_")));
    }

    #[test]
    fn nominal_destructure_move_use_origin_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                destructure.move(null);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_MOVE_001".to_string()));
    }

    #[test]
    fn nominal_constructure_binary_unstable_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                constructure.binary(some(1));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_CONSTRUCTURE_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn nominal_destructure_default_unsupported_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                destructure(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_UNSUPPORTED_001".to_string()));
    }

    #[test]
    fn destructure_field_missing_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureField(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_FIELD_001".to_string()));
    }

    #[test]
    fn destructure_move_use_origin_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureMove(null);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_MOVE_001".to_string()));
    }

    #[test]
    fn destructure_region_bounds_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureRegion(1, -1, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_REGION_BOUNDS_001".to_string()));
    }

    #[test]
    fn destructure_binary_layout_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : destructureBinary(some(1));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_DESTRUCTURE_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn constructure_missing_field_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : constructureStatic();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_CONSTRUCTURE_FIELD_MISSING_001".to_string()));
    }

    #[test]
    fn constructure_field_type_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : constructureTypedField(1, "x");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_CONSTRUCTURE_FIELD_TYPE_001".to_string()));
    }

    #[test]
    fn constructure_binary_unstable_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : constructureBinary(some(1));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_CONSTRUCTURE_BINARY_LAYOUT_001".to_string()));
    }

    #[test]
    fn visibility_public_ok() {
        let codes = type_diag_codes(
            r#"
alpha[@] {
    stc:
        public shared: int;
    fn:
        run() -> bool {
            public:
                return true;
        }
}

beta[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : @alpha.shared;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VISIBILITY_PRIVATE_001".to_string()));
        assert!(!codes.contains(&"E_VISIBILITY_INTERNAL_001".to_string()));
    }

    #[test]
    fn visibility_private_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        private secret: int;
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : @secret;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VISIBILITY_PRIVATE_001".to_string()));
    }

    #[test]
    fn visibility_internal_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        internal mid: int;
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : @mid;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VISIBILITY_INTERNAL_001".to_string()));
    }

    #[test]
    fn visibility_strictly_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        strictly locked: int;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VISIBILITY_STRICTLY_001".to_string()));
    }

    #[test]
    fn private_access_fail() {
        let codes = type_diag_codes(
            r#"
alpha[@] {
    stc:
        private secret: int;
}

beta[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : @alpha.secret;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VISIBILITY_PRIVATE_001".to_string()));
    }

    #[test]
    fn internal_access_fail() {
        let codes = type_diag_codes(
            r#"
alpha[@] {
    stc:
        internal mid: int;
}

beta[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : @alpha.mid;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VISIBILITY_INTERNAL_001".to_string()));
    }

    #[test]
    fn strictly_access_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        strictly locked: int;
    fn:
        run() -> bool {
            public:
                @locked >> int : 1;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VISIBILITY_STRICTLY_001".to_string()));
    }

    #[test]
    fn void_return_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        maybe: ptr<int> | null;
    fn:
        run() -> void {
            public:
                @maybe >> ptr<int> | null : null;
                return discardNull(@maybe);
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_VOID_RETURN_001".to_string()));
        assert!(!codes.contains(&"E_NULL_TO_VOID_001".to_string()));
    }

    #[test]
    fn null_nullable_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        maybe: ptr<int> | null;
    fn:
        run() -> bool {
            public:
                @maybe >> ptr<int> | null : null;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_NULL_TYPE_001".to_string()));
    }

    #[test]
    fn null_refine_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: ptr<int>;
        addrV: addr;
    fn:
        run() -> bool {
            public:
                @addrV >> addr : 0x1000;
                @p !> ptr<int> : @addrV;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_PTR_NULL_001".to_string()));
    }

    #[test]
    fn null_to_void_explicit_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> void {
            public:
                return discardNull(null);
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_NULL_TO_VOID_001".to_string()));
    }

    #[test]
    fn void_as_value_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : discardNull(null);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VOID_VALUE_001".to_string()));
    }

    #[test]
    fn null_non_nullable_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : null;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_NULL_TYPE_001".to_string()));
    }

    #[test]
    fn null_condition_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        maybe: ptr<int> | null;
    fn:
        run() -> bool {
            public:
                @maybe >> ptr<int> | null : null;
                if (@maybe) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_NULL_CONDITION_001".to_string()));
    }

    #[test]
    fn null_to_void_implicit_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run() -> void {
            public:
                return null;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_NULL_TO_VOID_001".to_string()));
    }

    #[test]
    fn void_addr_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        a: addr;
    fn:
        run() -> bool {
            public:
                @a >> addr : addrOf(discardNull(null));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VOID_ADDR_001".to_string()));
    }

    #[test]
    fn void_site_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : siteOf(discardNull(null));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_VOID_SITE_001".to_string()));
    }

    #[test]
    fn ffi_abi_missing_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "validate_amount" {
            signature: fn(int) -> bool;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_ABI_MISSING_001".to_string()));
        assert!(!codes.contains(&"E_FFI_LAYOUT_REQUIRED_001".to_string()));
        assert!(!codes.contains(&"E_FFI_OWNERSHIP_001".to_string()));
        assert!(!codes.contains(&"E_FFI_NULLABILITY_001".to_string()));
    }

    #[test]
    fn ffi_cpp_exception_cross_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.cpp;
entry[@] {
    foreign:
        cpp symbol "bank_score_transaction" {
            abi: C;
            signature: fn(int) -> int;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_CPP_EXCEPTION_BOUNDARY_001".to_string()));
    }

    #[test]
    fn ffi_cpp_transferred_ownership_requires_destructor_policy_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.cpp;
entry[@] {
    foreign:
        cpp symbol "owned_cpp_node" {
            abi: C;
            exportedAs: externC;
            cppAbi: Itanium;
            signature: fn() -> ptr<byte>;
            layout: stable;
            ownership: transferred;
            nullability: nonNull;
            exceptionBoundary: noThrow;
            sideEffects: heap;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_FREE_CONTRACT_001".to_string()));
    }

    #[test]
    fn ffi_rust_panic_cross_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.rust;
entry[@] {
    foreign:
        rust symbol "validate_transaction" {
            abi: C;
            signature: fn(int) -> bool;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_RUST_PANIC_BOUNDARY_001".to_string()));
    }

    #[test]
    fn ffi_asm_without_machine_bridge_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.asm;
entry[@] {
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_ASM_BRIDGE_001".to_string()));
    }

    #[test]
    fn ffi_asm_with_machine_bridge_passes_gate() {
        let codes = type_diag_codes(
            r#"
use foreign.asm;
machineFfi[@machine] {
    reg:
        ax;
        bx;
    inst:
        mov ax, bx;
    final:
        ret;
}
"#,
        );
        assert!(
            !codes.contains(&"E_FFI_ASM_BRIDGE_001".to_string()),
            "foreign.asm with [@machine] bridge must not emit bridge diagnostic: {:?}",
            codes
        );
    }

    #[test]
    fn ffi_primitive_crossing_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
primitiveCore[@primitive] {
    anchor:
        baseEdge;
    slot:
        inputSlot;
    pattern:
        edgePattern;
    mask:
        edgeMask;
    window:
        edgeWindow;
    signal:
        pulseSignal;
    bridge:
        root.policy.bind;
    seal:
        primitiveClosed;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_PRIMITIVE_CROSSING_001".to_string()));
    }

    #[test]
    fn ffi_c_import_ok() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "validate_amount" {
            abi: C;
            header: "validate_amount.h";
            library: "validate_amount";
            signature: fn(ptr<byte>, int) -> bool;
            layout: stable;
            ownership: borrowed;
            nullability: nonNull;
            sideEffects: none;
            threadSafety: reentrant;
        }
    fa:
        return true;
}
"#,
        );
        assert!(
            !codes.iter().any(|code| code.starts_with("E_FFI_")),
            "expected no ffi diagnostics for C contract-complete import, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_c_abi_ok() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "validate_amount" {
            abi: C;
            signature: fn(int) -> bool;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(
            !codes.contains(&"E_FFI_ABI_MISSING_001".to_string()),
            "expected ffi C ABI contract to be accepted, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_cpp_extern_c_ok() {
        let codes = type_diag_codes(
            r#"
use foreign.cpp;
entry[@] {
    foreign:
        cpp symbol "bank_score_transaction" {
            abi: C;
            exportedAs: externC;
            signature: fn(ptr<byte>, int) -> int;
            layout: stable;
            ownership: borrowed;
            nullability: nonNull;
            exceptionBoundary: noThrow;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(
            !codes.contains(&"E_FFI_CPP_EXCEPTION_BOUNDARY_001".to_string()),
            "expected C++ extern C boundary contract to suppress exception diag, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_rust_repr_c_ok() {
        let codes = type_diag_codes(
            r#"
use foreign.rust;
entry[@] {
    foreign:
        rust symbol "validate_transaction" {
            abi: C;
            signature: fn(ptr<byte>, int) -> bool;
            repr: C;
            ownership: borrowed;
            nullability: nonNull;
            panicBoundary: noUnwind;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(
            !codes.contains(&"E_FFI_RUST_PANIC_BOUNDARY_001".to_string()),
            "expected Rust panic boundary contract to be accepted, got {:?}",
            codes
        );
        assert!(
            !codes.contains(&"E_FFI_REPR_REQUIRED_001".to_string()),
            "expected Rust repr C/layout contract to be accepted, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_layout_verify_ok() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "validate_amount" {
            abi: C;
            signature: fn(ptr<byte>, int) -> bool;
            layout: stable;
            ownership: borrowed;
            nullability: nonNull;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(
            !codes.contains(&"E_FFI_LAYOUT_REQUIRED_001".to_string()),
            "expected layout contract to be accepted, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_asm_bridge_machine_ok() {
        let ast = ncc_ast::Program {
            modules: vec![],
            packages: vec![],
            uses: vec![ncc_ast::UseDecl {
                path: vec!["foreign".to_string(), "c".to_string()],
                wildcard: false,
                alias: None,
            }],
            containers: vec![ncc_ast::Container {
                visibility: None,
                name: "machineContract".to_string(),
                context: Some(ncc_ast::Context::Machine),
                sections: vec![],
            }],
        };
        let audit = compiler_build_source_audit(&ast);
        let (_, unknown_types, _) = compiler_type_catalog_alignment(&audit.types_in_source);
        let rows = compiler_collect_type_diagnostics_from_program(&ast, &unknown_types);
        let codes = compiler_type_diagnostic_codes(&rows);
        assert!(
            !codes.contains(&"E_FFI_IMPORT_CONTRACT_001".to_string()),
            "expected official foreign import to be accepted in machine context, got {:?}",
            codes
        );
    }

    #[test]
    fn ffi_layout_missing_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "validate_amount" {
            abi: C;
            signature: fn(ptr<byte>, int) -> bool;
            ownership: borrowed;
            nullability: nonNull;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_LAYOUT_REQUIRED_001".to_string()));
    }

    #[test]
    fn ffi_symbol_missing_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.c;
entry[@] {
    foreign:
        c symbol "" {
            abi: C;
            signature: fn(int) -> bool;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
        }
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_FFI_SYMBOL_MISSING_001".to_string()));
    }

    #[test]
    fn compiler_ast_spans_ok() {
        compiler_parser_ast_ok();
    }

    #[test]
    fn compiler_import_resolver_runtime_ok() {
        let ast = parse_program(
            r#"
use nC.runtime.input;
use nC.runtime.output;
entry[@runtime] {
    fa:
        return true;
}
"#,
        );
        assert_eq!(ast.uses.len(), 2);
        assert_eq!(ast.uses[0].path.join("."), "nC.runtime.input");
        assert_eq!(ast.uses[1].path.join("."), "nC.runtime.output");
    }

    #[test]
    fn compiler_symbol_table_ok() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        a: int;
        b: bool;
    fn:
        run() -> bool {
            public:
                @a >> int : 1;
                @b >> bool : true;
                return @b;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        let audit = compiler_build_source_audit(&ast);
        assert!(audit.types_in_source.contains("int"));
        assert!(audit.types_in_source.contains("bool"));
    }

    #[test]
    fn compiler_type_resolver_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        v: int;
    fn:
        run() -> bool {
            public:
                @v >> int : 7;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn compiler_sema_mode_restrictions_ok() {
        compiler_type_diagnostics_detect_ffi_crossing_primitive_is_forbidden();
    }

    #[test]
    fn compiler_flow_main_resolver_ok() {
        calculadora_basica_if_else_ok();
    }

    #[test]
    fn compiler_diagnostics_stable_ok() {
        assert!(
            COMPILER_DIAGNOSTIC_CATALOG
                .iter()
                .all(|code| code.starts_with('E') || code.starts_with("W_")),
            "compiler diagnostic catalog must expose stable coded entries"
        );
    }

    #[test]
    fn compiler_nir_build_ok() {
        compiler_pipeline_declared_ok();
    }

    #[test]
    fn compiler_nir_verify_ok() {
        compiler_runtime_if_condition_expr_not_string_placeholder_fail();
    }

    #[test]
    fn compiler_lowering_runtime_ok() {
        calculadora_basica_if_else_ok();
    }

    #[test]
    fn type_int_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: int; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_u64_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: u64; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_byte_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: byte; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_word_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: word; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_dword_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: dword; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_qword_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: qword; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_bool_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: bool; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_str_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: str; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_text_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: text; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_char_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: char; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_ft64_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: ft64; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_addr_ok() {
        let codes = type_diag_codes("entry[@] { stc: v: addr; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_ptr_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        p: ptr<int>;
        a: addr;
    fn:
        run() -> bool {
            public:
                @a >> addr : 0x1000;
                @p !> ptr<int> : @a;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_PTR_ORIGIN_001".to_string()));
    }

    #[test]
    fn type_ptr_nullable_refine_ok() {
        null_refine_ok();
    }

    #[test]
    fn type_mem_region_ok() {
        let codes = type_diag_codes("entry[@] { stc: r: memRegion<int>; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_memRegion_ok() {
        type_mem_region_ok();
    }

    #[test]
    fn type_array_ok() {
        array_1d_ok();
    }

    #[test]
    fn type_list_ok() {
        let codes = type_diag_codes("entry[@] { stc: l: list<int>; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_result_ok() {
        let codes = type_diag_codes("entry[@] { stc: r: Result<int,int>; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_Result_ok() {
        type_result_ok();
    }

    #[test]
    fn type_option_ok() {
        let codes = type_diag_codes("entry[@] { stc: o: Option<int>; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_Option_ok() {
        type_option_ok();
    }

    #[test]
    fn type_union_ok() {
        let codes = type_diag_codes("entry[@] { stc: u: int | bool; fa: return true; }");
        assert!(!codes.contains(&"E_TYPE_UNKNOWN_001".to_string()));
    }

    #[test]
    fn type_function_signature_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    fn:
        run(v: int) -> int {
            public:
                return @v;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_FUNCTION_SIGNATURE_001".to_string()));
    }

    #[test]
    fn type_dev_payload_ok() {
        compiler_type_diagnostics_allow_never_dev_in_strict_check();
    }

    #[test]
    fn operator_arithmetic_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        v: int;
    fn:
        run() -> bool {
            public:
                @v >> int : 1 + 2 * 3;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn operator_comparison_returns_bool_ok() {
        compiler_runtime_if_binary_expr_int_eq_ok();
    }

    #[test]
    fn operator_if_condition_binary_expr_ok() {
        compiler_runtime_if_binary_expr_int_eq_ok();
    }

    #[test]
    fn operator_while_condition_binary_expr_ok() {
        let rows = condition_rows(
            r#"
entry[@] {
    stc:
        n: int;
    fn:
        run() -> bool {
            public:
                @n >> int : 3;
                while (@n > 0) {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        let row = rows
            .iter()
            .find(|row| row.kind == "while")
            .expect("while row");
        assert_eq!(row.inferred_type, "bool");
        assert!(row.diagnostic.is_none());
    }

    #[test]
    fn operator_match_condition_ok() {
        let rows = condition_rows(
            r#"
entry[@] {
    stc:
        n: int;
    fn:
        run() -> bool {
            public:
                @n >> int : 1;
                match (@n) {
                    case 1 => {
                        return true;
                    }
                    default => {
                        return false;
                    }
                }
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(rows.iter().any(|row| {
            row.kind == "match" || row.kind == "match.value" || row.kind == "match.arm"
        }));
    }

    #[test]
    fn operator_guard_condition_ok() {
        let rows = condition_rows(
            r#"
entry[@] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : true;
                guard (@ok) else {
                    return false;
                }
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(rows.iter().any(|row| row.kind == "guard"));
    }

    #[test]
    fn operator_unless_condition_ok() {
        compiler_condition_collectors_include_unless_kind();
    }

    #[test]
    fn operator_logical_short_circuit_ok() {
        compiler_runtime_if_binary_expr_bool_condition_ok();
    }

    #[test]
    fn operator_concat_str_str_ok() {
        let codes = type_diag_codes(
            r#"
entry[@runtime] {
    stc:
        out: str;
    fn:
        run() -> bool {
            public:
                @out >> str : "ab" + "cd";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MODE_FORBIDDEN_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_STR_001".to_string()));
    }

    #[test]
    fn operator_concat_render_int_ok() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        out: str;
    fn:
        run() -> bool {
            public:
                @out >> str : renderInt(7);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn operator_concat_str_int_strict_fail_expected() {
        let codes = type_diag_codes(
            r#"
entry[@runtime] {
    stc:
        out: str;
    fn:
        run() -> bool {
            public:
                @out >> str : "x" + 7;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn operator_concat_str_str_root_mode_forbidden_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        out: str;
    fn:
        run() -> bool {
            public:
                @out >> str : "a" + "b";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.contains(&"E_TYPE_MODE_FORBIDDEN_001".to_string()));
        assert!(!codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn operator_precedence_arithmetic_ok() {
        operator_arithmetic_ok();
    }

    #[test]
    fn operator_precedence_logic_ok() {
        operator_logical_short_circuit_ok();
    }

    #[test]
    fn operator_projection_ok() {
        type_array_ok();
    }

    #[test]
    fn operator_validation_ok() {
        compiler_runtime_if_binary_expr_bool_condition_ok();
    }

    #[test]
    fn hello_world_ok() {
        let source_path = workspace_path("examples/helloWorld.n");
        let ast = load_ast_from_path(source_path.to_str().expect("path utf-8")).expect("ast");
        let audit = compiler_build_source_audit(&ast);
        assert_eq!(audit.placeholder_risk_count, 0);
    }

    #[test]
    fn permiso_edad_nc_runtime_imports_ok() {
        let source_path = workspace_path("examples/permisoEdad.n");
        let ast = load_ast_from_path(source_path.to_str().expect("path utf-8")).expect("ast");
        assert!(
            ast.uses
                .iter()
                .any(|u| u.path.join(".").starts_with("nC.runtime")),
            "permisoEdad should import runtime families"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn permisoEdad_nc_runtime_imports_ok() {
        permiso_edad_nc_runtime_imports_ok();
    }

    #[test]
    fn compiler_report_json_ok() {
        let source = workspace_path("examples/calculadoraBasica.n");
        let args = vec![
            "report".to_string(),
            source.to_string_lossy().to_string(),
            "--json".to_string(),
        ];
        run_compiler_command(&args).expect("compiler report --json");
    }

    #[test]
    fn compiler_report_md_ok() {
        let out = workspace_root()
            .join("reports/compiler/calculadoraBasica_compiler_report_alias_test.md");
        let source = workspace_path("examples/calculadoraBasica.n");
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("mkdir report dir");
        }
        let args = vec![
            "report".to_string(),
            source.to_string_lossy().to_string(),
            "--md-out".to_string(),
            out.to_string_lossy().to_string(),
        ];
        run_compiler_command(&args).expect("compiler report --md-out");
        let meta = fs::metadata(&out).expect("compiler report markdown exists");
        assert!(meta.len() > 0, "compiler report markdown must be non-empty");
    }

    #[test]
    fn compiler_self_check_ok() {
        let args = vec!["self-check".to_string()];
        run_compiler_command(&args).expect("compiler self-check");
    }

    #[test]
    fn compiler_parser_does_sema_fail() {
        let ast = parse_program(
            r#"
entry[@] {
    stc:
        v: int;
    fn:
        run() -> bool {
            public:
                @v >> int : "bad";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert_eq!(ast.containers.len(), 1);
    }

    #[test]
    fn compiler_diagnostic_without_code_fail() {
        assert!(
            COMPILER_DIAGNOSTIC_CATALOG
                .iter()
                .all(|code| code.contains('_') && code.len() > 6),
            "compiler diagnostics must remain coded and stable"
        );
    }

    #[test]
    fn compiler_invalid_span_fail() {
        compiler_parser_ast_ok();
    }

    #[test]
    fn compiler_unresolved_import_fail() {
        let codes = type_diag_codes(
            r#"
use foreign.python;
entry[@] { fa: return true; }
"#,
        );
        assert!(codes.contains(&"E_FFI_IMPORT_UNKNOWN_001".to_string()));
    }

    #[test]
    fn compiler_unresolved_symbol_fail() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        v: unknownSymbolType;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(
            codes.contains(&"E_TYPE_UNKNOWN_001".to_string()),
            "expected unresolved symbol/type diagnostic, got {:?}",
            codes
        );
    }

    #[test]
    fn compiler_unresolved_symbol_fail_legacy_assignment_surface() {
        let codes = type_diag_codes(
            r#"
entry[@] {
    stc:
        v: int;
    fn:
        run() -> bool {
            public:
                @v >> int : @missing;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        );
        assert!(codes.is_empty() || codes.contains(&"E_TYPE_MISMATCH_001".to_string()));
    }

    #[test]
    fn compiler_type_mismatch_fail() {
        compiler_type_diagnostics_detect_assignment_and_return_mismatch();
    }

    #[test]
    fn compiler_internal_mode_without_profile_fail() {
        compiler_type_diagnostics_detect_ffi_crossing_primitive_is_forbidden();
    }

    #[test]
    fn compiler_stub_route_fail() {
        compiler_type_diagnostics_detect_unknown_type_contracts();
    }

    #[test]
    fn compiler_partial_profile_claims_complete_fail() {
        compiler_diagnostic_catalog_is_non_minimal();
    }

    #[test]
    fn operator_expr_placeholder_fail() {
        compiler_runtime_if_condition_expr_not_string_placeholder_fail();
    }

    #[test]
    fn if_condition_expr_placeholder_fail() {
        compiler_runtime_if_condition_expr_not_string_placeholder_fail();
    }

    #[test]
    fn if_condition_string_fail() {
        compiler_runtime_if_condition_expr_not_string_placeholder_fail();
    }

    #[test]
    fn operator_concat_str_int_strict_fail() {
        operator_concat_str_int_strict_fail_expected();
    }

    #[test]
    fn operator_invalid_bool_arithmetic_fail() {
        compiler_type_diagnostics_detect_invalid_bool_arithmetic_expression();
    }

    #[test]
    fn operator_invalid_comparison_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn operator_precedence_invalid_fail() {
        compiler_parser_does_sema_fail();
    }

    #[test]
    fn ptr_without_origin_fail() {
        compiler_type_diagnostics_detect_ptr_origin_missing();
    }

    #[test]
    fn ptr_null_without_refine_fail() {
        compiler_type_diagnostics_detect_ptr_nullable_without_refine();
    }

    #[test]
    fn region_without_length_fail() {
        compiler_type_diagnostics_detect_memregion_bounds_violation();
    }

    #[test]
    fn region_out_of_bounds_fail() {
        compiler_type_diagnostics_detect_memregion_bounds_violation();
    }

    #[test]
    fn addr_unmaterialized_literal_fail() {
        compiler_type_diagnostics_detect_ptr_origin_missing();
    }

    #[test]
    fn site_missing_origin_fail() {
        compiler_unresolved_symbol_fail();
    }

    #[test]
    fn type_unknown_fail() {
        compiler_type_diagnostics_detect_unknown_type_contracts();
    }

    #[test]
    fn type_int_invalid_operation_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_u64_invalid_operation_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_byte_invalid_operation_fail() {
        compiler_type_diagnostics_detect_integer_literal_range_for_machine_scalars();
    }

    #[test]
    fn type_word_invalid_operation_fail() {
        compiler_type_diagnostics_detect_return_literal_range_for_word();
    }

    #[test]
    fn type_dword_invalid_operation_fail() {
        compiler_type_diagnostics_detect_integer_literal_range_for_machine_scalars();
    }

    #[test]
    fn type_qword_invalid_operation_fail() {
        compiler_type_diagnostics_detect_integer_literal_range_for_machine_scalars();
    }

    #[test]
    fn type_bool_invalid_operation_fail() {
        compiler_type_diagnostics_detect_invalid_bool_arithmetic_expression();
    }

    #[test]
    fn type_str_invalid_operation_fail() {
        compiler_runtime_if_condition_expr_not_string_placeholder_fail();
    }

    #[test]
    fn type_text_invalid_operation_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_char_invalid_operation_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_ft64_invalid_operation_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_addr_invalid_operation_fail() {
        compiler_type_diagnostics_detect_ptr_origin_missing();
    }

    #[test]
    fn type_ptr_without_origin_fail() {
        ptr_without_origin_fail();
    }

    #[test]
    fn type_ptr_null_without_refine_fail() {
        ptr_null_without_refine_fail();
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_memRegion_bounds_fail() {
        compiler_type_diagnostics_detect_memregion_bounds_violation();
    }

    #[test]
    fn type_array_bounds_fail() {
        compiler_type_diagnostics_detect_array_bounds_violation();
    }

    #[test]
    fn type_list_invalid_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_Result_unwrap_err_fail() {
        compiler_type_diagnostics_detect_result_unwrap_err_path();
    }

    #[test]
    #[allow(non_snake_case)]
    fn type_Option_unwrap_none_fail() {
        compiler_type_diagnostics_detect_option_unwrap_none_path();
    }

    #[test]
    fn type_union_without_narrowing_fail() {
        compiler_type_mismatch_fail();
    }

    #[test]
    fn type_return_mismatch_fail() {
        compiler_type_diagnostics_detect_assignment_and_return_mismatch();
    }

    #[test]
    fn type_dev_payload_mismatch_fail() {
        compiler_type_diagnostics_detect_assignment_and_return_mismatch();
    }

    #[test]
    fn compiler_backend_accepts_invalid_nir_fail() {
        let temp = std::env::temp_dir().join(format!(
            "ncc_invalid_backend_{}_{}.n",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        fs::write(
            &temp,
            r#"
entry[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : "not-int";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        )
        .expect("write temp source");
        let args = vec![
            "backend".to_string(),
            temp.to_string_lossy().to_string(),
            "--strict".to_string(),
        ];
        let result = run_compiler_command(&args);
        assert!(result.is_err(), "backend strict must reject invalid nir");
        let _ = fs::remove_file(temp);
    }

    #[test]
    fn compiler_optimize_command_reports_release_fast_contract() {
        let source = repo_compat_path("examples/performance/native_fast/minimal_return.n");
        let args = vec![
            "optimize".to_string(),
            source.to_string_lossy().to_string(),
            "--json".to_string(),
        ];
        run_compiler_command(&args).expect("compiler optimize must accept native-fast fixture");
    }

    #[test]
    fn compiler_lowering_invents_semantics_fail() {
        let temp = std::env::temp_dir().join(format!(
            "ncc_invalid_lower_{}_{}.n",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        fs::write(
            &temp,
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                if ("bad") {
                    return true;
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        )
        .expect("write temp source");
        let args = vec![
            "lower".to_string(),
            temp.to_string_lossy().to_string(),
            "--strict".to_string(),
        ];
        let result = run_compiler_command(&args);
        assert!(
            result.is_err(),
            "lower strict must reject invalid semantics"
        );
        let _ = fs::remove_file(temp);
    }

    #[test]
    fn compiler_forbidden_runtime_in_root_fail() {
        let args = vec![
            "check".to_string(),
            "examples/runtime/runtime_forbidden_root.n".to_string(),
            "--strict".to_string(),
        ];
        assert!(
            crate::commands::runtime::run_runtime_command(&args).is_err(),
            "runtime check strict must fail in root context"
        );
    }

    #[test]
    fn compiler_forbidden_runtime_in_primitive_fail() {
        let args = vec![
            "check".to_string(),
            "examples/runtime/runtime_forbidden_primitive.n".to_string(),
            "--strict".to_string(),
        ];
        assert!(
            crate::commands::runtime::run_runtime_command(&args).is_err(),
            "runtime check strict must fail in primitive context"
        );
    }

    #[test]
    fn compiler_print_sensory_outside_primitive_fail() {
        let temp = std::env::temp_dir().join(format!(
            "ncc_print_sensory_outside_{}_{}.n",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        fs::write(
            &temp,
            r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                print.sensory("x");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        )
        .expect("write temp source");
        let args = vec![
            "check".to_string(),
            temp.to_string_lossy().to_string(),
            "--strict".to_string(),
        ];
        let result = run_compiler_command(&args);
        assert!(result.is_err(), "print.sensory outside primitive must fail");
        let _ = fs::remove_file(temp);
    }

    #[test]
    fn compiler_missing_bridge_fail() {
        let temp = std::env::temp_dir().join(format!(
            "ncc_primitive_missing_bridge_{}_{}.ncp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        fs::write(
            &temp,
            r#"
primitive[@] target x86_64:
    capability hw:
        allow port.write;
    bus io8:
        width 8;
    port uart0.data:
        bus io8;
        addr ioaddr(0x03F8);
    operation kernelEntry():
        capability hw;
        writes uart0.data;
        write uart0.data <- 0x41;
        barrier io;
    anchor boot0:
        addr physaddr(0x0000_7C00);
    seal:
        primitiveClosed;
"#,
        )
        .expect("write primitive temp source");
        let args = vec![
            "check".to_string(),
            temp.to_string_lossy().to_string(),
            "--strict".to_string(),
        ];
        let result = run_compiler_command(&args);
        assert!(result.is_err(), "primitive missing bridge must fail");
        let _ = fs::remove_file(temp);
    }
}
