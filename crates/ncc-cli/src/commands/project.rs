#![allow(dead_code)]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime},
};

static PROJECT_ATOMIC_WRITE_SEQ: AtomicU64 = AtomicU64::new(0);
const PROJECT_CACHE_LOCK_TIMEOUT_MS: u64 = 30_000;
const PROJECT_CACHE_LOCK_STALE_SECS: u64 = 600;

thread_local! {
    static PROJECT_CACHE_LOCKS_HELD: RefCell<BTreeSet<String>> = const {
        RefCell::new(BTreeSet::new())
    };
}

fn project_runtime_error(code: &'static str, found: String) -> Box<dyn Error> {
    ncc_diagnostics::registered_error_text(code, found).into()
}

#[derive(Debug, Clone)]
struct ProjectCommandOptions {
    root: PathBuf,
    main_name: String,
    json: bool,
    show_return: bool,
    out: Option<PathBuf>,
    linker: Option<String>,
    profile: ncc_ir::BackendLoweringProfileIR,
}

#[derive(Debug, Clone)]
struct ProjectSourceUnit {
    name: String,
    source: PathBuf,
    relative_source: String,
    hash: String,
    code_relative: String,
}

#[derive(Debug, Clone)]
struct ProjectBridgeUnit {
    name: String,
    source: PathBuf,
    relative_source: String,
    code_relative: Option<String>,
    source_hash: String,
    object_relative: String,
    object_hash: String,
    symbols: Vec<serde_json::Value>,
    imports: Vec<String>,
    deps: Vec<String>,
    is_main: bool,
}

#[derive(Debug, Clone)]
struct ProjectBridgeManifest {
    units: Vec<ProjectBridgeUnit>,
    edges: Vec<(String, String)>,
    link_order: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProjectPhysicalUnit {
    name: String,
    relative_source: String,
    code_relative: Option<String>,
    source_hash: String,
    object_relative: String,
    object_hash: String,
    symbols: Vec<serde_json::Value>,
    imports: Vec<String>,
    deps: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProjectPhysicalSourceUnit {
    unit: ProjectSourceUnit,
    ast: Option<ncc_ast::Program>,
    imports: Vec<String>,
    symbols: Vec<serde_json::Value>,
    deps: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProjectCompiledUnit {
    unit: ProjectBridgeUnit,
    source_text: String,
    source_bytes: Vec<u8>,
    hash: String,
    hash_bytes: Vec<u8>,
    ast: ncc_ast::Program,
    ir_debug: String,
    backend_debug: String,
    object_artifact: Option<ProjectObjectArtifact>,
    token_count: usize,
    container_count: usize,
    import_count: usize,
    ast_cache_status: String,
}

#[derive(Debug, Clone)]
struct ProjectObjectArtifact {
    object_relative: String,
    metadata_relative: String,
    extension: String,
    bytes: Vec<u8>,
    hash: String,
    hash_bytes: Vec<u8>,
    text: String,
    object: ncc_ir::BackendObjectIR,
    cache_status: String,
    cache_key: String,
}

#[derive(Debug, Clone, Copy)]
enum ProjectEntryExitMode {
    Bool,
    Int,
    Void,
}

#[derive(Debug, Clone)]
struct ProjectEntryAbi {
    symbol: String,
    exit_mode: ProjectEntryExitMode,
}

#[derive(Debug, Clone)]
struct ProjectCompileResult {
    units: Vec<ProjectCompiledUnit>,
    edges: Vec<(String, String)>,
    linked_program: ncc_ast::Program,
    linked_ir_debug: String,
    linked_backend_debug: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectNativeBuildInput {
    pub(crate) program: ncc_ast::Program,
    root: PathBuf,
    bridge: Option<PathBuf>,
    main_source: PathBuf,
    source_mode: &'static str,
    unit_names: Vec<String>,
    edges: Vec<(String, String)>,
}

impl ProjectNativeBuildInput {
    pub(crate) fn identity_payload(&self) -> serde_json::Value {
        json!({
            "schema": "ncc.project.native_build_input.v1",
            "source_mode": self.source_mode,
            "root": slash_path(&self.root),
            "bridge": self.bridge.as_ref().map(|path| slash_path(path)),
            "main_source": slash_path(&self.main_source),
            "unit_count": self.unit_names.len(),
            "link_order": self.unit_names,
            "edges": self.edges,
            "linked_ast": true,
            "runtime_c_shim_used": false,
        })
    }
}

pub(crate) fn run_project_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(action) = args.first().map(String::as_str) else {
        return Err(project_usage().into());
    };

    match action {
        "init" => {
            let opts = parse_project_options(&args[1..])?;
            let (units, edges) = init_project(&opts)?;
            print_project_report("init", &opts, units.len(), &edges, opts.json)?;
        }
        "sync" => {
            let opts = parse_project_options(&args[1..])?;
            let (units, edges) = sync_project(&opts)?;
            print_project_report("sync", &opts, units.len(), &edges, opts.json)?;
        }
        "doctor" => {
            let opts = parse_project_options(&args[1..])?;
            let report = project_doctor_report(&opts)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "nC project doctor: {}",
                    report["status"].as_str().unwrap_or("unknown")
                );
                println!("root={}", report["root"].as_str().unwrap_or(""));
                println!("bridge={}", report["bridge"].as_str().unwrap_or(""));
                println!("units={}", report["units"].as_u64().unwrap_or(0));
            }
        }
        "check" => {
            let opts = parse_project_options(&args[1..])?;
            if let Some(payload) = project_cached_check_report(&opts)? {
                print_cached_check_report(&opts, &payload)?;
                return Ok(());
            }
            let compiled = compile_project(&opts)?;
            write_project_artifact_files(&opts.root, &compiled)?;
            print_compile_report("check", &opts, &compiled)?;
        }
        "emit-ir" => {
            let opts = parse_project_options(&args[1..])?;
            let compiled = compile_project(&opts)?;
            write_project_artifact_files(&opts.root, &compiled)?;
            let payload = project_ir_payload(&compiled);
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "model" => {
            let opts = parse_project_options(&args[1..])?;
            let payload = project_model_report(&opts)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                print_project_model_report(&payload);
            }
        }
        "incremental" => {
            let opts = parse_project_options(&args[1..])?;
            let payload = project_incremental_report(&opts)?;
            write_project_incremental_status(&opts.root, &payload)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                print_project_incremental_report(&payload);
            }
        }
        "graph" => {
            let opts = parse_project_options(&args[1..])?;
            let mut compiled =
                compile_project_with_object_materialization(&opts, false, false, false)?;
            materialize_project_objects(&opts.root, &mut compiled.units, &opts.profile, true)?;
            write_project_graph_artifact_files(&opts.root, &compiled)?;
            print_project_graph_report(&opts, &compiled)?;
        }
        "link" => {
            let opts = parse_project_options(&args[1..])?;
            let compiled = compile_project_with_object_materialization(&opts, true, true, false)?;
            write_project_artifact_files(&opts.root, &compiled)?;
            let payload = link_project_executable(&opts, &compiled)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "OK: project link output={} objects={}",
                    payload.get("output").and_then(|v| v.as_str()).unwrap_or(""),
                    payload
                        .get("object_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
            }
        }
        "run" => {
            let opts = parse_project_options(&args[1..])?;
            let compiled = compile_project_runtime_fast(&opts)?;
            write_project_runtime_lowering_files(&opts.root, &compiled)?;
            ncc_sema::validate_main_entries(
                &compiled.linked_program,
                ncc_sema::MainEntryRequirement::Required,
            )
            .map_err(|e| -> Box<dyn Error> { e.into() })?;
            let ret = crate::run_main_with_profile(&compiled.linked_program, None)?;
            write_project_run_route_report(&opts, "project-bridge-interpreter", "debug")?;
            if opts.show_return {
                println!(
                    "[return: {} profile=debug route=project-bridge-interpreter]",
                    ncc_runtime::render_value(&ret)
                );
            }
            print_compile_report("run", &opts, &compiled)?;
        }
        "targets" => {
            let json = args.iter().skip(1).any(|arg| arg == "--json");
            let payload = project_target_matrix_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("nC project targets:");
                if let Some(rows) = payload.get("targets").and_then(|v| v.as_array()) {
                    for row in rows {
                        println!(
                            "  {id:<18} {triple:<32} format={format:<8} tier={tier}",
                            id = row.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                            triple = row
                                .get("target_triple")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?"),
                            format = row
                                .get("object_format")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?"),
                            tier = row
                                .get("support_tier")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?")
                        );
                    }
                }
            }
        }
        "--help" | "-h" | "help" => {
            println!("{}", project_usage());
        }
        other => {
            return Err(format!(
                "unknown project action '{}'; expected init, sync, doctor, model, incremental, check, emit-ir, graph, link, run or targets",
                other
            )
            .into());
        }
    }

    Ok(())
}

pub(crate) fn try_run_project_main_file(
    source_path: &Path,
    show_return: bool,
) -> Result<bool, Box<dyn Error>> {
    let absolute_source = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(source_path)
    };
    let absolute_source = fs::canonicalize(&absolute_source).unwrap_or(absolute_source);
    let Some(mut root) = absolute_source.parent().map(Path::to_path_buf) else {
        return Ok(false);
    };

    loop {
        let bridge_path = root.join("bridge.json");
        if bridge_path.is_file() {
            if project_bridge_main_matches_source(&root, &bridge_path, &absolute_source)? {
                let opts = ProjectCommandOptions {
                    root: root.clone(),
                    main_name: "main.n".to_string(),
                    json: false,
                    show_return,
                    out: None,
                    linker: None,
                    profile: default_project_backend_profile(),
                };
                let compiled = compile_project_runtime_fast(&opts)?;
                write_project_runtime_lowering_files(&opts.root, &compiled)?;
                ncc_sema::validate_main_entries(
                    &compiled.linked_program,
                    ncc_sema::MainEntryRequirement::Required,
                )
                .map_err(|e| -> Box<dyn Error> { e.into() })?;
                let ret = crate::run_main_with_profile(&compiled.linked_program, None)?;
                write_project_run_route_report(&opts, "project-bridge-interpreter", "debug")?;
                if opts.show_return {
                    println!(
                        "[return: {} profile=debug route=project-bridge-interpreter]",
                        ncc_runtime::render_value(&ret)
                    );
                }
                print_compile_report("run", &opts, &compiled)?;
                return Ok(true);
            }
            return Ok(false);
        }

        if !root.pop() {
            break;
        }
    }

    Ok(false)
}

pub(crate) fn load_project_native_build_input(
    source_path: &Path,
) -> Result<Option<ProjectNativeBuildInput>, Box<dyn Error>> {
    let absolute_source = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(source_path)
    };
    let absolute_source = fs::canonicalize(&absolute_source).unwrap_or(absolute_source);
    let Some(mut root) = absolute_source.parent().map(Path::to_path_buf) else {
        return Ok(None);
    };

    loop {
        let bridge_path = root.join("bridge.json");
        if bridge_path.is_file() {
            if !project_bridge_main_matches_source(&root, &bridge_path, &absolute_source)? {
                return Ok(None);
            }
            let opts = ProjectCommandOptions {
                root: root.clone(),
                main_name: absolute_source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("main.n")
                    .to_string(),
                json: false,
                show_return: false,
                out: None,
                linker: None,
                profile: default_project_backend_profile(),
            };
            let compiled = compile_project_runtime_fast(&opts)?;
            let unit_names = compiled
                .units
                .iter()
                .map(|unit| unit.unit.name.clone())
                .collect();
            return Ok(Some(ProjectNativeBuildInput {
                program: compiled.linked_program,
                root,
                bridge: Some(bridge_path),
                main_source: absolute_source,
                source_mode: "formal-bridge",
                unit_names,
                edges: compiled.edges,
            }));
        }

        if !root.pop() {
            break;
        }
    }

    let source = fs::read_to_string(&absolute_source)?;
    if !source_has_project_import(&source) {
        return Ok(None);
    }
    let Some((root, compiled)) = compile_single_file_workspace(&absolute_source)? else {
        return Ok(None);
    };
    let unit_names = compiled
        .units
        .iter()
        .map(|unit| unit.unit.name.clone())
        .collect();
    Ok(Some(ProjectNativeBuildInput {
        program: compiled.linked_program,
        root,
        bridge: None,
        main_source: absolute_source,
        source_mode: "implicit-use-project",
        unit_names,
        edges: compiled.edges,
    }))
}

pub(crate) fn load_single_file_workspace_program(
    source_path: &Path,
) -> Result<Option<ncc_ast::Program>, Box<dyn Error>> {
    let Some((root, compiled)) = compile_single_file_workspace(source_path)? else {
        return Ok(None);
    };
    write_project_runtime_lowering_files(&root, &compiled)?;
    write_single_file_run_route_report(
        &root,
        source_path,
        "single-file-workspace-interpreter",
        "debug",
    )?;
    Ok(Some(compiled.linked_program))
}

pub(crate) fn load_single_file_workspace_program_for_run(
    source_path: &Path,
) -> Result<Option<ncc_ast::Program>, Box<dyn Error>> {
    let source = fs::read_to_string(source_path)?;
    if !source_has_project_import(&source) {
        return Ok(None);
    }
    load_single_file_workspace_program(source_path)
}

fn source_has_project_import(source: &str) -> bool {
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact.contains("use<project:")
}

fn project_bridge_main_matches_source(
    root: &Path,
    bridge_path: &Path,
    source_path: &Path,
) -> Result<bool, Box<dyn Error>> {
    let raw = fs::read_to_string(bridge_path)?;
    let bridge: serde_json::Value = serde_json::from_str(&raw)?;
    if bridge.get("schema").and_then(|v| v.as_str()) != Some("ncc.bridge.v1") {
        return Ok(false);
    }
    let main_name = bridge
        .get("main")
        .and_then(|v| v.as_str())
        .unwrap_or("main.n");
    let bridge_main = root.join(main_name);
    Ok(same_project_file(&bridge_main, source_path))
}

fn same_project_file(a: &Path, b: &Path) -> bool {
    let a = fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let b = fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    if cfg!(windows) {
        slash_path(&a).eq_ignore_ascii_case(&slash_path(&b))
    } else {
        a == b
    }
}

fn compile_single_file_workspace(
    source_path: &Path,
) -> Result<Option<(PathBuf, ProjectCompileResult)>, Box<dyn Error>> {
    let absolute_source = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(source_path)
    };
    let absolute_source = fs::canonicalize(&absolute_source).unwrap_or(absolute_source);
    let source_text = match fs::read_to_string(&absolute_source) {
        Ok(value) => value,
        Err(err) => return Err(Box::new(err)),
    };
    if !is_code_json_eligible_source(&absolute_source, &source_text) {
        return Ok(None);
    }
    let Some(root) = absolute_source.parent().map(Path::to_path_buf) else {
        return Ok(None);
    };

    let root_unit_name = file_stem_unit_name(&absolute_source)?;
    let mut units_by_name = BTreeMap::<String, ProjectSourceUnit>::new();
    let mut units_by_path = BTreeMap::<String, String>::new();
    let mut edges = BTreeSet::<(String, String)>::new();
    let mut import_stack = Vec::new();
    collect_single_file_workspace_unit(
        &root,
        &absolute_source,
        &root_unit_name,
        &mut units_by_name,
        &mut units_by_path,
        &mut edges,
        &mut import_stack,
    )?;

    let mut units = units_by_name.into_values().collect::<Vec<_>>();
    units.sort_by(|a, b| a.name.cmp(&b.name));
    write_code_json_files(&root, &units)?;

    let unit_names = units
        .iter()
        .map(|unit| unit.name.clone())
        .collect::<Vec<_>>();
    let edges = edges.into_iter().collect::<Vec<_>>();
    let link_order = compute_project_link_order(&unit_names, &edges);
    let profile = default_project_backend_profile();
    let object_hash = sha256_bytes(b"ncc.single_file_workspace.deferred_object");
    let mut bridges = BTreeMap::<String, ProjectBridgeUnit>::new();
    for unit in units {
        let is_main = same_project_file(&unit.source, &absolute_source);
        let deps = project_deps_for_unit(&unit.name, &edges);
        bridges.insert(
            unit.name.to_ascii_lowercase(),
            ProjectBridgeUnit {
                name: unit.name.clone(),
                source: unit.source.clone(),
                relative_source: unit.relative_source.clone(),
                code_relative: Some(unit.code_relative.clone()),
                source_hash: unit.hash.clone(),
                object_relative: project_empty_object_relative_path(
                    &unit.name,
                    &profile.target_spec,
                ),
                object_hash: object_hash.clone(),
                symbols: Vec::new(),
                imports: deps.clone(),
                deps,
                is_main,
            },
        );
    }

    let mut compiled_units = Vec::new();
    for unit_name in &link_order {
        let Some(unit) = bridges.remove(&unit_name.to_ascii_lowercase()) else {
            return Err(format!(
                "E_SINGLE_FILE_LINK_ORDER_001: implicit link_order unit '{}' was not loaded",
                unit_name
            )
            .into());
        };
        compiled_units.push(compile_project_unit_ast_only(&root, unit)?);
    }
    let linked_program = link_project_program(&compiled_units)?;
    let compiled = ProjectCompileResult {
        units: compiled_units,
        edges,
        linked_program,
        linked_ir_debug: "single_file_workspace_runtime_fast".to_string(),
        linked_backend_debug: "single_file_workspace_runtime_fast".to_string(),
    };
    Ok(Some((root, compiled)))
}

fn collect_single_file_workspace_unit(
    root: &Path,
    source_path: &Path,
    unit_name: &str,
    units_by_name: &mut BTreeMap<String, ProjectSourceUnit>,
    units_by_path: &mut BTreeMap<String, String>,
    edges: &mut BTreeSet<(String, String)>,
    import_stack: &mut Vec<(String, String, PathBuf)>,
) -> Result<(), Box<dyn Error>> {
    let official = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let path_key = if cfg!(windows) {
        slash_path(&official).to_ascii_lowercase()
    } else {
        slash_path(&official)
    };
    if let Some(pos) = import_stack
        .iter()
        .position(|(stack_key, _, _)| stack_key == &path_key)
    {
        let mut chain = import_stack[pos..]
            .iter()
            .map(|(_, name, _)| name.clone())
            .collect::<Vec<_>>();
        chain.push(unit_name.to_string());
        let mut paths = import_stack[pos..]
            .iter()
            .map(|(_, _, path)| slash_path(path))
            .collect::<Vec<_>>();
        paths.push(slash_path(&official));
        return Err(format!(
            "E_SINGLE_FILE_IMPORT_CYCLE_001: implicit project import cycle detected: {} (paths: {})",
            chain.join(" -> "),
            paths.join(" -> ")
        )
        .into());
    }
    if let Some(existing) = units_by_path.get(&path_key) {
        if existing.eq_ignore_ascii_case(unit_name) {
            return Ok(());
        }
        return Err(format!(
            "E_SINGLE_FILE_IMPORT_003: source '{}' is imported as both '{}' and '{}'",
            slash_path(&official),
            existing,
            unit_name
        )
        .into());
    }
    let name_key = unit_name.to_ascii_lowercase();
    if let Some(existing) = units_by_name.get(&name_key) {
        if same_project_file(&existing.source, &official) {
            return Ok(());
        }
        return Err(format!(
            "E_SINGLE_FILE_IMPORT_004: import unit '{}' resolves to multiple files: '{}' and '{}'",
            unit_name,
            slash_path(&existing.source),
            slash_path(&official)
        )
        .into());
    }

    let source = fs::read_to_string(&official)?;
    if !is_code_json_eligible_source(&official, &source) {
        return Err(format!(
            "E_SINGLE_FILE_IMPORT_005: project import target '{}' must resolve to normal .n/.nc source, got '{}'",
            unit_name,
            slash_path(&official)
        )
        .into());
    }
    let source_bytes = source.as_bytes().to_vec();
    let hash = sha256_text(&source);
    let relative_source =
        relative_slash_path(root, &official).unwrap_or_else(|_| slash_path(&official));
    let code_relative = format!(".ncc/code/{}.json", sanitize_unit_symbol(unit_name));
    let program = read_or_parse_single_file_workspace_ast(
        root,
        unit_name,
        &official,
        &relative_source,
        &code_relative,
        &hash,
        &source_bytes,
        &source,
    )?;
    let imports = collect_project_imports_from_program(&program);
    let unit = ProjectSourceUnit {
        name: unit_name.to_string(),
        source: official.clone(),
        relative_source,
        hash,
        code_relative,
    };
    units_by_path.insert(path_key, unit_name.to_string());
    units_by_name.insert(name_key, unit);

    import_stack.push((
        if cfg!(windows) {
            slash_path(&official).to_ascii_lowercase()
        } else {
            slash_path(&official)
        },
        unit_name.to_string(),
        official.clone(),
    ));
    for import in imports {
        let child = find_single_file_workspace_import(root, &import)?;
        edges.insert((unit_name.to_string(), import.clone()));
        collect_single_file_workspace_unit(
            root,
            &child,
            &import,
            units_by_name,
            units_by_path,
            edges,
            import_stack,
        )?;
    }
    import_stack.pop();
    Ok(())
}

fn read_or_parse_single_file_workspace_ast(
    root: &Path,
    unit_name: &str,
    source_path: &Path,
    relative_source: &str,
    code_relative: &str,
    hash: &str,
    source_bytes: &[u8],
    source_text: &str,
) -> Result<ncc_ast::Program, Box<dyn Error>> {
    let hash_bytes = sha256_digest_bytes(source_bytes);
    let cache_unit = ProjectBridgeUnit {
        name: unit_name.to_string(),
        source: source_path.to_path_buf(),
        relative_source: relative_source.to_string(),
        code_relative: Some(code_relative.to_string()),
        source_hash: hash.to_string(),
        object_relative: String::new(),
        object_hash: String::new(),
        symbols: Vec::new(),
        imports: Vec::new(),
        deps: Vec::new(),
        is_main: false,
    };
    if let Some((ast, _)) = read_project_ast_cache(root, &cache_unit, hash, source_bytes)? {
        return Ok(ast);
    }
    let tokens = ncc_lexer::lex(source_text)?;
    let token_count = tokens.len();
    let ast = ncc_parser::parse(tokens)?;
    write_project_ast_cache(
        root,
        &cache_unit,
        hash,
        &hash_bytes,
        source_bytes,
        &ast,
        token_count,
    )?;
    Ok(ast)
}

fn find_single_file_workspace_import(root: &Path, target: &str) -> Result<PathBuf, Box<dyn Error>> {
    if target.contains('/') || target.contains('\\') || target.trim().is_empty() {
        return Err(format!(
            "E_SINGLE_FILE_IMPORT_001: project import target '{}' must be a file unit name",
            target
        )
        .into());
    }
    let mut candidates = Vec::new();
    let search_roots = single_file_workspace_import_search_roots(root);
    for search_root in &search_roots {
        collect_single_file_workspace_import_candidates(search_root, target, &mut candidates)?;
    }
    candidates.sort();
    candidates.dedup_by(|a, b| same_project_file(a, b));
    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(format!(
            "E_SINGLE_FILE_IMPORT_001: project import target '{}' was not found near '{}' (searched: {})",
            target,
            slash_path(root),
            search_roots
                .iter()
                .map(|path| slash_path(path))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into()),
        _ => Err(format!(
            "E_SINGLE_FILE_IMPORT_002: project import target '{}' is ambiguous ({} candidates). Rename one file or narrow NCC_PROJECT_PATH. Candidates: {}",
            target,
            candidates.len(),
            candidates
                .iter()
                .map(|path| slash_path(path))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into()),
    }
}

fn single_file_workspace_import_search_roots(root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    roots.push(root.to_path_buf());
    roots.push(root.join("src"));
    if let Some(raw) = env::var_os("NCC_PROJECT_PATH") {
        for path in env::split_paths(&raw) {
            roots.push(if path.is_absolute() {
                path
            } else {
                root.join(path)
            });
        }
    }
    let mut seen = BTreeSet::new();
    roots
        .into_iter()
        .filter(|path| path.is_dir())
        .filter(|path| {
            let key = fs::canonicalize(path)
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            seen.insert(key)
        })
        .collect()
}

fn collect_single_file_workspace_import_candidates(
    dir: &Path,
    target: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    if !dir.is_dir() {
        return Ok(());
    }
    let dir_name = dir.file_name().and_then(|name| name.to_str()).unwrap_or("");
    if dir_name.eq_ignore_ascii_case(".ncc")
        || dir_name.eq_ignore_ascii_case("target")
        || dir_name.eq_ignore_ascii_case("node_modules")
    {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_single_file_workspace_import_candidates(&path, target, out)?;
            continue;
        }
        let extension_ok = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("n") || ext.eq_ignore_ascii_case("nc"))
            .unwrap_or(false);
        let stem_ok = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.eq_ignore_ascii_case(target))
            .unwrap_or(false);
        if extension_ok && stem_ok {
            out.push(path);
        }
    }
    Ok(())
}

fn file_stem_unit_name(path: &Path) -> Result<String, Box<dyn Error>> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .map(|stem| stem.to_string())
        .ok_or_else(|| {
            format!(
                "E_SINGLE_FILE_IMPORT_006: source '{}' does not have a valid file stem",
                slash_path(path)
            )
            .into()
        })
}

fn project_usage() -> &'static str {
    "Usage: project <init|sync|doctor|model|incremental|check|emit-ir|graph|link|run|targets> [path] [--main <main.n>] [--json] [--show-return] [--cc <profile>] [--target <triple>] [--out <path>] [--linker <bin>]"
}

fn parse_project_options(args: &[String]) -> Result<ProjectCommandOptions, Box<dyn Error>> {
    let mut root: Option<PathBuf> = None;
    let mut main_name = "main.n".to_string();
    let mut json = false;
    let mut show_return = false;
    let mut out: Option<PathBuf> = None;
    let mut linker: Option<String> = None;
    let mut profile: Option<ncc_ir::BackendLoweringProfileIR> = None;
    let mut idx = 0usize;

    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--show-return" => {
                show_return = true;
                idx += 1;
            }
            "--out" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("project: --out requires an output path".into());
                };
                out = Some(PathBuf::from(value));
                idx += 2;
            }
            "--linker" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("project: --linker requires a linker binary".into());
                };
                linker = Some(value.clone());
                idx += 2;
            }
            "--cc" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("project: --cc requires nC.sysv64, nC.win64, nC.aapcs64, nC.aarch64-linux, nC.portable or nC.sovereign".into());
                };
                if profile.is_some() {
                    return Err("project: duplicate backend target selection".into());
                }
                profile = Some(project_backend_profile_from_selector(value)?);
                idx += 2;
            }
            "--target" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("project: --target requires a target triple".into());
                };
                if profile.is_some() {
                    return Err("project: duplicate backend target selection".into());
                }
                profile = Some(project_backend_profile_from_selector(value)?);
                idx += 2;
            }
            "--main" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("project: --main requires a file name".into());
                };
                if value.contains('/') || value.contains('\\') {
                    return Err("project: --main only accepts a file name, not a path".into());
                }
                main_name = value.clone();
                idx += 2;
            }
            flag if flag.starts_with("--") => {
                return Err(format!("project: unknown option '{}'", flag).into());
            }
            path => {
                if root.is_some() {
                    return Err("project: expected at most one project path".into());
                }
                root = Some(PathBuf::from(path));
                idx += 1;
            }
        }
    }

    Ok(ProjectCommandOptions {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        main_name,
        json,
        show_return,
        out,
        linker,
        profile: profile.unwrap_or_else(default_project_backend_profile),
    })
}

