#![cfg(all(any(unix, windows), target_arch = "x86_64"))]

use std::io::Write;
use std::path::{Path, PathBuf};
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
        .expect("workspace root must resolve")
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("binary parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn native_toolchain_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn powershell_available() -> bool {
    Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", "exit 0"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_native(path: &str, input: Option<&str>) -> Output {
    let ncc = ncc_bin();
    let mut child = Command::new(&ncc)
        .current_dir(workspace_root())
        .args(["run", path])
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc run must start");

    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .expect("native stdin")
            .write_all(input.as_bytes())
            .expect("native stdin write");
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("ncc run must finish")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_native_ok(path: &str, input: Option<&str>, marker: Option<&str>) {
    let output = run_native(path, input);
    let text = rendered(&output);
    assert!(output.status.success(), "{path}\n{text}");
    assert!(!text.contains("0xc0000005"), "{path}\n{text}");
    assert!(!text.contains("E_MCT_LLVM_"), "{path}\n{text}");
    assert!(!text.contains("Invalid string escape"), "{path}\n{text}");
    if let Some(marker) = marker {
        assert!(text.contains(marker), "{path}\n{text}");
    }
}

#[test]
fn reported_memory_and_atomic_programs_execute_through_native_mct() {
    if !native_toolchain_available() {
        eprintln!("skipping native MCT regressions: clang is unavailable");
        return;
    }

    let cases = [
        (
            "_archive/old_examples/generated/drivers/industrial_driver/industrial_driver.n",
            None,
            Some("driver:checksum=16"),
        ),
        ("examples/06_memory/memoryMapComplete.n", None, None),
        (
            "examples/06_memory/gestor_alumnos_ptr.n",
            Some("7\n"),
            Some("Saliendo del gestor."),
        ),
        (
            "examples/06_memory/compact_driver_mem.n",
            None,
            Some("ok = true"),
        ),
        (
            "examples/06_memory/c11_atomic_orders.n",
            None,
            Some("C11 atomic value=9"),
        ),
        (
            "examples/06_memory/alpha_beta_ptr_bad.n",
            None,
            Some("corrupcion controlada = true"),
        ),
    ];

    for (path, input, marker) in cases {
        assert_native_ok(path, input, marker);
    }
}

#[test]
fn terminal_and_timer_calls_are_materialized_by_the_native_backend() {
    if !native_toolchain_available() {
        eprintln!("skipping native terminal regression: clang is unavailable");
        return;
    }

    assert_native_ok(
        "examples/07_runtime/terminal_timer_runtime.n",
        None,
        Some("terminal/timer runtime"),
    );
}

#[test]
fn powershell_command_preserves_script_arguments_in_native_execution() {
    if !native_toolchain_available() || !powershell_available() {
        eprintln!("skipping native PowerShell regression: required host tools are unavailable");
        return;
    }

    assert_native_ok(
        "examples/07_runtime/runtime_real_process_powershell_args.n",
        None,
        Some("ps-args-ok = true"),
    );
}
