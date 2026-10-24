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

fn fixture(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    relative
        .strip_prefix("examples/")
        .map(|rest| root.join("_archive/old_examples").join(rest))
        .filter(|path| path.exists())
        .unwrap_or(direct)
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc invocation")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_fixture_command_ok(command: &str, relative: &str, extra: &[&str]) -> String {
    let path = fixture(relative);
    let mut args = vec![command, path.to_str().expect("utf8 fixture path")];
    args.extend_from_slice(extra);
    let output = run_ncc(&args);
    let text = output_text(&output);
    assert!(
        output.status.success(),
        "{command} {relative} failed:\n{text}"
    );
    text
}

fn temp_source(name: &str, source: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ncc_{name}_{}_{}.n", std::process::id(), nonce));
    std::fs::write(&path, source).expect("write temporary source");
    path
}

fn condition_source(name: &str, statement: &str) -> String {
    format!(
        r#"
{name}[@runtime] {{
    stc:
        text: str;
        i: int;
        items: array<int, 1>;

    fn:
        run() -> bool {{
            public:
                @text >> str : "active";
                @i >> int : 0;
                @items >> array<int, 1> : [1];
                {statement}
                return true;
        }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#
    )
}

#[test]
fn raw_pointer_out_of_range_and_bounded_region_contracts_are_distinct() {
    let raw = "examples/06_memory/rawPointerOutOfBounds.n";
    assert_fixture_command_ok("check", raw, &[]);
    let ast = assert_fixture_command_ok("emit-ast", raw, &[]);
    assert!(ast.contains("ptrOffset") && ast.contains("ptrWrite"));
    let ir = assert_fixture_command_ok("emit-ir", raw, &[]);
    assert!(ir.contains("ptrOffset") && ir.contains("ptrWrite"));
    let run = assert_fixture_command_ok("run", raw, &["--show-return"]);
    assert!(run.contains("return: true"), "unexpected run output: {run}");

    let bounded = fixture("examples/pointers/negative/ptr_region_oob_fail.n");
    let output = run_ncc(&["run", bounded.to_str().expect("utf8 fixture path")]);
    let text = output_text(&output);
    assert!(
        text.contains("E_REGION_BOUNDS_001"),
        "bounded memRegion must retain its bounds contract:\n{text}"
    );
}

#[test]
fn every_control_condition_requires_bool() {
    let cases = [
        ("if", "if (@text) { return false; }"),
        ("while", "while (@text) { break; }"),
        ("do_while", "do { @i = @i + 1; } while (@text);"),
        ("for_classic", "for (@i = 0; @text; @i = @i + 1) { break; }"),
        ("repeat_until", "repeat { @i = @i + 1; } until (@text);"),
        ("guard", "guard (@text) else { return false; }"),
        ("unless", "unless (@text) { return false; }"),
        ("logical_not", "if (!@text) { return false; }"),
        ("logical_and", "if (@text && true) { return false; }"),
        ("logical_or", "if (false || @text) { return false; }"),
        (
            "each_do_while",
            "each item in @items do { @i = @item; } while (@text);",
        ),
        (
            "select_when",
            "select { when (@text) { return false; } default { return true; } }",
        ),
        ("ternary", "@i >> int : @text ? 1 : 0;"),
        ("if_expression", "@i >> int : if (@text) { 1 } else { 0 };"),
    ];

    for (name, statement) in cases {
        let source = condition_source(&format!("strictCondition{name}"), statement);
        let path = temp_source(name, &source);
        let output = run_ncc(&["check", path.to_str().expect("utf8 temp path")]);
        let text = output_text(&output);
        let _ = std::fs::remove_file(path);
        assert!(
            !output.status.success(),
            "{name} unexpectedly passed:\n{text}"
        );
        assert!(
            text.contains("E_CONDITION_BOOL_REQUIRED_001") && text.contains("must be bool"),
            "{name} did not report the strict bool contract:\n{text}"
        );
    }
}

#[test]
fn pattern_match_guards_require_bool() {
    let source = condition_source(
        "strictPatternGuard",
        "match (@i) { value if @text => return false; _ => return true; }",
    );
    let path = temp_source("pattern_guard", &source);
    let output = run_ncc(&["check", path.to_str().expect("utf8 temp path")]);
    let text = output_text(&output);
    let _ = std::fs::remove_file(path);
    assert!(!output.status.success(), "pattern guard passed:\n{text}");
    assert!(
        text.contains("E_HIGH_GUARD_TYPE_001") && text.contains("must be bool"),
        "unexpected pattern guard diagnostic:\n{text}"
    );
}

#[test]
fn string_operands_are_allowed_when_the_expression_returns_bool() {
    let source = r#"
strictStringComparison[@runtime] {
    fn:
        run() -> bool {
            public:
                if ("alpha" < "beta" && "beta" > "alpha") {
                    while ("same" <= "same" && "same" >= "same") {
                        guard ("left" != "right" && "left" == "left") else { return false; }
                        return true;
                    }
                }
                return false;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
    let path = temp_source("string_comparison", source);
    for command in ["check", "emit-ast", "emit-ir"] {
        let output = run_ncc(&[command, path.to_str().expect("utf8 temp path")]);
        assert!(
            output.status.success(),
            "{command} failed:\n{}",
            output_text(&output)
        );
    }
    let output = run_ncc(&[
        "run",
        path.to_str().expect("utf8 temp path"),
        "--show-return",
    ]);
    let text = output_text(&output);
    let _ = std::fs::remove_file(path);
    assert!(
        output.status.success() && text.contains("return: true"),
        "{text}"
    );
}

#[test]
fn option_result_inference_propagates_through_functions_collections_and_payloads() {
    let relative = "examples/03_types/optionResultInference.n";
    assert_fixture_command_ok("check", relative, &[]);
    let ast = assert_fixture_command_ok("emit-ast", relative, &[]);
    assert!(
        ast.contains("Option")
            && ast.contains("Result")
            && ast.contains("QualifiedCall")
            && ast.contains("nested")
    );
    let ir = assert_fixture_command_ok("emit-ir", relative, &[]);
    for expected in ["Option<int>", "Result<int, str>", "AggregateByRef"] {
        assert!(ir.contains(expected), "IR missing {expected}");
    }
    let run = assert_fixture_command_ok("run", relative, &["--show-return"]);
    assert!(run.contains("return: true"), "unexpected run output: {run}");
}

#[test]
fn option_result_payload_mismatches_remain_rejected() {
    let source = r#"
optionResultMismatch {
    fn:
        badOption() -> Option<int> {
            public:
                return Some("wrong");
        }

        badResult() -> Result<int, str> {
            public:
                return Err(7);
        }

    fa:
        return true;
}
"#;
    let path = temp_source("option_result_mismatch", source);
    let output = run_ncc(&["check", path.to_str().expect("utf8 temp path")]);
    let text = output_text(&output);
    let _ = std::fs::remove_file(path);
    assert!(
        !output.status.success(),
        "mismatch unexpectedly passed:\n{text}"
    );
    assert!(
        text.contains("E_HIGH_OPTION_SOME_TYPE_001") || text.contains("E_HIGH_RESULT_ERR_TYPE_001"),
        "unexpected mismatch diagnostic:\n{text}"
    );
}
