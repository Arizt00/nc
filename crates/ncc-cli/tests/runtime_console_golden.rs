use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn fixture_path(name: &str) -> PathBuf {
    let root = workspace_root();
    let current = root.join("examples").join("runtime_console_f7").join(name);
    if current.exists() {
        return current;
    }
    let archived = root
        .join("_archive")
        .join("old_examples")
        .join("runtime_console_f7")
        .join(name);
    if archived.exists() {
        return archived;
    }
    current
}

fn example_path(rel: &str) -> PathBuf {
    let root = workspace_root();
    let current = root.join(rel);
    if current.exists() {
        return current;
    }
    if let Some(rest) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    current
}

fn run_fixture(name: &str, nc_no_color: Option<&str>) -> Output {
    let mut cmd = Command::new(ncc_bin());
    cmd.arg("run").arg(fixture_path(name)).arg("--interpreted");
    if let Some(v) = nc_no_color {
        cmd.env("NC_NO_COLOR", v);
    }
    cmd.output().expect("ncc run must execute")
}

fn run_example_with_input(rel: &str, input: &str) -> Output {
    let mut child = Command::new(ncc_bin())
        .arg("run")
        .arg(example_path(rel))
        .arg("--interpreted")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc run must spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin must be piped")
        .write_all(input.as_bytes())
        .expect("test input must be written");
    child.wait_with_output().expect("ncc run must finish")
}

fn run_example_with_env(rel: &str, envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(ncc_bin());
    cmd.arg("run").arg(example_path(rel)).arg("--interpreted");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.output().expect("ncc run must execute")
}

struct CapturedRun {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

fn run_example_wait_prompt_then_input(rel: &str, prompt: &str, input: &str) -> CapturedRun {
    run_example_wait_prompt_then_input_with_args(
        rel,
        prompt,
        input,
        &["--interpreted"],
        Duration::from_secs(3),
    )
}

fn run_example_wait_prompt_then_input_with_args(
    rel: &str,
    prompt: &str,
    input: &str,
    args: &[&str],
    prompt_timeout: Duration,
) -> CapturedRun {
    let prompt_owned = prompt.to_string();
    let prompt_for_thread = prompt_owned.clone();
    let mut command = Command::new(ncc_bin());
    command.arg("run").arg(example_path(rel)).args(args);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc run must spawn");

    let mut stdout = child.stdout.take().expect("stdout must be piped");
    let mut stderr = child.stderr.take().expect("stderr must be piped");
    let (tx, rx) = mpsc::channel();

    let stdout_handle = thread::spawn(move || {
        let mut buf = [0u8; 1];
        let mut all = Vec::new();
        let mut notified = false;
        loop {
            match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    all.extend_from_slice(&buf[..n]);
                    if !notified && String::from_utf8_lossy(&all).contains(&prompt_for_thread) {
                        let _ = tx.send(());
                        notified = true;
                    }
                }
                Err(_) => break,
            }
        }
        all
    });
    let stderr_handle = thread::spawn(move || {
        let mut all = Vec::new();
        let _ = stderr.read_to_end(&mut all);
        all
    });

    if rx.recv_timeout(prompt_timeout).is_err() {
        let _ = child.kill();
        panic!(
            "prompt {:?} was not visible before input was written",
            prompt_owned
        );
    }
    child
        .stdin
        .as_mut()
        .expect("stdin must be piped")
        .write_all(input.as_bytes())
        .expect("test input must be written");
    drop(child.stdin.take());

    let status = child.wait().expect("ncc run must finish");
    let stdout = normalize_stream(&stdout_handle.join().expect("stdout thread must join"));
    let stderr = normalize_stream(&stderr_handle.join().expect("stderr thread must join"));
    CapturedRun {
        status,
        stdout,
        stderr,
    }
}

