use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NpkgSemver {
    major: u64,
    minor: u64,
    patch: u64,
}

impl NpkgSemver {
    fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        let parts = trimmed.split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(format!(
                "invalid semver '{}': expected <major>.<minor>.<patch>",
                raw
            ));
        }
        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| format!("invalid semver '{}': major must be unsigned integer", raw))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| format!("invalid semver '{}': minor must be unsigned integer", raw))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| format!("invalid semver '{}': patch must be unsigned integer", raw))?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NpkgCmpOp {
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NpkgComparator {
    op: NpkgCmpOp,
    version: NpkgSemver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NpkgSemverRequirement {
    Exact(NpkgSemver),
    Caret(NpkgSemver),
    Tilde(NpkgSemver),
    MajorWildcard(u64),
    MajorMinorWildcard(u64, u64),
    ComparatorSet(Vec<NpkgComparator>),
}

impl NpkgSemverRequirement {
    fn parse(raw: &str) -> Result<Self, String> {
        let req = raw.trim();
        if req.is_empty() {
            return Err("invalid semver requirement: empty".to_string());
        }

        if let Some(rest) = req.strip_prefix('^') {
            return Ok(Self::Caret(NpkgSemver::parse(rest)?));
        }
        if let Some(rest) = req.strip_prefix('~') {
            return Ok(Self::Tilde(NpkgSemver::parse(rest)?));
        }

        if req.contains(char::is_whitespace) {
            let mut comparators = Vec::new();
            for token in req.split_whitespace() {
                comparators.push(parse_npkg_comparator(token)?);
            }
            return Ok(Self::ComparatorSet(comparators));
        }

        if let Some(base) = req.strip_suffix(".x").or_else(|| req.strip_suffix(".*")) {
            let parts = base.split('.').collect::<Vec<_>>();
            match parts.as_slice() {
                [major] => {
                    let major = major.parse::<u64>().map_err(|_| {
                        format!(
                            "invalid semver requirement '{}': wildcard major is invalid",
                            raw
                        )
                    })?;
                    return Ok(Self::MajorWildcard(major));
                }
                [major, minor] => {
                    let major = major.parse::<u64>().map_err(|_| {
                        format!(
                            "invalid semver requirement '{}': wildcard major is invalid",
                            raw
                        )
                    })?;
                    let minor = minor.parse::<u64>().map_err(|_| {
                        format!(
                            "invalid semver requirement '{}': wildcard minor is invalid",
                            raw
                        )
                    })?;
                    return Ok(Self::MajorMinorWildcard(major, minor));
                }
                _ => {
                    return Err(format!(
                        "invalid semver requirement '{}': unsupported wildcard shape",
                        raw
                    ));
                }
            }
        }

        if req.starts_with('>') || req.starts_with('<') || req.starts_with('=') {
            return Ok(Self::ComparatorSet(vec![parse_npkg_comparator(req)?]));
        }

        Ok(Self::Exact(NpkgSemver::parse(req)?))
    }

    fn matches(&self, version: NpkgSemver) -> bool {
        match self {
            Self::Exact(base) => version == *base,
            Self::Caret(base) => {
                if version < *base {
                    return false;
                }
                if base.major > 0 {
                    version.major == base.major
                } else if base.minor > 0 {
                    version.major == 0 && version.minor == base.minor
                } else {
                    version.major == 0 && version.minor == 0 && version.patch == base.patch
                }
            }
            Self::Tilde(base) => {
                version >= *base && version.major == base.major && version.minor == base.minor
            }
            Self::MajorWildcard(major) => version.major == *major,
            Self::MajorMinorWildcard(major, minor) => {
                version.major == *major && version.minor == *minor
            }
            Self::ComparatorSet(comparators) => comparators.iter().all(|cmp| match cmp.op {
                NpkgCmpOp::Eq => version == cmp.version,
                NpkgCmpOp::Gt => version > cmp.version,
                NpkgCmpOp::Ge => version >= cmp.version,
                NpkgCmpOp::Lt => version < cmp.version,
                NpkgCmpOp::Le => version <= cmp.version,
            }),
        }
    }
}

fn parse_npkg_comparator(token: &str) -> Result<NpkgComparator, String> {
    let (op, raw_version) = if let Some(raw) = token.strip_prefix(">=") {
        (NpkgCmpOp::Ge, raw)
    } else if let Some(raw) = token.strip_prefix("<=") {
        (NpkgCmpOp::Le, raw)
    } else if let Some(raw) = token.strip_prefix("==") {
        (NpkgCmpOp::Eq, raw)
    } else if let Some(raw) = token.strip_prefix('=') {
        (NpkgCmpOp::Eq, raw)
    } else if let Some(raw) = token.strip_prefix('>') {
        (NpkgCmpOp::Gt, raw)
    } else if let Some(raw) = token.strip_prefix('<') {
        (NpkgCmpOp::Lt, raw)
    } else {
        return Err(format!(
            "invalid semver comparator '{}': expected one of >= <= > < =",
            token
        ));
    };
    let version = NpkgSemver::parse(raw_version)?;
    Ok(NpkgComparator { op, version })
}

#[derive(Debug, Clone)]
struct NpkgRegistryVersion {
    version_raw: String,
    version: NpkgSemver,
    checksum: String,
}

#[derive(Debug, Clone)]
struct NpkgRegistryEntry {
    import_prefix: String,
    package: String,
    requirement: String,
    source: String,
    versions: Vec<NpkgRegistryVersion>,
}

#[derive(Debug, Clone)]
struct NpkgResolvedDependency {
    import: String,
    package: String,
    requirement: String,
    selected_version: String,
    checksum: String,
    source: String,
    registry_import_prefix: String,
}

fn npkg_default_registry() -> serde_json::Value {
    serde_json::json!({
        "schema": "npkg.registry.v1",
        "name": "nC-default-registry",
        "packages": [
            {
                "import": "nC",
                "package": "nC.core",
                "require": "^1.0.0",
                "source": "builtin://registry/nc-core",
                "versions": [
                    { "version": "1.0.0", "checksum": "sha256:nc-core-1.0.0" },
                    { "version": "1.0.1", "checksum": "sha256:nc-core-1.0.1" }
                ]
            },
            {
                "import": "system",
                "package": "nC.system",
                "require": "^1.0.0",
                "source": "builtin://registry/nc-system",
                "versions": [
                    { "version": "1.0.0", "checksum": "sha256:nc-system-1.0.0" },
                    { "version": "1.0.1", "checksum": "sha256:nc-system-1.0.1" }
                ]
            },
            {
                "import": "root",
                "package": "nC.root",
                "require": "^1.0.0",
                "source": "builtin://registry/nc-root",
                "versions": [
                    { "version": "1.0.0", "checksum": "sha256:nc-root-1.0.0" },
                    { "version": "1.0.1", "checksum": "sha256:nc-root-1.0.1" }
                ]
            }
        ]
    })
}

fn parse_npkg_registry(
    registry: &serde_json::Value,
) -> Result<Vec<NpkgRegistryEntry>, Box<dyn std::error::Error>> {
    let packages = registry
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "npkg registry: missing 'packages' array".to_string())?;

    let mut entries = Vec::new();
    for package in packages {
        let import_prefix = package
            .get("import")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "npkg registry: package entry missing 'import'".to_string())?
            .trim()
            .to_string();
        if import_prefix.is_empty() {
            return Err("npkg registry: package entry 'import' cannot be empty".into());
        }
        let package_name = package
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "npkg registry: package entry missing 'package'".to_string())?
            .trim()
            .to_string();
        if package_name.is_empty() {
            return Err("npkg registry: package entry 'package' cannot be empty".into());
        }
        let requirement = package
            .get("require")
            .or_else(|| package.get("range"))
            .and_then(|v| v.as_str())
            .unwrap_or("^1.0.0")
            .trim()
            .to_string();
        if requirement.is_empty() {
            return Err("npkg registry: package entry requirement cannot be empty".into());
        }
        let source = package
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("registry://local")
            .trim()
            .to_string();
        let versions_value = package
            .get("versions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "npkg registry: package entry missing 'versions' array".to_string())?;
        if versions_value.is_empty() {
            return Err(format!(
                "npkg registry: package '{}' must declare at least one version",
                package_name
            )
            .into());
        }
        let mut versions = Vec::new();
        for version_entry in versions_value {
            let version_raw = version_entry
                .get("version")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "npkg registry: version entry missing 'version'".to_string())?
                .trim()
                .to_string();
            let version =
                NpkgSemver::parse(&version_raw).map_err(|err| format!("npkg registry: {}", err))?;
            let checksum = version_entry
                .get("checksum")
                .and_then(|v| v.as_str())
                .unwrap_or("sha256:unknown")
                .trim()
                .to_string();
            versions.push(NpkgRegistryVersion {
                version_raw,
                version,
                checksum,
            });
        }
        versions.sort_by(|a, b| b.version.cmp(&a.version));

        entries.push(NpkgRegistryEntry {
            import_prefix,
            package: package_name,
            requirement,
            source,
            versions,
        });
    }

    Ok(entries)
}

