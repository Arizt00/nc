use super::super::*;
use super::mct_json::*;
use super::mct_support::*;

fn build_mct_backend_point10_parity_payload() -> serde_json::Value {
    let diagnostics = vec![
        serde_json::json!({
            "code": "E_BACKEND_PARITY_MCT_BYTES_001",
            "severity": "error",
            "message": "MCT byte contract missing or non-deterministic for a declared backend route"
        }),
        serde_json::json!({
            "code": "E_BACKEND_PARITY_LLVM_EXEC_001",
            "severity": "error",
            "message": "LLVM executable route could not emit, link or run the declared smoke without an explicit missing-tool classification"
        }),
        serde_json::json!({
            "code": "E_BACKEND_PARITY_NATIVE_FAST_001",
            "severity": "error",
            "message": "native-fast route did not execute the declared critical fixture through the native fast path"
        }),
        serde_json::json!({
            "code": "E_BACKEND_PARITY_MACHINE_AUTHORITY_001",
            "severity": "error",
            "message": "machine authority route failed to validate or encode low-level authority bytes"
        }),
        serde_json::json!({
            "code": "E_BACKEND_PARITY_DIVERGENCE_001",
            "severity": "error",
            "message": "backend routes disagree on the declared observable result for the same fixture"
        }),
        serde_json::json!({
            "code": "E_BACKEND_TOOLCHAIN_MISSING_001",
            "severity": "warning",
            "message": "external LLVM/native toolchain is missing; the route is reported as missing environment, never as fake success"
        }),
    ];

    let routes = vec![
        serde_json::json!({
            "id": "mct_bytes_contract",
            "order": 1,
            "provider": "MCT",
            "status": "closed_real_activation_bytes",
            "purpose": "MCT is the first truth layer: every declared import/backend activation row must expose deterministic bytes before higher routes claim execution.",
            "commands": ["ncc mct imports --json", "ncc imports backend --json"],
            "evidence": [
                "UseBackendLoweringManifest.emitted_bytes",
                "UseBackendLoweringManifest.runtime_symbols",
                "closed_real_backing_bytes for runtime-backed imports",
                "profiles: nC.sysv64, nC.win64, nC.aapcs64, nC.portable",
                "diagnostic E_USE_BACKING_DEFERRED_001 for catalog-only imports"
            ],
            "required_diagnostic_if_broken": "E_BACKEND_PARITY_MCT_BYTES_001"
        }),
        serde_json::json!({
            "id": "llvm_executables",
            "order": 2,
            "provider": "LLVM through MCT",
            "status": "closed_for_declared_executable_subset",
            "purpose": "LLVM emits real textual IR, then clang links and runs hosted executables when the host toolchain exists.",
            "commands": [
                "ncc mct emit-llvm examples/backend/mct_llvm_array_executable.n --out target/tmp/mct-point10/array.ll --cc nC.sysv64",
                "ncc mct llvm-exe-smoke examples/backend/mct_llvm_array_executable.n --out-dir target/tmp/mct-point10 --json --cc nC.sysv64 --allow-missing-tools"
            ],
            "evidence": [
                "real .ll text",
                "clang -x ir executable smoke",
                "pass pipeline, debug metadata and target attributes in MCT JSON"
            ],
            "required_diagnostic_if_broken": "E_BACKEND_PARITY_LLVM_EXEC_001",
            "missing_toolchain_diagnostic": "E_BACKEND_TOOLCHAIN_MISSING_001"
        }),
        serde_json::json!({
            "id": "native_fast_critical_paths",
            "order": 3,
            "provider": "ncc-native/native-fast",
            "status": "closed_for_critical_runtime_speed_paths",
            "purpose": "native-fast owns direct execution for scalar, arrays, pointers, structs, calls and critical memory fixtures.",
            "commands": [
                "ncc run examples/performance/native_fast/return_7.n --native-fast --show-return",
                "ncc run examples/performance/native_fast/memory_ptr_read_write_widths.n --native-fast --show-return"
            ],
            "evidence": [
                "route=native-fast-path",
                "native_fast_path integration tests",
                "real array/pointer/struct fixtures under examples/performance/native_fast"
            ],
            "required_diagnostic_if_broken": "E_BACKEND_PARITY_NATIVE_FAST_001"
        }),
        serde_json::json!({
            "id": "machine_low_authority",
            "order": 4,
            "provider": "machine",
            "status": "closed_for_low_authority_native_bytes",
            "purpose": "machine remains the low-authority layer for typed native instructions, validation and encoded bytes.",
            "commands": [
                "ncc machine check examples/machine_at/machine_at_nop_wait_halt_ok.n",
                "ncc machine emit-bytes examples/machine_at/machine_at_nop_wait_halt_ok.n --json"
            ],
            "evidence": [
                "machine parser/auditor diagnostics",
                "encoded bytes per target where supported",
                "machine authority rejects runtime imports"
            ],
            "required_diagnostic_if_broken": "E_BACKEND_PARITY_MACHINE_AUTHORITY_001"
        }),
        serde_json::json!({
            "id": "cross_route_parity_tests",
            "order": 5,
            "provider": "test matrix",
            "status": "closed_for_declared_current_surface",
            "purpose": "parity tests compare route contracts and observable values where routes overlap, while preserving honest missing-tool diagnostics.",
            "commands": [
                "cargo test -p ncc-cli --test backend_point10 -- --nocapture",
                "cargo test -p ncc-cli --test native_fast_path -- --nocapture",
                "cargo test -p ncc-cli --test compact_surface -- --nocapture"
            ],
            "evidence": [
                "MCT parity manifest",
                "LLVM IR emission checks",
                "native-fast observable return checks",
                "machine authority diagnostics"
            ],
            "required_diagnostic_if_broken": "E_BACKEND_PARITY_DIVERGENCE_001"
        }),
    ];

    serde_json::json!({
        "schema": "nc.backend.point10.parity.v1",
        "status": "backend_point10_closed_for_current_surface",
        "order": [
            "MCT as byte contract",
            "LLVM for executables",
            "native-fast for critical paths",
            "machine for low authority",
            "parity tests across declared overlaps"
        ],
        "routes": routes,
        "diagnostics": diagnostics,
        "truth_boundary": {
            "mct_bytes_contract": true,
            "llvm_executable_route": true,
            "native_fast_critical_route": true,
            "machine_low_authority_route": true,
            "parity_tests_declared": true,
            "full_llvm_universe_parity": false,
            "full_gcc_universe_parity": false,
            "full_rustc_universe_parity": false,
            "missing_tools_are_reported_not_faked": true
        },
        "docs": [
            "docs/spec/backend/backend_point10_real_backend_closure_v0_1.md",
            "docs/spec/backend/runtime_backend_parity_priority10_contract.md",
            "docs/spec/backend/backend_mct.md",
            "docs/spec/backend/backend_llvm_mct.md",
            "docs/spec/backend/abi_lowering_contracts_v0_1.md"
        ]
    })
}

