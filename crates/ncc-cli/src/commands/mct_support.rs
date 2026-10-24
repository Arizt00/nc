use super::super::*;
use super::mct_json::*;

pub(crate) fn parse_mct_llvm_optimization_level_from_args(
    args: &[String],
) -> Result<ncc_ir::MctLlvmOptimizationLevelIR, Box<dyn std::error::Error>> {
    let mut selected: Option<ncc_ir::MctLlvmOptimizationLevelIR> = None;
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--opt-level" || args[i] == "-O" {
            if selected.is_some() {
                return Err("duplicate --opt-level option is not allowed".into());
            }
            let value = args
                .get(i + 1)
                .ok_or("MCT LLVM optimizer syntax: --opt-level <0|1|2|3|s|z|O0|O1|O2|O3|Os|Oz>")?;
            selected = Some(parse_mct_llvm_optimization_level_value(value)?);
            i += 2;
            continue;
        }
        i += 1;
    }
    Ok(selected.unwrap_or(ncc_ir::MctLlvmOptimizationLevelIR::O0))
}

pub(crate) fn parse_mct_cpu_tuning_from_args(
    args: &[String],
) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
    let mut cpu: Option<String> = None;
    let mut profile: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--cpu" | "--microarch" => {
                if cpu.is_some() {
                    return Err("duplicate --cpu/--microarch option is not allowed".into());
                }
                let value = args.get(i + 1).ok_or(
                    "MCT CPU tuning syntax: --cpu <microarchitecture-id> [--tune-profile <profile>]",
                )?;
                cpu = Some(value.clone());
                i += 2;
            }
            "--tune-profile" | "--tune" | "--fine-profile" => {
                if profile.is_some() {
                    return Err("duplicate --tune-profile/--tune option is not allowed".into());
                }
                let value = args.get(i + 1).ok_or(
                    "MCT CPU tuning syntax: --tune-profile <debug|baseline|latency|throughput|size>",
                )?;
                profile = Some(value.clone());
                i += 2;
            }
            _ => i += 1,
        }
    }

    Ok(cpu.map(|cpu| (cpu, profile.unwrap_or_else(|| "baseline".to_string()))))
}

pub(crate) fn parse_mct_llvm_optimization_level_value(
    raw: &str,
) -> Result<ncc_ir::MctLlvmOptimizationLevelIR, Box<dyn std::error::Error>> {
    match raw.to_ascii_lowercase().as_str() {
        "0" | "o0" | "debug" | "none" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::O0),
        "1" | "o1" | "release-light" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::O1),
        "2" | "o2" | "release" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::O2),
        "3" | "o3" | "release-fast" | "aggressive" => {
            Ok(ncc_ir::MctLlvmOptimizationLevelIR::O3)
        }
        "s" | "os" | "size" | "release-small" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::Os),
        "z" | "oz" | "tiny" | "release-tiny" => Ok(ncc_ir::MctLlvmOptimizationLevelIR::Oz),
        _ => Err(format!(
            "MCT LLVM optimizer syntax: unknown --opt-level '{}'; expected 0,1,2,3,s,z,O0,O1,O2,O3,Os,Oz",
            raw
        )
        .into()),
    }
}

pub(crate) fn mct_llvm_tool_candidates(tool: &str) -> Vec<&'static str> {
    match tool {
        "llvm-as" => vec![
            "llvm-as",
            "llvm-as-20",
            "llvm-as-19",
            "llvm-as-18",
            "llvm-as-17",
            "llvm-as-16",
            "llvm-as-15",
        ],
        "opt" => vec![
            "opt", "opt-20", "opt-19", "opt-18", "opt-17", "opt-16", "opt-15",
        ],
        "llc" => vec![
            "llc", "llc-20", "llc-19", "llc-18", "llc-17", "llc-16", "llc-15",
        ],
        "ld.lld" => vec![
            "ld.lld",
            "ld.lld-20",
            "ld.lld-19",
            "ld.lld-18",
            "ld.lld-17",
            "ld.lld-16",
            "ld.lld-15",
        ],
        "lld-link" => vec![
            "lld-link",
            "lld-link-20",
            "lld-link-19",
            "lld-link-18",
            "lld-link-17",
            "lld-link-16",
            "lld-link-15",
        ],
        "ld64.lld" => vec![
            "ld64.lld",
            "ld64.lld-20",
            "ld64.lld-19",
            "ld64.lld-18",
            "ld64.lld-17",
            "ld64.lld-16",
            "ld64.lld-15",
        ],
        "wasm-ld" => vec![
            "wasm-ld",
            "wasm-ld-20",
            "wasm-ld-19",
            "wasm-ld-18",
            "wasm-ld-17",
            "wasm-ld-16",
            "wasm-ld-15",
        ],
        "clang" => vec![
            "clang", "clang-20", "clang-19", "clang-18", "clang-17", "clang-16", "clang-15",
        ],
        "llvm-profdata" => vec![
            "llvm-profdata",
            "llvm-profdata-20",
            "llvm-profdata-19",
            "llvm-profdata-18",
            "llvm-profdata-17",
            "llvm-profdata-16",
            "llvm-profdata-15",
        ],
        "llvm-objdump" => vec![
            "llvm-objdump",
            "llvm-objdump-20",
            "llvm-objdump-19",
            "llvm-objdump-18",
            "llvm-objdump-17",
            "llvm-objdump-16",
            "llvm-objdump-15",
        ],
        _ => Vec::new(),
    }
}

fn mct_running_under_wsl() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists())
}

fn mct_known_tool_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    for env_name in ["LLVM_HOME", "LLVM_DIR", "CLANG_HOME"] {
        if let Some(value) = std::env::var_os(env_name) {
            let root = std::path::PathBuf::from(value);
            dirs.push(root.join("bin"));
            dirs.push(root);
        }
    }
    if cfg!(windows) {
        for env_name in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(value) = std::env::var_os(env_name) {
                dirs.push(std::path::PathBuf::from(value).join("LLVM").join("bin"));
            }
        }
        dirs.extend([
            std::path::PathBuf::from(r"C:\msys64\ucrt64\bin"),
            std::path::PathBuf::from(r"C:\msys64\clang64\bin"),
            std::path::PathBuf::from(r"C:\msys64\mingw64\bin"),
            std::path::PathBuf::from(r"C:\msys64\usr\bin"),
        ]);
    }
    if mct_running_under_wsl() {
        dirs.extend([
            std::path::PathBuf::from("/mnt/c/Program Files/LLVM/bin"),
            std::path::PathBuf::from("/mnt/c/msys64/ucrt64/bin"),
            std::path::PathBuf::from("/mnt/c/msys64/clang64/bin"),
            std::path::PathBuf::from("/mnt/c/msys64/mingw64/bin"),
            std::path::PathBuf::from("/mnt/c/msys64/usr/bin"),
        ]);
    }
    dirs
}

fn mct_tool_file_names(tool: &str) -> Vec<String> {
    let mut names = mct_llvm_tool_candidates(tool)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if names.is_empty() {
        names.push(tool.to_string());
    }
    if cfg!(windows) || mct_running_under_wsl() {
        let exe_names = names
            .iter()
            .filter(|name| !name.ends_with(".exe"))
            .map(|name| format!("{name}.exe"))
            .collect::<Vec<_>>();
        names.extend(exe_names);
    }
    names.sort();
    names.dedup();
    names
}

fn mct_tool_runs(path: &str) -> bool {
    Command::new(path).arg("--version").output().is_ok()
}