fn normalize_stream(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_no_ansi(s: &str) {
    assert!(
        !s.contains("\u{1b}["),
        "stream should not contain ANSI escapes, got: {:?}",
        s
    );
}

fn assert_clean_console_output(s: &str) {
    assert!(
        !s.contains("none"),
        "console output must not materialize none, got: {:?}",
        s
    );
    assert!(
        !s.contains("<expr>"),
        "console output must not materialize expression sentinels, got: {:?}",
        s
    );
    assert!(
        !s.contains("String(\"<expr>\")"),
        "console output must not materialize expression sentinels, got: {:?}",
        s
    );
    assert!(
        !s.contains("RuntimeValue::Any"),
        "console output must not leak Any, got: {:?}",
        s
    );
}

fn write_temp_source(name: &str, src: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must work")
        .as_nanos();
    let dir = workspace_root()
        .join("target")
        .join("tmp")
        .join("runtime_console_tests");
    std::fs::create_dir_all(&dir).expect("temp dir must be created");
    let path = dir.join(format!("{name}_{nonce}.n"));
    std::fs::write(&path, src).expect("temp source must be written");
    path
}

fn check_source(path: &PathBuf) -> Output {
    Command::new(ncc_bin())
        .arg("check")
        .arg(path)
        .output()
        .expect("ncc check must execute")
}

fn run_source(path: &PathBuf, input: &str) -> Output {
    let mut child = Command::new(ncc_bin())
        .arg("run")
        .arg(path)
        .arg("--interpreted")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc run must execute");
    child
        .stdin
        .as_mut()
        .expect("stdin must be piped")
        .write_all(input.as_bytes())
        .expect("test input must be written");
    child.wait_with_output().expect("ncc run must finish")
}

#[test]
fn f7_stdout_stderr_split_is_official_without_color() {
    let output = run_fixture("f7ConsoleSplit.n", Some("1"));
    assert!(
        output.status.success(),
        "run must succeed, status={:?}",
        output.status
    );

    let stdout = normalize_stream(&output.stdout);
    let stderr = normalize_stream(&output.stderr);

    assert_eq!(stdout, "A B\n");
    assert_eq!(stderr, "E1\nW1\n");
    assert_no_ansi(&stdout);
    assert_no_ansi(&stderr);
}

#[test]
fn f7_stderr_keeps_ansi_prefixes_when_color_enabled() {
    let output = run_fixture("f7ConsoleSplit.n", Some("0"));
    assert!(
        output.status.success(),
        "run must succeed, status={:?}",
        output.status
    );

    let stdout = normalize_stream(&output.stdout);
    let stderr = normalize_stream(&output.stderr);

    assert_eq!(stdout, "A B\n");
    assert!(stderr.contains("\u{1b}[31mE1\u{1b}[0m\n"));
    assert!(stderr.contains("\u{1b}[33mW1\u{1b}[0m\n"));
}

#[test]
fn f7_spacing_surface_output_is_exact() {
    let output = run_fixture("f7ConsoleSpacing.n", Some("1"));
    assert!(
        output.status.success(),
        "run must succeed, status={:?}",
        output.status
    );

    let stdout = normalize_stream(&output.stdout);
    let stderr = normalize_stream(&output.stderr);

    assert_eq!(stdout, "A B\nC\n\n");
    assert_eq!(stderr, "");
    assert_no_ansi(&stdout);
}

#[test]
fn f7_render_modes_output_contract_is_stable() {
    let output = run_fixture("f7ConsoleRenderModes.n", Some("1"));
    assert!(
        output.status.success(),
        "run must succeed, status={:?}",
        output.status
    );

    let stdout = normalize_stream(&output.stdout);
    let stderr = normalize_stream(&output.stderr);

    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "num(42)",
            "str(\"hola\")",
            "\"hola\"",
            "true",
            "[inspect] symbol=n value=num:42 type=int class=stack",
        ]
    );
    assert_eq!(stderr, "");
    assert_no_ansi(&stdout);
}

#[test]
fn runtime_console_rich_output_plain_mode_ok() {
    let output = run_example_with_env(
        "examples/runtime/console/rich_output_colors_emoji.n",
        &[("NC_NO_COLOR", "1")],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );

    let stdout = normalize_stream(&output.stdout);
    assert!(stdout.contains("estado=ok"), "stdout={stdout:?}");
    assert!(stdout.contains("alerta"), "stdout={stdout:?}");
    assert!(stdout.contains("cabecera"), "stdout={stdout:?}");
    assert!(stdout.contains("subrayado"), "stdout={stdout:?}");
    assert!(stdout.contains("emoji=\u{1f680}"), "stdout={stdout:?}");
    assert_no_ansi(&stdout);
    assert_clean_console_output(&stdout);
}

