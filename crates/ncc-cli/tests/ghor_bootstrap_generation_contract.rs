use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(path: impl AsRef<Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root().join(path)).expect("read retained bootstrap JSON"))
        .expect("parse retained bootstrap JSON")
}

#[test]
fn stage3_plan_has_one_compiler_one_launcher_and_explicit_sources() {
    let plan = json("bootstrap/stage3-generation-plan.json");
    assert_eq!(plan["schema"], "ghor.bootstrap.generation-manifest.v1");
    let components = plan["components"].as_array().expect("components");
    assert_eq!(
        components
            .iter()
            .filter(|component| component["role"] == "compiler")
            .count(),
        1
    );
    assert_eq!(
        components
            .iter()
            .filter(|component| component["role"] == "launcher")
            .count(),
        1
    );
    assert!(components.iter().all(|component| {
        component["sources"]
            .as_array()
            .is_some_and(|sources| !sources.is_empty())
    }));
    assert!(components.iter().all(|component| {
        component["smoke_args"]
            .as_array()
            .is_some_and(|arguments| !arguments.is_empty())
    }));
    assert!(components.iter().all(|component| {
        component["smoke_dependencies"]
            .as_array()
            .is_some_and(|dependencies| !dependencies.is_empty())
    }));
    assert!(components.iter().all(|component| {
        component["smoke_dependencies"]
            .as_array()
            .is_some_and(|dependencies| {
                dependencies.iter().all(|dependency| {
                    dependency["path"]
                        .as_str()
                        .is_some_and(|path| !path.is_empty())
                        && dependency["min_bytes"]
                            .as_u64()
                            .is_some_and(|bytes| bytes > 0)
                })
            })
    }));
    assert!(components.iter().all(|component| {
        component["migration"]["native_units"]
            .as_array()
            .is_some_and(|units| !units.is_empty())
            && component["migration"]["stage0_units"].is_array()
            && component["migration"]["complete"].is_boolean()
    }));
    let compiler = components
        .iter()
        .find(|component| component["role"] == "compiler")
        .expect("compiler component");
    assert_eq!(compiler["migration"]["complete"], false);
    assert!(compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| !units.is_empty()));
    let corpus = plan["corpus"].as_array().expect("generation corpus");
    assert_eq!(corpus.len(), 309);
    for retained in [
        "ncc-lexer-surface",
        "ncc-content-cache-native-io",
        "ncc-parser-outline-surface",
        "ncc-parser-outline-mismatched",
        "ncc-parser-ast-head-shape",
        "ncc-parser-ast-head-shape-reject",
        "ncc-parser-ast-all-sections",
        "ncc-parser-ast-declarations-json",
        "ncc-parser-ast-declarations-shape",
        "ncc-parser-ast-declarations-shape-reject",
        "ncc-parser-ast-material-json",
        "ncc-parser-ast-material-shape",
        "ncc-parser-ast-invalid-type",
        "ncc-parser-ast-invalid-expression",
        "ncc-parser-ast-statements-json",
        "ncc-parser-ast-statements-shape",
        "ncc-parser-ast-invalid-statement",
        "ncc-parser-ast-advanced-json",
        "ncc-parser-ast-advanced-shape",
        "ncc-parser-ast-invalid-advanced",
        "ncc-sema-core-json",
        "ncc-sema-core-shape",
        "ncc-sema-core-shape-reject",
        "ncc-sema-duplicate",
        "ncc-sema-unresolved",
        "ncc-sema-unknown-type",
        "ncc-sema-condition",
        "ncc-sema-call-arity",
        "ncc-sema-return",
        "ncc-sema-entry",
        "ncc-sema-break",
        "ncc-sema-import-runtime-modal-json",
        "ncc-sema-import-runtime-modal-shape",
        "ncc-sema-import-runtime-modal-shape-reject",
        "ncc-sema-import-catalog-273",
        "ncc-sema-authority-catalog-json",
        "ncc-sema-authority-catalog-shape",
        "ncc-sema-authority-system",
        "ncc-sema-authority-network",
        "ncc-sema-authority-security",
        "ncc-sema-authority-compiler",
        "ncc-sema-authority-kernel-core",
        "ncc-sema-authority-context-denied",
        "ncc-sema-authority-dom-denied",
        "ncc-runtime-signatures-catalog-json",
        "ncc-runtime-signatures-catalog-shape",
        "ncc-sema-runtime-signatures-json",
        "ncc-sema-runtime-signature-arity",
        "ncc-sema-runtime-signature-type",
        "ncc-sema-runtime-signature-import",
        "ncc-sema-generic-trait-abi-json",
        "ncc-sema-generic-duplicate-param",
        "ncc-sema-generic-unknown-bound",
        "ncc-sema-const-generic-range",
        "ncc-sema-trait-impl-missing-member",
        "ncc-sema-abi-incomplete",
        "ncc-sema-abi-unknown-binding",
        "ncc-sema-dynamic-dispatch-implicit",
        "ncc-sema-pattern-async-resource-json",
        "ncc-sema-match-non-exhaustive",
        "ncc-sema-match-unreachable",
        "ncc-sema-match-guard-type",
        "ncc-sema-pattern-range-empty",
        "ncc-sema-capture-duplicate",
        "ncc-sema-capture-unknown",
        "ncc-sema-capture-mut-required",
        "ncc-sema-capture-moved",
        "ncc-sema-defer-control-flow",
        "ncc-sema-await-context",
        "ncc-sema-await-non-future",
        "ncc-sema-yield-context",
        "ncc-ir-material-json",
        "ncc-ir-x86-64-sysv",
        "ncc-ir-x86-64-win64",
        "ncc-ir-x86-64-darwin",
        "ncc-ir-i386-sysv",
        "ncc-ir-aarch64-sysv",
        "ncc-ir-aarch64-darwin",
        "ncc-ir-armv7-eabi",
        "ncc-ir-riscv32",
        "ncc-ir-riscv64",
        "ncc-ir-wasm32",
        "ncc-ir-target-unsupported",
        "ncc-mct-material-json",
        "ncc-mct-scalar-immediate-json",
        "ncc-mct-x86-64-sysv",
        "ncc-mct-x86-64-win64",
        "ncc-mct-x86-64-darwin",
        "ncc-mct-i386-sysv",
        "ncc-mct-aarch64-sysv",
        "ncc-mct-aarch64-darwin",
        "ncc-mct-armv7-eabi",
        "ncc-mct-riscv32",
        "ncc-mct-riscv64",
        "ncc-mct-wasm32",
        "ncc-mct-target-unsupported",
        "ncc-mct-pointer-intrinsics-win64-readiness",
        "ncc-mct-pointer-inference-win64-readiness",
        "ncc-mct-memory-layout-win64-readiness",
        "ncc-mct-target-x86-64-sysv",
        "ncc-mct-target-x86-64-win64",
        "ncc-mct-target-x86-64-darwin",
        "ncc-mct-target-i386-sysv",
        "ncc-mct-target-aarch64-sysv",
        "ncc-mct-target-aarch64-darwin",
        "ncc-mct-target-armv7-eabi",
        "ncc-mct-target-riscv32",
        "ncc-mct-target-riscv64",
        "ncc-mct-target-wasm32",
        "ncc-platform-debug-x86-64-sysv",
        "ncc-platform-debug-x86-64-win64",
        "ncc-platform-debug-x86-64-darwin",
        "ncc-platform-debug-i386-sysv",
        "ncc-platform-debug-aarch64-sysv",
        "ncc-platform-debug-aarch64-darwin",
        "ncc-platform-debug-armv7-eabi",
        "ncc-platform-debug-riscv32",
        "ncc-platform-debug-riscv64",
        "ncc-platform-debug-wasm32",
        "ncc-llvm-optimization-o0",
        "ncc-llvm-optimization-o1",
        "ncc-llvm-optimization-o2",
        "ncc-llvm-optimization-o3",
        "ncc-llvm-optimization-os",
        "ncc-llvm-optimization-oz",
        "ncc-llvm-material-cfg-transform-o2",
        "ncc-llvm-ssa-interprocedural-dce-o2",
        "ncc-llvm-static-trip-loops-o2",
        "ncc-llvm-induction-strength-o2",
        "ncc-llvm-licm-derived-induction-o2",
        "ncc-llvm-private-leaf-inlining-o2",
        "ncc-llvm-linear-parameter-inlining-o2",
        "ncc-llvm-bounded-argument-rematerialization-o2",
        "ncc-llvm-observable-temporary-o2",
        "ncc-llvm-observable-temporary-pair-o2",
        "ncc-llvm-observable-temporary-effect-guard",
        "ncc-llvm-observable-temporary-capacity-guard",
        "ncc-llvm-linked-graph-gvn-o2",
        "ncc-msf7-pdb",
        "ncc-native-unwind-x86-64-sysv",
        "ncc-native-unwind-x86-64-win64",
        "ncc-native-unwind-x86-64-darwin",
        "ncc-native-unwind-i386-sysv",
        "ncc-native-unwind-aarch64-sysv",
        "ncc-native-unwind-aarch64-darwin",
        "ncc-native-unwind-armv7-eabi",
        "ncc-native-unwind-riscv32",
        "ncc-native-unwind-riscv64",
        "ncc-native-unwind-wasm32",
        "ncc-object-material-json",
        "ncc-symbol-bundle-x86-64-sysv",
        "ncc-symbol-bundle-x86-64-win64",
        "ncc-symbol-bundle-x86-64-darwin",
        "ncc-symbol-bundle-i386-sysv",
        "ncc-symbol-bundle-aarch64-sysv",
        "ncc-symbol-bundle-aarch64-darwin",
        "ncc-symbol-bundle-armv7-eabi",
        "ncc-symbol-bundle-riscv32",
        "ncc-symbol-bundle-riscv64",
        "ncc-symbol-bundle-wasm32",
        "ncc-object-x86-64-sysv",
        "ncc-object-x86-64-win64",
        "ncc-object-x86-64-darwin",
        "ncc-object-i386-sysv",
        "ncc-object-aarch64-sysv",
        "ncc-object-aarch64-darwin",
        "ncc-object-armv7-eabi",
        "ncc-object-riscv32",
        "ncc-object-riscv64",
        "ncc-object-wasm32",
        "ncc-object-target-unsupported",
        "ncc-object-identity-x86-64-sysv",
        "ncc-object-identity-x86-64-win64",
        "ncc-object-identity-x86-64-darwin",
        "ncc-object-identity-i386-sysv",
        "ncc-object-identity-aarch64-sysv",
        "ncc-object-identity-aarch64-darwin",
        "ncc-object-identity-armv7-eabi",
        "ncc-object-identity-riscv32",
        "ncc-object-identity-riscv64",
        "ncc-object-identity-wasm32",
        "ncc-object-source-mangling-x86-64-sysv",
        "ncc-object-source-mangling-x86-64-win64",
        "ncc-object-source-mangling-x86-64-darwin",
        "ncc-object-source-mangling-i386-sysv",
        "ncc-object-source-mangling-aarch64-sysv",
        "ncc-object-source-mangling-aarch64-darwin",
        "ncc-object-source-mangling-armv7-eabi",
        "ncc-object-source-mangling-riscv32",
        "ncc-object-source-mangling-riscv64",
        "ncc-object-source-mangling-wasm32",
        "ncc-object-runtime-abi-hosted-entry-x86-64-sysv",
        "ncc-final-link-plan-x86-64-sysv",
        "ncc-final-link-plan-x86-64-win64",
        "ncc-final-link-plan-x86-64-darwin",
        "ncc-final-link-plan-i386-sysv",
        "ncc-final-link-plan-aarch64-sysv",
        "ncc-final-link-plan-aarch64-darwin",
        "ncc-final-link-plan-armv7-eabi",
        "ncc-final-link-plan-riscv32",
        "ncc-final-link-plan-riscv64",
        "ncc-final-link-plan-wasm32-wasi",
        "ncc-final-image-pe64-loader-exit",
        "ncc-final-image-pe64-runtime-iat",
        "ncc-final-image-elf64-x86-64-loader-exit",
        "ncc-final-image-elf64-x86-64-runtime-dynamic",
        "ncc-final-image-elf64-aarch64-loader-exit",
        "ncc-final-image-elf64-aarch64-runtime-dynamic",
        "ncc-final-image-elf64-riscv64-loader-exit",
        "ncc-final-image-elf64-riscv64-runtime-dynamic",
        "ncc-final-image-elf32-i386-loader-exit",
        "ncc-final-image-elf32-i386-runtime-dynamic",
        "ncc-final-image-elf32-armv7-loader-exit",
        "ncc-final-image-elf32-riscv32-loader-exit",
        "ncc-final-image-macho64-x86-64-loader-exit",
        "ncc-final-image-macho64-x86-64-runtime-dynamic",
        "ncc-final-image-macho64-aarch64-loader-exit",
        "ncc-final-image-macho64-aarch64-runtime-dynamic",
        "ncc-final-image-wasm32-loader-exit",
        "ncc-final-image-wasm32-runtime-import",
        "ncc-project-object-x86-64-sysv",
        "ncc-project-object-x86-64-win64",
        "ncc-project-object-x86-64-darwin",
        "ncc-project-object-i386-sysv",
        "ncc-project-object-aarch64-sysv",
        "ncc-project-object-aarch64-darwin",
        "ncc-project-object-armv7-eabi",
        "ncc-project-object-riscv32",
        "ncc-project-object-riscv64",
        "ncc-project-object-wasm32",
        "ncc-project-private-cross-unit-rejected",
        "ncc-relocation-x86-64-sysv",
        "ncc-relocation-x86-64-win64",
        "ncc-relocation-x86-64-darwin",
        "ncc-relocation-i386-sysv",
        "ncc-relocation-aarch64-sysv",
        "ncc-relocation-aarch64-darwin",
        "ncc-relocation-armv7-eabi",
        "ncc-relocation-riscv32",
        "ncc-relocation-riscv64",
        "ncc-relocation-wasm32",
        "ncc-relocation-cross-isa-rejected",
        "ncc-sema-import-domain",
        "ncc-sema-import-family",
        "ncc-sema-import-item",
        "ncc-sema-import-wildcard",
        "ncc-sema-import-machine-gate",
        "ncc-sema-runtime-use-missing",
        "ncc-stage0-boundary",
        "ghor-self-test",
    ] {
        assert!(
            corpus.iter().any(|case| case["name"] == retained),
            "generation corpus omitted {retained}"
        );
    }
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "parser-declarations-and-signatures")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "parser-material-types-and-expressions")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "parser-core-statements-and-control-flow")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "parser-patterns-closures-and-advanced-expressions")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-core-scopes-symbols-and-type-flow")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-canonical-import-catalog-and-modal-gates")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-core-runtime-import-resolution")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-runtime-context-capability-and-policy-authority")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-advanced-runtime-signatures")));
    assert!(compiler["sources"].as_array().is_some_and(|sources| sources
        .iter()
        .any(|source| source == "src/runtime_signatures.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-generics-traits-and-abi-contracts")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/generic_abi.n")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-generics-traits-and-abi-contracts")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-pattern-async-and-resource-contracts")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "ir-material-graph-target-abi-and-effects")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(
            |unit| unit == "ir-mct-source-spelling-qualified-mangling-and-collision-resolution"
        )));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/ir.n")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit == "ir")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-material-instruction-selection-relocation-and-target-bytes")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-material-relocation-target-identity")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct.n")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit == "mct")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-relocatable-object-writers-elf-coff-macho-wasm")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-object-symbol-and-relocation-identity")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-canonical-runtime-abi-and-hosted-native-entry")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-final-link-provider-placement-and-target-thunks")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct_object.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-final-relocation-engine-x86-aarch64-arm-riscv-wasm")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct_link.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-pe64-final-image-writer-import-directory-and-loader-smoke")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| {
            unit
            == "mct-elf64-final-image-writer-x86-64-aarch64-riscv64-dynamic-loader-and-linux-smoke"
        })));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit
            == "mct-macho64-final-image-writer-x86-64-aarch64-dyld-bind-symbols-and-ad-hoc-signature")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| {
            unit
            == "mct-elf32-final-image-writer-i386-and-armv7-dynamic-rv32-static-and-linux-smokes"
        })));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-canonical-prelude-runtime-symbols-all-target-images")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit
                == "mct-wasm32-final-image-writer-runtime-imports-names-mct-and-node-smoke")));
    for source in [
        "src/mct_image_contract.n",
        "src/mct_image_pe.n",
        "src/mct_image_elf.n",
        "src/mct_image_elf32.n",
        "src/mct_image_macho.n",
        "src/mct_image_wasm.n",
        "src/mct_image.n",
    ] {
        assert!(compiler["sources"]
            .as_array()
            .is_some_and(|sources| sources.iter().any(|item| item == source)));
    }
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit
                == "mct-portable-source-map-and-function-unwind-records-all-target-objects")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct_debug.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-platform-dwarf4-and-codeview-c13-debug-record-writers")));
    assert!(compiler["sources"].as_array().is_some_and(|sources| sources
        .iter()
        .any(|source| source == "src/mct_platform_debug.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-native-dwarf-cfi-win64-and-macho-compact-unwind-writers")));
    assert!(compiler["sources"].as_array().is_some_and(|sources| sources
        .iter()
        .any(|source| source == "src/mct_native_unwind.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-canonical-symbol-bundle-writer-all-target-objects")));
    assert!(compiler["sources"].as_array().is_some_and(|sources| sources
        .iter()
        .any(|source| source == "src/mct_symbol_bundle.n")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-optimization-policy")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-material-cfg-transform-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-ssa-interprocedural-dce-transform-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-static-trip-loop-transform-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-induction-strength-reduction-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-licm-derived-induction-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-private-leaf-inlining-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-linear-parameter-inlining-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-bounded-argument-rematerialization-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-observable-temporary-inlining-engine-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit == "mct-msf7-pdb-writer")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct_llvm.n")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/mct_pdb.n")));
    assert!(compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-lto-pgo-transform-engine")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-global-ssa-gvn-lto-pgo-transform-engine")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-ir-transform-engine")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-optimization-pdb-writer")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-final-wasi-image-writer")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit
            .as_str()
            .is_some_and(|name| name.contains("final-executable-relocation")))));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "project-final-symbol-resolution-plan")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "project-cross-unit-owner-visibility-and-relocatable-objects")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-owner-qualified-cross-unit-inlining-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "mct-llvm-linked-graph-ssa-gvn-all-targets")));
    assert!(compiler["migration"]["native_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "project-unit-symbol-scope-without-local-leakage")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/project.n")));
    assert!(compiler["sources"]
        .as_array()
        .is_some_and(|sources| sources.iter().any(|source| source == "src/sha256.n")));
    assert!(compiler["sources"].as_array().is_some_and(|sources| sources
        .iter()
        .any(|source| source == "src/pattern_async_resource.n")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-pattern-async-and-resource-contracts")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-import-runtime-and-modal-contracts")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-advanced-runtime-signatures-and-context-capabilities")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units
            .iter()
            .any(|unit| unit == "sema-advanced-runtime-signatures")));
    assert!(!compiler["migration"]["stage0_units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit == "sema")));
    let declaration_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-parser-ast-declarations-json")
        .expect("retained declaration AST JSON case");
    assert_eq!(
        declaration_json["expected_output_sha256"],
        "7c51f2539c9885fec5b11678d0ebe39a3083b5f469ee32cd5e96bbac70ae4ee0"
    );
    assert_eq!(
        declaration_json["expected_output_sha256_windows"],
        "d86ab7a8d3b2eca0ff97ce83d7cedae3d727e986dd0a167121b1805664ca1eff"
    );
    let material_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-parser-ast-material-json")
        .expect("retained material AST JSON case");
    assert_eq!(
        material_json["expected_output_sha256"],
        "5f1e31f511e76305e9c42e8c6d3df0113c8f3a990730346818f843d321d46d5c"
    );
    assert_eq!(
        material_json["expected_output_sha256_windows"],
        "1550430e7d21f5653771fd5d6f09fb168c3c4de2fd4b90de11bd73f9e6b56dd1"
    );
    let statement_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-parser-ast-statements-json")
        .expect("retained statement AST JSON case");
    assert_eq!(
        statement_json["expected_output_sha256"],
        "b4d3acf74872281ccab083069f1a27599c8143646f0445201dc38312d88f5257"
    );
    assert_eq!(
        statement_json["expected_output_sha256_windows"],
        "cd0d016d648dd0c343b788cc2f362f11639550443568140ec20d9556debb0302"
    );
    let advanced_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-parser-ast-advanced-json")
        .expect("retained advanced AST JSON case");
    assert_eq!(
        advanced_json["expected_output_sha256"],
        "82fb4aa22e380f377fc058d6f2ef3b725313f43fa790e08a8929e6bab29a7b79"
    );
    assert_eq!(
        advanced_json["expected_output_sha256_windows"],
        "a0a781cc0711ced0233bb99a2e0cc3bf1f15f30567c9649fedb5f9f7c6cb7864"
    );
    let sema_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-sema-core-json")
        .expect("retained core semantic JSON case");
    assert_eq!(
        sema_json["expected_output_sha256"],
        "1720f6f03c87d5575fc530b8e8b1379bfc485db9711b8721e417178f868e2923"
    );
    assert_eq!(
        sema_json["expected_output_sha256_windows"],
        "ad2de6804b331f967307fcca830cc42ecf768117d42eec7292eb31b08946cd86"
    );
    let authority_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-sema-authority-catalog-json")
        .expect("retained authority catalog JSON case");
    assert_eq!(
        authority_json["expected_output_sha256"],
        "4d1ac51ff3f23b65b0b9e07842bb729be2ad2067473afa8d628994a879e06e95"
    );
    assert_eq!(
        authority_json["expected_output_sha256_windows"],
        "8df946761cf81cbc1cf196cfc36548b06aceaaeb0310d5e4d49c214e95299795"
    );
    let signature_json = corpus
        .iter()
        .find(|case| case["name"] == "ncc-runtime-signatures-catalog-json")
        .expect("retained runtime signature JSON case");
    assert_eq!(
        signature_json["expected_output_sha256"],
        "205978b98c91f43426c8c8426c375775e650cbace57ca91b41f801862475a42e"
    );
    assert_eq!(
        signature_json["expected_output_sha256_windows"],
        "f3d660f3c6c6c2018d881e0de8a58ff50d3e0ca3896f0444a6436155de6e68a9"
    );
    let signature_calls = corpus
        .iter()
        .find(|case| case["name"] == "ncc-sema-runtime-signatures-json")
        .expect("retained typed runtime call JSON case");
    assert_eq!(
        signature_calls["expected_output_sha256"],
        "57f41fa9e79ceb7dce6bab01282fe84090c5565c8cbd35efac0b7fa285b33054"
    );
    assert_eq!(
        signature_calls["expected_output_sha256_windows"],
        "94b7569c152f5553022a9e20c44cbd795c661ac1acaf7fca99e5ff7b0b847ca7"
    );
    let generic_abi = corpus
        .iter()
        .find(|case| case["name"] == "ncc-sema-generic-trait-abi-json")
        .expect("retained generic/trait/ABI JSON case");
    assert_eq!(
        generic_abi["expected_output_sha256"],
        "ec663f21f5f0da1400ee75aff97efc9e23e827e4d3ea783766b7a794cb86aa9d"
    );
    assert_eq!(
        generic_abi["expected_output_sha256_windows"],
        "e1df02e07e8905be50bd040ec09f0d1e5d86a69331f1c21f8880f9a58c2d876a"
    );
    let performance = plan["performance"]
        .as_array()
        .expect("generation performance gate");
    assert!(performance.len() >= 2);
    assert!(performance.iter().all(|case| {
        case["reference"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
            && case["component"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && case["warmup"].as_i64().is_some_and(|value| value >= 1)
            && case["iterations"].as_i64().is_some_and(|value| value >= 5)
            && case["max_regression_percent"]
                .as_i64()
                .is_some_and(|value| (0..=5).contains(&value))
    }));
    assert_eq!(
        plan["promotion"]["policy"],
        "all-sources-builds-artifacts-corpus-performance"
    );
}

#[test]
fn promotion_fixtures_satisfy_every_generation_gate() {
    for fixture in [
        "bootstrap/fixtures/stage2-ready-generation-report.json",
        "bootstrap/fixtures/stage3-ready-generation-report.json",
    ] {
        let report = json(fixture);
        assert_eq!(report["schema"], "ghor.bootstrap.generation-report.v1");
        assert_eq!(report["status"], "ready");
        for gate in [
            "sources_complete",
            "migration_complete",
            "builds_complete",
            "artifacts_equal",
            "corpus_complete",
            "performance_complete",
            "stage3_ready",
            "rust_stage0_retirement_ready",
        ] {
            assert_eq!(report[gate], true, "{fixture}: gate {gate}");
        }
        assert_eq!(report["diagnostics"].as_array().map(Vec::len), Some(0));
        assert!(report["components"].as_array().is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row["source_ready"] == true
                        && row["migration_complete"] == true
                        && row["built"] == true
                })
        }));
        assert!(report["artifacts"].as_array().is_some_and(|rows| {
            !rows.is_empty() && rows.iter().all(|row| row["equal"] == true)
        }));
        assert!(report["corpus"].as_array().is_some_and(|rows| {
            !rows.is_empty() && rows.iter().all(|row| row["passed"] == true)
        }));
        assert!(report["performance"].as_array().is_some_and(|rows| {
            !rows.is_empty() && rows.iter().all(|row| row["passed"] == true)
        }));
    }
}