pub(crate) fn mct_find_tool(tool: &str) -> Option<String> {
    for candidate in mct_llvm_tool_candidates(tool) {
        if mct_tool_runs(candidate) {
            return Some(candidate.to_string());
        }
    }
    for dir in mct_known_tool_dirs() {
        for file_name in mct_tool_file_names(tool) {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                let candidate = candidate.display().to_string();
                if mct_tool_runs(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(crate) fn mct_path_for_tool(tool: &str, path: &std::path::Path) -> Result<String, String> {
    if !mct_running_under_wsl() || !tool.to_ascii_lowercase().ends_with(".exe") {
        return Ok(path.display().to_string());
    }

    let output = Command::new("wslpath")
        .args(["-a", "-w"])
        .arg(path)
        .output()
        .map_err(|error| {
            format!(
                "cannot translate '{}' for Windows tool '{tool}': {error}",
                path.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "wslpath rejected '{}' for Windows tool '{tool}': {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let translated = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if translated.is_empty() {
        return Err(format!(
            "wslpath returned an empty path for '{}' and Windows tool '{tool}'",
            path.display()
        ));
    }
    Ok(translated)
}

pub(crate) fn mct_tool_version(tool: &str) -> Option<String> {
    let output = Command::new(tool).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|line| line.trim().to_string())
}

pub(crate) fn mct_llvm_lld_tool_for_format(format: &ncc_ir::BackendObjectFormatIR) -> &'static str {
    match format {
        ncc_ir::BackendObjectFormatIR::Elf32
        | ncc_ir::BackendObjectFormatIR::Elf64
        | ncc_ir::BackendObjectFormatIR::FreestandingImage => "ld.lld",
        ncc_ir::BackendObjectFormatIR::Coff => "lld-link",
        ncc_ir::BackendObjectFormatIR::MachO => "ld64.lld",
        ncc_ir::BackendObjectFormatIR::WasmModule => "wasm-ld",
    }
}

pub(crate) fn mct_llvm_toolchain_matrix_profiles(
    selected: Option<ncc_ir::BackendLoweringProfileIR>,
) -> Vec<(&'static str, ncc_ir::BackendLoweringProfileIR)> {
    if let Some(profile) = selected {
        let label = backend_lowering_profile_label(&profile);
        return vec![(label, profile)];
    }
    vec![
        ("nC.sysv64", ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
        ("nC.win64", ncc_ir::BackendLoweringProfileIR::nc_win64()),
        ("nC.aapcs64", ncc_ir::BackendLoweringProfileIR::nc_aapcs64()),
        (
            "nC.portable",
            ncc_ir::BackendLoweringProfileIR::nc_portable(),
        ),
    ]
}

pub(crate) fn backend_lowering_profile_label(
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> &'static str {
    match profile.target_spec.abi {
        ncc_ir::BackendAbiModelIR::I386SysV => return "nC.i386-sysv",
        ncc_ir::BackendAbiModelIR::I386Windows => return "nC.i386-msvc",
        _ => {}
    }
    match profile.calling_convention {
        ncc_ir::BackendCallingConventionIR::NcSysV64 => "nC.sysv64",
        ncc_ir::BackendCallingConventionIR::NcWin64 => "nC.win64",
        ncc_ir::BackendCallingConventionIR::NcAapcs64 => "nC.aapcs64",
        ncc_ir::BackendCallingConventionIR::NcPortable => "nC.portable",
        ncc_ir::BackendCallingConventionIR::NcSovereign => "nC.sovereign",
    }
}

#[derive(Clone)]
pub(crate) struct MctCpuTuningObjectDisasmMatrixRow {
    id: &'static str,
    profile_label: &'static str,
    profile: ncc_ir::BackendLoweringProfileIR,
    microarchitecture_id: &'static str,
    fine_tuning_profile_id: &'static str,
}

pub(crate) fn default_cpu_tuning_row_for_profile(
    profile: ncc_ir::BackendLoweringProfileIR,
) -> MctCpuTuningObjectDisasmMatrixRow {
    let profile_label = backend_lowering_profile_label(&profile);
    match &profile.target_spec.isa {
        ncc_ir::BackendIsaIR::X86 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-i386-generic-baseline",
            profile_label,
            profile,
            microarchitecture_id: "x86_64-v2",
            fine_tuning_profile_id: "baseline",
        },
        ncc_ir::BackendIsaIR::X86_64 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-x86_64-intel-skylake-throughput",
            profile_label,
            profile,
            microarchitecture_id: "intel-skylake",
            fine_tuning_profile_id: "throughput",
        },
        ncc_ir::BackendIsaIR::ArmV7 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-armv7-thumb2-generic-baseline",
            profile_label,
            profile,
            microarchitecture_id: "armv7-generic",
            fine_tuning_profile_id: "baseline",
        },
        ncc_ir::BackendIsaIR::AArch64 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-aarch64-apple-m1-throughput",
            profile_label,
            profile,
            microarchitecture_id: "apple-m1",
            fine_tuning_profile_id: "throughput",
        },
        ncc_ir::BackendIsaIR::Wasm32 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-wasm32-generic-baseline",
            profile_label,
            profile,
            microarchitecture_id: "wasm32-generic",
            fine_tuning_profile_id: "baseline",
        },
        ncc_ir::BackendIsaIR::RiscV32 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-riscv32-generic-baseline",
            profile_label,
            profile,
            microarchitecture_id: "riscv64-generic",
            fine_tuning_profile_id: "baseline",
        },
        ncc_ir::BackendIsaIR::RiscV64 => MctCpuTuningObjectDisasmMatrixRow {
            id: "selected-riscv64-generic-baseline",
            profile_label,
            profile,
            microarchitecture_id: "riscv64-generic",
            fine_tuning_profile_id: "baseline",
        },
    }
}

pub(crate) fn mct_cpu_tuning_object_disasm_matrix_rows(
    selected: Option<ncc_ir::BackendLoweringProfileIR>,
) -> Vec<MctCpuTuningObjectDisasmMatrixRow> {
    if let Some(profile) = selected {
        return vec![default_cpu_tuning_row_for_profile(profile)];
    }

    vec![
        MctCpuTuningObjectDisasmMatrixRow {
            id: "sysv64-intel-skylake-throughput",
            profile_label: "nC.sysv64",
            profile: ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
            microarchitecture_id: "intel-skylake",
            fine_tuning_profile_id: "throughput",
        },
        MctCpuTuningObjectDisasmMatrixRow {
            id: "win64-intel-skylake-throughput",
            profile_label: "nC.win64",
            profile: ncc_ir::BackendLoweringProfileIR::nc_win64(),
            microarchitecture_id: "intel-skylake",
            fine_tuning_profile_id: "throughput",
        },
        MctCpuTuningObjectDisasmMatrixRow {
            id: "aapcs64-apple-m1-throughput",
            profile_label: "nC.aapcs64",
            profile: ncc_ir::BackendLoweringProfileIR::nc_aapcs64(),
            microarchitecture_id: "apple-m1",
            fine_tuning_profile_id: "throughput",
        },
    ]
}

pub(crate) fn mct_cpu_tuning_object_disasm_matrix_manifest() -> serde_json::Value {
    let rows = mct_cpu_tuning_object_disasm_matrix_rows(None)
        .into_iter()
        .map(|row| {
            let native_selection_policy =
                ncc_ir::backend_native_cpu_tuning_policy_for_mct_cpu(
                    row.microarchitecture_id,
                    row.fine_tuning_profile_id,
                )
                .map(|policy| backend_native_cpu_tuning_policy_to_json(&policy))
                .unwrap_or_else(|err| {
                    serde_json::json!({
                        "schema": "nc.backend.native_cpu_tuning_policy.v1",
                        "status": "backend_native_cpu_tuning_policy_error",
                        "error": err,
                    })
                });
            serde_json::json!({
                "id": row.id,
                "profile": row.profile_label,
                "target_triple": row.profile.target_spec.target_triple,
                "object_format": render_backend_object_format_label(&row.profile.target_spec.object_format),
                "abi": render_backend_abi_label(&row.profile.target_spec.abi),
                "microarchitecture_id": row.microarchitecture_id,
                "fine_tuning_profile_id": row.fine_tuning_profile_id,
                "expected_pipeline": ["emit-llvm", "llvm-as", "opt", "llc-object", "llvm-objdump"],
                "native_selection_policy": native_selection_policy,
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "schema": "nc.mct.cpu_tuning_object_disasm_matrix.v1",
        "status": "mct_cpu_tuning_object_disasm_matrix_defined",
        "phase": "F12",
        "rows": rows,
        "tools": [
            {
                "stage": "llvm-as",
                "candidates": mct_llvm_tool_candidates("llvm-as"),
                "selected": mct_find_tool("llvm-as"),
            },
            {
                "stage": "opt",
                "candidates": mct_llvm_tool_candidates("opt"),
                "selected": mct_find_tool("opt"),
            },
            {
                "stage": "llc",
                "candidates": mct_llvm_tool_candidates("llc"),
                "selected": mct_find_tool("llc"),
            },
            {
                "stage": "llvm-objdump",
                "candidates": mct_llvm_tool_candidates("llvm-objdump"),
                "selected": mct_find_tool("llvm-objdump"),
            },
        ],
        "truth_boundary": {
            "object_disasm_only": true,
            "link_execution_not_claimed": true,
            "missing_tools_are_not_faked": true,
            "full_multi_host_ci_parity": false,
        }
    })
}

pub(crate) fn mct_llvm_toolchain_matrix_manifest() -> serde_json::Value {
    serde_json::json!({
        "schema": "nc.mct.llvm.toolchain_matrix.v1",
        "status": "mct_llvm_toolchain_matrix_defined",
        "phase": "F6",
        "pipeline": ["emit-llvm", "llvm-as", "opt", "llc", "lld-compatible-link"],
        "tools": [
            {
                "stage": "llvm-as",
                "candidates": mct_llvm_tool_candidates("llvm-as"),
                "selected": mct_find_tool("llvm-as"),
            },
            {
                "stage": "opt",
                "candidates": mct_llvm_tool_candidates("opt"),
                "selected": mct_find_tool("opt"),
            },
            {
                "stage": "llc",
                "candidates": mct_llvm_tool_candidates("llc"),
                "selected": mct_find_tool("llc"),
            },
            {
                "stage": "llvm-objdump",
                "candidates": mct_llvm_tool_candidates("llvm-objdump"),
                "selected": mct_find_tool("llvm-objdump"),
            },
            {
                "stage": "ld.lld",
                "candidates": mct_llvm_tool_candidates("ld.lld"),
                "selected": mct_find_tool("ld.lld"),
            },
            {
                "stage": "lld-link",
                "candidates": mct_llvm_tool_candidates("lld-link"),
                "selected": mct_find_tool("lld-link"),
            },
            {
                "stage": "ld64.lld",
                "candidates": mct_llvm_tool_candidates("ld64.lld"),
                "selected": mct_find_tool("ld64.lld"),
            },
            {
                "stage": "wasm-ld",
                "candidates": mct_llvm_tool_candidates("wasm-ld"),
                "selected": mct_find_tool("wasm-ld"),
            },
        ],
        "profiles": mct_llvm_toolchain_matrix_profiles(None).iter().map(|(id, profile)| {
            serde_json::json!({
                "id": id,
                "target_triple": profile.target_spec.target_triple,
                "object_format": render_backend_object_format_label(&profile.target_spec.object_format),
                "abi": render_backend_abi_label(&profile.target_spec.abi),
                "required_lld": mct_llvm_lld_tool_for_format(&profile.target_spec.object_format),
            })
        }).collect::<Vec<_>>(),
        "truth_boundary": {
            "external_tools_are_required_for_evidence": true,
            "missing_tools_are_reported": true,
            "missing_tools_are_not_counted_as_pass": true,
            "full_multi_host_ci_parity": false,
        }
    })
}

pub(crate) fn mct_stage_to_json(
    stage: &str,
    tool: Option<&str>,
    args: &[String],
    output: Result<std::process::Output, std::io::Error>,
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
            "status": "tool_error",
            "error": err.to_string(),
        }),
    }
}

