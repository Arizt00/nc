use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

fn run_ncc(args: &[&str]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn runtime_fixture(path: &str) -> String {
    let root = workspace_root();
    if root.join(path).exists() {
        return path.to_string();
    }
    if let Some(suffix) = path.strip_prefix("examples/runtime/") {
        let archived = format!("_archive/old_examples/runtime/{suffix}");
        if root.join(&archived).exists() {
            return archived;
        }
    }
    path.to_string()
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn assert_run_ok(path: &str, marker: &str) {
    let resolved = runtime_fixture(path);
    let out = run_ncc(&["run", resolved.as_str(), "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{resolved}\n{rendered}");
    assert!(rendered.contains(marker), "{rendered}");
}

fn assert_run_success(path: &str) {
    let resolved = runtime_fixture(path);
    let out = run_ncc(&["run", resolved.as_str(), "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{resolved}\n{rendered}");
}

fn assert_ncc_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    let rendered = combined(&out);
    assert!(out.status.success(), "{args:?}\n{rendered}");
    serde_json::from_str(&rendered)
        .unwrap_or_else(|err| panic!("{args:?} must return json: {err}\n{rendered}"))
}

fn assert_abi_symbol(symbol_name: &str, return_type: &str) {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    assert!(
        json["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(
                |symbol| symbol["symbol"] == symbol_name && symbol["return"]["type"] == return_type
            ),
        "{symbol_name} -> {return_type} must be present in runtime ABI"
    );
}

fn assert_abi_layout(layout_type: &str) {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    assert!(
        json["layouts"]
            .as_array()
            .expect("layouts")
            .iter()
            .any(|layout| layout["type"] == layout_type),
        "{layout_type} must be present in runtime ABI layouts"
    );
}

fn powershell_exe_available() -> bool {
    Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("exit 0")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn runtime_abi_cli_exposes_stable_symbol_contracts() {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    assert_eq!(json["schema"], "nc.runtime.symbol_abi.v1");
    assert_eq!(json["status"], "closed");
    assert!(json["symbol_count"].as_u64().expect("symbol count") >= 45);
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "Result<T,E>"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "FileLockHandle"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "TcpError"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "NetworkError"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "MutexHandle"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "AtomicHandle"));
    assert!(json["layouts"]
        .as_array()
        .expect("layouts")
        .iter()
        .any(|layout| layout["type"] == "ChannelHandle"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.thread.threadJoin"
            && symbol["return"]["type"] == "Result<any,ThreadError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.fs.writeBytes"
            && symbol["return"]["type"] == "Result<int,FsError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.fs.readBytesInto"
            && symbol["return"]["type"] == "Result<int,FsError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.udp.udpRecvFrom"
            && symbol["return"]["type"] == "Result<str,NetworkError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.tls.tlsConnect"
            && symbol["return"]["type"] == "Result<TcpHandle,NetworkError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.process.spawnPipe"
            && symbol["return"]["type"] == "Result<ProcessHandle,ProcessError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(
            |symbol| symbol["symbol"] == "nC.runtime.process.waitTimeout"
                && symbol["return"]["type"] == "Result<str,ProcessError>"
        ));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(
            |symbol| symbol["symbol"] == "nC.runtime.process.signalGroup"
                && symbol["return"]["type"] == "Result<bool,ProcessError>"
        ));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(
            |symbol| symbol["symbol"] == "nC.runtime.thread.threadJoinTimeout"
                && symbol["return"]["type"] == "Result<any,ThreadError>"
        ));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.sync.mutexCreate"
            && symbol["return"]["type"] == "Result<MutexHandle,ThreadError>"));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(
            |symbol| symbol["symbol"] == "nC.runtime.atomic.atomicCreate"
                && symbol["return"]["type"] == "Result<AtomicHandle,ThreadError>"
        ));
    assert!(json["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .any(
            |symbol| symbol["symbol"] == "nC.runtime.channel.channelCreate"
                && symbol["return"]["type"] == "Result<ChannelHandle,ThreadError>"
        ));

    let spawn = assert_ncc_json(&["runtime", "abi", "nC.runtime.process.spawn", "--json"]);
    assert!(spawn["symbol_count"].as_u64().expect("spawn symbol count") >= 1);
    assert!(spawn["symbols"]
        .as_array()
        .expect("spawn symbols")
        .iter()
        .any(|symbol| symbol["symbol"] == "nC.runtime.process.spawn"
            && symbol["return"]["type"] == "Result<ProcessHandle,ProcessError>"));
}

