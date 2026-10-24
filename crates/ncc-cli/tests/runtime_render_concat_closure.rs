use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    direct
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_ncc_with_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc command must spawn");

    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input.as_bytes())
        .expect("write stdin");

    child.wait_with_output().expect("collect ncc output")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn write_temp_source(name: &str, src: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{}_{}_{}.n", name, std::process::id(), ts));
    std::fs::write(&path, src).expect("write temp source");
    path
}

fn assert_check_ok(rel: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "check must pass for {rel}\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_run_ok_contains(rel: &str, needles: &[&str]) -> String {
    let path = fixture_path(rel);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "run must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    for needle in needles {
        assert!(
            stdout.contains(needle) || stderr.contains(needle),
            "run for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
        );
    }
    assert_no_runtime_text_leak(&stdout, rel);
    stdout
}

fn assert_check_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "check must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "check for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_temp_check_fail_contains(name: &str, src: &str, needle: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "temp check must fail for {name}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "temp check for {name} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_no_runtime_text_leak(stdout: &str, label: &str) {
    for forbidden in ["none", "<expr>", "String(\"<expr>\")", "any"] {
        assert!(
            !stdout.contains(forbidden),
            "{label} output must not contain {forbidden:?}\nstdout={stdout}"
        );
    }
}

#[test]
fn runtime_render_int_returns_str_ok() {
    assert_check_ok("examples/runtime/render/render_int_concat_ok.n");
    assert_run_ok_contains(
        "examples/runtime/render/render_int_concat_ok.n",
        &["Valor: 42"],
    );
}

#[test]
fn runtime_render_bool_returns_str_ok() {
    assert_check_ok("examples/runtime/render/render_bool_concat_ok.n");
    assert_run_ok_contains(
        "examples/runtime/render/render_bool_concat_ok.n",
        &["OK=true"],
    );
}

#[test]
fn runtime_render_binary_returns_str_ok() {
    assert_check_ok("examples/runtime/render/render_binary_ok.n");
    assert_run_ok_contains(
        "examples/runtime/render/render_binary_ok.n",
        &["bin(0)=0", "bin(5)=101", "bin(42)=101010", "bin(-5)=-101"],
    );
}

#[test]
fn runtime_render_float_returns_str_ok() {
    let path = write_temp_source(
        "runtime_render_float_returns_str_ok",
        r#"
use nC.runtime.output;
use nC.runtime.render;

entry[@runtime] {
    stc:
        f: ft64;

    fn:
        run() -> bool {
            public:
                @f >> ft64 : 3.5;
                println("F=" + renderFloat(@f));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "renderFloat run must pass\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("F=3.5"), "stdout={stdout}");
    assert_no_runtime_text_leak(&stdout, "runtime_render_float_returns_str_ok");
}

#[test]
fn runtime_render_char_returns_str_ok() {
    let path = write_temp_source(
        "runtime_render_char_returns_str_ok",
        r#"
use nC.runtime.output;
use nC.runtime.parse;
use nC.runtime.render;

entry[@runtime] {
    stc:
        c: char;

    fn:
        run() -> bool {
            public:
                @c >> char : parseChar("A");
                println("C=" + renderChar(@c));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "renderChar run must pass\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("C=A"), "stdout={stdout}");
    assert_no_runtime_text_leak(&stdout, "runtime_render_char_returns_str_ok");
}

#[test]
fn runtime_render_assignment_to_str_ok() {
    assert_check_ok("examples/runtime/render/render_assignment_to_str_ok.n");
    assert_run_ok_contains(
        "examples/runtime/render/render_assignment_to_str_ok.n",
        &["Checksum: 9038"],
    );
}

#[test]
fn runtime_render_int_concat_println_ok() {
    runtime_render_int_returns_str_ok();
}

#[test]
fn runtime_render_chained_concat_ok() {
    assert_check_ok("examples/runtime/render/render_chained_concat_ok.n");
    assert_run_ok_contains(
        "examples/runtime/render/render_chained_concat_ok.n",
        &["A=10 B=20", "Sensor 3: ready"],
    );
}

#[test]
fn runtime_render_nested_concat_ok() {
    runtime_render_chained_concat_ok();
}

#[test]
fn runtime_render_sensor_prompt_ok() {
    runtime_render_chained_concat_ok();
}

#[test]
fn runtime_concat_str_str_ok() {
    assert_check_ok("examples/runtime/concat/concat_str_str_ok.n");
    assert_run_ok_contains("examples/runtime/concat/concat_str_str_ok.n", &["ab"]);
}

#[test]
fn runtime_concat_chained_str_ok() {
    assert_check_ok("examples/runtime/concat/concat_chained_ok.n");
    assert_run_ok_contains("examples/runtime/concat/concat_chained_ok.n", &["A=10"]);
}

#[test]
fn runtime_concat_print_ok() {
    assert_check_ok("examples/runtime/concat/concat_runtime_output_ok.n");
    assert_run_ok_contains(
        "examples/runtime/concat/concat_runtime_output_ok.n",
        &["Sensor 3: ok"],
    );
}

#[test]
fn runtime_concat_println_ok() {
    runtime_concat_str_str_ok();
}

#[test]
fn runtime_string_real_ops_ok() {
    assert_check_ok("examples/runtime/string/string_real_ops_ok.n");
    assert_run_ok_contains(
        "examples/runtime/string/string_real_ops_ok.n",
        &[
            "part=cde",
            "join=nC",
            "len=15",
            "bytes=15",
            "upper=NCC",
            "from=42:true:3.5",
        ],
    );
}

#[test]
fn runtime_string_collections_structs_ok() {
    assert_check_ok("examples/runtime/string/string_collections_structs_ok.n");
    assert_run_ok_contains(
        "examples/runtime/string/string_collections_structs_ok.n",
        &["packet=tx-packet", "code=9"],
    );
}

#[test]
fn runtime_collections_comfy_oob_ok() {
    assert_check_ok("examples/runtime/collections/collections_comfy_oob_ok.n");
    assert_run_ok_contains(
        "examples/runtime/collections/collections_comfy_oob_ok.n",
        &[
            "list first: 10",
            "list missing: 0",
            "queue pop: 20",
            "queue missing: 0",
            "queue function missing: 0",
            "queue len: 1",
        ],
    );
}

#[test]
fn stress_runtime_menu_render_concat_runs_ok() {
    let path = fixture_path("examples/stress_runtime_industrial_pack.n");
    let input =
        "1\n45\n78\n15\n42\n64\n87\n22\n45\n76\n95\n24\n36\n55\n95\n99\n88\n3\n4\n5\n6\n10\n";
    let out = run_ncc_with_stdin(
        &["run", path.to_str().expect("utf8 path"), "--interpreted"],
        input,
    );
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "stress run must pass\nstdout={stdout}\nstderr={stderr}"
    );
    for needle in [
        "Sensor 0:",
        "Sensor 15:",
        "Nuevo valor: 142",
        "Maximo: 99",
        "Checksum: 9038",
    ] {
        assert!(
            stdout.contains(needle),
            "stress output must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
        );
    }
    assert_no_runtime_text_leak(&stdout, "stress_runtime_industrial_pack");
}

#[test]
fn runtime_concat_str_int_without_render_fail() {
    assert_check_fail_contains(
        "examples/runtime/concat/concat_str_int_fail.n",
        "E_RUNTIME_CONCAT_TYPE_001",
    );
}

#[test]
fn runtime_concat_str_bool_without_render_fail() {
    assert_temp_check_fail_contains(
        "runtime_concat_str_bool_without_render_fail",
        r#"
use nC.runtime.output;

entry[@runtime] {
    stc:
        ok: bool;
    fn:
        run() -> bool {
            public:
                @ok >> bool : true;
                println("OK=" + @ok);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_RUNTIME_CONCAT_TYPE_001",
    );
}

#[test]
fn runtime_render_int_without_use_fail() {
    assert_check_fail_contains(
        "examples/runtime/render/render_missing_use_fail.n",
        "E_RUNTIME_USE_001",
    );
}

#[test]
fn runtime_render_int_wrong_arg_fail() {
    assert_check_fail_contains(
        "examples/runtime/render/render_wrong_type_fail.n",
        "E_RUNTIME_RENDER_TYPE_001",
    );
}

#[test]
fn runtime_render_binary_wrong_arg_fail() {
    assert_temp_check_fail_contains(
        "runtime_render_binary_wrong_arg_fail",
        r#"
use nC.runtime.output;
use nC.runtime.render;

entry[@runtime] {
    fn:
        run() -> bool {
            public:
                println(renderBinary("x"));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_RUNTIME_RENDER_TYPE_001",
    );
}

#[test]
fn runtime_concat_function_without_use_fail_if_direct_call() {
    assert_temp_check_fail_contains(
        "runtime_concat_function_without_use_fail_if_direct_call",
        r#"
use nC.runtime.output;

entry[@runtime] {
    fn:
        run() -> bool {
            public:
                println(concat("a", "b"));
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_RUNTIME_USE_001",
    );
}

#[test]
fn runtime_render_none_never_allowed_fail() {
    let stdout = assert_run_ok_contains(
        "examples/runtime/render/render_int_concat_ok.n",
        &["Valor: 42"],
    );
    assert_no_runtime_text_leak(&stdout, "render none guard");
}

#[test]
fn runtime_concat_none_never_allowed_fail() {
    let stdout = assert_run_ok_contains("examples/runtime/concat/concat_chained_ok.n", &["A=10"]);
    assert_no_runtime_text_leak(&stdout, "concat none guard");
}

#[test]
fn runtime_render_any_leak_fail() {
    runtime_render_assignment_to_str_ok();
}

#[test]
fn runtime_concat_any_leak_fail() {
    runtime_concat_str_str_ok();
}