pub(crate) fn mct_stage_passed(stage: &serde_json::Value) -> bool {
    stage
        .get("status")
        .and_then(|value| value.as_str())
        .is_some_and(|status| status == "passed")
}

pub(crate) fn mct_pipeline_stage_ok(stage: &serde_json::Value) -> bool {
    stage
        .get("status")
        .and_then(|value| value.as_str())
        .is_some_and(|status| {
            matches!(
                status,
                "passed" | "skipped" | "skipped_no_main" | "profile_written"
            )
        })
}

pub(crate) fn mct_collect_profraw_files(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            mct_collect_profraw_files(&path, out)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("profraw") {
            out.push(path);
        }
    }
    Ok(())
}

pub(crate) fn mct_pipeline_lto_modes(
    raw: &str,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    match raw.to_ascii_lowercase().as_str() {
        "off" | "none" | "false" | "0" => Ok(Vec::new()),
        "thin" => Ok(vec!["thin"]),
        "full" => Ok(vec!["full"]),
        "both" | "all" => Ok(vec!["thin", "full"]),
        other => Err(actionable_command_error(
            "mct",
            format!(
                "mct llvm-pipeline-smoke: unknown --lto '{}'; expected off, thin, full or both",
                other
            ),
        )
        .into()),
    }
}

pub(crate) fn mct_pipeline_compile_args(
    module: &ncc_ir::MctLlvmModuleIR,
    input: &Path,
    output: &Path,
    extra: &[String],
    compile_only: bool,
) -> Vec<String> {
    let mut args = vec![
        format!("--target={}", module.target_triple),
        module.pass_pipeline.clang_opt_arg.to_string(),
    ];
    if module.debug_metadata.enabled {
        args.extend(
            module
                .pass_pipeline
                .clang_debug_args
                .iter()
                .map(|arg| (*arg).to_string()),
        );
    }
    args.extend(extra.iter().cloned());
    args.push("-x".to_string());
    args.push("ir".to_string());
    if compile_only {
        args.push("-c".to_string());
    }
    args.push(input.display().to_string());
    args.push("-o".to_string());
    args.push(output.display().to_string());
    args
}

pub(crate) fn run_mct_llvm_pipeline_smoke(
    path: &str,
    out_dir: PathBuf,
    profile: ncc_ir::BackendLoweringProfileIR,
    pass_pipeline: ncc_ir::MctLlvmPassPipelineIR,
    lto_mode: &str,
    pgo_mode: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    fs::create_dir_all(&out_dir)?;
    let source = fs::read_to_string(path)?;
    let ast = load_ast_from_path(path)?;
    sema_check_with_warnings(&ast)?;
    let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile.clone(), path, &source);
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config(
        &ir,
        pass_pipeline,
        mct_llvm_debug_config_for_path(path),
    );
    let stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module")
        .to_string();
    let ll_path = out_dir.join(format!("{stem}.f8.ll"));
    let opt_ll_path = out_dir.join(format!("{stem}.f8.opt.ll"));
    let obj_path = out_dir.join(format!("{stem}.f8.o"));
    fs::write(&ll_path, module.llvm_ir.as_bytes())?;

    let clang = mct_find_tool("clang");
    let opt = mct_find_tool("opt");
    let profdata = mct_find_tool("llvm-profdata");
    let pgo_generate_use = matches!(
        pgo_mode.to_ascii_lowercase().as_str(),
        "generate-use" | "gen-use" | "on" | "true" | "1"
    );
    let pgo_off = matches!(
        pgo_mode.to_ascii_lowercase().as_str(),
        "off" | "none" | "false" | "0"
    );
    if !pgo_generate_use && !pgo_off {
        return Err(actionable_command_error(
            "mct",
            format!(
                "mct llvm-pipeline-smoke: unknown --pgo '{}'; expected off or generate-use",
                pgo_mode
            ),
        )
        .into());
    }
    let mut missing_tools = Vec::new();
    if clang.is_none() {
        missing_tools.push("clang");
    }
    if opt.is_none() {
        missing_tools.push("opt");
    }
    if pgo_generate_use && profdata.is_none() {
        missing_tools.push("llvm-profdata");
    }
    if !missing_tools.is_empty() {
        return Ok(serde_json::json!({
            "schema": "nc.mct.llvm.pipeline_smoke.v1",
            "status": "mct_llvm_pipeline_smoke_missing_tools",
            "file": path,
            "out_dir": out_dir.display().to_string(),
            "missing_tools": missing_tools,
            "ll_path": ll_path.display().to_string(),
            "pipeline": mct_llvm_pass_pipeline_to_json(&module.pass_pipeline),
            "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
            "stages": [],
            "truth_boundary": {
                "missing_tools_are_not_faked": true,
                "applies_clang_new_pass_manager": false,
                "applies_opt_new_pass_manager": false,
                "applies_lto": false,
                "applies_pgo": false,
                "debug_metadata_preserved": module.debug_metadata.enabled,
            }
        }));
    }
    let clang_tool = clang.as_deref().expect("checked above");
    let opt_tool = opt.as_deref().expect("checked above");
    let mut stages = Vec::new();

    let opt_args = vec![
        format!("-passes={}", module.pass_pipeline.opt_passes_arg),
        ll_path.display().to_string(),
        "-S".to_string(),
        "-o".to_string(),
        opt_ll_path.display().to_string(),
    ];
    stages.push(mct_stage_to_json(
        "opt-new-pass-manager",
        Some(opt_tool),
        &opt_args,
        Command::new(opt_tool).args(&opt_args).output(),
    ));

    if stages.last().is_some_and(mct_stage_passed) {
        let clang_object_args =
            mct_pipeline_compile_args(&module, &opt_ll_path, &obj_path, &[], true);
        stages.push(mct_stage_to_json(
            "clang-object-debug",
            Some(clang_tool),
            &clang_object_args,
            Command::new(clang_tool).args(&clang_object_args).output(),
        ));
    }

    for mode in mct_pipeline_lto_modes(lto_mode)? {
        if stages.iter().all(mct_pipeline_stage_ok) {
            let lto_path = out_dir.join(format!("{stem}.f8.{mode}lto.o"));
            let lto_arg = format!("-flto={mode}");
            let lto_args =
                mct_pipeline_compile_args(&module, &ll_path, &lto_path, &[lto_arg], true);
            stages.push(mct_stage_to_json(
                &format!("clang-lto-{mode}"),
                Some(clang_tool),
                &lto_args,
                Command::new(clang_tool).args(&lto_args).output(),
            ));
        }
    }

    if pgo_generate_use {
        let exe_suffix = if module.target_triple.contains("windows") {
            ".exe"
        } else {
            ""
        };
        if !module.llvm_ir.contains("define i32 @main()") {
            stages.push(serde_json::json!({
                "stage": "clang-pgo-generate-use",
                "status": "skipped_no_main",
                "reason": "PGO requires a hosted executable main; the emitted module did not contain define i32 @main().",
            }));
        } else if stages.iter().all(mct_pipeline_stage_ok) {
            let profile_dir = out_dir.join(format!("{stem}.f8.profraw.d"));
            fs::create_dir_all(&profile_dir)?;
            let pgo_gen_exe = out_dir.join(format!("{stem}.f8.pgo-gen{exe_suffix}"));
            let pgo_gen_args = mct_pipeline_compile_args(
                &module,
                &ll_path,
                &pgo_gen_exe,
                &[format!("-fprofile-generate={}", profile_dir.display())],
                false,
            );
            stages.push(mct_stage_to_json(
                "clang-pgo-generate-compile",
                Some(clang_tool),
                &pgo_gen_args,
                Command::new(clang_tool).args(&pgo_gen_args).output(),
            ));

            let mut profraw_files = Vec::new();
            if stages.last().is_some_and(mct_stage_passed) {
                let run_output = Command::new(&pgo_gen_exe).output();
                let run_stage = mct_stage_to_json(
                    "clang-pgo-generate-run",
                    Some(&pgo_gen_exe.display().to_string()),
                    &[],
                    run_output,
                );
                stages.push(run_stage);
                mct_collect_profraw_files(&profile_dir, &mut profraw_files)?;
                if profraw_files.is_empty() {
                    stages.push(serde_json::json!({
                        "stage": "clang-pgo-profraw",
                        "status": "failed",
                        "profile_dir": profile_dir.display().to_string(),
                        "reason": "profile generation ran but produced no .profraw files",
                    }));
                } else {
                    stages.push(serde_json::json!({
                        "stage": "clang-pgo-profraw",
                        "status": "profile_written",
                        "profile_dir": profile_dir.display().to_string(),
                        "profraw_files": profraw_files.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                    }));
                }
            }

            if stages.iter().all(mct_pipeline_stage_ok) && !profraw_files.is_empty() {
                let profdata_path = out_dir.join(format!("{stem}.f8.profdata"));
                let profdata_tool = profdata.as_deref().expect("checked above");
                let mut merge_args = vec![
                    "merge".to_string(),
                    "-output".to_string(),
                    profdata_path.display().to_string(),
                ];
                merge_args.extend(profraw_files.iter().map(|path| path.display().to_string()));
                stages.push(mct_stage_to_json(
                    "llvm-profdata-merge",
                    Some(profdata_tool),
                    &merge_args,
                    Command::new(profdata_tool).args(&merge_args).output(),
                ));
                if stages.last().is_some_and(mct_stage_passed) {
                    let pgo_use_exe = out_dir.join(format!("{stem}.f8.pgo-use{exe_suffix}"));
                    let pgo_use_args = mct_pipeline_compile_args(
                        &module,
                        &ll_path,
                        &pgo_use_exe,
                        &[format!("-fprofile-use={}", profdata_path.display())],
                        false,
                    );
                    stages.push(mct_stage_to_json(
                        "clang-pgo-use-compile",
                        Some(clang_tool),
                        &pgo_use_args,
                        Command::new(clang_tool).args(&pgo_use_args).output(),
                    ));
                }
            }
        }
    }

    let passed = stages.iter().all(mct_pipeline_stage_ok);
    Ok(serde_json::json!({
        "schema": "nc.mct.llvm.pipeline_smoke.v1",
        "status": if passed {
            "mct_llvm_pipeline_smoke_passed"
        } else {
            "mct_llvm_pipeline_smoke_failed"
        },
        "file": path,
        "out_dir": out_dir.display().to_string(),
        "ll_path": ll_path.display().to_string(),
        "optimized_ll_path": opt_ll_path.display().to_string(),
        "object_path": obj_path.display().to_string(),
        "object_exists": obj_path.exists(),
        "lto_mode": lto_mode,
        "pgo_mode": pgo_mode,
        "pipeline": mct_llvm_pass_pipeline_to_json(&module.pass_pipeline),
        "tool_versions": {
            "clang": mct_tool_version(clang_tool),
            "opt": mct_tool_version(opt_tool),
            "llvm_profdata": profdata.as_ref().and_then(|tool| mct_tool_version(tool)),
        },
        "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
        "stages": stages,
        "truth_boundary": {
            "applies_clang_new_pass_manager": true,
            "applies_opt_new_pass_manager": true,
            "applies_lto": !mct_pipeline_lto_modes(lto_mode)?.is_empty(),
            "applies_pgo": pgo_generate_use,
            "debug_metadata_preserved": module.debug_metadata.enabled,
            "embedded_libllvm_pass_manager": false,
            "missing_tools_are_not_faked": true,
        }
    }))
}

