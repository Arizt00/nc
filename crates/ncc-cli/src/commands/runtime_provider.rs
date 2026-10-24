use super::super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

const PROVIDER_SOURCE: &str = include_str!("../../../../runtime/nc/stage3_provider.n");
const PROVIDER_SOURCE_PATH: &str = "runtime/nc/stage3_provider.n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderKind {
    Pe64,
    Elf64,
    MachO64,
    Rv32Archive,
    WasmImports,
}

#[derive(Clone, Copy, Debug)]
struct ProviderTarget {
    id: &'static str,
    triple: &'static str,
    file_name: &'static str,
    kind: ProviderKind,
}

const PROVIDER_TARGETS: &[ProviderTarget] = &[
    ProviderTarget {
        id: "windows-x86_64",
        triple: "x86_64-pc-windows-msvc",
        file_name: "ncruntime.dll",
        kind: ProviderKind::Pe64,
    },
    ProviderTarget {
        id: "linux-x86_64",
        triple: "x86_64-unknown-linux-gnu",
        file_name: "libncruntime.so.2",
        kind: ProviderKind::Elf64,
    },
    ProviderTarget {
        id: "darwin-x86_64",
        triple: "x86_64-apple-darwin",
        file_name: "libncruntime.2.dylib",
        kind: ProviderKind::MachO64,
    },
    ProviderTarget {
        id: "rv32-static",
        triple: "riscv32imac-unknown-elf",
        file_name: "libncruntime-rv32.a",
        kind: ProviderKind::Rv32Archive,
    },
    ProviderTarget {
        id: "wasm32-imports",
        triple: "wasm32-wasi",
        file_name: "ncruntime.imports.wasm",
        kind: ProviderKind::WasmImports,
    },
];

#[derive(Debug)]
struct ProviderOptions {
    action: String,
    target: Option<String>,
    out_dir: PathBuf,
    json: bool,
}

pub(crate) fn help_text() -> &'static str {
    "Usage: ncc runtime provider <plan|build|verify|all> [--target <triple|id>] [--out <dir>] [--json]\n\
     Materialize distributable runtime providers from nC source through MCT.\n\
     Outputs: ncruntime.dll, libncruntime.so.2, libncruntime.2.dylib, libncruntime-rv32.a and ncruntime.imports.wasm"
}

pub(crate) fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || is_help_token(&args[0]) {
        println!("{}", help_text());
        return Ok(());
    }
    let opts = parse_options(args)?;
    let targets = selected_targets(opts.target.as_deref())?;
    let payload = match opts.action.as_str() {
        "plan" => provider_plan(&targets),
        "build" => {
            if targets.len() != 1 {
                return Err("runtime provider build requires exactly one --target".into());
            }
            materialize_provider(targets[0], &opts.out_dir)?
        }
        "verify" => verify_providers(&targets, &opts.out_dir)?,
        "all" => {
            let mut artifacts = Vec::new();
            for target in targets {
                artifacts.push(materialize_provider(target, &opts.out_dir)?);
            }
            serde_json::json!({
                "schema": "nc.runtime.provider.bundle.v1",
                "status": "materialized",
                "source": PROVIDER_SOURCE_PATH,
                "source_sha256": crate::content_cache::sha256_bytes(PROVIDER_SOURCE.as_bytes()),
                "artifacts": artifacts,
            })
        }
        other => return Err(format!("unknown runtime provider action '{other}'").into()),
    };
    if opts.action == "build" || opts.action == "all" {
        fs::create_dir_all(&opts.out_dir)?;
        fs::write(
            opts.out_dir.join("ncruntime.providers.json"),
            serde_json::to_vec_pretty(&payload)?,
        )?;
    }
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "runtime provider {}: {}",
            opts.action,
            payload
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ok")
        );
    }
    if opts.action == "verify"
        && payload.get("status").and_then(serde_json::Value::as_str) != Some("passed")
    {
        return Err("runtime provider verification failed".into());
    }
    Ok(())
}

fn parse_options(args: &[String]) -> Result<ProviderOptions, Box<dyn std::error::Error>> {
    let action = args[0].to_ascii_lowercase();
    let mut target = None;
    let mut out_dir = PathBuf::from("target/bootstrap/stage2");
    let mut json = false;
    let mut at = 1usize;
    while at < args.len() {
        match args[at].as_str() {
            "--target" => {
                target = Some(
                    args.get(at + 1)
                        .ok_or("runtime provider --target requires a value")?
                        .clone(),
                );
                at += 2;
            }
            "--out" | "--out-dir" => {
                out_dir = PathBuf::from(
                    args.get(at + 1)
                        .ok_or("runtime provider --out requires a directory")?,
                );
                at += 2;
            }
            "--json" => {
                json = true;
                at += 1;
            }
            other => return Err(format!("unknown runtime provider option '{other}'").into()),
        }
    }
    Ok(ProviderOptions {
        action,
        target,
        out_dir,
        json,
    })
}

fn selected_targets(
    query: Option<&str>,
) -> Result<Vec<ProviderTarget>, Box<dyn std::error::Error>> {
    let Some(query) = query else {
        return Ok(PROVIDER_TARGETS.to_vec());
    };
    let selected = PROVIDER_TARGETS
        .iter()
        .copied()
        .filter(|target| {
            target.id.eq_ignore_ascii_case(query) || target.triple.eq_ignore_ascii_case(query)
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!("unknown runtime provider target '{query}'").into());
    }
    Ok(selected)
}

fn provider_plan(targets: &[ProviderTarget]) -> serde_json::Value {
    serde_json::json!({
        "schema": "nc.runtime.provider.plan.v1",
        "status": "ready",
        "source": PROVIDER_SOURCE_PATH,
        "source_language": "nC",
        "c_translation_units": 0,
        "source_sha256": crate::content_cache::sha256_bytes(PROVIDER_SOURCE.as_bytes()),
        "abi": super::runtime::runtime_abi_manifest(),
        "targets": targets.iter().map(|target| serde_json::json!({
            "id": target.id,
            "triple": target.triple,
            "file": target.file_name,
            "kind": provider_kind_label(target.kind),
        })).collect::<Vec<_>>(),
    })
}