fn default_project_backend_profile() -> ncc_ir::BackendLoweringProfileIR {
    if cfg!(target_os = "windows") {
        ncc_ir::BackendLoweringProfileIR::nc_win64()
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        ncc_ir::BackendLoweringProfileIR::nc_aapcs64()
    } else if cfg!(target_os = "macos") {
        project_x86_64_macos_profile()
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        project_aarch64_linux_profile()
    } else {
        ncc_ir::BackendLoweringProfileIR::nc_sysv64()
    }
}

fn project_backend_profile_from_selector(
    raw: &str,
) -> Result<ncc_ir::BackendLoweringProfileIR, Box<dyn Error>> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "nc.sysv64" | "sysv64" | "nc-sysv64" | "x86_64-unknown-linux-gnu" | "linux-x64"
        | "linux-x86_64" => Ok(ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
        "nc.win64" | "win64" | "nc-win64" | "x86_64-pc-windows-msvc"
        | "x86_64-pc-windows-gnu" | "windows-x64" | "windows-x86_64" => {
            Ok(ncc_ir::BackendLoweringProfileIR::nc_win64())
        }
        "nc.aapcs64" | "aapcs64" | "nc-aapcs64" | "aarch64-apple-darwin"
        | "arm64-apple-darwin" | "macos-arm64" | "macos-aarch64" => {
            Ok(ncc_ir::BackendLoweringProfileIR::nc_aapcs64())
        }
        "x86_64-apple-darwin" | "macos-x64" | "macos-x86_64" => {
            Ok(project_x86_64_macos_profile())
        }
        "aarch64-unknown-linux-gnu" | "arm64-unknown-linux-gnu" | "linux-arm64"
        | "linux-aarch64" => Ok(project_aarch64_linux_profile()),
        "nc.portable" | "portable" | "nc-portable" | "wasm32-wasi" | "wasi32" => {
            Ok(ncc_ir::BackendLoweringProfileIR::nc_portable())
        }
        "nc.sovereign" | "sovereign" | "nc-sovereign" | "x86_64-unknown-sovereign-nc" => {
            Ok(ncc_ir::BackendLoweringProfileIR::nc_sovereign())
        }
        "nc.destructive" | "destructive" | "nc-destructive" => {
            Ok(ncc_ir::BackendLoweringProfileIR::nc_destructive())
        }
        _ => Err(format!(
            "project: unknown backend target '{}'; expected nC.sysv64, nC.win64, nC.aapcs64, x86_64-apple-darwin, aarch64-unknown-linux-gnu, nC.portable, nC.sovereign or nC.destructive",
            raw
        )
        .into()),
    }
}

fn project_aarch64_linux_profile() -> ncc_ir::BackendLoweringProfileIR {
    let target = ncc_ir::BackendTargetIR::AArch64;
    let calling_convention = ncc_ir::BackendCallingConventionIR::NcAapcs64;
    let mut target_spec = ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(
        &target,
        &calling_convention,
    );
    target_spec.target_triple = "aarch64-unknown-linux-gnu".to_string();
    target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
    target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
    ncc_ir::BackendLoweringProfileIR {
        target_spec,
        cpu_tuning: ncc_ir::backend_cpu_tuning_profile_for_backend_target(&target),
        destructive_policy: ncc_ir::BackendDestructivePolicyIR::strict(),
        target,
        calling_convention,
    }
}

fn project_x86_64_macos_profile() -> ncc_ir::BackendLoweringProfileIR {
    let target = ncc_ir::BackendTargetIR::X86_64;
    let calling_convention = ncc_ir::BackendCallingConventionIR::NcSysV64;
    let mut target_spec = ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(
        &target,
        &calling_convention,
    );
    target_spec.target_triple = "x86_64-apple-darwin".to_string();
    target_spec.object_format = ncc_ir::BackendObjectFormatIR::MachO;
    target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
    ncc_ir::BackendLoweringProfileIR {
        target_spec,
        cpu_tuning: ncc_ir::backend_cpu_tuning_profile_for_backend_target(&target),
        destructive_policy: ncc_ir::BackendDestructivePolicyIR::strict(),
        target,
        calling_convention,
    }
}

fn project_target_matrix_payload() -> serde_json::Value {
    let profiles = vec![
        (
            "nC.sysv64",
            "linux",
            "x86_64",
            ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
        ),
        (
            "nC.win64",
            "windows",
            "x86_64",
            ncc_ir::BackendLoweringProfileIR::nc_win64(),
        ),
        (
            "nC.aapcs64",
            "macos",
            "aarch64",
            ncc_ir::BackendLoweringProfileIR::nc_aapcs64(),
        ),
        (
            "nC.macos64",
            "macos",
            "x86_64",
            project_x86_64_macos_profile(),
        ),
        (
            "nC.aarch64-linux",
            "linux",
            "aarch64",
            project_aarch64_linux_profile(),
        ),
        (
            "nC.portable",
            "portable",
            "wasm32",
            ncc_ir::BackendLoweringProfileIR::nc_portable(),
        ),
        (
            "nC.sovereign",
            "sovereign",
            "x86_64",
            ncc_ir::BackendLoweringProfileIR::nc_sovereign(),
        ),
    ];
    json!({
        "schema": "ncc.project.targets.v1",
        "status": "ok",
        "default": project_profile_payload(&default_project_backend_profile()),
        "targets": profiles.into_iter().map(|(id, os, arch, profile)| {
            let spec = &profile.target_spec;
            json!({
                "id": id,
                "os": os,
                "arch": arch,
                "target_triple": spec.target_triple,
                "object_format": project_backend_object_format_label(&spec.object_format),
                "abi": project_backend_abi_label(&spec.abi),
                "isa": project_backend_isa_label(&spec.isa),
                "pointer_width": spec.pointer_width,
                "support_tier": project_backend_support_tier_label(&spec.support_tier),
                "run_support": "ncc-run-runtime",
                "object_support": "backend-object-bytes",
                "link_support": "requires-native-or-cross-linker"
            })
        }).collect::<Vec<_>>()
    })
}

struct ProjectCacheLock {
    root_key: String,
    lock_dir: PathBuf,
    owned: bool,
}

impl Drop for ProjectCacheLock {
    fn drop(&mut self) {
        if self.owned {
            let _ = fs::remove_dir_all(&self.lock_dir);
            PROJECT_CACHE_LOCKS_HELD.with(|held| {
                held.borrow_mut().remove(&self.root_key);
            });
        }
    }
}