pub(crate) fn run_mct_llvm_toolchain_smoke(
    path: &str,
    out_dir: PathBuf,
    selected_profile: Option<ncc_ir::BackendLoweringProfileIR>,
    pass_pipeline: ncc_ir::MctLlvmPassPipelineIR,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let profiles = mct_llvm_toolchain_matrix_profiles(selected_profile);
    fs::create_dir_all(&out_dir)?;

    let llvm_as = mct_find_tool("llvm-as");
    let opt = mct_find_tool("opt");
    let llc = mct_find_tool("llc");
    let stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module")
        .to_string();
    let mut rows = Vec::new();
    let source = fs::read_to_string(path)?;

    for (profile_id, profile) in profiles {
        let ast = load_ast_from_path(path)?;
        sema_check_with_warnings(&ast)?;
        let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile.clone(), path, &source);
        let module = ncc_ir::mct_emit_llvm_module_with_debug_config(
            &ir,
            pass_pipeline.clone(),
            mct_llvm_debug_config_for_path(path),
        );
        let row_stem = format!(
            "{}_{}",
            stem,
            profile_id.replace("nC.", "nc_").replace(['.', '-'], "_")
        );
        let ll_path = out_dir.join(format!("{row_stem}.ll"));
        let bc_path = out_dir.join(format!("{row_stem}.bc"));
        let opt_bc_path = out_dir.join(format!("{row_stem}.opt.bc"));
        let obj_path = out_dir.join(format!("{row_stem}.o"));
        let linked_path = out_dir.join(format!("{row_stem}.linked"));
        fs::write(&ll_path, module.llvm_ir.as_bytes())?;

        let lld_name = mct_llvm_lld_tool_for_format(&profile.target_spec.object_format);
        let lld = mct_find_tool(lld_name);
        let mut missing_tools = Vec::new();
        if llvm_as.is_none() {
            missing_tools.push("llvm-as");
        }
        if opt.is_none() {
            missing_tools.push("opt");
        }
        if llc.is_none() {
            missing_tools.push("llc");
        }
        if lld.is_none() {
            missing_tools.push(lld_name);
        }

        if !missing_tools.is_empty() {
            rows.push(serde_json::json!({
                "profile": profile_id,
                "target_triple": module.target_triple,
                "object_format": render_backend_object_format_label(&profile.target_spec.object_format),
                "status": "missing_toolchain",
                "missing_tools": missing_tools,
                "ll_path": ll_path.display().to_string(),
                "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
                "stages": [],
            }));
            continue;
        }

        let llvm_as_tool = llvm_as.as_deref().expect("checked above");
        let opt_tool = opt.as_deref().expect("checked above");
        let llc_tool = llc.as_deref().expect("checked above");
        let lld_tool = lld.as_deref().expect("checked above");

        let llvm_as_args = vec![
            ll_path.display().to_string(),
            "-o".to_string(),
            bc_path.display().to_string(),
        ];
        let llvm_as_stage = mct_stage_to_json(
            "llvm-as",
            Some(llvm_as_tool),
            &llvm_as_args,
            Command::new(llvm_as_tool).args(&llvm_as_args).output(),
        );

        let mut stages = vec![llvm_as_stage];
        if stages.last().is_some_and(mct_stage_passed) {
            let opt_args = vec![
                format!("-passes={}", module.pass_pipeline.opt_passes_arg),
                bc_path.display().to_string(),
                "-o".to_string(),
                opt_bc_path.display().to_string(),
            ];
            stages.push(mct_stage_to_json(
                "opt",
                Some(opt_tool),
                &opt_args,
                Command::new(opt_tool).args(&opt_args).output(),
            ));
        }

        if stages.last().is_some_and(mct_stage_passed) {
            let llc_args = vec![
                format!("-mtriple={}", module.target_triple),
                "-filetype=obj".to_string(),
                opt_bc_path.display().to_string(),
                "-o".to_string(),
                obj_path.display().to_string(),
            ];
            stages.push(mct_stage_to_json(
                "llc",
                Some(llc_tool),
                &llc_args,
                Command::new(llc_tool).args(&llc_args).output(),
            ));
        }

        if stages.last().is_some_and(mct_stage_passed) {
            let link_args = match profile.target_spec.object_format {
                ncc_ir::BackendObjectFormatIR::Elf32
                | ncc_ir::BackendObjectFormatIR::Elf64
                | ncc_ir::BackendObjectFormatIR::FreestandingImage => vec![
                    "-r".to_string(),
                    obj_path.display().to_string(),
                    "-o".to_string(),
                    linked_path.display().to_string(),
                ],
                ncc_ir::BackendObjectFormatIR::Coff => vec![
                    "/LIB".to_string(),
                    "/NOLOGO".to_string(),
                    format!("/OUT:{}", linked_path.display()),
                    obj_path.display().to_string(),
                ],
                ncc_ir::BackendObjectFormatIR::MachO => vec![
                    "-arch".to_string(),
                    "arm64".to_string(),
                    "-platform_version".to_string(),
                    "macos".to_string(),
                    "11.0".to_string(),
                    "11.0".to_string(),
                    "-r".to_string(),
                    obj_path.display().to_string(),
                    "-o".to_string(),
                    linked_path.display().to_string(),
                ],
                ncc_ir::BackendObjectFormatIR::WasmModule => vec![
                    "--no-entry".to_string(),
                    "--allow-undefined".to_string(),
                    obj_path.display().to_string(),
                    "-o".to_string(),
                    linked_path.with_extension("wasm").display().to_string(),
                ],
            };
            stages.push(mct_stage_to_json(
                "lld",
                Some(lld_tool),
                &link_args,
                Command::new(lld_tool).args(&link_args).output(),
            ));
        }

        let passed = stages.iter().all(mct_stage_passed);
        rows.push(serde_json::json!({
            "profile": profile_id,
            "target_triple": module.target_triple,
            "object_format": render_backend_object_format_label(&profile.target_spec.object_format),
            "status": if passed { "passed" } else { "failed" },
            "ll_path": ll_path.display().to_string(),
            "bitcode_path": bc_path.display().to_string(),
            "optimized_bitcode_path": opt_bc_path.display().to_string(),
            "object_path": obj_path.display().to_string(),
            "linked_path": linked_path.display().to_string(),
            "object_exists": obj_path.exists(),
            "linked_exists": linked_path.exists() || linked_path.with_extension("wasm").exists(),
            "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
            "stages": stages,
        }));
    }

    let passed_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(|value| value.as_str()) == Some("passed"))
        .count();
    let missing_rows = rows
        .iter()
        .filter(|row| {
            row.get("status").and_then(|value| value.as_str()) == Some("missing_toolchain")
        })
        .count();
    let failed_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(|value| value.as_str()) == Some("failed"))
        .count();
    let status = if failed_rows > 0 {
        "mct_llvm_toolchain_matrix_failed"
    } else if missing_rows > 0 {
        "mct_llvm_toolchain_matrix_missing_tools"
    } else {
        "mct_llvm_toolchain_matrix_passed"
    };

    Ok(serde_json::json!({
        "schema": "nc.mct.llvm.toolchain_smoke.v1",
        "status": status,
        "file": path,
        "out_dir": out_dir.display().to_string(),
        "pipeline": ["emit-llvm", "llvm-as", "opt", "llc", "lld-compatible-link"],
        "tool_versions": {
            "llvm_as": llvm_as.as_ref().and_then(|tool| mct_tool_version(tool)),
            "opt": opt.as_ref().and_then(|tool| mct_tool_version(tool)),
            "llc": llc.as_ref().and_then(|tool| mct_tool_version(tool)),
            "ld_lld": mct_find_tool("ld.lld").as_ref().and_then(|tool| mct_tool_version(tool)),
            "lld_link": mct_find_tool("lld-link").as_ref().and_then(|tool| mct_tool_version(tool)),
            "ld64_lld": mct_find_tool("ld64.lld").as_ref().and_then(|tool| mct_tool_version(tool)),
            "wasm_ld": mct_find_tool("wasm-ld").as_ref().and_then(|tool| mct_tool_version(tool)),
        },
        "summary": {
            "rows": rows.len(),
            "passed": passed_rows,
            "missing_toolchain": missing_rows,
            "failed": failed_rows,
        },
        "rows": rows,
        "truth_boundary": {
            "uses_llvm_as": true,
            "uses_opt": true,
            "uses_llc": true,
            "uses_lld_family": true,
            "missing_tools_are_not_faked": true,
            "full_multi_host_ci_parity": false,
        }
    }))
}