fn provider_kind_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Pe64 => "pe64-dynamic-library",
        ProviderKind::Elf64 => "elf64-shared-object",
        ProviderKind::MachO64 => "macho64-dylib",
        ProviderKind::Rv32Archive => "elf32-static-archive",
        ProviderKind::WasmImports => "wasm32-typed-import-module",
    }
}

fn materialize_provider(
    target: ProviderTarget,
    out_dir: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;
    let output = out_dir.join(target.file_name);
    let required = provider_required_symbols();
    if target.kind == ProviderKind::WasmImports {
        let bytes = write_wasm_import_module(&required)?;
        fs::write(&output, &bytes)?;
        return write_artifact_manifest(target, &output, &bytes, &required, "typed-imports");
    }

    let profile = provider_profile(target)?;
    let object = out_dir.join(format!("{}.provider.o", target.id));
    let llvm = out_dir.join(format!("{}.provider.ll", target.id));
    let object_bytes = compile_nc_provider_object(&profile, &object, &llvm)?;
    match target.kind {
        ProviderKind::Pe64 => link_pe_provider(&object, &output, &required)?,
        ProviderKind::Elf64 => link_elf_provider(&object, &output, &required)?,
        ProviderKind::MachO64 => link_macho_provider(&object, &output, &required)?,
        ProviderKind::Rv32Archive => archive_rv32_provider(&object, &output, &required)?,
        ProviderKind::WasmImports => unreachable!(),
    }
    let bytes = fs::read(&output)?;
    if bytes.is_empty() {
        return Err(format!("runtime provider '{}' is empty", output.display()).into());
    }
    let mode = if target.kind == ProviderKind::Rv32Archive {
        "static-provider"
    } else {
        "dynamic-provider"
    };
    let mut manifest = write_artifact_manifest(target, &output, &bytes, &required, mode)?;
    manifest["object"] = serde_json::json!({
        "path": object.display().to_string(),
        "bytes": object_bytes.len(),
        "sha256": crate::content_cache::sha256_bytes(&object_bytes),
        "llvm": llvm.display().to_string(),
    });
    Ok(manifest)
}

fn provider_profile(
    target: ProviderTarget,
) -> Result<ncc_ir::BackendLoweringProfileIR, Box<dyn std::error::Error>> {
    if target.kind != ProviderKind::Rv32Archive {
        return super::compiler::compiler_native_build_profile_from_target(target.triple);
    }
    let mut profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
    profile.target_spec.target_triple = target.triple.to_string();
    profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
    profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV32;
    profile.target_spec.abi = ncc_ir::BackendAbiModelIR::Ilp32;
    profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
    profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
    profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
    profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::ilp32();
    profile.target_spec.pointer_width = 32;
    profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
    profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
    profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
    profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
    profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
    profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
    profile.target_spec.profile_constraints =
        vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
    Ok(profile)
}