pub(crate) fn run_mct_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || (args.len() == 1 && is_help_token(&args[0])) {
        println!("{}", command_help("mct").expect("mct help must exist"));
        return Ok(());
    }

    let subcommand = args[0].as_str();
    let json = args.iter().any(|arg| arg == "--json");
    match subcommand {
        "doctor" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" | "-h" | "--help" | "help" => {}
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct doctor: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let payload = build_mct_contract_manifest_json();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("mct doctor: contract-ready");
                println!("ncc-native: integrated");
                println!("llvm: adapter integrated with owned Clang/opt pass pipeline");
                println!("llvm F8: LTO/PGO/debug metadata application closed via clang/opt");
                println!("gcc: external bridge contract defined");
                println!("rustc: planned extern-C adapter");
                println!("full LLVM/GCC/rustc parity: not claimed");
            }
            Ok(())
        }
        "manifest" | "providers" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" | "--list" | "-h" | "--help" | "help" => {}
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct manifest: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let payload = build_mct_contract_manifest_json();
            if args.iter().any(|arg| arg == "--list") {
                let providers = payload
                    .get("providers")
                    .and_then(|value| value.as_array())
                    .ok_or("mct manifest internal error: providers array missing")?;
                for provider in providers {
                    let name = provider
                        .get("provider")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let status = provider
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    println!("{name}\t{status}");
                }
            } else if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct manifest: {}",
                    payload["status"].as_str().unwrap_or("-")
                );
                println!("providers: ncc-native, llvm, gcc, rustc");
            }
            Ok(())
        }
        "imports" | "import-lowering" | "import-backend" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" | "--list" | "-h" | "--help" | "help" => {}
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct imports: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let manifest = ncc_ast::use_backend_lowering_manifest_all();
            if json {
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            } else if args.iter().any(|arg| arg == "--list") {
                for group in manifest.groups {
                    println!(
                        "{}::{}\t{}\t{}",
                        match group.domain {
                            ncc_ast::UseDomain::Core => "core",
                            ncc_ast::UseDomain::Alloc => "alloc",
                            ncc_ast::UseDomain::Stoud => "stoud",
                            ncc_ast::UseDomain::Auth => "auth",
                            ncc_ast::UseDomain::Dom => "dom",
                            ncc_ast::UseDomain::LegacyBaseical => "legacy",
                        },
                        group.family,
                        group.entries.len(),
                        group.status
                    );
                }
            } else {
                println!("mct imports: {}", manifest.status);
                println!(
                    "families={} entries={} profiles=sysv64,win64,aapcs64,portable",
                    manifest.total_families, manifest.total_entries
                );
            }
            Ok(())
        }
        "parity" | "point10" | "backend-parity" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" | "--list" | "-h" | "--help" | "help" => {}
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct parity: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let payload = build_mct_backend_point10_parity_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if args.iter().any(|arg| arg == "--list") {
                if let Some(routes) = payload["routes"].as_array() {
                    for route in routes {
                        println!(
                            "{}\t{}\t{}",
                            route["order"].as_i64().unwrap_or_default(),
                            route["id"].as_str().unwrap_or("-"),
                            route["status"].as_str().unwrap_or("-")
                        );
                    }
                }
            } else {
                println!("mct parity: {}", payload["status"].as_str().unwrap_or("-"));
                println!("routes: mct, llvm, native-fast, machine, tests");
                println!("full LLVM/GCC/rustc universe parity: not claimed");
            }
            Ok(())
        }
        "llvm-attributes" | "attributes" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" | "-h" | "--help" | "help" => {}
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct llvm-attributes: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let payload = mct_llvm_attribute_catalog_to_json();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                let count = payload
                    .get("catalog_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                println!(
                    "mct llvm-attributes: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!("attributes: {count}");
                println!("source: {}", ncc_ir::MCT_LLVM_ATTRIBUTE_CATALOG_SOURCE);
            }
            Ok(())
        }
        "cpu-tuning" | "cpu" | "tuning" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }

            let mut target_filter: Option<String> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" | "--list" => idx += 1,
                    "--cc" | "--cpu" | "--microarch" | "--tune-profile" | "--tune"
                    | "--fine-profile" => {
                        if args.get(idx + 1).is_none() {
                            return Err(actionable_command_error(
                                "mct",
                                format!("mct cpu-tuning {} expects a value", args[idx]),
                            )
                            .into());
                        }
                        idx += 2;
                    }
                    "--target" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct cpu-tuning --target expects x86_64|aarch64|wasm32|all",
                            )
                            .into());
                        };
                        target_filter = Some(value.clone());
                        idx += 2;
                    }
                    other if other.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct cpu-tuning: unknown option '{}'", other),
                        )
                        .into());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!(
                                "mct cpu-tuning does not accept positional argument '{}'",
                                other
                            ),
                        )
                        .into());
                    }
                }
            }

            let matrix = ncc_ir::mct_cpu_tuning_matrix();
            let mut payload = mct_cpu_tuning_matrix_to_json(&matrix);

            if let Some(target) = target_filter.as_deref() {
                if !matches!(target, "all" | "x86_64" | "aarch64" | "wasm32") {
                    return Err(actionable_command_error(
                        "mct",
                        format!(
                            "mct cpu-tuning --target '{}' is not supported; expected x86_64, aarch64, wasm32 or all",
                            target
                        ),
                    )
                    .into());
                }
                payload["target_filter"] = serde_json::json!(target);
                if target != "all" {
                    payload["microarchitectures"] = serde_json::Value::Array(
                        matrix
                            .microarchitectures
                            .iter()
                            .filter(|profile| profile.target == target)
                            .map(mct_cpu_microarchitecture_to_json)
                            .collect(),
                    );
                    payload["scheduling_profiles"] = serde_json::Value::Array(
                        matrix
                            .scheduling_profiles
                            .iter()
                            .filter(|profile| profile.target == target)
                            .map(mct_cpu_scheduling_profile_to_json)
                            .collect(),
                    );
                }
            }

            if let Some((cpu, fine_profile)) = parse_mct_cpu_tuning_from_args(args)? {
                let microarchitecture =
                    ncc_ir::mct_cpu_microarchitecture_by_id(&cpu).ok_or_else(|| {
                        actionable_command_error(
                            "mct",
                            format!("unknown MCT CPU microarchitecture '{}'", cpu),
                        )
                    })?;
                let fine =
                    ncc_ir::mct_cpu_fine_tuning_profile_by_id(&fine_profile).ok_or_else(|| {
                        actionable_command_error(
                            "mct",
                            format!("unknown MCT fine tuning profile '{}'", fine_profile),
                        )
                    })?;
                let profile = if args.iter().any(|arg| arg == "--cc") {
                    parse_backend_profile_from_args(args)?
                } else {
                    match microarchitecture.target {
                        "x86_64" => ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
                        "aarch64" => ncc_ir::BackendLoweringProfileIR::nc_aapcs64(),
                        "wasm32" => ncc_ir::BackendLoweringProfileIR::nc_portable(),
                        _ => ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
                    }
                };
                let schedule = ncc_ir::mct_cpu_scheduling_profile(&microarchitecture, &fine);
                let attributes = ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
                    &profile.target_spec,
                    microarchitecture.id,
                    fine.id,
                )
                .map_err(|err| actionable_command_error("mct", err))?;
                let native_scheduler_bridge =
                    ncc_ir::backend_native_cpu_tuning_bridge_for_mct_cpu(microarchitecture.id)
                        .map_err(|err| actionable_command_error("mct", err))?;
                let native_selection_policy = ncc_ir::backend_native_cpu_tuning_policy_for_mct_cpu(
                    microarchitecture.id,
                    fine.id,
                )
                .map_err(|err| actionable_command_error("mct", err))?;

                payload["selected"] = serde_json::json!({
                    "microarchitecture": mct_cpu_microarchitecture_to_json(&microarchitecture),
                    "fine_tuning_profile": mct_cpu_fine_tuning_profile_to_json(&fine),
                    "scheduling_profile": mct_cpu_scheduling_profile_to_json(&schedule),
                    "target_attributes": mct_llvm_target_attributes_to_json(&attributes),
                    "native_scheduler_bridge": backend_native_cpu_tuning_bridge_to_json(&native_scheduler_bridge),
                    "native_selection_policy": backend_native_cpu_tuning_policy_to_json(&native_selection_policy),
                });
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if args.iter().any(|arg| arg == "--list") {
                let rows = payload
                    .get("microarchitectures")
                    .and_then(|value| value.as_array())
                    .ok_or("mct cpu-tuning internal error: microarchitectures array missing")?;
                for row in rows {
                    let id = row
                        .get("id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let target = row
                        .get("target")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let cpu = row
                        .get("llvm_target_cpu")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let scheduler = row
                        .get("scheduling_model")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    println!("{id}\t{target}\t{cpu}\t{scheduler}");
                }
            } else {
                println!("mct cpu-tuning: {}", matrix.status);
                println!(
                    "microarchitectures={} fine_profiles={} schedules={}",
                    matrix.microarchitecture_count,
                    matrix.fine_tuning_profile_count,
                    matrix.scheduling_profile_count
                );
                if let Some(selected) = payload.get("selected") {
                    let id = selected
                        .get("microarchitecture")
                        .and_then(|value| value.get("id"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let profile = selected
                        .get("fine_tuning_profile")
                        .and_then(|value| value.get("id"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    println!("selected: {id} / {profile}");
                }
                println!("full LLVM CPU universe: not claimed");
            }
            Ok(())
        }
        "cpu-tuning-smoke" | "cpu-smoke" | "tuning-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut allow_missing_tools = false;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--allow-missing-tools" => {
                        allow_missing_tools = true;
                        idx += 1;
                    }
                    "--cc" | "--cpu" | "--microarch" | "--tune-profile" | "--tune"
                    | "--fine-profile" | "--opt-level" | "-O" => {
                        if args.get(idx + 1).is_none() {
                            return Err(actionable_command_error(
                                "mct",
                                format!("mct cpu-tuning-smoke {} expects a value", args[idx]),
                            )
                            .into());
                        }
                        idx += 2;
                    }
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct cpu-tuning-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    other if other.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct cpu-tuning-smoke: unknown option '{}'", other),
                        )
                        .into());
                    }
                    other => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct cpu-tuning-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(other);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct cpu-tuning-smoke expects <file.n>")
            })?;
            let (cpu, fine_profile) = parse_mct_cpu_tuning_from_args(args)?
                .unwrap_or_else(|| ("intel-skylake".to_string(), "baseline".to_string()));
            let microarchitecture =
                ncc_ir::mct_cpu_microarchitecture_by_id(&cpu).ok_or_else(|| {
                    actionable_command_error(
                        "mct",
                        format!("unknown MCT CPU microarchitecture '{}'", cpu),
                    )
                })?;
            let profile = if args.iter().any(|arg| arg == "--cc") {
                parse_backend_profile_from_args(args)?
            } else {
                match microarchitecture.target {
                    "x86_64" => ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
                    "aarch64" => ncc_ir::BackendLoweringProfileIR::nc_aapcs64(),
                    "wasm32" => ncc_ir::BackendLoweringProfileIR::nc_portable(),
                    _ => ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
                }
            };
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-f12")
                    .join("cpu-tuning-smoke")
            });
            let payload = run_mct_cpu_tuning_object_disasm_smoke(
                path,
                out_dir,
                profile,
                pass_pipeline,
                &cpu,
                &fine_profile,
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct cpu-tuning-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "object: {}",
                    payload
                        .get("object_path")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "disasm: {}",
                    payload
                        .get("disasm_path")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
            }
            let status = payload
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status == "mct_cpu_tuning_object_disasm_failed"
                || (status == "mct_cpu_tuning_object_disasm_missing_tools" && !allow_missing_tools)
            {
                return Err(actionable_command_error(
                    "mct",
                    "mct cpu-tuning-smoke did not complete object/disasm evidence",
                )
                .into());
            }
            Ok(())
        }
        "cpu-tuning-matrix-smoke" | "cpu-matrix-smoke" | "tuning-matrix-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut allow_missing_tools = false;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" | "--all-targets" => idx += 1,
                    "--allow-missing-tools" => {
                        allow_missing_tools = true;
                        idx += 1;
                    }
                    "--cc" | "--opt-level" | "-O" => {
                        if args.get(idx + 1).is_none() {
                            return Err(actionable_command_error(
                                "mct",
                                format!(
                                    "mct cpu-tuning-matrix-smoke {} expects a value",
                                    args[idx]
                                ),
                            )
                            .into());
                        }
                        idx += 2;
                    }
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct cpu-tuning-matrix-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    other if other.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct cpu-tuning-matrix-smoke: unknown option '{}'", other),
                        )
                        .into());
                    }
                    other => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct cpu-tuning-matrix-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(other);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct cpu-tuning-matrix-smoke expects <file.n>")
            })?;
            let selected_profile = if args.iter().any(|arg| arg == "--cc") {
                Some(parse_backend_profile_from_args(args)?)
            } else {
                None
            };
            let rows = mct_cpu_tuning_object_disasm_matrix_rows(selected_profile);
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-f12")
                    .join("cpu-tuning-matrix-smoke")
            });
            let payload =
                run_mct_cpu_tuning_object_disasm_matrix_smoke(path, out_dir, rows, pass_pipeline)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct cpu-tuning-matrix-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                if let Some(summary) = payload.get("summary") {
                    println!("summary: {}", serde_json::to_string(summary)?);
                }
            }
            let status = payload
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status == "mct_cpu_tuning_object_disasm_matrix_failed"
                || (status == "mct_cpu_tuning_object_disasm_matrix_missing_tools"
                    && !allow_missing_tools)
            {
                return Err(actionable_command_error(
                    "mct",
                    "mct cpu-tuning-matrix-smoke did not complete every object/disasm row",
                )
                .into());
            }
            Ok(())
        }
        "pass-pipeline" | "optimizer" => {
            for (idx, arg) in args.iter().enumerate().skip(1) {
                match arg.as_str() {
                    "--json" | "-h" | "--help" | "help" => {}
                    "--opt-level" | "-O" => {}
                    _ if idx > 1
                        && matches!(
                            args.get(idx - 1).map(String::as_str),
                            Some("--opt-level" | "-O")
                        ) => {}
                    other if other.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct pass-pipeline: unknown option '{}'", other),
                        )
                        .into());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "mct",
                            format!(
                                "mct pass-pipeline does not accept positional argument '{}'",
                                other
                            ),
                        )
                        .into());
                    }
                }
            }
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let opt_level = parse_mct_llvm_optimization_level_from_args(args)?;
            let pipeline = ncc_ir::mct_llvm_pass_pipeline(opt_level);
            let payload = mct_llvm_pass_pipeline_to_json(&pipeline);
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct pass-pipeline: {} {}",
                    ncc_ir::render_mct_llvm_optimization_level(pipeline.level),
                    pipeline.status
                );
                println!("owner: {}", pipeline.owner);
                println!("clang: {}", pipeline.clang_opt_arg);
                println!("opt: {}", pipeline.opt_passes_arg);
                println!("passes: {}", pipeline.pass_count);
            }
            Ok(())
        }
        "emit-llvm" | "llvm-ir" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out: Option<PathBuf> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--cc" => idx += 2,
                    "--cpu" | "--microarch" | "--tune-profile" | "--tune" | "--fine-profile" => {
                        idx += 2
                    }
                    "--opt-level" | "-O" => idx += 2,
                    "--out" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct emit-llvm --out expects a file path",
                            )
                            .into());
                        };
                        out = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct emit-llvm: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct emit-llvm expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path
                .ok_or_else(|| actionable_command_error("mct", "mct emit-llvm expects <file.n>"))?;
            let profile = parse_backend_profile_from_args(args)?;
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let source = fs::read_to_string(path)?;
            let ast = load_ast_from_path(path)?;
            sema_check_with_warnings(&ast)?;
            let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile, path, &source);
            let target_attributes =
                if let Some((cpu, fine_profile)) = parse_mct_cpu_tuning_from_args(args)? {
                    ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
                        &ir.target_spec,
                        &cpu,
                        &fine_profile,
                    )
                    .map_err(|err| actionable_command_error("mct", err))?
                } else {
                    ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec)
                };
            let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
                &ir,
                pass_pipeline,
                mct_llvm_debug_config_for_path(path),
                target_attributes,
            );
            if module.emitted_function_count == 0 {
                let payload = mct_llvm_module_to_json(&module, true, out.as_deref());
                if json {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                let detail = module
                    .diagnostics
                    .iter()
                    .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(CliFailure::backend(
                    "mct.emit-llvm",
                    "materialize an LLVM module with executable functions",
                    out.as_deref(),
                    if detail.is_empty() {
                        "the backend emitted zero functions".to_string()
                    } else {
                        detail
                    },
                )
                .into());
            }
            if let Some(out_path) = out.as_ref() {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(out_path, module.llvm_ir.as_bytes())?;
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&mct_llvm_module_to_json(
                        &module,
                        out.is_none(),
                        out.as_deref()
                    ))?
                );
            } else if out.is_none() {
                print!("{}", module.llvm_ir);
            } else if let Some(out_path) = out.as_ref() {
                println!("mct emit-llvm: wrote {}", out_path.display());
                println!("status: {}", module.status);
                println!("functions: {}", module.emitted_function_count);
            }
            Ok(())
        }
        "llvm-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--cc" => idx += 2,
                    "--cpu" | "--microarch" | "--tune-profile" | "--tune" | "--fine-profile" => {
                        idx += 2
                    }
                    "--opt-level" | "-O" => idx += 2,
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct llvm-smoke: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct llvm-smoke expects <file.n>")
            })?;
            let profile = parse_backend_profile_from_args(args)?;
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let source = fs::read_to_string(path)?;
            let ast = load_ast_from_path(path)?;
            sema_check_with_warnings(&ast)?;
            let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile, path, &source);
            let target_attributes =
                if let Some((cpu, fine_profile)) = parse_mct_cpu_tuning_from_args(args)? {
                    ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
                        &ir.target_spec,
                        &cpu,
                        &fine_profile,
                    )
                    .map_err(|err| actionable_command_error("mct", err))?
                } else {
                    ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec)
                };
            let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
                &ir,
                pass_pipeline,
                mct_llvm_debug_config_for_path(path),
                target_attributes,
            );
            if module.emitted_function_count == 0 {
                let payload = serde_json::json!({
                    "schema": "nc.mct.llvm.smoke.v1",
                    "status": "llvm_smoke_rejected",
                    "module": mct_llvm_module_to_json(&module, true, None),
                    "clang": null,
                });
                if json {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                return Err(actionable_command_error(
                    "mct",
                    "mct llvm-smoke could not emit any LLVM function",
                )
                .into());
            }
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-llvm")
            });
            fs::create_dir_all(&out_dir)?;
            let stem = Path::new(path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("module");
            let ll_path = out_dir.join(format!("{stem}.ll"));
            let obj_path = out_dir.join(format!("{stem}.o"));
            fs::write(&ll_path, module.llvm_ir.as_bytes())?;
            let mut clang_args = vec![
                format!("--target={}", module.target_triple),
                module.pass_pipeline.clang_opt_arg.to_string(),
            ];
            if module.debug_metadata.enabled {
                clang_args.push("-g".to_string());
            }
            clang_args.extend([
                "-x".to_string(),
                "ir".to_string(),
                "-c".to_string(),
                ll_path.display().to_string(),
                "-o".to_string(),
                obj_path.display().to_string(),
            ]);
            let mut clang_command = Command::new("clang");
            clang_command
                .arg(format!("--target={}", module.target_triple))
                .arg(module.pass_pipeline.clang_opt_arg);
            if module.debug_metadata.enabled {
                clang_command.arg("-g");
            }
            let clang_output = clang_command
                .arg("-x")
                .arg("ir")
                .arg("-c")
                .arg(&ll_path)
                .arg("-o")
                .arg(&obj_path)
                .output();
            let payload = match clang_output {
                Ok(output) => {
                    let passed = output.status.success();
                    serde_json::json!({
                        "schema": "nc.mct.llvm.smoke.v1",
                        "status": if passed { "llvm_smoke_passed" } else { "llvm_smoke_failed" },
                        "ll_path": ll_path.display().to_string(),
                        "object_path": obj_path.display().to_string(),
                        "object_exists": obj_path.exists(),
                        "clang_args": clang_args,
                        "clang": {
                            "available": true,
                            "status_code": output.status.code(),
                            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        },
                        "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
                    })
                }
                Err(err) => serde_json::json!({
                    "schema": "nc.mct.llvm.smoke.v1",
                    "status": "llvm_smoke_missing_clang",
                    "ll_path": ll_path.display().to_string(),
                    "object_path": obj_path.display().to_string(),
                    "object_exists": false,
                    "clang_args": clang_args,
                    "clang": {
                        "available": false,
                        "error": err.to_string(),
                    },
                    "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
                }),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct llvm-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!("ll: {}", ll_path.display());
                println!("object: {}", obj_path.display());
            }
            if payload.get("status").and_then(|value| value.as_str()) != Some("llvm_smoke_passed") {
                return Err(actionable_command_error("mct", "mct llvm-smoke failed").into());
            }
            Ok(())
        }
        "llvm-exe-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--cc" => idx += 2,
                    "--cpu" | "--microarch" | "--tune-profile" | "--tune" | "--fine-profile" => {
                        idx += 2
                    }
                    "--opt-level" | "-O" => idx += 2,
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-exe-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct llvm-exe-smoke: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-exe-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct llvm-exe-smoke expects <file.n>")
            })?;
            let profile = parse_backend_profile_from_args(args)?;
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let source = fs::read_to_string(path)?;
            let ast = load_ast_from_path(path)?;
            sema_check_with_warnings(&ast)?;
            let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile, path, &source);
            let target_attributes =
                if let Some((cpu, fine_profile)) = parse_mct_cpu_tuning_from_args(args)? {
                    ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
                        &ir.target_spec,
                        &cpu,
                        &fine_profile,
                    )
                    .map_err(|err| actionable_command_error("mct", err))?
                } else {
                    ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec)
                };
            let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
                &ir,
                pass_pipeline,
                mct_llvm_debug_config_for_path(path),
                target_attributes,
            );
            if module.emitted_function_count == 0 || !module.llvm_ir.contains("define i32 @main()")
            {
                let payload = serde_json::json!({
                    "schema": "nc.mct.llvm.exe_smoke.v1",
                    "status": "llvm_exe_smoke_rejected",
                    "module": mct_llvm_module_to_json(&module, true, None),
                    "clang": null,
                    "run": null,
                });
                if json {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                return Err(actionable_command_error(
                    "mct",
                    "mct llvm-exe-smoke requires emitted functions and a main ref",
                )
                .into());
            }
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-llvm")
            });
            fs::create_dir_all(&out_dir)?;
            let stem = Path::new(path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("module");
            let ll_path = out_dir.join(format!("{stem}.ll"));
            let exe_name = if module.target_triple.contains("windows") {
                format!("{stem}.exe")
            } else {
                stem.to_string()
            };
            let exe_path = out_dir.join(exe_name);
            fs::write(&ll_path, module.llvm_ir.as_bytes())?;
            let mut clang_args = vec![
                format!("--target={}", module.target_triple),
                module.pass_pipeline.clang_opt_arg.to_string(),
            ];
            if module.debug_metadata.enabled {
                clang_args.push("-g".to_string());
            }
            clang_args.extend([
                "-x".to_string(),
                "ir".to_string(),
                ll_path.display().to_string(),
                "-o".to_string(),
                exe_path.display().to_string(),
            ]);
            let mut clang_command = Command::new("clang");
            clang_command
                .arg(format!("--target={}", module.target_triple))
                .arg(module.pass_pipeline.clang_opt_arg);
            if module.debug_metadata.enabled {
                clang_command.arg("-g");
            }
            let clang_output = clang_command
                .arg("-x")
                .arg("ir")
                .arg(&ll_path)
                .arg("-o")
                .arg(&exe_path)
                .output();
            let payload = match clang_output {
                Ok(output) => {
                    let link_passed = output.status.success();
                    let run_output = if link_passed
                        && exe_path.exists()
                        && module.target_triple.contains("linux")
                    {
                        Command::new(&exe_path).output().ok()
                    } else {
                        None
                    };
                    let run_passed = run_output
                        .as_ref()
                        .map(|output| output.status.success())
                        .unwrap_or(false);
                    serde_json::json!({
                        "schema": "nc.mct.llvm.exe_smoke.v1",
                        "status": if link_passed && run_passed {
                            "llvm_exe_smoke_passed"
                        } else if link_passed {
                            "llvm_exe_smoke_run_failed"
                        } else {
                            "llvm_exe_smoke_link_failed"
                        },
                        "ll_path": ll_path.display().to_string(),
                        "exe_path": exe_path.display().to_string(),
                        "exe_exists": exe_path.exists(),
                        "clang_args": clang_args,
                        "clang": {
                            "available": true,
                            "status_code": output.status.code(),
                            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        },
                        "run": run_output.map(|output| serde_json::json!({
                            "status_code": output.status.code(),
                            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        })),
                        "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
                    })
                }
                Err(err) => serde_json::json!({
                    "schema": "nc.mct.llvm.exe_smoke.v1",
                    "status": "llvm_exe_smoke_missing_clang",
                    "ll_path": ll_path.display().to_string(),
                    "exe_path": exe_path.display().to_string(),
                    "exe_exists": false,
                    "clang_args": clang_args,
                    "clang": {
                        "available": false,
                        "error": err.to_string(),
                    },
                    "run": null,
                    "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
                }),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct llvm-exe-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!("ll: {}", ll_path.display());
                println!("exe: {}", exe_path.display());
            }
            if payload.get("status").and_then(|value| value.as_str())
                != Some("llvm_exe_smoke_passed")
            {
                return Err(actionable_command_error("mct", "mct llvm-exe-smoke failed").into());
            }
            Ok(())
        }
        "llvm-pipeline-smoke" | "llvm-clang-pipeline-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut allow_missing_tools = false;
            let mut lto_mode = "thin".to_string();
            let mut pgo_mode = "generate-use".to_string();
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--allow-missing-tools" => {
                        allow_missing_tools = true;
                        idx += 1;
                    }
                    "--cc" => idx += 2,
                    "--opt-level" | "-O" => idx += 2,
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-pipeline-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    "--lto" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-pipeline-smoke --lto expects off, thin, full or both",
                            )
                            .into());
                        };
                        lto_mode = value.clone();
                        idx += 2;
                    }
                    "--pgo" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-pipeline-smoke --pgo expects off or generate-use",
                            )
                            .into());
                        };
                        pgo_mode = value.clone();
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct llvm-pipeline-smoke: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-pipeline-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct llvm-pipeline-smoke expects <file.n>")
            })?;
            let profile = parse_backend_profile_from_args(args)?;
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-llvm-f8")
            });
            let payload = run_mct_llvm_pipeline_smoke(
                path,
                out_dir,
                profile,
                pass_pipeline,
                &lto_mode,
                &pgo_mode,
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct llvm-pipeline-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "ll: {}",
                    payload
                        .get("ll_path")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
            }
            let status = payload
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status == "mct_llvm_pipeline_smoke_failed"
                || (status == "mct_llvm_pipeline_smoke_missing_tools" && !allow_missing_tools)
            {
                return Err(actionable_command_error(
                    "mct",
                    "mct llvm-pipeline-smoke did not complete the Clang/LLVM LTO/PGO/debug pipeline",
                )
                .into());
            }
            Ok(())
        }
        "llvm-toolchain-smoke" | "llvm-matrix-smoke" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut all_targets = false;
            let mut allow_missing_tools = false;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => idx += 1,
                    "--all-targets" => {
                        all_targets = true;
                        idx += 1;
                    }
                    "--allow-missing-tools" => {
                        allow_missing_tools = true;
                        idx += 1;
                    }
                    "--cc" => idx += 2,
                    "--opt-level" | "-O" => idx += 2,
                    "--out-dir" => {
                        let Some(value) = args.get(idx + 1) else {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-toolchain-smoke --out-dir expects a directory path",
                            )
                            .into());
                        };
                        out_dir = Some(PathBuf::from(value));
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct llvm-toolchain-smoke: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct llvm-toolchain-smoke expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path.ok_or_else(|| {
                actionable_command_error("mct", "mct llvm-toolchain-smoke expects <file.n>")
            })?;
            let has_explicit_cc = args.iter().any(|arg| arg == "--cc");
            if all_targets && has_explicit_cc {
                return Err(actionable_command_error(
                    "mct",
                    "mct llvm-toolchain-smoke accepts either --all-targets or --cc <profile>, not both",
                )
                .into());
            }
            let selected_profile = if has_explicit_cc {
                Some(parse_backend_profile_from_args(args)?)
            } else {
                None
            };
            let pass_pipeline =
                ncc_ir::mct_llvm_pass_pipeline(parse_mct_llvm_optimization_level_from_args(args)?);
            let out_dir = out_dir.unwrap_or_else(|| {
                workspace_repo_root()
                    .join("target")
                    .join("tmp")
                    .join("mct-llvm-f6")
            });
            let payload =
                run_mct_llvm_toolchain_smoke(path, out_dir, selected_profile, pass_pipeline)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "mct llvm-toolchain-smoke: {}",
                    payload
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "rows: {} passed={} missing={} failed={}",
                    payload
                        .get("summary")
                        .and_then(|value| value.get("rows"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    payload
                        .get("summary")
                        .and_then(|value| value.get("passed"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    payload
                        .get("summary")
                        .and_then(|value| value.get("missing_toolchain"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    payload
                        .get("summary")
                        .and_then(|value| value.get("failed"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                );
            }
            let status = payload
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if status == "mct_llvm_toolchain_matrix_failed"
                || (status == "mct_llvm_toolchain_matrix_missing_tools" && !allow_missing_tools)
            {
                return Err(actionable_command_error(
                    "mct",
                    "mct llvm-toolchain-smoke did not complete every required LLVM toolchain row",
                )
                .into());
            }
            Ok(())
        }
        "translate" | "report" => {
            if args.iter().any(|arg| is_help_token(arg)) {
                println!("{}", command_help("mct").expect("mct help must exist"));
                return Ok(());
            }
            let mut path: Option<&str> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--json" => {
                        idx += 1;
                    }
                    "--cc" => {
                        idx += 2;
                    }
                    arg if arg.starts_with("--") => {
                        return Err(actionable_command_error(
                            "mct",
                            format!("mct translate: unknown option '{}'", arg),
                        )
                        .into());
                    }
                    arg => {
                        if path.is_some() {
                            return Err(actionable_command_error(
                                "mct",
                                "mct translate expects exactly one source file",
                            )
                            .into());
                        }
                        path = Some(arg);
                        idx += 1;
                    }
                }
            }
            let path = path
                .ok_or_else(|| actionable_command_error("mct", "mct translate expects <file.n>"))?;
            let profile = parse_backend_profile_from_args(args)?;
            let ast = load_ast_from_path(path)?;
            sema_check_with_warnings(&ast)?;
            let ir = ncc_ir::lower_with_backend_profile(ast, profile);
            let report = ncc_ir::mct_translate_program(&ir);
            let payload = mct_report_to_json(&report);
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("mct translate: {}", report.status);
                println!("target: {}", report.target_triple);
                println!("units: {}", report.units.len());
                println!("ncc-native: integrated");
                println!("full LLVM/GCC/rustc parity: not claimed");
            }
            Ok(())
        }
        other => {
            Err(actionable_command_error("mct", format!("unknown mct command '{}'", other)).into())
        }
    }
}
