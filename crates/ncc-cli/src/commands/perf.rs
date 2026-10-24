use super::super::*;

fn perf_help_text() -> &'static str {
    command_help("perf").expect("perf help must exist")
}

fn command_version_line(tool: &str, args: &[&str]) -> serde_json::Value {
    match Command::new(tool).args(args).output() {
        Ok(output) => {
            let text = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).to_string()
            };
            serde_json::json!({
                "tool": tool,
                "available": output.status.success(),
                "version": text.lines().next().unwrap_or("").trim(),
            })
        }
        Err(err) => serde_json::json!({
            "tool": tool,
            "available": false,
            "error": err.to_string(),
        }),
    }
}

fn duration_us_u128(duration: std::time::Duration) -> u128 {
    duration.as_micros()
}

const C_STYLE_COMPILERS: [&str; 2] = ["clang", "gcc"];
const C_STYLE_OPT_LEVELS: [&str; 2] = ["O2", "O3"];
const PERF_OPTIMIZER_GUIDANCE_COUNTERS: [&str; 13] = [
    "inline_expansions",
    "whole_program_inline_candidates",
    "whole_program_cross_container_inlines",
    "whole_program_devirtualized_calls",
    "whole_program_dead_functions_removed",
    "lto_linkage_units",
    "lto_imported_call_edges",
    "lto_internalized_symbols",
    "pgo_instrumented_sites",
    "pgo_hot_call_sites",
    "pgo_hot_inline_biases",
    "pgo_cold_functions",
    "loop_vectorization_candidates",
];

fn perf_c_style_compilers_json() -> Vec<serde_json::Value> {
    C_STYLE_COMPILERS
        .iter()
        .map(|compiler| serde_json::json!(compiler))
        .collect()
}

fn perf_c_style_opt_levels_json() -> Vec<serde_json::Value> {
    C_STYLE_OPT_LEVELS
        .iter()
        .map(|opt_level| serde_json::json!(opt_level))
        .collect()
}

fn perf_c_style_toolchains_payload() -> serde_json::Value {
    serde_json::json!({
        "clang": command_version_line("clang", &["--version"]),
        "gcc": command_version_line("gcc", &["--version"]),
    })
}

fn perf_safe_stem(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn perf_fixture_class(size: &str, name: &str) -> &'static str {
    if size == "small" || name.contains("minimal") || name.contains("return") {
        "microbenchmark"
    } else {
        "program"
    }
}

#[cfg(not(ncc_public_beta))]
const PERF_CONTRACT_CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../_archive/old_examples/performance/native_fast/benchmark_contracts.json"
));

#[cfg(ncc_public_beta)]
const PERF_CONTRACT_CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/old_examples/performance/native_fast/benchmark_contracts.json"
));

fn perf_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn perf_fixture_text_sha256(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path).map_err(|error| {
        format!(
            "performance contract fixture '{}' cannot be read as UTF-8: {error}",
            path.display()
        )
    })?;
    let canonical = source.replace("\r\n", "\n").replace('\r', "\n");
    Ok(perf_sha256_hex(canonical.as_bytes()))
}

fn perf_contract_required_u64(value: &serde_json::Value, pointer: &str) -> Result<u64, String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            format!("performance contract missing required unsigned integer '{pointer}'")
        })
}

fn perf_contract_required_str<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
) -> Result<&'a str, String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| format!("performance contract missing required string '{pointer}'"))
}