fn acquire_project_cache_lock(
    root: &Path,
    reason: &str,
) -> Result<ProjectCacheLock, Box<dyn Error>> {
    let root_abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        env::current_dir()?.join(root)
    };
    let root_abs = fs::canonicalize(&root_abs).unwrap_or(root_abs);
    let root_key = slash_path(&root_abs).to_ascii_lowercase();
    let lock_dir = root_abs
        .join(".ncc")
        .join("locks")
        .join("project-cache.lock");

    let already_held = PROJECT_CACHE_LOCKS_HELD.with(|held| held.borrow().contains(&root_key));
    if already_held {
        return Ok(ProjectCacheLock {
            root_key,
            lock_dir,
            owned: false,
        });
    }

    let timeout_ms = env::var("NCC_CACHE_LOCK_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(PROJECT_CACHE_LOCK_TIMEOUT_MS);
    let stale_after = Duration::from_secs(PROJECT_CACHE_LOCK_STALE_SECS);
    let started = Instant::now();
    fs::create_dir_all(
        lock_dir
            .parent()
            .ok_or("project cache lock without parent")?,
    )?;

    loop {
        match fs::create_dir(&lock_dir) {
            Ok(()) => {
                PROJECT_CACHE_LOCKS_HELD.with(|held| {
                    held.borrow_mut().insert(root_key.clone());
                });
                let owner = json!({
                    "schema": "ncc.project.cache_lock.v1",
                    "status": "held",
                    "reason": reason,
                    "pid": std::process::id(),
                    "root": slash_path(&root_abs)
                });
                let _ = write_json_unlocked(&lock_dir.join("owner.json"), &owner);
                let _ = write_project_cache_lock_report(&root_abs);
                return Ok(ProjectCacheLock {
                    root_key,
                    lock_dir,
                    owned: true,
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if project_cache_lock_is_stale(&lock_dir, stale_after) {
                    let _ = fs::remove_dir_all(&lock_dir);
                    continue;
                }
                if started.elapsed() >= Duration::from_millis(timeout_ms) {
                    return Err(format!(
                        "E_PROJECT_CACHE_LOCK_001: timed out waiting for cache lock '{}' while {}; another ncc process is writing project artifacts",
                        slash_path(&lock_dir),
                        reason
                    )
                    .into());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(Box::new(err)),
        }
    }
}

fn project_cache_lock_is_stale(lock_dir: &Path, stale_after: Duration) -> bool {
    let Ok(metadata) = fs::metadata(lock_dir) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age >= stale_after)
        .unwrap_or(false)
}

fn init_project(
    opts: &ProjectCommandOptions,
) -> Result<(Vec<ProjectSourceUnit>, Vec<(String, String)>), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(&opts.root, "initializing project cache")?;
    init_project_unlocked(opts)?;
    let mut units = collect_project_sources(&opts.root)?;
    assign_duplicate_safe_unit_names(&mut units);
    let edges = collect_project_edges(&opts.root, &opts.main_name, &units)?;
    validate_project_edges_known(&edges, &units)?;
    Ok((units, edges))
}

fn init_project_unlocked(opts: &ProjectCommandOptions) -> Result<(), Box<dyn Error>> {
    ensure_project_dirs(&opts.root)?;
    let main_path = opts.root.join(&opts.main_name);
    if !main_path.exists() {
        write_text(&main_path, "")?;
    }
    let main_unit = project_main_source_unit(opts)?;
    write_code_json_files(&opts.root, std::slice::from_ref(&main_unit))?;
    if !opts.root.join("bridge.json").exists() {
        let mut units = collect_project_sources(&opts.root)?;
        assign_duplicate_safe_unit_names(&mut units);
        let edges = collect_project_edges(&opts.root, &opts.main_name, &units)?;
        validate_project_edges_known(&edges, &units)?;
        write_code_json_files(&opts.root, &units)?;
        write_bridge_json(&opts.root, &main_unit, &units, &edges, &opts.profile)?;
    }
    Ok(())
}

fn sync_project(
    opts: &ProjectCommandOptions,
) -> Result<(Vec<ProjectSourceUnit>, Vec<(String, String)>), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(&opts.root, "synchronizing project graph")?;
    init_project_unlocked(opts)?;
    let main_unit = project_main_source_unit(opts)?;
    let mut units = collect_project_sources(&opts.root)?;
    assign_duplicate_safe_unit_names(&mut units);
    let edges = collect_project_edges(&opts.root, &opts.main_name, &units)?;
    validate_project_edges_known(&edges, &units)?;
    write_code_json_files(&opts.root, std::slice::from_ref(&main_unit))?;
    write_code_json_files(&opts.root, &units)?;
    write_bridge_json(&opts.root, &main_unit, &units, &edges, &opts.profile)?;
    maybe_write_project_sync_baseline(opts, &main_unit, &units)?;
    Ok((units, edges))
}

fn maybe_write_project_sync_baseline(
    opts: &ProjectCommandOptions,
    main_unit: &ProjectSourceUnit,
    units: &[ProjectSourceUnit],
) -> Result<(), Box<dyn Error>> {
    if !project_sync_sources_are_ready_for_lowering(main_unit, units)? {
        return Ok(());
    }
    let Ok(compiled) = compile_project_with_object_materialization(opts, false, false, false)
    else {
        return Ok(());
    };
    write_project_runtime_lowering_files(&opts.root, &compiled)
}

fn project_sync_sources_are_ready_for_lowering(
    main_unit: &ProjectSourceUnit,
    units: &[ProjectSourceUnit],
) -> Result<bool, Box<dyn Error>> {
    for unit in std::iter::once(main_unit).chain(units.iter()) {
        let source = fs::read_to_string(&unit.source)?;
        if source.trim().is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn project_doctor_report(
    opts: &ProjectCommandOptions,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let bridge = opts.root.join("bridge.json");
    let code_dir = opts.root.join(".ncc").join("code");
    let code_files = if code_dir.is_dir() {
        fs::read_dir(&code_dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    };
    let mut bridge_units = 0usize;
    let mut edges = 0usize;
    let mut physical_objects = 0usize;
    let mut link_order = Vec::new();
    let mut bridge_errors = Vec::new();
    let mut identity_errors = Vec::new();

    if bridge.is_file() {
        match load_bridge_manifest(opts) {
            Ok(manifest) => {
                bridge_units = manifest.units.len();
                physical_objects = manifest.units.len();
                edges = manifest.edges.len();
                link_order = manifest.link_order.clone();
                for unit in manifest.units {
                    match fs::read(&unit.source) {
                        Ok(source_bytes) => {
                            let hash_bytes = sha256_digest_bytes(&source_bytes);
                            let hash = format!("sha256:{}", bytes_to_hex_joined(&hash_bytes, ""));
                            if unit.source_hash != hash {
                                identity_errors.push(format!(
                                    "E_PROJECT_BRIDGE_SOURCE_HASH_001: unit '{}' bridge source_hash mismatch: expected '{}', got '{}'",
                                    unit.name, hash, unit.source_hash
                                ));
                            }
                            match &unit.code_relative {
                                Some(code_relative) => {
                                    if let Err(err) =
                                        verify_code_json(&opts.root, &unit, code_relative, &hash)
                                    {
                                        identity_errors.push(err.to_string());
                                    }
                                }
                                None => identity_errors.push(format!(
                                    "E_PROJECT_CODE_IDENTITY_003: unit '{}' has no code identity in bridge.json",
                                    unit.name
                                )),
                            }
                        }
                        Err(err) => identity_errors.push(format!(
                            "E_PROJECT_SOURCE_MISSING_001: source '{}' for unit '{}' cannot be read: {err}",
                            slash_path(&unit.source),
                            unit.name
                        )),
                    }
                }
            }
            Err(err) => bridge_errors.push(err.to_string()),
        }
    }

    let status = if !bridge.is_file() {
        "missing_bridge"
    } else if identity_errors.is_empty() && bridge_errors.is_empty() {
        "ok"
    } else {
        "error"
    };

    Ok(json!({
        "schema": "ncc.project.priority13a.doctor.v1",
        "status": status,
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "bridge": slash_path(&bridge),
        "units": bridge_units,
        "edges": edges,
        "physical_objects": physical_objects,
        "link_order": link_order,
        "code_files": code_files,
        "bridge_errors": bridge_errors,
        "identity_errors": identity_errors
    }))
}

fn project_model_report(opts: &ProjectCommandOptions) -> Result<serde_json::Value, Box<dyn Error>> {
    let bridge_path = opts.root.join("bridge.json");
    if !bridge_path.is_file() {
        return Ok(json!({
            "schema": "ncc.project.model.v1",
            "status": "missing_bridge",
            "root": slash_path(&opts.root),
            "main": opts.main_name,
            "bridge": slash_path(&bridge_path),
            "target": project_profile_payload(&opts.profile),
            "units": [],
            "edges": [],
            "link_order": [],
            "dirty_units": [],
            "diagnostics": [{
                "code": "E_PROJECT_BRIDGE_MISSING_001",
                "message": format!("missing bridge.json; run `ncc project sync {}` first", slash_path(&opts.root))
            }]
        }));
    }

    let bridge_raw = fs::read_to_string(&bridge_path)?;
    let bridge: serde_json::Value = serde_json::from_str(&bridge_raw)?;
    if bridge.get("schema").and_then(|v| v.as_str()) != Some("ncc.bridge.v1") {
        return Err(
            "E_PROJECT_BRIDGE_SCHEMA_001: bridge.json must use schema ncc.bridge.v1".into(),
        );
    }

    let raw_units = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge.json requires units array")?;
    let edges = project_model_edges_payload(&bridge)?;
    let link_order = bridge
        .get("link_order")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut units = Vec::new();
    let mut dirty_units = Vec::new();
    let mut diagnostics = Vec::new();

    for raw in raw_units {
        let name = raw
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge unit requires name")?;
        let source_rel = raw
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge unit requires source")?;
        let expected_source_hash = raw
            .get("source_hash")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_004: bridge unit requires source_hash")?;
        let code_rel = raw.get("code").and_then(|v| v.as_str());
        let object_rel = raw
            .get("object")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_OBJECT_001: bridge unit requires object path")?;
        let expected_object_hash = raw
            .get("object_hash")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_OBJECT_HASH_001: bridge unit requires object_hash")?;

        let source_path = opts.root.join(source_rel);
        let (source_status, current_source_hash) =
            project_model_source_status(&source_path, expected_source_hash)?;
        let (code_status, code_diagnostics) = project_model_code_status(
            &opts.root,
            name,
            source_rel,
            expected_source_hash,
            code_rel,
        )?;
        let (lowering_status, lowering_diagnostics) =
            project_model_lowering_status(&opts.root, name, source_rel, expected_source_hash)?;
        let (object_status, current_object_hash, object_bytes, object_diagnostics) =
            project_model_object_status(&opts.root, object_rel, expected_object_hash)?;

        diagnostics.extend(code_diagnostics);
        diagnostics.extend(lowering_diagnostics);
        diagnostics.extend(object_diagnostics);

        let unit_state =
            project_model_unit_state(source_status, code_status, lowering_status, object_status);
        if unit_state != "clean" {
            dirty_units.push(name.to_string());
        }

        units.push(json!({
            "name": name,
            "source": source_rel,
            "source_hash": expected_source_hash,
            "current_source_hash": current_source_hash,
            "source_status": source_status,
            "code": code_rel,
            "code_status": code_status,
            "lowering": format!(".ncc/lowering/{}.lowering.json", name),
            "lowering_status": lowering_status,
            "object": object_rel,
            "object_hash": expected_object_hash,
            "current_object_hash": current_object_hash,
            "object_bytes": object_bytes,
            "object_status": object_status,
            "state": unit_state,
            "symbols": raw.get("symbols").cloned().unwrap_or_else(|| json!([])),
            "imports": raw.get("imports").cloned().unwrap_or_else(|| json!([])),
            "deps": raw.get("deps").cloned().unwrap_or_else(|| json!([])),
            "is_main": name.eq_ignore_ascii_case("main")
        }));
    }

    let (linked_lowering_status, linked_lowering_diagnostics) =
        project_model_linked_lowering_status(&opts.root, &units)?;
    diagnostics.extend(linked_lowering_diagnostics);
    if linked_lowering_status != "clean" {
        dirty_units = units
            .iter()
            .filter_map(|unit| {
                unit.get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
    }
    let status = if dirty_units.is_empty() && diagnostics.is_empty() {
        "ok"
    } else if diagnostics.is_empty() {
        "stale"
    } else {
        "needs_attention"
    };

    Ok(json!({
        "schema": "ncc.project.model.v1",
        "status": status,
        "root": slash_path(&opts.root),
        "main": bridge.get("main").and_then(|v| v.as_str()).unwrap_or(&opts.main_name),
        "bridge": slash_path(&bridge_path),
        "code_dir": ".ncc/code",
        "lowering_dir": ".ncc/lowering",
        "linked_lowering": ".ncc/lowering/__linked.lowering.json",
        "linked_lowering_status": linked_lowering_status,
        "object_dir": ".ncc/obj",
        "target": project_profile_payload(&opts.profile),
        "units": units,
        "edges": edges,
        "link_order": link_order,
        "dirty_units": dirty_units,
        "diagnostics": diagnostics
    }))
}

fn project_incremental_report(
    opts: &ProjectCommandOptions,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let model = project_model_report(opts)?;
    let model_status = model
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("error");
    if model_status == "missing_bridge" {
        return Ok(json!({
            "schema": "ncc.project.incremental.v1",
            "status": "missing_bridge",
            "root": slash_path(&opts.root),
            "main": opts.main_name,
            "model": model,
            "dirty_roots": [],
            "rebuild_units": [],
            "cached_units": [],
            "reason_by_unit": {}
        }));
    }

    let units = model
        .get("units")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let edges = model
        .get("edges")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let link_order = model
        .get("link_order")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut known_units = BTreeSet::new();
    let mut dirty = BTreeSet::<String>::new();
    let mut reason_by_unit = BTreeMap::<String, Vec<String>>::new();

    for unit in &units {
        let Some(name) = unit.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        known_units.insert(name.to_ascii_lowercase());
        let mut reasons = Vec::new();
        for field in [
            "source_status",
            "code_status",
            "lowering_status",
            "object_status",
        ] {
            let status = unit
                .get(field)
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            if status != "clean" {
                reasons.push(format!("{field}={status}"));
            }
        }
        if !reasons.is_empty() {
            dirty.insert(name.to_ascii_lowercase());
            reason_by_unit.insert(name.to_string(), reasons);
        }
    }
    let linked_lowering_status = model
        .get("linked_lowering_status")
        .and_then(|v| v.as_str())
        .unwrap_or("missing");
    if linked_lowering_status != "clean" {
        for unit in &units {
            let Some(name) = unit.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            dirty.insert(name.to_ascii_lowercase());
            reason_by_unit
                .entry(name.to_string())
                .or_default()
                .push(format!("linked_lowering_status={linked_lowering_status}"));
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for edge in &edges {
            let Some(pair) = edge.as_array() else {
                continue;
            };
            if pair.len() != 2 {
                continue;
            }
            let Some(from) = pair[0].as_str() else {
                continue;
            };
            let Some(to) = pair[1].as_str() else {
                continue;
            };
            if dirty.contains(&to.to_ascii_lowercase()) {
                let reasons = reason_by_unit.entry(from.to_string()).or_default();
                let dependency_reason = format!("dependency_changed={to}");
                if !reasons.iter().any(|reason| reason == &dependency_reason) {
                    reasons.push(dependency_reason);
                }
                if dirty.insert(from.to_ascii_lowercase()) {
                    changed = true;
                }
            }
        }
    }

    let ordered_names = if link_order.is_empty() {
        units
            .iter()
            .filter_map(|unit| unit.get("name").and_then(|v| v.as_str()))
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
    } else {
        link_order
            .iter()
            .filter_map(|value| value.as_str())
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
    };
    let rebuild_units = ordered_names
        .iter()
        .filter(|name| dirty.contains(&name.to_ascii_lowercase()))
        .cloned()
        .collect::<Vec<_>>();
    let cached_units = ordered_names
        .iter()
        .filter(|name| {
            known_units.contains(&name.to_ascii_lowercase())
                && !dirty.contains(&name.to_ascii_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();
    let dirty_roots = reason_by_unit.keys().cloned().collect::<Vec<_>>();
    let status = if rebuild_units.is_empty() {
        "clean"
    } else {
        "rebuild_required"
    };
    let semantic_validation = if rebuild_units.is_empty() {
        "skipped_linked_clean_cache"
    } else {
        "linked_program_full_validation_required"
    };
    let fingerprint = project_incremental_fingerprint(
        &opts.profile,
        &units,
        &edges,
        &link_order,
        &rebuild_units,
        &cached_units,
        linked_lowering_status,
    )?;

    Ok(json!({
        "schema": "ncc.project.incremental.v1",
        "status": status,
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "cache": {
            "status_file": ".ncc/incremental/status.json",
            "identity": "target + source_hash + code_json + lowering_json + object_hash + bridge_edges + link_order",
            "fingerprint": fingerprint,
            "semantic_compile": semantic_validation,
            "semantic_validation": semantic_validation,
            "linked_program_policy": "full validation runs only when a dirty root or dependency propagation requires rebuild"
        },
        "dirty_roots": dirty_roots,
        "rebuild_units": rebuild_units,
        "cached_units": cached_units,
        "reason_by_unit": reason_by_unit,
        "model": model
    }))
}

fn project_cached_check_report(
    opts: &ProjectCommandOptions,
) -> Result<Option<serde_json::Value>, Box<dyn Error>> {
    let payload = project_incremental_report(opts)?;
    if payload.get("status").and_then(|value| value.as_str()) != Some("clean") {
        return Ok(None);
    }
    if !project_persisted_incremental_status_matches(opts, &payload)? {
        return Ok(None);
    }
    let manifest = load_bridge_manifest(opts)?;
    write_project_incremental_status(&opts.root, &payload)?;
    let linked_containers = payload
        .pointer("/model/units")
        .and_then(|value| value.as_array())
        .map(|units| {
            units
                .iter()
                .filter_map(|unit| unit.get("symbols").and_then(|value| value.as_array()))
                .flatten()
                .filter(|symbol| {
                    symbol.get("kind").and_then(|value| value.as_str()) == Some("container")
                })
                .count()
        })
        .unwrap_or(0);
    Ok(Some(json!({
        "schema": "ncc.project.cached_check.v1",
        "status": "ok",
        "action": "check",
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "units": manifest.units.len(),
        "edges": manifest.edges.len(),
        "linked_containers": linked_containers,
        "lowering_dir": ".ncc/lowering",
        "object_dir": ".ncc/obj",
        "semantic_validation": "skipped_linked_clean_cache",
        "incremental": payload
    })))
}

fn project_incremental_fingerprint(
    profile: &ncc_ir::BackendLoweringProfileIR,
    units: &[serde_json::Value],
    edges: &[serde_json::Value],
    link_order: &[serde_json::Value],
    rebuild_units: &[String],
    cached_units: &[String],
    linked_lowering_status: &str,
) -> Result<String, Box<dyn Error>> {
    let identity = json!({
        "schema": "ncc.project.incremental.identity.v1",
        "target": project_profile_payload(profile),
        "linked_lowering_status": linked_lowering_status,
        "units": units.iter().map(|unit| {
            json!({
                "name": unit.get("name").cloned().unwrap_or(Value::Null),
                "source": unit.get("source").cloned().unwrap_or(Value::Null),
                "source_hash": unit.get("source_hash").cloned().unwrap_or(Value::Null),
                "current_source_hash": unit.get("current_source_hash").cloned().unwrap_or(Value::Null),
                "source_status": unit.get("source_status").cloned().unwrap_or(Value::Null),
                "code": unit.get("code").cloned().unwrap_or(Value::Null),
                "code_status": unit.get("code_status").cloned().unwrap_or(Value::Null),
                "lowering": unit.get("lowering").cloned().unwrap_or(Value::Null),
                "lowering_status": unit.get("lowering_status").cloned().unwrap_or(Value::Null),
                "object": unit.get("object").cloned().unwrap_or(Value::Null),
                "object_hash": unit.get("object_hash").cloned().unwrap_or(Value::Null),
                "current_object_hash": unit.get("current_object_hash").cloned().unwrap_or(Value::Null),
                "object_status": unit.get("object_status").cloned().unwrap_or(Value::Null),
                "deps": unit.get("deps").cloned().unwrap_or_else(|| json!([]))
            })
        }).collect::<Vec<_>>(),
        "edges": edges,
        "link_order": link_order,
        "rebuild_units": rebuild_units,
        "cached_units": cached_units
    });
    let bytes = serde_json::to_vec(&identity)?;
    Ok(sha256_bytes(&bytes))
}

fn project_persisted_incremental_status_matches(
    opts: &ProjectCommandOptions,
    current: &serde_json::Value,
) -> Result<bool, Box<dyn Error>> {
    let status_path = opts
        .root
        .join(".ncc")
        .join("incremental")
        .join("status.json");
    if !status_path.is_file() {
        return Ok(false);
    }
    let raw = fs::read_to_string(status_path)?;
    let persisted: serde_json::Value = serde_json::from_str(&raw)?;
    if persisted.get("schema").and_then(|value| value.as_str())
        != Some("ncc.project.incremental.v1")
    {
        return Ok(false);
    }
    if persisted.get("status").and_then(|value| value.as_str()) != Some("clean") {
        return Ok(false);
    }
    let persisted_target = persisted
        .pointer("/target/target_triple")
        .and_then(|value| value.as_str());
    let current_target = current
        .pointer("/target/target_triple")
        .and_then(|value| value.as_str());
    let persisted_fingerprint = persisted
        .pointer("/cache/fingerprint")
        .and_then(|value| value.as_str());
    let current_fingerprint = current
        .pointer("/cache/fingerprint")
        .and_then(|value| value.as_str());
    Ok(persisted_target == current_target
        && persisted_fingerprint.is_some()
        && persisted_fingerprint == current_fingerprint)
}

fn write_project_incremental_status(
    root: &Path,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing incremental status")?;
    let status_path = root.join(".ncc").join("incremental").join("status.json");
    write_json(&status_path, payload)
}

fn print_project_model_report(payload: &serde_json::Value) {
    println!(
        "nC project model: status={} units={} edges={} dirty={}",
        payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("?"),
        payload
            .get("units")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0),
        payload
            .get("edges")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0),
        payload
            .get("dirty_units")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0)
    );
}

fn print_project_incremental_report(payload: &serde_json::Value) {
    println!(
        "nC project incremental: status={} rebuild={} cached={} status_file=.ncc/incremental/status.json",
        payload.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
        payload
            .get("rebuild_units")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0),
        payload
            .get("cached_units")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0)
    );
}

fn project_model_edges_payload(
    bridge: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let Some(raw_edges) = bridge.get("edges").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for edge in raw_edges {
        let Some(pair) = edge.as_array() else {
            return Err("E_PROJECT_BRIDGE_EDGE_001: bridge edge must be [from, to]".into());
        };
        if pair.len() != 2 || !pair[0].is_string() || !pair[1].is_string() {
            return Err("E_PROJECT_BRIDGE_EDGE_001: bridge edge must be [from, to]".into());
        }
        out.push(edge.clone());
    }
    Ok(out)
}

fn project_model_source_status(
    source_path: &Path,
    expected_hash: &str,
) -> Result<(&'static str, Option<String>), Box<dyn Error>> {
    if !source_path.is_file() {
        return Ok(("missing", None));
    }
    let bytes = fs::read(source_path)?;
    let hash = sha256_bytes(&bytes);
    if hash == expected_hash {
        Ok(("clean", Some(hash)))
    } else {
        Ok(("dirty", Some(hash)))
    }
}

fn project_model_code_status(
    root: &Path,
    unit: &str,
    source: &str,
    source_hash: &str,
    code_relative: Option<&str>,
) -> Result<(&'static str, Vec<serde_json::Value>), Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let Some(code_relative) = code_relative else {
        diagnostics.push(json!({
            "code": "E_PROJECT_CODE_IDENTITY_003",
            "unit": unit,
            "message": "bridge unit has no code identity"
        }));
        return Ok(("missing", diagnostics));
    };
    let path = root.join(code_relative);
    if !path.is_file() {
        diagnostics.push(json!({
            "code": "E_PROJECT_CODE_IDENTITY_001",
            "unit": unit,
            "path": code_relative,
            "message": "code identity file is missing"
        }));
        return Ok(("missing", diagnostics));
    }
    let raw = fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let checks = [
        ("schema", "ncc.code.v1"),
        ("unit", unit),
        ("source", source),
        ("hash", source_hash),
        ("bridge", "../../bridge.json"),
    ];
    for (field, expected) in checks {
        if value.get(field).and_then(|v| v.as_str()) != Some(expected) {
            diagnostics.push(json!({
                "code": "E_PROJECT_CODE_IDENTITY_002",
                "unit": unit,
                "path": code_relative,
                "field": field,
                "expected": expected,
                "message": "code identity field mismatch"
            }));
        }
    }
    if diagnostics.is_empty() {
        Ok(("clean", diagnostics))
    } else {
        Ok(("stale", diagnostics))
    }
}

fn project_model_lowering_status(
    root: &Path,
    unit: &str,
    source: &str,
    source_hash: &str,
) -> Result<(&'static str, Vec<serde_json::Value>), Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let lowering_relative = format!(".ncc/lowering/{}.lowering.json", unit);
    let path = root.join(&lowering_relative);
    if !path.is_file() {
        diagnostics.push(json!({
            "code": "E_PROJECT_LOWERING_MISSING_001",
            "unit": unit,
            "path": lowering_relative,
            "message": "unit lowering file is missing"
        }));
        return Ok(("missing", diagnostics));
    }
    let raw = fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let checks = [
        ("schema", "ncc.project.lowering.v1"),
        ("unit", unit),
        ("source", source),
        ("hash", source_hash),
    ];
    for (field, expected) in checks {
        if value.get(field).and_then(|v| v.as_str()) != Some(expected) {
            diagnostics.push(json!({
                "code": "E_PROJECT_LOWERING_STALE_001",
                "unit": unit,
                "path": lowering_relative,
                "field": field,
                "expected": expected,
                "message": "unit lowering identity field mismatch"
            }));
        }
    }
    if diagnostics.is_empty() {
        Ok(("clean", diagnostics))
    } else {
        Ok(("stale", diagnostics))
    }
}

fn project_model_linked_lowering_status(
    root: &Path,
    units: &[serde_json::Value],
) -> Result<(&'static str, Vec<serde_json::Value>), Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let lowering_relative = ".ncc/lowering/__linked.lowering.json";
    let path = root.join(lowering_relative);
    if !path.is_file() {
        diagnostics.push(json!({
            "code": "E_PROJECT_LINKED_LOWERING_MISSING_001",
            "path": lowering_relative,
            "message": "linked lowering file is missing"
        }));
        return Ok(("missing", diagnostics));
    }
    let raw = fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    if value.get("schema").and_then(|v| v.as_str()) != Some("ncc.project.linked_lowering.v1") {
        diagnostics.push(json!({
            "code": "E_PROJECT_LINKED_LOWERING_STALE_001",
            "path": lowering_relative,
            "field": "schema",
            "expected": "ncc.project.linked_lowering.v1",
            "message": "linked lowering schema mismatch"
        }));
    }
    let linked_units = value
        .get("units")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut linked_hashes = BTreeMap::<String, String>::new();
    for linked in linked_units {
        if let (Some(unit), Some(hash)) = (
            linked.get("unit").and_then(|v| v.as_str()),
            linked.get("hash").and_then(|v| v.as_str()),
        ) {
            linked_hashes.insert(unit.to_ascii_lowercase(), hash.to_string());
        }
    }
    for unit in units {
        let Some(name) = unit.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(expected_hash) = unit.get("source_hash").and_then(|v| v.as_str()) else {
            continue;
        };
        match linked_hashes.get(&name.to_ascii_lowercase()) {
            Some(actual) if actual == expected_hash => {}
            Some(actual) => diagnostics.push(json!({
                "code": "E_PROJECT_LINKED_LOWERING_STALE_002",
                "unit": name,
                "path": lowering_relative,
                "expected": expected_hash,
                "actual": actual,
                "message": "linked lowering unit hash mismatch"
            })),
            None => diagnostics.push(json!({
                "code": "E_PROJECT_LINKED_LOWERING_STALE_003",
                "unit": name,
                "path": lowering_relative,
                "expected": expected_hash,
                "message": "linked lowering is missing unit identity"
            })),
        }
    }
    if diagnostics.is_empty() {
        Ok(("clean", diagnostics))
    } else {
        Ok(("stale", diagnostics))
    }
}

fn project_model_object_status(
    root: &Path,
    object_relative: &str,
    expected_hash: &str,
) -> Result<(&'static str, Option<String>, u64, Vec<serde_json::Value>), Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let path = root.join(object_relative);
    if !path.is_file() {
        diagnostics.push(json!({
            "code": "E_PROJECT_BRIDGE_OBJECT_MISSING_001",
            "object": object_relative,
            "message": "object file is missing"
        }));
        return Ok(("missing", None, 0, diagnostics));
    }
    let bytes = fs::read(&path)?;
    let hash = sha256_bytes(&bytes);
    if hash != expected_hash {
        diagnostics.push(json!({
            "code": "E_PROJECT_BRIDGE_OBJECT_HASH_002",
            "object": object_relative,
            "expected": expected_hash,
            "actual": hash,
            "message": "object hash mismatch"
        }));
        return Ok(("stale", Some(hash), bytes.len() as u64, diagnostics));
    }
    Ok(("clean", Some(hash), bytes.len() as u64, diagnostics))
}

fn project_model_unit_state(
    source_status: &str,
    code_status: &str,
    lowering_status: &str,
    object_status: &str,
) -> &'static str {
    if source_status == "clean"
        && code_status == "clean"
        && lowering_status == "clean"
        && object_status == "clean"
    {
        "clean"
    } else {
        "dirty"
    }
}

fn compile_project(opts: &ProjectCommandOptions) -> Result<ProjectCompileResult, Box<dyn Error>> {
    compile_project_with_object_materialization(opts, true, true, true)
}

fn compile_project_runtime_fast(
    opts: &ProjectCommandOptions,
) -> Result<ProjectCompileResult, Box<dyn Error>> {
    let manifest = load_bridge_manifest_for_runtime(opts)?;
    let reachable = project_runtime_reachable_unit_keys(&manifest)?;
    let edges = manifest
        .edges
        .iter()
        .filter(|(from, to)| {
            reachable.contains(&from.to_ascii_lowercase())
                && reachable.contains(&to.to_ascii_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut units = Vec::new();
    let mut units_by_name = manifest
        .units
        .into_iter()
        .map(|unit| (unit.name.to_ascii_lowercase(), unit))
        .collect::<BTreeMap<_, _>>();
    for unit_name in &manifest.link_order {
        if !reachable.contains(&unit_name.to_ascii_lowercase()) {
            continue;
        }
        let Some(unit) = units_by_name.remove(&unit_name.to_ascii_lowercase()) else {
            return Err(format!(
                "E_PROJECT_BRIDGE_LINK_ORDER_004: link_order unit '{}' was not loaded",
                unit_name
            )
            .into());
        };
        units.push(compile_project_unit_ast_only(&opts.root, unit)?);
    }

    let linked_program = link_project_program(&units)?;
    Ok(ProjectCompileResult {
        units,
        edges,
        linked_program,
        linked_ir_debug: "skipped_runtime_fast_clean_cache".to_string(),
        linked_backend_debug: "skipped_runtime_fast_clean_cache".to_string(),
    })
}

fn project_runtime_reachable_unit_keys(
    manifest: &ProjectBridgeManifest,
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let units = manifest
        .units
        .iter()
        .map(|unit| (unit.name.to_ascii_lowercase(), unit))
        .collect::<BTreeMap<_, _>>();
    let roots = manifest
        .units
        .iter()
        .filter(|unit| unit.is_main)
        .map(|unit| unit.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err(project_runtime_error(
            ncc_diagnostics::codes::E_PROJECT_RUNTIME_ROOT_001,
            format!(
                "runtime graph requires exactly one main unit, found {}",
                roots.len()
            ),
        ));
    }

    let mut reachable = BTreeSet::new();
    let mut pending = roots;
    while let Some(unit_key) = pending.pop() {
        if !reachable.insert(unit_key.clone()) {
            continue;
        }
        let unit = units.get(&unit_key).ok_or_else(|| {
            project_runtime_error(
                ncc_diagnostics::codes::E_PROJECT_RUNTIME_ROOT_002,
                format!("runtime graph references missing unit '{}'", unit_key),
            )
        })?;
        for dependency in &unit.deps {
            let dependency_key = dependency.to_ascii_lowercase();
            if !units.contains_key(&dependency_key) {
                return Err(project_runtime_error(
                    ncc_diagnostics::codes::E_PROJECT_RUNTIME_DEP_001,
                    format!(
                        "unit '{}' depends on missing unit '{}'",
                        unit.name, dependency
                    ),
                ));
            }
            if !reachable.contains(&dependency_key) {
                pending.push(dependency_key);
            }
        }
    }
    Ok(reachable)
}

fn compile_project_with_object_materialization(
    opts: &ProjectCommandOptions,
    materialize_objects: bool,
    validate_linked_semantics: bool,
    lower_linked_backend: bool,
) -> Result<ProjectCompileResult, Box<dyn Error>> {
    let manifest = if materialize_objects {
        load_bridge_manifest_with_object_validation(opts, false)?
    } else {
        load_bridge_manifest(opts)?
    };
    let mut units = Vec::new();
    let mut units_by_name = manifest
        .units
        .into_iter()
        .map(|unit| (unit.name.to_ascii_lowercase(), unit))
        .collect::<BTreeMap<_, _>>();
    for unit_name in &manifest.link_order {
        let Some(unit) = units_by_name.remove(&unit_name.to_ascii_lowercase()) else {
            return Err(format!(
                "E_PROJECT_BRIDGE_LINK_ORDER_004: link_order unit '{}' was not loaded",
                unit_name
            )
            .into());
        };
        units.push(if lower_linked_backend {
            compile_project_unit(&opts.root, unit, &opts.profile)?
        } else {
            compile_project_unit_ast_only(&opts.root, unit)?
        });
    }

    let linked_program = link_project_program(&units)?;
    let (linked_ir_debug, linked_backend_debug) = if validate_linked_semantics {
        let sema_status = validate_project_linked_sema_cached(opts, &linked_program)?;
        if lower_linked_backend {
            let linked_ir =
                ncc_ir::lower_with_backend_profile(linked_program.clone(), opts.profile.clone());
            ncc_ir::verify_type_contract_lowering(&linked_program, &linked_ir)
                .map_err(|err| format!("E_PROJECT_LINK_IR_001: {err}"))?;
            let linked_backend = ncc_ir::emit_backend(&linked_ir);
            ncc_ir::verify_type_contract_backend_enforcement(&linked_ir, &linked_backend)
                .map_err(|err| format!("E_PROJECT_LINK_BACKEND_001: {err}"))?;
            (format!("{linked_ir:#?}"), format!("{linked_backend:#?}"))
        } else {
            (sema_status, "per_unit_object_backend".to_string())
        }
    } else {
        (
            "skipped_linked_clean_cache".to_string(),
            "skipped_linked_clean_cache".to_string(),
        )
    };
    if materialize_objects {
        materialize_project_objects(&opts.root, &mut units, &opts.profile, false)?;
    }

    Ok(ProjectCompileResult {
        units,
        edges: manifest.edges,
        linked_program,
        linked_ir_debug,
        linked_backend_debug,
    })
}

fn validate_project_linked_sema_cached(
    opts: &ProjectCommandOptions,
    linked_program: &ncc_ast::Program,
) -> Result<String, Box<dyn Error>> {
    let ast_bytes = serde_json::to_vec(linked_program)?;
    let material = json!({
        "schema": "ncc.project.linked_sema_content_key.v1",
        "linked_ast_sha256": crate::content_cache::sha256_bytes(&ast_bytes),
        "target": project_profile_payload(&opts.profile),
        "compiler": crate::content_cache::compiler_image_identity(),
    });
    let key = crate::content_cache::content_cache_key("project-linked-sema", &material);
    let marker = opts
        .root
        .join(".ncc")
        .join("incremental")
        .join("linked-sema")
        .join(format!("{key}.json"));
    let incremental_enabled = std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_none();
    if incremental_enabled && marker.is_file() {
        if let Ok(raw) = fs::read_to_string(&marker) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                if value.get("schema").and_then(|item| item.as_str())
                    == Some("ncc.project.linked_sema.v1")
                    && value.get("status").and_then(|item| item.as_str()) == Some("ok")
                    && value.get("key").and_then(|item| item.as_str()) == Some(key.as_str())
                {
                    if let Some(warnings) = value.get("warnings").and_then(|item| item.as_array()) {
                        for warning in warnings.iter().filter_map(|item| item.as_str()) {
                            eprintln!("warning: {warning}");
                        }
                    }
                    return Ok("validated_linked_sema_cache_hit".to_string());
                }
            }
        }
    }

    let sema = ncc_sema::analyze(linked_program)
        .map_err(|err| format!("E_PROJECT_LINK_SEMA_001: {err}"))?;
    for warning in &sema.warnings {
        eprintln!("warning: {warning}");
    }
    if incremental_enabled {
        write_json(
            &marker,
            &json!({
                "schema": "ncc.project.linked_sema.v1",
                "status": "ok",
                "key": key,
                "material": material,
                "warnings": sema.warnings,
            }),
        )?;
    }
    Ok(if incremental_enabled {
        "validated_linked_sema_cache_miss".to_string()
    } else {
        "validated_linked_sema_cache_disabled".to_string()
    })
}

fn load_bridge_manifest(
    opts: &ProjectCommandOptions,
) -> Result<ProjectBridgeManifest, Box<dyn Error>> {
    load_bridge_manifest_with_object_validation(opts, true)
}

fn load_bridge_manifest_for_runtime(
    opts: &ProjectCommandOptions,
) -> Result<ProjectBridgeManifest, Box<dyn Error>> {
    load_bridge_manifest_with_object_validation(opts, false)
}

fn load_bridge_manifest_with_object_validation(
    opts: &ProjectCommandOptions,
    validate_objects: bool,
) -> Result<ProjectBridgeManifest, Box<dyn Error>> {
    let bridge_path = opts.root.join("bridge.json");
    if !bridge_path.is_file() {
        return Err(format!(
            "E_PROJECT_BRIDGE_MISSING_001: missing bridge.json in '{}'; run `ncc project sync {}` first",
            slash_path(&opts.root),
            slash_path(&opts.root)
        )
        .into());
    }
    let bridge_raw = fs::read_to_string(&bridge_path)?;
    let bridge: serde_json::Value = serde_json::from_str(&bridge_raw)?;
    if bridge.get("schema").and_then(|v| v.as_str()) != Some("ncc.bridge.v1") {
        return Err(
            "E_PROJECT_BRIDGE_SCHEMA_001: bridge.json must use schema ncc.bridge.v1".into(),
        );
    }
    let main_name = bridge
        .get("main")
        .and_then(|v| v.as_str())
        .unwrap_or(&opts.main_name);

    let mut units = Vec::new();
    let mut seen_main = false;
    let raw_units = bridge
        .get("units")
        .and_then(|v| v.as_array())
        .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge.json requires units array")?;
    for raw in raw_units {
        let name = raw
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge unit requires name")?;
        let source = raw
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_001: bridge unit requires source")?;
        let source_hash = raw
            .get("source_hash")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_UNIT_004: bridge unit requires source_hash")?;
        if !source_hash.starts_with("sha256:") {
            return Err(format!(
                "E_PROJECT_BRIDGE_UNIT_004: unit '{}' source_hash must use sha256:",
                name
            )
            .into());
        }
        let object_relative = raw
            .get("object")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_OBJECT_001: bridge unit requires object path")?;
        validate_project_object_relative_path(name, object_relative)?;
        let object_hash = raw
            .get("object_hash")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_OBJECT_HASH_001: bridge unit requires object_hash")?;
        if !object_hash.starts_with("sha256:") {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_HASH_001: unit '{}' object_hash must use sha256:",
                name
            )
            .into());
        }
        let symbols = read_bridge_symbols(raw, name)?;
        let imports = read_bridge_string_array(raw, "imports", name)?;
        let deps = read_bridge_string_array(raw, "deps", name)?;
        let is_main = name.eq_ignore_ascii_case("main");
        if is_main {
            seen_main = true;
        }
        units.push(ProjectBridgeUnit {
            name: name.to_string(),
            source: opts.root.join(source),
            relative_source: source.to_string(),
            code_relative: raw
                .get("code")
                .and_then(|v| v.as_str())
                .map(|value| value.to_string()),
            source_hash: source_hash.to_string(),
            object_relative: object_relative.to_string(),
            object_hash: object_hash.to_string(),
            symbols,
            imports,
            deps,
            is_main,
        });
    }
    if !seen_main {
        return Err(format!(
            "E_PROJECT_BRIDGE_UNIT_002: bridge.json must include physical main unit '{}'",
            main_name
        )
        .into());
    }

    let mut known = BTreeSet::new();
    for unit in &units {
        let key = unit.name.to_ascii_lowercase();
        if !known.insert(key) {
            return Err(format!(
                "E_PROJECT_BRIDGE_UNIT_DUP_001: duplicate bridge unit '{}'",
                unit.name
            )
            .into());
        }
    }

    let mut edges = Vec::new();
    if let Some(raw_edges) = bridge.get("edges").and_then(|v| v.as_array()) {
        for raw in raw_edges {
            let Some(pair) = raw.as_array() else {
                return Err("E_PROJECT_BRIDGE_EDGE_001: bridge edge must be [from, to]".into());
            };
            if pair.len() != 2 {
                return Err("E_PROJECT_BRIDGE_EDGE_001: bridge edge must be [from, to]".into());
            }
            let from = pair[0]
                .as_str()
                .ok_or("E_PROJECT_BRIDGE_EDGE_001: bridge edge from must be string")?;
            let to = pair[1]
                .as_str()
                .ok_or("E_PROJECT_BRIDGE_EDGE_001: bridge edge to must be string")?;
            if !known.contains(&from.to_ascii_lowercase()) {
                return Err(format!(
                    "E_PROJECT_BRIDGE_EDGE_002: bridge edge source '{}' is not a known unit",
                    from
                )
                .into());
            }
            if !known.contains(&to.to_ascii_lowercase()) {
                return Err(format!(
                    "E_PROJECT_BRIDGE_EDGE_003: bridge edge target '{}' is not a known unit",
                    to
                )
                .into());
            }
            edges.push((from.to_string(), to.to_string()));
        }
    }

    validate_project_bridge_imports_and_deps(&units, &edges)?;
    if validate_objects {
        validate_project_bridge_objects(&opts.root, &units)?;
    }
    let link_order = read_project_link_order(&bridge, &known)?;

    Ok(ProjectBridgeManifest {
        units,
        edges,
        link_order,
    })
}

fn read_bridge_symbols(
    raw: &serde_json::Value,
    unit_name: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let raw_symbols = raw
        .get("symbols")
        .and_then(|v| v.as_array())
        .ok_or("E_PROJECT_BRIDGE_SYMBOLS_001: bridge unit requires symbols array")?;
    let mut symbols = Vec::with_capacity(raw_symbols.len());
    let mut seen = BTreeSet::new();
    for symbol in raw_symbols {
        let name = symbol
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_SYMBOLS_001: symbol requires name")?;
        let kind = symbol
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or("E_PROJECT_BRIDGE_SYMBOLS_001: symbol requires kind")?;
        let key = format!("{}:{name}", kind.to_ascii_lowercase());
        if !seen.insert(key) {
            return Err(format!(
                "E_PROJECT_BRIDGE_SYMBOLS_002: duplicate symbol '{}' in unit '{}'",
                name, unit_name
            )
            .into());
        }
        symbols.push(symbol.clone());
    }
    Ok(symbols)
}

fn read_bridge_string_array(
    raw: &serde_json::Value,
    field: &str,
    unit_name: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let values = raw.get(field).and_then(|v| v.as_array()).ok_or_else(|| {
        format!("E_PROJECT_BRIDGE_UNIT_003: unit '{unit_name}' requires {field} array")
    })?;
    let mut out = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for value in values {
        let Some(text) = value.as_str() else {
            return Err(format!(
                "E_PROJECT_BRIDGE_UNIT_003: unit '{}' field '{}' must contain strings",
                unit_name, field
            )
            .into());
        };
        let key = text.to_ascii_lowercase();
        if !seen.insert(key) {
            return Err(format!(
                "E_PROJECT_BRIDGE_UNIT_003: duplicate '{}' entry '{}' in unit '{}'",
                field, text, unit_name
            )
            .into());
        }
        out.push(text.to_string());
    }
    Ok(out)
}

fn validate_project_object_relative_path(
    unit_name: &str,
    object_relative: &str,
) -> Result<(), Box<dyn Error>> {
    let normalized = object_relative.replace('\\', "/");
    if normalized.contains(':')
        || normalized.starts_with('/')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "..")
    {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_PATH_001: unit '{}' object path '{}' must be a safe relative path",
            unit_name, object_relative
        )
        .into());
    }
    for component in Path::new(&normalized).components() {
        if matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        ) {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_PATH_001: unit '{}' object path '{}' must stay relative",
                unit_name, object_relative
            )
            .into());
        }
    }
    if !normalized.starts_with(".ncc/obj/") {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_PATH_002: unit '{}' object path '{}' must live under .ncc/obj",
            unit_name, object_relative
        )
        .into());
    }
    let extension = Path::new(&normalized)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if !extension.eq_ignore_ascii_case("o") && !extension.eq_ignore_ascii_case("obj") {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_PATH_003: unit '{}' object path '{}' must end in .o or .obj",
            unit_name, object_relative
        )
        .into());
    }
    Ok(())
}

fn validate_project_bridge_imports_and_deps(
    units: &[ProjectBridgeUnit],
    edges: &[(String, String)],
) -> Result<(), Box<dyn Error>> {
    let mut edge_deps = BTreeMap::<String, BTreeSet<String>>::new();
    for (from, to) in edges {
        edge_deps
            .entry(from.to_ascii_lowercase())
            .or_default()
            .insert(to.to_ascii_lowercase());
    }
    for unit in units {
        let key = unit.name.to_ascii_lowercase();
        let expected = edge_deps.remove(&key).unwrap_or_default();
        let imports = string_set(&unit.imports);
        if imports != expected {
            return Err(format!(
                "E_PROJECT_BRIDGE_IMPORTS_001: unit '{}' imports must match bridge edges",
                unit.name
            )
            .into());
        }
        let deps = string_set(&unit.deps);
        if deps != expected {
            return Err(format!(
                "E_PROJECT_BRIDGE_DEPS_001: unit '{}' deps must match bridge edges",
                unit.name
            )
            .into());
        }
    }
    if let Some((from, _)) = edge_deps.into_iter().next() {
        return Err(format!(
            "E_PROJECT_BRIDGE_EDGE_002: bridge edge source '{}' is not represented by a unit",
            from
        )
        .into());
    }
    Ok(())
}

fn validate_project_bridge_objects(
    root: &Path,
    units: &[ProjectBridgeUnit],
) -> Result<(), Box<dyn Error>> {
    for unit in units {
        let object_path = root.join(&unit.object_relative);
        if !object_path.is_file() {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_MISSING_001: object '{}' for unit '{}' is missing",
                slash_path(&object_path),
                unit.name
            )
            .into());
        }
        let bytes = fs::read(&object_path)?;
        let actual_hash = sha256_bytes(&bytes);
        if actual_hash != unit.object_hash {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_HASH_002: object '{}' for unit '{}' hash mismatch: expected '{}', got '{}'",
                slash_path(&object_path),
                unit.name,
                unit.object_hash,
                actual_hash
            )
            .into());
        }
        let metadata_path = root.join(format!("{}.json", unit.object_relative));
        let object: serde_json::Value = if metadata_path.is_file() {
            let raw = fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&raw)?
        } else {
            serde_json::from_slice(&bytes)?
        };
        validate_project_object_payload(unit, &object)?;
    }
    Ok(())
}

