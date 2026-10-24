#![cfg(unix)]

use std::path::{Path, PathBuf};
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
        .expect("workspace root must resolve")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ncc-{label}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir must create");
    dir
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("ncc binary parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn run_ncc(args: &[String]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn periodic_table_builds_real_pe_with_materialized_grid_and_win32_runtime() {
    let root = workspace_root();
    let source = root.join("examples/12_apps/periodic_table_interactive.n");
    let out_dir = unique_temp_dir("periodic-table-win32-e2e");
    let output = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.win64".into(),
        "--target".into(),
        "x86_64-pc-windows-msvc".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(output.status.success(), "{}", rendered(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("native build must emit JSON: {err}\n{}", rendered(&output)));

    assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
    assert_eq!(json["status"], "native_build_passed");
    assert_eq!(json["target"]["object_format"], "coff");
    assert_eq!(json["target"]["abi"], "win64");
    assert_eq!(json["speed_contract"]["interpreter_fallback"], false);
    assert_eq!(
        json["truth_boundary"]["executable_is_native_pe_direct"],
        true
    );

    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let object = PathBuf::from(json["paths"]["object"].as_str().expect("object path"));
    let machine = PathBuf::from(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    );
    let backend_object = PathBuf::from(
        json["paths"]["machine_backend_object_ir"]
            .as_str()
            .expect("backend object IR path"),
    );
    for artifact in [&exe, &object, &machine, &backend_object] {
        assert!(
            artifact.is_file(),
            "missing artifact: {}",
            artifact.display()
        );
        assert!(
            std::fs::metadata(artifact)
                .expect("artifact metadata")
                .len()
                > 0,
            "empty artifact: {}",
            artifact.display()
        );
    }

    let exe_bytes = std::fs::read(&exe).expect("PE executable must read");
    assert_eq!(&exe_bytes[..2], b"MZ", "executable must use PE DOS header");
    let pe_offset = u32::from_le_bytes(exe_bytes[0x3c..0x40].try_into().expect("e_lfanew"));
    let pe_offset = usize::try_from(pe_offset).expect("PE offset must fit usize");
    assert_eq!(
        &exe_bytes[pe_offset..pe_offset + 4],
        b"PE\0\0",
        "executable must expose a real PE signature"
    );
    let optional_header = pe_offset + 4 + 20;
    assert_eq!(
        u16::from_le_bytes(
            exe_bytes[optional_header + 68..optional_header + 70]
                .try_into()
                .expect("PE subsystem"),
        ),
        2,
        "native-window application must use IMAGE_SUBSYSTEM_WINDOWS_GUI"
    );
    assert_eq!(
        json["link"]["phase12_report"]["pe64_evidence"]["subsystem"],
        "windows_gui"
    );

    let backend_text = std::fs::read_to_string(&backend_object).expect("backend IR must read");
    for function in ["rango", "periodo", "elementoEn"] {
        let marker = format!(".section .text.tablaPeriodicaInteractiva.{function}");
        assert!(backend_text.contains(&marker), "missing {marker}");
    }
    let grid_section = backend_text
        .split(".section .text.tablaPeriodicaInteractiva.elementoEn")
        .nth(1)
        .expect("periodic grid lookup section")
        .split("\n.section ")
        .next()
        .expect("periodic grid lookup section body");
    assert!(
        grid_section.contains("lowering-coverage")
            && grid_section.contains("complete=true")
            && grid_section.contains("mov rax, rcx")
            && grid_section.contains("mov rax, rdx")
            && grid_section.contains("scalar-branch-cmp")
            && grid_section.contains("tablaPeriodicaInteractiva_elementoEn.__cf"),
        "periodic grid lookup must retain complete parameter-dependent branches"
    );
    assert!(
        !backend_text.contains(
            "tablaPeriodicaInteractiva.elementoEn:\n    mov rax, 0 ; scalar-return\n    ret"
        ),
        "grid lookup must not collapse to an unconditional empty-cell return"
    );

    let imports = json["link"]["phase12_report"]["dynamic_imports"]["imports"]
        .as_array()
        .expect("PE imports array")
        .iter()
        .filter_map(|entry| entry["symbol"].as_str())
        .collect::<Vec<_>>();
    for symbol in [
        "CreateWindowExW",
        "PeekMessageA",
        "RemovePropA",
        "TextOutW",
        "MultiByteToWideChar",
        "FillRect",
        "GetCursorPos",
        "malloc",
        "strcmp",
    ] {
        assert!(imports.contains(&symbol), "missing real PE import {symbol}");
    }
    for legacy_ansi in ["CreateWindowExA", "SetWindowTextA", "TextOutA"] {
        assert!(
            !imports.contains(&legacy_ansi),
            "Unicode PE unexpectedly imports {legacy_ansi}"
        );
    }
    for abstract_symbol in [
        "windowOpen",
        "windowReadKey",
        "canvasRect",
        "strEq",
        "strSubstring",
        "strToLower",
        "nC.runtime.terminal.windowOpen",
        "nC.runtime.terminal.windowPoll",
        "nC.runtime.render.canvasPresent",
        "nC.runtime.input.windowReadKey",
        "nC.runtime.string.strEq",
    ] {
        assert!(
            !imports.contains(&abstract_symbol),
            "abstract runtime symbol leaked into PE imports: {abstract_symbol}"
        );
    }

    for owner_key in [b"nC.canvas.dc\0".as_slice(), b"nC.canvas.bmp\0".as_slice()] {
        assert!(
            exe_bytes
                .windows(owner_key.len())
                .any(|window| window == owner_key),
            "PE windowClose must retain cleanup ownership key {owner_key:?}"
        );
    }

    std::fs::remove_dir_all(out_dir).expect("temporary build tree must clean");
}