fn load_npkg_registry(
    registry_path: Option<&str>,
) -> Result<(String, Vec<NpkgRegistryEntry>), Box<dyn std::error::Error>> {
    let (label, value) = if let Some(path) = registry_path {
        let raw = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        (path.to_string(), value)
    } else {
        (
            "builtin://npkg-default-registry".to_string(),
            npkg_default_registry(),
        )
    };
    let entries = parse_npkg_registry(&value)?;
    Ok((label, entries))
}

fn npkg_import_matches_prefix(import_path: &str, prefix: &str) -> bool {
    let normalized = prefix.trim_end_matches(".*");
    if import_path == normalized {
        return true;
    }
    if normalized.is_empty() {
        return false;
    }
    import_path.starts_with(&(normalized.to_string() + "."))
}

fn resolve_npkg_dependencies(
    ast: &ncc_ast::Program,
    registry: &[NpkgRegistryEntry],
) -> Result<Vec<NpkgResolvedDependency>, Box<dyn std::error::Error>> {
    let mut imports = ast
        .uses
        .iter()
        .map(|use_decl| use_decl.path.join("."))
        .collect::<Vec<_>>();
    imports.sort();
    imports.dedup();

    let mut resolved = Vec::new();
    for import_path in imports {
        let mut matches = registry
            .iter()
            .filter(|entry| npkg_import_matches_prefix(&import_path, &entry.import_prefix))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(format!(
                "npkg resolve: unresolved import '{}' in registry",
                import_path
            )
            .into());
        }
        matches.sort_by(|a, b| b.import_prefix.len().cmp(&a.import_prefix.len()));
        let best_len = matches[0].import_prefix.len();
        let tied = matches
            .iter()
            .filter(|entry| entry.import_prefix.len() == best_len)
            .collect::<Vec<_>>();
        if tied.len() > 1 {
            return Err(format!(
                "npkg resolve: ambiguous registry mapping for import '{}' ({} candidates with same prefix depth)",
                import_path,
                tied.len()
            )
            .into());
        }
        let selected_entry = matches[0];
        let req = NpkgSemverRequirement::parse(&selected_entry.requirement).map_err(|err| {
            format!(
                "npkg resolve: invalid requirement for '{}': {}",
                import_path, err
            )
        })?;

        let candidate = selected_entry
            .versions
            .iter()
            .find(|version| req.matches(version.version))
            .ok_or_else(|| {
                format!(
                    "npkg resolve: no version satisfies requirement '{}' for import '{}'",
                    selected_entry.requirement, import_path
                )
            })?;

        resolved.push(NpkgResolvedDependency {
            import: import_path,
            package: selected_entry.package.clone(),
            requirement: selected_entry.requirement.clone(),
            selected_version: candidate.version_raw.clone(),
            checksum: candidate.checksum.clone(),
            source: selected_entry.source.clone(),
            registry_import_prefix: selected_entry.import_prefix.clone(),
        });
    }

    resolved.sort_by(|a, b| {
        a.package
            .cmp(&b.package)
            .then_with(|| a.import.cmp(&b.import))
            .then_with(|| a.selected_version.cmp(&b.selected_version))
    });

    Ok(resolved)
}