pub(crate) fn run_mct_cpu_tuning_object_disasm_smoke(
    path: &str,
    out_dir: PathBuf,
    profile: ncc_ir::BackendLoweringProfileIR,
    pass_pipeline: ncc_ir::MctLlvmPassPipelineIR,
    microarchitecture_id: &str,
    fine_tuning_profile_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    fs::create_dir_all(&out_dir)?;
    let source = fs::read_to_string(path)?;
    let ast = load_ast_from_path(path)?;
    sema_check_with_warnings(&ast)?;
    let ir = ncc_ir::lower_with_backend_profile_and_source(ast, profile.clone(), path, &source);
    let target_attributes = ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
        &profile.target_spec,
        microarchitecture_id,
        fine_tuning_profile_id,
    )
    .map_err(|err| actionable_command_error("mct", err))?;
    let native_scheduler_bridge =
        ncc_ir::backend_native_cpu_tuning_bridge_for_mct_cpu(microarchitecture_id)
            .map_err(|err| actionable_command_error("mct", err))?;
    let native_selection_policy = ncc_ir::backend_native_cpu_tuning_policy_for_mct_cpu(
        microarchitecture_id,
        fine_tuning_profile_id,
    )
    .map_err(|err| actionable_command_error("mct", err))?;
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &ir,
        pass_pipeline.clone(),
        mct_llvm_debug_config_for_path(path),
        target_attributes.clone(),
    );

    let stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module");
    let tuned_stem = format!(
        "{}_{}_{}",
        stem,
        microarchitecture_id.replace(['.', '-'], "_"),
        fine_tuning_profile_id.replace(['.', '-'], "_")
    );
    let ll_path = out_dir.join(format!("{tuned_stem}.ll"));
    let bc_path = out_dir.join(format!("{tuned_stem}.bc"));
    let opt_bc_path = out_dir.join(format!("{tuned_stem}.opt.bc"));
    let obj_path = out_dir.join(format!("{tuned_stem}.o"));
    let disasm_path = out_dir.join(format!("{tuned_stem}.disasm.txt"));
    fs::write(&ll_path, module.llvm_ir.as_bytes())?;

    let llvm_as = mct_find_tool("llvm-as");
    let opt = mct_find_tool("opt");
    let llc = mct_find_tool("llc");
    let llvm_objdump = mct_find_tool("llvm-objdump");
    let mut missing_tools = Vec::new();
    if llvm_as.is_none() {
        missing_tools.push("llvm-as");
    }
    if opt.is_none() {
        missing_tools.push("opt");
    }
    if llc.is_none() {
        missing_tools.push("llc");
    }
    if llvm_objdump.is_none() {
        missing_tools.push("llvm-objdump");
    }
    if !missing_tools.is_empty() {
        return Ok(serde_json::json!({
            "schema": "nc.mct.cpu_tuning_object_disasm_smoke.v1",
            "status": "mct_cpu_tuning_object_disasm_missing_tools",
            "file": path,
            "out_dir": out_dir.display().to_string(),
            "missing_tools": missing_tools,
            "ll_path": ll_path.display().to_string(),
            "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
            "target_attributes": mct_llvm_target_attributes_to_json(&target_attributes),
            "native_scheduler_bridge": backend_native_cpu_tuning_bridge_to_json(&native_scheduler_bridge),
            "native_selection_policy": backend_native_cpu_tuning_policy_to_json(&native_selection_policy),
            "stages": [],
            "truth_boundary": {
                "missing_tools_are_not_faked": true,
                "full_native_scheduler_parity": native_scheduler_bridge.full_native_scheduler_parity,
            }
        }));
    }

    let llvm_as_tool = llvm_as.as_deref().expect("checked above");
    let opt_tool = opt.as_deref().expect("checked above");
    let llc_tool = llc.as_deref().expect("checked above");
    let llvm_objdump_tool = llvm_objdump.as_deref().expect("checked above");
    let mut stages = Vec::new();

    let llvm_as_args = vec![
        ll_path.display().to_string(),
        "-o".to_string(),
        bc_path.display().to_string(),
    ];
    stages.push(mct_stage_to_json(
        "llvm-as",
        Some(llvm_as_tool),
        &llvm_as_args,
        Command::new(llvm_as_tool).args(&llvm_as_args).output(),
    ));

    if stages.last().is_some_and(mct_stage_passed) {
        let opt_args = vec![
            format!("-passes={}", module.pass_pipeline.opt_passes_arg),
            bc_path.display().to_string(),
            "-o".to_string(),
            opt_bc_path.display().to_string(),
        ];
        stages.push(mct_stage_to_json(
            "opt",
            Some(opt_tool),
            &opt_args,
            Command::new(opt_tool).args(&opt_args).output(),
        ));
    }

    if stages.last().is_some_and(mct_stage_passed) {
        let mut llc_args = vec![
            format!("-mtriple={}", module.target_triple),
            format!("-mcpu={}", target_attributes.target_cpu),
        ];
        if !target_attributes.target_features.is_empty() {
            llc_args.push(format!(
                "-mattr={}",
                target_attributes.target_features.join(",")
            ));
        }
        llc_args.extend([
            "-filetype=obj".to_string(),
            opt_bc_path.display().to_string(),
            "-o".to_string(),
            obj_path.display().to_string(),
        ]);
        stages.push(mct_stage_to_json(
            "llc",
            Some(llc_tool),
            &llc_args,
            Command::new(llc_tool).args(&llc_args).output(),
        ));
    }

    if stages.last().is_some_and(mct_stage_passed) {
        let objdump_args = vec!["-d".to_string(), obj_path.display().to_string()];
        let objdump_output = Command::new(llvm_objdump_tool).args(&objdump_args).output();
        if let Ok(output) = objdump_output.as_ref() {
            fs::write(&disasm_path, &output.stdout)?;
        }
        stages.push(mct_stage_to_json(
            "llvm-objdump",
            Some(llvm_objdump_tool),
            &objdump_args,
            objdump_output,
        ));
    }

    let passed = stages.iter().all(mct_stage_passed);
    let status = if passed {
        "mct_cpu_tuning_object_disasm_passed"
    } else {
        "mct_cpu_tuning_object_disasm_failed"
    };
    let object_sha256 = if obj_path.exists() {
        Some(sha256_hex_file(&obj_path)?)
    } else {
        None
    };
    let disasm_sha256 = if disasm_path.exists() {
        Some(sha256_hex_file(&disasm_path)?)
    } else {
        None
    };

    Ok(serde_json::json!({
        "schema": "nc.mct.cpu_tuning_object_disasm_smoke.v1",
        "status": status,
        "file": path,
        "out_dir": out_dir.display().to_string(),
        "microarchitecture_id": microarchitecture_id,
        "fine_tuning_profile_id": fine_tuning_profile_id,
        "ll_path": ll_path.display().to_string(),
        "bitcode_path": bc_path.display().to_string(),
        "optimized_bitcode_path": opt_bc_path.display().to_string(),
        "object_path": obj_path.display().to_string(),
        "disasm_path": disasm_path.display().to_string(),
        "object_exists": obj_path.exists(),
        "disasm_exists": disasm_path.exists(),
        "object_sha256": object_sha256,
        "disasm_sha256": disasm_sha256,
        "module": mct_llvm_module_to_json(&module, false, Some(&ll_path)),
        "target_attributes": mct_llvm_target_attributes_to_json(&target_attributes),
        "native_scheduler_bridge": backend_native_cpu_tuning_bridge_to_json(&native_scheduler_bridge),
        "native_selection_policy": backend_native_cpu_tuning_policy_to_json(&native_selection_policy),
        "tool_versions": {
            "llvm_as": llvm_as.as_ref().and_then(|tool| mct_tool_version(tool)),
            "opt": opt.as_ref().and_then(|tool| mct_tool_version(tool)),
            "llc": llc.as_ref().and_then(|tool| mct_tool_version(tool)),
            "llvm_objdump": llvm_objdump.as_ref().and_then(|tool| mct_tool_version(tool)),
        },
        "stages": stages,
        "truth_boundary": {
            "uses_llvm_as": true,
            "uses_opt": true,
            "uses_llc_object_emission": true,
            "uses_llvm_objdump_disasm": true,
            "target_cpu_applied_to_llc": true,
            "target_features_applied_to_llc": true,
            "full_native_scheduler_parity": native_scheduler_bridge.full_native_scheduler_parity,
        }
    }))
}

