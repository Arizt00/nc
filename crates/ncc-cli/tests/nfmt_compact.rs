use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
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

fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc_nfmt_compact_{}_{}_{}.n",
        label,
        std::process::id(),
        nanos
    ))
}

#[test]
fn nfmt_help_advertises_compact_mode() {
    let rendered = assert_success(run_ncc(&["help", "nfmt"]), "help nfmt");
    assert!(rendered.contains("--compact"), "{rendered}");
    assert!(
        rendered.contains("Compact syntax is the default"),
        "{rendered}"
    );
    assert!(rendered.contains("examples/foo.n --compact"), "{rendered}");
}

#[test]
fn nfmt_uses_compact_as_the_default_output_form() {
    let source = temp_path("default_source");
    let output = temp_path("default_compact");
    fs::write(
        &source,
        r#"defaultCompact {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : 7;
                return @value == 7;
        }
}
"#,
    )
    .expect("write default compact source");

    let source_arg = source.to_string_lossy().to_string();
    let output_arg = output.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--out", &output_arg]),
        "nfmt compact by default",
    );
    let compact = fs::read_to_string(&output).expect("default compact output");
    assert!(compact.contains("value >> int;"), "{compact}");
    assert!(compact.contains("value >> int <- 7;"), "{compact}");
    assert!(!compact.contains("@value"), "{compact}");

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(output);
}

#[test]
fn nfmt_compact_migrates_verbose_runtime_code_and_keeps_it_checkable() {
    let source = temp_path("source");
    let compact = temp_path("compact");
    fs::write(
        &source,
        r#"use::stoud::io::<output, render>;

swapVerbose[@runtime] {
    stc:
        first: int;
        second: int;
        ok: bool;

    fn:
        show(titulo: str) -> bool {
            public:
                println(colorBold(@titulo, "cyan"));
                println("First number  = " + renderInt(@first));
                println("Second number = " + renderInt(@second));
                return true;
        }

        run() -> bool {
            public:
                @first >> int : 5;
                @second >> int : 10;
                @ok >> bool : false;
                show("Before");
                swap(@first, @second);
                @ok >> bool : @first == 10 && @second == 5;
                println("Swap correcto = " + renderBool(@ok));
                return @ok;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    )
    .expect("write source");

    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--compact", "--out", &compact_arg]),
        "nfmt compact",
    );

    let compact_text = fs::read_to_string(&compact).expect("compact output");
    assert!(compact_text.contains("    fn:"), "{compact_text}");
    assert!(
        compact_text.contains("        show(titulo >> str) -> bool {"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("        run() -> bool {"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("    ref main ->> run;"),
        "{compact_text}"
    );
    assert!(!compact_text.contains(":: in"), "{compact_text}");
    assert!(
        compact_text.contains("first >> int <- 5;"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("swap(first, second);"),
        "{compact_text}"
    );
    assert!(compact_text.contains("return ok;"), "{compact_text}");
    assert!(
        !compact_text.contains("@first") && !compact_text.contains("@ok"),
        "{compact_text}"
    );

    assert_success(run_ncc(&["check", &compact_arg]), "check compact output");
    assert_success(
        run_ncc(&["emit-ir", &compact_arg]),
        "emit-ir compact output",
    );

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}

#[test]
fn nfmt_compact_preserves_kernel_at_mmio_and_direct_main() {
    let source = temp_path("kernel_source");
    let compact = temp_path("kernel_compact");
    fs::write(
        &source,
        r#"kernelFormatter {
    fn:
        run() -> bool {
            public:
                lapic at 0xFEE00000;
                value >> int : load32(lapic);
                *32(lapic, value);
                return true;
        }

    ref main ->> @fn(run);
}

"#,
    )
    .expect("write compact kernel source");

    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--compact", "--out", &compact_arg]),
        "nfmt compact kernel",
    );

    let compact_text = fs::read_to_string(&compact).expect("compact kernel output");
    for spelling in [
        "fn:",
        "run() -> bool {",
        "lapic at 0xFEE00000;",
        "value >> int <- load32(lapic);",
        "*32(lapic, value);",
        "ref main ->> run;",
    ] {
        assert!(
            compact_text.contains(spelling),
            "missing {spelling}:\n{compact_text}"
        );
    }
    assert!(!compact_text.contains(":: in"), "{compact_text}");
    assert!(!compact_text.contains("@fn"), "{compact_text}");
    assert_success(
        run_ncc(&["check", &compact_arg]),
        "check compact kernel output",
    );
    assert_success(
        run_ncc(&["emit-ir", &compact_arg]),
        "emit-ir compact kernel output",
    );

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}

#[test]
fn nfmt_compact_migrates_multideclarations_and_compat_calls() {
    let source = temp_path("legacy_multidecl");
    let compact = temp_path("legacy_multidecl_compact");
    fs::write(
        &source,
        r#"legacyMigration {
    stc:
        first: int; second: int; result: int;

    fn:
        add(left: int, right: int) -> int {
            return @left + @right;
        }

        run() -> bool {
            @first >> int : 2;
            @second >> int : 3;
            @result >> int : @fn(add, @first, @second);
            return @result == 5;
        }

    ref main ->> @fn(run);
}
"#,
    )
    .expect("write legacy migration source");

    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--out", &compact_arg]),
        "nfmt legacy multideclaration and call",
    );
    let compact_text = fs::read_to_string(&compact).expect("compact migration output");
    for spelling in [
        "first >> int;",
        "second >> int;",
        "result >> int;",
        "result >> int <- add(first, second);",
        "ref main ->> run;",
    ] {
        assert!(
            compact_text.contains(spelling),
            "missing {spelling}:\n{compact_text}"
        );
    }
    assert_success(run_ncc(&["check", &compact_arg]), "check migrated source");

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}

