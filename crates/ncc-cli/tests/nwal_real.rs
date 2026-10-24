use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn nwal_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_nwal")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_nwal must be set")
}

fn run_nwal(args: &[&str]) -> Output {
    Command::new(nwal_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("nwal command must execute")
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn nwal_real_e2e_persists_replicates_and_recovers() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!(
        "nwal-real-e2e-test-{}-{unique}",
        std::process::id()
    ));
    let data_dir_raw = data_dir.to_string_lossy().to_string();
    let out = run_nwal(&["e2e", "--data-dir", &data_dir_raw]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    assert!(rendered.contains("E2E REAL OK"), "{rendered}");

    for node in ["leader", "follower_a", "follower_b"] {
        let data = std::fs::read_to_string(data_dir.join(node).join("data.tsv"))
            .expect("data file must exist");
        assert!(data.contains("account:1\t250"), "{node}: {data}");

        let wal = std::fs::read_to_string(data_dir.join(node).join("wal.log"))
            .expect("wal file must exist");
        assert!(
            wal.contains("1|CREATE|account:1|100|COMMITTED"),
            "{node}: {wal}"
        );
        assert!(
            wal.contains("2|UPDATE|account:1|250|COMMITTED"),
            "{node}: {wal}"
        );
    }

    let _ = std::fs::remove_dir_all(data_dir);
}
