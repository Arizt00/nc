use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

fn temp_dir(name: &str) -> PathBuf {
    workspace_root().join("target").join("tmp").join(name)
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn run_ncc(args: &[&str]) -> std::process::Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn assert_success(out: std::process::Output, label: &str) -> String {
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "{label} failed\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn write_dap(seq: i64, command: &str, arguments: serde_json::Value) -> Vec<u8> {
    let payload = serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": command,
        "arguments": arguments
    });
    let body = serde_json::to_vec(&payload).expect("dap json");
    let mut packet = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    packet.extend(body);
    packet
}

enum NativeDebugger {
    Gdb(PathBuf),
    Lldb(PathBuf),
}

fn find_native_debugger() -> Option<NativeDebugger> {
    for candidate in [
        "gdb",
        "/usr/bin/gdb",
        r"C:\msys64\mingw64\bin\gdb.exe",
        r"C:\msys64\ucrt64\bin\gdb.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() || Command::new(candidate).arg("--version").output().is_ok() {
            return Some(NativeDebugger::Gdb(path));
        }
    }
    for candidate in [
        "lldb",
        "/usr/bin/lldb",
        r"C:\Program Files\LLVM\bin\lldb.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() || Command::new(candidate).arg("--version").output().is_ok() {
            return Some(NativeDebugger::Lldb(path));
        }
    }
    None
}

fn fixture_file(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("create fixture root");
    let file = root.join("debug_adapter_fixture.n");
    fs::write(
        &file,
        r#"use::stoud::io::<output>;

debugAdapterFixture {
    to main {
        println("debug adapter ok");
    }

    fa:
        return true;
}
"#,
    )
    .expect("write fixture");
    file
}

fn native_debug_fixture(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("create native debug root");
    let file = root.join("native_debug_fixture.n");
    fs::write(
        &file,
        r#"nativeDebugFixture {
    fn:
        plusOne(value: int) -> int {
            public:
                @next >> int : value + 1;
                return @next;
        }

        run() -> int {
            public:
                @seed >> int : 10;
                @result >> int : plusOne(@seed);
                println(@result);
                return @result;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    )
    .expect("write native debug fixture");
    file
}

#[test]
fn debug_adapter_probe_and_source_map_are_real_cli_surfaces() {
    let probe = assert_success(
        run_ncc(&["debug-adapter", "--probe", "--json"]),
        "debug adapter probe",
    );
    let probe_json: serde_json::Value = serde_json::from_str(&probe).expect("probe json");
    assert_eq!(
        probe_json.get("status").and_then(|v| v.as_str()),
        Some("closed_source_and_machine_debug_routes")
    );
    assert_eq!(
        probe_json
            .pointer("/capabilities/supportsSetBreakpointsRequest")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        probe_json
            .pointer("/capabilities/supportsEvaluateForHovers")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        probe_json
            .pointer("/capabilities/supportsRestartRequest")
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    let root = temp_dir("debug_adapter_source_map");
    let _ = fs::remove_dir_all(&root);
    let file = fixture_file(&root);
    let map = assert_success(
        run_ncc(&[
            "debug-adapter",
            "--source-map",
            file.to_string_lossy().as_ref(),
            "--json",
        ]),
        "debug adapter source map",
    );
    let map_json: serde_json::Value = serde_json::from_str(&map).expect("source map json");
    assert_eq!(map_json.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert!(map_json
        .get("breakable_lines")
        .and_then(|v| v.as_array())
        .is_some_and(|lines| lines.iter().any(|line| line.as_i64() == Some(5))));
}

#[test]
fn debug_adapter_accepts_minimal_dap_launch_session() {
    let root = temp_dir("debug_adapter_dap_session");
    let _ = fs::remove_dir_all(&root);
    let file = fixture_file(&root);

    let mut child = Command::new(ncc_bin())
        .arg("debug-adapter")
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
                "source": {"path": file.to_string_lossy()},
                "breakpoints": [{"line": 5}]
            }),
        ))
        .expect("write breakpoints");
    stdin
        .write_all(&write_dap(
            3,
            "launch",
            serde_json::json!({
                "program": file.to_string_lossy(),
                "cwd": root.to_string_lossy()
            }),
        ))
        .expect("write launch");
    stdin
        .write_all(&write_dap(4, "configurationDone", serde_json::json!({})))
        .expect("write configurationDone");
    stdin
        .write_all(&write_dap(
            5,
            "stackTrace",
            serde_json::json!({"threadId": 1}),
        ))
        .expect("write stackTrace");
    stdin
        .write_all(&write_dap(
            6,
            "continue",
            serde_json::json!({"threadId": 1}),
        ))
        .expect("write continue");
    stdin
        .write_all(&write_dap(7, "disconnect", serde_json::json!({})))
        .expect("write disconnect");
    drop(stdin);

    let output = child.wait_with_output().expect("wait dap");
    let stdout = assert_success(output, "dap session");
    assert!(stdout.contains("\"event\":\"initialized\""), "{stdout}");
    assert!(
        stdout.contains("\"command\":\"setBreakpoints\""),
        "{stdout}"
    );
    assert!(stdout.contains("\"event\":\"stopped\""), "{stdout}");
    assert!(stdout.contains("\"command\":\"stackTrace\""), "{stdout}");
    assert!(stdout.contains("debug adapter ok"), "{stdout}");
    assert!(stdout.contains("\"event\":\"terminated\""), "{stdout}");
}

