use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn example(rel: &str) -> String {
    let root = workspace_root();
    let direct = root.join(rel);
    if direct.exists() {
        return direct.to_string_lossy().to_string();
    }
    let archived = root.join("_archive").join("old_examples").join(
        rel.strip_prefix("examples/")
            .or_else(|| rel.strip_prefix("examples\\"))
            .unwrap_or(rel),
    );
    if archived.exists() {
        return archived.to_string_lossy().to_string();
    }
    direct.to_string_lossy().to_string()
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
}

fn assert_failure_contains(out: Output, label: &str, needle: &str) {
    let rendered = combined(&out);
    assert!(
        !out.status.success(),
        "{label} should fail but passed\n{rendered}"
    );
    assert!(
        rendered.contains(needle),
        "{label} must contain {needle:?}\n{rendered}"
    );
}

fn check_ok(rel: &str) {
    assert_success(run_ncc(&["check", &example(rel)]), rel);
}

fn check_fail(rel: &str, diag: &str) {
    assert_failure_contains(run_ncc(&["check", &example(rel)]), rel, diag);
}

#[test]
fn stoud_io_expands_to_runtime_imports() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "expand",
            &example("examples/imports/stoud_io_basic.n"),
            "--json",
        ]),
        "imports expand stoud io",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let imports = json["imports"].as_array().expect("imports");
    assert!(imports
        .iter()
        .any(|item| item["path"] == "nC.runtime.input"));
    assert!(imports
        .iter()
        .any(|item| item["path"] == "nC.runtime.output"));
    assert!(imports
        .iter()
        .any(|item| item["path"] == "nC.runtime.render"));
}

#[test]
fn stoud_memory_expands_to_runtime_imports() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "expand",
            &example("examples/imports/stoud_memory_basic.n"),
            "--json",
        ]),
        "imports expand stoud memory",
    );
    assert!(stdout.contains("nC.runtime.ptr"));
    assert!(stdout.contains("nC.runtime.region"));
}

#[test]
fn stoud_memory_map_is_a_known_runtime_import() {
    let source = example("examples/06_memory/memoryMapNativeSafe.n");
    let stdout = assert_success(
        run_ncc(&["imports", "expand", &source, "--json"]),
        "imports expand stoud memory map",
    );
    assert!(stdout.contains("nC.runtime.memory"), "{stdout}");
    assert_success(run_ncc(&["check", &source]), "check stoud memory map");
}

#[test]
fn stoud_collections_expands_to_runtime_imports() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "expand",
            &example("examples/imports/stoud_collections_basic.n"),
            "--json",
        ]),
        "imports expand stoud collections",
    );
    assert!(stdout.contains("nC.runtime.list"));
    assert!(stdout.contains("nC.runtime.option"));
    assert!(stdout.contains("nC.runtime.result"));
}

#[test]
fn stoud_io_fs_ops_expand_to_runtime_imports() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "expand",
            &example("examples/imports/stoud_io_fs_ops_basic.n"),
            "--json",
        ]),
        "imports expand stoud io fs ops",
    );
    assert!(stdout.contains("nC.runtime.random"));
    assert!(stdout.contains("nC.runtime.fs"));
}

#[test]
fn stoud_time_sleep_expands_to_runtime_time_import() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "expand",
            &example("examples/imports/stoud_time_sleep_basic.n"),
            "--json",
        ]),
        "imports expand stoud time sleep",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert!(
        json["imports"].as_array().is_some_and(Vec::is_empty),
        "sleep/time prelude must not inject a source import: {stdout}"
    );
    assert!(!stdout.contains("nC.runtime.sleep"), "{stdout}");
    check_ok("examples/imports/stoud_time_sleep_basic.n");
}

#[test]
fn stoud_io_fs_operations_are_calls_not_import_items() {
    check_fail(
        "examples/imports/negative/stoud_io_fs_operation_import_fail.n",
        "E_USE_ITEM_UNKNOWN_001",
    );
}

#[test]
fn stoud_single_item_import_ok() {
    check_ok("examples/imports/stoud_text_basic.n");
    check_ok("examples/imports/stoud_io_fs_ops_basic.n");
}

#[test]
fn auth_system_policy_is_optional() {
    check_ok("examples/imports/auth_system_policy.n");
    check_ok("examples/imports/negative/auth_without_policy_fail.n");
}

#[test]
fn dom_high_import_ok() {
    check_ok("examples/imports/dom_high_metadata.n");
}

#[test]
fn legacy_runtime_use_preserved() {
    check_ok("examples/imports/legacy_runtime_use_still_works.n");
}

