use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn final_certification_matrix_covers_material_execution_and_stress_contracts() {
    let root = workspace_root();
    let matrix_path = root.join("tools/certification/final-target-matrix.v1.json");
    let matrix: Value = serde_json::from_slice(
        &std::fs::read(&matrix_path).expect("final certification matrix must exist"),
    )
    .expect("final certification matrix must be JSON");

    assert_eq!(matrix["schema"], "ncc.final_certification.target_matrix.v1");
    assert_eq!(
        matrix["required_pipeline_phases"],
        serde_json::json!([
            "parser",
            "sema",
            "typed_ir",
            "build",
            "llvm_ir",
            "machine_ir",
            "object",
            "object_verify",
            "disassembly",
            "link",
            "execution"
        ])
    );
    assert_eq!(
        matrix["stress_families"],
        serde_json::json!([
            "heap-realloc",
            "fragmentation",
            "concurrent-atomics",
            "vm-exit-storm",
            "smp-contention",
            "cow-page-faults",
            "invalid-dma",
            "simultaneous-interrupts",
            "snapshot-restore"
        ])
    );

    let routes = matrix["routes"]
        .as_array()
        .expect("routes must be an array");
    assert_eq!(routes.len(), 16, "the declared target surface changed");
    let ids = routes
        .iter()
        .map(|route| route["id"].as_str().expect("route id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), routes.len(), "route ids must remain unique");

    let targets = routes
        .iter()
        .map(|route| route["target"].as_str().expect("route target"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        targets.len(),
        routes.len(),
        "each target must have one authoritative certification route"
    );
    for required in [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "i386-unknown-linux-gnu",
        "riscv64-unknown-linux-gnu",
        "x86_64-unknown-none",
        "i386-unknown-none-elf",
        "aarch64-unknown-none",
        "armv7a-unknown-none-eabi",
        "armv7em-none-eabi",
        "riscv32imac-unknown-elf",
        "riscv64-unknown-elf",
        "wasm32-wasi",
        "wasm32-unknown-unknown",
    ] {
        assert!(
            targets.contains(required),
            "missing target route {required}"
        );
    }

    for route in routes {
        let source = route["source"].as_str().expect("route source");
        assert!(root.join(source).is_file(), "missing route source {source}");
        if route["physical_required"] == true {
            assert_eq!(route["profile"], "kernel");
            assert_ne!(route["architecture"], "wasm32");
        }
    }
}

#[test]
fn final_certifier_enforces_exact_artifact_bound_physical_evidence() {
    let root = workspace_root();
    let read_module = |name: &str| {
        std::fs::read_to_string(
            root.join(format!("tools/ncc/nc/src/verification/physical43_{name}.n")),
        )
        .unwrap_or_else(|error| panic!("native certifier module {name} must exist: {error}"))
    };
    let stages = read_module("stages");
    let routes = read_module("routes");
    let boards = read_module("boards");
    let final_pipeline = read_module("final");
    let script = [
        stages.as_str(),
        routes.as_str(),
        boards.as_str(),
        final_pipeline.as_str(),
    ]
    .join("\n");
    for required in [
        "emit-ast",
        "emit-ir",
        "mct",
        "machine",
        "verify-object",
        "disasm",
        "raw-memory-stress",
        "verify-raw-memory-stress",
        "verify-stress",
        "verify-board-certificate",
        "artifact_bound_to_route",
        "linked_sha256",
        "require-exact-head",
        "require-execution",
        "require-physical",
        "E_FINAL_CERT_QEMU_PLACEHOLDER_001",
    ] {
        assert!(
            script.contains(required),
            "certifier lost contract {required}"
        );
    }

    let phase_literal = stages
        .lines()
        .find(|line| line.trim_start().starts_with("PHASES >> str <-"))
        .expect("native certifier must declare its complete phase contract")
        .split_once("<-")
        .unwrap()
        .1
        .trim()
        .trim_end_matches(';');
    let phase_json: String = serde_json::from_str(phase_literal).expect("nC JSON string literal");
    let phases: Value = serde_json::from_str(&phase_json).expect("native phase array");
    let matrix: Value = serde_json::from_slice(
        &std::fs::read(root.join("tools/certification/final-target-matrix.v1.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(phases, matrix["required_pipeline_phases"]);
    assert!(routes.contains("required >> int <- physical43_stages.physical43Stages.phases()"));
    assert!(routes.contains("at < jsonArrayLen(required)-1"));
    assert!(routes.contains("if (get(row,\"status\") == \"failed\") failed <- true"));
    assert!(routes.contains("material <- material && passed && !failed"));
    assert!(boards.contains("/artifact/sha256\") == get(route,\"linked_sha256\")"));
    assert!(boards.contains("if (!exact || !verified)"));

    let dispatcher = read_module("cli");
    let alias_line = dispatcher
        .lines()
        .find(|line| line.contains("command == \"certify-raw-optimizer\""))
        .expect("raw optimizer alias must remain supported");
    for required in [
        "command == \"certify-final-pipeline\"",
        "command == \"certify_raw_optimizer.py\"",
        "return physical43_final.physical43Final.execute(arguments)",
    ] {
        assert!(
            alias_line.contains(required),
            "alias lost delegation {required}"
        );
    }
    let entry = std::fs::read_to_string(root.join("tools/tests/physical43_main.n")).unwrap();
    assert!(entry.contains("command <- \"certify-final-pipeline\""));
    assert!(entry.contains("physical43_cli.physical43Cli.execute(arguments,command)"));
    let build: Value = serde_json::from_slice(
        &std::fs::read(root.join("bootstrap/physical43-build.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(build["entry"], "tools/tests/physical43_main.n");
    assert_eq!(
        build["aliases"]["certify-raw-optimizer"],
        "certify-final-pipeline"
    );
}

#[test]
fn normal_wasm_build_retains_target_llvm_and_per_source_machine_ir() {
    let root = workspace_root();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let out = std::env::temp_dir().join(format!(
        "ncc_final_wasm_contract_{}_{}",
        std::process::id(),
        nonce
    ));
    let source = root.join("examples/01_basics/root_lineage.n");
    let source_text = source.to_string_lossy().into_owned();
    let out_text = out.to_string_lossy().into_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_ncc"))
        .args([
            "build",
            source_text.as_str(),
            "--target",
            "wasm32-wasi",
            "--profile",
            "release-fast",
            "--out-dir",
            out_text.as_str(),
            "--json",
        ])
        .output()
        .expect("run ncc wasm build");
    assert!(
        output.status.success(),
        "wasm build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let llvm = out.join("lowering/root_lineage.wasm.mct.ll");
    let machine = out.join("lowering/root_lineage.nm");
    let manifest = out.join("run/root_lineage.wasm-build.json");
    assert!(llvm.is_file() && std::fs::metadata(&llvm).unwrap().len() > 0);
    assert!(machine.is_file() && std::fs::metadata(&machine).unwrap().len() > 0);
    let manifest: Value = serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    assert_eq!(manifest["schema"], "nc.compiler.standard_wasm_build.v2");
    assert_eq!(
        PathBuf::from(manifest["paths"]["llvm_ir"].as_str().unwrap()),
        llvm
    );
    assert_eq!(
        PathBuf::from(manifest["paths"]["machine_source"].as_str().unwrap()),
        machine
    );
    assert_eq!(
        manifest["truth_boundary"]["machine_source_is_per_input_and_validated"],
        true
    );

    std::fs::remove_dir_all(out).expect("remove wasm contract output");
}
