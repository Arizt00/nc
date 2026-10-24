use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    if let Some(stripped) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(stripped);
        if archived.exists() {
            return archived;
        }
    }
    direct
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc executes")
}

fn assert_success(output: Output, label: &str) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
    assert!(
        output.status.success(),
        "{label} must succeed\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn write_dap(seq: i64, command: &str, arguments: serde_json::Value) -> Vec<u8> {
    let body = serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": command,
        "arguments": arguments
    })
    .to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
}

#[test]
fn machine_debug_bridge_probe_and_x86_register_step_are_closed() {
    let probe = assert_success(
        run_ncc(&["machine", "debug-bridge", "probe", "--json"]),
        "machine debug bridge probe",
    );
    let probe_json: serde_json::Value = serde_json::from_str(&probe).expect("probe json");
    assert_eq!(
        probe_json.get("status").and_then(|v| v.as_str()),
        Some("closed_machine_debug_bridge")
    );
    assert_eq!(
        probe_json
            .pointer("/capabilities/cpu_registers")
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    let source = fixture("examples/machine/native/debug_unwind_elf64_x86_64.nm");
    let out = assert_success(
        run_ncc(&[
            "machine",
            "debug-bridge",
            source.to_string_lossy().as_ref(),
            "--target",
            "x86_64-unknown-linux-gnu",
            "--arg0",
            "7",
            "--arg1",
            "35",
            "--trace",
            "--json",
        ]),
        "machine debug bridge x86 trace",
    );
    let json: serde_json::Value = serde_json::from_str(&out).expect("bridge json");
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("closed_machine_debug_bridge")
    );
    let rax = json
        .get("registers")
        .and_then(|v| v.as_array())
        .and_then(|regs| {
            regs.iter()
                .find(|reg| reg.get("name").and_then(|v| v.as_str()) == Some("rax"))
        })
        .expect("rax register");
    assert_eq!(rax.get("value").and_then(|v| v.as_str()), Some("42"));
    assert_eq!(
        rax.get("value_hex").and_then(|v| v.as_str()),
        Some("0x000000000000002a")
    );
    assert!(json
        .get("trace")
        .and_then(|v| v.as_array())
        .is_some_and(|steps| steps.iter().any(|step| {
            step.get("mnemonic").and_then(|v| v.as_str()) == Some("add")
                && step.get("semantic_status").and_then(|v| v.as_str())
                    == Some("closed_register_step")
        })));
}

#[test]
fn debug_adapter_probe_advertises_live_os_attach_surface() {
    let probe = assert_success(
        run_ncc(&["debug-adapter", "--probe", "--json"]),
        "debug adapter probe",
    );
    let probe_json: serde_json::Value = serde_json::from_str(&probe).expect("dap probe json");
    assert_eq!(
        probe_json.get("status").and_then(|v| v.as_str()),
        Some("closed_source_and_machine_debug_routes")
    );
    assert_eq!(
        probe_json
            .pointer("/capabilities/supportsAttachRequest")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        probe_json
            .pointer("/live_os_attach/status")
            .and_then(|v| v.as_str()),
        Some("closed_snapshot_then_detach")
    );
    assert!(probe_json
        .pointer("/live_os_attach/capabilities")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items
            .iter()
            .any(|item| item.as_str() == Some("process_snapshot"))));
}

#[test]
fn machine_debug_attach_probe_and_invalid_pid_are_typed() {
    let probe = assert_success(
        run_ncc(&["machine", "debug-attach", "probe", "--json"]),
        "machine debug attach probe",
    );
    let probe_json: serde_json::Value = serde_json::from_str(&probe).expect("attach probe json");
    assert_eq!(
        probe_json.get("status").and_then(|v| v.as_str()),
        Some("closed_snapshot_then_detach")
    );
    assert!(probe_json
        .pointer("/capabilities")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items
            .iter()
            .any(|item| item.as_str() == Some("process_snapshot"))));

    let output = run_ncc(&["machine", "debug-attach", "--pid", "999999999", "--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        !output.status.success(),
        "invalid pid attach must fail\nstdout={stdout}"
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("invalid pid json");
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("failed"));
    assert!(json
        .get("diagnostics")
        .and_then(|v| v.as_array())
        .is_some_and(|rows| rows.iter().any(|row| row
            .get("code")
            .and_then(|v| v.as_str())
            .is_some_and(|code| code.starts_with("E_MACHINE_DEBUG_ATTACH")))));
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn machine_debug_attach_self_test_reads_linux_x86_64_registers() {
    let output = assert_success(
        run_ncc(&["machine", "debug-attach", "self-test", "--json"]),
        "machine debug attach self-test",
    );
    let json: serde_json::Value = serde_json::from_str(&output).expect("self-test json");
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("closed_live_os_attach")
    );
    assert_eq!(json.get("self_test").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        json.get("authority").and_then(|v| v.as_str()),
        Some("linux.ptrace")
    );
    assert!(json
        .get("registers")
        .and_then(|v| v.as_array())
        .is_some_and(|registers| registers
            .iter()
            .any(|reg| reg.get("name").and_then(|v| v.as_str()) == Some("rip"))));
}

#[test]
fn debug_adapter_exposes_machine_register_scope_for_nm_files() {
    let source = fixture("examples/machine/native/debug_unwind_elf64_x86_64.nm");
    let mut child = Command::new(ncc_bin())
        .arg("debug-adapter")
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn debug adapter");

    let mut stdin = child.stdin.take().expect("dap stdin");
    stdin
        .write_all(&write_dap(1, "initialize", serde_json::json!({})))
        .expect("write initialize");
    stdin
        .write_all(&write_dap(
            2,
            "setBreakpoints",
            serde_json::json!({
                "source": {"path": source.to_string_lossy()},
                "breakpoints": [{"line": 1}]
            }),
        ))
        .expect("write breakpoints");
    stdin
        .write_all(&write_dap(
            3,
            "launch",
            serde_json::json!({
                "program": source.to_string_lossy(),
                "cwd": workspace_root().to_string_lossy()
            }),
        ))
        .expect("write launch");
    stdin
        .write_all(&write_dap(4, "configurationDone", serde_json::json!({})))
        .expect("write configurationDone");
    stdin
        .write_all(&write_dap(5, "scopes", serde_json::json!({"frameId": 1})))
        .expect("write scopes");
    stdin
        .write_all(&write_dap(
            6,
            "variables",
            serde_json::json!({"variablesReference": 1}),
        ))
        .expect("write variables");
    stdin
        .write_all(&write_dap(7, "disconnect", serde_json::json!({})))
        .expect("write disconnect");
    drop(stdin);

    let output = child.wait_with_output().expect("wait dap");
    let stdout = assert_success(output, "dap machine register scope");
    assert!(stdout.contains("\"event\":\"stopped\""), "{stdout}");
    assert!(stdout.contains("Machine Registers"), "{stdout}");
    assert!(stdout.contains("\"name\":\"rax\""), "{stdout}");
    assert!(stdout.contains("\"type\":\"return\""), "{stdout}");
}