pub(crate) fn run_mct_cpu_tuning_object_disasm_matrix_smoke(
    path: &str,
    out_dir: PathBuf,
    rows: Vec<MctCpuTuningObjectDisasmMatrixRow>,
    pass_pipeline: ncc_ir::MctLlvmPassPipelineIR,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    fs::create_dir_all(&out_dir)?;
    let mut row_results = Vec::new();

    for row in rows {
        let row_out_dir = out_dir.join(row.id);
        let result = run_mct_cpu_tuning_object_disasm_smoke(
            path,
            row_out_dir,
            row.profile.clone(),
            pass_pipeline.clone(),
            row.microarchitecture_id,
            row.fine_tuning_profile_id,
        )?;
        let result_status = result
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("mct_cpu_tuning_object_disasm_unknown");
        let row_status = match result_status {
            "mct_cpu_tuning_object_disasm_passed" => "passed",
            "mct_cpu_tuning_object_disasm_missing_tools" => "missing_toolchain",
            _ => "failed",
        };
        let native_bridge_status = result
            .get("native_scheduler_bridge")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            .unwrap_or("missing");

        row_results.push(serde_json::json!({
            "id": row.id,
            "status": row_status,
            "profile": row.profile_label,
            "target_triple": row.profile.target_spec.target_triple,
            "object_format": render_backend_object_format_label(&row.profile.target_spec.object_format),
            "abi": render_backend_abi_label(&row.profile.target_spec.abi),
            "microarchitecture_id": row.microarchitecture_id,
            "fine_tuning_profile_id": row.fine_tuning_profile_id,
            "native_scheduler_bridge_status": native_bridge_status,
            "result": result,
        }));
    }

    let passed_rows = row_results
        .iter()
        .filter(|row| row.get("status").and_then(|value| value.as_str()) == Some("passed"))
        .count();
    let missing_rows = row_results
        .iter()
        .filter(|row| {
            row.get("status").and_then(|value| value.as_str()) == Some("missing_toolchain")
        })
        .count();
    let failed_rows = row_results
        .iter()
        .filter(|row| row.get("status").and_then(|value| value.as_str()) == Some("failed"))
        .count();
    let unsupported_native_bridge_rows = row_results
        .iter()
        .filter(|row| {
            row.get("native_scheduler_bridge_status")
                .and_then(|value| value.as_str())
                == Some("backend_native_cpu_tuning_bridge_unsupported")
        })
        .count();
    let status = if failed_rows > 0 {
        "mct_cpu_tuning_object_disasm_matrix_failed"
    } else if missing_rows > 0 {
        "mct_cpu_tuning_object_disasm_matrix_missing_tools"
    } else {
        "mct_cpu_tuning_object_disasm_matrix_passed"
    };

    Ok(serde_json::json!({
        "schema": "nc.mct.cpu_tuning_object_disasm_matrix_smoke.v1",
        "status": status,
        "file": path,
        "out_dir": out_dir.display().to_string(),
        "pipeline": ["emit-llvm", "llvm-as", "opt", "llc-object", "llvm-objdump"],
        "tool_versions": {
            "llvm_as": mct_find_tool("llvm-as").as_ref().and_then(|tool| mct_tool_version(tool)),
            "opt": mct_find_tool("opt").as_ref().and_then(|tool| mct_tool_version(tool)),
            "llc": mct_find_tool("llc").as_ref().and_then(|tool| mct_tool_version(tool)),
            "llvm_objdump": mct_find_tool("llvm-objdump").as_ref().and_then(|tool| mct_tool_version(tool)),
        },
        "summary": {
            "rows": row_results.len(),
            "passed": passed_rows,
            "missing_toolchain": missing_rows,
            "failed": failed_rows,
            "unsupported_native_bridge": unsupported_native_bridge_rows,
        },
        "rows": row_results,
        "truth_boundary": {
            "uses_llvm_as": true,
            "uses_opt": true,
            "uses_llc_object_emission": true,
            "uses_llvm_objdump_disasm": true,
            "object_disasm_only": true,
            "link_execution_not_claimed": true,
            "missing_tools_are_not_faked": true,
            "full_multi_host_ci_parity": false,
        }
    }))
}

pub(crate) fn parse_ncc_execution_profile(
    raw: &str,
) -> Result<NccExecutionProfile, Box<dyn std::error::Error>> {
    match raw.to_ascii_lowercase().as_str() {
        "debug" | "dev" | "runtime" | "interpreter" | "interpreted" => {
            Ok(NccExecutionProfile::Debug)
        }
        "release" | "native" => Ok(NccExecutionProfile::Release),
        "standard" | "release-fast" | "fast" | "native-fast" | "real" | "backend-real" => {
            Ok(NccExecutionProfile::ReleaseFast)
        }
        "release-small" | "small" | "size" => Ok(NccExecutionProfile::ReleaseSmall),
        "unsafe-fast" | "raw-fast" => Ok(NccExecutionProfile::UnsafeFast),
        "instrumented" | "checked" | "checks" => Ok(NccExecutionProfile::Instrumented),
        _ => Err(format!(
            "unknown ncc execution profile '{}'; expected standard, debug, interpreted, real, release, release-fast, release-small, unsafe-fast or instrumented",
            raw
        )
        .into()),
    }
}

pub(crate) fn parse_ncc_profile_value(
    args: &[String],
    idx: usize,
    command: &str,
) -> Result<NccExecutionProfile, Box<dyn std::error::Error>> {
    let value = args.get(idx + 1).ok_or_else(|| {
        actionable_command_error(
            command,
            format!("{command} syntax: --profile requires a value"),
        )
    })?;
    parse_ncc_execution_profile(value)
}

pub(crate) fn parse_optional_ncc_execution_profile(
    args: &[String],
    command: &str,
    default: NccExecutionProfile,
) -> Result<NccExecutionProfile, Box<dyn std::error::Error>> {
    let mut selected = None;
    let mut idx = 0usize;
    while idx < args.len() {
        if args[idx] == "--profile" {
            if selected.is_some() {
                return Err(format!(
                    "duplicate --profile option is not allowed in {command} command"
                )
                .into());
            }
            selected = Some(parse_ncc_profile_value(args, idx, command)?);
            idx += 2;
            continue;
        }
        idx += 1;
    }
    Ok(selected.unwrap_or(default))
}

pub(crate) fn official_toolchain_channels() -> &'static [&'static str] {
    &["stable", "beta", "nightly"]
}

pub(crate) fn official_toolchain_components() -> &'static [&'static str] {
    &[
        "compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs", "sources",
    ]
}

pub(crate) fn official_install_profile_components(
    profile: &str,
) -> Option<&'static [&'static str]> {
    match profile {
        "minimal" => Some(&["compiler", "runtime", "stdlib"]),
        "default" => Some(&[
            "compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs",
        ]),
        "complete" => Some(&[
            "compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs", "sources",
        ]),
        _ => None,
    }
}

pub(crate) fn phase10_p2_owned_scope_artifacts() -> &'static [&'static str] {
    &[
        "docs/spec/release/v2_0_0_phase10_toolchain_model_spec.md",
        "docs/spec/release/v2_0_0_phase10_toolchain_model_compatibility_spec.md",
        "docs/spec/release/v2_0_0_phase10_toolchain_model_contract.json",
        "docs/status/STATUS_PHASE10_punto2_toolchain_model_v2_0_0.md",
        "docs/status/F10_P2_toolchain_model_report.json",
        "tools/release/snapshot_phase10_toolchain_model_v2_0_0.sh",
        "tools/release/qa_phase10_p2_v2_0_0.sh",
        "tools/release/promote_release_train_v2_0_0.sh",
        "docs/spec/release/v2_0_0_release_train_contract.json",
        "docs/spec/release/v2_0_0_release_train_spec.md",
        "docs/status/STATUS_PHASE10_release_train_v2_0_0.md",
    ]
}

pub(crate) fn phase10_p2_locked_external_artifacts() -> &'static [&'static str] {
    &[
        "docs/status/F10_P1_low_level_extension_migration_wave1_report.json",
        "docs/status/F10_P1_low_level_extension_migration_wave2_report.json",
        "docs/status/F10_P1_low_level_extension_migration_wave3_report.json",
        "docs/status/F10_P1_low_level_extension_migration_wave4_report.json",
        "docs/status/F10_P1_distribution_report.json",
        "docs/status/F10_P1_clean_install_report.json",
    ]
}

pub(crate) fn render_backend_support_tier_label(
    tier: &ncc_ir::BackendSupportTierIR,
) -> &'static str {
    match tier {
        ncc_ir::BackendSupportTierIR::Tier1 => "tier1",
        ncc_ir::BackendSupportTierIR::Tier2 => "tier2",
        ncc_ir::BackendSupportTierIR::Tier3 => "tier3",
    }
}