#[test]
fn nc_json_runtime_uses_tokens_instead_of_truncated_addresses() {
    let source =
        fs::read_to_string(root().join("runtime/nc/json.n")).expect("read native nC JSON provider");
    assert!(source.contains("handles >> p<addr>"));
    assert!(source.contains("nextHandle >> int"));
    assert!(source.contains("ensureHandleCapacity"));
    assert!(source.contains("handleWithMode"));
    assert!(source.contains("borrowedHandleOf"));
    assert!(source.contains("releaseHandle"));
    assert!(!source.contains("node >> p<byte> <- cast<byte>(handle);"));
}

#[test]
fn ghor_generation_module_is_in_the_native_project_authority() {
    let bootstrap = json("tools/ghor/nc/bootstrap.json");
    assert!(bootstrap["stage1"]["modules"]
        .as_array()
        .is_some_and(|modules| modules
            .iter()
            .any(|module| module == "src/bootstrap_generation.n")));
    let source = fs::read_to_string(root().join("tools/ghor/nc/src/bootstrap_generation.n"))
        .expect("read generation module");
    for command in ["build", "verify", "promote", "rollback", "status"] {
        assert!(
            source.contains(&format!("action == \"{command}\"")),
            "native generation module omitted {command}"
        );
    }
    assert!(source.contains("expected_output_sha256"));
    assert!(source.contains("option(handle, \"--linker\")"));
    assert!(source.contains("listAdd(linkArgs, \"--linker\")"));
    assert!(source.contains("candidate >> str <- candidatePath(artifactDirectory, name, target)"));
    assert!(source.contains("\".candidate.exe\""));
    assert!(!source.contains("candidate >> str <- output + \".next\""));
    assert!(source.contains("publishArtifact(candidate, output, name, diagnostics)"));
    assert!(source.contains("if (candidateReady && smokeReady)"));
    assert!(!source.contains("candidateReady && smokeReady &&"));
    assert!(source.contains("backup >> str <- output + \".previous\""));
    assert!(source.contains("stale component candidate could not be removed"));
    assert!(!source.contains("exists(output) && !delete_file(output)"));
    assert!(!source.contains("stale component output could not be removed"));
    assert!(source.contains("expected_output_sha256_windows"));
    assert!(source.contains("expectedOutputHash"));
    assert!(source.contains("smokeDependencyReady(dependency >> int)"));
    assert!(source.contains("jsonObjectGet(dependency, \"min_bytes\")"));
    assert!(source.contains("jsonObjectGet(dependency, \"bytes\")"));
    assert!(source.contains("jsonObjectGet(dependency, \"sha256\")"));
    assert!(source.contains("sha256.sha256.file(path) == expectedHash"));
    assert!(source.contains("output_matches"));
    assert!(source.contains("assurance.assurance.captureArguments"));
    assert!(source.contains("argumentCount >> int <- jsonArrayLen(arguments)"));
    assert!(source.contains("listCreateWithCapacity<str>(argumentCount)"));
    assert!(!source.contains("values >> list<str> <- listCreateWithCapacity<str>(8)"));
    assert!(source.contains("executePerformance(manifest >> int, diagnostics >> int)"));
    assert!(source.contains("candidateMs * 100 <= normalizedReference * (100 + regression)"));
    assert!(source.contains("performance_complete"));
    assert!(source.contains("allRows(performance, \"passed\")"));
}