fn perf_contract_expect_u64(
    value: &serde_json::Value,
    pointer: &str,
    expected: u64,
) -> Result<(), String> {
    let actual = perf_contract_required_u64(value, pointer)?;
    if actual != expected {
        return Err(format!(
            "performance contract field '{pointer}' must be exactly {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn perf_contract_expect_str(
    value: &serde_json::Value,
    pointer: &str,
    expected: &str,
) -> Result<(), String> {
    let actual = perf_contract_required_str(value, pointer)?;
    if actual != expected {
        return Err(format!(
            "performance contract field '{pointer}' must be '{expected}', got '{actual}'"
        ));
    }
    Ok(())
}

fn perf_validate_contract_thresholds(
    value: &serde_json::Value,
    pointer: &str,
) -> Result<(), String> {
    let thresholds = value
        .pointer(pointer)
        .ok_or_else(|| format!("performance contract missing required object '{pointer}'"))?;
    if !thresholds.is_object() {
        return Err(format!(
            "performance contract field '{pointer}' must be an object"
        ));
    }
    for field in ["minimum_warmup_runs", "minimum_measured_runs"] {
        let field_pointer = format!("{pointer}/{field}");
        if perf_contract_required_u64(value, &field_pointer)? == 0 {
            return Err(format!(
                "performance contract threshold '{field_pointer}' must be greater than zero"
            ));
        }
    }
    if perf_contract_required_u64(value, &format!("{pointer}/required_exit_code"))? != 0 {
        return Err(format!(
            "performance contract threshold '{pointer}/required_exit_code' must be exactly zero"
        ));
    }
    for (field, expected) in [("iteration_match", "exact"), ("checksum_match", "exact")] {
        let field_pointer = format!("{pointer}/{field}");
        let actual = perf_contract_required_str(value, &field_pointer)?;
        if actual != expected {
            return Err(format!(
                "performance contract threshold '{field_pointer}' must be '{expected}', got '{actual}'"
            ));
        }
    }
    Ok(())
}

fn perf_validate_exact_checksum(
    contract: &serde_json::Value,
    field: &str,
    expected: u64,
) -> Result<(), String> {
    let pointer = format!("/expected/checksums/{field}");
    let actual = perf_contract_required_u64(contract, &pointer)?;
    if actual != expected {
        return Err(format!(
            "performance contract checksum mismatch for '{field}': expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn perf_triangular_low_byte_accumulator(seed: u64, iterations: u64) -> u64 {
    const PERIOD: u64 = 512;
    let partial = |count: u64| -> u64 {
        (0..count)
            .map(|index| {
                let triangular = index.saturating_mul(index + 1) / 2;
                seed.wrapping_add(triangular) & 255
            })
            .sum()
    };
    let total = partial(PERIOD)
        .saturating_mul(iterations / PERIOD)
        .saturating_add(partial(iterations % PERIOD));
    total & u32::MAX as u64
}

fn perf_validate_benchmark_contract(contract: &serde_json::Value) -> Result<(), String> {
    if perf_contract_required_str(contract, "/schema")? != "ncc.performance.semantic_equivalence.v1"
    {
        return Err("performance benchmark contract has an unsupported schema".to_string());
    }
    let name = perf_contract_required_str(contract, "/name")?;
    perf_contract_expect_str(contract, "/status", "source_enforced_exact_result")?;
    let algorithm = perf_contract_required_str(contract, "/workload/algorithm")?;
    let iterations = perf_contract_required_u64(contract, "/workload/iterations")?;
    if iterations == 0 {
        return Err(format!(
            "performance contract '{name}' requires a non-zero iteration count"
        ));
    }
    if perf_contract_required_u64(contract, "/expected/exit_code")? != 0 {
        return Err(format!(
            "performance contract '{name}' must require exit code zero"
        ));
    }
    let checksums = contract
        .pointer("/expected/checksums")
        .and_then(serde_json::Value::as_object)
        .filter(|checksums| !checksums.is_empty())
        .ok_or_else(|| {
            format!("performance contract '{name}' requires at least one exact checksum")
        })?;
    if checksums
        .values()
        .any(|checksum| checksum.as_u64().is_none())
    {
        return Err(format!(
            "performance contract '{name}' checksums must be unsigned integers"
        ));
    }
    perf_validate_contract_thresholds(contract, "/thresholds")?;

    for language in ["nC", "C", "Rust"] {
        let fixture_pointer = format!("/fixtures/{language}");
        let fixture = perf_contract_required_str(contract, &fixture_pointer)?;
        if Path::new(fixture).components().count() != 1 {
            return Err(format!(
                "performance contract fixture '{fixture_pointer}' must be a file name without path traversal"
            ));
        }
        let hash_pointer = format!("/fixture_sha256/{language}");
        let hash = perf_contract_required_str(contract, &hash_pointer)?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!(
                "performance contract field '{hash_pointer}' must be a lowercase SHA-256 digest"
            ));
        }
    }

    let expected_fixtures = match name {
        "minimal_process_entry" => Some([
            ("nC", "minimal_return.n"),
            ("C", "minimal_return.c"),
            ("Rust", "minimal_return.rs"),
        ]),
        "arithmetic_call_pipeline" => Some([
            ("nC", "bench_medium.n"),
            ("C", "bench_medium.c"),
            ("Rust", "bench_medium.rs"),
        ]),
        "volatile_loop_memory_pipeline" => Some([
            ("nC", "bench_workload.n"),
            ("C", "bench_workload.c"),
            ("Rust", "bench_workload.rs"),
        ]),
        _ => None,
    };
    if let Some(expected_fixtures) = expected_fixtures {
        for (language, expected) in expected_fixtures {
            perf_contract_expect_str(contract, &format!("/fixtures/{language}"), expected)?;
        }
    }

    match name {
        "minimal_process_entry" => {
            perf_contract_expect_str(contract, "/workload/algorithm", "process_entry")?;
            perf_contract_expect_u64(contract, "/workload/iterations", 1)?;
        }
        "arithmetic_call_pipeline" => {
            perf_contract_expect_str(
                contract,
                "/workload/algorithm",
                "dependent_lcg_xor_mask_u64",
            )?;
            perf_contract_expect_u64(contract, "/workload/iterations", 5_000_000)?;
            perf_contract_expect_u64(contract, "/workload/initial_seed_u64", 1)?;
            perf_contract_expect_u64(contract, "/workload/multiplier_u64", 1_664_525)?;
            perf_contract_expect_u64(contract, "/workload/xor_u64", 1_013_904_223)?;
            perf_contract_expect_u64(contract, "/workload/mask_u64", 2_147_483_647)?;
            perf_contract_expect_str(
                contract,
                "/workload/volatile_reads",
                "initial_seed_and_final_observation",
            )?;
            perf_contract_expect_u64(contract, "/workload/volatile_writes_per_iteration", 1)?;
        }
        "volatile_loop_memory_pipeline" => {
            perf_contract_expect_str(
                contract,
                "/workload/algorithm",
                "volatile_u32_triangular_accumulator",
            )?;
            perf_contract_expect_u64(contract, "/workload/iterations", 20_000_000)?;
            perf_contract_expect_u64(contract, "/workload/initial_cell_u32", 1)?;
            perf_contract_expect_u64(contract, "/workload/volatile_reads_per_iteration", 1)?;
            perf_contract_expect_u64(contract, "/workload/volatile_writes_per_iteration", 1)?;
            perf_contract_expect_u64(contract, "/workload/accumulator_mask_u32", 255)?;
        }
        _ => {}
    }

    match algorithm {
        "process_entry" => {
            if iterations != 1 {
                return Err("minimal process entry must execute exactly once".to_string());
            }
            perf_validate_exact_checksum(contract, "return_i32", 0)?;
        }
        "dependent_lcg_xor_mask_u64" => {
            let mut value = perf_contract_required_u64(contract, "/workload/initial_seed_u64")?;
            let multiplier = perf_contract_required_u64(contract, "/workload/multiplier_u64")?;
            let xor = perf_contract_required_u64(contract, "/workload/xor_u64")?;
            let mask = perf_contract_required_u64(contract, "/workload/mask_u64")?;
            for index in 0..iterations {
                value = value.wrapping_mul(multiplier).wrapping_add(index) ^ xor;
                value &= mask;
            }
            perf_validate_exact_checksum(contract, "final_seed_u32", value)?;
            perf_validate_exact_checksum(contract, "final_index_u64", iterations)?;
        }
        "volatile_u32_triangular_accumulator" => {
            let seed = perf_contract_required_u64(contract, "/workload/initial_cell_u32")?;
            let mask = perf_contract_required_u64(contract, "/workload/accumulator_mask_u32")?;
            if mask != 255 {
                return Err(format!(
                    "performance contract '{name}' accumulator mask must be exactly 255"
                ));
            }
            let triangular =
                (iterations as u128).saturating_mul(iterations.saturating_sub(1) as u128) / 2;
            let final_cell = seed.wrapping_add(triangular as u64) & u32::MAX as u64;
            let accumulator = perf_triangular_low_byte_accumulator(seed, iterations);
            perf_validate_exact_checksum(contract, "final_cell_u32", final_cell)?;
            perf_validate_exact_checksum(contract, "low_byte_accumulator_u32", accumulator)?;
            perf_validate_exact_checksum(contract, "final_index_u64", iterations)?;
        }
        "fixture_defined" => {}
        other => {
            return Err(format!(
                "performance contract '{name}' uses unknown workload algorithm '{other}'"
            ));
        }
    }
    Ok(())
}

fn perf_validate_contract_catalog(catalog: &serde_json::Value) -> Result<(), String> {
    if perf_contract_required_str(catalog, "/schema")? != "ncc.performance.contract_catalog.v1" {
        return Err("performance contract catalog has an unsupported schema".to_string());
    }
    perf_contract_required_str(catalog, "/suite")?;
    perf_contract_expect_str(catalog, "/fixture_hash_canonicalization", "utf8_lf")?;
    perf_validate_contract_thresholds(catalog, "/measurement_thresholds")?;
    let contracts = catalog
        .get("contracts")
        .and_then(serde_json::Value::as_array)
        .filter(|contracts| !contracts.is_empty())
        .ok_or_else(|| "performance contract catalog requires contracts".to_string())?;
    let mut names = std::collections::BTreeSet::new();
    let mut selectors = std::collections::BTreeSet::new();
    for contract in contracts {
        perf_validate_benchmark_contract(contract)?;
        let name = perf_contract_required_str(contract, "/name")?;
        if !names.insert(name.to_string()) {
            return Err(format!("duplicate performance contract '{name}'"));
        }
        if !selectors.insert(name.to_string()) {
            return Err(format!("duplicate performance contract selector '{name}'"));
        }
        let aliases = contract
            .get("aliases")
            .and_then(serde_json::Value::as_array)
            .filter(|aliases| !aliases.is_empty())
            .ok_or_else(|| format!("performance contract '{name}' requires aliases"))?;
        for alias in aliases {
            let alias = alias
                .as_str()
                .filter(|alias| !alias.is_empty())
                .ok_or_else(|| {
                    format!("performance contract '{name}' contains an invalid alias")
                })?;
            if !selectors.insert(alias.to_string()) {
                return Err(format!(
                    "duplicate performance contract selector or alias '{alias}'"
                ));
            }
        }
    }
    for required in [
        "minimal_process_entry",
        "arithmetic_call_pipeline",
        "volatile_loop_memory_pipeline",
    ] {
        if !names.contains(required) {
            return Err(format!(
                "performance contract catalog missing required benchmark '{required}'"
            ));
        }
    }
    Ok(())
}

fn perf_contract_catalog() -> Result<serde_json::Value, String> {
    let catalog: serde_json::Value = serde_json::from_str(PERF_CONTRACT_CATALOG_JSON)
        .map_err(|err| format!("invalid performance contract JSON: {err}"))?;
    perf_validate_contract_catalog(&catalog)?;
    Ok(catalog)
}

fn perf_validate_contract_sources(
    catalog: &serde_json::Value,
    suite_dir: &Path,
) -> Result<(), String> {
    let contracts = catalog
        .get("contracts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "performance contract catalog requires contracts".to_string())?;
    for contract in contracts {
        let name = perf_contract_required_str(contract, "/name")?;
        for language in ["nC", "C", "Rust"] {
            let fixture = perf_contract_required_str(contract, &format!("/fixtures/{language}"))?;
            let expected =
                perf_contract_required_str(contract, &format!("/fixture_sha256/{language}"))?;
            let path = suite_dir.join(fixture);
            let actual = perf_fixture_text_sha256(&path)?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(format!(
                    "performance contract source hash mismatch for '{name}' {language} fixture '{}': expected {expected}, got {actual}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn perf_contract_catalog_for_suite(suite_dir: &Path) -> Result<serde_json::Value, String> {
    let catalog = perf_contract_catalog()?;
    perf_validate_contract_sources(&catalog, suite_dir)?;
    Ok(catalog)
}

fn perf_contract_catalog_summary(catalog: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema": catalog.get("schema").cloned().unwrap_or(serde_json::Value::Null),
        "suite": catalog.get("suite").cloned().unwrap_or(serde_json::Value::Null),
        "status": "validated",
        "catalog_sha256": perf_sha256_hex(PERF_CONTRACT_CATALOG_JSON.as_bytes()),
        "fixture_hash_canonicalization": catalog
            .get("fixture_hash_canonicalization")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "fixture_sources_verified": true,
        "contract_count": catalog
            .get("contracts")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "measurement_thresholds": catalog
            .get("measurement_thresholds")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

fn perf_semantic_equivalence_payload(
    catalog: &serde_json::Value,
    name: &str,
) -> Result<serde_json::Value, String> {
    let selected = catalog
        .get("contracts")
        .and_then(serde_json::Value::as_array)
        .and_then(|contracts| {
            contracts.iter().find(|contract| {
                contract.get("name").and_then(serde_json::Value::as_str) == Some(name)
                    || contract
                        .get("aliases")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .any(|alias| alias.as_str() == Some(name))
            })
        })
        .cloned();
    let Some(mut contract) = selected else {
        return Ok(serde_json::json!({
            "schema": "ncc.performance.semantic_equivalence.v1",
            "name": name,
            "status": "uncatalogued_fixture_exit_only",
            "workload": {
                "algorithm": "fixture_defined",
                "iterations": 1,
            },
            "expected": {
                "exit_code": 0,
                "checksums": {
                    "return_i32": 0,
                },
            },
            "thresholds": catalog
                .get("measurement_thresholds")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            "selected_as": name,
        }));
    };
    if let Some(object) = contract.as_object_mut() {
        object.insert("selected_as".to_string(), serde_json::json!(name));
    }
    perf_validate_benchmark_contract(&contract)?;
    Ok(contract)
}

#[cfg(test)]
mod performance_contract_tests {
    use super::*;

    fn shipped_contract_suite_dir() -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if cfg!(ncc_public_beta) {
            root.join("../../tests/fixtures/old_examples/performance/native_fast")
        } else {
            root.join("../../_archive/old_examples/performance/native_fast")
        }
    }

    #[test]
    fn exact_performance_contract_catalog_is_valid() {
        let catalog = perf_contract_catalog().expect("shipped performance contract must validate");
        let large = perf_semantic_equivalence_payload(&catalog, "volatile_loop_memory_pipeline")
            .expect("large exact contract must resolve");
        assert_eq!(
            large
                .pointer("/workload/iterations")
                .and_then(serde_json::Value::as_u64),
            Some(20_000_000)
        );
        assert_eq!(
            large
                .pointer("/expected/checksums/final_cell_u32")
                .and_then(serde_json::Value::as_u64),
            Some(542_894_465)
        );
        perf_validate_contract_sources(&catalog, &shipped_contract_suite_dir())
            .expect("shipped fixture fingerprints must match the validated catalog");
    }

    #[test]
    fn altered_performance_checksum_is_rejected() {
        let mut catalog: serde_json::Value =
            serde_json::from_str(PERF_CONTRACT_CATALOG_JSON).expect("contract JSON");
        let large = catalog["contracts"]
            .as_array_mut()
            .expect("contracts")
            .iter_mut()
            .find(|contract| {
                contract.get("name").and_then(serde_json::Value::as_str)
                    == Some("volatile_loop_memory_pipeline")
            })
            .expect("large contract");
        *large
            .pointer_mut("/expected/checksums/final_cell_u32")
            .expect("final checksum") = serde_json::json!(542_894_466_u64);
        let error = perf_validate_contract_catalog(&catalog)
            .expect_err("an altered exact checksum must reject the catalog");
        assert!(error.contains("checksum mismatch"), "{error}");
    }

    #[test]
    fn incomplete_performance_contract_is_rejected() {
        let mut catalog: serde_json::Value =
            serde_json::from_str(PERF_CONTRACT_CATALOG_JSON).expect("contract JSON");
        let medium = catalog["contracts"]
            .as_array_mut()
            .expect("contracts")
            .iter_mut()
            .find(|contract| {
                contract.get("name").and_then(serde_json::Value::as_str)
                    == Some("arithmetic_call_pipeline")
            })
            .expect("medium contract");
        medium["workload"]
            .as_object_mut()
            .expect("workload")
            .remove("iterations");
        let error = perf_validate_contract_catalog(&catalog)
            .expect_err("a missing required field must reject the catalog");
        assert!(error.contains("/workload/iterations"), "{error}");
    }

    #[test]
    fn altered_performance_fixture_fingerprint_is_rejected() {
        let mut catalog: serde_json::Value =
            serde_json::from_str(PERF_CONTRACT_CATALOG_JSON).expect("contract JSON");
        catalog["contracts"][1]["fixture_sha256"]["nC"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
        let error = perf_validate_contract_sources(&catalog, &shipped_contract_suite_dir())
            .expect_err("an altered source fingerprint must reject the catalog");
        assert!(error.contains("source hash mismatch"), "{error}");
    }

    #[test]
    fn reduced_hard_workload_iteration_count_is_rejected() {
        let mut catalog: serde_json::Value =
            serde_json::from_str(PERF_CONTRACT_CATALOG_JSON).expect("contract JSON");
        catalog["contracts"][2]["workload"]["iterations"] = serde_json::json!(19_999_999_u64);
        let error = perf_validate_contract_catalog(&catalog)
            .expect_err("the hard workload cannot silently lose an iteration");
        assert!(error.contains("must be exactly 20000000"), "{error}");
    }
}

fn perf_missing_fixture_build(
    language: &str,
    tool: &str,
    source: &Path,
    command: String,
) -> serde_json::Value {
    serde_json::json!({
        "language": language,
        "tool": tool,
        "status": "missing_fixture",
        "duration_us": 0u64,
        "binary_size_bytes": serde_json::Value::Null,
        "reason": "baseline_source_missing",
        "source": source.display().to_string(),
        "command": command,
    })
}

fn run_perf_binary(path: &Path, iterations: usize) -> serde_json::Value {
    let warmup_iterations = iterations.min(3);
    let mut warmups = Vec::new();
    let mut runs = Vec::new();
    let mut samples_ns = Vec::with_capacity(iterations);
    let mut ok = true;

    for idx in 0..warmup_iterations {
        let status = Command::new(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(status) => {
                let success = status.success();
                ok &= success;
                warmups.push(serde_json::json!({
                    "iteration": idx,
                    "exit_code": status.code(),
                    "success": success,
                }));
            }
            Err(err) => {
                ok = false;
                warmups.push(serde_json::json!({
                    "iteration": idx,
                    "success": false,
                    "error": err.to_string(),
                }));
            }
        }
    }

    for idx in 0..iterations {
        let started = Instant::now();
        let status = Command::new(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let elapsed_ns = started.elapsed().as_nanos();
        let elapsed_us = elapsed_ns / 1_000;
        samples_ns.push(elapsed_ns);
        match status {
            Ok(status) => {
                let success = status.success();
                ok &= success;
                runs.push(serde_json::json!({
                    "iteration": idx,
                    "duration_ns": elapsed_ns,
                    "duration_us": elapsed_us,
                    "exit_code": status.code(),
                    "success": success,
                }));
            }
            Err(err) => {
                ok = false;
                runs.push(serde_json::json!({
                    "iteration": idx,
                    "duration_ns": elapsed_ns,
                    "duration_us": elapsed_us,
                    "success": false,
                    "error": err.to_string(),
                }));
            }
        }
    }

    let total_ns = samples_ns.iter().copied().sum::<u128>();
    let mut ordered_ns = samples_ns.clone();
    ordered_ns.sort_unstable();
    let median_ns = match ordered_ns.as_slice() {
        [] => 0,
        values if values.len() % 2 == 1 => values[values.len() / 2],
        values => {
            let upper = values.len() / 2;
            values[upper - 1].saturating_add(values[upper]) / 2
        }
    };
    let p95_ns = if ordered_ns.is_empty() {
        0
    } else {
        let index = (ordered_ns.len() * 95).div_ceil(100).saturating_sub(1);
        ordered_ns[index]
    };
    let min_ns = ordered_ns.first().copied().unwrap_or(0);
    let max_ns = ordered_ns.last().copied().unwrap_or(0);

    serde_json::json!({
        "status": if ok { "ok" } else { "failed" },
        "measurement_scope": "execution_only_after_build",
        "clock": "monotonic_steady_clock",
        "warmup_iterations": warmup_iterations,
        "warmups": warmups,
        "iterations": iterations,
        "total_ns": total_ns,
        "avg_ns": if iterations == 0 { 0 } else { total_ns / iterations as u128 },
        "median_ns": median_ns,
        "p95_ns": p95_ns,
        "min_ns": min_ns,
        "max_ns": max_ns,
        "total_us": total_ns / 1_000,
        "avg_us": if iterations == 0 { 0 } else { (total_ns / iterations as u128) / 1_000 },
        "median_us": median_ns / 1_000,
        "p95_us": p95_ns / 1_000,
        "min_us": min_ns / 1_000,
        "max_us": max_ns / 1_000,
        "runs": runs,
    })
}

fn perf_external_build(
    language: &str,
    tool: &str,
    args: &[String],
    output: &Path,
) -> serde_json::Value {
    let started = Instant::now();
    let result = Command::new(tool)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    let duration_us = duration_us_u128(started.elapsed());
    match result {
        Ok(output_result) => {
            let binary_size = fs::metadata(output).map(|meta| meta.len()).ok();
            serde_json::json!({
                "language": language,
                "tool": tool,
                "status": if output_result.status.success() && binary_size.unwrap_or(0) > 0 { "ok" } else { "failed" },
                "duration_us": duration_us,
                "exit_code": output_result.status.code(),
                "binary_size_bytes": binary_size,
                "stderr": String::from_utf8_lossy(&output_result.stderr).trim(),
                "command": format!("{} {}", tool, args.join(" ")),
            })
        }
        Err(err) => serde_json::json!({
            "language": language,
            "tool": tool,
            "status": "missing_toolchain",
            "duration_us": duration_us,
            "error": err.to_string(),
            "command": format!("{} {}", tool, args.join(" ")),
        }),
    }
}

fn perf_executable_path(dir: &Path, stem: &str) -> PathBuf {
    if cfg!(windows) {
        dir.join(format!("{stem}.exe"))
    } else {
        dir.join(stem)
    }
}

fn perf_ncc_native_build(source: &Path, output: &Path) -> serde_json::Value {
    let started = Instant::now();
    let result =
        (|| -> Result<(serde_json::Value, serde_json::Value), Box<dyn std::error::Error>> {
            let source_bytes = fs::metadata(source).map(|meta| meta.len()).unwrap_or(0);
            let read_started = Instant::now();
            let src = fs::read_to_string(source)?;
            let read_us = duration_us_u128(read_started.elapsed());

            let lex_started = Instant::now();
            let tokens = ncc_lexer::lex(&src)?;
            let lex_us = duration_us_u128(lex_started.elapsed());

            let parse_started = Instant::now();
            let ast = ncc_parser::parse(tokens)?;
            let parse_us = duration_us_u128(parse_started.elapsed());

            let sema_started = Instant::now();
            ncc_sema::analyze(&ast).map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
            verify_type_contract_execution_path(&ast)?;
            ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
            let sema_us = duration_us_u128(sema_started.elapsed());

            let ir_profile = default_host_backend_profile();
            validate_backend_artifact_profile(BackendArtifactKind::BuildExe, &ir_profile)?;

            let ir_started = Instant::now();
            let ir = ncc_ir::lower_with_backend_profile(ast.clone(), ir_profile);
            ncc_ir::verify_type_contract_lowering(&ast, &ir)
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
            let ir_lower_us = duration_us_u128(ir_started.elapsed());

            let backend_started = Instant::now();
            let backend = ncc_ir::emit_backend(&ir);
            let backend_emit_us = duration_us_u128(backend_started.elapsed());

            let object_started = Instant::now();
            let measured_object = ncc_ir::emit_object(&backend);
            let object_ir_us = duration_us_u128(object_started.elapsed());

            let object_emit_started = Instant::now();
            let measured_elf = ncc_ir::emit_object_elf64(&measured_object)?;
            let object_emit_us = duration_us_u128(object_emit_started.elapsed());
            let measured_stats = backend_object_stats_payload(&measured_object);

            let optimized_started = Instant::now();
            let build_profile = default_host_backend_profile();
            let artifact =
                emit_optimized_backend_object_ir_from_ast_with_profile(&ast, build_profile)?;
            let optimizer_us = duration_us_u128(optimized_started.elapsed());
            let object = &artifact.object;
            let entry = resolve_native_entry_symbol(&ast, None);
            let elf = ncc_ir::emit_object_elf64(object)?;
            let internal_link_format = super::tooling::nbuild_internal_os_link_format(object, None);
            let optimization = native_fast_optimization_payload(
                &source.display().to_string(),
                object,
                &elf,
                &entry,
                Some(internal_optimizer_payload(&artifact)),
            );
            let link_started = Instant::now();
            build_hosted_executable_from_object_with_linker(
                object,
                &output.to_string_lossy(),
                None,
                &entry,
            )?;
            let link_us = duration_us_u128(link_started.elapsed());
            let pipeline = serde_json::json!({
                "schema": "ncc.performance.pipeline.v1",
                "source_bytes": source_bytes,
                "source_chars": src.chars().count(),
                "read_us": read_us,
                "lex_us": lex_us,
                "parse_us": parse_us,
                "sema_us": sema_us,
                "ir_lower_us": ir_lower_us,
                "backend_emit_us": backend_emit_us,
                "object_ir_us": object_ir_us,
                "object_emit_us": object_emit_us,
                "optimizer_us": optimizer_us,
                "link_us": link_us,
                "linker": {
                    "route": if internal_link_format.is_some() {
                        "machine_phase12_internal_os_executable"
                    } else {
                        "external_host_linker"
                    },
                    "format": internal_link_format,
                    "explicit_external_linker": false,
                },
                "bytes_emitted": {
                    "measured_elf_object_bytes": measured_elf.len(),
                    "optimized_elf_object_bytes": elf.len(),
                    "hosted_executable_bytes": fs::metadata(output).map(|meta| meta.len()).ok(),
                },
                "object_stats": measured_stats,
                "stages": [
                    "read",
                    "lex",
                    "parse",
                    "sema",
                    "ir_lower",
                    "backend_emit",
                    "object_ir",
                    "object_emit",
                    "optimizer",
                    "link"
                ],
            });
            Ok((optimization, pipeline))
        })();
    let duration_us = duration_us_u128(started.elapsed());
    let binary_size = fs::metadata(output).map(|meta| meta.len()).ok();
    match result {
        Ok((optimization, pipeline)) => serde_json::json!({
            "language": "nC",
            "tool": "ncc",
            "status": if binary_size.unwrap_or(0) > 0 { "ok" } else { "failed" },
            "duration_us": duration_us,
            "binary_size_bytes": binary_size,
            "command": format!("ncc internal native-fast build {}", source.display()),
            "pipeline": pipeline,
            "optimization": optimization,
        }),
        Err(err) => serde_json::json!({
            "language": "nC",
            "tool": "ncc",
            "status": "failed",
            "duration_us": duration_us,
            "binary_size_bytes": binary_size,
            "error": err.to_string(),
            "command": format!("ncc internal native-fast build {}", source.display()),
        }),
    }
}

fn perf_case_payload(
    language: &str,
    source: &Path,
    output: &Path,
    build: serde_json::Value,
    iterations: usize,
) -> serde_json::Value {
    let run = if build.get("status").and_then(serde_json::Value::as_str) == Some("ok") {
        run_perf_binary(output, iterations)
    } else {
        serde_json::json!({
            "status": "not_run",
            "reason": "build_not_ok",
            "iterations": iterations,
        })
    };
    serde_json::json!({
        "language": language,
        "source": source.display().to_string(),
        "output": output.display().to_string(),
        "build": build,
        "run": run,
    })
}

fn perf_c_style_matrix_case(
    source: &Path,
    output_dir: &Path,
    benchmark_name: &str,
    fixture_class: &str,
    compiler: &str,
    opt_level: &str,
    iterations: usize,
) -> serde_json::Value {
    let opt_flag = format!("-{opt_level}");
    let case_id = format!("{compiler}-{opt_level}");
    let output = perf_executable_path(
        output_dir,
        &format!(
            "{}_c_{}_{}",
            perf_safe_stem(benchmark_name),
            perf_safe_stem(compiler),
            opt_level.to_ascii_lowercase()
        ),
    );
    let args = vec![
        opt_flag.clone(),
        source.display().to_string(),
        "-o".to_string(),
        output.display().to_string(),
    ];
    let command = format!("{} {}", compiler, args.join(" "));
    let build = if source.is_file() {
        perf_external_build("C", compiler, &args, &output)
    } else {
        perf_missing_fixture_build("C", compiler, source, command)
    };
    let run = if build.get("status").and_then(serde_json::Value::as_str) == Some("ok") {
        run_perf_binary(&output, iterations)
    } else {
        serde_json::json!({
            "status": "not_run",
            "reason": "build_not_ok",
            "iterations": iterations,
        })
    };
    serde_json::json!({
        "schema": "ncc.performance.c_style_matrix.case.v1",
        "language": "C",
        "case_id": case_id,
        "compiler": compiler,
        "opt_level": opt_level,
        "optimization_flag": opt_flag,
        "fixture_class": fixture_class,
        "source": source.display().to_string(),
        "output": output.display().to_string(),
        "build": build,
        "run": run,
    })
}

fn perf_c_style_matrix_cases(
    source: &Path,
    output_dir: &Path,
    benchmark_name: &str,
    fixture_class: &str,
    iterations: usize,
) -> Vec<serde_json::Value> {
    let mut cases = Vec::new();
    for compiler in C_STYLE_COMPILERS {
        for opt_level in C_STYLE_OPT_LEVELS {
            cases.push(perf_c_style_matrix_case(
                source,
                output_dir,
                benchmark_name,
                fixture_class,
                compiler,
                opt_level,
                iterations,
            ));
        }
    }
    cases
}

fn perf_case_build_us(case: &serde_json::Value) -> Option<u64> {
    case.get("build")
        .and_then(|build| build.get("duration_us"))
        .and_then(serde_json::Value::as_u64)
}

fn perf_case_binary_size(case: &serde_json::Value) -> Option<u64> {
    case.get("build")
        .and_then(|build| build.get("binary_size_bytes"))
        .and_then(serde_json::Value::as_u64)
}

fn perf_case_run_avg_us(case: &serde_json::Value) -> Option<u64> {
    case.get("run")
        .and_then(|run| run.get("avg_us"))
        .and_then(serde_json::Value::as_u64)
}

fn perf_case_run_median_us(case: &serde_json::Value) -> Option<u64> {
    case.get("run")
        .and_then(|run| run.get("median_us"))
        .and_then(serde_json::Value::as_u64)
}

fn perf_ratio_value(numerator: Option<u64>, denominator: Option<u64>) -> serde_json::Value {
    match (numerator, denominator) {
        (Some(numerator), Some(denominator)) if denominator > 0 => {
            serde_json::json!((numerator as f64) / (denominator as f64))
        }
        _ => serde_json::Value::Null,
    }
}

fn perf_overhead_payload(
    nc_case: &serde_json::Value,
    baseline_case: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "build_time_ratio": perf_ratio_value(
            perf_case_build_us(nc_case),
            perf_case_build_us(baseline_case),
        ),
        "run_time_ratio": perf_ratio_value(
            perf_case_run_avg_us(nc_case),
            perf_case_run_avg_us(baseline_case),
        ),
        "run_time_median_ratio": perf_ratio_value(
            perf_case_run_median_us(nc_case),
            perf_case_run_median_us(baseline_case),
        ),
        "binary_size_ratio": perf_ratio_value(
            perf_case_binary_size(nc_case),
            perf_case_binary_size(baseline_case),
        ),
    })
}

fn perf_overhead_vs_c_matrix_payload(
    nc_case: &serde_json::Value,
    c_matrix: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::Value::Array(
        c_matrix
            .iter()
            .map(|case| {
                let ratios = perf_overhead_payload(nc_case, case);
                serde_json::json!({
                    "case_id": case.get("case_id").and_then(serde_json::Value::as_str),
                    "compiler": case.get("compiler").and_then(serde_json::Value::as_str),
                    "opt_level": case.get("opt_level").and_then(serde_json::Value::as_str),
                    "build_status": case.pointer("/build/status").and_then(serde_json::Value::as_str),
                    "run_status": case.pointer("/run/status").and_then(serde_json::Value::as_str),
                    "ratios": ratios,
                })
            })
            .collect(),
    )
}

fn perf_c_matrix_summary_payload(rows: &[serde_json::Value]) -> serde_json::Value {
    let mut ok_rows = 0u64;
    let mut missing_toolchain_rows = 0u64;
    let mut missing_fixture_rows = 0u64;
    let mut failed_rows = 0u64;
    let mut best_case_id: Option<String> = None;
    let mut best_run_avg_us: Option<u64> = None;

    for row in rows {
        let build_status = row
            .pointer("/build/status")
            .and_then(serde_json::Value::as_str);
        let run_status = row
            .pointer("/run/status")
            .and_then(serde_json::Value::as_str);
        match build_status {
            Some("ok") if run_status == Some("ok") => {
                ok_rows += 1;
                if let Some(avg_us) = perf_case_run_avg_us(row) {
                    if best_run_avg_us.is_none_or(|current| avg_us < current) {
                        best_run_avg_us = Some(avg_us);
                        best_case_id = row
                            .get("case_id")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string);
                    }
                }
            }
            Some("missing_toolchain") => missing_toolchain_rows += 1,
            Some("missing_fixture") => missing_fixture_rows += 1,
            _ => failed_rows += 1,
        }
    }

    serde_json::json!({
        "schema": "ncc.performance.c_style_matrix.summary.v1",
        "planned_rows": rows.len(),
        "ok_rows": ok_rows,
        "missing_toolchain_rows": missing_toolchain_rows,
        "missing_fixture_rows": missing_fixture_rows,
        "failed_rows": failed_rows,
        "best_case_id": best_case_id,
        "best_run_avg_us": best_run_avg_us,
    })
}

fn perf_c_style_corpus_summary_payload(
    benchmarks: &[serde_json::Value],
    corpus_mode: bool,
    iterations: usize,
) -> serde_json::Value {
    let mut rows = Vec::new();
    let mut microbenchmarks = 0u64;
    let mut programs = 0u64;
    for benchmark in benchmarks {
        match benchmark
            .get("fixture_class")
            .and_then(serde_json::Value::as_str)
        {
            Some("microbenchmark") => microbenchmarks += 1,
            Some("program") => programs += 1,
            _ => {}
        }
        if let Some(cases) = benchmark
            .get("c_matrix")
            .and_then(serde_json::Value::as_array)
        {
            rows.extend(cases.iter().cloned());
        }
    }
    let summary = perf_c_matrix_summary_payload(&rows);
    serde_json::json!({
        "schema": "ncc.performance.c_style_corpus.v1",
        "status": "closed_measured",
        "enabled": true,
        "corpus_mode": corpus_mode,
        "iterations": iterations,
        "compilers": perf_c_style_compilers_json(),
        "opt_levels": perf_c_style_opt_levels_json(),
        "benchmark_count": benchmarks.len(),
        "microbenchmarks": microbenchmarks,
        "programs": programs,
        "planned_rows": summary.get("planned_rows").cloned().unwrap_or(serde_json::Value::Null),
        "ok_rows": summary.get("ok_rows").cloned().unwrap_or(serde_json::Value::Null),
        "missing_toolchain_rows": summary.get("missing_toolchain_rows").cloned().unwrap_or(serde_json::Value::Null),
        "missing_fixture_rows": summary.get("missing_fixture_rows").cloned().unwrap_or(serde_json::Value::Null),
        "failed_rows": summary.get("failed_rows").cloned().unwrap_or(serde_json::Value::Null),
        "best_case_id": summary.get("best_case_id").cloned().unwrap_or(serde_json::Value::Null),
        "best_run_avg_us": summary.get("best_run_avg_us").cloned().unwrap_or(serde_json::Value::Null),
        "retention": {
            "ci_artifact": "perf-native-fast.json",
            "dashboard_artifact": "perf-trend-dashboard-local.json",
            "multi_host_certificate_artifact": "perf-host-certification.json",
            "aggregate_artifacts": [
                "perf-trend-dashboard.json",
                "perf-multi-host-certification.json",
                "perf-budget-report.json",
                "perf-baseline-source.json"
            ]
        },
        "policy": "compare_real_nC_native_fast_against_clang_and_gcc_at_O2_and_O3_without_fabricating_missing_toolchains_or_missing_C_fixtures",
    })
}

fn perf_optimizer_guidance_totals(
    benchmark: &serde_json::Value,
) -> Option<BTreeMap<&'static str, u64>> {
    let totals = perf_budget_nc_case(benchmark)?
        .get("build")?
        .get("optimization")?
        .get("internal_optimizer")?
        .get("totals")?;
    let mut row = BTreeMap::new();
    for counter in PERF_OPTIMIZER_GUIDANCE_COUNTERS {
        row.insert(
            counter,
            totals
                .get(counter)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        );
    }
    Some(row)
}

fn perf_optimizer_guidance_roles(counters: &BTreeMap<&'static str, u64>) -> Vec<&'static str> {
    let mut roles = Vec::new();
    if counters
        .get("whole_program_inline_candidates")
        .copied()
        .unwrap_or(0)
        > 0
        || counters.get("inline_expansions").copied().unwrap_or(0) > 0
    {
        roles.push("inline_driver");
    }
    if counters.get("pgo_hot_call_sites").copied().unwrap_or(0) > 0
        || counters.get("pgo_hot_inline_biases").copied().unwrap_or(0) > 0
    {
        roles.push("pgo_hotness_driver");
    }
    if counters
        .get("lto_imported_call_edges")
        .copied()
        .unwrap_or(0)
        > 0
        || counters
            .get("whole_program_cross_container_inlines")
            .copied()
            .unwrap_or(0)
            > 0
        || counters.get("lto_linkage_units").copied().unwrap_or(0) > 1
    {
        roles.push("lto_cross_unit_driver");
    }
    if counters
        .get("whole_program_dead_functions_removed")
        .copied()
        .unwrap_or(0)
        > 0
        || counters.get("pgo_cold_functions").copied().unwrap_or(0) > 0
    {
        roles.push("whole_program_dce_driver");
    }
    if counters
        .get("loop_vectorization_candidates")
        .copied()
        .unwrap_or(0)
        > 0
    {
        roles.push("vectorization_driver");
    }
    roles
}

fn perf_optimizer_guidance_row_payload(benchmark: &serde_json::Value) -> serde_json::Value {
    let counters = perf_optimizer_guidance_totals(benchmark).unwrap_or_default();
    let roles = perf_optimizer_guidance_roles(&counters);
    let optimizer_present = !counters.is_empty();
    let inline_score = counters
        .get("inline_expansions")
        .copied()
        .unwrap_or(0)
        .saturating_add(
            counters
                .get("whole_program_inline_candidates")
                .copied()
                .unwrap_or(0),
        );
    let pgo_score = counters
        .get("pgo_hot_call_sites")
        .copied()
        .unwrap_or(0)
        .saturating_add(counters.get("pgo_hot_inline_biases").copied().unwrap_or(0));
    let lto_score = counters
        .get("lto_imported_call_edges")
        .copied()
        .unwrap_or(0)
        .saturating_add(
            counters
                .get("whole_program_cross_container_inlines")
                .copied()
                .unwrap_or(0),
        )
        .saturating_add(counters.get("lto_linkage_units").copied().unwrap_or(0));
    serde_json::json!({
        "schema": "ncc.performance.optimizer_guidance.row.v1",
        "name": benchmark.get("name").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "fixture_class": benchmark.get("fixture_class").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "has_c_baseline": benchmark.pointer("/baselines/has_c").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "optimizer_present": optimizer_present,
        "roles": roles,
        "scores": {
            "inline": inline_score,
            "pgo": pgo_score,
            "lto": lto_score,
        },
        "counters": counters,
    })
}

fn perf_optimizer_guidance_top_rows(
    rows: &[serde_json::Value],
    metric: &str,
    limit: usize,
) -> Vec<serde_json::Value> {
    let mut scored = rows
        .iter()
        .filter_map(|row| {
            let value = row
                .pointer(&format!("/scores/{metric}"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if value == 0 {
                return None;
            }
            Some((value, row))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left_row), (right_score, right_row)| {
        right_score.cmp(left_score).then_with(|| {
            left_row
                .get("name")
                .and_then(serde_json::Value::as_str)
                .cmp(&right_row.get("name").and_then(serde_json::Value::as_str))
        })
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(score, row)| {
            serde_json::json!({
                "name": row.get("name").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
                "score": score,
                "roles": row.get("roles").cloned().unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect()
}

fn perf_optimizer_guidance_payload(
    benchmarks: &[serde_json::Value],
    corpus_mode: bool,
) -> serde_json::Value {
    let rows = benchmarks
        .iter()
        .map(perf_optimizer_guidance_row_payload)
        .collect::<Vec<_>>();
    let mut totals = BTreeMap::new();
    for counter in PERF_OPTIMIZER_GUIDANCE_COUNTERS {
        let value = rows
            .iter()
            .map(|row| {
                row.get("counters")
                    .and_then(|counters| counters.get(counter))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            })
            .sum::<u64>();
        totals.insert(counter, value);
    }
    let benchmarks_with_optimizer_reports = rows
        .iter()
        .filter(|row| {
            row.get("optimizer_present")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
        .count();
    let inline_drivers = rows
        .iter()
        .filter(|row| {
            row.get("roles")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|roles| {
                    roles
                        .iter()
                        .any(|role| role.as_str() == Some("inline_driver"))
                })
        })
        .count();
    let pgo_drivers = rows
        .iter()
        .filter(|row| {
            row.get("roles")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|roles| {
                    roles
                        .iter()
                        .any(|role| role.as_str() == Some("pgo_hotness_driver"))
                })
        })
        .count();
    let lto_drivers = rows
        .iter()
        .filter(|row| {
            row.get("roles")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|roles| {
                    roles
                        .iter()
                        .any(|role| role.as_str() == Some("lto_cross_unit_driver"))
                })
        })
        .count();

    serde_json::json!({
        "schema": "ncc.performance.optimizer_guidance.v1",
        "status": if corpus_mode { "corpus_guided" } else { "fixed_suite_guided" },
        "source": if corpus_mode { "native_fast_corpus" } else { "native_fast_small_medium_large" },
        "coverage": {
            "benchmark_count": benchmarks.len(),
            "benchmarks_with_optimizer_reports": benchmarks_with_optimizer_reports,
            "inline_drivers": inline_drivers,
            "pgo_hotness_drivers": pgo_drivers,
            "lto_cross_unit_drivers": lto_drivers,
        },
        "totals": totals,
        "top_inline": perf_optimizer_guidance_top_rows(&rows, "inline", 8),
        "top_pgo": perf_optimizer_guidance_top_rows(&rows, "pgo", 8),
        "top_lto": perf_optimizer_guidance_top_rows(&rows, "lto", 8),
        "rows": rows,
        "decisions": {
            "inline": "rank pure inline candidates and expansions by corpus frequency before changing hot/cold expression budgets",
            "pgo": "use loop-contained hot call sites and hot inline biases as the first profile-guided signal for release-fast",
            "lto": "use cross-container inline/import edges and internalization counts to guide whole-program import and DCE thresholds",
        },
        "policy": "derive optimizer tuning evidence from real corpus builds and runs; do not invent missing optimizer reports or external baselines",
    })
}

fn perf_host_payload(label_override: Option<&str>) -> serde_json::Value {
    let label = label_override
        .map(str::to_string)
        .or_else(|| env::var("HOSTNAME").ok())
        .or_else(|| env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "local-host".into());
    serde_json::json!({
        "label": label,
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "family": env::consts::FAMILY,
    })
}

fn perf_report_hash(report: &serde_json::Value) -> String {
    let raw = serde_json::to_vec(report).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(raw);
    format!("{:x}", hasher.finalize())
}

fn perf_case_summary(benchmark: &serde_json::Value, language: &str) -> serde_json::Value {
    let case = benchmark
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .and_then(|cases| {
            cases.iter().find(|case| {
                case.get("language").and_then(serde_json::Value::as_str) == Some(language)
            })
        });
    if let Some(case) = case {
        serde_json::json!({
            "language": language,
            "build_status": case.pointer("/build/status").and_then(serde_json::Value::as_str),
            "run_status": case.pointer("/run/status").and_then(serde_json::Value::as_str),
            "build_us": perf_case_build_us(case),
            "run_avg_us": perf_case_run_avg_us(case),
            "run_median_us": perf_case_run_median_us(case),
            "binary_size_bytes": perf_case_binary_size(case),
        })
    } else {
        serde_json::json!({
            "language": language,
            "build_status": "missing",
            "run_status": "missing",
        })
    }
}

fn perf_c_matrix_trend_payload(benchmark: &serde_json::Value) -> serde_json::Value {
    let rows = benchmark
        .get("c_matrix")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    serde_json::json!({
                        "case_id": row.get("case_id").cloned().unwrap_or(serde_json::Value::Null),
                        "compiler": row.get("compiler").cloned().unwrap_or(serde_json::Value::Null),
                        "opt_level": row.get("opt_level").cloned().unwrap_or(serde_json::Value::Null),
                        "fixture_class": row.get("fixture_class").cloned().unwrap_or(serde_json::Value::Null),
                        "build_status": row.pointer("/build/status").and_then(serde_json::Value::as_str),
                        "run_status": row.pointer("/run/status").and_then(serde_json::Value::as_str),
                        "build_us": perf_case_build_us(row),
                        "run_avg_us": perf_case_run_avg_us(row),
                        "run_median_us": perf_case_run_median_us(row),
                        "binary_size_bytes": perf_case_binary_size(row),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({
        "schema": "ncc.performance.c_style_matrix.trend.v1",
        "summary": benchmark.get("c_matrix_summary").cloned().unwrap_or_else(|| perf_c_matrix_summary_payload(&[])),
        "rows": rows,
        "row_count": rows.len(),
    })
}

fn perf_benchmark_trend_row(
    report: &serde_json::Value,
    benchmark: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "host": report.get("host").cloned().unwrap_or_else(|| perf_host_payload(None)),
        "size": benchmark.get("size").cloned().unwrap_or(serde_json::Value::Null),
        "name": benchmark.get("name").cloned().unwrap_or(serde_json::Value::Null),
        "fixture_class": benchmark.get("fixture_class").cloned().unwrap_or(serde_json::Value::Null),
        "nc": perf_case_summary(benchmark, "nC"),
        "c": perf_case_summary(benchmark, "C"),
        "rust": perf_case_summary(benchmark, "Rust"),
        "c_matrix": perf_c_matrix_trend_payload(benchmark),
        "overhead_vs_c": benchmark.get("overhead_vs_c").cloned().unwrap_or(serde_json::Value::Null),
        "overhead_vs_c_matrix": benchmark.get("overhead_vs_c_matrix").cloned().unwrap_or(serde_json::Value::Null),
        "overhead_vs_rust": benchmark.get("overhead_vs_rust").cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn perf_report_summary_payload(report: &serde_json::Value) -> serde_json::Value {
    let benchmark_count = report
        .get("benchmarks")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    serde_json::json!({
        "schema": report.get("schema").and_then(serde_json::Value::as_str),
        "status": report.get("status").and_then(serde_json::Value::as_str),
        "suite": report.get("suite").and_then(serde_json::Value::as_str),
        "sha256": perf_report_hash(report),
        "host": report.get("host").cloned().unwrap_or_else(|| perf_host_payload(None)),
        "benchmark_count": benchmark_count,
        "iterations": report.get("iterations").and_then(serde_json::Value::as_u64),
        "c_style_corpus": report.get("c_style_corpus").cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn perf_c_style_aggregate_summary_payload(reports: &[serde_json::Value]) -> serde_json::Value {
    let mut planned_rows = 0u64;
    let mut ok_rows = 0u64;
    let mut missing_toolchain_rows = 0u64;
    let mut missing_fixture_rows = 0u64;
    let mut failed_rows = 0u64;
    let mut benchmark_count = 0u64;
    for report in reports {
        benchmark_count += report
            .get("benchmarks")
            .and_then(serde_json::Value::as_array)
            .map(|benchmarks| benchmarks.len() as u64)
            .unwrap_or(0);
        if let Some(summary) = report.get("c_style_corpus") {
            planned_rows += summary
                .get("planned_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            ok_rows += summary
                .get("ok_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            missing_toolchain_rows += summary
                .get("missing_toolchain_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            missing_fixture_rows += summary
                .get("missing_fixture_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            failed_rows += summary
                .get("failed_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
        }
    }
    serde_json::json!({
        "schema": "ncc.performance.c_style_corpus.aggregate.v1",
        "status": "retained",
        "report_count": reports.len(),
        "benchmark_count": benchmark_count,
        "planned_rows": planned_rows,
        "ok_rows": ok_rows,
        "missing_toolchain_rows": missing_toolchain_rows,
        "missing_fixture_rows": missing_fixture_rows,
        "failed_rows": failed_rows,
        "compilers": perf_c_style_compilers_json(),
        "opt_levels": perf_c_style_opt_levels_json(),
    })
}

fn perf_trend_dashboard_payload(report: &serde_json::Value) -> serde_json::Value {
    let rows = report
        .get("benchmarks")
        .and_then(serde_json::Value::as_array)
        .map(|benchmarks| {
            benchmarks
                .iter()
                .map(|benchmark| perf_benchmark_trend_row(report, benchmark))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({
        "schema": "ncc.performance.trend_dashboard.v1",
        "status": "ok",
        "source_schema": report.get("schema").and_then(serde_json::Value::as_str),
        "source_status": report.get("status").and_then(serde_json::Value::as_str),
        "source_suite": report.get("suite").and_then(serde_json::Value::as_str),
        "source_report_sha256": perf_report_hash(report),
        "host": report.get("host").cloned().unwrap_or_else(|| perf_host_payload(None)),
        "reports": [perf_report_summary_payload(report)],
        "report_count": 1,
        "c_style_corpus": report.get("c_style_corpus").cloned().unwrap_or(serde_json::Value::Null),
        "rows": rows,
        "row_count": rows.len(),
    })
}

fn perf_trend_dashboard_payload_many(reports: &[serde_json::Value]) -> serde_json::Value {
    if reports.len() == 1 {
        return perf_trend_dashboard_payload(&reports[0]);
    }
    let mut rows = Vec::new();
    for report in reports {
        if let Some(benchmarks) = report
            .get("benchmarks")
            .and_then(serde_json::Value::as_array)
        {
            for benchmark in benchmarks {
                rows.push(perf_benchmark_trend_row(report, benchmark));
            }
        }
    }
    let report_summaries = reports
        .iter()
        .map(perf_report_summary_payload)
        .collect::<Vec<_>>();
    let all_ok = reports
        .iter()
        .all(|report| report.get("status").and_then(serde_json::Value::as_str) == Some("ok"));
    serde_json::json!({
        "schema": "ncc.performance.trend_dashboard.v1",
        "status": if all_ok { "ok" } else { "partial" },
        "source_schema": "multi_report",
        "source_status": if all_ok { "ok" } else { "partial" },
        "source_suite": "multi_host_native_fast",
        "source_report_sha256": perf_report_hash(&serde_json::Value::Array(reports.to_vec())),
        "host": {
            "label": "multi-host",
            "os": "mixed",
            "arch": "mixed",
            "family": "mixed",
        },
        "reports": report_summaries,
        "report_count": reports.len(),
        "c_style_corpus": perf_c_style_aggregate_summary_payload(reports),
        "rows": rows,
        "row_count": rows.len(),
    })
}

fn perf_certification_payload(
    report: &serde_json::Value,
    host_override: Option<&str>,
) -> serde_json::Value {
    perf_certification_payload_many(std::slice::from_ref(report), host_override)
}

fn perf_certification_host_row(
    report: &serde_json::Value,
    host_override: Option<&str>,
) -> serde_json::Value {
    let benchmark_count = report
        .get("benchmarks")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let host_payload = perf_host_payload(host_override.or_else(|| {
        report
            .pointer("/host/label")
            .and_then(serde_json::Value::as_str)
    }));
    let host_label = host_payload
        .get("label")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("local-host");
    serde_json::json!({
        "host": host_label,
        "host_info": host_payload,
        "status": report.get("status").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "benchmark_count": benchmark_count,
        "fixture_count": report.pointer("/corpus/fixture_count").and_then(serde_json::Value::as_u64).unwrap_or(benchmark_count as u64),
        "iterations": report.get("iterations").and_then(serde_json::Value::as_u64),
        "c_style_corpus": report.get("c_style_corpus").cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn perf_certification_payload_many(
    reports: &[serde_json::Value],
    host_override: Option<&str>,
) -> serde_json::Value {
    let hosts = reports
        .iter()
        .enumerate()
        .map(|(idx, report)| {
            let override_for_row = if reports.len() == 1 {
                host_override
            } else {
                None
            };
            let mut row = perf_certification_host_row(report, override_for_row);
            if let Some(obj) = row.as_object_mut() {
                obj.insert("input_index".into(), serde_json::json!(idx));
                obj.insert(
                    "source_report_sha256".into(),
                    serde_json::json!(perf_report_hash(report)),
                );
            }
            row
        })
        .collect::<Vec<_>>();
    let mut distinct_hosts = BTreeSet::new();
    for host in &hosts {
        if let Some(label) = host.get("host").and_then(serde_json::Value::as_str) {
            distinct_hosts.insert(label.to_string());
        }
    }
    let all_ok = reports
        .iter()
        .all(|report| report.get("status").and_then(serde_json::Value::as_str) == Some("ok"));
    serde_json::json!({
        "schema": "ncc.performance.multi_host_certification.v1",
        "status": if all_ok { "certified" } else { "partial" },
        "source_schema": if reports.len() == 1 {
            reports[0].get("schema").and_then(serde_json::Value::as_str).unwrap_or("unknown")
        } else {
            "multi_report"
        },
        "source_suite": if reports.len() == 1 {
            reports[0].get("suite").and_then(serde_json::Value::as_str).unwrap_or("unknown")
        } else {
            "multi_host_native_fast"
        },
        "source_report_sha256": perf_report_hash(&serde_json::Value::Array(reports.to_vec())),
        "hosts": hosts,
        "host_count": distinct_hosts.len(),
        "report_count": reports.len(),
        "level": if distinct_hosts.len() > 1 { "multi_host" } else { "single_host_artifact" },
        "c_style_corpus": perf_c_style_aggregate_summary_payload(reports),
        "notes": [
            "Certification is generated from a real ncc perf report.",
            "Multi-host aggregation consumes every provided --input report and preserves C-style clang/gcc O2/O3 timing summaries."
        ],
    })
}

#[derive(Clone, Copy)]
struct PerfBudgetPolicy {
    host_class: &'static str,
    max_nc_vs_best_c_run_ratio: f64,
    max_nc_vs_best_c_build_ratio: f64,
    max_nc_run_avg_us: u64,
    max_nc_build_us: u64,
    max_historical_regression_ratio: f64,
    min_c_matrix_ok_rows_for_c_baseline: u64,
}

fn perf_host_class(host: &serde_json::Value) -> String {
    let os = host
        .get("os")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let arch = host
        .get("arch")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    format!("{os}-{arch}")
}

fn perf_budget_policy_for_host(host: &serde_json::Value) -> PerfBudgetPolicy {
    match perf_host_class(host).as_str() {
        "linux-x86_64" => PerfBudgetPolicy {
            host_class: "linux-x86_64",
            max_nc_vs_best_c_run_ratio: 12.0,
            max_nc_vs_best_c_build_ratio: 80.0,
            max_nc_run_avg_us: 750_000,
            max_nc_build_us: 12_000_000,
            max_historical_regression_ratio: 1.35,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        "linux-aarch64" => PerfBudgetPolicy {
            host_class: "linux-aarch64",
            max_nc_vs_best_c_run_ratio: 16.0,
            max_nc_vs_best_c_build_ratio: 95.0,
            max_nc_run_avg_us: 1_000_000,
            max_nc_build_us: 15_000_000,
            max_historical_regression_ratio: 1.40,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        "macos-x86_64" => PerfBudgetPolicy {
            host_class: "macos-x86_64",
            max_nc_vs_best_c_run_ratio: 14.0,
            max_nc_vs_best_c_build_ratio: 90.0,
            max_nc_run_avg_us: 900_000,
            max_nc_build_us: 14_000_000,
            max_historical_regression_ratio: 1.40,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        "macos-aarch64" => PerfBudgetPolicy {
            host_class: "macos-aarch64",
            max_nc_vs_best_c_run_ratio: 14.0,
            max_nc_vs_best_c_build_ratio: 90.0,
            max_nc_run_avg_us: 900_000,
            max_nc_build_us: 14_000_000,
            max_historical_regression_ratio: 1.40,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        "windows-x86_64" => PerfBudgetPolicy {
            host_class: "windows-x86_64",
            max_nc_vs_best_c_run_ratio: 20.0,
            max_nc_vs_best_c_build_ratio: 120.0,
            max_nc_run_avg_us: 1_500_000,
            max_nc_build_us: 20_000_000,
            max_historical_regression_ratio: 1.50,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        "windows-aarch64" => PerfBudgetPolicy {
            host_class: "windows-aarch64",
            max_nc_vs_best_c_run_ratio: 24.0,
            max_nc_vs_best_c_build_ratio: 140.0,
            max_nc_run_avg_us: 1_800_000,
            max_nc_build_us: 24_000_000,
            max_historical_regression_ratio: 1.55,
            min_c_matrix_ok_rows_for_c_baseline: 4,
        },
        _ => PerfBudgetPolicy {
            host_class: "generic",
            max_nc_vs_best_c_run_ratio: 25.0,
            max_nc_vs_best_c_build_ratio: 150.0,
            max_nc_run_avg_us: 2_000_000,
            max_nc_build_us: 30_000_000,
            max_historical_regression_ratio: 1.60,
            min_c_matrix_ok_rows_for_c_baseline: 1,
        },
    }
}

fn perf_budget_policy_payload(policy: PerfBudgetPolicy) -> serde_json::Value {
    serde_json::json!({
        "schema": "ncc.performance.budget.policy.v1",
        "host_class": policy.host_class,
        "max_nc_vs_best_c_run_ratio": policy.max_nc_vs_best_c_run_ratio,
        "max_nc_vs_best_c_build_ratio": policy.max_nc_vs_best_c_build_ratio,
        "max_nc_run_avg_us": policy.max_nc_run_avg_us,
        "max_nc_build_us": policy.max_nc_build_us,
        "max_historical_regression_ratio": policy.max_historical_regression_ratio,
        "min_c_matrix_ok_rows_for_c_baseline": policy.min_c_matrix_ok_rows_for_c_baseline,
    })
}

fn perf_budget_baseline_key(report: &serde_json::Value, benchmark: &serde_json::Value) -> String {
    let host = report
        .get("host")
        .cloned()
        .unwrap_or_else(|| perf_host_payload(None));
    let name = benchmark
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    format!("{}::{name}", perf_host_class(&host))
}

fn perf_budget_baseline_key_from_row(row: &serde_json::Value) -> Option<String> {
    let name = row.get("name").and_then(serde_json::Value::as_str)?;
    let host_class = row
        .get("host_class")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| row.get("host").map(perf_host_class))?;
    Some(format!("{host_class}::{name}"))
}

fn perf_budget_nc_case(benchmark: &serde_json::Value) -> Option<&serde_json::Value> {
    benchmark
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .and_then(|cases| {
            cases
                .iter()
                .find(|case| case.get("language").and_then(serde_json::Value::as_str) == Some("nC"))
        })
}

fn perf_budget_best_c_matrix_case(benchmark: &serde_json::Value) -> Option<&serde_json::Value> {
    benchmark
        .get("c_matrix")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .filter(|row| {
            row.pointer("/build/status")
                .and_then(serde_json::Value::as_str)
                == Some("ok")
                && row
                    .pointer("/run/status")
                    .and_then(serde_json::Value::as_str)
                    == Some("ok")
        })
        .min_by_key(|row| perf_case_run_avg_us(row).unwrap_or(u64::MAX))
}

fn perf_budget_baseline_metrics(
    baseline_reports: &[serde_json::Value],
) -> BTreeMap<String, (u64, u64)> {
    let mut map = BTreeMap::new();
    for report in baseline_reports {
        if report.get("schema").and_then(serde_json::Value::as_str)
            == Some("ncc.performance.budget.v1")
        {
            if let Some(rows) = report.get("rows").and_then(serde_json::Value::as_array) {
                for row in rows {
                    let Some(key) = perf_budget_baseline_key_from_row(row) else {
                        continue;
                    };
                    let Some(run_avg_us) = row
                        .pointer("/nc/run_avg_us")
                        .and_then(serde_json::Value::as_u64)
                    else {
                        continue;
                    };
                    let Some(build_us) = row
                        .pointer("/nc/build_us")
                        .and_then(serde_json::Value::as_u64)
                    else {
                        continue;
                    };
                    map.insert(key, (run_avg_us, build_us));
                }
            }
            continue;
        }
        if let Some(benchmarks) = report
            .get("benchmarks")
            .and_then(serde_json::Value::as_array)
        {
            for benchmark in benchmarks {
                let Some(nc_case) = perf_budget_nc_case(benchmark) else {
                    continue;
                };
                let Some(run_avg_us) = perf_case_run_avg_us(nc_case) else {
                    continue;
                };
                let Some(build_us) = perf_case_build_us(nc_case) else {
                    continue;
                };
                map.insert(
                    perf_budget_baseline_key(report, benchmark),
                    (run_avg_us, build_us),
                );
            }
        }
    }
    map
}

fn perf_budget_baseline_source_payload(report: &serde_json::Value) -> serde_json::Value {
    let schema = report
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let row_count = if schema == "ncc.performance.budget.v1" {
        report
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len)
    } else {
        report
            .get("benchmarks")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len)
    };
    serde_json::json!({
        "schema": "ncc.performance.budget.baseline_source.v1",
        "source_schema": schema,
        "source_report_sha256": perf_report_hash(report),
        "row_count": row_count,
        "accepted_as": match schema {
            "ncc.performance.budget.v1" => "retained_budget_report",
            "ncc.performance.native_fast.compare.v1" => "native_fast_report",
            _ => "unsupported_report",
        },
    })
}

fn perf_budget_row_payload(
    report: &serde_json::Value,
    benchmark: &serde_json::Value,
    policy: PerfBudgetPolicy,
    baselines: &BTreeMap<String, (u64, u64)>,
) -> serde_json::Value {
    let host = report
        .get("host")
        .cloned()
        .unwrap_or_else(|| perf_host_payload(None));
    let name = benchmark
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let fixture_class = benchmark
        .get("fixture_class")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let has_c_baseline = benchmark
        .pointer("/baselines/has_c")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let Some(nc_case) = perf_budget_nc_case(benchmark) else {
        return serde_json::json!({
            "schema": "ncc.performance.budget.row.v1",
            "status": "warning",
            "reason": "missing_nc_case",
            "host": host,
            "name": name,
            "fixture_class": fixture_class,
        });
    };
    if !has_c_baseline {
        return serde_json::json!({
            "schema": "ncc.performance.budget.row.v1",
            "status": "inventory",
            "reason": "no_c_baseline",
            "host": host,
            "name": name,
            "fixture_class": fixture_class,
            "nc": perf_case_summary(benchmark, "nC"),
        });
    }

    let c_rows: &[serde_json::Value] = benchmark
        .get("c_matrix")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let c_ok_rows = c_rows
        .iter()
        .filter(|row| {
            row.pointer("/build/status")
                .and_then(serde_json::Value::as_str)
                == Some("ok")
                && row
                    .pointer("/run/status")
                    .and_then(serde_json::Value::as_str)
                    == Some("ok")
        })
        .count() as u64;
    let best_c = perf_budget_best_c_matrix_case(benchmark);
    let nc_build_us = perf_case_build_us(nc_case);
    let nc_run_avg_us = perf_case_run_avg_us(nc_case);
    let best_c_build_us = best_c.and_then(perf_case_build_us);
    let best_c_run_avg_us = best_c.and_then(perf_case_run_avg_us);
    let build_ratio = match (nc_build_us, best_c_build_us) {
        (Some(nc), Some(c)) if c > 0 => Some(nc as f64 / c as f64),
        _ => None,
    };
    let run_ratio = match (nc_run_avg_us, best_c_run_avg_us) {
        (Some(nc), Some(c)) if c > 0 => Some(nc as f64 / c as f64),
        _ => None,
    };
    let baseline_key = perf_budget_baseline_key(report, benchmark);
    let baseline = baselines.get(&baseline_key).copied();
    let historical_run_ratio = match (nc_run_avg_us, baseline.map(|(run, _)| run)) {
        (Some(current), Some(previous)) if previous > 0 => Some(current as f64 / previous as f64),
        _ => None,
    };
    let historical_build_ratio = match (nc_build_us, baseline.map(|(_, build)| build)) {
        (Some(current), Some(previous)) if previous > 0 => Some(current as f64 / previous as f64),
        _ => None,
    };

    let mut failures = Vec::new();
    let mut warnings = Vec::new();
    if c_ok_rows < policy.min_c_matrix_ok_rows_for_c_baseline {
        warnings.push(serde_json::json!({
            "kind": "insufficient_c_matrix_evidence",
            "actual": c_ok_rows,
            "budget": policy.min_c_matrix_ok_rows_for_c_baseline,
        }));
    }
    if let Some(value) = nc_run_avg_us {
        if value > policy.max_nc_run_avg_us {
            failures.push(serde_json::json!({
                "kind": "nc_run_avg_us",
                "actual": value,
                "budget": policy.max_nc_run_avg_us,
            }));
        }
    }
    if let Some(value) = nc_build_us {
        if value > policy.max_nc_build_us {
            failures.push(serde_json::json!({
                "kind": "nc_build_us",
                "actual": value,
                "budget": policy.max_nc_build_us,
            }));
        }
    }
    if let Some(value) = run_ratio {
        if value > policy.max_nc_vs_best_c_run_ratio {
            failures.push(serde_json::json!({
                "kind": "nc_vs_best_c_run_ratio",
                "actual": value,
                "budget": policy.max_nc_vs_best_c_run_ratio,
            }));
        }
    }
    if let Some(value) = build_ratio {
        if value > policy.max_nc_vs_best_c_build_ratio {
            failures.push(serde_json::json!({
                "kind": "nc_vs_best_c_build_ratio",
                "actual": value,
                "budget": policy.max_nc_vs_best_c_build_ratio,
            }));
        }
    }
    if let Some(value) = historical_run_ratio {
        if value > policy.max_historical_regression_ratio {
            failures.push(serde_json::json!({
                "kind": "historical_nc_run_regression",
                "actual": value,
                "budget": policy.max_historical_regression_ratio,
            }));
        }
    }
    if let Some(value) = historical_build_ratio {
        if value > policy.max_historical_regression_ratio {
            failures.push(serde_json::json!({
                "kind": "historical_nc_build_regression",
                "actual": value,
                "budget": policy.max_historical_regression_ratio,
            }));
        }
    }

    let status = if !failures.is_empty() {
        "regressed"
    } else if !warnings.is_empty() {
        "warning"
    } else {
        "ok"
    };

    serde_json::json!({
        "schema": "ncc.performance.budget.row.v1",
        "status": status,
        "host": host,
        "host_class": policy.host_class,
        "name": name,
        "fixture_class": fixture_class,
        "nc": {
            "build_us": nc_build_us,
            "run_avg_us": nc_run_avg_us,
            "build_status": nc_case.pointer("/build/status").and_then(serde_json::Value::as_str),
            "run_status": nc_case.pointer("/run/status").and_then(serde_json::Value::as_str),
        },
        "best_c": best_c.map(|row| serde_json::json!({
            "case_id": row.get("case_id").and_then(serde_json::Value::as_str),
            "compiler": row.get("compiler").and_then(serde_json::Value::as_str),
            "opt_level": row.get("opt_level").and_then(serde_json::Value::as_str),
            "build_us": best_c_build_us,
            "run_avg_us": best_c_run_avg_us,
        })),
        "ratios": {
            "nc_vs_best_c_build": build_ratio,
            "nc_vs_best_c_run": run_ratio,
            "historical_nc_build": historical_build_ratio,
            "historical_nc_run": historical_run_ratio,
        },
        "c_matrix_ok_rows": c_ok_rows,
        "failures": failures,
        "warnings": warnings,
    })
}

fn perf_budget_payload_many(
    reports: &[serde_json::Value],
    baseline_reports: &[serde_json::Value],
) -> serde_json::Value {
    let baselines = perf_budget_baseline_metrics(baseline_reports);
    let mut rows = Vec::new();
    let mut policy_rows = BTreeMap::new();
    for report in reports {
        let host = report
            .get("host")
            .cloned()
            .unwrap_or_else(|| perf_host_payload(None));
        let policy = perf_budget_policy_for_host(&host);
        policy_rows.insert(
            policy.host_class.to_string(),
            perf_budget_policy_payload(policy),
        );
        if let Some(benchmarks) = report
            .get("benchmarks")
            .and_then(serde_json::Value::as_array)
        {
            for benchmark in benchmarks {
                rows.push(perf_budget_row_payload(
                    report, benchmark, policy, &baselines,
                ));
            }
        }
    }

    let regressed_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(serde_json::Value::as_str) == Some("regressed"))
        .count();
    let warning_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(serde_json::Value::as_str) == Some("warning"))
        .count();
    let inventory_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(serde_json::Value::as_str) == Some("inventory"))
        .count();
    let ok_rows = rows
        .iter()
        .filter(|row| row.get("status").and_then(serde_json::Value::as_str) == Some("ok"))
        .count();
    let historical_comparisons = rows
        .iter()
        .filter(|row| {
            row.pointer("/ratios/historical_nc_run")
                .and_then(serde_json::Value::as_f64)
                .is_some()
                || row
                    .pointer("/ratios/historical_nc_build")
                    .and_then(serde_json::Value::as_f64)
                    .is_some()
        })
        .count();
    let status = if regressed_rows > 0 {
        "regressed"
    } else if warning_rows > 0 {
        "warning"
    } else {
        "ok"
    };

    serde_json::json!({
        "schema": "ncc.performance.budget.v1",
        "status": status,
        "report_count": reports.len(),
        "baseline_report_count": baseline_reports.len(),
        "source_report_sha256": perf_report_hash(&serde_json::Value::Array(reports.to_vec())),
        "baseline_report_sha256": if baseline_reports.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(perf_report_hash(&serde_json::Value::Array(baseline_reports.to_vec())))
        },
        "baseline_sources": baseline_reports
            .iter()
            .map(perf_budget_baseline_source_payload)
            .collect::<Vec<_>>(),
        "policy": {
            "schema": "ncc.performance.budget.policy_set.v1",
            "mode": "host_thresholds_plus_optional_historical_baseline",
            "host_policies": policy_rows.values().cloned().collect::<Vec<_>>(),
        },
        "summary": {
            "row_count": rows.len(),
            "ok_rows": ok_rows,
            "warning_rows": warning_rows,
            "regressed_rows": regressed_rows,
            "inventory_rows": inventory_rows,
            "historical_comparisons": historical_comparisons,
        },
        "rows": rows,
        "notes": [
            "Budgets compare nC native-fast timings against the best real C matrix row for each host and benchmark.",
            "When --baseline is provided, current nC build/run timings are compared against the matching historical host-class benchmark.",
            "Rows without a C baseline are retained as inventory and do not count as regressions."
        ],
    })
}

fn perf_write_json(
    path: &str,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_output_parent_dir(path)?;
    fs::write(path, serde_json::to_string_pretty(payload)?)?;
    Ok(())
}

fn perf_native_fast_corpus_payload(
    suite_dir: &Path,
    iterations: usize,
    include_c_matrix: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let contract_catalog = perf_contract_catalog_for_suite(suite_dir)?;
    let mut fixtures = fs::read_dir(suite_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("n"))
        .collect::<Vec<_>>();
    fixtures.sort();

    let mut without_c_baseline = 0u64;
    let mut without_rust_baseline = 0u64;
    let mut benchmarks = Vec::new();
    let tmp_dir = make_temp_path("ncc_perf_native_fast_corpus", "bundle");
    fs::create_dir_all(&tmp_dir)?;
    for source in &fixtures {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown");
        let fixture_class = perf_fixture_class("corpus", stem);
        let c_baseline = suite_dir.join(format!("{stem}.c"));
        let rust_baseline = suite_dir.join(format!("{stem}.rs"));
        if !c_baseline.is_file() {
            without_c_baseline += 1;
        }
        if !rust_baseline.is_file() {
            without_rust_baseline += 1;
        }
        let ncc_out = perf_executable_path(&tmp_dir, &format!("{}_nc", perf_safe_stem(stem)));
        let c_out = perf_executable_path(&tmp_dir, &format!("{}_c_cc_o2", perf_safe_stem(stem)));
        let rust_out = perf_executable_path(&tmp_dir, &format!("{}_rust", perf_safe_stem(stem)));
        let comparable_with_c = c_baseline.is_file();
        let ncc_build = if comparable_with_c {
            perf_ncc_native_build(source, &ncc_out)
        } else {
            serde_json::json!({
                "language": "nC",
                "tool": "ncc",
                "status": "enumerated",
                "duration_us": 0u64,
                "binary_size_bytes": serde_json::Value::Null,
                "reason": "corpus_fixture_without_c_baseline_is_inventory_only_for_c_style_comparison",
                "command": format!("ncc internal native-fast build {}", source.display()),
            })
        };
        let c_build = if c_baseline.is_file() {
            perf_external_build(
                "C",
                "cc",
                &[
                    "-O2".to_string(),
                    c_baseline.display().to_string(),
                    "-o".to_string(),
                    c_out.display().to_string(),
                ],
                &c_out,
            )
        } else {
            perf_missing_fixture_build(
                "C",
                "cc",
                &c_baseline,
                format!("cc -O2 {} -o {}", c_baseline.display(), c_out.display()),
            )
        };
        let rust_build = if comparable_with_c && rust_baseline.is_file() {
            perf_external_build(
                "Rust",
                "rustc",
                &[
                    "-C".to_string(),
                    "opt-level=3".to_string(),
                    rust_baseline.display().to_string(),
                    "-o".to_string(),
                    rust_out.display().to_string(),
                ],
                &rust_out,
            )
        } else {
            perf_missing_fixture_build(
                "Rust",
                "rustc",
                &rust_baseline,
                format!(
                    "rustc -C opt-level=3 {} -o {}",
                    rust_baseline.display(),
                    rust_out.display()
                ),
            )
        };
        let nc_case = perf_case_payload("nC", source, &ncc_out, ncc_build, iterations);
        let c_case = perf_case_payload("C", &c_baseline, &c_out, c_build, iterations);
        let rust_case =
            perf_case_payload("Rust", &rust_baseline, &rust_out, rust_build, iterations);
        let c_matrix = if include_c_matrix {
            perf_c_style_matrix_cases(&c_baseline, &tmp_dir, stem, fixture_class, iterations)
        } else {
            Vec::new()
        };
        let c_matrix_summary = perf_c_matrix_summary_payload(&c_matrix);
        let overhead_vs_c = perf_overhead_payload(&nc_case, &c_case);
        let overhead_vs_rust = perf_overhead_payload(&nc_case, &rust_case);
        let overhead_vs_c_matrix = perf_overhead_vs_c_matrix_payload(&nc_case, &c_matrix);
        let cases = vec![nc_case, c_case, rust_case];
        benchmarks.push(serde_json::json!({
            "size": "corpus",
            "name": stem,
            "fixture_class": fixture_class,
            "cases": cases,
            "baselines": {
                "c": if c_baseline.is_file() { c_baseline.display().to_string() } else { String::new() },
                "rust": if rust_baseline.is_file() { rust_baseline.display().to_string() } else { String::new() },
                "has_c": c_baseline.is_file(),
                "has_rust": rust_baseline.is_file(),
                "measured": comparable_with_c,
            },
            "c_matrix": c_matrix,
            "c_matrix_summary": c_matrix_summary,
            "overhead_vs_c": overhead_vs_c,
            "overhead_vs_c_matrix": overhead_vs_c_matrix,
            "overhead_vs_rust": overhead_vs_rust,
            "semantic_equivalence": perf_semantic_equivalence_payload(&contract_catalog, stem)?,
        }));
    }
    let cases = benchmarks
        .first()
        .and_then(|benchmark| benchmark.get("cases"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let c_style_corpus = perf_c_style_corpus_summary_payload(&benchmarks, true, iterations);
    let optimizer_guidance = perf_optimizer_guidance_payload(&benchmarks, true);

    Ok(serde_json::json!({
        "schema": "ncc.performance.native_fast.compare.v1",
        "status": if fixtures.is_empty() { "failed" } else { "ok" },
        "bench_harness": "ncc",
        "suite": "native_fast_corpus",
        "corpus_mode": true,
        "scope": "native_fast_fixture_corpus_c_comparable_measured",
        "iterations": iterations,
        "host": perf_host_payload(None),
        "corpus": {
            "root": suite_dir.display().to_string(),
            "fixture_count": fixtures.len(),
            "with_c_baseline": fixtures.len() as u64 - without_c_baseline,
            "without_c_baseline": without_c_baseline,
            "with_rust_baseline": fixtures.len() as u64 - without_rust_baseline,
            "without_rust_baseline": without_rust_baseline,
            "execution_policy": "measure_c_comparable_nc_fixtures_and_inventory_nc_only_fixtures_without_fabricating_missing_c_or_rust_baselines",
        },
        "benchmark_contract_catalog": perf_contract_catalog_summary(&contract_catalog),
        "c_style_corpus": c_style_corpus,
        "optimizer_guidance": optimizer_guidance,
        "benchmarks": benchmarks,
        "cases": cases,
        "toolchains": {
            "ncc": command_version_line("ncc", &["--help"]),
            "cc": command_version_line("cc", &["--version"]),
            "c_style": perf_c_style_toolchains_payload(),
            "rustc": command_version_line("rustc", &["--version"]),
        },
        "notes": [
            "Corpus mode inventories the whole native_fast nC fixture corpus.",
            "Fixtures with C baselines are measured through nC, cc -O2, Rust when present, and clang/gcc O2/O3.",
            "Each fixture with a C baseline is compared against clang/gcc at O2 and O3.",
            "Fixtures without C/Rust baselines are explicitly counted instead of fabricating overhead ratios.",
            "Build or run failures remain row-level evidence instead of aborting the corpus report."
        ],
    }))
}

fn resolve_perf_suite_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut seeds = Vec::new();
    seeds.push(env::current_dir()?);
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        seeds.push(PathBuf::from(manifest_dir));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            seeds.push(parent.to_path_buf());
        }
    }

    for seed in seeds {
        for base in seed.ancestors() {
            let required = [
                "minimal_return.n",
                "minimal_return.c",
                "minimal_return.rs",
                "bench_medium.n",
                "bench_medium.c",
                "bench_medium.rs",
                "bench_large.n",
                "bench_large.c",
                "bench_large.rs",
                "bench_workload.n",
                "bench_workload.c",
                "bench_workload.rs",
            ];
            for candidate in [
                base.join("examples/performance/native_fast"),
                base.join("_archive/old_examples/performance/native_fast"),
            ] {
                if required
                    .iter()
                    .all(|fixture| candidate.join(fixture).is_file())
                {
                    return Ok(candidate);
                }
            }
        }
    }

    Err("perf native-fast suite not found; expected examples/performance/native_fast or _archive/old_examples/performance/native_fast small/medium/large nC/C/Rust fixtures".into())
}

fn run_perf_native_fast(
    iterations: usize,
    out_path: Option<&str>,
    dashboard_out: Option<&str>,
    cert_out: Option<&str>,
    corpus_mode: bool,
    include_c_matrix: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let suite_dir = resolve_perf_suite_dir()?;
    if corpus_mode {
        let payload = perf_native_fast_corpus_payload(&suite_dir, iterations, include_c_matrix)?;
        let rendered = serde_json::to_string_pretty(&payload)?;
        if let Some(out) = out_path {
            ensure_output_parent_dir(out)?;
            fs::write(out, rendered.as_bytes())?;
        }
        if let Some(out) = dashboard_out {
            perf_write_json(out, &perf_trend_dashboard_payload(&payload))?;
        }
        if let Some(out) = cert_out {
            perf_write_json(out, &perf_certification_payload(&payload, None))?;
        }
        if json || out_path.is_none() {
            println!("{rendered}");
        } else {
            println!("OK: perf native-fast {}", out_path.unwrap_or(""));
        }
        return Ok(());
    }

    let contract_catalog = perf_contract_catalog_for_suite(&suite_dir)?;
    let tmp_dir = make_temp_path("ncc_perf_native_fast", "bundle");
    fs::create_dir_all(&tmp_dir)?;

    let benchmark_specs = [
        (
            "small",
            "minimal_process_entry",
            "minimal_return.n",
            "minimal_return.c",
            "minimal_return.rs",
        ),
        (
            "medium",
            "arithmetic_call_pipeline",
            "bench_medium.n",
            "bench_medium.c",
            "bench_medium.rs",
        ),
        (
            "large",
            "volatile_loop_memory_pipeline",
            "bench_workload.n",
            "bench_workload.c",
            "bench_workload.rs",
        ),
    ];

    let mut benchmarks = Vec::new();
    for (size, name, ncc_file, c_file, rust_file) in benchmark_specs {
        let ncc_source = suite_dir.join(ncc_file);
        let c_source = suite_dir.join(c_file);
        let rust_source = suite_dir.join(rust_file);

        let ncc_out = perf_executable_path(&tmp_dir, &format!("{name}_nc"));
        let c_out = perf_executable_path(&tmp_dir, &format!("{name}_c"));
        let rust_out = perf_executable_path(&tmp_dir, &format!("{name}_rust"));
        let fixture_class = perf_fixture_class(size, name);

        let ncc_build = perf_ncc_native_build(&ncc_source, &ncc_out);
        let c_build = perf_external_build(
            "C",
            "cc",
            &[
                "-O2".to_string(),
                c_source.display().to_string(),
                "-o".to_string(),
                c_out.display().to_string(),
            ],
            &c_out,
        );
        let rust_build = perf_external_build(
            "Rust",
            "rustc",
            &[
                "-C".to_string(),
                "opt-level=3".to_string(),
                rust_source.display().to_string(),
                "-o".to_string(),
                rust_out.display().to_string(),
            ],
            &rust_out,
        );

        let cases = vec![
            perf_case_payload("nC", &ncc_source, &ncc_out, ncc_build, iterations),
            perf_case_payload("C", &c_source, &c_out, c_build, iterations),
            perf_case_payload("Rust", &rust_source, &rust_out, rust_build, iterations),
        ];
        let nc_case = &cases[0];
        let c_case = &cases[1];
        let rust_case = &cases[2];
        let c_matrix = if include_c_matrix {
            perf_c_style_matrix_cases(&c_source, &tmp_dir, name, fixture_class, iterations)
        } else {
            Vec::new()
        };
        let c_matrix_summary = perf_c_matrix_summary_payload(&c_matrix);
        let overhead_vs_c = perf_overhead_payload(nc_case, c_case);
        let overhead_vs_c_matrix = perf_overhead_vs_c_matrix_payload(nc_case, &c_matrix);
        let overhead_vs_rust = perf_overhead_payload(nc_case, rust_case);
        benchmarks.push(serde_json::json!({
            "size": size,
            "name": name,
            "fixture_class": fixture_class,
            "cases": cases,
            "c_matrix": c_matrix,
            "c_matrix_summary": c_matrix_summary,
            "overhead_vs_c": overhead_vs_c,
            "overhead_vs_c_matrix": overhead_vs_c_matrix,
            "overhead_vs_rust": overhead_vs_rust,
            "semantic_equivalence": perf_semantic_equivalence_payload(&contract_catalog, name)?,
        }));
    }

    let cases = benchmarks
        .first()
        .and_then(|benchmark| benchmark.get("cases"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ok_cases = benchmarks
        .iter()
        .flat_map(|benchmark| {
            benchmark
                .get("cases")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|case| {
            case.get("build")
                .and_then(|build| build.get("status"))
                .and_then(serde_json::Value::as_str)
                == Some("ok")
        })
        .count();
    let c_style_corpus = perf_c_style_corpus_summary_payload(&benchmarks, false, iterations);
    let optimizer_guidance = perf_optimizer_guidance_payload(&benchmarks, false);
    let payload = serde_json::json!({
        "schema": "ncc.performance.native_fast.compare.v1",
        "status": if ok_cases > 0 { "ok" } else { "failed" },
        "bench_harness": "ncc",
        "suite": "native_fast_small_medium_large",
        "corpus_mode": false,
        "scope": "build_stages_and_post_build_execution",
        "iterations": iterations,
        "host": perf_host_payload(None),
        "sizes": ["small", "medium", "large"],
        "benchmark_contract_catalog": perf_contract_catalog_summary(&contract_catalog),
        "metrics": [
            "lex_us",
            "parse_us",
            "sema_us",
            "ir_lower_us",
            "backend_emit_us",
            "object_emit_us",
            "link_us",
            "binary_size_bytes",
            "measured_elf_object_bytes",
            "text_bytes",
            "run_avg_us",
            "run_median_us",
            "run_p95_us",
            "overhead_vs_c",
            "overhead_vs_c_matrix",
            "overhead_vs_rust"
        ],
        "c_style_corpus": c_style_corpus,
        "optimizer_guidance": optimizer_guidance,
        "toolchains": {
            "ncc": command_version_line("ncc", &["--help"]),
            "cc": command_version_line("cc", &["--version"]),
            "c_style": perf_c_style_toolchains_payload(),
            "rustc": command_version_line("rustc", &["--version"]),
        },
        "benchmarks": benchmarks,
        "cases": cases,
        "notes": [
            "The nC case measures the native-fast hosted process entry route.",
            "The nC case uses the same internal OS executable linker selected by normal build/run when the target is supported.",
            "The C-style matrix compares every C baseline against clang/gcc at O2 and O3.",
            "The nC build case embeds the optimizer report used for the measured artifact.",
            "The nC pipeline report measures parser, semantic analysis, IR lowering, backend, object bytes and linking.",
            "Execution samples use a monotonic clock after build, bounded warmups, median and p95 statistics.",
            "The medium fixture executes five million dependent arithmetic updates and requires its exact final value.",
            "The large fixture executes twenty million raw 32-bit volatile load/update/store iterations and requires exact final cell, accumulator and iteration values.",
            "This harness measures real local commands and marks missing tools explicitly.",
            "It does not claim optimizer parity with C/Rust by itself."
        ],
    });

    let rendered = serde_json::to_string_pretty(&payload)?;
    if let Some(out) = out_path {
        ensure_output_parent_dir(out)?;
        fs::write(out, rendered.as_bytes())?;
    }
    if let Some(out) = dashboard_out {
        perf_write_json(out, &perf_trend_dashboard_payload(&payload))?;
    }
    if let Some(out) = cert_out {
        perf_write_json(out, &perf_certification_payload(&payload, None))?;
    }

    if json || out_path.is_none() {
        println!("{rendered}");
    } else {
        println!("OK: perf native-fast {}", out_path.unwrap_or(""));
    }
    Ok(())
}

fn run_perf_trend_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut input_paths: Vec<String> = Vec::new();
    let mut out_path: Option<String> = None;
    let mut json = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--input" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf trend syntax: --input requires a file path",
                    )
                })?;
                input_paths.push(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf trend syntax: --out requires a file path",
                    )
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "perf",
                    format!("perf trend: unknown option '{}'", other),
                )
                .into());
            }
        }
    }
    if input_paths.is_empty() {
        return Err(
            actionable_command_error("perf", "perf trend syntax: --input is required").into(),
        );
    }
    let mut reports = Vec::new();
    for input in &input_paths {
        let raw = fs::read_to_string(input)?;
        reports.push(serde_json::from_str::<serde_json::Value>(&raw)?);
    }
    let payload = perf_trend_dashboard_payload_many(&reports);
    let rendered = serde_json::to_string_pretty(&payload)?;
    if let Some(out) = out_path.as_deref() {
        ensure_output_parent_dir(out)?;
        fs::write(out, rendered.as_bytes())?;
    }
    if json || out_path.is_none() {
        println!("{rendered}");
    } else {
        println!("OK: perf trend {}", out_path.unwrap_or_default());
    }
    Ok(())
}

fn run_perf_certify_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut input_paths: Vec<String> = Vec::new();
    let mut out_path: Option<String> = None;
    let mut host: Option<String> = None;
    let mut json = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--input" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf certify syntax: --input requires a file path",
                    )
                })?;
                input_paths.push(value.clone());
                idx += 2;
            }
            "--host" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("perf", "perf certify syntax: --host requires a value")
                })?;
                host = Some(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf certify syntax: --out requires a file path",
                    )
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "perf",
                    format!("perf certify: unknown option '{}'", other),
                )
                .into());
            }
        }
    }
    if input_paths.is_empty() {
        return Err(
            actionable_command_error("perf", "perf certify syntax: --input is required").into(),
        );
    }
    let mut reports = Vec::new();
    for input in &input_paths {
        let raw = fs::read_to_string(input)?;
        reports.push(serde_json::from_str::<serde_json::Value>(&raw)?);
    }
    let payload = perf_certification_payload_many(&reports, host.as_deref());
    let rendered = serde_json::to_string_pretty(&payload)?;
    if let Some(out) = out_path.as_deref() {
        ensure_output_parent_dir(out)?;
        fs::write(out, rendered.as_bytes())?;
    }
    if json || out_path.is_none() {
        println!("{rendered}");
    } else {
        println!("OK: perf certify {}", out_path.unwrap_or_default());
    }
    Ok(())
}

fn run_perf_budget_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut input_paths: Vec<String> = Vec::new();
    let mut baseline_paths: Vec<String> = Vec::new();
    let mut out_path: Option<String> = None;
    let mut json = false;
    let mut fail_on_regression = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--fail-on-regression" => {
                fail_on_regression = true;
                idx += 1;
            }
            "--input" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf budget syntax: --input requires a file path",
                    )
                })?;
                input_paths.push(value.clone());
                idx += 2;
            }
            "--baseline" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf budget syntax: --baseline requires a file path",
                    )
                })?;
                baseline_paths.push(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf budget syntax: --out requires a file path",
                    )
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "perf",
                    format!("perf budget: unknown option '{}'", other),
                )
                .into());
            }
        }
    }
    if input_paths.is_empty() {
        return Err(
            actionable_command_error("perf", "perf budget syntax: --input is required").into(),
        );
    }

    let mut reports = Vec::new();
    for input in &input_paths {
        let raw = fs::read_to_string(input)?;
        reports.push(serde_json::from_str::<serde_json::Value>(&raw)?);
    }
    let mut baseline_reports = Vec::new();
    for input in &baseline_paths {
        let raw = fs::read_to_string(input)?;
        baseline_reports.push(serde_json::from_str::<serde_json::Value>(&raw)?);
    }
    let payload = perf_budget_payload_many(&reports, &baseline_reports);
    let rendered = serde_json::to_string_pretty(&payload)?;
    if let Some(out) = out_path.as_deref() {
        ensure_output_parent_dir(out)?;
        fs::write(out, rendered.as_bytes())?;
    }
    if json || out_path.is_none() {
        println!("{rendered}");
    } else {
        println!("OK: perf budget {}", out_path.unwrap_or_default());
    }
    if fail_on_regression
        && payload.get("status").and_then(serde_json::Value::as_str) == Some("regressed")
    {
        return Err(actionable_command_error(
            "perf",
            "perf budget failed: at least one benchmark row exceeded its budget",
        )
        .into());
    }
    Ok(())
}

