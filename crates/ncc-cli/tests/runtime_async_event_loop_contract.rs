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

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn fixture_path(rel: &str) -> PathBuf {
    let root = workspace_root();
    let primary = root.join(rel);
    if primary.exists() {
        return primary;
    }
    if let Some(rest) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    primary
}

fn assert_run_ok(path: &str, marker: &str) {
    let fixture = fixture_path(path);
    let fixture = fixture.to_string_lossy().to_string();
    let out = run_ncc(&["run", &fixture, "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{path}\n{rendered}");
    assert!(rendered.contains(marker), "{path}\n{rendered}");
}

fn assert_ncc_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    let rendered = combined(&out);
    assert!(out.status.success(), "{args:?}\n{rendered}");
    serde_json::from_str(&rendered)
        .unwrap_or_else(|err| panic!("{args:?} must return json: {err}\n{rendered}"))
}

#[test]
fn async_timer_task_scheduler_cancel_examples_run() {
    assert_run_ok(
        "examples/runtime/runtime_real_async_timer_await.n",
        "async-timer-ok=true",
    );
    assert_run_ok(
        "examples/runtime/runtime_real_task_scheduler_cancel.n",
        "task-scheduler-cancel-ok=true",
    );
    assert_run_ok(
        "examples/runtime/negative/runtime_real_async_invalid_await_result.n",
        "invalid-await-err=true",
    );
}

#[test]
fn tcp_async_loopback_runs_real_socket_future_flow() {
    assert_run_ok(
        "examples/runtime/runtime_real_tcp_async_loopback.n",
        "tcp-async-ok=true",
    );
}

#[test]
fn async_event_loop_imports_are_closed_real_backing_bytes() {
    let json = assert_ncc_json(&["imports", "runtime-table", "--json"]);
    let entries = json["entries"].as_array().expect("entries");

    for (surface, symbol) in [
        ("use::stoud::time::<timer>;", "nC.runtime.timer.timeout"),
        ("use::stoud::task::<async>;", "nC.runtime.async.await"),
        ("use::stoud::task::<task>;", "nC.runtime.task.taskSpawn"),
        (
            "use::stoud::task::<scheduler>;",
            "nC.runtime.scheduler.scheduleAfter",
        ),
        (
            "use::stoud::task::<cancel>;",
            "nC.runtime.cancel.cancelToken",
        ),
        ("use::stoud::net::<tcp>;", "nC.runtime.tcp.tcpConnectAsync"),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing runtime table entry {surface}"));
        assert_eq!(entry["closure_status"], "closed_real_backing_bytes");
        assert_eq!(entry["can_run_without_runtime_import_failure"], true);
        assert!(entry["runtime_symbols"]
            .as_array()
            .expect("runtime symbols")
            .iter()
            .any(|candidate| candidate == symbol));
    }
}

#[test]
fn async_event_loop_runtime_abi_is_stable_and_typed() {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    let layouts = json["layouts"].as_array().expect("layouts");
    for required in [
        "FutureHandle",
        "TaskHandle",
        "TimerHandle",
        "CancelToken",
        "EventHandle",
        "AsyncError",
    ] {
        assert!(
            layouts.iter().any(|layout| layout["type"] == required),
            "missing layout {required}"
        );
    }

    let symbols = json["symbols"].as_array().expect("symbols");
    for (symbol, return_ty) in [
        (
            "nC.runtime.timer.timeout",
            "Result<FutureHandle,AsyncError>",
        ),
        ("nC.runtime.async.await", "Result<any,AsyncError>"),
        ("nC.runtime.task.taskSpawn", "Result<TaskHandle,AsyncError>"),
        (
            "nC.runtime.scheduler.scheduleEvery",
            "Result<TaskHandle,AsyncError>",
        ),
        (
            "nC.runtime.cancel.cancelToken",
            "Result<CancelToken,AsyncError>",
        ),
        (
            "nC.runtime.tcp.tcpConnectAsync",
            "Result<FutureHandle,AsyncError>",
        ),
        (
            "nC.runtime.tcp.tcpRecvAsync",
            "Result<FutureHandle,AsyncError>",
        ),
    ] {
        assert!(
            symbols
                .iter()
                .any(|row| row["symbol"] == symbol && row["return"]["type"] == return_ty),
            "missing ABI row {symbol} -> {return_ty}"
        );
    }
}