#[test]
fn stoud_system_requires_auth_diagnostic() {
    check_fail(
        "examples/imports/negative/stoud_system_auth_required_fail.n",
        "E_USE_STOUD_AUTH_REQUIRED_001",
    );
}

#[test]
fn unknown_domain_fails() {
    check_fail(
        "examples/imports/negative/unknown_domain_fail.n",
        "E_USE_DOMAIN_UNKNOWN_001",
    );
}

#[test]
fn unknown_family_fails() {
    check_fail(
        "examples/imports/negative/unknown_family_fail.n",
        "E_USE_FAMILY_UNKNOWN_001",
    );
}

#[test]
fn unknown_item_fails() {
    check_fail(
        "examples/imports/negative/unknown_item_fail.n",
        "E_USE_ITEM_UNKNOWN_001",
    );
}

#[test]
fn imports_forbidden_in_machine() {
    check_fail(
        "examples/imports/negative/machine_import_forbidden_fail.nm",
        "E_USE_FORBIDDEN_IN_MACHINE_001",
    );
}

#[test]
fn imports_forbidden_in_primitive() {
    check_fail(
        "examples/imports/negative/primitive_import_forbidden_fail.ncp",
        "E_USE_FORBIDDEN_IN_PRIMITIVE_001",
    );
}

#[test]
fn imports_forbidden_in_basal() {
    check_fail(
        "examples/imports/negative/basal_import_forbidden_fail.nb",
        "E_USE_FORBIDDEN_IN_BASAL_001",
    );
}

#[test]
fn imports_forbidden_in_kernel() {
    check_fail(
        "examples/imports/negative/kernel_import_forbidden_fail.n",
        "E_USE_FORBIDDEN_IN_KERNEL_001",
    );
}

#[test]
fn import_manifest_closed() {
    let stdout = assert_success(
        run_ncc(&["imports", "manifest", "--json"]),
        "imports manifest",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "closed");
    assert!(json["entries"].as_array().expect("entries").len() >= 20);
}

#[test]
fn import_doctor_ok() {
    let stdout = assert_success(run_ncc(&["imports", "doctor", "--json"]), "imports doctor");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "closed");
    assert_eq!(json["stoud"], "closed");
    assert_eq!(json["auth"], "closed");
    assert_eq!(json["dom"], "closed");
    assert_eq!(json["legacy_use_nC_runtime"], "preserved");
}

#[test]
fn import_closure_plan_is_grouped_and_evidence_driven() {
    let stdout = assert_success(
        run_ncc(&["imports", "closure-plan", "--json"]),
        "imports closure plan",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "nc.imports.closure_plan.v1");
    assert_eq!(json["strategy"], "one_import_at_a_time_dependency_order");
    let pending = json["counts"]["pending"].as_u64().expect("pending count");
    let queue = json["queue"].as_array().expect("queue");
    assert_eq!(queue.len() as u64, pending);
    let families = json["families"].as_array().expect("family groups");
    assert_eq!(json["status"], "closed");
    assert_eq!(pending, 0);
    assert!(families.is_empty());
    assert_eq!(
        families
            .iter()
            .map(|family| family["pending"].as_u64().expect("family pending"))
            .sum::<u64>(),
        pending
    );
    assert!(queue.iter().all(|entry| entry["required_evidence"]
        .as_array()
        .is_some_and(|evidence| evidence.len() == 6)));
}

