#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn example_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    for base in ["examples", "_archive/old_examples"] {
        let candidate = root.join(base).join(relative);
        if candidate.exists() {
            return candidate;
        }
    }
    root.join("examples").join(relative)
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let bin_dir = ncc.parent().expect("ncc binary must have parent dir");
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ncc-{label}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir must create");
    dir
}

fn assert_no_materialized_c_sources(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("artifact directory") {
            let path = entry.expect("artifact entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                assert!(
                    !path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("c")),
                    "compiler-owned C source leaked into .ncc artifacts: {}",
                    path.display()
                );
            }
        }
    }
}

fn run_ncc(args: &[String], cwd: &Path) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(cwd)
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn combined(out: &Output) -> String {
    format!("{}{}", normalize(&out.stdout), normalize(&out.stderr))
}

fn fake_linker(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("fake-linker.sh");
    std::fs::write(
        &path,
        r#"#!/usr/bin/env bash
set -eu
out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
    break
  fi
  shift
done
if [ -z "$out" ]; then
  exit 64
fi
cat > "$out" <<'EOS'
#!/usr/bin/env bash
exit 0
EOS
chmod +x "$out"
"#,
    )
    .expect("fake linker must write");
    let mut perms = std::fs::metadata(&path)
        .expect("fake linker metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("fake linker must be executable");
    path
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn hosted_linux_x86_64_native_build_available() -> bool {
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return false;
    }
    if let Ok(matrix_arch) = std::env::var("NCC_MATRIX_ARCH") {
        if matrix_arch != std::env::consts::ARCH {
            return false;
        }
    }
    tool_available("clang")
}

#[test]
fn compiler_build_native_emits_obj_exe_nm_and_runs() {
    if !hosted_linux_x86_64_native_build_available() {
        return;
    }

    let cwd = unique_temp_dir("compiler-native-build-cwd");
    let out_dir = unique_temp_dir("compiler-native-build-out");
    let source = workspace_root().join("examples/10_mct/print_bytes.n");
    let out = run_ncc(
        &[
            "compiler".into(),
            "build-native".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--cc".into(),
            "nC.sysv64".into(),
            "-O3".into(),
            "--run".into(),
            "--json".into(),
            "--strict".into(),
        ],
        &cwd,
    );
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("build-native json");

    assert_eq!(json["schema"], "nc.compiler.native_build.v1");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(json["optimizer"]["level"], "O3", "{json:#?}");
    assert_eq!(json["speed_contract"]["interpreter_fallback"], false);

    let obj = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let exe = PathBuf::from(json["paths"]["executable"].as_str().expect("exe path"));
    let nm = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    let manifest = PathBuf::from(json["paths"]["manifest"].as_str().expect("manifest path"));
    for path in [&obj, &exe, &nm, &manifest] {
        assert!(
            path.is_file(),
            "missing artifact: {}\n{json:#?}",
            path.display()
        );
        assert!(
            std::fs::metadata(path).expect("metadata").len() > 0,
            "empty artifact: {}",
            path.display()
        );
    }
    assert_eq!(
        obj.extension().and_then(|value| value.to_str()),
        Some("obj")
    );
    assert_eq!(
        exe.extension().and_then(|value| value.to_str()),
        Some("exe")
    );
    assert_eq!(nm.extension().and_then(|value| value.to_str()), Some("nm"));
    assert!(
        json["run"]["stdout"]
            .as_str()
            .is_some_and(|stdout| stdout.contains("MCT bytes surface")),
        "{json:#?}"
    );

    let nm_source = std::fs::read_to_string(&nm).expect("nm source");
    assert!(nm_source.contains("[@machine]"), "{nm_source}");
    assert!(nm_source.contains("extern:"), "{nm_source}");
    assert!(
        nm_source.contains("call main;") || nm_source.contains("call extern_main;"),
        "{nm_source}"
    );

    let check = run_ncc(
        &[
            "machine".into(),
            "check".into(),
            nm.to_string_lossy().to_string(),
        ],
        &cwd,
    );
    assert!(check.status.success(), "{}", combined(&check));

    let _ = std::fs::remove_dir_all(cwd);
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn compiler_release_fast_trains_and_reuses_real_thin_lto_pgo_profile() {
    if !hosted_linux_x86_64_native_build_available() || !tool_available("llvm-profdata") {
        return;
    }

    let cwd = unique_temp_dir("compiler-native-pgo-cwd");
    let out_dir = unique_temp_dir("compiler-native-pgo-out");
    let source = example_path("performance/native_fast/minimal_return.n");
    let train = run_ncc(
        &[
            "compiler".into(),
            "build-native".into(),
            source.to_string_lossy().to_string(),
            "--profile".into(),
            "release-fast".into(),
            "--lto".into(),
            "thin".into(),
            "--pgo".into(),
            "train".into(),
            "--run".into(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(train.status.success(), "{}", combined(&train));
    let trained: serde_json::Value =
        serde_json::from_str(&normalize(&train.stdout)).expect("trained native build JSON");
    assert_eq!(trained["status"], "native_build_passed", "{trained:#?}");
    assert_eq!(trained["optimizer"]["lto"]["applied"], "thin");
    assert_eq!(trained["optimizer"]["pgo"]["requested"], "train");
    assert_eq!(trained["optimizer"]["pgo"]["profile_used"], true);
    let stages = trained["stages"]
        .as_array()
        .expect("training stages")
        .iter()
        .filter_map(|stage| stage["stage"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "clang-lto-link",
        "pgo-training-run",
        "llvm-profdata-merge",
        "clang-pgo-use-object",
        "clang-pgo-use-link",
    ] {
        assert!(
            stages.contains(&expected),
            "missing {expected}: {trained:#?}"
        );
    }
    let profile = PathBuf::from(
        trained["optimizer"]["pgo"]["profile_path"]
            .as_str()
            .expect("PGO profile path"),
    );
    assert!(profile.is_file());

    let reuse = run_ncc(
        &[
            "compiler".into(),
            "build-native".into(),
            source.to_string_lossy().to_string(),
            "--profile".into(),
            "release-fast".into(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--no-run".into(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(reuse.status.success(), "{}", combined(&reuse));
    let reused: serde_json::Value =
        serde_json::from_str(&normalize(&reuse.stdout)).expect("reused native build JSON");
    assert_eq!(reused["optimizer"]["pgo"]["requested"], "auto");
    assert_eq!(reused["optimizer"]["pgo"]["profile_used"], true);
    assert_eq!(reused["truth_boundary"]["pgo_profile_applied"], true);
    let object = PathBuf::from(reused["paths"]["object"].as_str().expect("object path"));
    assert_eq!(
        std::fs::read(object).expect("native object").get(0..4),
        Some(b"\x7fELF".as_slice()),
        "the retained .obj must remain a material target object, not LLVM bitcode"
    );

    let _ = std::fs::remove_dir_all(cwd);
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn compiler_build_native_win64_links_llvm_coff_with_internal_pe_authority() {
    if !tool_available("clang") {
        return;
    }

    let cwd = unique_temp_dir("compiler-native-win64-internal-pe-cwd");
    let out_dir = unique_temp_dir("compiler-native-win64-internal-pe-out");
    let source = workspace_root().join("examples/12_apps/win32_window_canvas_smoke.n");
    let out = run_ncc(
        &[
            "compiler".into(),
            "build-native".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--cc".into(),
            "nC.win64".into(),
            "-O3".into(),
            "--json".into(),
            "--strict".into(),
        ],
        &cwd,
    );
    assert!(
        out.status.success(),
        "Win64 LLVM/COFF build must use the internal PE linker\n{}",
        combined(&out)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("Win64 LLVM build JSON");

    assert_eq!(json["schema"], "nc.compiler.native_build.v1");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(
        json["link"]["route"], "mct-llvm-coff-phase12-pe64",
        "{json:#?}"
    );
    assert_eq!(json["link"]["internal_os_linker"], true, "{json:#?}");
    assert_eq!(
        json["truth_boundary"]["object_is_real_clang_target_output"], true,
        "{json:#?}"
    );
    assert_eq!(
        json["truth_boundary"]["object_and_executable_are_real_host_toolchain_outputs"], false,
        "{json:#?}"
    );
    assert_eq!(
        json["truth_boundary"]["executable_is_native_pe_direct"], true,
        "{json:#?}"
    );
    assert!(
        json["pipeline"]
            .as_array()
            .is_some_and(|pipeline| pipeline.iter().any(|stage| {
                stage
                    .as_str()
                    .is_some_and(|stage| stage.contains("Machine Phase 12"))
            })),
        "{json:#?}"
    );

    let stage_names = json["stages"]
        .as_array()
        .expect("stage array")
        .iter()
        .filter_map(|stage| stage["stage"].as_str())
        .collect::<Vec<_>>();
    assert!(stage_names.contains(&"clang-object"), "{json:#?}");
    assert!(
        stage_names.contains(&"ncc-machine-entry-object"),
        "{json:#?}"
    );
    assert!(stage_names.contains(&"ncc-internal-pe-link"), "{json:#?}");
    assert!(!stage_names.contains(&"clang-link"), "{json:#?}");

    let object = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let entry_object = PathBuf::from(
        json["paths"]["machine_entry_object"]
            .as_str()
            .expect("entry object path"),
    );
    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let link_map = PathBuf::from(json["paths"]["link_map"].as_str().expect("link map path"));
    let link_report = PathBuf::from(
        json["paths"]["link_report"]
            .as_str()
            .expect("link report path"),
    );
    for artifact in [&object, &entry_object, &executable, &link_map, &link_report] {
        assert!(
            artifact.is_file()
                && std::fs::metadata(artifact)
                    .expect("artifact metadata")
                    .len()
                    > 0,
            "missing internal PE artifact: {}\n{json:#?}",
            artifact.display()
        );
    }

    let bytes = std::fs::read(&executable).expect("PE bytes");
    assert_eq!(&bytes[..2], b"MZ");
    let pe_offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().expect("e_lfanew"));
    let pe_offset = usize::try_from(pe_offset).expect("PE offset");
    assert_eq!(&bytes[pe_offset..pe_offset + 4], b"PE\0\0");
    assert_eq!(
        json["link"]["phase12_report"]["artifact_magic"],
        "PE32_PLUS_MZ_PE"
    );
    assert_eq!(
        json["link"]["phase12_report"]["pe64_evidence"]["subsystem"],
        "windows_gui"
    );

    let requested_imports = json["link"]["phase12_report"]["executable_evidence"]
        ["object_requested_dynamic_imports"]
        .as_array()
        .expect("object requested imports")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    assert!(requested_imports.contains(&"windowOpen"), "{json:#?}");
    assert!(requested_imports.contains(&"canvasText"), "{json:#?}");

    let loader_imports = json["link"]["phase12_report"]["dynamic_imports"]["imports"]
        .as_array()
        .expect("PE loader imports")
        .iter()
        .filter_map(|entry| entry["symbol"].as_str())
        .collect::<Vec<_>>();
    for symbol in ["CreateWindowExW", "TextOutW", "ExitProcess"] {
        assert!(
            loader_imports.contains(&symbol),
            "missing {symbol}: {json:#?}"
        );
    }
    for abstract_symbol in ["windowOpen", "canvasText", "canvasPresent"] {
        assert!(
            !loader_imports.contains(&abstract_symbol),
            "abstract runtime import leaked into PE: {abstract_symbol}\n{json:#?}"
        );
    }

    let machine = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("Machine bridge path"),
    );
    let machine_source = std::fs::read_to_string(machine).expect("Machine bridge source");
    assert!(machine_source.contains("call main;"), "{machine_source}");
    assert!(
        machine_source.contains("call ExitProcess;"),
        "{machine_source}"
    );

    let _ = std::fs::remove_dir_all(cwd);
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn compiler_cross_build_arm64_darwin_links_runtime_macho_internally() {
    if !hosted_linux_x86_64_native_build_available() {
        return;
    }

    let cwd = unique_temp_dir("compiler-native-arm64-darwin-cwd");
    let out_dir = unique_temp_dir("compiler-native-arm64-darwin-out");
    let source = workspace_root().join("examples/07_runtime/backend_mct_real_bytes.n");
    let out = run_ncc(
        &[
            "compiler".into(),
            "build-native".into(),
            source.to_string_lossy().to_string(),
            "--target".into(),
            "aarch64-apple-darwin".into(),
            "--profile".into(),
            "release-fast".into(),
            "--lto".into(),
            "off".into(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--no-run".into(),
            "--json".into(),
            "--strict".into(),
        ],
        &cwd,
    );
    assert!(out.status.success(), "{}", combined(&out));
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("Darwin build JSON");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(json["link"]["route"], "mct-llvm-macho-phase12");
    assert_eq!(json["link"]["internal_os_link_format"], "macho");
    assert_eq!(
        json["truth_boundary"]["executable_is_native_macho_direct"],
        true
    );
    assert_eq!(
        json["link"]["phase12_report"]["artifact_magic"],
        "MACHO64_MH_EXECUTE"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["import_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "{json:#?}"
    );
    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("Mach-O executable path"),
    );
    let bytes = std::fs::read(executable).expect("Mach-O executable bytes");
    assert_eq!(&bytes[0..4], &0xfeed_facf_u32.to_le_bytes());
    assert_eq!(&bytes[4..8], &0x0100_000c_u32.to_le_bytes());

    let stages = json["stages"]
        .as_array()
        .expect("Darwin stages")
        .iter()
        .filter_map(|stage| stage["stage"].as_str())
        .collect::<Vec<_>>();
    assert!(
        !stages.contains(&"hosted-runtime-object"),
        "the nC-backed hosted surface must not materialize the legacy provider: {json:#?}"
    );
    assert!(stages.contains(&"ncc-internal-macho-link"), "{json:#?}");
    assert!(!stages.contains(&"clang-link"), "{json:#?}");
    assert!(json["runtime_hosted"]["source"].is_null());
    assert_eq!(
        json["runtime_hosted"]["source_materialization"],
        "compiler-embedded-stdin"
    );
    assert_no_materialized_c_sources(&out_dir);

    let _ = std::fs::remove_dir_all(cwd);
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn ncc_build_standard_emits_nm_obj_exe_and_manifest() {
    let cwd = unique_temp_dir("standard-native-build-cwd");
    let out_dir = cwd.join(".ncc");
    let linker = fake_linker(&cwd);
    let source = example_path("01_basics/root_lineage.n");
    let out = run_ncc(
        &[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(
        out.status.success(),
        "ncc build must succeed\n{}",
        combined(&out)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");

    assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(
        json["speed_contract"]["c_like_artifact_set"],
        serde_json::json!([".nb", ".nbb", ".init", ".nm", ".obj", ".exe"])
    );
    assert_eq!(
        json["speed_contract"]["complete_artifact_set"],
        serde_json::json!([
            ".nb",
            ".nbb",
            ".optimizer.json",
            ".backend-object.nbo",
            ".standard.mct.ll",
            ".init",
            ".nm",
            ".obj",
            ".entry.obj",
            ".exe"
        ])
    );
    assert_eq!(json["speed_contract"]["interpreter_fallback"], false);
    assert_eq!(json["speed_contract"]["release_fast_automatic"], true);
    assert_eq!(
        json["speed_contract"]["special_optimize_command_required"],
        false
    );
    assert_eq!(json["speed_contract"]["measurable_release_fast"], true);
    assert_eq!(json["optimization"]["requested"], "auto");
    assert_eq!(json["optimization"]["effective"], "O3");
    assert_eq!(
        json["release_fast_auto"]["schema"], "nc.compiler.release_fast_auto.v1",
        "{json:#?}"
    );
    assert_eq!(
        json["release_fast_auto"]["status"], "automatic_release_fast_applied",
        "{json:#?}"
    );
    assert_eq!(
        json["release_fast_auto"]["special_command_required"], false,
        "{json:#?}"
    );
    assert_eq!(json["release_fast_auto"]["measurable"], true, "{json:#?}");
    assert_eq!(
        json["release_fast_auto"]["optimization"]["effective"], "O3",
        "{json:#?}"
    );
    assert!(
        json["release_fast_auto"]["timings_ms"]
            .as_object()
            .is_some_and(|timings| !timings.is_empty()),
        "{json:#?}"
    );
    assert!(
        json["release_fast_auto"]["artifact_metrics"]["executable_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "{json:#?}"
    );
    assert!(
        json["release_fast_auto"]["optimizer_report"]["instruction_selection_transforms"]
            .is_number()
            || json["release_fast_auto"]["optimizer_report"]["release_fast_pipeline_status"]
                .as_str()
                .is_some(),
        "{json:#?}"
    );
    assert_eq!(
        json["truth_boundary"]["object_is_optimized_backend_output"],
        true
    );
    assert_eq!(json["truth_boundary"]["optimized_ir_report_retained"], true);
    assert_eq!(
        json["truth_boundary"]["machine_backend_object_ir_retained"],
        true
    );
    let native_call_admission =
        &json["machine_backend"]["runtime_import_materialization"]["native_call_admission"];
    assert_eq!(
        native_call_admission["schema"], "nc.compiler.runtime_native_call_admission.v1",
        "{json:#?}"
    );
    assert_eq!(
        native_call_admission["status"], "closed_catalog_derived",
        "{json:#?}"
    );
    assert_eq!(
        native_call_admission["manual_runtime_allowlist"], false,
        "{json:#?}"
    );
    assert_eq!(
        native_call_admission["catalog_only_activation_symbols_excluded"], true,
        "{json:#?}"
    );
    assert!(
        native_call_admission["official_runtime_function_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "{json:#?}"
    );

    let obj = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let basal_root = PathBuf::from(
        json["paths"]["basal_root_source"]
            .as_str()
            .expect("Basal root source path"),
    );
    let basal_binary = PathBuf::from(
        json["paths"]["basal_root_binary"]
            .as_str()
            .expect("Basal root binary path"),
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let optimizer = PathBuf::from(
        json["paths"]["optimized_ir_report"]
            .as_str()
            .expect("optimized IR report path"),
    );
    let backend_object_ir = PathBuf::from(
        json["paths"]["machine_backend_object_ir"]
            .as_str()
            .expect("machine backend object IR path"),
    );
    let nm = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    let target_init = PathBuf::from(
        json["paths"]["target_init"]
            .as_str()
            .expect("target init path"),
    );
    let manifest = PathBuf::from(json["paths"]["manifest"].as_str().expect("manifest path"));
    for path in [
        &basal_root,
        &basal_binary,
        &optimizer,
        &backend_object_ir,
        &target_init,
        &obj,
        &exe,
        &nm,
        &manifest,
    ] {
        assert!(
            path.is_file(),
            "missing standard build artifact: {}\n{json:#?}",
            path.display()
        );
        assert!(
            std::fs::metadata(path).expect("metadata").len() > 0,
            "empty standard build artifact: {}",
            path.display()
        );
    }
    assert_eq!(
        obj.extension().and_then(|value| value.to_str()),
        Some("obj")
    );
    assert_eq!(
        exe.extension().and_then(|value| value.to_str()),
        Some("exe")
    );
    assert_eq!(nm.extension().and_then(|value| value.to_str()), Some("nm"));
    assert_eq!(
        target_init.extension().and_then(|value| value.to_str()),
        Some("init")
    );
    assert_eq!(
        basal_root.extension().and_then(|value| value.to_str()),
        Some("nb")
    );
    assert_eq!(
        basal_binary.extension().and_then(|value| value.to_str()),
        Some("nbb")
    );
    assert_eq!(
        basal_root.file_stem().and_then(|value| value.to_str()),
        nm.file_stem().and_then(|value| value.to_str())
    );
    assert!(std::fs::read(&basal_binary)
        .expect("Basal binary")
        .starts_with(b"NBB0"));
    let nbo_text = std::fs::read_to_string(&backend_object_ir).expect("NBO text");
    assert!(nbo_text.contains("; nc.lowering.source-root-machine-nbo.v2"));
    assert!(nbo_text.contains("; basal-source-sha256="));
    assert!(nbo_text.contains("; basal-binary-sha256="));
    assert_eq!(
        json["lowering_lineage"]["order"],
        serde_json::json!([
            "source.n",
            "root.nb",
            "root.nbb",
            "target.init",
            "machine.nm",
            "backend-object.nbo",
            "native-object",
            "executable"
        ])
    );
    let init_bytes = std::fs::read(&target_init).expect("target init bytes");
    assert!(init_bytes.starts_with(b"NCI1\n"));
    let init_header_end = init_bytes
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|offset| offset + 2)
        .expect("target init header terminator");
    let init_header =
        std::str::from_utf8(&init_bytes[..init_header_end]).expect("target init UTF-8 header");
    assert!(init_header.contains("schema=nc.compiler.target-init.v1\n"));
    assert!(init_header.contains("arch=x86_64\n"));
    assert!(init_header.contains("payload-kind=machine-entry-object\n"));
    assert!(init_header.contains("machine-sha256="));
    assert!(init_header.contains("payload-sha256="));
    assert_eq!(
        json["target_init"]["container_bytes"].as_u64(),
        Some(init_bytes.len() as u64)
    );
    assert_eq!(
        json["target_init"]["payload_bytes"].as_u64(),
        Some((init_bytes.len() - init_header_end) as u64)
    );
    assert_eq!(
        optimizer.extension().and_then(|value| value.to_str()),
        Some("json")
    );
    assert_eq!(
        backend_object_ir
            .extension()
            .and_then(|value| value.to_str()),
        Some("nbo")
    );

    let nm_source = std::fs::read_to_string(&nm).expect("standard build nm source");
    assert!(nm_source.contains(":: machine"), "{nm_source}");
    assert!(
        nm_source.contains(
            "producer: \"ncc hosted .n -> root -> .nm -> .nbo -> .obj -> .exe entry bridge\""
        ),
        "{nm_source}"
    );

    let check = run_ncc(
        &[
            "machine".into(),
            "check".into(),
            nm.to_string_lossy().to_string(),
        ],
        &cwd,
    );
    assert!(check.status.success(), "{}", combined(&check));

    let _ = std::fs::remove_dir_all(cwd);
}

#[test]
fn ncc_build_win64_without_linker_emits_native_pe64() {
    let cwd = unique_temp_dir("standard-win64-pe-build-cwd");
    let out_dir = cwd.join(".ncc");
    let source = example_path("root_at/root_at_minimal_ok.n");
    let out = run_ncc(
        &[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--cc".into(),
            "nC.win64".into(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(
        out.status.success(),
        "ncc build win64 must succeed\n{}",
        combined(&out)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard win64 build json");

    assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(json["link"]["route"], "native-pe64-internal", "{json:#?}");
    assert_eq!(json["link"]["native_pe64_internal"], true, "{json:#?}");
    assert_eq!(
        json["truth_boundary"]["executable_is_native_pe_direct"], true,
        "{json:#?}"
    );
    assert_eq!(
        json["truth_boundary"]["executable_is_configured_linker_output"], false,
        "{json:#?}"
    );

    let obj = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let bridge_obj = PathBuf::from(
        json["paths"]["machine_entry_object"]
            .as_str()
            .expect("machine entry object path"),
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let nm = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    for path in [&obj, &bridge_obj, &exe, &nm] {
        assert!(
            path.is_file(),
            "missing win64 build artifact: {}\n{json:#?}",
            path.display()
        );
        assert!(
            std::fs::metadata(path).expect("metadata").len() > 0,
            "empty win64 build artifact: {}",
            path.display()
        );
    }

    let bytes = std::fs::read(&exe).expect("win64 exe bytes");
    assert!(bytes.starts_with(b"MZ"), "PE must start with MZ");
    assert!(
        bytes
            .windows(b"ExitProcess".len())
            .any(|window| window == b"ExitProcess"),
        "PE import table must retain ExitProcess"
    );
    let pe_offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&bytes[pe_offset..pe_offset + 4], b"PE\0\0");
    assert_eq!(
        u16::from_le_bytes(bytes[pe_offset + 4..pe_offset + 6].try_into().unwrap()),
        0x8664
    );
    assert_eq!(
        u16::from_le_bytes(bytes[pe_offset + 24..pe_offset + 26].try_into().unwrap()),
        0x020b
    );

    let nm_source = std::fs::read_to_string(&nm).expect("win64 nm source");
    assert!(nm_source.contains("ExitProcess"), "{nm_source}");
    assert!(nm_source.contains("mov rbx, rax;"), "{nm_source}");
    assert!(nm_source.contains("mov rcx, rbx;"), "{nm_source}");

    let verify = run_ncc(
        &[
            "machine".into(),
            "verify-link".into(),
            exe.to_string_lossy().to_string(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(verify.status.success(), "{}", combined(&verify));
    assert!(
        normalize(&verify.stdout).contains("\"artifact_type\": \"pe64_os_executable\""),
        "{}",
        combined(&verify)
    );

    let _ = std::fs::remove_dir_all(cwd);
}

#[test]
fn ncc_build_machine_source_reuses_nm_without_duplicate_and_emits_obj_exe_manifest() {
    let cwd = unique_temp_dir("standard-machine-build-cwd");
    let out_dir = cwd.join(".ncc");
    let source = workspace_root().join("examples/machine/native/link_final_windows_exit42.nm");
    let out = run_ncc(
        &[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--json".into(),
        ],
        &cwd,
    );
    assert!(
        out.status.success(),
        "ncc build .nm must succeed\n{}",
        combined(&out)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard machine build json");

    assert_eq!(json["schema"], "nc.compiler.standard_machine_build.v1");
    assert_eq!(json["status"], "native_build_passed", "{json:#?}");
    assert_eq!(
        json["machine_source_materialization"],
        "original_source_reused_without_copy"
    );
    assert_eq!(
        json["truth_boundary"]["machine_source_is_original_input"],
        true
    );
    assert_eq!(json["truth_boundary"]["machine_source_copy_emitted"], false);
    assert_eq!(
        json["link"]["route"], "machine-phase12-internal",
        "{json:#?}"
    );
    assert_eq!(
        json["speed_contract"]["c_like_artifact_set"],
        serde_json::json!([".nm", ".obj", ".exe"])
    );

    let nm = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    assert_eq!(
        std::fs::canonicalize(&nm).expect("canonical reported source"),
        std::fs::canonicalize(&source).expect("canonical input source")
    );
    assert!(
        !out_dir
            .join("code")
            .join("link_final_windows_exit42.nm")
            .exists(),
        "handwritten .nm must not be copied into .ncc/code"
    );
    assert!(
        !out_dir
            .join("lowering")
            .join("link_final_windows_exit42.nm")
            .exists(),
        "handwritten .nm must not be duplicated in .ncc/lowering"
    );
    let obj = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let manifest = PathBuf::from(json["paths"]["manifest"].as_str().expect("manifest path"));
    for path in [&nm, &obj, &exe, &manifest] {
        assert!(
            path.is_file(),
            "missing machine build artifact: {}\n{json:#?}",
            path.display()
        );
        assert!(
            std::fs::metadata(path).expect("metadata").len() > 0,
            "empty machine build artifact: {}",
            path.display()
        );
    }
    let bytes = std::fs::read(&exe).expect("machine build exe bytes");
    assert!(bytes.starts_with(b"MZ"), "machine build must emit PE bytes");
    assert!(bytes.windows(b".reloc".len()).any(|w| w == b".reloc"));

    let _ = std::fs::remove_dir_all(cwd);
}