#[test]
fn runtime_console_rich_output_ansi_mode_ok() {
    let output = run_example_with_env(
        "examples/runtime/console/rich_output_colors_emoji.n",
        &[("NC_NO_COLOR", "0")],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );

    let stdout = normalize_stream(&output.stdout);
    assert!(
        stdout.contains("\u{1b}[38;2;0;128;0mestado=ok\u{1b}[0m"),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("\u{1b}[1;38;2;255;0;0malerta\u{1b}[0m"),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("\u{1b}[48;2;0;0;255mcabecera\u{1b}[0m"),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("\u{1b}[4msubrayado\u{1b}[0m"),
        "stdout={stdout:?}"
    );
    assert!(stdout.contains("emoji=\u{1f680}"), "stdout={stdout:?}");
}

#[test]
fn runtime_console_rich_output_table_animation_ok() {
    let output = run_example_with_env(
        "examples/runtime/console/rich_output_table_animation.n",
        &[("NC_NO_COLOR", "1")],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );

    let stdout = normalize_stream(&output.stdout);
    assert!(
        stdout.contains("compile [######--] 3/4"),
        "stdout={stdout:?}"
    );
    assert!(stdout.contains("build |"), "stdout={stdout:?}");
    assert!(stdout.contains("+------+---------+"), "stdout={stdout:?}");
    assert!(stdout.contains("| mode | runtime |"), "stdout={stdout:?}");
    assert!(
        stdout.contains("+-------+------+--------+"),
        "stdout={stdout:?}"
    );
    assert!(stdout.contains("| print | rich | ok"), "stdout={stdout:?}");
    assert_no_ansi(&stdout);
    assert_clean_console_output(&stdout);
}

