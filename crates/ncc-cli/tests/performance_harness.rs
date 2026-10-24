use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn unique_out_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ncc-{name}-{}-{nanos}.json", std::process::id()))
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn command_available(tool: &str, args: &[&str]) -> bool {
    Command::new(tool)
        .args(args)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn c_compiler_can_compile(tool: &str) -> bool {
    let source = unique_out_path(&format!("{tool}-probe")).with_extension("c");
    let object = unique_out_path(&format!("{tool}-probe")).with_extension("o");
    if std::fs::write(&source, "int main(void){return 0;}\n").is_err() {
        return false;
    }
    let status = Command::new(tool)
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .env("NC_NO_COLOR", "1")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(source);
    let _ = std::fs::remove_file(object);
    status
}

fn host_x86_64_native_fast_profile_available() -> bool {
    matches!(
        (std::env::consts::OS, std::env::consts::ARCH),
        ("linux", "x86_64") | ("windows", "x86_64")
    )
}

fn host_c_toolchain_can_build_native_launcher() -> bool {
    if !host_x86_64_native_fast_profile_available() {
        return false;
    }
    ["cc", "gcc", "clang"]
        .iter()
        .any(|tool| c_compiler_can_compile(tool))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn perf_fixture(name: &str) -> PathBuf {
    let root = workspace_root();
    let current = root.join("examples/performance/native_fast").join(name);
    if current.is_file() {
        return current;
    }
    let archived = root
        .join("_archive/old_examples/performance/native_fast")
        .join(name);
    if archived.is_file() {
        return archived;
    }
    current
}

#[test]
fn ncc_perf_native_fast_json_reports_real_nc_case() {
    if !host_c_toolchain_can_build_native_launcher() {
        return;
    }

    let out = Command::new(ncc_bin())
        .args(["perf", "native-fast", "--iterations", "1", "--json"])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf must execute");
    assert!(
        out.status.success(),
        "perf must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("perf output must be JSON");
    assert_eq!(
        payload["schema"].as_str(),
        Some("ncc.performance.native_fast.compare.v1")
    );
    let cases = payload["cases"].as_array().expect("cases array");
    let nc = cases
        .iter()
        .find(|case| case["language"].as_str() == Some("nC"))
        .expect("nC case must exist");
    assert_eq!(nc["build"]["status"].as_str(), Some("ok"));
    assert_eq!(nc["run"]["status"].as_str(), Some("ok"));
    assert!(
        nc["build"]["binary_size_bytes"].as_u64().unwrap_or(0) > 0,
        "nC binary size must be measured"
    );
    assert_eq!(
        nc["build"]["optimization"]["schema"].as_str(),
        Some("ncc.performance.optimizer.v1"),
        "nC perf build must embed optimizer report"
    );
    assert_eq!(
        nc["build"]["optimization"]["elf"]["non_executable_stack_contract"].as_str(),
        Some("present"),
        "nC optimizer report must confirm non-executable stack contract"
    );
}

#[test]
fn ncc_perf_native_fast_reports_small_medium_large_pipeline_metrics() {
    if !host_c_toolchain_can_build_native_launcher() {
        return;
    }

    let out = Command::new(ncc_bin())
        .args(["perf", "native-fast", "--iterations", "1", "--json"])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf must execute");
    assert!(
        out.status.success(),
        "perf must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("perf output must be JSON");
    assert_eq!(payload["bench_harness"].as_str(), Some("ncc"));
    assert_eq!(
        payload["suite"].as_str(),
        Some("native_fast_small_medium_large")
    );
    let benchmarks = payload["benchmarks"].as_array().expect("benchmarks array");
    let sizes: Vec<_> = benchmarks
        .iter()
        .filter_map(|benchmark| benchmark["size"].as_str())
        .collect();
    assert_eq!(sizes, vec!["small", "medium", "large"]);
    assert_eq!(
        payload["benchmark_contract_catalog"]["status"].as_str(),
        Some("validated")
    );
    assert_eq!(
        payload["benchmark_contract_catalog"]["contract_count"].as_u64(),
        Some(3)
    );
    assert_eq!(
        payload["benchmark_contract_catalog"]["fixture_sources_verified"].as_bool(),
        Some(true)
    );
    assert_eq!(
        payload["benchmark_contract_catalog"]["catalog_sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    assert_eq!(
        payload["benchmark_contract_catalog"]["measurement_thresholds"]["checksum_match"].as_str(),
        Some("exact")
    );
    for benchmark in benchmarks {
        let cases = benchmark["cases"].as_array().expect("benchmark cases");
        let nc = cases
            .iter()
            .find(|case| case["language"].as_str() == Some("nC"))
            .expect("nC case must exist per benchmark");
        assert_eq!(nc["build"]["status"].as_str(), Some("ok"));
        assert_eq!(nc["run"]["status"].as_str(), Some("ok"));
        assert!(
            nc["run"]["warmup_iterations"].as_u64().unwrap_or(0) >= 1,
            "nC execution must be warmed before measurement"
        );
        let median_ns = nc["run"]["median_ns"].as_u64().unwrap_or(0);
        let p95_ns = nc["run"]["p95_ns"].as_u64().unwrap_or(0);
        assert!(median_ns > 0, "nC execution median must be measured");
        assert!(p95_ns >= median_ns, "p95 must not be below the median");
        let pipeline = &nc["build"]["pipeline"];
        assert_eq!(
            pipeline["schema"].as_str(),
            Some("ncc.performance.pipeline.v1")
        );
        for metric in [
            "lex_us",
            "parse_us",
            "sema_us",
            "ir_lower_us",
            "backend_emit_us",
            "object_emit_us",
            "link_us",
        ] {
            assert!(
                pipeline[metric].as_u64().is_some(),
                "{metric} must be reported for {}",
                benchmark["name"].as_str().unwrap_or("benchmark")
            );
        }
        assert!(
            pipeline["bytes_emitted"]["measured_elf_object_bytes"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "nC benchmark must report emitted object bytes"
        );
        assert!(
            pipeline["object_stats"]["text_bytes"].as_u64().unwrap_or(0) > 0,
            "nC benchmark must report native text bytes"
        );
        assert!(
            benchmark.get("overhead_vs_c").is_some(),
            "C overhead section must exist"
        );
        assert!(
            benchmark.get("overhead_vs_rust").is_some(),
            "Rust overhead section must exist"
        );
        assert_eq!(
            benchmark["semantic_equivalence"]["schema"].as_str(),
            Some("ncc.performance.semantic_equivalence.v1"),
            "every measured fixture must publish its semantic equivalence contract"
        );
        assert_eq!(
            benchmark["semantic_equivalence"]["expected"]["exit_code"].as_u64(),
            Some(0),
            "every measured fixture must require the exact successful exit code"
        );
        assert_eq!(
            benchmark["semantic_equivalence"]["thresholds"]["checksum_match"].as_str(),
            Some("exact"),
            "every measured fixture must require exact checksum matching"
        );
        assert_eq!(
            pipeline["linker"]["route"].as_str(),
            Some("machine_phase12_internal_os_executable"),
            "native-fast must measure the same internal OS linker route as normal build/run"
        );
    }
    let large = benchmarks
        .iter()
        .find(|benchmark| benchmark["size"].as_str() == Some("large"))
        .expect("large benchmark must exist");
    let medium = benchmarks
        .iter()
        .find(|benchmark| benchmark["size"].as_str() == Some("medium"))
        .expect("medium benchmark must exist");
    assert_eq!(
        medium["semantic_equivalence"]["workload"]["iterations"].as_u64(),
        Some(5_000_000)
    );
    assert_eq!(
        medium["semantic_equivalence"]["expected"]["checksums"]["final_seed_u32"].as_u64(),
        Some(1_035_787_873)
    );
    let medium_nc = medium["cases"]
        .as_array()
        .expect("medium cases")
        .iter()
        .find(|case| case["language"].as_str() == Some("nC"))
        .expect("medium nC case");
    assert!(
        medium_nc["build"]["optimization"]["internal_optimizer"]["totals"]["inline_expansions"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "the arithmetic benchmark must exercise real whole-program inlining"
    );
    assert_eq!(
        large["name"].as_str(),
        Some("volatile_loop_memory_pipeline")
    );
    assert_eq!(
        large["semantic_equivalence"]["workload"]["iterations"].as_u64(),
        Some(20_000_000)
    );
    assert_eq!(
        large["semantic_equivalence"]["expected"]["checksums"]["final_cell_u32"].as_u64(),
        Some(542_894_465)
    );
    assert_eq!(
        large["semantic_equivalence"]["expected"]["checksums"]["low_byte_accumulator_u32"].as_u64(),
        Some(2_550_000_000)
    );
    assert_eq!(
        large["semantic_equivalence"]["expected"]["checksums"]["final_index_u64"].as_u64(),
        Some(20_000_000)
    );
}

#[test]
fn ncc_perf_native_fast_reports_numeric_small_medium_c_rust_overhead_when_available() {
    if !host_c_toolchain_can_build_native_launcher() || !command_available("rustc", &["--version"])
    {
        return;
    }

    let out = Command::new(ncc_bin())
        .args(["perf", "native-fast", "--iterations", "1", "--json"])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf must execute");
    assert!(
        out.status.success(),
        "perf must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("perf output must be JSON");
    let benchmarks = payload["benchmarks"].as_array().expect("benchmarks array");
    for wanted_size in ["small", "medium"] {
        let benchmark = benchmarks
            .iter()
            .find(|benchmark| benchmark["size"].as_str() == Some(wanted_size))
            .unwrap_or_else(|| panic!("missing {wanted_size} benchmark"));
        for language in ["nC", "C", "Rust"] {
            let case = benchmark["cases"]
                .as_array()
                .expect("cases array")
                .iter()
                .find(|case| case["language"].as_str() == Some(language))
                .unwrap_or_else(|| panic!("missing {language} case for {wanted_size}"));
            assert_eq!(
                case["build"]["status"].as_str(),
                Some("ok"),
                "{language} build must pass for {wanted_size}: {case}"
            );
            assert_eq!(
                case["run"]["status"].as_str(),
                Some("ok"),
                "{language} run must pass for {wanted_size}: {case}"
            );
            assert!(
                case["run"]["avg_us"].as_u64().unwrap_or(0) > 0,
                "{language} run time must be measured for {wanted_size}: {case}"
            );
        }

        for section in ["overhead_vs_c", "overhead_vs_rust"] {
            for metric in [
                "build_time_ratio",
                "run_time_ratio",
                "run_time_median_ratio",
                "binary_size_ratio",
            ] {
                let value = benchmark[section][metric].as_f64().unwrap_or_else(|| {
                    panic!("{section}.{metric} must be numeric for {wanted_size}")
                });
                assert!(
                    value.is_finite() && value > 0.0,
                    "{section}.{metric} must be a finite positive ratio for {wanted_size}, got {value}"
                );
            }
        }
    }
}

#[test]
fn ncc_perf_native_fast_reports_c_style_clang_gcc_o2_o3_matrix() {
    let out = Command::new(ncc_bin())
        .args([
            "perf",
            "native-fast",
            "--iterations",
            "1",
            "--c-matrix",
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf c matrix must execute");
    assert!(
        out.status.success(),
        "perf c matrix must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&out.stdout).expect("perf c matrix output must be JSON");
    assert_eq!(
        payload["schema"].as_str(),
        Some("ncc.performance.native_fast.compare.v1")
    );
    assert_eq!(
        payload["c_style_corpus"]["schema"].as_str(),
        Some("ncc.performance.c_style_corpus.v1")
    );
    assert_eq!(
        payload["c_style_corpus"]["planned_rows"].as_u64(),
        Some(12),
        "small/medium/large must plan clang/gcc x O2/O3 rows"
    );
    assert_eq!(
        payload["c_style_corpus"]["retention"]["ci_artifact"].as_str(),
        Some("perf-native-fast.json")
    );
    for compiler in ["clang", "gcc"] {
        assert!(
            payload["c_style_corpus"]["compilers"]
                .as_array()
                .expect("compilers array")
                .iter()
                .any(|value| value.as_str() == Some(compiler)),
            "{compiler} must be part of the C-style compiler matrix"
        );
    }
    for opt_level in ["O2", "O3"] {
        assert!(
            payload["c_style_corpus"]["opt_levels"]
                .as_array()
                .expect("opt levels array")
                .iter()
                .any(|value| value.as_str() == Some(opt_level)),
            "{opt_level} must be part of the C-style optimization matrix"
        );
    }

    let benchmarks = payload["benchmarks"].as_array().expect("benchmarks array");
    assert_eq!(benchmarks.len(), 3);
    for benchmark in benchmarks {
        assert!(
            matches!(
                benchmark["fixture_class"].as_str(),
                Some("microbenchmark") | Some("program")
            ),
            "benchmark must classify C-style fixture kind: {benchmark}"
        );
        let rows = benchmark["c_matrix"].as_array().expect("c matrix rows");
        assert_eq!(
            rows.len(),
            4,
            "each fixed benchmark must include clang/gcc x O2/O3 rows"
        );
        assert_eq!(
            benchmark["c_matrix_summary"]["planned_rows"].as_u64(),
            Some(4)
        );
        assert_eq!(
            benchmark["overhead_vs_c_matrix"]
                .as_array()
                .expect("overhead matrix")
                .len(),
            4
        );
        for compiler in ["clang", "gcc"] {
            for opt_level in ["O2", "O3"] {
                let row = rows
                    .iter()
                    .find(|row| {
                        row["compiler"].as_str() == Some(compiler)
                            && row["opt_level"].as_str() == Some(opt_level)
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "missing C matrix row {compiler}-{opt_level} for {}",
                            benchmark["name"].as_str().unwrap_or("benchmark")
                        )
                    });
                assert_eq!(
                    row["schema"].as_str(),
                    Some("ncc.performance.c_style_matrix.case.v1")
                );
                assert_eq!(row["language"].as_str(), Some("C"));
                assert_eq!(
                    row["optimization_flag"].as_str().unwrap_or_default(),
                    format!("-{opt_level}")
                );
                assert!(
                    matches!(
                        row["build"]["status"].as_str(),
                        Some("ok") | Some("failed") | Some("missing_toolchain")
                    ),
                    "C matrix build status must be explicit: {row}"
                );
                assert!(
                    matches!(row["run"]["status"].as_str(), Some("ok") | Some("not_run")),
                    "C matrix run status must be explicit: {row}"
                );
                if row["run"]["status"].as_str() == Some("ok") {
                    assert!(
                        row["run"]["warmup_iterations"].as_u64().unwrap_or(0) >= 1,
                        "C matrix execution must be warmed before measurement: {row}"
                    );
                    let median_ns = row["run"]["median_ns"].as_u64().unwrap_or(0);
                    let p95_ns = row["run"]["p95_ns"].as_u64().unwrap_or(0);
                    assert!(median_ns > 0, "C matrix median must be measured: {row}");
                    assert!(
                        p95_ns >= median_ns,
                        "C matrix p95 must not be below median: {row}"
                    );
                }
            }
        }
    }
}

#[test]
fn ncc_perf_large_volatile_pointer_workload_is_native_linkable() {
    if !host_c_toolchain_can_build_native_launcher() {
        return;
    }

    let out = Command::new(ncc_bin())
        .args(["perf", "native-fast", "--iterations", "1", "--json"])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf must execute");
    assert!(
        out.status.success(),
        "perf must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("perf output must be JSON");
    let large = payload["benchmarks"]
        .as_array()
        .expect("benchmarks array")
        .iter()
        .find(|benchmark| benchmark["size"].as_str() == Some("large"))
        .expect("large benchmark must exist");
    let nc = large["cases"]
        .as_array()
        .expect("large cases")
        .iter()
        .find(|case| case["language"].as_str() == Some("nC"))
        .expect("large nC case must exist");

    assert_eq!(
        large["name"].as_str(),
        Some("volatile_loop_memory_pipeline")
    );
    assert!(nc["source"]
        .as_str()
        .unwrap_or_default()
        .ends_with("bench_workload.n"));
    assert_eq!(nc["build"]["status"].as_str(), Some("ok"));
    assert_eq!(nc["run"]["status"].as_str(), Some("ok"));
    assert!(
        nc["build"]["pipeline"]["object_stats"]["text_bytes"]
            .as_u64()
            .unwrap_or(0)
            >= 120,
        "large nC volatile workload must emit material native loop and pointer bytes"
    );
    assert!(
        nc["build"]["pipeline"]["object_stats"]["relocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "volatile helpers must preserve their material internal relocations"
    );
    assert_eq!(
        nc["build"]["pipeline"]["object_stats"]["unresolved_symbols"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "pointer intrinsics must not leave unresolved external symbols"
    );
    assert_eq!(
        nc["build"]["optimization"]["object_stats"]["unresolved_symbols"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "optimized pointer artifact must remain self-contained"
    );
}

#[test]
fn ncc_perf_native_fast_writes_out_report() {
    if !host_c_toolchain_can_build_native_launcher() {
        return;
    }

    let out_path = unique_out_path("perf-native-fast");
    let out = Command::new(ncc_bin())
        .args([
            "perf",
            "native-fast",
            "--iterations",
            "1",
            "--out",
            out_path.to_str().expect("utf8 temp path"),
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf --out must execute");
    assert!(
        out.status.success(),
        "perf --out must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    assert!(out_path.exists(), "perf report must be written");
    let raw = std::fs::read_to_string(&out_path).expect("perf report must read");
    let payload: Value = serde_json::from_str(&raw).expect("perf report must be JSON");
    assert_eq!(payload["status"].as_str(), Some("ok"));
    let _ = std::fs::remove_file(out_path);
}

#[test]
fn ncc_perf_native_fast_corpus_reports_long_running_scope() {
    let out = Command::new(ncc_bin())
        .args([
            "perf",
            "native-fast",
            "--iterations",
            "1",
            "--corpus",
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf corpus must execute");
    assert!(
        out.status.success(),
        "perf corpus must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&out.stdout).expect("perf corpus output must be JSON");
    assert_eq!(
        payload["schema"].as_str(),
        Some("ncc.performance.native_fast.compare.v1")
    );
    assert_eq!(payload["suite"].as_str(), Some("native_fast_corpus"));
    assert_eq!(payload["corpus_mode"].as_bool(), Some(true));
    assert!(
        payload["corpus"]["fixture_count"].as_u64().unwrap_or(0) >= 20,
        "corpus mode must enumerate the native_fast fixture corpus"
    );
    assert!(
        payload["corpus"]["without_c_baseline"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "corpus mode must mark missing C baselines instead of inventing them"
    );
    assert_eq!(
        payload["c_style_corpus"]["schema"].as_str(),
        Some("ncc.performance.c_style_corpus.v1")
    );
    assert_eq!(
        payload["optimizer_guidance"]["schema"].as_str(),
        Some("ncc.performance.optimizer_guidance.v1")
    );
    assert_eq!(
        payload["optimizer_guidance"]["status"].as_str(),
        Some("corpus_guided")
    );
    assert!(
        payload["optimizer_guidance"]["coverage"]["benchmarks_with_optimizer_reports"]
            .as_u64()
            .unwrap_or(0)
            >= 3,
        "corpus guidance must aggregate real optimizer reports"
    );
    assert!(
        payload["optimizer_guidance"]["totals"]["inline_expansions"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "corpus guidance must expose inline pressure"
    );
    assert!(
        payload["optimizer_guidance"]["totals"]["pgo_hot_call_sites"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "corpus guidance must expose PGO hot call pressure"
    );
    assert!(
        payload["optimizer_guidance"]["totals"]["lto_imported_call_edges"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "corpus guidance must expose LTO cross-unit import pressure"
    );
    assert_eq!(
        payload["c_style_corpus"]["corpus_mode"].as_bool(),
        Some(true)
    );
    assert_eq!(
        payload["c_style_corpus"]["planned_rows"].as_u64(),
        Some(payload["corpus"]["fixture_count"].as_u64().unwrap_or(0) * 4),
        "corpus mode must plan clang/gcc x O2/O3 rows for every fixture and mark missing baselines honestly"
    );
    assert!(
        payload["c_style_corpus"]["missing_fixture_rows"]
            .as_u64()
            .unwrap_or(0)
            >= payload["corpus"]["without_c_baseline"]
                .as_u64()
                .unwrap_or(0)
                * 4,
        "missing C fixtures must become explicit C matrix rows"
    );
    let benchmarks = payload["benchmarks"].as_array().expect("benchmarks array");
    assert_eq!(
        benchmarks.len() as u64,
        payload["corpus"]["fixture_count"].as_u64().unwrap_or(0)
    );
    for required in [
        "optimizer_inline_chain",
        "optimizer_pgo_hot_loop",
        "optimizer_lto_cross_container",
    ] {
        let benchmark = benchmarks
            .iter()
            .find(|benchmark| benchmark["name"].as_str() == Some(required))
            .unwrap_or_else(|| panic!("corpus must include optimizer fixture {required}"));
        assert_eq!(
            benchmark["baselines"]["has_c"].as_bool(),
            Some(true),
            "optimizer fixture {required} must keep a real C baseline"
        );
        assert_eq!(
            benchmark["cases"][0]["build"]["status"].as_str(),
            Some("ok"),
            "optimizer fixture {required} must build through nC"
        );
        assert_eq!(
            benchmark["cases"][0]["run"]["status"].as_str(),
            Some("ok"),
            "optimizer fixture {required} must run through native-fast"
        );
    }
    for (fixture, expected_iterations) in [
        ("bench_medium", 5_000_000_u64),
        ("bench_workload", 20_000_000_u64),
    ] {
        let benchmark = benchmarks
            .iter()
            .find(|benchmark| benchmark["name"].as_str() == Some(fixture))
            .unwrap_or_else(|| panic!("corpus must include exact fixture {fixture}"));
        assert_eq!(
            benchmark["semantic_equivalence"]["selected_as"].as_str(),
            Some(fixture)
        );
        assert_eq!(
            benchmark["semantic_equivalence"]["workload"]["iterations"].as_u64(),
            Some(expected_iterations)
        );
        assert_eq!(
            benchmark["semantic_equivalence"]["thresholds"]["checksum_match"].as_str(),
            Some("exact")
        );
    }
    for benchmark in benchmarks {
        assert_eq!(
            benchmark["c_matrix"]
                .as_array()
                .expect("corpus c matrix")
                .len(),
            4,
            "each corpus fixture must keep the full C-style matrix shape"
        );
    }
}

#[test]
fn ncc_perf_writes_dashboard_certificate_and_rehydrates_inputs() {
    let report_path = unique_out_path("perf-native-fast-report");
    let dashboard_path = unique_out_path("perf-native-fast-dashboard");
    let cert_path = unique_out_path("perf-native-fast-cert");
    let out = Command::new(ncc_bin())
        .args([
            "perf",
            "native-fast",
            "--iterations",
            "1",
            "--out",
            report_path.to_str().expect("utf8 report path"),
            "--dashboard-out",
            dashboard_path.to_str().expect("utf8 dashboard path"),
            "--cert-out",
            cert_path.to_str().expect("utf8 cert path"),
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf artifact command must execute");
    assert!(
        out.status.success(),
        "perf artifact command must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );

    let report_raw = std::fs::read_to_string(&report_path).expect("perf report must read");
    let report: Value = serde_json::from_str(&report_raw).expect("perf report must be JSON");
    assert_eq!(
        report["schema"].as_str(),
        Some("ncc.performance.native_fast.compare.v1")
    );
    assert!(
        report["host"]["label"].as_str().is_some(),
        "report must identify measured host"
    );

    let dashboard_raw = std::fs::read_to_string(&dashboard_path).expect("dashboard must read");
    let dashboard: Value = serde_json::from_str(&dashboard_raw).expect("dashboard must be JSON");
    assert_eq!(
        dashboard["schema"].as_str(),
        Some("ncc.performance.trend_dashboard.v1")
    );
    assert_eq!(dashboard["status"].as_str(), Some("ok"));

    let cert_raw = std::fs::read_to_string(&cert_path).expect("certificate must read");
    let cert: Value = serde_json::from_str(&cert_raw).expect("certificate must be JSON");
    assert_eq!(
        cert["schema"].as_str(),
        Some("ncc.performance.multi_host_certification.v1")
    );

    let trend = Command::new(ncc_bin())
        .args([
            "perf",
            "trend",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf trend must execute");
    assert!(
        trend.status.success(),
        "perf trend must pass\nstdout={}\nstderr={}",
        normalize(&trend.stdout),
        normalize(&trend.stderr)
    );
    let trend_payload: Value =
        serde_json::from_slice(&trend.stdout).expect("perf trend output must be JSON");
    assert_eq!(
        trend_payload["schema"].as_str(),
        Some("ncc.performance.trend_dashboard.v1")
    );
    assert!(
        trend_payload["rows"].as_array().map_or(0, Vec::len) >= 3,
        "trend dashboard must preserve benchmark rows"
    );
    assert_eq!(trend_payload["report_count"].as_u64(), Some(1));
    assert_eq!(
        trend_payload["c_style_corpus"]["schema"].as_str(),
        Some("ncc.performance.c_style_corpus.v1")
    );

    let trend_multi = Command::new(ncc_bin())
        .args([
            "perf",
            "trend",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf trend multi-input must execute");
    assert!(
        trend_multi.status.success(),
        "perf trend multi-input must pass\nstdout={}\nstderr={}",
        normalize(&trend_multi.stdout),
        normalize(&trend_multi.stderr)
    );
    let trend_multi_payload: Value =
        serde_json::from_slice(&trend_multi.stdout).expect("perf trend multi output must be JSON");
    assert_eq!(trend_multi_payload["report_count"].as_u64(), Some(2));
    assert!(
        trend_multi_payload["row_count"].as_u64().unwrap_or(0)
            >= trend_payload["row_count"].as_u64().unwrap_or(0) * 2,
        "multi-input trend must preserve rows from every input"
    );
    assert_eq!(
        trend_multi_payload["c_style_corpus"]["schema"].as_str(),
        Some("ncc.performance.c_style_corpus.aggregate.v1")
    );

    let certify = Command::new(ncc_bin())
        .args([
            "perf",
            "certify",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--host",
            "local-test",
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf certify must execute");
    assert!(
        certify.status.success(),
        "perf certify must pass\nstdout={}\nstderr={}",
        normalize(&certify.stdout),
        normalize(&certify.stderr)
    );
    let certify_payload: Value =
        serde_json::from_slice(&certify.stdout).expect("perf certify output must be JSON");
    assert_eq!(
        certify_payload["schema"].as_str(),
        Some("ncc.performance.multi_host_certification.v1")
    );
    assert_eq!(
        certify_payload["hosts"][0]["host"].as_str(),
        Some("local-test")
    );
    assert_eq!(certify_payload["report_count"].as_u64(), Some(1));

    let certify_multi = Command::new(ncc_bin())
        .args([
            "perf",
            "certify",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf certify multi-input must execute");
    assert!(
        certify_multi.status.success(),
        "perf certify multi-input must pass\nstdout={}\nstderr={}",
        normalize(&certify_multi.stdout),
        normalize(&certify_multi.stderr)
    );
    let certify_multi_payload: Value = serde_json::from_slice(&certify_multi.stdout)
        .expect("perf certify multi output must be JSON");
    assert_eq!(certify_multi_payload["report_count"].as_u64(), Some(2));
    assert_eq!(
        certify_multi_payload["c_style_corpus"]["schema"].as_str(),
        Some("ncc.performance.c_style_corpus.aggregate.v1")
    );

    let budget = Command::new(ncc_bin())
        .args([
            "perf",
            "budget",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf budget must execute");
    assert!(
        budget.status.success(),
        "perf budget must pass\nstdout={}\nstderr={}",
        normalize(&budget.stdout),
        normalize(&budget.stderr)
    );
    let budget_payload: Value =
        serde_json::from_slice(&budget.stdout).expect("perf budget output must be JSON");
    assert_eq!(
        budget_payload["schema"].as_str(),
        Some("ncc.performance.budget.v1")
    );
    assert!(
        matches!(
            budget_payload["status"].as_str(),
            Some("ok" | "warning" | "regressed")
        ),
        "budget status must be explicit"
    );
    assert!(
        budget_payload["summary"]["row_count"].as_u64().unwrap_or(0) >= 3,
        "budget report must preserve benchmark rows"
    );
    assert_eq!(
        budget_payload["policy"]["schema"].as_str(),
        Some("ncc.performance.budget.policy_set.v1")
    );

    let budget_path = unique_out_path("perf-budget-report");
    let budget_file = Command::new(ncc_bin())
        .args([
            "perf",
            "budget",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--baseline",
            report_path.to_str().expect("utf8 report path"),
            "--out",
            budget_path.to_str().expect("utf8 budget path"),
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf budget out must execute");
    assert!(
        budget_file.status.success(),
        "perf budget out must pass\nstdout={}\nstderr={}",
        normalize(&budget_file.stdout),
        normalize(&budget_file.stderr)
    );
    let budget_file_raw = std::fs::read_to_string(&budget_path).expect("budget file must read");
    let budget_file_payload: Value =
        serde_json::from_str(&budget_file_raw).expect("budget file must be JSON");
    assert_eq!(
        budget_file_payload["baseline_report_count"].as_u64(),
        Some(1)
    );
    assert!(
        budget_file_payload["summary"]["historical_comparisons"]
            .as_u64()
            .unwrap_or(0)
            >= 3,
        "budget baseline mode must compare historical rows"
    );
    assert_eq!(
        budget_file_payload["baseline_sources"][0]["accepted_as"].as_str(),
        Some("native_fast_report")
    );

    let budget_from_retained_budget = Command::new(ncc_bin())
        .args([
            "perf",
            "budget",
            "--input",
            report_path.to_str().expect("utf8 report path"),
            "--baseline",
            budget_path.to_str().expect("utf8 retained budget path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf budget retained baseline must execute");
    assert!(
        budget_from_retained_budget.status.success(),
        "perf budget retained baseline must pass\nstdout={}\nstderr={}",
        normalize(&budget_from_retained_budget.stdout),
        normalize(&budget_from_retained_budget.stderr)
    );
    let retained_baseline_payload: Value =
        serde_json::from_slice(&budget_from_retained_budget.stdout)
            .expect("retained budget baseline output must be JSON");
    assert_eq!(
        retained_baseline_payload["baseline_sources"][0]["accepted_as"].as_str(),
        Some("retained_budget_report")
    );
    assert!(
        retained_baseline_payload["summary"]["historical_comparisons"]
            .as_u64()
            .unwrap_or(0)
            >= 3,
        "retained budget report must be usable as the historical baseline"
    );

    let regressed_path = unique_out_path("perf-budget-regressed");
    let mut regressed_report = report.clone();
    let run_avg = regressed_report
        .pointer_mut("/benchmarks/0/cases/0/run/avg_us")
        .expect("first nC case must have run avg");
    *run_avg = Value::from(999_999_999u64);
    std::fs::write(
        &regressed_path,
        serde_json::to_string_pretty(&regressed_report).expect("regressed report serializes"),
    )
    .expect("regressed report writes");
    let regressed_budget = Command::new(ncc_bin())
        .args([
            "perf",
            "budget",
            "--input",
            regressed_path.to_str().expect("utf8 regressed path"),
            "--baseline",
            report_path.to_str().expect("utf8 report path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc perf budget regressed must execute");
    assert!(
        regressed_budget.status.success(),
        "perf budget regressed report must still emit JSON\nstdout={}\nstderr={}",
        normalize(&regressed_budget.stdout),
        normalize(&regressed_budget.stderr)
    );
    let regressed_payload: Value =
        serde_json::from_slice(&regressed_budget.stdout).expect("regressed budget must be JSON");
    assert_eq!(regressed_payload["status"].as_str(), Some("regressed"));
    assert!(
        regressed_payload["summary"]["regressed_rows"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "inflated runtime must be reported as a regression"
    );

    let _ = std::fs::remove_file(report_path);
    let _ = std::fs::remove_file(dashboard_path);
    let _ = std::fs::remove_file(cert_path);
    let _ = std::fs::remove_file(budget_path);
    let _ = std::fs::remove_file(regressed_path);
}

#[test]
fn ncc_optimize_json_reports_release_fast_pipeline() {
    let source = perf_fixture("minimal_return.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(
        payload["schema"].as_str(),
        Some("ncc.performance.optimizer.v1")
    );
    assert_eq!(payload["status"].as_str(), Some("closed"));
    assert_eq!(payload["profile"].as_str(), Some("release-fast"));
    assert_eq!(
        payload["release_fast_pipeline"]["schema"].as_str(),
        Some("ncc.release_fast.pipeline.v1")
    );
    assert_eq!(
        payload["release_fast_pipeline"]["status"].as_str(),
        Some("closed_measurable")
    );
    assert_eq!(
        payload["release_fast_pipeline"]["chain"][3].as_str(),
        Some("ssa_optimized_ir")
    );
    assert_eq!(
        payload["release_fast_pipeline"]["machine"]["entry_symbol"].as_str(),
        payload["entry_symbol"].as_str()
    );
    assert_eq!(
        payload["release_fast_pipeline"]["c_comparison"]["schema"].as_str(),
        Some("ncc.performance.native_fast.compare.v1")
    );
    assert_eq!(
        payload["release_fast_pipeline"]["whole_program"]["passes"][0].as_str(),
        Some("whole_program_symbol_index")
    );
    assert!(
        payload["release_fast_pipeline"]["whole_program"]["functions_scanned"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "release-fast pipeline must expose whole-program function index evidence"
    );
    assert_eq!(
        payload["release_fast_pipeline"]["lto"]["passes"][0].as_str(),
        Some("linked_ast_unit_merge")
    );
    assert!(
        payload["release_fast_pipeline"]["lto"]["linkage_units"].is_u64(),
        "release-fast pipeline must expose LTO linkage-unit evidence"
    );
    assert_eq!(
        payload["release_fast_pipeline"]["pgo"]["passes"][0].as_str(),
        Some("entry_and_callsite_profile_points")
    );
    assert!(
        payload["release_fast_pipeline"]["pgo"]["instrumented_sites"].is_u64(),
        "release-fast pipeline must expose PGO profile-site evidence"
    );
    assert_eq!(
        payload["release_fast_pipeline"]["loop_optimizer"]["passes"][0].as_str(),
        Some("dependency_analysis")
    );
    assert_eq!(
        payload["release_fast_pipeline"]["alias_analysis"]["passes"][0].as_str(),
        Some("memory_location_canonicalization")
    );
    assert!(
        payload["release_fast_pipeline"]["alias_analysis"]["transforms"].is_u64(),
        "release-fast pipeline must expose alias-analysis transform evidence"
    );
    assert_eq!(
        payload["release_fast_pipeline"]["register_allocator"]["passes"][0].as_str(),
        Some("cfg_liveness_fixed_point")
    );
    assert!(
        payload["release_fast_pipeline"]["register_allocator"]["live_intervals"].is_u64(),
        "release-fast pipeline must expose register allocator interval evidence"
    );
    assert_eq!(
        payload["release_fast_pipeline"]["instruction_selection"]["passes"][0].as_str(),
        Some("lea_selection")
    );
    assert!(
        payload["release_fast_pipeline"]["instruction_selection"]["patterns"].is_u64(),
        "release-fast pipeline must expose instruction-selection pattern evidence"
    );
    assert!(
        payload["release_fast_pipeline"]["instruction_selection"]["lea"].is_u64(),
        "release-fast pipeline must expose LEA selection evidence"
    );
    assert_eq!(
        payload["elf"]["non_executable_stack_contract"].as_str(),
        Some("present")
    );
    assert_eq!(
        payload["zero_cost_runtime"]["interpreter_runtime_used"].as_bool(),
        Some(false)
    );
    assert!(
        payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) > 0,
        "optimizer must report real text bytes"
    );
}

#[test]
fn ncc_optimize_json_reports_internal_optimizer_passes() {
    let source = perf_fixture("optimizer_fold_dead.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    let internal = &payload["internal_optimizer"];
    assert_eq!(
        internal["schema"].as_str(),
        Some("ncc.performance.internal_optimizer.v1")
    );
    assert_eq!(internal["status"].as_str(), Some("closed"));
    assert!(
        internal["totals"]["constant_folds"].as_u64().unwrap_or(0) > 0,
        "optimizer must fold constants for real nC source"
    );
    assert!(
        internal["totals"]["statements_removed"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "optimizer must prune unreachable statements for real nC source"
    );
    assert!(
        internal["ast"]["ssa"]["cfg_simplifications"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "optimizer must expose SSA/CFG simplification counters"
    );
    assert!(
        internal["totals"]["ssa_transforms"].is_u64(),
        "internal optimizer totals must expose SSA transform count"
    );
    assert!(
        internal["ast"]["whole_program"]["functions_scanned"].is_u64(),
        "optimizer must expose whole-program function index counters"
    );
    assert!(
        internal["ast"]["lto"]["linkage_units"].is_u64(),
        "optimizer must expose LTO linkage unit counters"
    );
    assert!(
        internal["ast"]["pgo"]["instrumented_sites"].is_u64(),
        "optimizer must expose PGO profile-site counters"
    );
    assert!(
        internal["totals"]["whole_program_functions_scanned"].is_u64(),
        "internal optimizer totals must expose whole-program function count"
    );
    assert!(
        internal["totals"]["lto_linkage_units"].is_u64(),
        "internal optimizer totals must expose LTO linkage unit count"
    );
    assert!(
        internal["totals"]["pgo_instrumented_sites"].is_u64(),
        "internal optimizer totals must expose PGO instrumentation count"
    );
    assert!(
        internal["ast"]["loop_optimizer"]["dependency_analyses"].is_u64(),
        "optimizer must expose loop dependency analysis counters"
    );
    assert!(
        internal["totals"]["loop_transforms"].is_u64(),
        "internal optimizer totals must expose loop transform count"
    );
    assert!(
        internal["backend"]["alias_analysis"]["queries"].is_u64(),
        "backend optimizer must expose alias-analysis query counters"
    );
    assert!(
        internal["totals"]["alias_transforms"].is_u64(),
        "internal optimizer totals must expose alias-analysis transform count"
    );
    assert!(
        internal["ast"]["before_ast_statements"]
            .as_u64()
            .unwrap_or(0)
            > internal["ast"]["after_ast_statements"]
                .as_u64()
                .unwrap_or(0),
        "AST optimizer must reduce statement count"
    );
    assert!(
        internal["backend"]["register_allocation"]["functions"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "backend optimizer must expose register allocation function evidence"
    );
    assert!(
        internal["backend"]["register_allocation"]["live_intervals"].is_u64(),
        "backend optimizer must expose register allocation live intervals"
    );
    assert!(
        internal["backend"]["register_allocation"]["interference_edges"].is_u64(),
        "backend optimizer must expose register allocation interference edges"
    );
    assert!(
        internal["backend"]["register_allocation"]["stack_traffic_bytes_avoided"].is_u64(),
        "backend optimizer must expose register allocation stack traffic accounting"
    );
    assert!(
        internal["backend"]["instruction_selection"]["lea"].is_u64(),
        "backend optimizer must expose instruction-selection LEA counters"
    );
    assert!(
        internal["backend"]["instruction_selection"]["macro_fusion_pairs"].is_u64(),
        "backend optimizer must expose macro-fusion pair counters"
    );
    assert!(
        internal["totals"]["instruction_selection_transforms"].is_u64(),
        "internal optimizer totals must expose instruction-selection transform count"
    );
    assert!(
        internal["backend"]["abi_call_lowering"]["call_sites"].is_u64(),
        "backend optimizer must expose ABI call-site evidence"
    );
    assert!(
        internal["backend"]["aggregate_layout"]["layouts"].is_u64(),
        "backend optimizer must expose aggregate layout evidence"
    );
    let pass_names = internal["backend"]["passes"]
        .as_array()
        .expect("backend optimizer passes")
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        pass_names.contains(&"backend_c_like_scalar_optimizer"),
        "backend optimizer pass list must include c-like scalar optimizer: {pass_names:?}"
    );
    assert!(
        pass_names.contains(&"backend_alias_analysis_optimizer"),
        "backend optimizer pass list must include alias analysis optimizer: {pass_names:?}"
    );
    assert!(
        pass_names.contains(&"backend_register_allocation_industrial"),
        "backend optimizer pass list must include industrial register allocator: {pass_names:?}"
    );
    assert!(
        pass_names.contains(&"backend_instruction_selection_industrial"),
        "backend optimizer pass list must include industrial instruction selection: {pass_names:?}"
    );
    let ast_pass_names = internal["ast"]["passes"]
        .as_array()
        .expect("ast optimizer passes")
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        ast_pass_names.contains(&"ast_whole_program_lto_inline_dce_pgo"),
        "AST optimizer pass list must include whole-program LTO/PGO: {ast_pass_names:?}"
    );
}

#[test]
fn ncc_optimize_json_reports_scalar_return_codegen_bytes() {
    let source = perf_fixture("return_7.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(
        payload["schema"].as_str(),
        Some("ncc.performance.optimizer.v1")
    );
    assert!(
        payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) >= 8,
        "scalar return codegen must emit mov return register plus ret"
    );
}

#[test]
fn ncc_optimize_json_reports_local_expression_return_codegen_bytes() {
    let source = perf_fixture("return_local_expr.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(
        payload["entry_symbol"].as_str(),
        Some("nativeLocalExpressionReturnPerf.boot")
    );
    assert_eq!(
        payload["object_stats"]["text_bytes"].as_u64(),
        Some(8),
        "local scalar expression return must emit only mov return register plus ret"
    );
}

#[test]
fn ncc_optimize_json_reports_register_stack_lowering_bytes() {
    if !host_x86_64_native_fast_profile_available() {
        return;
    }

    let source = perf_fixture("register_stack_sum.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(payload["status"].as_str(), Some("closed"));
    let inlined = payload["internal_optimizer"]["totals"]["inline_expansions"]
        .as_u64()
        .unwrap_or(0)
        > 0;
    if inlined {
        assert!(
            payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) >= 8,
            "inlined register/stack fixture must still emit real optimized return bytes"
        );
        assert!(
            payload["release_fast_pipeline"]["whole_program"]["inline_expansions"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "release-fast pipeline must report the whole-program inline"
        );
    } else {
        assert!(
            payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) > 16,
            "register/stack lowering must emit real call and arithmetic bytes"
        );
    }
}

#[test]
fn ncc_optimize_json_reports_native_call_abi_bytes() {
    if !host_x86_64_native_fast_profile_available() {
        return;
    }

    let source = perf_fixture("call_stack_args.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(payload["status"].as_str(), Some("closed"));
    let inlined = payload["internal_optimizer"]["totals"]["inline_expansions"]
        .as_u64()
        .unwrap_or(0)
        > 0;
    if inlined {
        assert!(
            payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) >= 8,
            "inlined native call fixture must still emit real optimized scalar bytes"
        );
    } else {
        assert!(
            payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) > 120,
            "native call ABI lowering must emit call, arg shuffle and stack-arg bytes"
        );
        assert!(
            payload["object_stats"]["relocations"].as_u64().unwrap_or(0) >= 1,
            "native call ABI lowering must retain real call relocations"
        );
    }
    assert_eq!(
        payload["object_stats"]["unresolved_symbols"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "native call ABI lowering must resolve local call targets"
    );
}

#[test]
fn ncc_optimize_json_reports_native_memory_ops_bytes() {
    if !host_x86_64_native_fast_profile_available() {
        return;
    }

    let source = perf_fixture("memory_ptr_offset_diff.n");
    let out = Command::new(ncc_bin())
        .args([
            "optimize",
            source.to_str().expect("utf8 source path"),
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc optimize must execute");
    assert!(
        out.status.success(),
        "optimize must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("optimize output must be JSON");
    assert_eq!(payload["status"].as_str(), Some("closed"));
    assert!(
        payload["object_stats"]["text_bytes"].as_u64().unwrap_or(0) > 80,
        "native memory lowering must emit lea/load/store/scale/diff bytes"
    );
    assert_eq!(
        payload["object_stats"]["unresolved_symbols"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "native memory ops must not require external relocations"
    );
}

#[test]
fn ncc_run_native_fast_show_return_reports_scalar_return() {
    if !host_c_toolchain_can_build_native_launcher() {
        return;
    }

    let source = perf_fixture("return_7.n");
    let out = Command::new(ncc_bin())
        .args([
            "run",
            source.to_str().expect("utf8 source path"),
            "--native-fast",
            "--show-return",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc run native-fast must execute");
    assert!(
        out.status.success(),
        "run native-fast must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    assert!(
        normalize(&out.stdout).contains("[return: 7 profile=release-fast route=native-fast-path]"),
        "native fast run must report the scalar return captured from native code\nstdout={}",
        normalize(&out.stdout)
    );
}