fn validate_project_object_payload(
    unit: &ProjectBridgeUnit,
    object: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    for (field, expected) in [
        ("schema", "ncc.project.object.v1"),
        ("unit", unit.name.as_str()),
        ("source", unit.relative_source.as_str()),
        ("source_hash", unit.source_hash.as_str()),
    ] {
        let actual = object.get(field).and_then(|v| v.as_str());
        if actual != Some(expected) {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_IDENTITY_001: unit '{}' object field '{}' mismatch: expected '{}', got '{:?}'",
                unit.name, field, expected, actual
            )
            .into());
        }
    }
    let actual_object = object
        .get("object")
        .or_else(|| object.get("artifact"))
        .and_then(|v| v.as_str());
    if actual_object != Some(unit.object_relative.as_str()) {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_IDENTITY_001: unit '{}' object field 'object/artifact' mismatch",
            unit.name
        )
        .into());
    }
    if let Some(bridge) = object.get("bridge").and_then(|v| v.as_str()) {
        if bridge != "../../bridge.json" {
            return Err(format!(
                "E_PROJECT_BRIDGE_OBJECT_IDENTITY_001: unit '{}' object bridge mismatch",
                unit.name
            )
            .into());
        }
    }
    let actual_code = object.get("code").and_then(|v| v.as_str());
    if actual_code != unit.code_relative.as_deref() {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_IDENTITY_001: unit '{}' object field 'code' mismatch",
            unit.name
        )
        .into());
    }
    let object_symbols = object.get("bridge_symbols").or_else(|| {
        object
            .get("symbols")
            .filter(|value| matches!(value, serde_json::Value::Array(_)))
    });
    if object_symbols != Some(&serde_json::Value::Array(unit.symbols.clone())) {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_IDENTITY_002: unit '{}' object symbols mismatch",
            unit.name
        )
        .into());
    }
    let object_imports = object
        .get("bridge_imports")
        .or_else(|| object.get("imports").filter(|value| value.is_array()))
        .or_else(|| object.pointer("/imports/project_units"));
    if object_imports != Some(&json!(&unit.imports)) {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_IDENTITY_003: unit '{}' object imports mismatch",
            unit.name
        )
        .into());
    }
    let object_deps = object.get("bridge_deps").or_else(|| object.get("deps"));
    if object_deps != Some(&json!(&unit.deps)) {
        return Err(format!(
            "E_PROJECT_BRIDGE_OBJECT_IDENTITY_004: unit '{}' object deps mismatch",
            unit.name
        )
        .into());
    }
    Ok(())
}

fn read_project_link_order(
    bridge: &serde_json::Value,
    known: &BTreeSet<String>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let raw_order = bridge
        .get("link_order")
        .and_then(|v| v.as_array())
        .ok_or("E_PROJECT_BRIDGE_LINK_ORDER_001: bridge.json requires link_order array")?;
    let mut order = Vec::with_capacity(raw_order.len());
    let mut seen = BTreeSet::new();
    for raw in raw_order {
        let unit = raw
            .as_str()
            .ok_or("E_PROJECT_BRIDGE_LINK_ORDER_001: link_order entries must be strings")?;
        let key = unit.to_ascii_lowercase();
        if !known.contains(&key) {
            return Err(format!(
                "E_PROJECT_BRIDGE_LINK_ORDER_002: link_order references unknown unit '{}'",
                unit
            )
            .into());
        }
        if !seen.insert(key) {
            return Err(format!(
                "E_PROJECT_BRIDGE_LINK_ORDER_003: duplicate link_order unit '{}'",
                unit
            )
            .into());
        }
        order.push(unit.to_string());
    }
    if seen.len() != known.len() {
        let missing = known
            .iter()
            .find(|unit| !seen.contains(*unit))
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_string());
        return Err(format!(
            "E_PROJECT_BRIDGE_LINK_ORDER_005: link_order is missing unit '{}'",
            missing
        )
        .into());
    }
    Ok(order)
}

fn string_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn compile_project_unit_ast_only(
    root: &Path,
    unit: ProjectBridgeUnit,
) -> Result<ProjectCompiledUnit, Box<dyn Error>> {
    let (unit, source_text, source_bytes, hash, hash_bytes, ast, token_count, ast_cache_status) =
        read_project_unit_source_and_ast(root, unit)?;
    Ok(ProjectCompiledUnit {
        container_count: ast.containers.len(),
        import_count: count_project_ast_imports(&ast),
        unit,
        source_text,
        source_bytes,
        hash,
        hash_bytes,
        ast,
        ir_debug: "skipped_runtime_fast_clean_cache".to_string(),
        backend_debug: "skipped_runtime_fast_clean_cache".to_string(),
        object_artifact: None,
        token_count,
        ast_cache_status,
    })
}

fn compile_project_unit(
    root: &Path,
    unit: ProjectBridgeUnit,
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> Result<ProjectCompiledUnit, Box<dyn Error>> {
    let (unit, source_text, source_bytes, hash, hash_bytes, ast, token_count, ast_cache_status) =
        read_project_unit_source_and_ast(root, unit)?;
    let ir = ncc_ir::lower_with_backend_profile(ast.clone(), profile.clone());
    ncc_ir::verify_type_contract_lowering(&ast, &ir)
        .map_err(|err| format!("E_PROJECT_UNIT_IR_001: unit '{}': {err}", unit.name))?;
    let backend = ncc_ir::emit_backend(&ir);
    ncc_ir::verify_type_contract_backend_enforcement(&ir, &backend)
        .map_err(|err| format!("E_PROJECT_UNIT_BACKEND_001: unit '{}': {err}", unit.name))?;

    Ok(ProjectCompiledUnit {
        container_count: ast.containers.len(),
        import_count: count_project_ast_imports(&ast),
        unit,
        source_text,
        source_bytes,
        hash,
        hash_bytes,
        ast,
        ir_debug: format!("{ir:#?}"),
        backend_debug: format!("{backend:#?}"),
        object_artifact: None,
        token_count,
        ast_cache_status,
    })
}

fn read_project_unit_source_and_ast(
    root: &Path,
    unit: ProjectBridgeUnit,
) -> Result<
    (
        ProjectBridgeUnit,
        String,
        Vec<u8>,
        String,
        Vec<u8>,
        ncc_ast::Program,
        usize,
        String,
    ),
    Box<dyn Error>,
> {
    if !unit.source.is_file() {
        return Err(format!(
            "E_PROJECT_SOURCE_MISSING_001: source '{}' for unit '{}' does not exist",
            slash_path(&unit.source),
            unit.name
        )
        .into());
    }
    let source_bytes = fs::read(&unit.source)?;
    let source_text = String::from_utf8(source_bytes.clone()).map_err(|err| {
        format!(
            "E_PROJECT_SOURCE_UTF8_001: source '{}' is not valid UTF-8: {err}",
            slash_path(&unit.source)
        )
    })?;
    let hash_bytes = sha256_digest_bytes(&source_bytes);
    let hash = format!("sha256:{}", bytes_to_hex_joined(&hash_bytes, ""));

    if let Some(code_relative) = &unit.code_relative {
        verify_code_json(root, &unit, code_relative, &hash)?;
    } else if !unit.is_main {
        return Err(format!(
            "E_PROJECT_CODE_IDENTITY_001: non-main unit '{}' requires .ncc/code identity",
            unit.name
        )
        .into());
    }
    if unit.source_hash != hash {
        return Err(format!(
            "E_PROJECT_BRIDGE_SOURCE_HASH_001: unit '{}' bridge source_hash mismatch: expected '{}', got '{}'",
            unit.name, hash, unit.source_hash
        )
        .into());
    }

    if let Some((ast, token_count)) = read_project_ast_cache(root, &unit, &hash, &source_bytes)? {
        return Ok((
            unit,
            source_text,
            source_bytes,
            hash,
            hash_bytes,
            ast,
            token_count,
            "hit".to_string(),
        ));
    }

    let tokens = ncc_lexer::lex(&source_text)?;
    let token_count = tokens.len();
    let ast = ncc_parser::parse(tokens)?;
    write_project_ast_cache(
        root,
        &unit,
        &hash,
        &hash_bytes,
        &source_bytes,
        &ast,
        token_count,
    )?;
    Ok((
        unit,
        source_text,
        source_bytes,
        hash,
        hash_bytes,
        ast,
        token_count,
        "miss_written".to_string(),
    ))
}

fn project_ast_cache_path(root: &Path, unit: &ProjectBridgeUnit, hash: &str) -> PathBuf {
    let short_hash = hash
        .strip_prefix("sha256:")
        .unwrap_or(hash)
        .chars()
        .take(16)
        .collect::<String>();
    root.join(".ncc").join("cache").join("ast").join(format!(
        "{}-{}.ast.json",
        sanitize_unit_symbol(&unit.name),
        short_hash
    ))
}

fn read_project_ast_cache(
    root: &Path,
    unit: &ProjectBridgeUnit,
    hash: &str,
    source_bytes: &[u8],
) -> Result<Option<(ncc_ast::Program, usize)>, Box<dyn Error>> {
    let path = project_ast_cache_path(root, unit, hash);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let schema_ok =
        value.get("schema").and_then(|v| v.as_str()) == Some("ncc.project.ast_cache.v1");
    let unit_ok = value.get("unit").and_then(|v| v.as_str()) == Some(unit.name.as_str());
    let source_ok =
        value.get("source").and_then(|v| v.as_str()) == Some(unit.relative_source.as_str());
    let hash_ok = value.get("hash").and_then(|v| v.as_str()) == Some(hash);
    let bytes_ok =
        value.get("source_byte_count").and_then(|v| v.as_u64()) == Some(source_bytes.len() as u64);
    if !(schema_ok && unit_ok && source_ok && hash_ok && bytes_ok) {
        return Ok(None);
    }
    let token_count = value
        .get("tokens")
        .and_then(|v| v.as_u64())
        .map(|value| value as usize)
        .unwrap_or(0);
    let Some(ast_value) = value.get("ast") else {
        return Ok(None);
    };
    let ast = match serde_json::from_value::<ncc_ast::Program>(ast_value.clone()) {
        Ok(ast) => ast,
        Err(_) => return Ok(None),
    };
    Ok(Some((ast, token_count)))
}

fn write_project_ast_cache(
    root: &Path,
    unit: &ProjectBridgeUnit,
    hash: &str,
    hash_bytes: &[u8],
    source_bytes: &[u8],
    ast: &ncc_ast::Program,
    token_count: usize,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing AST cache")?;
    let path = project_ast_cache_path(root, unit, hash);
    let payload = json!({
        "schema": "ncc.project.ast_cache.v1",
        "unit": unit.name,
        "source": unit.relative_source,
        "hash": hash,
        "hash_digest_bytes_hex": bytes_to_hex_vec(hash_bytes),
        "source_byte_count": source_bytes.len(),
        "tokens": token_count,
        "containers": ast.containers.len(),
        "imports": count_project_ast_imports(ast),
        "ast": ast
    });
    write_json(&path, &payload)
}

fn verify_code_json(
    root: &Path,
    unit: &ProjectBridgeUnit,
    code_relative: &str,
    expected_hash: &str,
) -> Result<(), Box<dyn Error>> {
    let code_path = root.join(code_relative);
    if !code_path.is_file() {
        return Err(format!(
            "E_PROJECT_CODE_MISSING_001: missing code identity '{}' for unit '{}'",
            slash_path(&code_path),
            unit.name
        )
        .into());
    }
    let raw = fs::read_to_string(&code_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    for (field, expected) in [
        ("schema", "ncc.code.v1"),
        ("unit", unit.name.as_str()),
        ("source", unit.relative_source.as_str()),
        ("hash", expected_hash),
        ("bridge", "../../bridge.json"),
    ] {
        let actual = value.get(field).and_then(|v| v.as_str());
        if actual != Some(expected) {
            return Err(format!(
                "E_PROJECT_CODE_IDENTITY_002: unit '{}' code identity field '{}' mismatch: expected '{}', got '{:?}'",
                unit.name, field, expected, actual
            )
            .into());
        }
    }
    Ok(())
}

fn count_project_ast_imports(ast: &ncc_ast::Program) -> usize {
    let section_uses = ast
        .containers
        .iter()
        .flat_map(|container| container.sections.iter())
        .filter_map(|section| match section {
            ncc_ast::Section::Use(uses) => Some(uses.len()),
            _ => None,
        })
        .sum::<usize>();
    ast.uses.len() + section_uses
}

fn collect_project_imports_from_program(program: &ncc_ast::Program) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for use_decl in &program.uses {
        if let Some(target) = project_import_target(&use_decl.path) {
            imports.insert(target);
        }
    }
    for container in &program.containers {
        for section in &container.sections {
            if let ncc_ast::Section::Use(use_decls) = section {
                for use_decl in use_decls {
                    if let Some(target) = project_import_target(&use_decl.path) {
                        imports.insert(target);
                    }
                }
            }
        }
    }
    imports
}

fn collect_project_symbols_from_program(program: &ncc_ast::Program) -> Vec<serde_json::Value> {
    let mut symbols = Vec::new();
    for container in &program.containers {
        symbols.push(project_symbol(
            &container.name,
            &container.name,
            "container",
            &container.visibility,
        ));
        for section in &container.sections {
            collect_project_section_symbols(&container.name, section, &mut symbols);
        }
    }
    symbols
}

fn collect_project_section_symbols(
    container: &str,
    section: &ncc_ast::Section,
    symbols: &mut Vec<serde_json::Value>,
) {
    match section {
        ncc_ast::Section::Stc(decls) => {
            for decl in decls {
                let ncc_ast::StcDecl::Typed {
                    visibility, name, ..
                } = decl;
                symbols.push(project_member_symbol(container, name, "state", visibility));
            }
        }
        ncc_ast::Section::Local(local) => {
            for decl in &local.decls {
                match decl {
                    ncc_ast::LocalDecl::Var {
                        visibility, name, ..
                    } => symbols.push(project_member_symbol(
                        container,
                        name,
                        "local_var",
                        visibility,
                    )),
                    ncc_ast::LocalDecl::Const {
                        visibility, name, ..
                    } => symbols.push(project_member_symbol(
                        container,
                        name,
                        "local_const",
                        visibility,
                    )),
                    ncc_ast::LocalDecl::TypeAlias {
                        visibility, name, ..
                    } => symbols.push(project_member_symbol(
                        container,
                        name,
                        "type_alias",
                        visibility,
                    )),
                    ncc_ast::LocalDecl::TypeRecord {
                        visibility, name, ..
                    } => symbols.push(project_member_symbol(
                        container,
                        name,
                        "type_record",
                        visibility,
                    )),
                    ncc_ast::LocalDecl::Metadata {
                        visibility, name, ..
                    } => symbols.push(project_member_symbol(
                        container, name, "metadata", visibility,
                    )),
                }
            }
        }
        ncc_ast::Section::ConstEval(decls) | ncc_ast::Section::Comptime(decls) => {
            for decl in decls {
                symbols.push(project_member_symbol(
                    container,
                    &decl.name,
                    "const_eval",
                    &decl.visibility,
                ));
            }
        }
        ncc_ast::Section::ConstFn(functions) => {
            collect_project_function_symbols(container, functions, "const_function", symbols);
        }
        ncc_ast::Section::Fn(functions) => {
            collect_project_function_symbols(container, functions, "function", symbols);
        }
        ncc_ast::Section::MoveFn(functions) => {
            collect_project_function_symbols(container, functions, "move_function", symbols);
        }
        ncc_ast::Section::Interfaces(items) => {
            for item in items {
                symbols.push(project_member_symbol(
                    container,
                    &item.name,
                    "interface",
                    &None,
                ));
            }
        }
        ncc_ast::Section::Traits(items) => {
            for item in items {
                symbols.push(project_member_symbol(container, &item.name, "trait", &None));
                collect_project_function_symbols(
                    container,
                    &item.default_methods,
                    "trait_default_function",
                    symbols,
                );
            }
        }
        ncc_ast::Section::ImplBlocks(items) => {
            for item in items {
                collect_project_function_symbols(
                    container,
                    &item.methods,
                    "impl_function",
                    symbols,
                );
            }
        }
        ncc_ast::Section::Contracts(items) => {
            for item in items {
                symbols.push(project_member_symbol(
                    container, &item.name, "contract", &None,
                ));
            }
        }
        ncc_ast::Section::Enums(items) => {
            for item in items {
                symbols.push(project_member_symbol(container, &item.name, "enum", &None));
                for variant in &item.variants {
                    let variant_name = format!("{}.{}", item.name, variant.name);
                    symbols.push(project_member_symbol(
                        container,
                        &variant_name,
                        "enum_variant",
                        &None,
                    ));
                }
            }
        }
        ncc_ast::Section::Foreign(items) => {
            for item in items {
                symbols.push(project_member_symbol(
                    container,
                    &item.symbol,
                    "foreign",
                    &None,
                ));
            }
        }
        ncc_ast::Section::TypeContracts(items) => {
            for item in items {
                symbols.push(project_member_symbol(
                    container,
                    &item.name,
                    "type_contract",
                    &None,
                ));
            }
        }
        ncc_ast::Section::MainEntry { .. } => {
            symbols.push(project_member_symbol(
                container,
                "__main",
                "main_entry",
                &None,
            ));
        }
        _ => {}
    }
}

fn collect_project_function_symbols(
    container: &str,
    functions: &[ncc_ast::FunctionDecl],
    kind: &str,
    symbols: &mut Vec<serde_json::Value>,
) {
    for function in functions {
        symbols.push(project_member_symbol(
            container,
            &function.name,
            kind,
            &function.visibility,
        ));
    }
}

fn project_member_symbol(
    container: &str,
    member: &str,
    kind: &str,
    visibility: &Option<ncc_ast::Visibility>,
) -> serde_json::Value {
    project_symbol(
        container,
        &format!("{container}.{member}"),
        kind,
        visibility,
    )
}

fn project_symbol(
    container: &str,
    name: &str,
    kind: &str,
    visibility: &Option<ncc_ast::Visibility>,
) -> serde_json::Value {
    json!({
        "name": name,
        "kind": kind,
        "container": container,
        "visibility": project_visibility_label(visibility)
    })
}

fn project_visibility_label(visibility: &Option<ncc_ast::Visibility>) -> &'static str {
    match visibility {
        Some(ncc_ast::Visibility::Public) => "public",
        Some(ncc_ast::Visibility::Private) => "private",
        Some(ncc_ast::Visibility::Internal) => "internal",
        Some(ncc_ast::Visibility::Strictly) => "strictly",
        None => "unspecified",
    }
}

