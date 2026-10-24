use std::path::PathBuf;
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
        .expect("workspace root")
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .current_dir(workspace_root())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn temp_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-diagnostic-machine-v2-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temporary evidence directory");
    path
}

#[test]
fn machine_diagnostic_record_emits_object_and_loader_executable() {
    let source = workspace_root().join("examples/machine/diagnostic_record_v2_x86_64.nm");
    let evidence = temp_dir();
    let object = evidence.join("diagnostic-record-v2.o");
    let executable = evidence.join("diagnostic-record-v2.elf");

    let check = run_ncc(&[
        "machine".into(),
        "check".into(),
        source.to_string_lossy().to_string(),
    ]);
    assert!(check.status.success(), "{}", rendered(&check));

    let emit = run_ncc(&[
        "machine".into(),
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--out".into(),
        object.to_string_lossy().to_string(),
    ]);
    assert!(emit.status.success(), "{}", rendered(&emit));
    let object_bytes = std::fs::read(&object).expect("machine object");
    assert_eq!(&object_bytes[..4], b"\x7fELF");
    assert!(contains_bytes(
        &object_bytes,
        &[0x4e, 0x43, 0x44, 0x49, 0x02, 0x00, 0x80, 0x00]
    ));

    let link = run_ncc(&[
        "machine".into(),
        "build-exe".into(),
        source.to_string_lossy().to_string(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--out".into(),
        executable.to_string_lossy().to_string(),
    ]);
    assert!(link.status.success(), "{}", rendered(&link));
    let executable_bytes = std::fs::read(&executable).expect("machine executable");
    assert_eq!(&executable_bytes[..4], b"\x7fELF");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&executable)
            .expect("executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).expect("mark executable");
        let execution = Command::new(&executable)
            .output()
            .expect("execute diagnostic machine image");
        assert!(execution.status.success(), "{}", rendered(&execution));
    }

    std::fs::remove_dir_all(evidence).expect("clean temporary evidence");
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