#[test]
fn machine_debug_adapter_evaluates_register_expressions_and_restarts_cleanly() {
    let machine = workspace_root().join("examples/machine/jit_return42_x86_64.nm");
    let mut child = Command::new(ncc_bin())
        .arg("debug-adapter")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn machine debug adapter");
    let mut stdin = child.stdin.take().expect("dap stdin");
    for packet in [
        write_dap(1, "initialize", serde_json::json!({})),
        write_dap(
            2,
            "launch",
            serde_json::json!({
                "program": machine.to_string_lossy(),
                "cwd": workspace_root().to_string_lossy()
            }),
        ),
        write_dap(3, "configurationDone", serde_json::json!({})),
        write_dap(
            4,
            "evaluate",
            serde_json::json!({"expression": "$rax + 2", "context": "watch"}),
        ),
        write_dap(5, "restart", serde_json::json!({})),
        write_dap(6, "disconnect", serde_json::json!({})),
    ] {
        stdin.write_all(&packet).expect("write DAP packet");
    }
    drop(stdin);
    let stdout = assert_success(
        child.wait_with_output().expect("wait machine DAP"),
        "machine DAP",
    );
    assert!(stdout.contains("\"command\":\"evaluate\""), "{stdout}");
    assert!(stdout.contains("\"result\":\"2\""), "{stdout}");
    assert!(stdout.contains("\"command\":\"restart\""), "{stdout}");
    assert!(stdout.contains("\"startMethod\":\"restart\""), "{stdout}");
    assert!(
        stdout.matches("\"event\":\"terminated\"").count() >= 2,
        "initial run, restart and disconnect must terminate cleanly: {stdout}"
    );
}