fn link_project_program(units: &[ProjectCompiledUnit]) -> Result<ncc_ast::Program, Box<dyn Error>> {
    let mut linked = ncc_ast::Program {
        modules: Vec::new(),
        packages: Vec::new(),
        uses: Vec::new(),
        containers: Vec::new(),
    };
    let mut container_counts = BTreeMap::<String, usize>::new();
    for unit in units {
        for container in &unit.ast.containers {
            *container_counts
                .entry(container.name.to_ascii_lowercase())
                .or_default() += 1;
        }
    }
    let materialized_names = project_container_materialized_names(units, &container_counts);
    let known_units = units
        .iter()
        .map(|unit| unit.unit.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    for unit in units {
        let ast = project_materialized_unit_program(
            unit,
            &container_counts,
            &materialized_names,
            &known_units,
        )?;
        linked.modules.extend(ast.modules);
        linked.packages.extend(ast.packages);
        linked.uses.extend(ast.uses);
        linked.containers.extend(ast.containers);
    }

    Ok(linked)
}

fn project_materialized_unit_program(
    unit: &ProjectCompiledUnit,
    container_counts: &BTreeMap<String, usize>,
    materialized_names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
) -> Result<ncc_ast::Program, Box<dyn Error>> {
    project_materialized_source_program(
        &unit.unit.name,
        unit.unit.is_main,
        &unit.ast,
        container_counts,
        materialized_names,
        known_units,
    )
}

fn materialize_project_objects(
    root: &Path,
    units: &mut [ProjectCompiledUnit],
    profile: &ncc_ir::BackendLoweringProfileIR,
    allow_deferred: bool,
) -> Result<(), Box<dyn Error>> {
    let mut container_counts = BTreeMap::<String, usize>::new();
    for unit in units.iter() {
        for container in &unit.ast.containers {
            *container_counts
                .entry(container.name.to_ascii_lowercase())
                .or_default() += 1;
        }
    }
    let materialized_names = project_container_materialized_names(units, &container_counts);
    let known_units = units
        .iter()
        .map(|unit| unit.unit.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let materialized_asts = units
        .iter()
        .map(|unit| {
            project_materialized_unit_program(
                unit,
                &container_counts,
                &materialized_names,
                &known_units,
            )
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let runtime_providers =
        project_nc_runtime_provider_program(&profile.target_spec.target_triple)?;
    let mut signature_asts = materialized_asts.clone();
    signature_asts.push(runtime_providers);
    let project_symbols = project_foreign_symbol_catalog(&signature_asts);

    for (unit, ast) in units.iter_mut().zip(materialized_asts) {
        match build_project_object_artifact(root, &unit.unit.name, &ast, profile, &project_symbols)
        {
            Ok(artifact) => unit.object_artifact = Some(artifact),
            Err(err)
                if allow_deferred
                    && (err.to_string().contains("has no native bytes")
                        || err
                            .to_string()
                            .contains("refusing to materialize backend text as machine code")) =>
            {
                unit.object_artifact = None;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn build_project_object_artifact(
    root: &Path,
    unit_name: &str,
    ast: &ncc_ast::Program,
    profile: &ncc_ir::BackendLoweringProfileIR,
    project_symbols: &[ncc_ir::ForeignSymbolIR],
) -> Result<ProjectObjectArtifact, Box<dyn Error>> {
    let ast_bytes = serde_json::to_vec(ast)?;
    let cache_material = json!({
        "schema": "ncc.project.unit_object_content_key.v1",
        "unit": unit_name,
        "ast_sha256": crate::content_cache::sha256_bytes(&ast_bytes),
        "project_symbol_abi_sha256": crate::content_cache::sha256_bytes(
            format!("{project_symbols:#?}").as_bytes()
        ),
        "target": project_profile_payload(profile),
        "compiler": crate::content_cache::compiler_image_identity(),
        "object_contract": "typed-backend-object-v3-unified-mct-abi-no-debug"
    });
    let cache_key = crate::content_cache::content_cache_key("project-unit-object", &cache_material);
    let extension = project_object_extension(&profile.target_spec.object_format).to_string();
    let target_dir = project_target_object_dir(&profile.target_spec);
    let object_relative = format!("{target_dir}/{unit_name}.{extension}");
    let metadata_relative = format!("{object_relative}.json");
    let object_path = root.join(&object_relative);
    let backend_object_path = root
        .join(".ncc")
        .join("incremental")
        .join("project-objects")
        .join(sanitize_unit_symbol(&profile.target_spec.target_triple))
        .join(format!("{unit_name}.backend-object.json"));
    let content_cache_root = crate::content_cache::content_cache_root(None, &root.join(".ncc"));
    let incremental_enabled = std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_none();
    if incremental_enabled {
        let destinations = BTreeMap::from([
            ("object".to_string(), object_path.clone()),
            ("backend_object_ir".to_string(), backend_object_path.clone()),
        ]);
        if let Ok(Some(_)) = crate::content_cache::restore_content_cache_entry(
            &content_cache_root,
            "project-unit-object",
            &cache_key,
            &destinations,
        ) {
            if let (Ok(bytes), Ok(serialized)) =
                (fs::read(&object_path), fs::read(&backend_object_path))
            {
                if let Ok(object) = serde_json::from_slice::<ncc_ir::BackendObjectIR>(&serialized) {
                    if !bytes.is_empty() && object.target_spec == profile.target_spec {
                        let hash_bytes = sha256_digest_bytes(&bytes);
                        return Ok(ProjectObjectArtifact {
                            object_relative,
                            metadata_relative,
                            extension,
                            hash: format!("sha256:{}", bytes_to_hex_joined(&hash_bytes, "")),
                            hash_bytes,
                            text: ncc_ir::render_object_text(&object),
                            object,
                            bytes,
                            cache_status: "hit".to_string(),
                            cache_key,
                        });
                    }
                }
            }
        }
    }

    let local_symbols = project_defined_function_symbols(ast);
    let external_symbols = project_symbols
        .iter()
        .filter(|symbol| !local_symbols.contains(&symbol.symbol))
        .cloned()
        .collect::<Vec<_>>();
    let mut ir = ncc_ir::lower_with_backend_profile(ast.clone(), profile.clone());
    for container in &mut ir.containers {
        match container {
            ncc_ir::ContainerIR::Regular(regular) => {
                regular
                    .foreign_symbols
                    .extend(external_symbols.iter().cloned());
            }
            ncc_ir::ContainerIR::Root(root) => {
                root.foreign_symbols
                    .extend(external_symbols.iter().cloned());
            }
            ncc_ir::ContainerIR::Security(_)
            | ncc_ir::ContainerIR::Primitive(_)
            | ncc_ir::ContainerIR::Machine(_) => {}
        }
    }
    ncc_ir::verify_type_contract_lowering(ast, &ir)
        .map_err(|err| format!("E_PROJECT_UNIT_OBJECT_IR_001: unit '{unit_name}': {err}"))?;
    let backend = ncc_ir::emit_backend(&ir);
    ncc_ir::verify_type_contract_backend_enforcement(&ir, &backend)
        .map_err(|err| format!("E_PROJECT_UNIT_OBJECT_BACKEND_001: unit '{unit_name}': {err}"))?;
    let object = ncc_ir::emit_object(&backend);
    let module = project_mct_emit_release_module(&ir, unit_name);
    let bytes = emit_project_mct_object_bytes(unit_name, &module, &object_path).map_err(|err| {
        ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_UNIT_OBJECT_MCT_001,
            format!("unit '{unit_name}' MCT emission failed ({err})"),
        )
    })?;
    let hash_bytes = sha256_digest_bytes(&bytes);
    let hash = format!("sha256:{}", bytes_to_hex_joined(&hash_bytes, ""));
    let text = ncc_ir::render_object_text(&object);

    if incremental_enabled {
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = backend_object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_bytes(&object_path, &bytes)?;
        write_bytes(&backend_object_path, &serde_json::to_vec(&object)?)?;
        let artifacts = [
            crate::content_cache::ContentCacheArtifact::new("object", &object_path),
            crate::content_cache::ContentCacheArtifact::new(
                "backend_object_ir",
                &backend_object_path,
            ),
        ];
        let _ = crate::content_cache::store_content_cache_entry(
            &content_cache_root,
            "project-unit-object",
            &cache_key,
            &cache_material,
            json!({
                "unit": unit_name,
                "object_hash": hash,
                "object_bytes": bytes.len(),
            }),
            &artifacts,
        );
    }

    Ok(ProjectObjectArtifact {
        object_relative,
        metadata_relative,
        extension,
        bytes,
        hash,
        hash_bytes,
        text,
        object,
        cache_status: if incremental_enabled {
            "miss"
        } else {
            "disabled"
        }
        .to_string(),
        cache_key,
    })
}

fn project_defined_function_symbols(ast: &ncc_ast::Program) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    for container in &ast.containers {
        for section in &container.sections {
            let ncc_ast::Section::Fn(functions) = section else {
                continue;
            };
            symbols.extend(
                functions
                    .iter()
                    .map(|function| format!("{}.{}", container.name, function.name)),
            );
        }
    }
    symbols
}

fn project_foreign_symbol_catalog(asts: &[ncc_ast::Program]) -> Vec<ncc_ir::ForeignSymbolIR> {
    let mut symbols = BTreeMap::<String, ncc_ir::ForeignSymbolIR>::new();
    for ast in asts {
        for container in &ast.containers {
            for section in &container.sections {
                let ncc_ast::Section::Fn(functions) = section else {
                    continue;
                };
                for function in functions {
                    let symbol = format!("{}.{}", container.name, function.name);
                    symbols
                        .entry(symbol.clone())
                        .or_insert_with(|| ncc_ir::ForeignSymbolIR {
                            language: "nC".to_string(),
                            symbol,
                            abi: "nC".to_string(),
                            linkage: "external".to_string(),
                            parameter_types: function
                                .params
                                .iter()
                                .map(|param| project_type_name(&param.ty))
                                .collect(),
                            return_type: project_type_name(&function.return_type),
                        });
                }
            }
        }
    }
    symbols.into_values().collect()
}

pub(crate) fn project_nc_runtime_provider_program(
    target_triple: &str,
) -> Result<ncc_ast::Program, Box<dyn Error>> {
    let mut program = ncc_ast::Program {
        modules: Vec::new(),
        packages: Vec::new(),
        uses: Vec::new(),
        containers: Vec::new(),
    };
    for source in [
        super::compiler::COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_PARSE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_JSON_RUNTIME_SOURCE,
    ] {
        let source = if target_triple.contains("windows") {
            source.to_string()
        } else {
            source.replace("vmCommit(base, total, 4)", "vmCommit(base, total, 3)")
        };
        let mut provider = ncc_parser::parse(ncc_lexer::lex(&source)?)?;
        program.modules.append(&mut provider.modules);
        program.packages.append(&mut provider.packages);
        program.uses.append(&mut provider.uses);
        program.containers.append(&mut provider.containers);
    }
    Ok(program)
}

fn project_type_name(ty: &ncc_ast::TypeName) -> String {
    match ty {
        ncc_ast::TypeName::Simple(name) => name.clone(),
        ncc_ast::TypeName::Vector(element) => format!("vector<{}>", project_type_name(element)),
        ncc_ast::TypeName::Array(element) => format!("array<{}>", project_type_name(element)),
        ncc_ast::TypeName::Union(items) => items
            .iter()
            .map(project_type_name)
            .collect::<Vec<_>>()
            .join(" | "),
        ncc_ast::TypeName::Generic { base, args } => format!(
            "{}<{}>",
            base,
            args.iter()
                .map(project_type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn emit_project_mct_object_bytes(
    unit_name: &str,
    module: &ncc_ir::MctLlvmModuleIR,
    object_path: &Path,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if module.emitted_function_count == 0 || module.rejected_function_count > 0 {
        let rejected = module
            .functions
            .iter()
            .filter(|function| matches!(function.status, ncc_ir::MctLlvmFunctionStatusIR::Rejected))
            .map(|function| {
                format!(
                    "{}:{}:{:?}",
                    function.source_name, function.symbol_name, function.diagnostics
                )
            })
            .collect::<Vec<_>>();
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_UNIT_OBJECT_MCT_001,
            format!(
                "emitted={} rejected={} status={} functions={:?} diagnostics={:?}",
                module.emitted_function_count,
                module.rejected_function_count,
                module.status,
                rejected,
                module.diagnostics
            ),
        )
        .into());
    }
    let clang = project_find_clang(None).ok_or_else(|| {
        format!("unit '{unit_name}' requires clang for per-unit MCT object materialization")
    })?;
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut child = Command::new(&clang)
        .arg(format!("--target={}", module.target_triple))
        .args(["-O3", "-x", "ir", "-c", "-", "-o"])
        .arg(object_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let exported_llvm_ir = if unit_name == "__project_nc_runtime" {
        project_export_nc_runtime_abi(&module.llvm_ir, module)
    } else {
        module.llvm_ir.clone()
    };
    let llvm_ir = project_internalize_mct_runtime_definitions(&exported_llvm_ir);
    child
        .stdin
        .take()
        .ok_or("clang did not expose stdin for the per-unit MCT module")?
        .write_all(llvm_ir.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_UNIT_OBJECT_MCT_001,
            format!(
                "clang='{}' unit='{}' status={} stdout={} stderr={}",
                clang,
                unit_name,
                output.status,
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        )
        .into());
    }
    let bytes = fs::read(object_path)?;
    if bytes.is_empty() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_UNIT_OBJECT_MCT_001,
            format!("clang emitted an empty object for unit '{unit_name}'"),
        )
        .into());
    }
    Ok(bytes)
}

fn project_mct_emit_release_module(
    ir: &ncc_ir::ProgramIR,
    unit_name: &str,
) -> ncc_ir::MctLlvmModuleIR {
    ncc_ir::mct_emit_llvm_module_with_debug_config(
        ir,
        ncc_ir::mct_llvm_pass_pipeline(ncc_ir::MctLlvmOptimizationLevelIR::O3),
        ncc_ir::MctLlvmDebugConfigIR {
            enabled: false,
            source_filename: format!("{unit_name}.n"),
            source_directory: ".".to_string(),
            producer: "ncc project mct release".to_string(),
            dwarf_version: 5,
            debug_info_version: 3,
            emission_kind: "NoDebug",
        },
    )
}

fn project_internalize_mct_runtime_definitions(llvm_ir: &str) -> String {
    let mut out = String::with_capacity(llvm_ir.len() + 1);
    for line in llvm_ir.lines() {
        if line.starts_with("define ")
            && !line.starts_with("define internal ")
            && (line.contains(" @__ncc_") || line.contains(" @ncc_rt_"))
        {
            out.push_str("define internal ");
            out.push_str(&line["define ".len()..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn project_export_nc_runtime_abi(llvm_ir: &str, module: &ncc_ir::MctLlvmModuleIR) -> String {
    let mut out = llvm_ir.to_string();
    for function in &module.functions {
        let Some(name) = function.source_name.strip_prefix("nccJsonRuntime.") else {
            continue;
        };
        if !name.starts_with("json") {
            continue;
        }
        out = out.replace(&format!("@{}(", function.symbol_name), &format!("@{name}("));
    }
    out
}

fn emit_project_object_bytes(
    unit_name: &str,
    object: &ncc_ir::BackendObjectIR,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let result = match object.target_spec.object_format {
        ncc_ir::BackendObjectFormatIR::Elf64 => ncc_ir::emit_object_elf64(object),
        ncc_ir::BackendObjectFormatIR::Coff => ncc_ir::emit_object_coff(object),
        ncc_ir::BackendObjectFormatIR::MachO => ncc_ir::emit_object_macho64(object),
        ncc_ir::BackendObjectFormatIR::FreestandingImage => ncc_ir::emit_image_sovereign(object),
        ncc_ir::BackendObjectFormatIR::Elf32 => Err(
            "project object emission routes ELF32 profiles through machine/clang materialization"
                .to_string(),
        ),
        ncc_ir::BackendObjectFormatIR::WasmModule => Err(
            "project object emission does not yet write per-unit wasm-module artifacts".to_string(),
        ),
    };
    result.map_err(|err| {
        format!(
            "E_PROJECT_UNIT_OBJECT_EMIT_001: unit '{}' object emission failed: {}",
            unit_name, err
        )
        .into()
    })
}

fn project_object_extension(format: &ncc_ir::BackendObjectFormatIR) -> &'static str {
    match format {
        ncc_ir::BackendObjectFormatIR::Coff => "obj",
        ncc_ir::BackendObjectFormatIR::Elf32
        | ncc_ir::BackendObjectFormatIR::Elf64
        | ncc_ir::BackendObjectFormatIR::MachO
        | ncc_ir::BackendObjectFormatIR::FreestandingImage
        | ncc_ir::BackendObjectFormatIR::WasmModule => "o",
    }
}

fn project_target_object_dir(spec: &ncc_ir::BackendTargetSpecIR) -> String {
    format!(".ncc/obj/{}", sanitize_unit_symbol(&spec.target_triple))
}

fn project_container_materialized_names(
    units: &[ProjectCompiledUnit],
    container_counts: &BTreeMap<String, usize>,
) -> BTreeMap<(String, String), String> {
    let mut names = BTreeMap::new();
    for unit in units {
        let unit_key = unit.unit.name.to_ascii_lowercase();
        for container in &unit.ast.containers {
            let container_key = container.name.to_ascii_lowercase();
            let is_duplicate = container_counts
                .get(&container_key)
                .copied()
                .unwrap_or_default()
                > 1;
            let materialized = if !unit.unit.is_main && is_duplicate {
                format!(
                    "{}__{}",
                    sanitize_unit_symbol(&unit.unit.name),
                    container.name
                )
            } else {
                container.name.clone()
            };
            names.insert((unit_key.clone(), container_key), materialized);
        }
    }
    names
}

fn rewrite_project_unit_qualified_program(
    program: &mut ncc_ast::Program,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for container in &mut program.containers {
        for section in &mut container.sections {
            rewrite_project_unit_qualified_section(section, names, known_units, &mut diagnostics);
        }
    }
    diagnostics
}

fn rewrite_project_unit_qualified_section(
    section: &mut ncc_ast::Section,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    match section {
        ncc_ast::Section::Stc(decls) => {
            for decl in decls {
                let ncc_ast::StcDecl::Typed { init, .. } = decl;
                if let Some(expr) = init {
                    rewrite_project_unit_qualified_expr(expr, names, known_units, diagnostics);
                }
            }
        }
        ncc_ast::Section::Local(local) => {
            for decl in &mut local.decls {
                match decl {
                    ncc_ast::LocalDecl::Var { init, .. } => {
                        if let Some(expr) = init {
                            rewrite_project_unit_qualified_expr(
                                expr,
                                names,
                                known_units,
                                diagnostics,
                            );
                        }
                    }
                    ncc_ast::LocalDecl::Const { value, .. }
                    | ncc_ast::LocalDecl::Metadata { value, .. } => {
                        rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
                    }
                    ncc_ast::LocalDecl::TypeAlias { .. }
                    | ncc_ast::LocalDecl::TypeRecord { .. } => {}
                }
            }
        }
        ncc_ast::Section::ConstEval(decls) | ncc_ast::Section::Comptime(decls) => {
            for decl in decls {
                rewrite_project_unit_qualified_expr(
                    &mut decl.value,
                    names,
                    known_units,
                    diagnostics,
                );
            }
        }
        ncc_ast::Section::ConstFn(functions)
        | ncc_ast::Section::Fn(functions)
        | ncc_ast::Section::MoveFn(functions) => {
            for function in functions {
                rewrite_project_unit_qualified_block(
                    &mut function.public_body,
                    names,
                    known_units,
                    diagnostics,
                );
            }
        }
        ncc_ast::Section::ComptimeUnsafe(blocks) => {
            for block in blocks {
                rewrite_project_unit_qualified_block(
                    &mut block.body,
                    names,
                    known_units,
                    diagnostics,
                );
            }
        }
        ncc_ast::Section::Traits(traits) => {
            for trait_decl in traits {
                for function in &mut trait_decl.default_methods {
                    rewrite_project_unit_qualified_block(
                        &mut function.public_body,
                        names,
                        known_units,
                        diagnostics,
                    );
                }
            }
        }
        ncc_ast::Section::ImplBlocks(impls) => {
            for impl_decl in impls {
                for function in &mut impl_decl.methods {
                    rewrite_project_unit_qualified_block(
                        &mut function.public_body,
                        names,
                        known_units,
                        diagnostics,
                    );
                }
            }
        }
        ncc_ast::Section::Contracts(contracts) => {
            for contract in contracts {
                for expr in contract
                    .requires
                    .iter_mut()
                    .chain(contract.ensures.iter_mut())
                    .chain(contract.invariants.iter_mut())
                {
                    rewrite_project_unit_qualified_expr(expr, names, known_units, diagnostics);
                }
            }
        }
        ncc_ast::Section::Probe(stmts)
        | ncc_ast::Section::Structure(stmts)
        | ncc_ast::Section::Final(stmts)
        | ncc_ast::Section::Bind(stmts)
        | ncc_ast::Section::Raw(stmts)
        | ncc_ast::Section::Binary(stmts) => {
            rewrite_project_unit_qualified_block(stmts, names, known_units, diagnostics);
        }
        ncc_ast::Section::MainEntry { kind, body } => {
            rewrite_project_unit_qualified_main_kind(kind, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        _ => {}
    }
}

fn rewrite_project_unit_qualified_block(
    block: &mut [ncc_ast::Stmt],
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    for stmt in block {
        rewrite_project_unit_qualified_stmt(stmt, names, known_units, diagnostics);
    }
}

fn rewrite_project_unit_qualified_stmt(
    stmt: &mut ncc_ast::Stmt,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    match stmt {
        ncc_ast::Stmt::Return(expr) | ncc_ast::Stmt::Expr(expr) => {
            rewrite_project_unit_qualified_expr(expr, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Projection { target, value, .. }
        | ncc_ast::Stmt::StrictCheck { target, value, .. } => {
            rewrite_project_unit_qualified_assign_target(target, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(then_block, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(else_block, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Unless { condition, body }
        | ncc_ast::Stmt::While { condition, body }
        | ncc_ast::Stmt::RepeatUntil { body, condition } => {
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::EachDoWhile {
            iterable,
            body,
            condition,
            ..
        } => {
            rewrite_project_unit_qualified_expr(iterable, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::DoWhile { body, condition } => {
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(init) = init {
                rewrite_project_unit_qualified_stmt(init, names, known_units, diagnostics);
            }
            if let Some(condition) = condition {
                rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            }
            if let Some(step) = step {
                rewrite_project_unit_qualified_stmt(step, names, known_units, diagnostics);
            }
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            rewrite_project_unit_qualified_expr(iterable, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            rewrite_project_unit_qualified_expr(start, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(end, names, known_units, diagnostics);
            if let Some(step) = step {
                rewrite_project_unit_qualified_expr(step, names, known_units, diagnostics);
            }
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::Defer { body } => {
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::RepeatCount { count, body } => {
            rewrite_project_unit_qualified_expr(count, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => {
            rewrite_project_unit_qualified_stmt(stmt, names, known_units, diagnostics);
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
            rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
            for (case, body) in cases {
                rewrite_project_unit_qualified_expr(case, names, known_units, diagnostics);
                rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
            }
            rewrite_project_unit_qualified_block(default, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::PatternMatch { value, arms } => {
            rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    rewrite_project_unit_qualified_expr(guard, names, known_units, diagnostics);
                }
                if let ncc_ast::Pattern::Literal(expr) = &mut arm.pattern {
                    rewrite_project_unit_qualified_expr(expr, names, known_units, diagnostics);
                }
                rewrite_project_unit_qualified_block(
                    &mut arm.body,
                    names,
                    known_units,
                    diagnostics,
                );
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            rewrite_project_unit_qualified_block(else_block, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Select { branches, default } => {
            for (condition, body) in branches {
                rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
                rewrite_project_unit_qualified_block(body, names, known_units, diagnostics);
            }
            rewrite_project_unit_qualified_block(default, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::MainRefFn { fn_ref, .. }
        | ncc_ast::Stmt::FunctionRelation { fn_ref, .. } => {
            rewrite_project_unit_qualified_path(&mut fn_ref.path, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Print { value, .. } => {
            rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
        }
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for arg in args {
                rewrite_project_unit_qualified_expr(arg, names, known_units, diagnostics);
            }
        }
        ncc_ast::Stmt::Dev(_) | ncc_ast::Stmt::Break { .. } | ncc_ast::Stmt::Continue { .. } => {}
    }
}

fn rewrite_project_unit_qualified_expr(
    expr: &mut ncc_ast::Expr,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    match expr {
        ncc_ast::Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
            }
        }
        ncc_ast::Expr::VectorLiteral(items) | ncc_ast::Expr::ArrayLiteral(items) => {
            for item in items {
                rewrite_project_unit_qualified_expr(item, names, known_units, diagnostics);
            }
        }
        ncc_ast::Expr::Index { target, index } => {
            rewrite_project_unit_qualified_expr(target, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(index, names, known_units, diagnostics);
        }
        ncc_ast::Expr::Closure(closure) => {
            rewrite_project_unit_qualified_block(
                &mut closure.body,
                names,
                known_units,
                diagnostics,
            );
        }
        ncc_ast::Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            if let ncc_ast::Expr::ContextRef(context) = &mut **receiver {
                context.path.push(method.clone());
                rewrite_project_unit_qualified_path(
                    &mut context.path,
                    names,
                    known_units,
                    diagnostics,
                );
                if let Some(rewritten_method) = context.path.pop() {
                    *method = rewritten_method;
                }
            }
            rewrite_project_unit_qualified_expr(receiver, names, known_units, diagnostics);
            for arg in args {
                rewrite_project_unit_qualified_expr(arg, names, known_units, diagnostics);
            }
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            rewrite_project_unit_qualified_path(path, names, known_units, diagnostics);
            for arg in args {
                rewrite_project_unit_qualified_expr(arg, names, known_units, diagnostics);
            }
        }
        ncc_ast::Expr::StrictCheckExpr { target, value, .. } => {
            rewrite_project_unit_qualified_path(&mut target.path, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(value, names, known_units, diagnostics);
        }
        ncc_ast::Expr::PostIncrement { target }
        | ncc_ast::Expr::FieldPostIncrement { target }
        | ncc_ast::Expr::PointerPostIncrementRead { target }
        | ncc_ast::Expr::ContextRef(target) => {
            rewrite_project_unit_qualified_path(&mut target.path, names, known_units, diagnostics);
        }
        ncc_ast::Expr::Unary { expr, .. } => {
            rewrite_project_unit_qualified_expr(expr, names, known_units, diagnostics);
        }
        ncc_ast::Expr::Binary { left, right, .. } => {
            rewrite_project_unit_qualified_expr(left, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(right, names, known_units, diagnostics);
        }
        ncc_ast::Expr::Call { callee, args } => {
            let mut path = callee.split('.').map(str::to_string).collect::<Vec<_>>();
            rewrite_project_unit_qualified_path(&mut path, names, known_units, diagnostics);
            *callee = path.join(".");
            for arg in args {
                rewrite_project_unit_qualified_expr(arg, names, known_units, diagnostics);
            }
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            rewrite_project_unit_qualified_expr(condition, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(then_expr, names, known_units, diagnostics);
            rewrite_project_unit_qualified_expr(else_expr, names, known_units, diagnostics);
        }
        ncc_ast::Expr::Bool(_)
        | ncc_ast::Expr::Number(_)
        | ncc_ast::Expr::Char(_)
        | ncc_ast::Expr::String(_)
        | ncc_ast::Expr::Identifier(_)
        | ncc_ast::Expr::Dev(_) => {}
    }
}

fn rewrite_project_unit_qualified_assign_target(
    target: &mut ncc_ast::AssignTarget,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    match target {
        ncc_ast::AssignTarget::Context(ctx) => {
            rewrite_project_unit_qualified_path(&mut ctx.path, names, known_units, diagnostics);
        }
        ncc_ast::AssignTarget::Mem { addr } => {
            rewrite_project_unit_qualified_expr(addr, names, known_units, diagnostics);
        }
    }
}

fn rewrite_project_unit_qualified_main_kind(
    kind: &mut ncc_ast::MainEntryKind,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    match kind {
        ncc_ast::MainEntryKind::RefFn(fn_ref) | ncc_ast::MainEntryKind::SkkFn(fn_ref) => {
            rewrite_project_unit_qualified_path(&mut fn_ref.path, names, known_units, diagnostics);
        }
        ncc_ast::MainEntryKind::ToMain | ncc_ast::MainEntryKind::ToMainProfile(_) => {}
    }
}

fn rewrite_project_unit_qualified_path(
    path: &mut Vec<String>,
    names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    if path.len() < 3 {
        return;
    }
    let unit_key = path[0].to_ascii_lowercase();
    if !known_units.contains(&unit_key) {
        return;
    }
    let container_key = path[1].to_ascii_lowercase();
    let Some(materialized_container) = names.get(&(unit_key, container_key)) else {
        diagnostics.push(format!(
            "E_PROJECT_UNIT_QUALIFIED_001: unit-qualified path '{}' references unknown container '{}' in unit '{}'",
            path.join("."),
            path[1],
            path[0]
        ));
        return;
    };
    let mut rewritten = Vec::with_capacity(path.len() - 1);
    rewritten.push(materialized_container.clone());
    rewritten.extend(path.iter().skip(2).cloned());
    *path = rewritten;
}

fn sanitize_unit_symbol(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unit".to_string()
    } else {
        out
    }
}

fn write_lowering_json_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let lowering_dir = root.join(".ncc").join("lowering");
    fs::create_dir_all(&lowering_dir)?;
    for unit in &compiled.units {
        write_json(
            &lowering_dir.join(format!("{}.lowering.json", unit.unit.name)),
            &unit_lowering_payload(unit),
        )?;
    }
    write_json(
        &lowering_dir.join("__linked.lowering.json"),
        &linked_lowering_payload(compiled),
    )?;
    Ok(())
}

fn write_project_runtime_lowering_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing runtime lowering files")?;
    let lowering_dir = root.join(".ncc").join("lowering");
    let stamp = project_runtime_lowering_stamp(compiled);
    let stamp_path = lowering_dir.join(".run_stamp.json");
    let stamp_matches = fs::read_to_string(&stamp_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .as_ref()
        == Some(&stamp);
    if stamp_matches && project_runtime_lowering_files_exist(&lowering_dir, compiled) {
        return Ok(());
    }

    write_lowering_json_files(root, compiled)?;
    write_json(&stamp_path, &stamp)?;
    Ok(())
}

fn write_project_run_route_report(
    opts: &ProjectCommandOptions,
    route: &str,
    profile: &str,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(&opts.root, "writing run route report")?;
    let path = opts.root.join(".ncc").join("run").join("last.json");
    let payload = json!({
        "schema": "ncc.run.route.v1",
        "status": "ok",
        "source": slash_path(&opts.root.join(&opts.main_name)),
        "route": route,
        "profile": profile,
        "source_mode": "project-bridge",
        "cwd": slash_path(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    });
    write_json(&path, &payload)
}

fn write_single_file_run_route_report(
    root: &Path,
    source_path: &Path,
    route: &str,
    profile: &str,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing single-file run route report")?;
    let path = root.join(".ncc").join("run").join("last.json");
    let source = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let payload = json!({
        "schema": "ncc.run.route.v1",
        "status": "ok",
        "source": slash_path(&source),
        "route": route,
        "profile": profile,
        "source_mode": "single-file-workspace",
        "cwd": slash_path(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    });
    write_json(&path, &payload)
}

fn project_runtime_lowering_stamp(compiled: &ProjectCompileResult) -> serde_json::Value {
    json!({
        "schema": "ncc.project.runtime_lowering_stamp.v1",
        "units": compiled.units.iter().map(|unit| {
            json!({
                "unit": unit.unit.name,
                "hash": unit.hash,
                "tokens": unit.token_count,
                "containers": unit.container_count,
                "imports": unit.import_count
            })
        }).collect::<Vec<_>>(),
        "edges": compiled.edges.iter().map(|(from, to)| json!([from, to])).collect::<Vec<_>>()
    })
}

fn project_runtime_lowering_files_exist(
    lowering_dir: &Path,
    compiled: &ProjectCompileResult,
) -> bool {
    if !lowering_dir.join("__linked.lowering.json").is_file() {
        return false;
    }
    compiled.units.iter().all(|unit| {
        lowering_dir
            .join(format!("{}.lowering.json", unit.unit.name))
            .is_file()
    })
}

fn write_project_artifact_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing project artifacts")?;
    write_lowering_json_files(root, compiled)?;
    write_project_object_files(root, compiled)?;
    write_compiled_bridge_json(root, compiled)?;
    Ok(())
}

fn write_project_graph_artifact_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing project graph artifacts")?;
    write_lowering_json_files(root, compiled)?;
    write_available_project_object_files(root, compiled)?;
    if compiled
        .units
        .iter()
        .all(|unit| unit.object_artifact.is_some())
    {
        write_compiled_bridge_json(root, compiled)?;
    }
    Ok(())
}

fn write_available_project_object_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(root.join(".ncc").join("obj"))?;
    for (unit, artifact) in compiled.units.iter().filter_map(|unit| {
        unit.object_artifact
            .as_ref()
            .map(|artifact| (unit, artifact))
    }) {
        let object_path = root.join(&artifact.object_relative);
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_bytes(&object_path, &artifact.bytes)?;
        write_json(
            &root.join(&artifact.metadata_relative),
            &project_object_payload(unit, artifact),
        )?;
    }
    Ok(())
}

fn write_project_object_files(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(root.join(".ncc").join("obj"))?;
    for unit in &compiled.units {
        let artifact = unit.object_artifact.as_ref().ok_or_else(|| {
            format!(
                "E_PROJECT_UNIT_OBJECT_MISSING_001: unit '{}' has no object artifact",
                unit.unit.name
            )
        })?;
        let object_path = root.join(&artifact.object_relative);
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_bytes(&object_path, &artifact.bytes)?;
        write_json(
            &root.join(&artifact.metadata_relative),
            &project_object_payload(unit, artifact),
        )?;
    }
    Ok(())
}

fn write_compiled_bridge_json(
    root: &Path,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let unit_values = compiled
        .units
        .iter()
        .map(|unit| {
            let artifact = unit.object_artifact.as_ref().ok_or_else(|| {
                format!(
                    "E_PROJECT_UNIT_OBJECT_MISSING_001: unit '{}' has no object artifact",
                    unit.unit.name
                )
            })?;
            Ok(json!({
                "name": unit.unit.name,
                "source": unit.unit.relative_source,
                "code": unit.unit.code_relative,
                "source_hash": unit.hash,
                "object": artifact.object_relative,
                "object_hash": artifact.hash,
                "symbols": unit.unit.symbols,
                "imports": unit.unit.imports,
                "deps": unit.unit.deps
            }))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let link_order = compiled
        .units
        .iter()
        .map(|unit| unit.unit.name.clone())
        .collect::<Vec<_>>();
    let main = compiled
        .units
        .iter()
        .find(|unit| unit.unit.is_main)
        .map(|unit| unit.unit.relative_source.as_str())
        .unwrap_or("main.n");
    let edge_values = compiled
        .edges
        .iter()
        .map(|(from, to)| json!([from, to]))
        .collect::<Vec<_>>();
    let payload = json!({
        "schema": "ncc.bridge.v1",
        "linker": "ncc.project.physical_linker.v1",
        "main": main,
        "object_dir": ".ncc/obj",
        "target": project_backend_target_payload(&compiled.units),
        "units": unit_values,
        "edges": edge_values,
        "link_order": link_order
    });
    write_json(&root.join("bridge.json"), &payload)
}

fn link_project_executable(
    opts: &ProjectCommandOptions,
    compiled: &ProjectCompileResult,
) -> Result<serde_json::Value, Box<dyn Error>> {
    ncc_sema::validate_main_entries(
        &compiled.linked_program,
        ncc_sema::MainEntryRequirement::Required,
    )
    .map_err(|e| -> Box<dyn Error> { e.into() })?;
    let object_items = compiled
        .units
        .iter()
        .map(|unit| {
            let artifact = unit.object_artifact.as_ref().ok_or_else(|| {
                format!(
                    "E_PROJECT_UNIT_OBJECT_MISSING_001: unit '{}' has no object artifact",
                    unit.unit.name
                )
            })?;
            let path = opts.root.join(&artifact.object_relative);
            if !path.exists() {
                return Err(format!(
                    "E_PROJECT_LINK_001: object '{}' for unit '{}' does not exist",
                    slash_path(&path),
                    unit.unit.name
                )
                .into());
            }
            Ok((unit, artifact, path))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    if object_items.is_empty() {
        return Err("E_PROJECT_LINK_001: bridge has no objects to link".into());
    }

    let out_path = project_link_output_path(opts);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let entry_abi = project_entry_abi(&compiled.linked_program)?;
    let linker = opts.linker.clone().unwrap_or_else(|| {
        default_project_linker_for_target(&object_items[0].1.object.target_spec)
    });
    let runtime_object =
        write_project_runtime_object(opts, &linker, &object_items[0].1.object.target_spec)?;
    let nc_runtime_object =
        write_project_nc_runtime_object(opts, &object_items[0].1.object.target_spec)?;
    let input_section_names = object_items
        .iter()
        .flat_map(|(_, artifact, _)| artifact.object.sections.iter())
        .map(|section| section.name.as_str())
        .collect::<BTreeSet<_>>();
    let link_flags = project_link_flags(
        &object_items[0].1.object.target_spec,
        &linker,
        input_section_names.len(),
    );

    let mut command = Command::new(&linker);
    for flag in &link_flags {
        command.arg(flag);
    }
    command.arg(&runtime_object.object_path);
    command.arg(&nc_runtime_object.object_path);
    for (_, _, path) in &object_items {
        command.arg(path);
    }
    command.arg("-o").arg(&out_path);

    let output = command.output().map_err(|err| {
        format!(
            "E_PROJECT_LINK_001: failed to launch linker '{}': {}",
            linker, err
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "E_PROJECT_LINK_001: linker '{}' failed with status {}; stdout={}; stderr={}",
            linker,
            output.status,
            stdout.trim(),
            stderr.trim()
        )
        .into());
    }

    let exe_bytes = fs::read(&out_path).map_err(|err| {
        format!(
            "E_PROJECT_LINK_001: linked executable '{}' was not readable: {}",
            slash_path(&out_path),
            err
        )
    })?;
    if exe_bytes.is_empty() {
        return Err(format!(
            "E_PROJECT_LINK_001: linked executable '{}' is empty",
            slash_path(&out_path)
        )
        .into());
    }
    let exe_hash = sha256_bytes(&exe_bytes);
    let object_values = object_items
        .iter()
        .map(|(unit, artifact, path)| {
            json!({
                "unit": unit.unit.name,
                "object": artifact.object_relative,
                "path": slash_path(path),
                "hash": artifact.hash,
                "bytes": artifact.bytes.len(),
                "cache_status": artifact.cache_status,
                "cache_key": artifact.cache_key,
                "format": project_backend_object_format_label(&artifact.object.target_spec.object_format)
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "schema": "ncc.project.physical_link.v1",
        "status": "ok",
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "bridge": "bridge.json",
        "linker": linker,
        "output": slash_path(&out_path),
        "output_hash": exe_hash,
        "output_bytes": exe_bytes.len(),
        "link_flags": link_flags,
        "entry": {
            "schema": "ncc.project.entry_object.v1",
            "symbol": entry_abi.symbol,
            "exit_mode": entry_abi.exit_mode.label(),
            "source": serde_json::Value::Null,
            "materialization": "mct-hosted-main-in-project-unit",
            "object": object_items.iter().find(|(unit, _, _)| unit.unit.is_main).map(|(_, _, path)| slash_path(path)),
            "format": project_backend_object_format_label(&object_items[0].1.object.target_spec.object_format)
        },
        "runtime": {
            "schema": "ncc.project.runtime_object.v1",
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-llvm-ir-stdin",
            "source_sha256": runtime_object.source_sha256,
            "object": slash_path(&runtime_object.object_path),
            "object_hash": runtime_object.object_hash,
            "object_bytes": runtime_object.object_bytes,
            "symbols": ["readInt", "nC.runtime.input.readInt", "renderInt", "renderBool", "renderFloat", "colorBold"]
        },
        "nc_runtime": {
            "schema": "ncc.project.nc_runtime_object.v1",
            "source": "runtime/nc",
            "source_sha256": nc_runtime_object.source_sha256,
            "object": slash_path(&nc_runtime_object.object_path),
            "object_hash": nc_runtime_object.object_hash,
            "object_bytes": nc_runtime_object.object_bytes,
            "cache_status": nc_runtime_object.cache_status,
            "cache_key": nc_runtime_object.cache_key
        },
        "object_count": object_items.len(),
        "object_dir": ".ncc/obj",
        "objects": object_values,
        "link_order": compiled.units.iter().map(|unit| unit.unit.name.clone()).collect::<Vec<_>>()
    }))
}

fn project_link_flags(
    target: &ncc_ir::BackendTargetSpecIR,
    linker: &str,
    input_section_count: usize,
) -> Vec<String> {
    if matches!(target.object_format, ncc_ir::BackendObjectFormatIR::Elf64) {
        return vec!["-no-pie".to_string()];
    }
    let linker = linker.to_ascii_lowercase().replace('\\', "/");
    if matches!(target.object_format, ncc_ir::BackendObjectFormatIR::Coff)
        && (linker.contains("mingw")
            || linker.ends_with("/gcc")
            || linker.ends_with("/gcc.exe")
            || linker == "gcc"
            || linker == "gcc.exe"
            || linker == "cc")
    {
        let mut flags = vec![
            "-Wl,--defsym,__chkstk=___chkstk_ms".to_string(),
            "-lshell32".to_string(),
        ];
        let section_alignment = project_coff_image_section_alignment(input_section_count);
        if section_alignment > 4096 {
            flags.push(format!("-Wl,--section-alignment,{section_alignment}"));
        }
        return flags;
    }
    if matches!(target.object_format, ncc_ir::BackendObjectFormatIR::Coff)
        && (linker.contains("clang") || linker.contains("clang-cl"))
    {
        return vec![
            "-lshell32".to_string(),
            "-llegacy_stdio_definitions".to_string(),
        ];
    }
    Vec::new()
}

fn project_coff_image_section_alignment(input_section_count: usize) -> usize {
    const PE_HEADERS_WITHOUT_SECTIONS: usize = 392;
    const PE_SECTION_HEADER_BYTES: usize = 40;
    const LINKER_GENERATED_SECTION_RESERVE: usize = 16;
    let projected_sections = input_section_count.saturating_add(LINKER_GENERATED_SECTION_RESERVE);
    let required = PE_HEADERS_WITHOUT_SECTIONS
        .saturating_add(projected_sections.saturating_mul(PE_SECTION_HEADER_BYTES));
    required.next_power_of_two().max(4096)
}

struct ProjectEntryObject {
    source_path: PathBuf,
    object_path: PathBuf,
    object_hash: String,
    object_bytes: usize,
}

struct ProjectRuntimeObject {
    source_sha256: String,
    object_path: PathBuf,
    object_hash: String,
    object_bytes: usize,
}

struct ProjectNcRuntimeObject {
    source_sha256: String,
    object_path: PathBuf,
    object_hash: String,
    object_bytes: usize,
    cache_status: String,
    cache_key: String,
}

fn write_project_nc_runtime_object(
    opts: &ProjectCommandOptions,
    target: &ncc_ir::BackendTargetSpecIR,
) -> Result<ProjectNcRuntimeObject, Box<dyn Error>> {
    let program = project_nc_runtime_provider_program(&target.target_triple)?;
    ncc_sema::analyze(&program).map_err(|err| {
        ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_LINK_004,
            format!("nC runtime provider sema failed: {err}"),
        )
    })?;
    let source_material = [
        super::compiler::COMPILER_NATIVE_NC_TEXT_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_ESCAPE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_PARSE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_UNICODE_TABLES_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_UNICODE_RUNTIME_SOURCE,
        super::compiler::COMPILER_NATIVE_NC_JSON_RUNTIME_SOURCE,
    ]
    .join("\n");
    let source_sha256 = sha256_bytes(source_material.as_bytes());
    let object_path = opts
        .root
        .join(project_target_object_dir(target))
        .join(format!(
            "__project_nc_runtime.{}",
            project_object_extension(&target.object_format)
        ));
    let cache_material = json!({
        "schema": "ncc.project.nc_runtime_object_content_key.v1",
        "source_sha256": source_sha256,
        "target": project_target_spec_payload(target),
        "compiler": crate::content_cache::compiler_image_identity(),
    });
    let cache_key =
        crate::content_cache::content_cache_key("project-nc-runtime-object", &cache_material);
    let cache_root = crate::content_cache::content_cache_root(None, &opts.root.join(".ncc"));
    let incremental_enabled = std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_none();
    if incremental_enabled {
        let destinations = BTreeMap::from([("object".to_string(), object_path.clone())]);
        if let Ok(Some(_)) = crate::content_cache::restore_content_cache_entry(
            &cache_root,
            "project-nc-runtime-object",
            &cache_key,
            &destinations,
        ) {
            let bytes = fs::read(&object_path)?;
            return Ok(ProjectNcRuntimeObject {
                source_sha256,
                object_hash: sha256_bytes(&bytes),
                object_bytes: bytes.len(),
                object_path,
                cache_status: "hit".to_string(),
                cache_key,
            });
        }
    }
    let ir = ncc_ir::lower_with_backend_profile(program, opts.profile.clone());
    let module = project_mct_emit_release_module(&ir, "__project_nc_runtime");
    let bytes = emit_project_mct_object_bytes("__project_nc_runtime", &module, &object_path)?;
    let object_hash = sha256_bytes(&bytes);
    if incremental_enabled {
        let artifacts = [crate::content_cache::ContentCacheArtifact::new(
            "object",
            &object_path,
        )];
        let _ = crate::content_cache::store_content_cache_entry(
            &cache_root,
            "project-nc-runtime-object",
            &cache_key,
            &cache_material,
            json!({"object_hash": object_hash, "object_bytes": bytes.len()}),
            &artifacts,
        );
    }
    Ok(ProjectNcRuntimeObject {
        source_sha256,
        object_path,
        object_hash,
        object_bytes: bytes.len(),
        cache_status: if incremental_enabled {
            "miss"
        } else {
            "disabled"
        }
        .to_string(),
        cache_key,
    })
}

fn write_project_runtime_object(
    opts: &ProjectCommandOptions,
    linker: &str,
    target: &ncc_ir::BackendTargetSpecIR,
) -> Result<ProjectRuntimeObject, Box<dyn Error>> {
    if !matches!(
        target.object_format,
        ncc_ir::BackendObjectFormatIR::Elf64
            | ncc_ir::BackendObjectFormatIR::Coff
            | ncc_ir::BackendObjectFormatIR::MachO
    ) {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_LINK_003,
            format!(
                "project runtime object format={}",
                project_backend_object_format_label(&target.object_format)
            ),
        )
        .into());
    }

    let obj_dir = opts.root.join(project_target_object_dir(target));
    fs::create_dir_all(&obj_dir)?;
    let object_path = obj_dir.join(format!(
        "__project_runtime.{}",
        project_object_extension(&target.object_format)
    ));
    let source = project_runtime_llvm_ir(&target.target_triple);
    let source_sha256 = sha256_bytes(source.as_bytes());
    let Some(compiler) = project_find_clang(Some(linker)) else {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_LINK_003,
            "project runtime object requires clang for the embedded LLVM IR provider",
        )
        .into());
    };
    let mut child = Command::new(&compiler)
        .arg(format!("--target={}", target.target_triple))
        .args(["-O2", "-x", "ir", "-c", "-", "-o"])
        .arg(&object_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| {
            ncc_diagnostics::registered_error_text(
                ncc_diagnostics::codes::E_PROJECT_LINK_003,
                format!("failed to launch runtime compiler '{compiler}': {err}"),
            )
        })?;
    child
        .stdin
        .take()
        .ok_or_else(|| {
            ncc_diagnostics::registered_error_text(
                ncc_diagnostics::codes::E_PROJECT_LINK_003,
                "clang did not expose stdin for the LLVM IR runtime",
            )
        })?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_PROJECT_LINK_003,
            format!(
                "compiler='{}' status={} stdout={} stderr={}",
                compiler,
                output.status,
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        )
        .into());
    }

    let bytes = fs::read(&object_path)?;
    let object_hash = sha256_bytes(&bytes);
    write_json(
        &object_path.with_extension(format!(
            "{}.json",
            project_object_extension(&target.object_format)
        )),
        &json!({
            "schema": "ncc.project.runtime_object.v1",
            "status": "emitted",
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-llvm-ir-stdin",
            "source_sha256": source_sha256,
            "object": slash_path(&object_path),
            "object_hash": object_hash,
            "object_bytes": bytes.len(),
            "format": project_backend_object_format_label(&target.object_format),
            "compiler": compiler,
            "symbols": ["readInt", "nC.runtime.input.readInt", "renderInt", "renderBool", "renderFloat", "colorBold"]
        }),
    )?;

    Ok(ProjectRuntimeObject {
        source_sha256,
        object_path,
        object_hash,
        object_bytes: bytes.len(),
    })
}

fn project_runtime_llvm_ir(target_triple: &str) -> String {
    r#"target triple = "NCC_TARGET_TRIPLE"

@ncc.render.index = internal global i64 0, align 8
@ncc.render.buffers = internal global [4096 x i8] zeroinitializer, align 16
@ncc.color.index = internal global i64 0, align 8
@ncc.color.buffers = internal global [8192 x i8] zeroinitializer, align 16
@ncc.fmt.int = private unnamed_addr constant [5 x i8] c"%lld\00"
@ncc.fmt.str = private unnamed_addr constant [3 x i8] c"%s\00"
@ncc.fmt.f0 = private unnamed_addr constant [5 x i8] c"%.0f\00"
@ncc.fmt.f1 = private unnamed_addr constant [5 x i8] c"%.1f\00"
@ncc.fmt.f2 = private unnamed_addr constant [5 x i8] c"%.2f\00"
@ncc.fmt.f3 = private unnamed_addr constant [5 x i8] c"%.3f\00"
@ncc.fmt.color = private unnamed_addr constant [14 x i8] c"\1B[1;%sm%s\1B[0m\00"
@ncc.true = private unnamed_addr constant [5 x i8] c"true\00"
@ncc.false = private unnamed_addr constant [6 x i8] c"false\00"
@ncc.empty = private unnamed_addr constant [1 x i8] zeroinitializer
@ncc.c31 = private unnamed_addr constant [3 x i8] c"31\00"
@ncc.c32 = private unnamed_addr constant [3 x i8] c"32\00"
@ncc.c33 = private unnamed_addr constant [3 x i8] c"33\00"
@ncc.c34 = private unnamed_addr constant [3 x i8] c"34\00"
@ncc.c35 = private unnamed_addr constant [3 x i8] c"35\00"
@ncc.c36 = private unnamed_addr constant [3 x i8] c"36\00"
@ncc.c37 = private unnamed_addr constant [3 x i8] c"37\00"

declare i32 @scanf(ptr, ...)
declare i32 @snprintf(ptr, i64, ptr, ...)

define internal ptr @ncc.next.render() {
entry:
  %old = load i64, ptr @ncc.render.index, align 8
  %next.raw = add i64 %old, 1
  %next = and i64 %next.raw, 31
  store i64 %next, ptr @ncc.render.index, align 8
  %offset = mul i64 %next, 128
  %out = getelementptr [4096 x i8], ptr @ncc.render.buffers, i64 0, i64 %offset
  ret ptr %out
}

define internal ptr @ncc.next.color() {
entry:
  %old = load i64, ptr @ncc.color.index, align 8
  %next.raw = add i64 %old, 1
  %next = and i64 %next.raw, 15
  store i64 %next, ptr @ncc.color.index, align 8
  %offset = mul i64 %next, 512
  %out = getelementptr [8192 x i8], ptr @ncc.color.buffers, i64 0, i64 %offset
  ret ptr %out
}

define i64 @readInt() {
entry:
  %value = alloca i64, align 8
  store i64 0, ptr %value, align 8
  %read = call i32 (ptr, ...) @scanf(ptr @ncc.fmt.int, ptr %value)
  %ok = icmp eq i32 %read, 1
  %loaded = load i64, ptr %value, align 8
  %result = select i1 %ok, i64 %loaded, i64 0
  ret i64 %result
}

define i64 @"nC.runtime.input.readInt"() {
entry:
  %value = call i64 @readInt()
  ret i64 %value
}

define ptr @renderInt(i64 %value) {
entry:
  %out = call ptr @ncc.next.render()
  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.int, i64 %value)
  ret ptr %out
}

define ptr @renderBool(i64 %value) {
entry:
  %truth = icmp ne i64 %value, 0
  %text = select i1 %truth, ptr @ncc.true, ptr @ncc.false
  %out = call ptr @ncc.next.render()
  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.str, ptr %text)
  ret ptr %out
}

define ptr @renderFloat(double %value, i64 %decimals) {
entry:
  %out = call ptr @ncc.next.render()
  %positive = icmp sgt i64 %decimals, 0
  br i1 %positive, label %one.check, label %zero
one.check:
  %is.one = icmp eq i64 %decimals, 1
  br i1 %is.one, label %one, label %two.check
two.check:
  %is.two = icmp eq i64 %decimals, 2
  br i1 %is.two, label %two, label %three
zero:
  %w0 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.f0, double %value)
  br label %done
one:
  %w1 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.f1, double %value)
  br label %done
two:
  %w2 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.f2, double %value)
  br label %done
three:
  %w3 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 128, ptr @ncc.fmt.f3, double %value)
  br label %done
done:
  ret ptr %out
}

define internal ptr @ncc.color.code(ptr %color) {
entry:
  %null = icmp eq ptr %color, null
  br i1 %null, label %default, label %load
load:
  %first = load i8, ptr %color, align 1
  %lower = or i8 %first, 32
  switch i8 %lower, label %default [
    i8 114, label %red
    i8 103, label %green
    i8 121, label %yellow
    i8 98, label %blue
    i8 109, label %magenta
    i8 99, label %cyan
  ]
red:
  ret ptr @ncc.c31
green:
  ret ptr @ncc.c32
yellow:
  ret ptr @ncc.c33
blue:
  ret ptr @ncc.c34
magenta:
  ret ptr @ncc.c35
cyan:
  ret ptr @ncc.c36
default:
  ret ptr @ncc.c37
}

define ptr @colorBold(ptr %text, ptr %color) {
entry:
  %out = call ptr @ncc.next.color()
  %text.null = icmp eq ptr %text, null
  %safe.text = select i1 %text.null, ptr @ncc.empty, ptr %text
  %code = call ptr @ncc.color.code(ptr %color)
  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %out, i64 512, ptr @ncc.fmt.color, ptr %code, ptr %safe.text)
  ret ptr %out
}
"#
    .replace("NCC_TARGET_TRIPLE", target_triple)
}

fn write_project_entry_object(
    opts: &ProjectCommandOptions,
    linker: &str,
    entry: &ProjectEntryAbi,
    target: &ncc_ir::BackendTargetSpecIR,
) -> Result<ProjectEntryObject, Box<dyn Error>> {
    if !matches!(
        target.isa,
        ncc_ir::BackendIsaIR::X86_64 | ncc_ir::BackendIsaIR::AArch64
    ) {
        return Err(format!(
            "E_PROJECT_LINK_002: project entry object currently supports x86_64/aarch64 host objects, got {:?}",
            target.isa
        )
        .into());
    }
    if !matches!(
        target.object_format,
        ncc_ir::BackendObjectFormatIR::Elf64
            | ncc_ir::BackendObjectFormatIR::Coff
            | ncc_ir::BackendObjectFormatIR::MachO
    ) {
        return Err(format!(
            "E_PROJECT_LINK_002: project entry object requires a host object format, got {}",
            project_backend_object_format_label(&target.object_format)
        )
        .into());
    }

    let obj_dir = opts.root.join(project_target_object_dir(target));
    fs::create_dir_all(&obj_dir)?;
    let source_path = obj_dir.join("__project_entry.S");
    let object_path = obj_dir.join(format!(
        "__project_entry.{}",
        project_object_extension(&target.object_format)
    ));
    let assembly = project_entry_assembly(entry, target);
    write_text(&source_path, &assembly)?;

    let output = Command::new(linker)
        .arg("-c")
        .arg(&source_path)
        .arg("-o")
        .arg(&object_path)
        .output()
        .map_err(|err| {
            format!(
                "E_PROJECT_LINK_002: failed to launch entry-object compiler '{}': {}",
                linker, err
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "E_PROJECT_LINK_002: entry-object compiler '{}' failed with status {}; stdout={}; stderr={}",
            linker,
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let bytes = fs::read(&object_path)?;
    let object_hash = sha256_bytes(&bytes);
    write_json(
        &object_path.with_extension(format!(
            "{}.json",
            project_object_extension(&target.object_format)
        )),
        &json!({
            "schema": "ncc.project.entry_object.v1",
            "status": "emitted",
            "source": slash_path(&source_path),
            "object": slash_path(&object_path),
            "object_hash": object_hash,
            "object_bytes": bytes.len(),
            "entry_symbol": entry.symbol,
            "exit_mode": entry.exit_mode.label(),
            "format": project_backend_object_format_label(&target.object_format),
            "assembler": linker
        }),
    )?;

    Ok(ProjectEntryObject {
        source_path,
        object_path,
        object_hash,
        object_bytes: bytes.len(),
    })
}

fn project_entry_assembly(entry: &ProjectEntryAbi, target: &ncc_ir::BackendTargetSpecIR) -> String {
    match target.isa {
        ncc_ir::BackendIsaIR::AArch64 => {
            project_entry_assembly_aarch64(entry, &target.object_format)
        }
        _ => project_entry_assembly_x86_64(entry, &target.object_format),
    }
}

fn project_symbol_for_object(
    symbol: &str,
    object_format: &ncc_ir::BackendObjectFormatIR,
) -> String {
    if matches!(object_format, ncc_ir::BackendObjectFormatIR::MachO) {
        format!("_{symbol}")
    } else {
        symbol.to_string()
    }
}

fn project_main_label_for_object(object_format: &ncc_ir::BackendObjectFormatIR) -> &'static str {
    if matches!(object_format, ncc_ir::BackendObjectFormatIR::MachO) {
        "_main"
    } else {
        "main"
    }
}

fn project_entry_assembly_x86_64(
    entry: &ProjectEntryAbi,
    object_format: &ncc_ir::BackendObjectFormatIR,
) -> String {
    let (main_label, call_symbol, function_type_line) = match object_format {
        ncc_ir::BackendObjectFormatIR::MachO => {
            ("_main", format!("_{}", entry.symbol), String::new())
        }
        ncc_ir::BackendObjectFormatIR::Coff => ("main", entry.symbol.clone(), String::new()),
        _ => (
            "main",
            entry.symbol.clone(),
            "    .type main, @function\n".to_string(),
        ),
    };
    let shadow_space = if matches!(object_format, ncc_ir::BackendObjectFormatIR::Coff) {
        "    sub $32, %rsp\n"
    } else {
        ""
    };
    let exit_body = match entry.exit_mode {
        ProjectEntryExitMode::Bool => {
            "    cmp $0, %rax\n    sete %al\n    movzbl %al, %eax\n    leave\n    ret\n"
        }
        ProjectEntryExitMode::Int => "    leave\n    ret\n",
        ProjectEntryExitMode::Void => "    xor %eax, %eax\n    leave\n    ret\n",
    };
    format!(
        "    .text\n    .globl {main_label}\n{function_type_line}{main_label}:\n    push %rbp\n    mov %rsp, %rbp\n{shadow_space}    call {call_symbol}\n{exit_body}"
    )
}

fn project_entry_assembly_aarch64(
    entry: &ProjectEntryAbi,
    object_format: &ncc_ir::BackendObjectFormatIR,
) -> String {
    let main_label = project_main_label_for_object(object_format);
    let call_symbol = project_symbol_for_object(&entry.symbol, object_format);
    let function_type_line = if matches!(object_format, ncc_ir::BackendObjectFormatIR::Elf64) {
        format!("    .type {main_label}, %function\n")
    } else {
        String::new()
    };
    let exit_body = match entry.exit_mode {
        ProjectEntryExitMode::Bool => "    cmp x0, #0\n    cset w0, eq\n",
        ProjectEntryExitMode::Int => "",
        ProjectEntryExitMode::Void => "    mov x0, #0\n",
    };
    format!(
        "    .text\n    .globl {main_label}\n{function_type_line}{main_label}:\n    stp x29, x30, [sp, #-16]!\n    mov x29, sp\n    bl {call_symbol}\n{exit_body}    ldp x29, x30, [sp], #16\n    ret\n"
    )
}

fn project_entry_abi(program: &ncc_ast::Program) -> Result<ProjectEntryAbi, Box<dyn Error>> {
    for container in &program.containers {
        for section in &container.sections {
            let ncc_ast::Section::MainEntry { kind, body } = section else {
                continue;
            };
            return match kind {
                ncc_ast::MainEntryKind::RefFn(fn_ref) | ncc_ast::MainEntryKind::SkkFn(fn_ref) => {
                    let (container_name, function_name) =
                        project_entry_target_path(&container.name, &fn_ref.path)?;
                    let return_type =
                        project_find_function_return_type(program, &container_name, &function_name)
                            .ok_or_else(|| {
                                format!(
                                    "E_PROJECT_LINK_002: entry target '{}.{}' not found",
                                    container_name, function_name
                                )
                            })?;
                    Ok(ProjectEntryAbi {
                        symbol: format!("{container_name}.{function_name}"),
                        exit_mode: project_entry_exit_mode(return_type)?,
                    })
                }
                ncc_ast::MainEntryKind::ToMain | ncc_ast::MainEntryKind::ToMainProfile(_) => {
                    if body.is_empty() {
                        Err("E_PROJECT_LINK_002: empty to main cannot synthesize a physical entry object; use ref main ->> @fn(...) for project link".into())
                    } else {
                        Err("E_PROJECT_LINK_002: to main inline bodies are runnable through ncc project run, but physical project link currently requires ref main ->> @fn(...) so it can bind a concrete object symbol".into())
                    }
                }
            };
        }
    }
    Err("E_PROJECT_LINK_002: project has no main entry".into())
}

fn project_entry_target_path(
    current_container: &str,
    path: &[String],
) -> Result<(String, String), Box<dyn Error>> {
    match path {
        [function] => Ok((current_container.to_string(), function.clone())),
        [container, function] => Ok((container.clone(), function.clone())),
        [] => Err("E_PROJECT_LINK_002: empty ref main target".into()),
        _ => Err(format!(
            "E_PROJECT_LINK_002: physical project link expects a materialized container.function entry target, got '{}'",
            path.join(".")
        )
        .into()),
    }
}

fn project_find_function_return_type<'a>(
    program: &'a ncc_ast::Program,
    container_name: &str,
    function_name: &str,
) -> Option<&'a ncc_ast::TypeName> {
    let container = program
        .containers
        .iter()
        .find(|container| container.name == container_name)?;
    for section in &container.sections {
        match section {
            ncc_ast::Section::Fn(functions)
            | ncc_ast::Section::MoveFn(functions)
            | ncc_ast::Section::ConstFn(functions) => {
                if let Some(function) = functions
                    .iter()
                    .find(|function| function.name == function_name)
                {
                    return Some(&function.return_type);
                }
            }
            _ => {}
        }
    }
    None
}

fn project_entry_exit_mode(
    return_type: &ncc_ast::TypeName,
) -> Result<ProjectEntryExitMode, Box<dyn Error>> {
    let ncc_ast::TypeName::Simple(name) = return_type else {
        return Err(format!(
            "E_PROJECT_LINK_002: physical entry return type must be scalar bool/int/void, got {:?}",
            return_type
        )
        .into());
    };
    let compact = ncc_ast::compact_type_surface_name(name);
    if compact == "bool" {
        return Ok(ProjectEntryExitMode::Bool);
    }
    if compact == "void" {
        return Ok(ProjectEntryExitMode::Void);
    }
    let Some(contract) = ncc_ast::scalar_type_contract(&compact, 8) else {
        return Err(format!(
            "E_PROJECT_LINK_002: physical entry return type '{}' is not a backend scalar exit type",
            name
        )
        .into());
    };
    if matches!(
        contract.class,
        ncc_ast::ScalarTypeClass::SignedInteger
            | ncc_ast::ScalarTypeClass::UnsignedInteger
            | ncc_ast::ScalarTypeClass::Char
    ) {
        Ok(ProjectEntryExitMode::Int)
    } else {
        Err(format!(
            "E_PROJECT_LINK_002: physical entry return type '{}' cannot be represented as a process exit code",
            name
        )
        .into())
    }
}

impl ProjectEntryExitMode {
    fn label(self) -> &'static str {
        match self {
            ProjectEntryExitMode::Bool => "bool_true_zero_false_one",
            ProjectEntryExitMode::Int => "int_passthrough",
            ProjectEntryExitMode::Void => "void_zero",
        }
    }
}

fn project_link_output_path(opts: &ProjectCommandOptions) -> PathBuf {
    if let Some(out) = &opts.out {
        if out.is_absolute() {
            return out.clone();
        }
        return opts.root.join(out);
    }
    let stem = Path::new(&opts.main_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("main");
    let file_name = if matches!(
        opts.profile.target_spec.object_format,
        ncc_ir::BackendObjectFormatIR::Coff
    ) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    };
    opts.root
        .join(".ncc")
        .join("bin")
        .join(sanitize_unit_symbol(
            &opts.profile.target_spec.target_triple,
        ))
        .join(file_name)
}

fn default_project_linker_for_target(target: &ncc_ir::BackendTargetSpecIR) -> String {
    if let Ok(linker) = std::env::var("NCC_LINKER") {
        return linker;
    }
    if matches!(target.object_format, ncc_ir::BackendObjectFormatIR::Coff)
        && command_exists_on_path("x86_64-w64-mingw32-gcc")
    {
        return "x86_64-w64-mingw32-gcc".to_string();
    }
    if matches!(target.object_format, ncc_ir::BackendObjectFormatIR::Coff) {
        if let Some(clang) = project_find_clang(None) {
            return clang;
        }
    }
    "cc".to_string()
}

fn command_exists_on_path(command: &str) -> bool {
    let candidate = Path::new(command);
    if candidate.components().count() > 1 {
        return candidate.exists();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    let candidates = command_file_candidates(command);
    std::env::split_paths(&paths).any(|dir| {
        candidates
            .iter()
            .any(|candidate| dir.join(candidate).is_file())
    })
}

fn command_file_candidates(command: &str) -> Vec<String> {
    let path = Path::new(command);
    if path.extension().is_some() || !cfg!(windows) {
        return vec![command.to_string()];
    }
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let mut candidates = vec![command.to_string()];
    for extension in extensions.split(';') {
        let extension = extension.trim();
        if extension.is_empty() {
            continue;
        }
        candidates.push(format!("{}{}", command, extension.to_ascii_lowercase()));
        candidates.push(format!("{}{}", command, extension.to_ascii_uppercase()));
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn project_find_clang(preferred: Option<&str>) -> Option<String> {
    if let Some(preferred) = preferred.filter(|value| {
        value.to_ascii_lowercase().contains("clang") && command_exists_on_path(value)
    }) {
        return Some(preferred.to_string());
    }
    if let Ok(configured) = std::env::var("NCC_CLANG") {
        if command_exists_on_path(&configured) {
            return Some(configured);
        }
    }
    if command_exists_on_path("clang") {
        return Some("clang".to_string());
    }
    if let Some(candidate) = project_known_clang_absolute() {
        return Some(candidate);
    }
    if cfg!(windows) {
        for candidate in [
            r"C:\Program Files\LLVM\bin\clang.exe",
            r"C:\Program Files (x86)\LLVM\bin\clang.exe",
        ] {
            if Path::new(candidate).is_file() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn project_known_clang_absolute() -> Option<String> {
    if cfg!(windows) {
        return None;
    }
    [
        "/usr/bin/clang",
        "/usr/local/bin/clang",
        "/opt/homebrew/opt/llvm/bin/clang",
        "/opt/local/libexec/llvm/bin/clang",
    ]
    .into_iter()
    .find(|candidate| Path::new(candidate).is_file())
    .map(str::to_string)
}

fn unit_lowering_payload(unit: &ProjectCompiledUnit) -> serde_json::Value {
    let mut payload = json!({
        "schema": "ncc.project.lowering.v1",
        "unit": unit.unit.name,
        "source": unit.unit.relative_source,
        "hash": unit.hash,
        "hash_digest_bytes_hex": bytes_to_hex_vec(&unit.hash_bytes),
        "hash_string_bytes_hex": bytes_to_hex_vec(unit.hash.as_bytes()),
        "source_byte_count": unit.source_bytes.len(),
        "source_bytes_hex": bytes_to_hex_vec(&unit.source_bytes),
        "source_chars": source_chars_payload(&unit.source_text),
        "tokens": unit.token_count,
        "containers": unit.container_count,
        "imports": unit.import_count,
        "ast_cache": unit.ast_cache_status,
        "parser": "closed_real_parser",
        "sema": "checked_in_linked_program",
        "ir": {
            "status": "lowered",
            "debug_bytes_hex": bytes_to_hex_vec(unit.ir_debug.as_bytes())
        },
        "backend": {
            "status": "lowered",
            "debug_bytes_hex": bytes_to_hex_vec(unit.backend_debug.as_bytes())
        }
    });
    if let Some(artifact) = &unit.object_artifact {
        payload["object"] = project_object_summary_payload(artifact);
    }
    payload
}

fn linked_lowering_payload(compiled: &ProjectCompileResult) -> serde_json::Value {
    let unit_hashes = compiled
        .units
        .iter()
        .map(|unit| {
            json!({
                "unit": unit.unit.name,
                "hash": unit.hash,
                "hash_digest_bytes_hex": bytes_to_hex_vec(&unit.hash_bytes)
            })
        })
        .collect::<Vec<_>>();
    let materialized_containers = project_materialized_containers_payload(&compiled.units);
    let object_artifacts = compiled
        .units
        .iter()
        .filter_map(|unit| {
            unit.object_artifact.as_ref().map(|artifact| {
                json!({
                    "unit": unit.unit.name,
                    "object": artifact.object_relative,
                    "metadata": artifact.metadata_relative,
                    "hash": artifact.hash,
                    "bytes": artifact.bytes.len(),
                    "format": project_backend_object_format_label(&artifact.object.target_spec.object_format)
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ncc.project.linked_lowering.v1",
        "status": "linked_real_compiler_path",
        "units": unit_hashes,
        "edges": compiled.edges.iter().map(|(from, to)| json!([from, to])).collect::<Vec<_>>(),
        "project_linker": {
            "status": "closed_unit_namespace_materialization",
            "materialized_containers": materialized_containers
        },
        "object_artifacts": object_artifacts,
        "linked_ir_bytes_hex": bytes_to_hex_vec(compiled.linked_ir_debug.as_bytes()),
        "linked_backend_bytes_hex": bytes_to_hex_vec(compiled.linked_backend_debug.as_bytes())
    })
}

fn project_object_payload(
    unit: &ProjectCompiledUnit,
    artifact: &ProjectObjectArtifact,
) -> serde_json::Value {
    let object = &artifact.object;
    let unresolved = object.unresolved_symbols();
    let exported_symbols = object
        .symbols
        .iter()
        .filter(|symbol| matches!(symbol.visibility, ncc_ir::BackendSymbolVisibilityIR::Global))
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();
    json!({
        "schema": "ncc.project.object.v1",
        "status": "emitted",
        "kind": "native_backend_object",
        "unit": unit.unit.name,
        "source": unit.unit.relative_source,
        "source_hash": unit.hash,
        "source_hash_digest_bytes_hex": bytes_to_hex_vec(&unit.hash_bytes),
        "object": artifact.object_relative,
        "artifact": artifact.object_relative,
        "metadata": artifact.metadata_relative,
        "bridge": "../../bridge.json",
        "code": unit.unit.code_relative,
        "extension": artifact.extension,
        "hash": artifact.hash,
        "hash_kind": "sha256:object-bytes",
        "object_hash": artifact.hash,
        "object_hash_digest_bytes_hex": bytes_to_hex_vec(&artifact.hash_bytes),
        "object_byte_count": artifact.bytes.len(),
        "object_bytes_hex": bytes_to_hex_vec(&artifact.bytes),
        "object_text_bytes_hex": bytes_to_hex_vec(artifact.text.as_bytes()),
        "incremental": {
            "mode": "content-addressed-unit-object",
            "status": artifact.cache_status,
            "key": artifact.cache_key
        },
        "abi": project_object_abi_payload(object),
        "exports": exported_symbols,
        "exported_symbols": exported_symbols,
        "imported_symbols": unresolved,
        "bridge_symbols": unit.unit.symbols,
        "bridge_imports": unit.unit.imports,
        "bridge_deps": unit.unit.deps,
        "imports": {
            "project_units": project_import_targets_from_ast(&unit.ast),
            "source_uses": project_use_paths_from_ast(&unit.ast),
            "unresolved_symbols": object.unresolved_symbols()
        },
        "symbols": {
            "defined": project_object_symbols_payload(object),
            "exported": project_object_exported_symbols_payload(object)
        },
        "sections": project_object_sections_payload(object),
        "relocations": project_object_relocations_payload(object)
    })
}

fn project_object_summary_payload(artifact: &ProjectObjectArtifact) -> serde_json::Value {
    json!({
        "schema": "ncc.project.object.summary.v1",
        "artifact": artifact.object_relative,
        "metadata": artifact.metadata_relative,
        "format": project_backend_object_format_label(&artifact.object.target_spec.object_format),
        "abi": project_backend_abi_label(&artifact.object.target_spec.abi),
        "hash": artifact.hash,
        "bytes": artifact.bytes.len(),
        "incremental": {
            "mode": "content-addressed-unit-object",
            "status": artifact.cache_status,
            "key": artifact.cache_key
        },
        "exported_symbols": artifact
            .object
            .symbols
            .iter()
            .filter(|symbol| matches!(symbol.visibility, ncc_ir::BackendSymbolVisibilityIR::Global))
            .map(|symbol| symbol.name.clone())
            .collect::<Vec<_>>(),
        "imported_symbols": artifact.object.unresolved_symbols()
    })
}

fn project_ir_payload(compiled: &ProjectCompileResult) -> serde_json::Value {
    let materialized_containers = project_materialized_containers_payload(&compiled.units);
    json!({
        "schema": "ncc.project.priority13b.ir.v1",
        "status": "ok",
        "units": compiled.units.iter().map(|unit| {
            json!({
                "unit": unit.unit.name,
                "source": unit.unit.relative_source,
                "hash": unit.hash,
                "ir": unit.ir_debug
            })
        }).collect::<Vec<_>>(),
        "project_linker": {
            "status": "closed_unit_namespace_materialization",
            "materialized_containers": materialized_containers
        },
        "linked_ir": compiled.linked_ir_debug
    })
}

fn project_materialized_containers_payload(
    units: &[ProjectCompiledUnit],
) -> Vec<serde_json::Value> {
    let mut container_counts = BTreeMap::<String, usize>::new();
    for unit in units {
        for container in &unit.ast.containers {
            *container_counts
                .entry(container.name.to_ascii_lowercase())
                .or_default() += 1;
        }
    }

    units
        .iter()
        .flat_map(|unit| {
            unit.ast.containers.iter().map(|container| {
                let duplicate_source_name = container_counts
                    .get(&container.name.to_ascii_lowercase())
                    .copied()
                    .unwrap_or_default()
                    > 1;
                let materialized_container = if !unit.unit.is_main && duplicate_source_name {
                    format!(
                        "{}__{}",
                        sanitize_unit_symbol(&unit.unit.name),
                        container.name
                    )
                } else {
                    container.name.clone()
                };
                let unit_qualified_prefix = format!("{}.{}", unit.unit.name, container.name);
                json!({
                    "unit": unit.unit.name,
                    "unit_bytes_hex": bytes_to_hex_vec(unit.unit.name.as_bytes()),
                    "source_container": container.name,
                    "source_container_bytes_hex": bytes_to_hex_vec(container.name.as_bytes()),
                    "materialized_container": materialized_container.clone(),
                    "materialized_container_bytes_hex": bytes_to_hex_vec(materialized_container.as_bytes()),
                    "duplicate_source_name": duplicate_source_name,
                    "unit_qualified_prefix": unit_qualified_prefix.clone(),
                    "unit_qualified_prefix_bytes_hex": bytes_to_hex_vec(unit_qualified_prefix.as_bytes())
                })
            })
        })
        .collect()
}

fn project_profile_payload(profile: &ncc_ir::BackendLoweringProfileIR) -> serde_json::Value {
    project_target_spec_payload(&profile.target_spec)
}

fn project_backend_target_payload(units: &[ProjectCompiledUnit]) -> serde_json::Value {
    units
        .iter()
        .filter_map(|unit| unit.object_artifact.as_ref())
        .map(|artifact| project_target_spec_payload(&artifact.object.target_spec))
        .next()
        .unwrap_or_else(|| project_profile_payload(&default_project_backend_profile()))
}

fn project_target_spec_payload(spec: &ncc_ir::BackendTargetSpecIR) -> serde_json::Value {
    json!({
        "target_triple": spec.target_triple,
        "target_family": project_backend_target_family_label(&spec.target_family),
        "isa": project_backend_isa_label(&spec.isa),
        "abi": project_backend_abi_label(&spec.abi),
        "object_format": project_backend_object_format_label(&spec.object_format),
        "relocation_model": project_backend_relocation_model_label(&spec.relocation_model),
        "code_model": project_backend_code_model_label(&spec.code_model),
        "endianness": project_backend_endianness_label(&spec.endianness),
        "pointer_width": spec.pointer_width,
        "runtime_model": project_backend_runtime_model_label(&spec.runtime_model),
        "host_services": project_backend_host_services_label(&spec.host_services),
        "debug_info_model": project_backend_debug_info_model_label(&spec.debug_info_model),
        "linker_model": project_backend_linker_model_label(&spec.linker_model),
        "support_tier": project_backend_support_tier_label(&spec.support_tier),
        "backend_kind": project_backend_kind_label(&spec.backend_kind),
    })
}

fn project_object_abi_payload(object: &ncc_ir::BackendObjectIR) -> serde_json::Value {
    let spec = &object.target_spec;
    json!({
        "target": project_backend_target_label(&object.target),
        "triple": spec.target_triple,
        "target_family": project_backend_target_family_label(&spec.target_family),
        "isa": project_backend_isa_label(&spec.isa),
        "model": project_backend_abi_label(&spec.abi),
        "object_format": project_backend_object_format_label(&spec.object_format),
        "relocation_model": project_backend_relocation_model_label(&spec.relocation_model),
        "code_model": project_backend_code_model_label(&spec.code_model),
        "endianness": project_backend_endianness_label(&spec.endianness),
        "pointer_width": spec.pointer_width,
        "runtime_model": project_backend_runtime_model_label(&spec.runtime_model),
        "host_services": project_backend_host_services_label(&spec.host_services),
        "debug_info_model": project_backend_debug_info_model_label(&spec.debug_info_model),
        "linker_model": project_backend_linker_model_label(&spec.linker_model),
        "support_tier": project_backend_support_tier_label(&spec.support_tier),
        "backend_kind": project_backend_kind_label(&spec.backend_kind),
        "target_spec": project_target_spec_payload(spec),
    })
}

fn project_object_symbols_payload(object: &ncc_ir::BackendObjectIR) -> Vec<serde_json::Value> {
    object
        .symbols
        .iter()
        .map(project_object_symbol_payload)
        .collect()
}

fn project_object_exported_symbols_payload(
    object: &ncc_ir::BackendObjectIR,
) -> Vec<serde_json::Value> {
    object
        .symbols
        .iter()
        .filter(|symbol| matches!(symbol.visibility, ncc_ir::BackendSymbolVisibilityIR::Global))
        .map(project_object_symbol_payload)
        .collect()
}

fn project_object_symbol_payload(symbol: &ncc_ir::BackendObjectSymbolIR) -> serde_json::Value {
    json!({
        "name": symbol.name,
        "section": symbol.section,
        "offset": symbol.offset,
        "visibility": project_object_symbol_visibility_label(&symbol.visibility)
    })
}

fn project_object_sections_payload(object: &ncc_ir::BackendObjectIR) -> Vec<serde_json::Value> {
    object
        .sections
        .iter()
        .map(|section| {
            let payload_bytes = section.bytes.clone().unwrap_or_else(|| {
                if section.lines.is_empty() {
                    Vec::new()
                } else {
                    section.lines.join("\n").into_bytes()
                }
            });
            json!({
                "name": section.name,
                "kind": project_object_section_kind_label(&section.kind),
                "unit": section.unit,
                "size_bytes": section.size_bytes,
                "payload_byte_count": payload_bytes.len(),
                "payload_bytes_hex": bytes_to_hex_vec(&payload_bytes),
                "line_count": section.lines.len()
            })
        })
        .collect()
}

fn project_object_relocations_payload(object: &ncc_ir::BackendObjectIR) -> Vec<serde_json::Value> {
    object
        .relocations
        .iter()
        .map(|relocation| {
            json!({
                "section": relocation.section,
                "offset": relocation.offset,
                "kind": project_object_relocation_kind_label(&relocation.kind),
                "symbol": relocation.symbol,
                "addend": relocation.addend
            })
        })
        .collect()
}

fn project_use_paths_from_ast(ast: &ncc_ast::Program) -> Vec<String> {
    let mut uses = BTreeSet::new();
    for use_decl in &ast.uses {
        uses.insert(use_decl.path.join("."));
    }
    for container in &ast.containers {
        for section in &container.sections {
            if let ncc_ast::Section::Use(use_decls) = section {
                for use_decl in use_decls {
                    uses.insert(use_decl.path.join("."));
                }
            }
        }
    }
    uses.into_iter().collect()
}

fn project_import_targets_from_ast(ast: &ncc_ast::Program) -> Vec<String> {
    let mut targets = BTreeSet::new();
    for use_decl in &ast.uses {
        if let Some(target) = project_import_target(&use_decl.path) {
            targets.insert(target);
        }
    }
    for container in &ast.containers {
        for section in &container.sections {
            if let ncc_ast::Section::Use(use_decls) = section {
                for use_decl in use_decls {
                    if let Some(target) = project_import_target(&use_decl.path) {
                        targets.insert(target);
                    }
                }
            }
        }
    }
    targets.into_iter().collect()
}

fn project_backend_target_label(target: &ncc_ir::BackendTargetIR) -> &'static str {
    match target {
        ncc_ir::BackendTargetIR::X86_64 => "x86_64",
        ncc_ir::BackendTargetIR::AArch64 => "aarch64",
        ncc_ir::BackendTargetIR::Wasm32 => "wasm32",
    }
}

fn project_backend_target_family_label(family: &ncc_ir::BackendTargetFamilyIR) -> &'static str {
    match family {
        ncc_ir::BackendTargetFamilyIR::Hosted => "hosted",
        ncc_ir::BackendTargetFamilyIR::Portable => "portable",
        ncc_ir::BackendTargetFamilyIR::Freestanding => "freestanding",
        ncc_ir::BackendTargetFamilyIR::Sovereign => "sovereign",
    }
}

fn project_backend_isa_label(isa: &ncc_ir::BackendIsaIR) -> &'static str {
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

fn project_backend_abi_label(abi: &ncc_ir::BackendAbiModelIR) -> &'static str {
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
        ncc_ir::BackendAbiModelIR::NcSovereign => "nC.sovereign",
    }
}

fn project_backend_object_format_label(format: &ncc_ir::BackendObjectFormatIR) -> &'static str {
    match format {
        ncc_ir::BackendObjectFormatIR::Elf32 => "elf32",
        ncc_ir::BackendObjectFormatIR::Elf64 => "elf64",
        ncc_ir::BackendObjectFormatIR::Coff => "coff",
        ncc_ir::BackendObjectFormatIR::MachO => "mach-o",
        ncc_ir::BackendObjectFormatIR::WasmModule => "wasm-module",
        ncc_ir::BackendObjectFormatIR::FreestandingImage => "freestanding-image",
    }
}

fn project_backend_relocation_model_label(
    model: &ncc_ir::BackendRelocationModelIR,
) -> &'static str {
    match model {
        ncc_ir::BackendRelocationModelIR::Static => "static",
        ncc_ir::BackendRelocationModelIR::Pic => "pic",
        ncc_ir::BackendRelocationModelIR::DynamicNoPic => "dynamic-no-pic",
        ncc_ir::BackendRelocationModelIR::Absolute => "absolute",
    }
}

fn project_backend_code_model_label(model: &ncc_ir::BackendCodeModelIR) -> &'static str {
    match model {
        ncc_ir::BackendCodeModelIR::Small => "small",
        ncc_ir::BackendCodeModelIR::Kernel => "kernel",
        ncc_ir::BackendCodeModelIR::Large => "large",
    }
}

fn project_backend_endianness_label(endianness: &ncc_ir::BackendEndiannessIR) -> &'static str {
    match endianness {
        ncc_ir::BackendEndiannessIR::Little => "little",
        ncc_ir::BackendEndiannessIR::Big => "big",
    }
}

fn project_backend_runtime_model_label(model: &ncc_ir::BackendRuntimeModelIR) -> &'static str {
    match model {
        ncc_ir::BackendRuntimeModelIR::Hosted => "hosted",
        ncc_ir::BackendRuntimeModelIR::Portable => "portable",
        ncc_ir::BackendRuntimeModelIR::Freestanding => "freestanding",
        ncc_ir::BackendRuntimeModelIR::Sovereign => "sovereign",
    }
}

fn project_backend_host_services_label(services: &ncc_ir::BackendHostServicesIR) -> &'static str {
    match services {
        ncc_ir::BackendHostServicesIR::Os => "os",
        ncc_ir::BackendHostServicesIR::Wasi => "wasi",
        ncc_ir::BackendHostServicesIR::BareMetal => "bare-metal",
        ncc_ir::BackendHostServicesIR::SovereignBridge => "sovereign-bridge",
    }
}

fn project_backend_debug_info_model_label(model: &ncc_ir::BackendDebugInfoModelIR) -> &'static str {
    match model {
        ncc_ir::BackendDebugInfoModelIR::Dwarf => "dwarf",
        ncc_ir::BackendDebugInfoModelIR::Pdb => "pdb",
        ncc_ir::BackendDebugInfoModelIR::Compact => "compact",
        ncc_ir::BackendDebugInfoModelIR::None => "none",
    }
}

fn project_backend_linker_model_label(model: &ncc_ir::BackendLinkerModelIR) -> &'static str {
    match model {
        ncc_ir::BackendLinkerModelIR::HostNative => "host-native",
        ncc_ir::BackendLinkerModelIR::WasmLld => "wasm-lld",
        ncc_ir::BackendLinkerModelIR::FreestandingInternal => "freestanding-internal",
        ncc_ir::BackendLinkerModelIR::SovereignInternal => "sovereign-internal",
    }
}

fn project_backend_support_tier_label(tier: &ncc_ir::BackendSupportTierIR) -> &'static str {
    match tier {
        ncc_ir::BackendSupportTierIR::Tier1 => "tier1",
        ncc_ir::BackendSupportTierIR::Tier2 => "tier2",
        ncc_ir::BackendSupportTierIR::Tier3 => "tier3",
    }
}

fn project_backend_kind_label(kind: &ncc_ir::BackendKindIR) -> &'static str {
    match kind {
        ncc_ir::BackendKindIR::HostedProduction => "hosted-production",
        ncc_ir::BackendKindIR::Portable => "portable",
        ncc_ir::BackendKindIR::Freestanding => "freestanding",
        ncc_ir::BackendKindIR::SovereignNc => "sovereign-nc",
    }
}

fn project_object_section_kind_label(kind: &ncc_ir::BackendObjectSectionKindIR) -> &'static str {
    match kind {
        ncc_ir::BackendObjectSectionKindIR::Text => "text",
        ncc_ir::BackendObjectSectionKindIR::Machine => "machine",
        ncc_ir::BackendObjectSectionKindIR::Data => "data",
        ncc_ir::BackendObjectSectionKindIR::Bss => "bss",
        ncc_ir::BackendObjectSectionKindIR::ReadOnlyData => "rodata",
        ncc_ir::BackendObjectSectionKindIR::TlsData => "tls-data",
        ncc_ir::BackendObjectSectionKindIR::TlsBss => "tls-bss",
        ncc_ir::BackendObjectSectionKindIR::PerCpuData => "percpu-data",
        ncc_ir::BackendObjectSectionKindIR::PerCpuBss => "percpu-bss",
        ncc_ir::BackendObjectSectionKindIR::InitArray => "init-array",
        ncc_ir::BackendObjectSectionKindIR::FiniArray => "fini-array",
        ncc_ir::BackendObjectSectionKindIR::PrimitiveMeta => "primitive-meta",
        ncc_ir::BackendObjectSectionKindIR::TypeMeta => "type-meta",
    }
}

fn project_object_symbol_visibility_label(
    visibility: &ncc_ir::BackendSymbolVisibilityIR,
) -> &'static str {
    match visibility {
        ncc_ir::BackendSymbolVisibilityIR::Local => "local",
        ncc_ir::BackendSymbolVisibilityIR::Global => "global",
        ncc_ir::BackendSymbolVisibilityIR::Weak => "weak",
        ncc_ir::BackendSymbolVisibilityIR::Common => "common",
        ncc_ir::BackendSymbolVisibilityIR::LinkOnceOdr => "linkonce-odr",
    }
}

fn project_object_relocation_kind_label(
    kind: &ncc_ir::BackendObjectRelocationKindIR,
) -> &'static str {
    match kind {
        ncc_ir::BackendObjectRelocationKindIR::CallNear32 => "call-near32",
        ncc_ir::BackendObjectRelocationKindIR::JumpNear32 => "jump-near32",
        ncc_ir::BackendObjectRelocationKindIR::PcRel32 => "pc-rel32",
        ncc_ir::BackendObjectRelocationKindIR::Abs32 => "abs32",
        ncc_ir::BackendObjectRelocationKindIR::Abs64 => "abs64",
    }
}

fn print_compile_report(
    action: &str,
    opts: &ProjectCommandOptions,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let object_artifacts = compiled
        .units
        .iter()
        .filter(|unit| unit.object_artifact.is_some())
        .count();
    let ast_cache_hits = compiled
        .units
        .iter()
        .filter(|unit| unit.ast_cache_status == "hit")
        .count();
    let object_byte_count = compiled
        .units
        .iter()
        .filter_map(|unit| unit.object_artifact.as_ref())
        .map(|artifact| artifact.bytes.len())
        .sum::<usize>();
    let payload = json!({
        "schema": "ncc.project.priority13b.report.v1",
        "status": "ok",
        "action": action,
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "units": compiled.units.len(),
        "edges": compiled.edges.len(),
        "linked_containers": compiled.linked_program.containers.len(),
        "lowering_dir": ".ncc/lowering",
        "object_dir": ".ncc/obj",
        "object_artifacts": object_artifacts,
        "ast_cache_hits": ast_cache_hits,
        "object_byte_count": object_byte_count,
        "linked_backend_byte_count": compiled.linked_backend_debug.len()
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "OK: project {} target={} format={} route=project-bridge-interpreter units={} edges={} linked_containers={} ast_cache_hits={} lowering=.ncc/lowering obj=.ncc/obj",
            action,
            opts.profile.target_spec.target_triple,
            project_backend_object_format_label(&opts.profile.target_spec.object_format),
            compiled.units.len(),
            compiled.edges.len(),
            compiled.linked_program.containers.len(),
            ast_cache_hits
        );
    }
    Ok(())
}

fn print_cached_check_report(
    opts: &ProjectCommandOptions,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    if opts.json {
        println!("{}", serde_json::to_string_pretty(payload)?);
    } else {
        println!(
            "OK: project check target={} format={} units={} edges={} linked_containers={} lowering=.ncc/lowering obj=.ncc/obj semantic_validation=skipped_linked_clean_cache",
            opts.profile.target_spec.target_triple,
            project_backend_object_format_label(&opts.profile.target_spec.object_format),
            payload.get("units").and_then(|value| value.as_u64()).unwrap_or(0),
            payload.get("edges").and_then(|value| value.as_u64()).unwrap_or(0),
            payload
                .get("linked_containers")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        );
    }
    Ok(())
}

fn print_project_graph_report(
    opts: &ProjectCommandOptions,
    compiled: &ProjectCompileResult,
) -> Result<(), Box<dyn Error>> {
    let materialized_containers = project_materialized_containers_payload(&compiled.units);
    let payload = json!({
        "schema": "ncc.project.priority13g.graph.v1",
        "status": "ok",
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "target": project_profile_payload(&opts.profile),
        "object_dir": ".ncc/obj",
        "units": compiled.units.iter().map(|unit| {
            json!({
                "name": unit.unit.name,
                "source": unit.unit.relative_source,
                "hash": unit.hash,
                "containers": unit.container_count,
                "imports": unit.import_count,
                "object": unit.object_artifact.as_ref().map(project_object_summary_payload)
            })
        }).collect::<Vec<_>>(),
        "edges": compiled.edges.iter().map(|(from, to)| json!([from, to])).collect::<Vec<_>>(),
        "linked_containers": compiled.linked_program.containers.len(),
        "project_linker": {
            "status": "closed_unit_namespace_materialization",
            "materialized_containers": materialized_containers
        }
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "OK: project graph target={} format={} units={} edges={} linked_containers={} obj=.ncc/obj",
            opts.profile.target_spec.target_triple,
            project_backend_object_format_label(&opts.profile.target_spec.object_format),
            compiled.units.len(),
            compiled.edges.len(),
            compiled.linked_program.containers.len()
        );
        if let Some(items) = payload
            .pointer("/project_linker/materialized_containers")
            .and_then(|value| value.as_array())
        {
            for item in items {
                let prefix = item
                    .get("unit_qualified_prefix")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let materialized = item
                    .get("materialized_container")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let duplicate = item
                    .get("duplicate_source_name")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                println!("{prefix} -> {materialized} duplicate={duplicate}");
            }
        }
    }
    Ok(())
}

fn ensure_project_dirs(root: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(root)?;
    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join(".ncc").join("code"))?;
    fs::create_dir_all(root.join(".ncc").join("lowering"))?;
    fs::create_dir_all(root.join(".ncc").join("obj"))?;
    Ok(())
}

fn collect_project_sources(root: &Path) -> Result<Vec<ProjectSourceUnit>, Box<dyn Error>> {
    let src_dir = root.join("src");
    let mut paths = Vec::new();
    collect_nc_source_paths(&src_dir, &mut paths)?;
    paths.sort();

    let mut units = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path)?;
        if !is_code_json_eligible_source(&path, &source) {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or("project: source file without valid stem")?
            .to_string();
        let hash = sha256_text(&source);
        let relative_source = relative_slash_path(root, &path)?;
        units.push(ProjectSourceUnit {
            name: stem,
            source: path,
            relative_source,
            hash,
            code_relative: String::new(),
        });
    }
    Ok(units)
}

fn collect_nc_source_paths(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_nc_source_paths(&path, out)?;
            continue;
        }
        let is_n = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("n") || ext.eq_ignore_ascii_case("nc"))
            .unwrap_or(false);
        if is_n {
            out.push(path);
        }
    }
    Ok(())
}

fn is_code_json_eligible_source(path: &Path, source: &str) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    !lower.ends_with(".nb")
        && !lower.ends_with(".nm")
        && !lower.ends_with(".ncp")
        && !source.contains("[@machine]")
        && !source.contains("primitive[@]")
}

fn assign_duplicate_safe_unit_names(units: &mut [ProjectSourceUnit]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for unit in units.iter() {
        *counts.entry(unit.name.to_ascii_lowercase()).or_default() += 1;
    }
    let mut seen = BTreeSet::<String>::new();
    for unit in units {
        let key = unit.name.to_ascii_lowercase();
        if counts.get(&key).copied().unwrap_or(0) > 1 {
            let suffix = unit.hash.trim_start_matches("sha256:");
            unit.name = format!("{}_{}", unit.name, &suffix[..8.min(suffix.len())]);
        }
        if !seen.insert(unit.name.to_ascii_lowercase()) {
            let suffix = unit.hash.trim_start_matches("sha256:");
            unit.name = format!("{}_{}", unit.name, &suffix[..12.min(suffix.len())]);
        }
        unit.code_relative = format!(".ncc/code/{}.json", unit.name);
    }
}

fn project_main_source_unit(
    opts: &ProjectCommandOptions,
) -> Result<ProjectSourceUnit, Box<dyn Error>> {
    let source = opts.root.join(&opts.main_name);
    let text = fs::read_to_string(&source)?;
    Ok(ProjectSourceUnit {
        name: "main".to_string(),
        source,
        relative_source: opts.main_name.clone(),
        hash: sha256_text(&text),
        code_relative: ".ncc/code/main.json".to_string(),
    })
}

fn write_code_json_files(root: &Path, units: &[ProjectSourceUnit]) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing code identity files")?;
    for unit in units {
        let payload = json!({
            "schema": "ncc.code.v1",
            "unit": unit.name,
            "source": unit.relative_source,
            "hash": unit.hash,
            "bridge": "../../bridge.json"
        });
        write_json(&root.join(&unit.code_relative), &payload)?;
    }
    Ok(())
}

pub(crate) fn materialize_single_file_code_json(source_path: &Path) -> Result<(), Box<dyn Error>> {
    let absolute_source = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(source_path)
    };
    let absolute_source = fs::canonicalize(&absolute_source).unwrap_or(absolute_source);
    if slash_path(&absolute_source)
        .to_ascii_lowercase()
        .contains("/_archive/old_examples/")
    {
        return Ok(());
    }
    let source = fs::read_to_string(&absolute_source)?;
    if !is_code_json_eligible_source(&absolute_source, &source) {
        return Ok(());
    }
    let Some(root) = absolute_source.parent().map(Path::to_path_buf) else {
        return Ok(());
    };
    let unit_name = file_stem_unit_name(&absolute_source)?;
    let relative_source = relative_slash_path(&root, &absolute_source)
        .unwrap_or_else(|_| slash_path(&absolute_source));
    let unit = ProjectSourceUnit {
        name: unit_name.clone(),
        source: absolute_source,
        relative_source,
        hash: sha256_text(&source),
        code_relative: format!(".ncc/code/{}.json", sanitize_unit_symbol(&unit_name)),
    };
    write_code_json_files(&root, &[unit])
}

fn collect_project_edges(
    root: &Path,
    main_name: &str,
    units: &[ProjectSourceUnit],
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut edges = BTreeSet::<(String, String)>::new();
    let main_path = root.join(main_name);
    if main_path.is_file() {
        collect_project_edges_from_file(&main_path, "main", &mut edges)?;
    }
    for unit in units {
        collect_project_edges_from_file(&unit.source, &unit.name, &mut edges)?;
    }
    Ok(edges.into_iter().collect())
}

fn validate_project_edges_known(
    edges: &[(String, String)],
    units: &[ProjectSourceUnit],
) -> Result<(), Box<dyn Error>> {
    let mut known = BTreeSet::from(["main".to_string()]);
    for unit in units {
        known.insert(unit.name.to_ascii_lowercase());
    }
    for (from, to) in edges {
        if !known.contains(&from.to_ascii_lowercase()) {
            return Err(format!(
                "E_PROJECT_BRIDGE_EDGE_002: bridge edge source '{}' is not a known unit",
                from
            )
            .into());
        }
        if !known.contains(&to.to_ascii_lowercase()) {
            return Err(format!(
                "E_PROJECT_BRIDGE_EDGE_003: bridge edge target '{}' is not a known unit",
                to
            )
            .into());
        }
    }
    Ok(())
}

fn collect_project_edges_from_file(
    path: &Path,
    from: &str,
    edges: &mut BTreeSet<(String, String)>,
) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    if source.trim().is_empty() {
        return Ok(());
    }
    let tokens = ncc_lexer::lex(&source)?;
    let program = ncc_parser::parse(tokens)?;
    for target in collect_project_imports_from_program(&program) {
        edges.insert((from.to_string(), target));
    }
    Ok(())
}

fn project_import_target(path: &[String]) -> Option<String> {
    if path.len() >= 2 && path[0].eq_ignore_ascii_case("project") {
        Some(path[1..].join("."))
    } else {
        None
    }
}

fn write_bridge_json(
    root: &Path,
    main_unit: &ProjectSourceUnit,
    units: &[ProjectSourceUnit],
    edges: &[(String, String)],
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> Result<(), Box<dyn Error>> {
    let _lock = acquire_project_cache_lock(root, "writing bridge manifest")?;
    let physical_units = build_project_physical_units(root, main_unit, units, edges, profile)?;
    let unit_names = physical_units
        .iter()
        .map(|unit| unit.name.clone())
        .collect::<Vec<_>>();
    let link_order = compute_project_link_order(&unit_names, edges);
    let unit_values = physical_units
        .iter()
        .map(|unit| {
            json!({
                "name": unit.name,
                "source": unit.relative_source,
                "code": unit.code_relative,
                "source_hash": unit.source_hash,
                "object": unit.object_relative,
                "object_hash": unit.object_hash,
                "symbols": unit.symbols,
                "imports": unit.imports,
                "deps": unit.deps
            })
        })
        .collect::<Vec<_>>();
    let edge_values = edges
        .iter()
        .map(|(from, to)| json!([from, to]))
        .collect::<Vec<_>>();
    let payload = json!({
        "schema": "ncc.bridge.v1",
        "linker": "ncc.project.physical_linker.v1",
        "main": main_unit.relative_source,
        "object_dir": ".ncc/obj",
        "target": project_profile_payload(profile),
        "units": unit_values,
        "edges": edge_values,
        "link_order": link_order
    });
    write_json(&root.join("bridge.json"), &payload)?;
    Ok(())
}

fn build_project_physical_units(
    root: &Path,
    main_unit: &ProjectSourceUnit,
    units: &[ProjectSourceUnit],
    edges: &[(String, String)],
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> Result<Vec<ProjectPhysicalUnit>, Box<dyn Error>> {
    fs::create_dir_all(root.join(project_target_object_dir(&profile.target_spec)))?;
    let mut source_units = Vec::with_capacity(units.len() + 1);
    source_units.push(read_project_physical_source_unit(main_unit, edges)?);
    for unit in units {
        source_units.push(read_project_physical_source_unit(unit, edges)?);
    }

    let container_counts = project_physical_container_counts(&source_units);
    let materialized_names =
        project_physical_container_materialized_names(&source_units, &container_counts);
    let known_units = source_units
        .iter()
        .map(|unit| unit.unit.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let mut physical_units = Vec::with_capacity(source_units.len());
    for source_unit in source_units {
        physical_units.push(build_project_physical_unit(
            root,
            source_unit,
            &container_counts,
            &materialized_names,
            &known_units,
            profile,
        )?);
    }
    Ok(physical_units)
}

fn read_project_physical_source_unit(
    unit: &ProjectSourceUnit,
    edges: &[(String, String)],
) -> Result<ProjectPhysicalSourceUnit, Box<dyn Error>> {
    let source = fs::read_to_string(&unit.source)?;
    let (ast, imports, symbols) = if source.trim().is_empty() {
        (None, Vec::new(), Vec::new())
    } else {
        let tokens = ncc_lexer::lex(&source)?;
        let program = ncc_parser::parse(tokens)?;
        let imports = collect_project_imports_from_program(&program)
            .into_iter()
            .collect::<Vec<_>>();
        let symbols = collect_project_symbols_from_program(&program);
        (Some(program), imports, symbols)
    };
    let deps = project_deps_for_unit(&unit.name, edges);
    if string_set(&imports) != string_set(&deps) {
        return Err(format!(
            "E_PROJECT_BRIDGE_IMPORTS_001: unit '{}' imports must match bridge edges",
            unit.name
        )
        .into());
    }
    Ok(ProjectPhysicalSourceUnit {
        unit: unit.clone(),
        ast,
        imports,
        symbols,
        deps,
    })
}

fn build_project_physical_unit(
    root: &Path,
    source_unit: ProjectPhysicalSourceUnit,
    container_counts: &BTreeMap<String, usize>,
    materialized_names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> Result<ProjectPhysicalUnit, Box<dyn Error>> {
    let (object_relative, object_hash) = project_unit_physical_object(
        root,
        &source_unit,
        container_counts,
        materialized_names,
        known_units,
        profile,
    )?;
    Ok(ProjectPhysicalUnit {
        name: source_unit.unit.name.clone(),
        relative_source: source_unit.unit.relative_source.clone(),
        code_relative: Some(source_unit.unit.code_relative.clone()),
        source_hash: source_unit.unit.hash.clone(),
        object_relative,
        object_hash,
        symbols: source_unit.symbols,
        imports: source_unit.imports,
        deps: source_unit.deps,
    })
}

fn project_unit_physical_object(
    root: &Path,
    source_unit: &ProjectPhysicalSourceUnit,
    _container_counts: &BTreeMap<String, usize>,
    _materialized_names: &BTreeMap<(String, String), String>,
    _known_units: &BTreeSet<String>,
    profile: &ncc_ir::BackendLoweringProfileIR,
) -> Result<(String, String), Box<dyn Error>> {
    let unit = &source_unit.unit;
    let object_relative = project_empty_object_relative_path(&unit.name, &profile.target_spec);
    let payload = project_sync_contract_object_payload(
        unit,
        &object_relative,
        &source_unit.symbols,
        &source_unit.imports,
    );
    let text = serde_json::to_string_pretty(&payload)? + "\n";
    let bytes = text.as_bytes().to_vec();
    let object_path = root.join(&object_relative);
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_text(&object_path, &text)?;
    Ok((object_relative, sha256_bytes(&bytes)))
}

fn project_physical_container_counts(
    units: &[ProjectPhysicalSourceUnit],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for unit in units {
        if let Some(ast) = &unit.ast {
            for container in &ast.containers {
                *counts
                    .entry(container.name.to_ascii_lowercase())
                    .or_default() += 1;
            }
        }
    }
    counts
}

fn project_physical_container_materialized_names(
    units: &[ProjectPhysicalSourceUnit],
    container_counts: &BTreeMap<String, usize>,
) -> BTreeMap<(String, String), String> {
    let mut names = BTreeMap::new();
    for unit in units {
        let Some(ast) = &unit.ast else {
            continue;
        };
        let unit_key = unit.unit.name.to_ascii_lowercase();
        let is_main = unit.unit.name.eq_ignore_ascii_case("main");
        for container in &ast.containers {
            let container_key = container.name.to_ascii_lowercase();
            let is_duplicate = container_counts
                .get(&container_key)
                .copied()
                .unwrap_or_default()
                > 1;
            let materialized = if !is_main && is_duplicate {
                format!(
                    "{}__{}",
                    sanitize_unit_symbol(&unit.unit.name),
                    container.name
                )
            } else {
                container.name.clone()
            };
            names.insert((unit_key.clone(), container_key), materialized);
        }
    }
    names
}

fn project_materialized_source_program(
    unit_name: &str,
    is_main: bool,
    ast: &ncc_ast::Program,
    container_counts: &BTreeMap<String, usize>,
    materialized_names: &BTreeMap<(String, String), String>,
    known_units: &BTreeSet<String>,
) -> Result<ncc_ast::Program, Box<dyn Error>> {
    let mut ast = ast.clone();
    let diagnostics =
        rewrite_project_unit_qualified_program(&mut ast, materialized_names, known_units);
    if !diagnostics.is_empty() {
        return Err(diagnostics.join("\n").into());
    }
    if !is_main {
        for container in &mut ast.containers {
            container
                .sections
                .retain(|section| !matches!(section, ncc_ast::Section::MainEntry { .. }));
            if container_counts
                .get(&container.name.to_ascii_lowercase())
                .copied()
                .unwrap_or(0)
                > 1
            {
                container.name = format!("{}__{}", sanitize_unit_symbol(unit_name), container.name);
            }
        }
    }
    Ok(ast)
}

fn project_sync_contract_object_payload(
    unit: &ProjectSourceUnit,
    object_relative: &str,
    symbols: &[serde_json::Value],
    imports: &[String],
) -> serde_json::Value {
    json!({
        "schema": "ncc.project.object.v1",
        "status": "sync_contract",
        "kind": "project_object_contract",
        "unit": unit.name,
        "source": unit.relative_source,
        "source_hash": unit.hash,
        "object": object_relative,
        "bridge": "../../bridge.json",
        "code": unit.code_relative,
        "symbols": symbols,
        "bridge_symbols": symbols,
        "imports": imports,
        "bridge_imports": imports,
        "deps": imports,
        "bridge_deps": imports,
        "object_bytes": {
            "status": "pending_backend_object",
            "note": "project check/run/graph replace this contract with backend object bytes and a sidecar"
        }
    })
}

fn project_empty_object_relative_path(
    unit_name: &str,
    spec: &ncc_ir::BackendTargetSpecIR,
) -> String {
    format!(
        "{}/{}.{}",
        project_target_object_dir(spec),
        sanitize_unit_symbol(unit_name),
        project_object_extension(&spec.object_format)
    )
}

fn project_deps_for_unit(unit_name: &str, edges: &[(String, String)]) -> Vec<String> {
    edges
        .iter()
        .filter(|(from, _)| from.eq_ignore_ascii_case(unit_name))
        .map(|(_, to)| to.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn compute_project_link_order(unit_names: &[String], edges: &[(String, String)]) -> Vec<String> {
    let mut name_by_key = BTreeMap::new();
    for name in unit_names {
        name_by_key.insert(name.to_ascii_lowercase(), name.clone());
    }
    let mut deps = BTreeMap::<String, BTreeSet<String>>::new();
    for (from, to) in edges {
        deps.entry(from.to_ascii_lowercase())
            .or_default()
            .insert(to.to_ascii_lowercase());
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for name in unit_names {
        compute_project_link_order_visit(
            &name.to_ascii_lowercase(),
            &deps,
            &name_by_key,
            &mut visiting,
            &mut visited,
            &mut order,
        );
    }
    order
}

fn compute_project_link_order_visit(
    unit_key: &str,
    deps: &BTreeMap<String, BTreeSet<String>>,
    name_by_key: &BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    if visited.contains(unit_key) || !name_by_key.contains_key(unit_key) {
        return;
    }
    if !visiting.insert(unit_key.to_string()) {
        return;
    }
    if let Some(unit_deps) = deps.get(unit_key) {
        for dep in unit_deps {
            compute_project_link_order_visit(dep, deps, name_by_key, visiting, visited, order);
        }
    }
    visiting.remove(unit_key);
    if visited.insert(unit_key.to_string()) {
        if let Some(name) = name_by_key.get(unit_key) {
            order.push(name.clone());
        }
    }
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    write_json_unlocked(path, value)
}

fn write_json_unlocked(path: &Path, value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let text = serde_json::to_string_pretty(value)?;
    write_text(path, &(text + "\n"))
}

fn write_text(path: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() == bytes.len() as u64 {
            if let Ok(existing) = fs::read(path) {
                if existing == bytes {
                    return Ok(());
                }
            }
        }
    }
    atomic_write_bytes(path, bytes)?;
    Ok(())
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "E_PROJECT_ATOMIC_WRITE_001: path '{}' has no parent directory",
            slash_path(path)
        )
        .into());
    };
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let seq = PROJECT_ATOMIC_WRITE_SEQ.fetch_add(1, Ordering::SeqCst);
    let tmp_path = parent.join(format!(".{}.{}.{}.tmp", file_name, std::process::id(), seq));
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.flush()?;
        let _ = file.sync_all();
    }
    match fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(_err) if cfg!(windows) && path.exists() => {
            fs::remove_file(path)?;
            fs::rename(&tmp_path, path)?;
            Ok(())
        }
        Err(err) => {
            let _ = fs::remove_file(&tmp_path);
            Err(Box::new(err))
        }
    }
}

fn project_cache_lock_report(root: &Path) -> serde_json::Value {
    let lock_dir = root.join(".ncc").join("locks").join("project-cache.lock");
    json!({
        "schema": "ncc.project.cache_lock.report.v1",
        "status": if lock_dir.exists() { "held" } else { "free" },
        "lock_dir": slash_path(&lock_dir),
        "timeout_ms": env::var("NCC_CACHE_LOCK_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(PROJECT_CACHE_LOCK_TIMEOUT_MS),
        "stale_after_seconds": PROJECT_CACHE_LOCK_STALE_SECS,
        "atomic_write": true
    })
}

fn write_project_cache_lock_report(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = root.join(".ncc").join("locks").join("last_report.json");
    let payload = project_cache_lock_report(root);
    write_json_unlocked(&path, &payload)?;
    Ok(())
}

fn sha256_text(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    format!("sha256:{digest:x}")
}

fn sha256_digest_bytes(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        bytes_to_hex_joined(&sha256_digest_bytes(bytes), "")
    )
}

fn bytes_to_hex_joined(bytes: &[u8], separator: &str) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(separator)
}

fn bytes_to_hex_vec(bytes: &[u8]) -> Vec<String> {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn source_chars_payload(source: &str) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for (char_index, (byte_start, ch)) in source.char_indices().enumerate() {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        out.push(json!({
            "char_index": char_index,
            "byte_start": byte_start,
            "scalar": format!("U+{:04X}", ch as u32),
            "text": ch.to_string(),
            "utf8_bytes_hex": bytes_to_hex_vec(encoded.as_bytes())
        }));
    }
    out
}

fn relative_slash_path(root: &Path, path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(slash_path(path.strip_prefix(root)?))
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn print_project_report(
    action: &str,
    opts: &ProjectCommandOptions,
    units: usize,
    edges: &[(String, String)],
    as_json: bool,
) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ncc.project.priority13a.report.v1",
        "status": "ok",
        "action": action,
        "root": slash_path(&opts.root),
        "main": opts.main_name,
        "bridge": "bridge.json",
        "code_dir": ".ncc/code",
        "lowering_dir": ".ncc/lowering",
        "units": units,
        "edges": edges.len()
    });
    if as_json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "OK: project {} root={} units={} edges={}",
            action,
            slash_path(&opts.root),
            units,
            edges.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod stage2_project_tests {
    use super::*;

    fn bridge_unit(name: &str, deps: &[&str], is_main: bool) -> ProjectBridgeUnit {
        ProjectBridgeUnit {
            name: name.to_string(),
            source: PathBuf::from(format!("{name}.n")),
            relative_source: format!("{name}.n"),
            code_relative: None,
            source_hash: "sha256:test".to_string(),
            object_relative: format!(".ncc/obj/{name}.obj"),
            object_hash: "sha256:test".to_string(),
            symbols: Vec::new(),
            imports: deps.iter().map(|value| (*value).to_string()).collect(),
            deps: deps.iter().map(|value| (*value).to_string()).collect(),
            is_main,
        }
    }

    #[test]
    fn runtime_graph_omits_units_unreachable_from_main() {
        let manifest = ProjectBridgeManifest {
            units: vec![
                bridge_unit("main", &["worker"], true),
                bridge_unit("worker", &["shared"], false),
                bridge_unit("shared", &[], false),
                bridge_unit("unused", &[], false),
            ],
            edges: vec![
                ("main".to_string(), "worker".to_string()),
                ("worker".to_string(), "shared".to_string()),
            ],
            link_order: vec![
                "shared".to_string(),
                "worker".to_string(),
                "main".to_string(),
                "unused".to_string(),
            ],
        };

        let reachable = project_runtime_reachable_unit_keys(&manifest).expect("runtime graph");
        assert_eq!(
            reachable,
            ["main", "shared", "worker"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        assert!(!reachable.contains("unused"));
    }

    #[test]
    fn project_runtime_helpers_are_internalized_without_touching_nc_symbols() {
        let source = "define ptr @__ncc_args_open() {\n}\n\
define i64 @ncc_rt_fs_size(ptr %p) {\n}\n\
define i1 @ghorText.same(ptr %a, ptr %b) {\n}\n";
        let actual = project_internalize_mct_runtime_definitions(source);
        assert!(actual.contains("define internal ptr @__ncc_args_open()"));
        assert!(actual.contains("define internal i64 @ncc_rt_fs_size(ptr %p)"));
        assert!(actual.contains("define i1 @ghorText.same(ptr %a, ptr %b)"));
        assert!(!actual.contains("define internal i1 @ghorText.same"));
    }

    #[test]
    fn combined_nc_runtime_provider_is_semantically_closed() {
        let program = project_nc_runtime_provider_program("x86_64-unknown-linux-gnu")
            .expect("runtime provider parses");
        ncc_sema::analyze(&program).expect("runtime provider sema");
        assert!(program
            .containers
            .iter()
            .any(|container| container.name == "nccJsonRuntime"));
    }

    #[test]
    fn coff_mingw_link_uses_the_compatible_stack_probe_alias() {
        let target = ncc_ir::BackendLoweringProfileIR::nc_win64().target_spec;
        let flags = project_link_flags(&target, "x86_64-w64-mingw32-gcc.exe", 0);
        assert_eq!(flags, ["-Wl,--defsym,__chkstk=___chkstk_ms", "-lshell32"]);
        assert_eq!(
            project_link_flags(&target, "clang.exe", 0),
            ["-lshell32", "-llegacy_stdio_definitions"]
        );
    }

    #[test]
    fn large_coff_projects_keep_headers_before_the_first_image_section() {
        let target = ncc_ir::BackendLoweringProfileIR::nc_win64().target_spec;
        let flags = project_link_flags(&target, "x86_64-w64-mingw32-gcc.exe", 80);
        assert_eq!(project_coff_image_section_alignment(80), 8192);
        assert!(flags.contains(&"-Wl,--section-alignment,8192".to_string()));
    }

    #[test]
    fn windows_command_candidates_include_executable_suffixes() {
        let candidates = command_file_candidates("x86_64-w64-mingw32-gcc");
        if cfg!(windows) {
            assert!(candidates
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case("x86_64-w64-mingw32-gcc.exe")));
        } else {
            assert_eq!(candidates, ["x86_64-w64-mingw32-gcc"]);
        }
    }

    #[test]
    fn clang_discovery_survives_a_missing_path_when_a_known_absolute_binary_exists() {
        if !cfg!(windows) && Path::new("/usr/bin/clang").is_file() {
            assert_eq!(
                project_known_clang_absolute().as_deref(),
                Some("/usr/bin/clang")
            );
        }
    }
}