#[test]
fn import_backend_manifest_emits_real_bytes_for_each_import() {
    let stdout = assert_success(
        run_ncc(&["imports", "backend", "--json"]),
        "imports backend manifest",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "nc.imports.backend_lowering.v1");
    assert_eq!(json["status"], "closed");
    assert_eq!(
        json["mct_connection"],
        "connected_to_mct_import_activation_and_runtime_symbols"
    );
    let entries = json["entries"].as_array().expect("entries");
    assert!(entries.len() >= 60, "expected broad import backend catalog");
    let io_input = entries
        .iter()
        .find(|entry| entry["surface_import"] == "use::stoud::io::<input>;")
        .expect("stoud io input entry");
    assert_eq!(
        io_input["backend_symbol"],
        "__nc_import_activate_stoud_io_input"
    );
    assert_eq!(io_input["status"], "closed_real_backing_bytes");
    assert_eq!(io_input["backing_status"], "closed_real_backing_bytes");
    assert_eq!(io_input["closed_real_backing_bytes"], true);
    assert_eq!(
        io_input["runtime_call_lowering"],
        "direct_runtime_symbol_call_without_real_flag"
    );
    assert!(io_input["runtime_symbols"]
        .as_array()
        .expect("runtime symbols")
        .iter()
        .any(|symbol| symbol == "nC.runtime.input.readInt"));
    let bytes = io_input["emitted_bytes"].as_array().expect("bytes");
    assert_eq!(bytes.len(), 4);
    assert!(bytes.iter().any(|row| row["profile"] == "nC.sysv64"
        && row["bytes_hex"]
            .as_str()
            .is_some_and(|hex| hex.starts_with("b8") && hex.ends_with("c3"))));
    assert!(bytes
        .iter()
        .any(|row| row["profile"] == "nC.aapcs64" && row["byte_count"] == 8));
    assert!(bytes.iter().any(|row| row["profile"] == "nC.portable"
        && row["bytes_hex"]
            .as_str()
            .is_some_and(|hex| hex.starts_with("41") && hex.ends_with("0b"))));

    for (surface, symbol) in [
        ("use::stoud::io::<fsappend>;", "nC.runtime.fs.fsappend"),
        ("use::stoud::io::<fsync>;", "nC.runtime.fs.fsync"),
        ("use::stoud::net::<tcp>;", "nC.runtime.tcp.tcpRecv"),
        ("use::auth::system::<process>;", "nC.runtime.process.spawn"),
        (
            "use::auth::system::<thread>;",
            "nC.runtime.thread.threadJoin",
        ),
        ("use::stoud::sync::<sync>;", "nC.runtime.sync.mutexCreate"),
        ("use::stoud::sync::<lock>;", "nC.runtime.lock.rwLockCreate"),
        (
            "use::stoud::sync::<atomic>;",
            "nC.runtime.atomic.atomicCreate",
        ),
        (
            "use::stoud::sync::<channel>;",
            "nC.runtime.channel.channelCreate",
        ),
        ("use::stoud::time::<sleep>;", "nC.runtime.time.sleep"),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing backend entry {surface}"));
        assert_eq!(entry["status"], "closed_real_backing_bytes");
        assert_eq!(entry["backing_status"], "closed_real_backing_bytes");
        assert_eq!(entry["closed_real_backing_bytes"], true);
        assert_eq!(
            entry["runtime_call_lowering"],
            "direct_runtime_symbol_call_without_real_flag"
        );
        assert!(
            entry["runtime_symbols"]
                .as_array()
                .expect("runtime symbols")
                .iter()
                .any(|candidate| candidate == symbol),
            "{surface} must expose runtime symbol {symbol}: {entry}"
        );
    }

    let entropy = entries
        .iter()
        .find(|entry| entry["surface_import"] == "use::stoud::io::<entropy>;")
        .expect("entropy catalog entry");
    assert_eq!(entropy["status"], "closed_real_backing_bytes");
    assert_eq!(entropy["closed_real_backing_bytes"], true);
    assert_eq!(
        entropy["runtime_call_lowering"],
        "direct_runtime_symbol_call_without_real_flag"
    );
    let entropy_symbols = entropy["runtime_symbols"]
        .as_array()
        .expect("runtime symbols");
    assert!(entropy_symbols
        .iter()
        .any(|symbol| symbol == "nC.runtime.entropy.entropyBytes"));
    assert!(entropy_symbols
        .iter()
        .any(|symbol| symbol == "nC.runtime.entropy.entropyAvailable"));
}

#[test]
fn import_backend_manifest_filters_by_source_file() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "backend",
            &example("examples/imports/stoud_io_basic.n"),
            "--json",
        ]),
        "imports backend filtered manifest",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "closed");
    let entries = json["entries"].as_array().expect("entries");
    assert!(entries
        .iter()
        .any(|entry| entry["surface_import"] == "use::stoud::io::<input>;"));
    assert!(entries
        .iter()
        .any(|entry| entry["surface_import"] == "use::stoud::io::<output>;"));
    assert!(entries
        .iter()
        .any(|entry| entry["surface_import"] == "use::stoud::io::<render>;"));
    let source_paths = json["source_import_paths"]
        .as_array()
        .expect("source import paths");
    assert!(entries.iter().all(|entry| {
        entry["expands_to"].as_array().is_some_and(|expanded| {
            expanded
                .iter()
                .any(|path| source_paths.iter().any(|source| source == path))
        })
    }));
}