fn npkg_manifest_json(path: &str, ast: &ncc_ast::Program) -> serde_json::Value {
    let imports = ast
        .uses
        .iter()
        .map(|use_decl| {
            if use_decl.wildcard {
                format!("{}.*", use_decl.path.join("."))
            } else {
                use_decl.path.join(".")
            }
        })
        .collect::<Vec<_>>();

    let containers = ast
        .containers
        .iter()
        .map(|container| {
            serde_json::json!({
                "name": container.name,
                "context": context_label(&container.context),
                "section_count": container.sections.len(),
            })
        })
        .collect::<Vec<_>>();

    let main_entries = ast
        .containers
        .iter()
        .map(|container| {
            container
                .sections
                .iter()
                .filter(|section| matches!(section, ncc_ast::Section::MainEntry { .. }))
                .count()
        })
        .sum::<usize>();

    serde_json::json!({
        "file": path,
        "imports": imports,
        "containers": containers,
        "main_entries": main_entries,
    })
}

fn npkg_resolve_json(
    path: &str,
    ast: &ncc_ast::Program,
    registry_path: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (registry_label, registry_entries) = load_npkg_registry(registry_path)?;
    let dependencies = resolve_npkg_dependencies(ast, &registry_entries)?;
    let deps_json = dependencies
        .iter()
        .map(|dep| {
            serde_json::json!({
                "import": dep.import,
                "package": dep.package,
                "requirement": dep.requirement,
                "selected": dep.selected_version,
                "checksum": dep.checksum,
                "source": dep.source,
                "registry_import_prefix": dep.registry_import_prefix,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "schema": "npkg.resolve.v1",
        "file": path,
        "registry": registry_label,
        "imports_total": ast.uses.len(),
        "resolved_total": deps_json.len(),
        "dependencies": deps_json,
    }))
}

fn npkg_lock_json(
    path: &str,
    ast: &ncc_ast::Program,
    registry_path: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let resolve = npkg_resolve_json(path, ast, registry_path)?;
    Ok(serde_json::json!({
        "schema": "npkg.lock.v1",
        "version": 1,
        "file": resolve.get("file").and_then(|v| v.as_str()).unwrap_or(path),
        "registry": resolve.get("registry").and_then(|v| v.as_str()).unwrap_or("builtin://npkg-default-registry"),
        "dependencies": resolve.get("dependencies").cloned().unwrap_or_else(|| serde_json::json!([])),
    }))
}

pub(crate) fn run_npkg_command(args: &[String]) -> Result<(), CliFailure> {
    run_npkg_command_legacy(args).map_err(|error| CliFailure::from_boxed_at(error, "ghor.npkg"))
}

fn run_npkg_command_legacy(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("npkg").expect("npkg help must exist"));
        return Ok(());
    }
    if args.is_empty() {
        return Err(actionable_command_error(
            "npkg",
            "npkg syntax: npkg <manifest|check-imports|list-imports|resolve|lock|verify-lock> <file.n> [--out <path>] [--registry <path>] [--lock <path>]",
        )
        .into());
    }

    let (action, path_idx) = if is_high_level_source_path(&args[0]) {
        ("manifest", 0usize)
    } else {
        (args[0].as_str(), 1usize)
    };

    let path = args
        .get(path_idx)
        .ok_or_else(|| actionable_command_error("npkg", "npkg syntax: missing <file.n> argument"))?
        .clone();

    let mut out_path: Option<String> = None;
    let mut registry_path: Option<String> = None;
    let mut lock_path: Option<String> = None;
    let mut idx = path_idx + 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" | "help" => {
                println!("{}", command_help("npkg").expect("npkg help must exist"));
                return Ok(());
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("npkg", "npkg syntax: --out requires a file path")
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            "--registry" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("npkg", "npkg syntax: --registry requires a file path")
                })?;
                registry_path = Some(value.clone());
                idx += 2;
            }
            "--lock" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("npkg", "npkg syntax: --lock requires a file path")
                })?;
                lock_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "npkg",
                    format!("npkg: unknown option '{}'", other),
                )
                .into())
            }
        }
    }

    let ast = load_ast_from_path(&path)?;
    sema_check_with_warnings(&ast)?;

    match action {
        "check-imports" => {
            println!(
                "OK: npkg imports file={} uses={} containers={}",
                path,
                ast.uses.len(),
                ast.containers.len()
            );
        }
        "list-imports" => {
            for use_decl in &ast.uses {
                if use_decl.wildcard {
                    println!("{}.*", use_decl.path.join("."));
                } else {
                    println!("{}", use_decl.path.join("."));
                }
            }
        }
        "manifest" => {
            let manifest = npkg_manifest_json(&path, &ast);
            let rendered = serde_json::to_string_pretty(&manifest)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: npkg manifest {}", out);
            } else {
                println!("{}", rendered);
            }
        }
        "resolve" => {
            let resolved = npkg_resolve_json(&path, &ast, registry_path.as_deref())?;
            let rendered = serde_json::to_string_pretty(&resolved)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: npkg resolve {}", out);
            } else {
                println!("{}", rendered);
            }
        }
        "lock" => {
            let lock = npkg_lock_json(&path, &ast, registry_path.as_deref())?;
            let rendered = serde_json::to_string_pretty(&lock)?;
            let target = lock_path
                .or(out_path)
                .unwrap_or_else(|| format!("{}.lock.json", path));
            fs::write(&target, rendered.as_bytes())?;
            println!("OK: npkg lock {}", target);
        }
        "verify-lock" => {
            let lock_target = lock_path.or(out_path).ok_or_else(|| {
                actionable_command_error(
                    "npkg",
                    "npkg verify-lock requires --lock <path> (or --out <path>)",
                )
            })?;
            let raw = fs::read_to_string(&lock_target)?;
            let on_disk: serde_json::Value = serde_json::from_str(&raw)?;
            if on_disk.get("schema").and_then(|v| v.as_str()) != Some("npkg.lock.v1") {
                return Err("npkg verify-lock: lock schema must be 'npkg.lock.v1'".into());
            }
            let expected = npkg_lock_json(&path, &ast, registry_path.as_deref())?;
            if on_disk != expected {
                return Err(
                    "npkg verify-lock failed: lock drift detected; regenerate with `npkg lock`"
                        .into(),
                );
            }
            println!("OK: npkg verify-lock {}", lock_target);
        }
        other => {
            let suggestion = suggestion_suffix(
                other,
                &[
                    "manifest",
                    "check-imports",
                    "list-imports",
                    "resolve",
                    "lock",
                    "verify-lock",
                ],
            );
            return Err(actionable_command_error(
                "npkg",
                format!(
                    "npkg: unknown action '{}'; expected manifest/check-imports/list-imports/resolve/lock/verify-lock{}",
                    other, suggestion
                ),
            )
            .into());
        }
    }

    Ok(())
}