#[test]
fn native_debug_symbols_drive_real_breakpoints_stack_variables_evaluate_and_stepping() {
    let Some(debugger) = find_native_debugger() else {
        eprintln!("SKIP: neither GDB nor LLDB is installed on this host");
        return;
    };
    let root =
        std::env::temp_dir().join(format!("ncc-native-debug-symbols-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let source = native_debug_fixture(&root);
    let artifact_root = root.join(".ncc/debug");
    let executable = artifact_root.join("bin/native_debug_fixture.debug.exe");
    assert_success(
        run_ncc(&[
            "compiler",
            "build-native",
            source.to_string_lossy().as_ref(),
            "--profile",
            "debug",
            "--out-dir",
            artifact_root.to_string_lossy().as_ref(),
            "--out",
            executable.to_string_lossy().as_ref(),
            "--no-run",
            "--strict",
        ]),
        "native debug build",
    );
    assert!(executable.is_file(), "debug executable must exist");

    let (output, debugger_name) = match debugger {
        NativeDebugger::Gdb(debugger) => {
            let breakpoint = format!("break {}:12", source.display());
            let output = Command::new(debugger)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .args([
                    "-q",
                    "-batch",
                    "-ex",
                    "set confirm off",
                    "-ex",
                    "set debuginfod enabled off",
                    "-ex",
                    &breakpoint,
                    "-ex",
                    "run",
                    "-ex",
                    "bt",
                    "-ex",
                    "info locals",
                    "-ex",
                    "print seed + 2",
                    "-ex",
                    "step",
                    "-ex",
                    "finish",
                    "-ex",
                    "next",
                    "-ex",
                    "kill",
                    "-ex",
                    "run",
                    "-ex",
                    "continue",
                ])
                .arg(&executable)
                .output()
                .expect("run GDB native debugger");
            (output, "gdb")
        }
        NativeDebugger::Lldb(debugger) => {
            let breakpoint = format!(
                "breakpoint set --file {} --line 12",
                source.file_name().unwrap().to_string_lossy()
            );
            let output = Command::new(debugger)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("DYLD_LIBRARY_PATH")
                .args([
                    "--batch",
                    "-o",
                    &breakpoint,
                    "-o",
                    "run",
                    "-o",
                    "bt",
                    "-o",
                    "frame variable seed",
                    "-o",
                    "expression seed + 2",
                    "-o",
                    "step",
                    "-o",
                    "finish",
                    "-o",
                    "next",
                    "-o",
                    "process kill",
                    "-o",
                    "run",
                    "-o",
                    "continue",
                ])
                .arg(&executable)
                .output()
                .expect("run LLDB native debugger");
            (output, "lldb")
        }
    };
    let stdout = assert_success(output, "native debugger session");
    assert!(
        stdout.contains("Breakpoint 1") || stdout.contains("breakpoint 1"),
        "debugger={debugger_name}\n{stdout}"
    );
    assert!(stdout.contains("nativeDebugFixture_run"), "{stdout}");
    assert!(
        stdout.contains("seed = 10") || stdout.contains("seed = (long) 10"),
        "{stdout}"
    );
    assert!(stdout.contains("= 12"), "{stdout}");
    assert!(stdout.contains("nativeDebugFixture_plusOne"), "{stdout}");
    assert!(stdout.contains("= 11"), "{stdout}");
    assert!(
        stdout.contains("exited with code")
            || stdout.contains("exited with status")
            || stdout.contains("Process") && stdout.contains("exited"),
        "{stdout}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn project_clean_incremental_check_skips_linked_semantic_revalidation() {
    let root = temp_dir("project_cached_check_semantics");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&["project", "init", root.to_string_lossy().as_ref()]),
        "project init",
    );
    fs::write(
        root.join("main.n"),
        r#"use <project:child>;

mainProject {
    fn:
        public:
            run() -> bool {
                return @fn(childUnit.ok);
            }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    )
    .expect("write main");
    fs::write(
        root.join("src").join("child.n"),
        r#"childUnit {
    fn:
        public:
            ok() -> bool {
                return true;
            }

    fa:
        return true;
}
"#,
    )
    .expect("write child");
    assert_success(
        run_ncc(&["project", "sync", root.to_string_lossy().as_ref()]),
        "project sync",
    );
    let incremental = assert_success(
        run_ncc(&[
            "project",
            "incremental",
            root.to_string_lossy().as_ref(),
            "--json",
        ]),
        "project incremental",
    );
    let incremental_json: serde_json::Value =
        serde_json::from_str(&incremental).expect("incremental json");
    assert_eq!(
        incremental_json
            .pointer("/cache/semantic_validation")
            .and_then(|v| v.as_str()),
        Some("skipped_linked_clean_cache")
    );

    let check = assert_success(
        run_ncc(&[
            "project",
            "check",
            root.to_string_lossy().as_ref(),
            "--json",
        ]),
        "project cached check",
    );
    let check_json: serde_json::Value = serde_json::from_str(&check).expect("check json");
    assert_eq!(
        check_json.get("schema").and_then(|v| v.as_str()),
        Some("ncc.project.cached_check.v1")
    );
    assert_eq!(
        check_json
            .get("semantic_validation")
            .and_then(|v| v.as_str()),
        Some("skipped_linked_clean_cache")
    );
}