#[test]
fn runtime_import_table_uses_single_closure_status_contract() {
    let stdout = assert_success(
        run_ncc(&["imports", "runtime-table", "--json"]),
        "imports runtime table",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "nc.imports.runtime_closure_table.v1");
    assert_eq!(json["status"], "closed");
    let allowed = json["allowed_statuses"].as_array().expect("statuses");
    for status in [
        "closed_real_backing_bytes",
        "closed_native_backend",
        "closed_interpreter_runtime",
        "catalog_only_with_diagnostic",
    ] {
        assert!(allowed.iter().any(|candidate| candidate == status));
    }

    let entries = json["entries"].as_array().expect("entries");
    assert!(entries.len() >= 60);
    assert_eq!(
        json["total_entries"].as_u64().expect("total"),
        entries.len() as u64
    );
    assert!(json["closed_real_backing_bytes"].as_u64().expect("real") > 0);
    assert!(json["closed_native_backend"].as_u64().expect("native") > 0);
    assert!(
        json["closed_interpreter_runtime"]
            .as_u64()
            .expect("interpreter")
            > 0
    );
    assert_eq!(
        json["catalog_only_with_diagnostic"]
            .as_u64()
            .expect("catalog"),
        0
    );

    for entry in entries {
        let status = entry["closure_status"]
            .as_str()
            .expect("entry closure status");
        assert!(
            allowed.iter().any(|candidate| candidate == status),
            "unknown closure status {status}: {entry}"
        );
        if status == "catalog_only_with_diagnostic" {
            assert_eq!(entry["can_run_without_runtime_import_failure"], false);
            assert!(entry["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .any(|diag| diag
                    .as_str()
                    .is_some_and(|text| text.contains("E_USE_BACKING_DEFERRED_001"))));
        } else {
            assert_eq!(entry["can_run_without_runtime_import_failure"], true);
            assert!(entry["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .is_empty());
        }
    }

    for (surface, expected_status, expected_symbol) in [
        (
            "use::stoud::io::<fsappend>;",
            "closed_real_backing_bytes",
            "nC.runtime.fs.fsappend",
        ),
        (
            "use::stoud::net::<tcp>;",
            "closed_real_backing_bytes",
            "nC.runtime.tcp.tcpRecv",
        ),
        (
            "use::auth::system::<thread>;",
            "closed_real_backing_bytes",
            "nC.runtime.thread.threadJoin",
        ),
        (
            "use::stoud::sync::<sync>;",
            "closed_real_backing_bytes",
            "nC.runtime.sync.mutexCreate",
        ),
        (
            "use::stoud::sync::<atomic>;",
            "closed_real_backing_bytes",
            "nC.runtime.atomic.atomicCreate",
        ),
        (
            "use::stoud::sync::<channel>;",
            "closed_real_backing_bytes",
            "nC.runtime.channel.channelCreate",
        ),
        (
            "use::stoud::memory::<ptr>;",
            "closed_native_backend",
            "nC.runtime.ptr.ptrRead",
        ),
        (
            "use::stoud::io::<render>;",
            "closed_real_backing_bytes",
            "nC.runtime.render.renderInt",
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing runtime table entry {surface}"));
        assert_eq!(entry["closure_status"], expected_status);
        assert!(entry["runtime_symbols"]
            .as_array()
            .expect("runtime symbols")
            .iter()
            .any(|candidate| candidate == expected_symbol));
    }

    let entropy = entries
        .iter()
        .find(|entry| entry["surface_import"] == "use::stoud::io::<entropy>;")
        .expect("entropy catalog entry");
    assert_eq!(entropy["closure_status"], "closed_real_backing_bytes");
    assert_eq!(entropy["can_run_without_runtime_import_failure"], true);
}

#[test]
fn runtime_import_table_filters_by_source_file() {
    let stdout = assert_success(
        run_ncc(&[
            "imports",
            "runtime-table",
            &example("examples/runtime/runtime_real_fs_append_fsync.n"),
            "--json",
        ]),
        "imports runtime table filtered",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "nc.imports.runtime_closure_table.v1");
    let entries = json["entries"].as_array().expect("entries");
    assert!(entries
        .iter()
        .any(|entry| entry["surface_import"] == "use::stoud::io::<fsappend>;"));
    assert!(entries
        .iter()
        .any(|entry| entry["surface_import"] == "use::stoud::io::<fsync>;"));
    assert!(entries.iter().all(|entry| {
        entry["closure_status"] != "catalog_only_with_diagnostic"
            || entry["can_run_without_runtime_import_failure"] == false
    }));
}

#[test]
fn mct_imports_manifest_is_connected_to_import_backend_lowering() {
    let stdout = assert_success(run_ncc(&["mct", "imports", "--json"]), "mct imports");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "nc.imports.backend_lowering.v1");
    assert_eq!(json["status"], "closed");
    assert_eq!(
        json["mct_connection"],
        "connected_to_mct_import_activation_and_runtime_symbols"
    );
    assert!(json["groups"].as_array().expect("groups").len() >= 10);
}

#[test]
fn wildcard_dangerous_denied_fails() {
    check_fail(
        "examples/imports/negative/wildcard_dangerous_denied_fail.n",
        "E_USE_WILDCARD_DENIED_001",
    );
}