#[test]
fn nfmt_compact_completes_legacy_projection_failure_chain() {
    let source = temp_path("legacy_projection_chain");
    let compact = temp_path("legacy_projection_chain_compact");
    fs::write(
        &source,
        r#"legacyProjection {
    fn:
        run() -> bool {
            value >> int : 1
            value !> int : 1;
            return value == 1;
        }
}

"#,
    )
    .expect("write legacy projection chain");

    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--out", &compact_arg]),
        "nfmt legacy projection chain",
    );
    let compact_text = fs::read_to_string(&compact).expect("projection chain output");
    assert!(
        compact_text.contains("value >> int <- 1;"),
        "{compact_text}"
    );
    assert!(compact_text.contains("value !> int : 1;"), "{compact_text}");

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}

#[test]
fn nfmt_compact_accepts_modal_angle_imports_and_formats_negative_fixtures() {
    let source = temp_path("modal_import");
    let compact = temp_path("modal_import_compact");
    fs::write(
        &source,
        r#"use ai.agent { core };
use ai.model { core };

modalImports :: ai {
    stc:
        sessionActive >> bool <- false;
        modelReady >> bool <- false;
        plannerReady >> bool <- false;
        criticReady >> bool <- false;
        executorReady >> bool <- false;
        memoryCount >> int <- 0;
        lastProvider >> str <- "none";
        lastModel >> str <- "none";
        lastGoal >> str <- "none";
        lastPrompt >> str <- "none";
        lastResponse >> str <- "none";

    boot :: in() -> bool {
        return true;
    }

    ref main ->> boot;

    fa:
        return true;
}
"#,
    )
    .expect("write modal import source");
    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--out", &compact_arg]),
        "nfmt modal imports",
    );
    let compact_text = fs::read_to_string(&compact).expect("modal compact output");
    assert!(
        compact_text.contains("use <ai.agent:core>;"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("use <ai.model:core>;"),
        "{compact_text}"
    );
    assert_success(run_ncc(&["check", &compact_arg]), "check modal imports");

    fs::write(
        &source,
        "negative :: ai {\n    invalidSection:\n        value;\n}\n",
    )
    .expect("write negative fixture");
    assert_success(
        run_ncc(&[
            "nfmt",
            &source_arg,
            "--allow-invalid",
            "--out",
            &compact_arg,
        ]),
        "format negative fixture",
    );

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}

#[test]
fn nfmt_compact_orders_imports_aligns_sections_and_preserves_comments() {
    let source = temp_path("imports");
    let compact = temp_path("imports_compact");
    fs::write(
        &source,
        r#"/* Header comment with @rawSymbol and use nC.runtime.output; */
use nC.runtime.render;
use::stoud::memory::<heap>;
use nC.runtime.output;
use::stoud::memory::<ptr>;
use nC.runtime.input;
// collection imports stay below this comment
use::stoud::collections::<queue>;
use::stoud::collections::<list>;

formatDemo[@runtime] {
    type:
        Packet {
            public:
                id: int;
                total: int;
        }

    stc:
        second: int;
        first: int;
        ok: bool;

    local:
        scratch: int;

    const:
        limit: int = 2;

    fn:
        run() -> bool {
            public:
                @first >> int : 1;
                @second >> int : 2;
                @scratch >> int : @first + @second;
                @ok >> bool : @scratch == 3;
                println("ok=" + renderBool(@ok)); // keep @ok inside comment
                return @ok;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    )
    .expect("write source");

    let source_arg = source.to_string_lossy().to_string();
    let compact_arg = compact.to_string_lossy().to_string();
    assert_success(
        run_ncc(&["nfmt", &source_arg, "--compact", "--out", &compact_arg]),
        "nfmt compact imports",
    );

    let compact_text = fs::read_to_string(&compact).expect("compact output");
    assert!(
        compact_text.contains("/* Header comment with @rawSymbol and use nC.runtime.output; */"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("use <io:output, render, input>;\nuse <memory:ptr, heap>;"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains(
            "// collection imports stay below this comment\nuse <collections:list, queue>;"
        ),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("    type:\n        Packet {"),
        "{compact_text}"
    );
    assert!(compact_text.contains("id >> int;"), "{compact_text}");
    assert!(compact_text.contains("total >> int;"), "{compact_text}");
    assert!(
        compact_text.contains("limit >> int <- 2;"),
        "{compact_text}"
    );
    assert!(
        compact_text.contains("println(\"ok=\" + renderBool(ok)); // keep @ok inside comment"),
        "{compact_text}"
    );
    assert_success(
        run_ncc(&["check", &compact_arg]),
        "check compact imports output",
    );
    assert_success(
        run_ncc(&["emit-ir", &compact_arg]),
        "emit-ir compact imports output",
    );

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(compact);
}