#[test]
fn fs_append_fsync_and_time_sleep_examples_run_with_facade_imports() {
    assert_run_ok("examples/07_runtime/runtime_io.n", "fswrite = true");
    assert_run_ok(
        "examples/07_runtime/runtime_std_punto9.n",
        "Runtime estandar consolidado",
    );
    assert_abi_symbol("nC.runtime.fs.fsappend", "Result<bool,FsError>");
    assert_abi_symbol("nC.runtime.fs.fsync", "Result<bool,FsError>");
    assert_abi_symbol("nC.runtime.fs.lockFile", "Result<FileLockHandle,FsError>");
    assert_run_ok(
        "examples/runtime/runtime_real_time_sleep.n",
        "Tiempo real del runtime",
    );
    assert_run_success("examples/runtime/runtime_real_process_child_sleep.n");
}

#[test]
fn real_network_priority4_examples_run_with_facade_imports() {
    assert_abi_symbol("nC.runtime.tcp.tcpListenOn", "Result<TcpHandle,TcpError>");
    assert_abi_symbol("nC.runtime.tcp.tcpRecv", "Result<str,TcpError>");
    assert_abi_symbol("nC.runtime.udp.udpRecvFrom", "Result<str,NetworkError>");
    assert_abi_symbol("nC.runtime.dns.resolve", "Result<str,NetworkError>");
    assert_abi_symbol(
        "nC.runtime.tls.tlsConnect",
        "Result<TcpHandle,NetworkError>",
    );
}

#[test]
fn process_id_and_spawn_kill_examples_run_with_typed_results() {
    assert_abi_symbol("nC.runtime.process.processId", "int");
    assert_abi_symbol("nC.runtime.process.parentProcessId", "int");
    assert_abi_symbol(
        "nC.runtime.process.spawn",
        "Result<ProcessHandle,ProcessError>",
    );
    assert_abi_symbol("nC.runtime.process.kill", "Result<bool,ProcessError>");
}

#[test]
fn process_spawn_wait_runs_real_child_and_waits_exit_code() {
    assert_run_ok(
        "examples/07_runtime/backend_mct_real_bytes.n",
        "process-exit=0",
    );
    assert_abi_symbol("nC.runtime.process.wait", "Result<int,ProcessError>");
}

#[test]
fn process_spawn_powershell_command_preserves_args_with_spaces() {
    if !powershell_exe_available() {
        eprintln!("skipping powershell arg contract: powershell.exe not available");
        return;
    }

    assert_run_ok(
        "examples/07_runtime/runtime_real_process_powershell_args.n",
        "ps-args-ok = true",
    );
}

#[test]
fn process_priority5_strong_runtime_examples_run_with_typed_results() {
    assert_abi_symbol(
        "nC.runtime.process.spawnPipe",
        "Result<ProcessHandle,ProcessError>",
    );
    assert_abi_symbol("nC.runtime.process.writeStdin", "Result<int,ProcessError>");
    assert_abi_symbol("nC.runtime.process.readStdout", "Result<str,ProcessError>");
    assert_abi_symbol("nC.runtime.process.readStderr", "Result<str,ProcessError>");
    assert_abi_symbol("nC.runtime.process.waitTimeout", "Result<str,ProcessError>");
    assert_abi_symbol(
        "nC.runtime.process.signalGroup",
        "Result<bool,ProcessError>",
    );
}

#[test]
fn thread_spawn_join_runs_real_thread_and_returns_payload() {
    let file = workspace_root().join("target/tmp/backend_mct_thread.txt");
    let _ = std::fs::remove_file(&file);

    assert_run_ok(
        "examples/07_runtime/backend_mct_real_bytes.n",
        "thread-file=true",
    );
}

#[test]
fn thread_join_timeout_and_sync_runtime_examples_run_with_typed_results() {
    assert_abi_symbol(
        "nC.runtime.thread.threadJoinTimeout",
        "Result<any,ThreadError>",
    );
    assert_abi_symbol(
        "nC.runtime.sync.mutexCreate",
        "Result<MutexHandle,ThreadError>",
    );
    assert_abi_symbol(
        "nC.runtime.atomic.atomicCreate",
        "Result<AtomicHandle,ThreadError>",
    );
    assert_abi_symbol(
        "nC.runtime.channel.channelCreate",
        "Result<ChannelHandle,ThreadError>",
    );
    assert_abi_symbol(
        "nC.runtime.channel.channelRecvTimeout",
        "Result<any,ThreadError>",
    );
}

#[test]
fn process_wait_invalid_handle_returns_process_error() {
    assert_abi_symbol("nC.runtime.process.wait", "Result<int,ProcessError>");
    assert_abi_layout("ProcessError");
}

#[test]
fn process_kill_missing_process_returns_process_error() {
    assert_abi_symbol("nC.runtime.process.kill", "Result<bool,ProcessError>");
    assert_abi_layout("ProcessError");
}