fn compile_nc_provider_object(
    profile: &ncc_ir::BackendLoweringProfileIR,
    object: &Path,
    llvm_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut program =
        super::project::project_nc_runtime_provider_program(&profile.target_spec.target_triple)?;
    let mut provider = ncc_parser::parse(ncc_lexer::lex(PROVIDER_SOURCE)?)?;
    program.modules.append(&mut provider.modules);
    program.packages.append(&mut provider.packages);
    // Runtime provider implementations and facade imports intentionally meet
    // in one module. Keeping facade imports here would make local nC
    // definitions such as jsonParse ambiguous with their public catalog row.
    provider.uses.retain(|use_decl| {
        !use_decl
            .path
            .last()
            .is_some_and(|part| part.eq_ignore_ascii_case("json"))
    });
    program.uses.append(&mut provider.uses);
    program.containers.append(&mut provider.containers);
    ncc_sema::analyze(&program)?;
    let ir = ncc_ir::lower_with_backend_profile_exact(program, profile.clone());
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config(
        &ir,
        ncc_ir::mct_llvm_pass_pipeline(ncc_ir::MctLlvmOptimizationLevelIR::O3),
        ncc_ir::MctLlvmDebugConfigIR {
            enabled: false,
            source_filename: "stage3_provider.n".to_string(),
            source_directory: "runtime/nc".to_string(),
            producer: "ncc runtime provider".to_string(),
            dwarf_version: 5,
            debug_info_version: 3,
            emission_kind: "NoDebug",
        },
    );
    if module.emitted_function_count == 0 || module.rejected_function_count > 0 {
        return Err(format!(
            "runtime provider MCT rejected functions: emitted={} rejected={} diagnostics={:?}",
            module.emitted_function_count, module.rejected_function_count, module.diagnostics
        )
        .into());
    }
    let llvm_target = if module.target_triple.starts_with("riscv32imac-") {
        module.target_triple.replacen("riscv32imac-", "riscv32-", 1)
    } else {
        module.target_triple.clone()
    };
    let mut llvm_ir = provider_export_llvm_ir(&module).replace(
        &format!("target triple = \"{}\"", module.target_triple),
        &format!("target triple = \"{llvm_target}\""),
    );
    if module.target_triple.starts_with("riscv32imac-") {
        llvm_ir = provider_declare_external_runtime_calls(&llvm_ir);
    }
    fs::write(llvm_path, llvm_ir.as_bytes())?;
    let clang = super::mct_support::mct_find_tool("clang")
        .ok_or("runtime provider requires clang for nC MCT object emission")?;
    let mut command = Command::new(&clang);
    command.arg(format!("--target={llvm_target}")).args([
        "-O3",
        "-ffunction-sections",
        "-fdata-sections",
    ]);
    if module.target_triple.starts_with("riscv32imac-") {
        command.args(["-march=rv32imac", "-mabi=ilp32"]);
    }
    if !module.target_triple.contains("windows") {
        command.arg("-fPIC");
    }
    let mut child = command
        .args(["-x", "ir", "-c", "-", "-o"])
        .arg(object)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let write_result = child
        .stdin
        .take()
        .ok_or("clang did not expose stdin for the nC provider")?
        .write_all(llvm_ir.as_bytes());
    let result = child.wait_with_output()?;
    if let Err(error) = write_result {
        return Err(format!(
            "nC provider object input failed for {}: {}; {}",
            module.target_triple,
            error,
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    if !result.status.success() {
        return Err(format!(
            "nC provider object failed for {}: {}",
            module.target_triple,
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    Ok(fs::read(object)?)
}

fn provider_declare_external_runtime_calls(llvm_ir: &str) -> String {
    let mut material = BTreeSet::new();
    for line in llvm_ir.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("define ") || trimmed.starts_with("declare ") {
            if let Some(symbol) = llvm_function_symbol(trimmed) {
                material.insert(symbol.to_string());
            }
        }
    }
    let mut declarations = BTreeMap::<String, String>::new();
    for line in llvm_ir.lines() {
        let Some(call_at) = line.find("call ").map(|at| at + 5) else {
            continue;
        };
        let call = &line[call_at..];
        let Some(symbol_at) = call.find('@') else {
            continue;
        };
        let signature_prefix = call[..symbol_at].trim();
        let return_ty = signature_prefix.split_whitespace().last().unwrap_or("void");
        let symbol_and_args = &call[symbol_at..];
        let Some(symbol) = llvm_function_symbol(symbol_and_args) else {
            continue;
        };
        if material.contains(symbol) || symbol.starts_with("llvm.") {
            continue;
        }
        let open = symbol_and_args
            .find('(')
            .expect("LLVM call has opening parenthesis");
        let Some(close) = symbol_and_args[open + 1..].find(')') else {
            continue;
        };
        let args = &symbol_and_args[open + 1..open + 1 + close];
        let types = args
            .split(',')
            .filter_map(|arg| arg.split_whitespace().next())
            .collect::<Vec<_>>()
            .join(", ");
        declarations
            .entry(symbol.to_string())
            .or_insert_with(|| format!("declare {return_ty} @{symbol}({types})"));
    }
    if declarations.is_empty() {
        return llvm_ir.to_string();
    }
    let block = declarations
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let insert_at = llvm_ir.find("\ndefine ").unwrap_or(llvm_ir.len());
    let mut out = String::with_capacity(llvm_ir.len() + block.len() + 2);
    out.push_str(&llvm_ir[..insert_at]);
    out.push('\n');
    out.push_str(&block);
    out.push_str(&llvm_ir[insert_at..]);
    out
}

fn llvm_function_symbol(line: &str) -> Option<&str> {
    let start = line.find('@')? + 1;
    let end = line[start..].find('(')? + start;
    Some(&line[start..end])
}

fn provider_export_llvm_ir(module: &ncc_ir::MctLlvmModuleIR) -> String {
    let mut out = module.llvm_ir.clone();
    for function in &module.functions {
        let Some(wrapper) = function.source_name.rsplit('.').next() else {
            continue;
        };
        let Some(rest) = wrapper.strip_prefix("provider_") else {
            continue;
        };
        let Some((family, leaf)) = rest.split_once('_') else {
            continue;
        };
        let canonical = format!("nC.runtime.{family}.{leaf}");
        out = out.replace(
            &format!("@{}(", function.symbol_name),
            &format!("@{canonical}("),
        );
    }
    out
}

fn link_pe_provider(
    object: &Path,
    output: &Path,
    required: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let gcc = [
        "C:/msys64/ucrt64/bin/gcc.exe",
        "C:/msys64/mingw64/bin/gcc.exe",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
    .ok_or("runtime provider PE64 requires a MinGW-w64 linker")?;
    let tool_dir = gcc
        .parent()
        .ok_or("MinGW-w64 linker has no parent directory")?;
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![tool_dir.to_path_buf()];
    paths.extend(std::env::split_paths(&inherited));
    let exports = output.with_extension("exports.def");
    fs::write(
        &exports,
        format!(
            "LIBRARY ncruntime\nEXPORTS\n{}\n",
            provider_symbol_names(required)
                .into_iter()
                .map(|symbol| format!("    {symbol}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )?;
    let result = Command::new(&gcc)
        .env("PATH", std::env::join_paths(paths)?)
        .args(["-shared", "-Wl,--defsym,__chkstk=___chkstk_ms"])
        .arg(object)
        .arg(&exports)
        .arg("-o")
        .arg(output)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "runtime provider linker '{}' failed: stdout={} stderr={}",
            gcc.display(),
            String::from_utf8_lossy(&result.stdout).trim(),
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    Ok(())
}

fn link_elf_provider(
    object: &Path,
    output: &Path,
    required: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let linker = super::mct_support::mct_find_tool("ld.lld")
        .ok_or("runtime provider ELF64 requires ld.lld")?;
    let exports = output.with_extension("exports.map");
    fs::write(
        &exports,
        format!(
            "NC_RUNTIME_2 {{\n  global:\n{}\n  local: *;\n}};\n",
            provider_symbol_names(required)
                .into_iter()
                .map(|symbol| format!("    \"{symbol}\";"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )?;
    run_provider_tool(
        Path::new(&linker),
        &[
            "-shared",
            "-soname",
            "libncruntime.so.2",
            "--version-script",
            exports
                .to_str()
                .ok_or("runtime provider ELF export map path is not UTF-8")?,
        ],
        object,
        output,
    )
}

fn link_macho_provider(
    object: &Path,
    output: &Path,
    required: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let linker = super::mct_support::mct_find_tool("ld64.lld")
        .ok_or("runtime provider Mach-O requires ld64.lld")?;
    let exports = output.with_extension("exports.list");
    fs::write(
        &exports,
        provider_symbol_names(required)
            .into_iter()
            .map(|symbol| format!("_{symbol}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )?;
    run_provider_tool(
        Path::new(&linker),
        &[
            "-dylib",
            "-arch",
            "x86_64",
            "-platform_version",
            "macos",
            "11.0",
            "14.0",
            "-install_name",
            "@loader_path/libncruntime.2.dylib",
            "-exported_symbols_list",
            exports
                .to_str()
                .ok_or("runtime provider Mach-O export list path is not UTF-8")?,
            "-undefined",
            "dynamic_lookup",
        ],
        object,
        output,
    )
}

fn archive_rv32_provider(
    object: &Path,
    output: &Path,
    required: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let ar = super::mct_support::mct_find_tool("llvm-ar")
        .ok_or("runtime provider RV32 requires llvm-ar")?;
    let objcopy = super::mct_support::mct_find_tool("llvm-objcopy")
        .ok_or("runtime provider RV32 requires llvm-objcopy")?;
    let exports = output.with_extension("exports.txt");
    let localized = output.with_extension("provider.o");
    fs::write(&exports, provider_symbol_names(required).join("\n"))?;
    let localize = Command::new(&objcopy)
        .arg(format!("--keep-global-symbols={}", exports.display()))
        .arg(object)
        .arg(&localized)
        .output()?;
    if !localize.status.success() {
        return Err(format!(
            "llvm-objcopy failed: {}",
            String::from_utf8_lossy(&localize.stderr).trim()
        )
        .into());
    }
    if output.is_file() {
        fs::remove_file(output)?;
    }
    let result = Command::new(&ar)
        .arg("rcs")
        .arg(output)
        .arg(&localized)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "llvm-ar failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    Ok(())
}

fn run_provider_tool(
    tool: &Path,
    prefix: &[&str],
    object: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = Command::new(tool)
        .args(prefix)
        .arg(object)
        .arg("-o")
        .arg(output)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "runtime provider linker '{}' failed: {}",
            tool.display(),
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    Ok(())
}

fn provider_required_symbols() -> Vec<serde_json::Value> {
    let manifest = super::runtime::runtime_abi_manifest();
    let by_name = manifest
        .get("symbols")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("symbol")
                .and_then(serde_json::Value::as_str)
                .map(|name| (name.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let names = PROVIDER_SOURCE
        .lines()
        .filter_map(|line| {
            let start = line.find("provider_")? + "provider_".len();
            let end = line[start..].find('(')? + start;
            let (family, leaf) = line[start..end].split_once('_')?;
            Some(format!("nC.runtime.{family}.{leaf}"))
        })
        .collect::<BTreeSet<_>>();
    names
        .into_iter()
        .filter_map(|name| by_name.get(&name))
        .map(|row| {
            serde_json::json!({
                "symbol": row.get("symbol").cloned().unwrap_or(serde_json::Value::Null),
                "family": row.get("family").cloned().unwrap_or(serde_json::Value::Null),
                "inputs": row.get("inputs")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|input| input.get("type").and_then(serde_json::Value::as_str))
                    .collect::<Vec<_>>(),
                "return": row.get("return")
                    .and_then(|value| value.get("type"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("void"),
                "calling_convention": "nC.runtime-value-v1",
            })
        })
        .collect()
}

fn provider_symbol_names(required: &[serde_json::Value]) -> Vec<String> {
    required
        .iter()
        .filter_map(|row| row.get("symbol").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

fn write_artifact_manifest(
    target: ProviderTarget,
    output: &Path,
    bytes: &[u8],
    required: &[serde_json::Value],
    mode: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let payload = serde_json::json!({
        "schema": "nc.runtime.provider.artifact.v1",
        "status": "materialized",
        "target": target.triple,
        "target_id": target.id,
        "kind": provider_kind_label(target.kind),
        "mode": mode,
        "path": output.display().to_string(),
        "bytes": bytes.len(),
        "sha256": crate::content_cache::sha256_bytes(bytes),
        "source": PROVIDER_SOURCE_PATH,
        "source_sha256": crate::content_cache::sha256_bytes(PROVIDER_SOURCE.as_bytes()),
        "source_language": "nC",
        "c_translation_units": 0,
        "required_symbol_count": required.len(),
        "required_symbols": required,
    });
    fs::write(
        provider_manifest_path(output),
        serde_json::to_vec_pretty(&payload)?,
    )?;
    Ok(payload)
}

fn provider_manifest_path(output: &Path) -> PathBuf {
    output.with_extension(format!(
        "{}manifest.json",
        output
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!("{value}."))
            .unwrap_or_default()
    ))
}

fn verify_providers(
    targets: &[ProviderTarget],
    out_dir: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let expected = provider_symbol_names(&provider_required_symbols())
        .into_iter()
        .collect::<BTreeSet<_>>();
    let rows = targets
        .iter()
        .map(|target| {
            let path = out_dir.join(target.file_name);
            let bytes = fs::read(&path).unwrap_or_default();
            let sha256 = crate::content_cache::sha256_bytes(&bytes);
            let magic = provider_magic_valid(target.kind, &bytes);
            let manifest_path = provider_manifest_path(&path);
            let manifest = fs::read(&manifest_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            let manifest_valid = manifest.as_ref().is_some_and(|manifest| {
                manifest.get("schema").and_then(serde_json::Value::as_str)
                    == Some("nc.runtime.provider.artifact.v1")
                    && manifest.get("target").and_then(serde_json::Value::as_str)
                        == Some(target.triple)
                    && manifest
                        .get("required_symbol_count")
                        .and_then(serde_json::Value::as_u64)
                        == Some(expected.len() as u64)
                    && manifest.get("bytes").and_then(serde_json::Value::as_u64)
                        == Some(bytes.len() as u64)
                    && manifest.get("sha256").and_then(serde_json::Value::as_str)
                        == Some(sha256.as_str())
            });
            let inspected = inspect_provider_contract(*target, &path, &bytes);
            let (symbols, metadata_valid, detail) = match inspected {
                Ok(value) => value,
                Err(error) => (BTreeSet::new(), false, error),
            };
            let missing = expected.difference(&symbols).cloned().collect::<Vec<_>>();
            let unexpected = symbols.difference(&expected).cloned().collect::<Vec<_>>();
            let symbols_exact = missing.is_empty() && unexpected.is_empty();
            let (loader_required, loader_passed, loader_detail) =
                provider_host_loader_smoke(*target, &path, &expected);
            let (link_smoke_required, link_smoke_passed, link_smoke_detail) =
                provider_static_link_smoke(*target, &path);
            let passed = path.is_file()
                && magic
                && manifest_valid
                && metadata_valid
                && symbols_exact
                && (!loader_required || loader_passed)
                && (!link_smoke_required || link_smoke_passed);
            serde_json::json!({
                "target": target.triple,
                "path": path.display().to_string(),
                "present": path.is_file(),
                "bytes": bytes.len(),
                "sha256": sha256,
                "magic_valid": magic,
                "manifest": manifest_path.display().to_string(),
                "manifest_valid": manifest_valid,
                "required_symbol_count": expected.len(),
                "material_symbol_count": symbols.len(),
                "symbols_exact": symbols_exact,
                "missing_symbols": missing,
                "unexpected_symbols": unexpected,
                "format_metadata_valid": metadata_valid,
                "format_detail": detail,
                "host_loader_required": loader_required,
                "host_loader_passed": loader_passed,
                "host_loader_detail": loader_detail,
                "static_link_required": link_smoke_required,
                "static_link_passed": link_smoke_passed,
                "static_link_detail": link_smoke_detail,
                "status": if passed { "passed" } else { "failed" },
            })
        })
        .collect::<Vec<_>>();
    let passed = rows
        .iter()
        .all(|row| row.get("status").and_then(serde_json::Value::as_str) == Some("passed"));
    Ok(serde_json::json!({
        "schema": "nc.runtime.provider.verification.v1",
        "status": if passed { "passed" } else { "failed" },
        "artifacts": rows,
    }))
}

fn provider_static_link_smoke(target: ProviderTarget, archive: &Path) -> (bool, bool, String) {
    if target.kind != ProviderKind::Rv32Archive {
        return (
            false,
            false,
            "dynamic provider or typed import module; static link smoke is not required"
                .to_string(),
        );
    }
    let Some(clang) = super::mct_support::mct_find_tool("clang") else {
        return (
            true,
            false,
            "RV32 static link smoke requires clang".to_string(),
        );
    };
    let Some(linker) = super::mct_support::mct_find_tool("ld.lld") else {
        return (
            true,
            false,
            "RV32 static link smoke requires ld.lld".to_string(),
        );
    };
    let Some(nm) = super::mct_support::mct_find_tool("llvm-nm") else {
        return (
            true,
            false,
            "RV32 static link smoke requires llvm-nm".to_string(),
        );
    };
    let parent = archive.parent().unwrap_or_else(|| Path::new("."));
    let source = parent.join(".ncruntime-rv32-link-smoke.s");
    let object = parent.join(".ncruntime-rv32-link-smoke.o");
    let executable = parent.join(".ncruntime-rv32-link-smoke.elf");
    let result = (|| -> Result<String, String> {
        fs::write(
            &source,
            ".section .text._start,\"ax\",@progbits\n\
             .globl _start\n\
             .type _start,@function\n\
             _start:\n\
               li a0, 1\n\
               call nC.runtime.result.isOk\n\
               ret\n\
             .size _start, .-_start\n",
        )
        .map_err(|error| format!("could not write RV32 link smoke: {error}"))?;
        let compile = Command::new(clang)
            .args([
                "--target=riscv32-unknown-elf",
                "-march=rv32imac",
                "-mabi=ilp32",
                "-c",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&object)
            .output()
            .map_err(|error| format!("RV32 smoke compiler could not start: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "RV32 smoke compile failed: {}",
                String::from_utf8_lossy(&compile.stderr).trim()
            ));
        }
        let link = Command::new(linker)
            .args(["-m", "elf32lriscv", "--gc-sections", "-e", "_start", "-o"])
            .arg(&executable)
            .arg(&object)
            .arg(archive)
            .output()
            .map_err(|error| format!("RV32 smoke linker could not start: {error}"))?;
        if !link.status.success() {
            return Err(format!(
                "RV32 static consumer link failed: {}",
                String::from_utf8_lossy(&link.stderr).trim()
            ));
        }
        let undefined = Command::new(nm)
            .arg("--undefined-only")
            .arg(&executable)
            .output()
            .map_err(|error| format!("RV32 smoke symbol inspector could not start: {error}"))?;
        if !undefined.status.success() || !undefined.stdout.iter().all(u8::is_ascii_whitespace) {
            return Err(format!(
                "RV32 static consumer retains undefined symbols: {}",
                String::from_utf8_lossy(&undefined.stdout).trim()
            ));
        }
        Ok("RV32 ELF32 consumer linked with --gc-sections and zero undefined symbols".to_string())
    })();
    for temporary in [&source, &object, &executable] {
        let _ = fs::remove_file(temporary);
    }
    match result {
        Ok(detail) => (true, true, detail),
        Err(detail) => (true, false, detail),
    }
}

fn provider_host_loader_smoke(
    target: ProviderTarget,
    path: &Path,
    expected: &BTreeSet<String>,
) -> (bool, bool, String) {
    match target.kind {
        ProviderKind::Pe64 if cfg!(windows) => provider_windows_loader_smoke(path, expected),
        ProviderKind::Elf64 if cfg!(all(unix, not(target_os = "macos"))) => {
            provider_unix_loader_smoke(path, expected)
        }
        ProviderKind::MachO64 if cfg!(target_os = "macos") => {
            provider_unix_loader_smoke(path, expected)
        }
        _ => (
            false,
            false,
            "cross-target artifact; loader smoke belongs to its native host".to_string(),
        ),
    }
}

#[cfg(windows)]
fn provider_windows_loader_smoke(path: &Path, expected: &BTreeSet<String>) -> (bool, bool, String) {
    use std::ffi::CString;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut std::ffi::c_void;
        fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
        fn FreeLibrary(module: *mut std::ffi::c_void) -> i32;
    }

    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut wide = absolute.as_os_str().encode_wide().collect::<Vec<_>>();
    wide.push(0);
    let module = unsafe { LoadLibraryW(wide.as_ptr()) };
    if module.is_null() {
        return (
            true,
            false,
            format!("LoadLibraryW rejected '{}'", absolute.display()),
        );
    }
    let mut missing = Vec::new();
    for symbol in expected {
        let name = CString::new(symbol.as_str()).expect("runtime ABI symbol contains no NUL");
        if unsafe { GetProcAddress(module, name.as_ptr().cast()) }.is_null() {
            missing.push(symbol.clone());
        }
    }
    let probe_name = CString::new("nC.runtime.string.strByteLen").expect("static symbol");
    let probe = unsafe { GetProcAddress(module, probe_name.as_ptr().cast()) };
    let probe_result = if probe.is_null() {
        None
    } else {
        let str_byte_len: unsafe extern "system" fn(*const u8) -> i32 =
            unsafe { std::mem::transmute(probe) };
        Some(unsafe { str_byte_len(b"provider\0".as_ptr()) })
    };
    unsafe {
        FreeLibrary(module);
    }
    let passed = missing.is_empty() && probe_result == Some(8);
    (
        true,
        passed,
        if passed {
            format!(
                "LoadLibraryW/GetProcAddress resolved {} symbols; strByteLen execution returned 8",
                expected.len()
            )
        } else if !missing.is_empty() {
            format!("GetProcAddress missed: {}", missing.join(", "))
        } else {
            format!(
                "runtime ABI execution returned {:?}; expected strByteLen('provider') == 8",
                probe_result
            )
        },
    )
}

#[cfg(not(windows))]
fn provider_windows_loader_smoke(
    _path: &Path,
    _expected: &BTreeSet<String>,
) -> (bool, bool, String) {
    (
        false,
        false,
        "PE loader is not available on this host".to_string(),
    )
}

#[cfg(unix)]
fn provider_unix_loader_smoke(path: &Path, expected: &BTreeSet<String>) -> (bool, bool, String) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    #[cfg_attr(not(target_os = "macos"), link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(path: *const std::ffi::c_char, mode: i32) -> *mut std::ffi::c_void;
        fn dlsym(
            handle: *mut std::ffi::c_void,
            symbol: *const std::ffi::c_char,
        ) -> *mut std::ffi::c_void;
        fn dlclose(handle: *mut std::ffi::c_void) -> i32;
    }

    const RTLD_NOW: i32 = 2;
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let encoded = CString::new(absolute.as_os_str().as_bytes()).expect("provider path has no NUL");
    let module = unsafe { dlopen(encoded.as_ptr(), RTLD_NOW) };
    if module.is_null() {
        return (
            true,
            false,
            format!("dlopen rejected '{}'", absolute.display()),
        );
    }
    let mut missing = Vec::new();
    for symbol in expected {
        let name = CString::new(symbol.as_str()).expect("runtime ABI symbol contains no NUL");
        if unsafe { dlsym(module, name.as_ptr()) }.is_null() {
            missing.push(symbol.clone());
        }
    }
    let probe_name = CString::new("nC.runtime.string.strByteLen").expect("static symbol");
    let probe = unsafe { dlsym(module, probe_name.as_ptr()) };
    let probe_result = if probe.is_null() {
        None
    } else {
        let str_byte_len: unsafe extern "C" fn(*const u8) -> i32 =
            unsafe { std::mem::transmute(probe) };
        Some(unsafe { str_byte_len(b"provider\0".as_ptr()) })
    };
    unsafe {
        dlclose(module);
    }
    let passed = missing.is_empty() && probe_result == Some(8);
    (
        true,
        passed,
        if passed {
            format!(
                "dlopen/dlsym resolved {} symbols; strByteLen execution returned 8",
                expected.len()
            )
        } else if !missing.is_empty() {
            format!("dlsym missed: {}", missing.join(", "))
        } else {
            format!(
                "runtime ABI execution returned {:?}; expected strByteLen('provider') == 8",
                probe_result
            )
        },
    )
}

#[cfg(not(unix))]
fn provider_unix_loader_smoke(_path: &Path, _expected: &BTreeSet<String>) -> (bool, bool, String) {
    (
        false,
        false,
        "Unix loader is not available on this host".to_string(),
    )
}

fn inspect_provider_contract(
    target: ProviderTarget,
    path: &Path,
    bytes: &[u8],
) -> Result<(BTreeSet<String>, bool, String), String> {
    if target.kind == ProviderKind::WasmImports {
        let imports = wasm_import_names(bytes)?;
        return Ok((
            imports.into_iter().collect(),
            true,
            "wasm32 typed function import section".to_string(),
        ));
    }
    let (symbols, metadata_valid, detail) = match target.kind {
        ProviderKind::Pe64 => {
            let output = run_provider_inspector("llvm-readobj", &["--coff-exports"], path)?;
            let symbols = output
                .lines()
                .filter_map(|line| line.trim().strip_prefix("Name: "))
                .map(str::to_string)
                .collect::<BTreeSet<_>>();
            (
                symbols,
                output.contains("Export {") && output.contains("nC.runtime.args.argsOpen"),
                "PE64 export directory".to_string(),
            )
        }
        ProviderKind::Elf64 => {
            let symbols = provider_nm_symbols(path, true, false)?;
            let dynamic = run_provider_inspector("llvm-readobj", &["--dynamic-table"], path)?;
            (
                symbols,
                dynamic.contains("SONAME") && dynamic.contains("libncruntime.so.2"),
                "ELF64 SONAME libncruntime.so.2".to_string(),
            )
        }
        ProviderKind::MachO64 => {
            let symbols = provider_nm_symbols(path, false, true)?;
            let id = run_provider_inspector("llvm-objdump", &["--macho", "--dylib-id"], path)?;
            (
                symbols,
                id.contains("@loader_path/libncruntime.2.dylib"),
                "Mach-O install-name @loader_path/libncruntime.2.dylib".to_string(),
            )
        }
        ProviderKind::Rv32Archive => {
            let symbols = provider_nm_symbols(path, false, false)?;
            let header = run_provider_inspector("llvm-readobj", &["--file-headers"], path)?;
            (
                symbols,
                header.contains("ELF32-riscv")
                    || (header.contains("Class: 32-bit") && header.contains("EM_RISCV")),
                "RV32 ELF relocatable archive".to_string(),
            )
        }
        ProviderKind::WasmImports => unreachable!(),
    };
    Ok((symbols, metadata_valid, detail))
}

fn run_provider_inspector(tool: &str, args: &[&str], path: &Path) -> Result<String, String> {
    let executable = super::mct_support::mct_find_tool(tool)
        .ok_or_else(|| format!("runtime provider verification requires {tool}"))?;
    let result = Command::new(executable)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| format!("{tool} could not inspect '{}': {error}", path.display()))?;
    if !result.status.success() {
        return Err(format!(
            "{tool} rejected '{}': {}",
            path.display(),
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&result.stdout).into_owned())
}

fn provider_nm_symbols(
    path: &Path,
    dynamic: bool,
    strip_macho_prefix: bool,
) -> Result<BTreeSet<String>, String> {
    let mut args = vec!["--defined-only", "--extern-only"];
    if dynamic {
        args.push("--dynamic");
    }
    let output = run_provider_inspector("llvm-nm", &args, path)?;
    Ok(output
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(|symbol| symbol.split('@').next().unwrap_or(symbol))
        .map(|symbol| {
            if strip_macho_prefix {
                symbol.strip_prefix('_').unwrap_or(symbol)
            } else {
                symbol
            }
        })
        .filter(|symbol| !symbol.ends_with(':') && *symbol != "NC_RUNTIME_2")
        .map(str::to_string)
        .collect())
}

fn provider_magic_valid(kind: ProviderKind, bytes: &[u8]) -> bool {
    match kind {
        ProviderKind::Pe64 => bytes.starts_with(b"MZ"),
        ProviderKind::Elf64 => bytes.starts_with(b"\x7fELF") && bytes.get(4) == Some(&2),
        ProviderKind::MachO64 => bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]),
        ProviderKind::Rv32Archive => bytes.starts_with(b"!<arch>\n"),
        ProviderKind::WasmImports => bytes.starts_with(b"\0asm\x01\0\0\0"),
    }
}

fn write_wasm_import_module(
    symbols: &[serde_json::Value],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut types = Vec::<(Vec<u8>, Option<u8>)>::new();
    let mut imports = Vec::<(String, String, u32)>::new();
    for row in symbols {
        let symbol = row
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .ok_or("runtime provider ABI row is missing symbol")?;
        let mut parts = symbol.split('.');
        let _nc = parts.next();
        let _runtime = parts.next();
        let family = parts
            .next()
            .ok_or("runtime provider symbol is missing family")?;
        let leaf = parts.collect::<Vec<_>>().join(".");
        let parameters = row
            .get("inputs")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .map(|input| wasm_value_type(input.as_str().unwrap_or("addr")))
            .collect::<Vec<_>>();
        let result = row
            .get("return")
            .and_then(serde_json::Value::as_str)
            .and_then(wasm_result_type);
        let key = (parameters, result);
        let index = types
            .iter()
            .position(|existing| existing == &key)
            .unwrap_or_else(|| {
                types.push(key);
                types.len() - 1
            });
        imports.push((format!("nC.runtime.{family}"), leaf, index as u32));
    }
    let mut module = b"\0asm\x01\0\0\0".to_vec();
    let mut type_body = Vec::new();
    wasm_u32(&mut type_body, types.len() as u32);
    for (params, result) in &types {
        type_body.push(0x60);
        wasm_u32(&mut type_body, params.len() as u32);
        type_body.extend(params);
        wasm_u32(&mut type_body, u32::from(result.is_some()));
        if let Some(result) = result {
            type_body.push(*result);
        }
    }
    wasm_section(&mut module, 1, &type_body);
    let mut import_body = Vec::new();
    wasm_u32(&mut import_body, imports.len() as u32);
    for (namespace, leaf, ty) in imports {
        wasm_name(&mut import_body, &namespace);
        wasm_name(&mut import_body, &leaf);
        import_body.push(0x00);
        wasm_u32(&mut import_body, ty);
    }
    wasm_section(&mut module, 2, &import_body);
    Ok(module)
}

fn wasm_import_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    if !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err("WASM provider has an invalid module header".to_string());
    }
    let mut cursor = 8usize;
    while cursor < bytes.len() {
        let section = bytes[cursor];
        cursor += 1;
        let size = wasm_read_u32(bytes, &mut cursor)? as usize;
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| "WASM provider section exceeds module bytes".to_string())?;
        if section != 2 {
            cursor = end;
            continue;
        }
        let count = wasm_read_u32(bytes, &mut cursor)? as usize;
        let mut names = Vec::with_capacity(count);
        for _ in 0..count {
            let namespace = wasm_read_name(bytes, &mut cursor)?;
            let field = wasm_read_name(bytes, &mut cursor)?;
            let kind = *bytes
                .get(cursor)
                .ok_or_else(|| "WASM provider import is missing its kind".to_string())?;
            cursor += 1;
            if kind != 0 {
                return Err(format!(
                    "WASM provider import '{namespace}.{field}' is not a function"
                ));
            }
            let _type_index = wasm_read_u32(bytes, &mut cursor)?;
            names.push(format!("{namespace}.{field}"));
        }
        if cursor != end {
            return Err("WASM provider import section has trailing bytes".to_string());
        }
        return Ok(names);
    }
    Err("WASM provider has no import section".to_string())
}

fn wasm_read_name(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let len = wasm_read_u32(bytes, cursor)? as usize;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| "WASM provider name exceeds module bytes".to_string())?;
    let value = std::str::from_utf8(&bytes[*cursor..end])
        .map_err(|_| "WASM provider import name is not UTF-8".to_string())?
        .to_string();
    *cursor = end;
    Ok(value)
}

fn wasm_read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let mut value = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *bytes
            .get(*cursor)
            .ok_or_else(|| "WASM provider contains a truncated LEB128 value".to_string())?;
        *cursor += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift >= 35 {
            return Err("WASM provider contains an invalid u32 LEB128 value".to_string());
        }
    }
}

fn wasm_value_type(ty: &str) -> u8 {
    match ty.trim() {
        "ft32" | "f32" => 0x7d,
        "ft64" | "f64" => 0x7c,
        "i64" | "u64" => 0x7e,
        _ => 0x7f,
    }
}

fn wasm_result_type(ty: &str) -> Option<u8> {
    if ty.trim().eq_ignore_ascii_case("void") {
        None
    } else {
        Some(wasm_value_type(ty))
    }
}

fn wasm_section(out: &mut Vec<u8>, id: u8, body: &[u8]) {
    out.push(id);
    wasm_u32(out, body.len() as u32);
    out.extend_from_slice(body);
}

fn wasm_name(out: &mut Vec<u8>, value: &str) {
    wasm_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn wasm_u32(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_source_is_nc_and_covers_exact_stage3_surface() {
        assert!(PROVIDER_SOURCE.contains("stage3Provider :: runtime"));
        assert!(PROVIDER_SOURCE.contains("provider_args_argsOpen"));
        assert!(PROVIDER_SOURCE.contains("provider_time_monotonicNs"));
        assert!(!PROVIDER_SOURCE.contains("#include"));
        assert!(!PROVIDER_SOURCE.contains("extern \"C\""));
        let required = provider_required_symbols();
        assert_eq!(required.len(), 85);
    }

    #[test]
    fn wasm_provider_is_a_real_typed_import_module() {
        let required = provider_required_symbols();
        let bytes = write_wasm_import_module(&required).expect("wasm imports");
        assert!(provider_magic_valid(ProviderKind::WasmImports, &bytes));
        assert!(bytes
            .windows("nC.runtime.args".len())
            .any(|row| row == b"nC.runtime.args"));
        assert!(bytes
            .windows("argsOpen".len())
            .any(|row| row == b"argsOpen"));
        let imports = wasm_import_names(&bytes).expect("decode wasm imports");
        assert_eq!(imports.len(), 85);
        assert_eq!(imports.into_iter().collect::<BTreeSet<_>>().len(), 85);
    }

    #[test]
    fn provider_target_names_are_distribution_contracts() {
        assert_eq!(PROVIDER_TARGETS.len(), 5);
        assert_eq!(PROVIDER_TARGETS[0].file_name, "ncruntime.dll");
        assert_eq!(PROVIDER_TARGETS[1].file_name, "libncruntime.so.2");
        assert_eq!(PROVIDER_TARGETS[2].file_name, "libncruntime.2.dylib");
        assert_eq!(PROVIDER_TARGETS[3].file_name, "libncruntime-rv32.a");
        assert_eq!(PROVIDER_TARGETS[4].file_name, "ncruntime.imports.wasm");
    }

    #[test]
    fn rv32_static_provider_declares_only_unmaterialized_runtime_calls() {
        let llvm = "target triple = \"riscv32-unknown-elf\"\n\
                    define i64 @material(i64 %value) { ret i64 %value }\n\
                    define i64 @entry(ptr %value) {\n\
                      %a = call i64 @material(i64 1)\n\
                      %b = call i64 @platform_hook(ptr %value, i64 %a)\n\
                      ret i64 %b\n\
                    }\n";
        let completed = provider_declare_external_runtime_calls(llvm);
        assert!(completed.contains("declare i64 @platform_hook(ptr, i64)"));
        assert!(!completed.contains("declare i64 @material(i64)"));
    }

    #[test]
    fn stage3_gate_requires_all_distributable_runtime_providers() {
        let plan: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../bootstrap/stage3-generation-plan.json"
        ))
        .expect("stage3 plan JSON");
        let providers = &plan["runtime_providers"];
        assert_eq!(providers["source_language"], "nC");
        assert_eq!(providers["artifact_count"], 5);
        assert_eq!(
            providers["bundle_manifest"],
            "target/bootstrap/stage2/ncruntime.providers.json"
        );

        for component in plan["components"].as_array().expect("components") {
            let dependencies = component["smoke_dependencies"]
                .as_array()
                .expect("smoke dependencies");
            assert!(dependencies.iter().any(|dependency| {
                dependency["artifact_schema"] == "nc.runtime.provider.artifact.v1"
                    && dependency["required_symbol_count"] == 85
            }));
        }

        let ghor = include_str!("../../../../tools/ghor/nc/src/bootstrap_generation.n");
        assert!(ghor.contains("listAdd(arguments, \"runtime\")"));
        assert!(ghor.contains("listAdd(arguments, \"provider\")"));
        assert!(ghor.contains("listAdd(arguments, \"all\")"));
        assert!(ghor.contains("runtime_providers_complete"));
        assert!(ghor.contains("jsonArrayLen(jsonObjectGet(bundle, \"artifacts\")) == 5"));
    }
}