pub(crate) fn render_backend_target_family_label(
    family: &ncc_ir::BackendTargetFamilyIR,
) -> &'static str {
    match family {
        ncc_ir::BackendTargetFamilyIR::Hosted => "hosted",
        ncc_ir::BackendTargetFamilyIR::Portable => "portable",
        ncc_ir::BackendTargetFamilyIR::Freestanding => "freestanding",
        ncc_ir::BackendTargetFamilyIR::Sovereign => "sovereign",
    }
}

pub(crate) fn render_backend_runtime_model_label(
    model: &ncc_ir::BackendRuntimeModelIR,
) -> &'static str {
    match model {
        ncc_ir::BackendRuntimeModelIR::Hosted => "hosted",
        ncc_ir::BackendRuntimeModelIR::Portable => "portable",
        ncc_ir::BackendRuntimeModelIR::Freestanding => "freestanding",
        ncc_ir::BackendRuntimeModelIR::Sovereign => "sovereign",
    }
}

pub(crate) fn official_toolchain_target_specs() -> Vec<ncc_ir::BackendTargetSpecIR> {
    let aarch64 = ncc_ir::BackendTargetIR::AArch64;
    let x86_64 = ncc_ir::BackendTargetIR::X86_64;
    let aapcs64 = ncc_ir::BackendCallingConventionIR::NcAapcs64;
    let sysv64 = ncc_ir::BackendCallingConventionIR::NcSysV64;
    let win64 = ncc_ir::BackendCallingConventionIR::NcWin64;

    vec![
        ncc_ir::BackendLoweringProfileIR::nc_sysv64().target_spec,
        ncc_ir::BackendLoweringProfileIR::nc_win64().target_spec,
        ncc_ir::BackendLoweringProfileIR::nc_aapcs64().target_spec,
        ncc_ir::BackendLoweringProfileIR::nc_portable().target_spec,
        ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(&aarch64, &sysv64),
        ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(&x86_64, &aapcs64),
        ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(&aarch64, &win64),
        ncc_ir::BackendLoweringProfileIR::nc_sovereign().target_spec,
    ]
}

pub(crate) fn toolchain_channels_for_tier(
    tier: &ncc_ir::BackendSupportTierIR,
) -> &'static [&'static str] {
    match tier {
        ncc_ir::BackendSupportTierIR::Tier1 => &["stable", "beta", "nightly"],
        ncc_ir::BackendSupportTierIR::Tier2 => &["beta", "nightly"],
        ncc_ir::BackendSupportTierIR::Tier3 => &["nightly"],
    }
}

pub(crate) fn build_toolchain_model_manifest() -> serde_json::Value {
    let channels = official_toolchain_channels();
    let components = official_toolchain_components();
    let targets = official_toolchain_target_specs();

    let target_rows: Vec<serde_json::Value> = targets
        .iter()
        .map(|spec| {
            serde_json::json!({
                "target": spec.target_triple,
                "tier": render_backend_support_tier_label(&spec.support_tier),
                "family": render_backend_target_family_label(&spec.target_family),
                "runtime_model": render_backend_runtime_model_label(&spec.runtime_model),
                "channels": toolchain_channels_for_tier(&spec.support_tier),
            })
        })
        .collect();

    let mut toolchains: Vec<serde_json::Value> = Vec::new();
    for spec in &targets {
        for channel in toolchain_channels_for_tier(&spec.support_tier) {
            toolchains.push(serde_json::json!({
                "id": format!("{}-{}", channel, spec.target_triple),
                "channel": channel,
                "target": spec.target_triple,
                "tier": render_backend_support_tier_label(&spec.support_tier),
                "components": components,
                "install_profiles": ["minimal", "default", "complete"],
            }));
        }
    }

    serde_json::json!({
        "version": "1.0.1",
        "state": "frozen",
        "phase": "phase2_toolchain_model",
        "channels": channels,
        "targets": target_rows,
        "components": components,
        "install_profiles": {
            "minimal": official_install_profile_components("minimal").expect("minimal profile must exist"),
            "default": official_install_profile_components("default").expect("default profile must exist"),
            "complete": official_install_profile_components("complete").expect("complete profile must exist"),
        },
        "toolchains": toolchains,
    })
}

pub(crate) fn build_phase10_toolchain_model_manifest() -> serde_json::Value {
    let phase2 = build_toolchain_model_manifest();
    let channels = phase2
        .get("channels")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(official_toolchain_channels()));
    let targets = phase2
        .get("targets")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let components = phase2
        .get("components")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(official_toolchain_components()));
    let toolchains = phase2
        .get("toolchains")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "schema": "nc.phase10.toolchain.model.v1",
        "version": "2.0.0",
        "state": "frozen",
        "series": "2.x",
        "phase": "phase10_toolchain_model",
        "channels": channels,
        "targets": targets,
        "components": components,
        "install_profiles": {
            "minimal": official_install_profile_components("minimal").expect("minimal profile must exist"),
            "default": official_install_profile_components("default").expect("default profile must exist"),
            "complete": official_install_profile_components("complete").expect("complete profile must exist"),
        },
        "toolchain_id_format": "<channel>-<target>",
        "toolchains": toolchains,
        "ncup_integration": {
            "bootstrap_binary": "ncup-init",
            "persistent_manager": "ncup",
            "required_commands": [
                "ncup init",
                "ncup toolchain list",
                "ncup toolchain list --available",
                "ncup toolchain install <id> [--profile <minimal|default|complete>] [--set-default]",
                "ncup toolchain install --channel <stable|beta|nightly> --target <triple> [--profile <minimal|default|complete>] [--set-default]",
                "ncup toolchain default <id>",
                "ncup doctor"
            ]
        },
        "owned_scope_artifacts": phase10_p2_owned_scope_artifacts(),
        "locked_external_artifacts": phase10_p2_locked_external_artifacts(),
        "no_breaking_without_rfc": true,
        "required_gates": [
            "release_phase10_toolchain_model_gate_tests",
            "release_phase10_installer_gate_tests",
            "release_phase10_release_train_gate_tests",
            "release_phase2_toolchain_model_gate_tests"
        ]
    })
}

pub(crate) fn official_runtime_stdlib_modules() -> &'static [&'static str] {
    &[
        "io",
        "mem",
        "collections",
        "net",
        "time",
        "trace",
        "inspect",
    ]
}

pub(crate) fn official_runtime_stdlib_runtime_models() -> &'static [&'static str] {
    &["hosted", "portable", "freestanding"]
}

pub(crate) fn build_runtime_stdlib_manifest() -> serde_json::Value {
    serde_json::json!({
        "version": "1.0.1",
        "state": "frozen",
        "phase": "phase3_runtime_stdlib",
        "series": "1.x",
        "runtime_models": official_runtime_stdlib_runtime_models(),
        "modules": official_runtime_stdlib_modules(),
        "contracts": {
            "io": {
                "stable_api": ["system.io.core.write(str)->bool"],
                "contexts": ["@runtime", "@system"],
            },
            "mem": {
                "stable_api": [
                    "memory.heap.manager.alloc(int)->addr",
                    "memory.heap.manager.free(addr)->bool",
                    "memory.heap.manager.realloc(addr,int)->addr",
                    "memory.heap.manager.zero(addr,int)->bool"
                ],
                "contexts": ["@runtime", "@"],
            },
            "collections": {
                "stable_api": [
                    "list<T>.push/get/set/len",
                    "queue<T>.enqueue/dequeue/front/isEmpty",
                    "watchList<T>.push/len",
                    "traceList<T>.push/len",
                    "memList<T>.push/get/set/len"
                ],
                "contexts": ["@", "@runtime", "@system", "@network", "@tooling", "@compiler", "@kernel", "@ai"],
            },
            "net": {
                "stable_api": [
                    "system.net.core.connect(str,int)->bool",
                    "system.net.core.listen()->bool",
                    "system.net.core.accept()->bool",
                    "system.net.core.send(str)->bool",
                    "system.net.core.recv()->bool",
                    "system.net.core.disconnect()->bool",
                    "alt.net.core.* mirrors system.net.core"
                ],
                "contexts": ["@runtime", "@network"],
            },
            "time": {
                "stable_api": [
                    "runtime.time.now()->int",
                    "runtime.time.monotonic()->int",
                    "runtime.time.sleep(int)->bool"
                ],
                "contexts": ["@runtime", "@system", "@"],
            },
            "trace": {
                "stable_api": [
                    "runtime.trace.mark(str)->bool",
                    "runtime.alias.mark(str)->bool"
                ],
                "contexts": ["@", "@runtime", "@inspect", "@system", "@network", "@tooling", "@compiler", "@kernel", "@ai"],
            },
            "inspect": {
                "stable_api": [
                    "inspect runtime paths: heap.*, net.*, system.*, kernel.*, security.*, ai.*",
                    "print.inspect/print.mem/print.st"
                ],
                "contexts": ["@inspect", "@runtime", "@"],
            }
        },
        "alias_surface": {
            "nC.runtime.heap": "memory.heap.manager",
            "nC.runtime.net": "system.net.core|alt.net.core",
            "nC.runtime.trace": "runtime.trace",
            "nC.runtime.alias": "runtime.alias",
            "nC.runtime.time": "runtime.time"
        },
        "no_breaking_without_rfc": true,
        "required_gates": [
            "release_phase3_runtime_stdlib_gate_tests",
            "release_phase2_toolchain_model_gate_tests",
            "release_phase1_language_base_gate_tests"
        ]
    })
}

