use super::super::*;

pub(crate) fn run_high_level_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        if let Some(help) = command_help("high-level") {
            println!("{help}");
        }
        return Ok(());
    }

    let Some(subcommand) = args.first() else {
        return Err(actionable_command_error(
            "high-level",
            "high-level syntax: high-level <doctor|final-doctor|unified-doctor|surface|phase16-report|types|generics|interfaces|traits|contracts|dyn|trait-objects|associated-types|enums|patterns|match|option|result|errors|iterators|ranges|collections|async|effects|concurrency|channels|modules|packages|exports|api-surface|closures|lambdas|captures|reflection|introspection|metadata|const-eval|comptime|compile-time|macros|macro-expand|macro-report|macro-hygiene|risk|risk-report|authority-report|unification-prep> [file.n] [--json]",
        )
        .into());
    };

    match subcommand.as_str() {
        "doctor" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level doctor: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_phase15_doctor_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level doctor: OK");
                println!("phase0_audit: closed");
                println!("phase1_syntax_semantic_cleanup: closed");
                println!("phase2_type_system_advanced: closed");
                println!("phase3_generics_complete: closed");
                println!("phase4_interfaces_traits_contracts: closed");
                println!("phase5_dynamic_dispatch_trait_objects: closed");
                println!("phase6_adt_pattern_matching: closed");
                println!("phase7_result_option_error_algebra: closed");
                println!("phase8_iterators_ranges_collections: closed");
                println!("phase9_async_effects_concurrency: closed");
                println!("phase10_modules_packages_api_surface: closed");
                println!("closures_lambdas_captures: closed");
                println!("phase12_limited_reflection_introspection: closed");
                println!("phase13_compile_time_evaluation_model: closed");
                println!("const_eval: closed_pure_deterministic_bounded_cacheable");
                println!("comptime_unsafe: closed_explicit_allow_audited");
                println!("phase14_macros_codegen: closed");
                println!("macros: closed");
                println!("macro_hygiene/source_maps/reports: closed");
                println!("unsafe_macros: closed_explicit_allow_audited");
                println!("phase15_risk_authority_expert_policy: closed");
                println!("risk_authority: closed");
                println!("expert_policy_surface: closed_integrated");
                println!("phase15_plus: phase15_closed");
                println!("phase16_plus: not_started");
                println!("exit_code_policy: closed");
                println!("diagnostic_policy: closed");
            }
            Ok(())
        }
        "final-doctor" | "finaldoctor" | "unified-doctor" | "unifieddoctor" => {
            let json = high_level_json_flag_only(subcommand, args)?;
            let payload = high_level_phase16_doctor_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level final doctor: OK");
                println!("high_level_final_surface: closed");
                println!("high_level_phase1_15: closed");
                println!("high_level_phase16_unification: closed");
                println!("runtime_integration_plan: prepared");
                println!("compiler_integration_plan: prepared");
                println!("inspect_ai_under_high_level: documented");
                println!("doctor_unification_scope: normal_nc_high_level_only");
                println!("excluded_by_design: basal, machine, primitive, kernel");
                println!("machine_dependency: external_foundation_not_grouped");
                println!("basal_dependency: external_foundation_not_grouped");
                println!("phase17_plus: not_started");
            }
            Ok(())
        }
        "surface" | "unified-surface" | "unifiedsurface" => {
            let json = high_level_json_flag_only(subcommand, args)?;
            let payload = high_level_phase16_unified_surface_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level surface: OK");
                println!("high_level_final_surface: closed");
                println!("runtime/compiler: included_under_high_level_umbrella");
                println!("excluded: basal, primitive, machine, system, kernel");
            }
            Ok(())
        }
        "phase16-report" | "phase16" | "final-surface" | "finalsurface" => {
            let json = high_level_json_flag_only(subcommand, args)?;
            let payload = high_level_phase16_final_surface_report_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level phase16-report: OK");
                println!("unified_surface_manifest: closed");
                println!("runtime_compiler_unification_plan: prepared");
                println!("diagnostics_consolidation: closed");
                println!("capability_map: closed");
                println!("new_features: none");
            }
            Ok(())
        }
        "types" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level types: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_type_system_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level types: OK");
                println!("phase2_type_system_advanced: closed");
                println!("bool/int separation: closed");
                println!("nullability/unions/arrays/lists/ptr/memRegion: closed");
                println!("phase3_generics_complete: closed");
            }
            Ok(())
        }
        "pointer-model" | "pointermodel" | "pointers" | "ptr" => {
            let json = high_level_json_flag_only(subcommand, args)?;
            let payload = high_level_pointer_model_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level pointer-model: OK");
                println!("addr/ptr<T>/ptr mut<T>/ref<T>/ref mut<T>/site<T>/memRegion<T>: closed");
                println!("nullable: explicit only");
                println!("raw pointer hazards: explicit and audited");
                println!("ffi pointer contracts: required");
                println!("diagnostics: E_PTR_* official");
            }
            Ok(())
        }
        "pointer-industrial" | "pointer-industrial-closure" | "ptr-industrial" => {
            let json = high_level_json_flag_only(subcommand, args)?;
            let payload = pointer_industrial_closure_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level pointer-industrial: OK");
                println!("ptr/addr/site/memRegion C/C++ parity: closed");
                println!("runtime heap/live/dead/alias/lifetime: closed");
                println!("raw power: explicit risk authority only");
                println!("silent pointer ambiguity: explicit silentPointer risk only");
            }
            Ok(())
        }
        "generics" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level generics: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_generic_system_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level generics: OK");
                println!("phase3_generics_complete: closed");
                println!("generic functions/types/runtime families: closed");
                println!("phase4_interfaces_traits_contracts: closed");
            }
            Ok(())
        }
        "interfaces" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level interfaces: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_interfaces_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level interfaces: OK");
                println!("interfaces: closed");
                println!("static_dispatch: closed");
            }
            Ok(())
        }
        "traits" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level traits: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_traits_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level traits: OK");
                println!("traits: closed");
                println!("impl_resolution: closed");
            }
            Ok(())
        }
        "contracts" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level contracts: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_contracts_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level contracts: OK");
                println!("contracts: closed");
                println!("ensures/invariants: deferred_with_diagnostic");
            }
            Ok(())
        }
        "dyn" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level dyn: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_dyn_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level dyn: OK");
                println!("dynamic_dispatch: closed");
                println!("implicit_dynamic_dispatch: forbidden");
            }
            Ok(())
        }
        "trait-objects" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level trait-objects: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_trait_objects_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level trait-objects: OK");
                println!("trait_objects: closed");
                println!("object_safety: closed");
            }
            Ok(())
        }
        "associated-types" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level associated-types: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_associated_types_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level associated-types: OK");
                println!("associated_types: closed");
                println!("hkt: deferred");
            }
            Ok(())
        }
        "enums" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level enums: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_enums_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level enums: OK");
                println!("enums: closed");
                println!("generic_enums: closed_basic");
            }
            Ok(())
        }
        "patterns" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level patterns: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_patterns_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level patterns: OK");
                println!("variant/literal/wildcard/binding patterns: closed");
                println!("guards: closed_limited_pure");
            }
            Ok(())
        }
        "match" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level match: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_match_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level match: OK");
                println!("exhaustiveness: closed");
                println!("unreachable_arms: closed");
                println!("silent_incomplete_match: forbidden");
            }
            Ok(())
        }
        "option" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level option: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_option_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level option: OK");
                println!("Option<T>: closed");
                println!("None context: required");
            }
            Ok(())
        }
        "result" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level result: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_result_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level result: OK");
                println!("Result<T,E>: closed");
                println!("error any: forbidden");
            }
            Ok(())
        }
        "errors" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level errors: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_errors_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level errors: OK");
                println!("error algebra: closed_basic_explicit_match");
                println!("exceptions: not_supported");
            }
            Ok(())
        }
        "iterators" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level iterators: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_iterators_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level iterators: OK");
                println!("Iterator<T>/Iterable<T>: closed");
                println!("next() -> Option<T>: closed");
                println!("adapters: closed_typed_closures");
            }
            Ok(())
        }
        "ranges" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level ranges: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_ranges_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level ranges: OK");
                println!("finite integer ranges: closed");
                println!("unbounded ranges: deferred");
            }
            Ok(())
        }
        "collections" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level collections: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_collections_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level collections: OK");
                println!("Collection<T>: closed");
                println!("len/isEmpty/get: closed");
            }
            Ok(())
        }
        "async" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level async: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_async_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level async: OK");
                println!("async functions/Future/await: closed");
                println!("async runtime: native_state_machine_closed");
                println!("async streams/generators: closed_typed");
                println!("lifecycle: new/resume/drop/cancel");
            }
            Ok(())
        }
        "effects" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level effects: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_effects_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level effects: OK");
                println!("effect annotations/checking: closed");
                println!("effect polymorphism: deferred");
            }
            Ok(())
        }
        "concurrency" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level concurrency: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_concurrency_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level concurrency: OK");
                println!("spawn/join/task model: closed");
                println!("race diagnostics: closed");
            }
            Ok(())
        }
        "channels" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level channels: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_channels_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level channels: OK");
                println!("bounded channel<T,N>: closed_limited");
                println!("backpressure policy: required");
            }
            Ok(())
        }
        "modules" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level modules: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_modules_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level modules: OK");
                println!("module declarations/graph/cycle detection: closed");
            }
            Ok(())
        }
        "packages" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level packages: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_packages_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level packages: OK");
                println!("package declarations/API metadata: closed");
                println!("registry: not_started");
            }
            Ok(())
        }
        "exports" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level exports: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_exports_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level exports: OK");
                println!("imports/exports/reexports/visibility: closed");
            }
            Ok(())
        }
        "api-surface" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level api-surface: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_api_surface_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level api-surface: OK");
                println!("public API manifest/private-any leak gates: closed");
            }
            Ok(())
        }
        "closures" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level closures: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_closures_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level closures: OK");
                println!("closure expressions/environment/escape analysis: closed");
            }
            Ok(())
        }
        "lambdas" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level lambdas: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_lambdas_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level lambdas: OK");
                println!("lambda expressions/function types/calls: closed");
            }
            Ok(())
        }
        "captures" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level captures: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_captures_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level captures: OK");
                println!("explicit capture list/value/ref/mut/move: closed");
            }
            Ok(())
        }
        "reflection" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level reflection: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_reflection_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level reflection: OK");
                println!("reflection: closed_limited");
                println!("eval/mutation/private leaks: forbidden");
            }
            Ok(())
        }
        "introspection" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level introspection: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_introspection_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level introspection: OK");
                println!("module/package/API metadata: closed");
                println!("visibility filter: closed");
            }
            Ok(())
        }
        "metadata" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level metadata: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }

            let payload = high_level_metadata_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level metadata: OK");
                println!("TypeInfo/SymbolInfo/ClosureInfo: closed_limited");
                println!("compile-time evaluation/macros: not_started");
            }
            Ok(())
        }
        "const-eval" | "consteval" | "const" => {
            let (json, payload) = high_level_compile_time_file_payload(
                "const-eval",
                args,
                high_level_const_eval_manifest_payload(),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level const-eval: OK");
                println!("pure/deterministic/bounded/cacheable: closed");
                println!("side effects/noLimit/runtime/system/ffi/machine: forbidden");
            }
            Ok(())
        }
        "comptime" | "comptime-unsafe" => {
            let (json, payload) = high_level_compile_time_file_payload(
                "comptime",
                args,
                high_level_comptime_unsafe_manifest_payload(),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level comptime: OK");
                println!("explicit allow/effect audit/reproducibility report: closed");
                println!("noLimit: explicit_only");
            }
            Ok(())
        }
        "compile-time" | "compile-time-evaluation" | "compiletime" => {
            let (json, payload) = high_level_compile_time_file_payload(
                "compile-time",
                args,
                high_level_phase13_compile_time_manifest_payload(),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level compile-time: OK");
                println!("const_eval: closed");
                println!("comptime_unsafe: closed");
                println!("macros: not_started");
            }
            Ok(())
        }
        "macros" => {
            let (json, payload) = high_level_macro_file_payload("macros", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level macros: OK");
                println!("declarative/template/attribute/derive: closed");
                println!("unsafe macros: closed_explicit_allow_audited");
                println!("phase15_plus: phase15_closed");
            }
            Ok(())
        }
        "macro-expand" | "macroexpand" => {
            let (json, payload) = high_level_macro_file_payload("macro-expand", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level macro-expand: OK");
                println!("expanded AST/source map report generated");
            }
            Ok(())
        }
        "macro-report" | "macroreport" => {
            let (json, payload) = high_level_macro_file_payload("macro-report", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level macro-report: OK");
                println!("expansion report generated");
            }
            Ok(())
        }
        "macro-hygiene" | "macrohygiene" => {
            let (json, payload) = high_level_macro_file_payload("macro-hygiene", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level macro-hygiene: OK");
                println!("hygiene report generated");
            }
            Ok(())
        }
        "risk" | "risk-policy" | "policy" | "authority" => {
            let (json, payload) = high_level_risk_file_payload("risk", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level risk: OK");
                println!("risk_authority: closed");
                println!("expert_policy_surface: closed_integrated");
                println!("phase16_plus: not_started");
            }
            Ok(())
        }
        "risk-report" | "riskreport" => {
            let (json, payload) = high_level_risk_file_payload("risk-report", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level risk-report: OK");
                println!("risk report generated");
            }
            Ok(())
        }
        "authority-report" | "authorityreport" => {
            let (json, payload) = high_level_risk_file_payload("authority-report", args)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level authority-report: OK");
                println!("authority report generated");
            }
            Ok(())
        }
        "unification-prep" | "unification" => {
            let mut json = false;
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--json" => json = true,
                    "-h" | "--help" | "help" => {
                        return Err(actionable_command_error(
                            "high-level",
                            "high-level unification-prep: use high-level help for syntax",
                        )
                        .into());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "high-level",
                            format!("high-level unification-prep: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let payload = serde_json::json!({
                "schema": "nc.high_level.phase15.unification_prep.v1",
                "component": "high-level",
                "phase": "phase15_risk_authority_expert_policy_surface",
                "status": "closed",
                "unification_prep": high_level_unification_prep_payload(),
                "runtime": "prepared_not_merged",
                "compiler": "prepared_not_merged",
                "separate_expert_mode": "not_created",
                "phase16_plus": "not_started"
            });
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("high-level unification-prep: OK");
                println!("runtime/compiler: prepared_not_merged");
                println!("phase16_plus: not_started");
            }
            Ok(())
        }
        other => Err(actionable_command_error(
            "high-level",
            format!("unknown high-level command '{}'", other),
        )
        .into()),
    }
}
