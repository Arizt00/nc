use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

static WAL_CLUSTER_TEST_LOCK: Mutex<()> = Mutex::new(());

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

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
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

fn fixture_arg(rel: &str) -> String {
    fixture_path(rel).to_string_lossy().to_string()
}

#[test]
fn wal_single_flow_and_follower_check_keep_runtime_imports_backed() {
    let _lock = WAL_CLUSTER_TEST_LOCK.lock().unwrap();
    let wal_single = fixture_arg("examples/runtime/wal_real_n_runtime.n");
    let rendered = assert_success(
        run_ncc(&["run", &wal_single, "--interpreted"]),
        "wal real single flow",
    );
    assert!(rendered.contains("WAL real .n completado"), "{rendered}");

    let wal_follower = fixture_arg("examples/runtime/walFollower.n");
    let rendered = assert_success(run_ncc(&["check", &wal_follower]), "wal follower check");
    assert!(
        rendered.contains("OK:") && rendered.contains("walFollower.n"),
        "{rendered}"
    );
}

#[test]
fn wal_cluster_n_examples_spawn_followers_recover_and_persist_majority() {
    let _lock = WAL_CLUSTER_TEST_LOCK.lock().unwrap();
    let root = workspace_root().join("target/tmp/nc_wal_real_cluster");
    let _ = std::fs::remove_dir_all(&root);

    let wal_leader = fixture_arg("examples/runtime/walLeader.n");
    let out = run_ncc(&["run", &wal_leader, "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    assert!(
        rendered.contains("WAL cluster real .n OK=true"),
        "{rendered}"
    );

    for node in ["leader", "follower_a", "follower_b"] {
        let wal_path = root.join(node).join("wal.log");
        let data_path = root.join(node).join("data.tsv");
        let wal = std::fs::read_to_string(&wal_path)
            .unwrap_or_else(|err| panic!("{} must be readable: {err}", wal_path.display()));
        let data = std::fs::read_to_string(&data_path)
            .unwrap_or_else(|err| panic!("{} must be readable: {err}", data_path.display()));

        assert!(
            wal.contains("1|CREATE|account:1|100|PENDING")
                && wal.contains("1|CREATE|account:1|100|COMMITTED"),
            "{node} WAL must contain pending and committed records, got {wal:?}"
        );
        assert!(
            data.contains("account:1\t100"),
            "{node} data must contain committed key, got {data:?}"
        );
    }
}

#[test]
fn wal_cluster_n_negative_without_followers_keeps_transaction_pending() {
    let _lock = WAL_CLUSTER_TEST_LOCK.lock().unwrap();
    let root = workspace_root().join("target/tmp/nc_wal_real_cluster_negative");
    let _ = std::fs::remove_dir_all(&root);

    let wal_no_majority = fixture_arg("examples/runtime/negative/walLeaderNoMajority.n");
    let out = run_ncc(&["run", &wal_no_majority, "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    assert!(
        rendered.contains("WAL no majority committed=false"),
        "{rendered}"
    );

    let wal_path = root.join("leader").join("wal.log");
    let wal = std::fs::read_to_string(&wal_path)
        .unwrap_or_else(|err| panic!("{} must be readable: {err}", wal_path.display()));
    assert!(wal.contains("PENDING"), "{wal}");
    assert!(!wal.contains("COMMITTED"), "{wal}");
}

#[test]
fn tcp_recv_typed_result_and_unwrap_infer_str() {
    let tcp_fixture = fixture_arg("examples/runtime/runtime_real_tcp_recv_typed_result.n");
    let out = run_ncc(&["run", &tcp_fixture, "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    assert!(
        rendered.contains("tcpRecv Result<str,TcpError>=true"),
        "{rendered}"
    );
}
