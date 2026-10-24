use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
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

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
}

fn example(rel: &str) -> String {
    let root = workspace_root();
    let direct = root.join(rel);
    if direct.exists() {
        return direct.to_string_lossy().to_string();
    }

    let mapped = match rel {
        "examples/swapNumerosCompacto.n" => Some("examples/01_basics/swap_nums_compact.n"),
        "examples/postIncrementCompact.n" => Some("examples/01_basics/post_increment_compact.n"),
        "examples/postIncrementCForms.n" => Some("examples/01_basics/post_increment_cforms.n"),
        "examples/compactSyntaxOficial.n" => Some("examples/01_basics/compact_syntax_std.n"),
        "examples/compoundOperatorsOficial.n" => {
            Some("examples/01_basics/compound_operators_std.n")
        }
        _ => None,
    };
    if let Some(mapped) = mapped {
        let path = root.join(mapped);
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    if let Some(legacy) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(legacy);
        if archived.exists() {
            return archived.to_string_lossy().to_string();
        }
    }

    direct.to_string_lossy().to_string()
}

#[test]
fn compact_swap_example_checks_and_runs() {
    let path = example("examples/swapNumerosCompacto.n");
    assert_success(run_ncc(&["check", &path]), "compact swap check");

    let output = assert_success(run_ncc(&["run", &path]), "compact swap run");
    assert!(output.contains("--Before swap--"), "{output}");
    assert!(output.contains("--After swap--"), "{output}");
    assert!(output.contains("First number  = 10"), "{output}");
    assert!(output.contains("Second number = 5"), "{output}");
    assert!(output.contains("Swap correcto = true"), "{output}");
}

#[test]
fn compact_post_increment_example_checks_and_runs() {
    let path = example("examples/postIncrementCompact.n");
    assert_success(run_ncc(&["check", &path]), "compact post increment check");

    let output = assert_success(run_ncc(&["run", &path]), "compact post increment run");
    assert!(output.contains("i=2"), "{output}");
    assert!(output.contains("ok=true"), "{output}");

    let backend_path = example("examples/backend/mct_llvm_post_increment.n");
    assert_success(
        run_ncc(&["check", &backend_path]),
        "compact post increment backend fixture check",
    );
    let out = std::env::temp_dir().join(format!(
        "ncc_post_increment_{}_compact_surface.ll",
        std::process::id()
    ));
    let out_path = out.to_string_lossy().to_string();
    let mct = assert_success(
        run_ncc(&[
            "mct",
            "emit-llvm",
            &backend_path,
            "--json",
            "--out",
            &out_path,
        ]),
        "compact post increment mct emit-llvm",
    );
    assert!(mct.contains("expr.post_increment"), "{mct}");
    assert!(mct.contains("load/add/store"), "{mct}");

    let llvm = std::fs::read_to_string(&out).expect("llvm output must be readable");
    assert!(llvm.contains("add i32"), "{llvm}");
    assert!(llvm.contains("store i32"), "{llvm}");
}

#[test]
fn compact_c_like_post_increment_forms_check_run_and_emit_mct() {
    let path = example("examples/postIncrementCForms.n");
    assert_success(
        run_ncc(&["check", &path]),
        "c-like post increment forms check",
    );

    let output = assert_success(run_ncc(&["run", &path]), "c-like post increment forms run");
    assert!(output.contains("fieldOld=4"), "{output}");
    assert!(output.contains("fieldNow=5"), "{output}");
    assert!(output.contains("ptrOld=41"), "{output}");
    assert!(output.contains("nextValue=99"), "{output}");
    assert!(output.contains("ok=true"), "{output}");

    let backend_path = example("examples/backend/mct_llvm_post_increment_c_forms.n");
    assert_success(
        run_ncc(&["check", &backend_path]),
        "c-like post increment mct fixture check",
    );
    let out = std::env::temp_dir().join(format!(
        "ncc_post_increment_c_forms_{}_compact_surface.ll",
        std::process::id()
    ));
    let out_path = out.to_string_lossy().to_string();
    let mct = assert_success(
        run_ncc(&[
            "mct",
            "emit-llvm",
            &backend_path,
            "--json",
            "--out",
            &out_path,
        ]),
        "c-like post increment mct emit-llvm",
    );
    assert!(mct.contains("expr.field_post_increment"), "{mct}");
    assert!(mct.contains("expr.pointer_post_increment_read"), "{mct}");

    let llvm = std::fs::read_to_string(&out).expect("llvm output must be readable");
    assert!(llvm.contains("getelementptr i8"), "{llvm}");
    assert!(llvm.contains("store ptr"), "{llvm}");
    assert!(llvm.contains("load i32"), "{llvm}");
}

#[test]
fn compact_syntax_official_normal_n_file_checks_and_runs() {
    let path = example("examples/compactSyntaxOficial.n");
    assert_success(run_ncc(&["check", &path]), "official compact syntax check");

    let output = assert_success(run_ncc(&["run", &path]), "official compact syntax run");
    assert!(output.contains("primero = 20"), "{output}");
    assert!(output.contains("segundo = 10"), "{output}");
    assert!(output.contains("ok = true"), "{output}");
}

#[test]
fn compound_operators_official_check_run_and_emit_mct() {
    let path = example("examples/compoundOperatorsOficial.n");
    assert_success(run_ncc(&["check", &path]), "compound operators check");

    let output = assert_success(run_ncc(&["run", &path]), "compound operators run");
    assert!(output.contains("i = 4"), "{output}");
    assert!(output.contains("x = 6"), "{output}");
    assert!(output.contains("campo = 5"), "{output}");
    assert!(output.contains("primero = 100"), "{output}");
    assert!(output.contains("segundo = 200"), "{output}");
    assert!(output.contains("distancia = 1"), "{output}");
    assert!(output.contains("checksum = 316"), "{output}");
    assert!(output.contains("ok = true"), "{output}");

    let backend_path = example("examples/backend/mct_llvm_compound_operators.n");
    assert_success(
        run_ncc(&["check", &backend_path]),
        "compound operators backend fixture check",
    );
    let out = std::env::temp_dir().join(format!(
        "ncc_compound_operators_{}_compact_surface.ll",
        std::process::id()
    ));
    let out_path = out.to_string_lossy().to_string();
    let mct = assert_success(
        run_ncc(&[
            "mct",
            "emit-llvm",
            &backend_path,
            "--json",
            "--out",
            &out_path,
        ]),
        "compound operators mct emit-llvm",
    );
    assert!(mct.contains("expr.post_increment"), "{mct}");

    let llvm = std::fs::read_to_string(&out).expect("llvm output must be readable");
    assert!(llvm.contains("add i32"), "{llvm}");
    assert!(llvm.contains("sub i32"), "{llvm}");
    assert!(llvm.contains("mul i32"), "{llvm}");
}