pub(crate) fn render_backend_target_label(target: &ncc_ir::BackendTargetIR) -> &'static str {
    match target {
        ncc_ir::BackendTargetIR::X86_64 => "x86_64",
        ncc_ir::BackendTargetIR::AArch64 => "aarch64",
        ncc_ir::BackendTargetIR::Wasm32 => "wasm32",
    }
}

pub(crate) fn render_backend_calling_convention_label(
    calling_convention: &ncc_ir::BackendCallingConventionIR,
) -> &'static str {
    match calling_convention {
        ncc_ir::BackendCallingConventionIR::NcSysV64 => "nC.sysv64",
        ncc_ir::BackendCallingConventionIR::NcWin64 => "nC.win64",
        ncc_ir::BackendCallingConventionIR::NcAapcs64 => "nC.aapcs64",
        ncc_ir::BackendCallingConventionIR::NcPortable => "nC.portable",
        ncc_ir::BackendCallingConventionIR::NcSovereign => "nC.sovereign",
    }
}

pub(crate) fn render_backend_isa_label(isa: &ncc_ir::BackendIsaIR) -> &'static str {
    match isa {
        ncc_ir::BackendIsaIR::X86 => "x86",
        ncc_ir::BackendIsaIR::X86_64 => "x86_64",
        ncc_ir::BackendIsaIR::ArmV7 => "armv7",
        ncc_ir::BackendIsaIR::AArch64 => "aarch64",
        ncc_ir::BackendIsaIR::Wasm32 => "wasm32",
        ncc_ir::BackendIsaIR::RiscV32 => "riscv32",
        ncc_ir::BackendIsaIR::RiscV64 => "riscv64",
    }
}

pub(crate) fn render_backend_abi_label(abi: &ncc_ir::BackendAbiModelIR) -> &'static str {
    match abi {
        ncc_ir::BackendAbiModelIR::I386SysV => "i386-sysv",
        ncc_ir::BackendAbiModelIR::I386Windows => "i386-msvc",
        ncc_ir::BackendAbiModelIR::SysV64 => "sysv64",
        ncc_ir::BackendAbiModelIR::Win64 => "win64",
        ncc_ir::BackendAbiModelIR::Aapcs32 => "aapcs32",
        ncc_ir::BackendAbiModelIR::Aapcs64 => "aapcs64",
        ncc_ir::BackendAbiModelIR::RiscV64 => "riscv64",
        ncc_ir::BackendAbiModelIR::Ilp32 => "ilp32",
        ncc_ir::BackendAbiModelIR::Wasi => "wasi",
        ncc_ir::BackendAbiModelIR::Freestanding => "freestanding",
        ncc_ir::BackendAbiModelIR::NcSovereign => "nc-sovereign",
    }
}

pub(crate) fn render_backend_object_format_label(
    format: &ncc_ir::BackendObjectFormatIR,
) -> &'static str {
    match format {
        ncc_ir::BackendObjectFormatIR::Elf32 => "elf32",
        ncc_ir::BackendObjectFormatIR::Elf64 => "elf64",
        ncc_ir::BackendObjectFormatIR::Coff => "coff",
        ncc_ir::BackendObjectFormatIR::MachO => "macho",
        ncc_ir::BackendObjectFormatIR::WasmModule => "wasm-module",
        ncc_ir::BackendObjectFormatIR::FreestandingImage => "freestanding-image",
    }
}

pub(crate) fn render_backend_relocation_model_label(
    model: &ncc_ir::BackendRelocationModelIR,
) -> &'static str {
    match model {
        ncc_ir::BackendRelocationModelIR::Static => "static",
        ncc_ir::BackendRelocationModelIR::Pic => "pic",
        ncc_ir::BackendRelocationModelIR::DynamicNoPic => "dynamic-nopic",
        ncc_ir::BackendRelocationModelIR::Absolute => "absolute",
    }
}

pub(crate) fn render_backend_code_model_label(model: &ncc_ir::BackendCodeModelIR) -> &'static str {
    match model {
        ncc_ir::BackendCodeModelIR::Small => "small",
        ncc_ir::BackendCodeModelIR::Kernel => "kernel",
        ncc_ir::BackendCodeModelIR::Large => "large",
    }
}

pub(crate) fn render_backend_endianness_label(
    endianness: &ncc_ir::BackendEndiannessIR,
) -> &'static str {
    match endianness {
        ncc_ir::BackendEndiannessIR::Little => "little",
        ncc_ir::BackendEndiannessIR::Big => "big",
    }
}

pub(crate) fn render_backend_host_services_label(
    host_services: &ncc_ir::BackendHostServicesIR,
) -> &'static str {
    match host_services {
        ncc_ir::BackendHostServicesIR::Os => "os",
        ncc_ir::BackendHostServicesIR::Wasi => "wasi",
        ncc_ir::BackendHostServicesIR::BareMetal => "bare-metal",
        ncc_ir::BackendHostServicesIR::SovereignBridge => "sovereign-bridge",
    }
}

pub(crate) fn render_backend_debug_info_model_label(
    model: &ncc_ir::BackendDebugInfoModelIR,
) -> &'static str {
    match model {
        ncc_ir::BackendDebugInfoModelIR::Dwarf => "dwarf",
        ncc_ir::BackendDebugInfoModelIR::Pdb => "pdb",
        ncc_ir::BackendDebugInfoModelIR::Compact => "compact",
        ncc_ir::BackendDebugInfoModelIR::None => "none",
    }
}

pub(crate) fn render_backend_linker_model_label(
    model: &ncc_ir::BackendLinkerModelIR,
) -> &'static str {
    match model {
        ncc_ir::BackendLinkerModelIR::HostNative => "host-native",
        ncc_ir::BackendLinkerModelIR::WasmLld => "wasm-lld",
        ncc_ir::BackendLinkerModelIR::FreestandingInternal => "freestanding-internal",
        ncc_ir::BackendLinkerModelIR::SovereignInternal => "sovereign-internal",
    }
}

pub(crate) fn render_backend_kind_label(kind: &ncc_ir::BackendKindIR) -> &'static str {
    match kind {
        ncc_ir::BackendKindIR::HostedProduction => "hosted-production",
        ncc_ir::BackendKindIR::Portable => "portable",
        ncc_ir::BackendKindIR::Freestanding => "freestanding",
        ncc_ir::BackendKindIR::SovereignNc => "sovereign-nc",
    }
}

pub(crate) fn render_backend_profile_constraint_label(
    constraint: &ncc_ir::BackendProfileConstraintIR,
) -> &'static str {
    match constraint {
        ncc_ir::BackendProfileConstraintIR::RequiresHostedRuntime => "requires-hosted-runtime",
        ncc_ir::BackendProfileConstraintIR::RequiresPortableRuntime => "requires-portable-runtime",
        ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime => {
            "requires-freestanding-runtime"
        }
        ncc_ir::BackendProfileConstraintIR::RequiresSovereignRuntime => {
            "requires-sovereign-runtime"
        }
    }
}

pub(crate) fn render_backend_legalization_issue_label(
    issue: &ncc_ir::BackendLegalizationIssueIR,
) -> &'static str {
    match issue {
        ncc_ir::BackendLegalizationIssueIR::IntegerWidthNonNative => "integer-width-non-native",
        ncc_ir::BackendLegalizationIssueIR::AggregatePassing => "aggregate-passing",
        ncc_ir::BackendLegalizationIssueIR::VectorSurfaceUnsupported => {
            "vector-surface-unsupported"
        }
        ncc_ir::BackendLegalizationIssueIR::RuntimeBuiltinBoundary => "runtime-builtin-boundary",
        ncc_ir::BackendLegalizationIssueIR::PrimitiveBridgeHostedConflict => {
            "primitive-bridge-hosted-conflict"
        }
        ncc_ir::BackendLegalizationIssueIR::RelocationModelUnsupported => {
            "relocation-model-unsupported"
        }
        ncc_ir::BackendLegalizationIssueIR::PointerRegionBoundary => "pointer-region-boundary",
        ncc_ir::BackendLegalizationIssueIR::ConsoleRenderSurface => "console-render-surface",
        ncc_ir::BackendLegalizationIssueIR::StructuredControlFlowSurface => {
            "structured-control-flow-surface"
        }
    }
}

pub(crate) fn render_backend_legalization_action_label(
    action: &ncc_ir::BackendLegalizationActionIR,
) -> &'static str {
    match action {
        ncc_ir::BackendLegalizationActionIR::KeepNative => "keep-native",
        ncc_ir::BackendLegalizationActionIR::RewriteByRef => "rewrite-by-ref",
        ncc_ir::BackendLegalizationActionIR::ExpandToScalarOps => "expand-to-scalar-ops",
        ncc_ir::BackendLegalizationActionIR::LowerToRuntimeCall => "lower-to-runtime-call",
        ncc_ir::BackendLegalizationActionIR::Reject => "reject",
    }
}

pub(crate) fn official_backend_profiles() -> Vec<(&'static str, ncc_ir::BackendLoweringProfileIR)> {
    vec![
        ("nC.sysv64", ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
        ("nC.win64", ncc_ir::BackendLoweringProfileIR::nc_win64()),
        ("nC.aapcs64", ncc_ir::BackendLoweringProfileIR::nc_aapcs64()),
        (
            "nC.portable",
            ncc_ir::BackendLoweringProfileIR::nc_portable(),
        ),
        (
            "nC.sovereign",
            ncc_ir::BackendLoweringProfileIR::nc_sovereign(),
        ),
        (
            "nC.destructive",
            ncc_ir::BackendLoweringProfileIR::nc_destructive(),
        ),
    ]
}
