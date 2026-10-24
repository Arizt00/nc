use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn ordered_atomic_program_executes_through_the_real_interpreter_bridge() {
    let output = Command::new(env!("CARGO_BIN_EXE_ncc"))
        .current_dir(workspace_root())
        .args([
            "run",
            "examples/06_memory/c11_atomic_orders.n",
            "--interpreted",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc run");
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{rendered}");
    assert!(rendered.contains("C11 atomic value=9"), "{rendered}");
}