#[test]
fn runtime_console_rich_output_unknown_color_fail() {
    let src = r#"
use nC.runtime.output;

rich_output_unknown_color[@runtime] {
    fn:
        run() -> bool {
            public:
                println(color("x", "ultraviolet"));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let path = write_temp_source("rich_output_unknown_color", src);
    let output = run_source(&path, "");
    assert!(!output.status.success(), "unknown color must fail");
    let combined = format!(
        "{}{}",
        normalize_stream(&output.stdout),
        normalize_stream(&output.stderr)
    );
    assert!(
        combined.contains("E_RUNTIME_RICH_OUTPUT_COLOR_001"),
        "combined={combined:?}"
    );
}

#[test]
fn runtime_console_print_flush_before_read_int_ok() {
    let output = run_example_wait_prompt_then_input(
        "examples/runtime/console/prompt_read_int_ok.n",
        "Valor: ",
        "42\n",
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(
        output.stdout.starts_with("Valor: "),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Leido: 42"),
        "stdout={:?}",
        output.stdout
    );
    assert_clean_console_output(&output.stdout);
}

#[test]
fn runtime_native_prompt_is_visible_before_read_int_unblocks() {
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping native prompt streaming test: clang is unavailable");
        return;
    }

    let output = run_example_wait_prompt_then_input_with_args(
        "examples/runtime/console/prompt_read_int_ok.n",
        "Valor: ",
        "42\n",
        &[],
        Duration::from_secs(30),
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(
        output.stdout.starts_with("Valor: "),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Leido: 42"),
        "stdout={:?}",
        output.stdout
    );
}

#[test]
fn runtime_native_prompt_streams_while_capturing_return() {
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping native return streaming test: clang is unavailable");
        return;
    }

    let output = run_example_wait_prompt_then_input_with_args(
        "examples/runtime/console/prompt_read_int_ok.n",
        "Valor: ",
        "7\n",
        &["--show-return"],
        Duration::from_secs(30),
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(
        output.stdout.contains("Leido: 7"),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("[return: true"),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        !output.stdout.contains("__NCC_NATIVE_RETURN__="),
        "the internal return marker leaked: {:?}",
        output.stdout
    );
}

#[cfg(windows)]
#[test]
fn runtime_native_child_terminates_when_ncc_parent_is_stopped() {
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping native process-tree test: clang is unavailable");
        return;
    }

    let mut child = Command::new(ncc_bin())
        .arg("run")
        .arg(example_path(
            "examples/runtime/console/prompt_read_int_ok.n",
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc run must spawn");
    let mut stdout = child.stdout.take().expect("stdout must be piped");
    let mut stderr = child.stderr.take().expect("stderr must be piped");
    let (prompt_tx, prompt_rx) = mpsc::channel();
    let (eof_tx, eof_rx) = mpsc::channel();

    let stdout_thread = thread::spawn(move || {
        let mut byte = [0_u8; 1];
        let mut all = Vec::new();
        let mut prompt_sent = false;
        loop {
            match stdout.read(&mut byte) {
                Ok(0) => break,
                Ok(read) => {
                    all.extend_from_slice(&byte[..read]);
                    if !prompt_sent && String::from_utf8_lossy(&all).contains("Valor: ") {
                        let _ = prompt_tx.send(());
                        prompt_sent = true;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = eof_tx.send(());
        all
    });
    let stderr_thread = thread::spawn(move || {
        let mut all = Vec::new();
        let _ = stderr.read_to_end(&mut all);
        all
    });

    if prompt_rx.recv_timeout(Duration::from_secs(30)).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("native prompt was not visible before stopping ncc");
    }
    child.kill().expect("ncc parent must stop");
    assert!(
        eof_rx.recv_timeout(Duration::from_secs(5)).is_ok(),
        "native child kept stdout open after its ncc parent was stopped"
    );

    drop(child.stdin.take());
    let _ = child.wait();
    let _ = stdout_thread.join().expect("stdout thread must join");
    let stderr = normalize_stream(&stderr_thread.join().expect("stderr thread must join"));
    assert!(
        !stderr.contains("process-tree lifetime setup failed"),
        "stderr={stderr}"
    );
}

#[test]
fn runtime_console_print_flush_before_read_line_ok() {
    let output = run_example_wait_prompt_then_input(
        "examples/runtime/console/prompt_read_line_ok.n",
        "Nombre: ",
        "Ada\n",
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(
        output.stdout.starts_with("Nombre: "),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Hola Ada"),
        "stdout={:?}",
        output.stdout
    );
    assert_clean_console_output(&output.stdout);
}

#[test]
fn runtime_console_menu_prompt_flush_ok() {
    let output = run_example_wait_prompt_then_input(
        "examples/runtime/console/menu_prompt_flush_ok.n",
        "> ",
        "1\n2\n",
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(output.stdout.contains("> "), "stdout={:?}", output.stdout);
    assert!(
        output.stdout.contains("Confirmar 1: "),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Seleccion: 1/2"),
        "stdout={:?}",
        output.stdout
    );
    assert_clean_console_output(&output.stdout);
}

#[test]
fn runtime_console_prompt_chained_concat_ok() {
    let output = run_example_wait_prompt_then_input(
        "examples/runtime/console/prompt_chained_concat_read_int_ok.n",
        "Slot 7 | Valor: ",
        "7\n",
    );
    assert!(output.status.success(), "stderr={}", output.stderr);
    assert!(
        output.stdout.contains("Slot 7 | Valor: "),
        "stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Slot 7 leyo 7"),
        "stdout={:?}",
        output.stdout
    );
    assert_clean_console_output(&output.stdout);
}

#[test]
fn runtime_console_render_int_prompt_ok() {
    let output = run_example_with_input(
        "examples/runtime/console/prompt_chained_concat_read_int_ok.n",
        "11\n",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    let stdout = normalize_stream(&output.stdout);
    assert!(stdout.contains("Slot 7 | Valor: "), "stdout={:?}", stdout);
    assert!(stdout.contains("Slot 7 leyo 11"), "stdout={:?}", stdout);
    assert_clean_console_output(&stdout);
}

#[test]
fn runtime_console_piped_input_preserves_prompt_order_ok() {
    let output = run_example_with_input(
        "examples/runtime/console/piped_input_prompt_order_ok.n",
        "10\n32\n",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    let stdout = normalize_stream(&output.stdout);
    let a = stdout.find("A: ").expect("A prompt must be present");
    let b = stdout.find("B: ").expect("B prompt must be present");
    let result = stdout
        .find("Resultado: 42")
        .expect("result output must be present");
    assert!(a < b && b < result, "stdout={:?}", stdout);
    assert_clean_console_output(&stdout);
}

#[test]
fn runtime_console_rectangulo_enterprise_prompt_order_ok() {
    let output = run_example_with_input(
        "examples/user_prompt_batch/rectanguloEnterpriseStress.n",
        "1\n25\n48\n74\n3\n5\n6\n10\n12\n",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    let stdout = normalize_stream(&output.stdout);
    let id = stdout
        .find("Slot 0 | ID del usuario: ")
        .expect("user id prompt must be present");
    let base = stdout
        .find("Base del rectangulo: ")
        .expect("base prompt must be present");
    let height = stdout
        .find("Altura del rectangulo: ")
        .expect("height prompt must be present");
    let created = stdout
        .find("Usuario creado correctamente.")
        .expect("created message must be present");
    assert!(
        id < base && base < height && height < created,
        "stdout={:?}",
        stdout
    );
    assert!(
        !stdout.contains("ID del usuario: Base del rectangulo: Altura del rectangulo:"),
        "prompts must not collapse after delayed input: {:?}",
        stdout
    );
    assert_clean_console_output(&stdout);
}

#[test]
fn runtime_console_output_no_none_ok() {
    let output = run_example_with_input("examples/runtime/console/prompt_read_int_ok.n", "9\n");
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    assert!(!normalize_stream(&output.stdout).contains("none"));
}

#[test]
fn runtime_console_output_no_expr_ok() {
    let output =
        run_example_with_input("examples/runtime/console/menu_prompt_flush_ok.n", "1\n2\n");
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    assert!(!normalize_stream(&output.stdout).contains("<expr>"));
}

#[test]
fn runtime_console_output_no_any_ok() {
    let output = run_example_with_input(
        "examples/runtime/console/piped_input_prompt_order_ok.n",
        "3\n4\n",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    assert!(!normalize_stream(&output.stdout).contains("RuntimeValue::Any"));
}

#[test]
fn runtime_console_read_int_invalid_input_fail() {
    let output = run_example_with_input("examples/runtime/console/read_int_parse_fail.n", "abc\n");
    assert!(
        !output.status.success(),
        "invalid input must fail, stdout={}",
        normalize_stream(&output.stdout)
    );
    let combined = format!(
        "{}{}",
        normalize_stream(&output.stdout),
        normalize_stream(&output.stderr)
    );
    assert!(
        combined.contains("E_RUNTIME_INPUT_PARSE_001"),
        "combined output={combined:?}"
    );
}

#[test]
fn runtime_console_input_uses_native_prelude_without_import() {
    let src = r#"
missing_input_use[@runtime] {
    stc:
        x: int;
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let path = write_temp_source("missing_input_use", src);
    let output = check_source(&path);
    assert!(
        output.status.success(),
        "readInt belongs to the native prelude: {}{}",
        normalize_stream(&output.stdout),
        normalize_stream(&output.stderr)
    );
}

#[test]
fn runtime_console_output_uses_native_prelude_without_import() {
    let src = r#"
missing_output_use[@runtime] {
    fn:
        run() -> bool {
            public:
                print("x");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let path = write_temp_source("missing_output_use", src);
    let output = run_source(&path, "");
    assert!(
        output.status.success(),
        "print belongs to the native prelude: {}{}",
        normalize_stream(&output.stdout),
        normalize_stream(&output.stderr)
    );
    assert!(
        normalize_stream(&output.stdout).contains('x'),
        "stdout={:?}",
        normalize_stream(&output.stdout)
    );
}

#[test]
fn runtime_console_flush_without_output_use_fail() {
    let src = r#"
missing_flush_use[@runtime] {
    fn:
        run() -> bool {
            public:
                flush();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let path = write_temp_source("missing_flush_use", src);
    let output = check_source(&path);
    assert!(
        !output.status.success(),
        "check must fail without output use"
    );
    let combined = format!(
        "{}{}",
        normalize_stream(&output.stdout),
        normalize_stream(&output.stderr)
    );
    assert!(
        combined.contains("E_RUNTIME_USE_001"),
        "combined={combined:?}"
    );
}

#[test]
fn runtime_console_prompt_order_regression_fail() {
    let output = run_example_with_input(
        "examples/user_prompt_batch/rectanguloEnterpriseStress.n",
        "1\n25\n48\n74\n10\n12\n",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        normalize_stream(&output.stderr)
    );
    let stdout = normalize_stream(&output.stdout);
    assert!(
        !stdout.contains("25\n48\n74\nSlot 0 | ID del usuario:"),
        "input must not appear before prompt in captured output: {:?}",
        stdout
    );
}