pub(crate) fn run_perf_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| is_help_token(arg)) {
        println!("{}", perf_help_text());
        return Ok(());
    }

    let sub = args[0].as_str();
    if sub == "trend" {
        return run_perf_trend_command(&args[1..]);
    }
    if sub == "certify" {
        return run_perf_certify_command(&args[1..]);
    }
    if sub == "budget" {
        return run_perf_budget_command(&args[1..]);
    }
    if !matches!(sub, "native-fast" | "compare") {
        return Err(actionable_command_error(
            "perf",
            format!(
                "perf: unknown subcommand '{}'; expected native-fast, compare, trend, certify or budget",
                sub
            ),
        )
        .into());
    }

    let mut iterations = 3usize;
    let mut out_path: Option<String> = None;
    let mut dashboard_out: Option<String> = None;
    let mut cert_out: Option<String> = None;
    let mut corpus_mode = false;
    let mut include_c_matrix = true;
    let mut json = false;
    let mut idx = 1usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--corpus" => {
                corpus_mode = true;
                idx += 1;
            }
            "--c-matrix" => {
                include_c_matrix = true;
                idx += 1;
            }
            "--no-c-matrix" => {
                include_c_matrix = false;
                idx += 1;
            }
            "--iterations" => {
                let raw = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("perf", "perf syntax: --iterations requires a value")
                })?;
                iterations = raw.parse::<usize>().map_err(|_| {
                    actionable_command_error(
                        "perf",
                        format!("perf: invalid --iterations '{}'", raw),
                    )
                })?;
                if iterations == 0 || iterations > 1000 {
                    return Err(actionable_command_error(
                        "perf",
                        "perf: --iterations must be between 1 and 1000",
                    )
                    .into());
                }
                idx += 2;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("perf", "perf syntax: --out requires a file path")
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            "--dashboard-out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "perf",
                        "perf syntax: --dashboard-out requires a file path",
                    )
                })?;
                dashboard_out = Some(value.clone());
                idx += 2;
            }
            "--cert-out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("perf", "perf syntax: --cert-out requires a file path")
                })?;
                cert_out = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "perf",
                    format!("perf: unknown option '{}'", other),
                )
                .into());
            }
        }
    }

    run_perf_native_fast(
        iterations,
        out_path.as_deref(),
        dashboard_out.as_deref(),
        cert_out.as_deref(),
        corpus_mode,
        include_c_matrix,
        json,
    )
}
