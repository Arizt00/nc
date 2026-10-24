use ncc_lexer::TokenKind;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ncc() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn native_executable() -> PathBuf {
    let extension = if cfg!(windows) { "exe" } else { "bin" };
    root()
        .join("target/tmp/ncc-stage2-lexer-differential")
        .join(format!("ncc-stage2-lexer.{extension}"))
}

fn linker() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("NCC_LINKER") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }
    if cfg!(windows) {
        for path in [
            PathBuf::from(r"C:\msys64\mingw64\bin\gcc.exe"),
            PathBuf::from(r"C:\msys64\ucrt64\bin\gcc.exe"),
        ] {
            if path.is_file() {
                return Some(path);
            }
        }
        return None;
    }
    Some(PathBuf::from("cc"))
}

fn with_linker_path(command: &mut Command, linker: &Path) {
    let Some(parent) = linker.parent() else {
        return;
    };
    let mut paths = vec![parent.to_path_buf()];
    if let Some(current) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&current));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        command.env("PATH", joined);
    }
}

fn build_native_lexer_once() -> Option<PathBuf> {
    let linker = linker()?;
    let output = native_executable();
    fs::create_dir_all(output.parent().expect("native output parent"))
        .expect("create native output parent");
    let mut command = Command::new(ncc());
    command
        .current_dir(root())
        .args(["project", "link", "tools/ncc/nc", "--main", "main.n"])
        .arg("--out")
        .arg(&output)
        .arg("--linker")
        .arg(&linker);
    with_linker_path(&mut command, &linker);
    let built = command.output().expect("launch ncc project link");
    assert!(
        built.status.success(),
        "native lexer build failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(output.is_file(), "native lexer output was omitted");
    Some(output)
}

fn build_native_lexer() -> Option<PathBuf> {
    static NATIVE: OnceLock<Option<PathBuf>> = OnceLock::new();
    NATIVE.get_or_init(build_native_lexer_once).clone()
}

fn run(executable: &Path, args: &[&str]) -> Output {
    Command::new(executable)
        .current_dir(root())
        .args(args)
        .output()
        .expect("run executable")
}

fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().expect("u16 object field"))
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("u32 object field"))
}

fn i32le(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes[at..at + 4].try_into().expect("i32 material field"))
}

fn u64le(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("u64 object field"))
}

fn object_string(bytes: &[u8], at: usize) -> &str {
    let end = bytes[at..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|length| at + length)
        .expect("terminated object symbol");
    std::str::from_utf8(&bytes[at..end]).expect("UTF-8 object symbol")
}

fn wasm_uleb(bytes: &[u8], at: &mut usize) -> u32 {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    loop {
        let byte = bytes[*at];
        *at += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
        assert!(shift < 35, "invalid WASM ULEB32");
    }
}

fn wasm_name<'a>(bytes: &'a [u8], at: &mut usize) -> &'a str {
    let count = wasm_uleb(bytes, at) as usize;
    let value = std::str::from_utf8(&bytes[*at..*at + count]).expect("UTF-8 WASM name");
    *at += count;
    value
}

fn wasm_section(bytes: &[u8], wanted: u8) -> (usize, usize) {
    assert_eq!(&bytes[..8], b"\0asm\x01\0\0\0");
    let mut at = 8;
    while at < bytes.len() {
        let id = bytes[at];
        at += 1;
        let count = wasm_uleb(bytes, &mut at) as usize;
        if id == wanted {
            return (at, count);
        }
        at += count;
    }
    panic!("missing WASM section {wanted}");
}

fn wasm_custom_section(bytes: &[u8], wanted: &str) -> (usize, usize) {
    assert_eq!(&bytes[..8], b"\0asm\x01\0\0\0");
    let mut at = 8;
    while at < bytes.len() {
        let id = bytes[at];
        at += 1;
        let count = wasm_uleb(bytes, &mut at) as usize;
        let end = at + count;
        if id == 0 {
            let name = wasm_name(bytes, &mut at);
            if name == wanted {
                return (at, end - at);
            }
        }
        at = end;
    }
    panic!("missing WASM custom section {wanted}");
}

fn elf64_section(bytes: &[u8], wanted: &str) -> (usize, usize, u64, u32, u32, u64) {
    assert_eq!(&bytes[..4], b"\x7fELF");
    assert_eq!(bytes[4], 2, "ELF64 section lookup requires ELFCLASS64");
    let table = u64le(bytes, 40) as usize;
    let stride = u16le(bytes, 58) as usize;
    let count = u16le(bytes, 60) as usize;
    let strings_index = u16le(bytes, 62) as usize;
    let strings = table + strings_index * stride;
    let strings_at = u64le(bytes, strings + 24) as usize;
    for index in 0..count {
        let section = table + index * stride;
        let name = object_string(bytes, strings_at + u32le(bytes, section) as usize);
        if name == wanted {
            return (
                u64le(bytes, section + 24) as usize,
                u64le(bytes, section + 32) as usize,
                u64le(bytes, section + 16),
                u32le(bytes, section + 40),
                u32le(bytes, section + 44),
                u64le(bytes, section + 56),
            );
        }
    }
    panic!("missing ELF64 section {wanted}");
}

fn elf32_section(bytes: &[u8], wanted: &str) -> (usize, usize, u32, u32, u32, u32) {
    assert_eq!(&bytes[..4], b"\x7fELF");
    assert_eq!(bytes[4], 1, "ELF32 section lookup requires ELFCLASS32");
    let table = u32le(bytes, 32) as usize;
    let stride = u16le(bytes, 46) as usize;
    let count = u16le(bytes, 48) as usize;
    let strings_index = u16le(bytes, 50) as usize;
    let strings = table + strings_index * stride;
    let strings_at = u32le(bytes, strings + 16) as usize;
    for index in 0..count {
        let section = table + index * stride;
        let name = object_string(bytes, strings_at + u32le(bytes, section) as usize);
        if name == wanted {
            return (
                u32le(bytes, section + 16) as usize,
                u32le(bytes, section + 20) as usize,
                u32le(bytes, section + 12),
                u32le(bytes, section + 24),
                u32le(bytes, section + 28),
                u32le(bytes, section + 36),
            );
        }
    }
    panic!("missing ELF32 section {wanted}");
}

fn pe_section(bytes: &[u8], wanted: &[u8]) -> (usize, usize, usize, usize) {
    let pe = u32le(bytes, 0x3c) as usize;
    let coff = pe + 4;
    let sections = u16le(bytes, coff + 2) as usize;
    let optional_bytes = u16le(bytes, coff + 16) as usize;
    let table = coff + 20 + optional_bytes;
    for index in 0..sections {
        let at = table + index * 40;
        let end = bytes[at..at + 8]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(8);
        if &bytes[at..at + end] == wanted {
            return (
                u32le(bytes, at + 12) as usize,
                u32le(bytes, at + 8) as usize,
                u32le(bytes, at + 20) as usize,
                u32le(bytes, at + 16) as usize,
            );
        }
    }
    panic!("missing PE section {}", String::from_utf8_lossy(wanted));
}

fn pe_rva_offset(bytes: &[u8], rva: usize) -> usize {
    let (text_rva, text_size, text_raw, text_raw_size) = pe_section(bytes, b".text");
    if rva >= text_rva && rva < text_rva + text_size.max(text_raw_size) {
        return text_raw + rva - text_rva;
    }
    let (idata_rva, idata_size, idata_raw, idata_raw_size) = pe_section(bytes, b".idata");
    if rva >= idata_rva && rva < idata_rva + idata_size.max(idata_raw_size) {
        return idata_raw + rva - idata_rva;
    }
    panic!("RVA {rva:#x} does not belong to a retained PE section");
}

fn elf_symbol_sections(bytes: &[u8], wanted: &str) -> Vec<u16> {
    let elf64 = bytes[4] == 2;
    let section_at = if elf64 {
        u64le(bytes, 40) as usize
    } else {
        u32le(bytes, 32) as usize
    };
    let section_bytes = u16le(bytes, if elf64 { 58 } else { 46 }) as usize;
    let section_count = u16le(bytes, if elf64 { 60 } else { 48 }) as usize;
    let symtab = (0..section_count)
        .map(|index| section_at + index * section_bytes)
        .find(|at| u32le(bytes, *at + 4) == 2)
        .expect("ELF symtab");
    let string_section_index = u32le(bytes, symtab + if elf64 { 40 } else { 24 }) as usize;
    let string_section = section_at + string_section_index * section_bytes;
    let strings = if elf64 {
        u64le(bytes, string_section + 24) as usize
    } else {
        u32le(bytes, string_section + 16) as usize
    };
    let symbols = if elf64 {
        u64le(bytes, symtab + 24) as usize
    } else {
        u32le(bytes, symtab + 16) as usize
    };
    let symbol_bytes = if elf64 {
        u64le(bytes, symtab + 56) as usize
    } else {
        u32le(bytes, symtab + 36) as usize
    };
    let symbols_size = if elf64 {
        u64le(bytes, symtab + 32) as usize
    } else {
        u32le(bytes, symtab + 20) as usize
    };
    (0..symbols_size / symbol_bytes)
        .filter_map(|index| {
            let at = symbols + index * symbol_bytes;
            let name = u32le(bytes, at) as usize;
            (name != 0 && object_string(bytes, strings + name) == wanted)
                .then(|| u16le(bytes, at + if elf64 { 6 } else { 14 }))
        })
        .collect()
}

fn coff_symbol_sections(bytes: &[u8], wanted: &str) -> Vec<i16> {
    let symbols = u32le(bytes, 8) as usize;
    let count = u32le(bytes, 12) as usize;
    let strings = symbols + count * 18;
    (0..count)
        .filter_map(|index| {
            let at = symbols + index * 18;
            let name = if u32le(bytes, at) == 0 {
                object_string(bytes, strings + u32le(bytes, at + 4) as usize)
            } else {
                let end = bytes[at..at + 8]
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(8);
                std::str::from_utf8(&bytes[at..at + end]).expect("UTF-8 COFF symbol")
            };
            (name == wanted)
                .then(|| i16::from_le_bytes(bytes[at + 12..at + 14].try_into().unwrap()))
        })
        .collect()
}

fn macho_symbol_sections(bytes: &[u8], wanted: &str) -> Vec<u8> {
    let commands = u32le(bytes, 16) as usize;
    let mut command = 32usize;
    let mut table = None;
    for _ in 0..commands {
        if u32le(bytes, command) == 2 {
            table = Some((
                u32le(bytes, command + 8) as usize,
                u32le(bytes, command + 12) as usize,
                u32le(bytes, command + 16) as usize,
            ));
            break;
        }
        command += u32le(bytes, command + 4) as usize;
    }
    let (symbols, count, strings) = table.expect("Mach-O LC_SYMTAB");
    (0..count)
        .filter_map(|index| {
            let at = symbols + index * 16;
            (object_string(bytes, strings + u32le(bytes, at) as usize) == wanted)
                .then(|| bytes[at + 5])
        })
        .collect()
}

fn macho_section(bytes: &[u8], wanted: &str) -> (usize, usize, u64, u32, u32, u32) {
    let commands = u32le(bytes, 16) as usize;
    let mut command = 32usize;
    for _ in 0..commands {
        let kind = u32le(bytes, command);
        let size = u32le(bytes, command + 4) as usize;
        if kind == 0x19 {
            let sections = u32le(bytes, command + 64) as usize;
            let mut section = command + 72;
            for _ in 0..sections {
                let end = bytes[section..section + 16]
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(16);
                if std::str::from_utf8(&bytes[section..section + end]).expect("Mach-O section")
                    == wanted
                {
                    return (
                        u32le(bytes, section + 48) as usize,
                        u64le(bytes, section + 40) as usize,
                        u64le(bytes, section + 32),
                        u32le(bytes, section + 64),
                        u32le(bytes, section + 68),
                        u32le(bytes, section + 72),
                    );
                }
                section += 80;
            }
        }
        command += size;
    }
    panic!("Mach-O section {wanted} was omitted");
}

fn macho_command(bytes: &[u8], wanted: u32) -> usize {
    let commands = u32le(bytes, 16) as usize;
    let mut command = 32usize;
    for _ in 0..commands {
        if u32le(bytes, command) == wanted {
            return command;
        }
        command += u32le(bytes, command + 4) as usize;
    }
    panic!("Mach-O load command 0x{wanted:x} was omitted");
}

fn u32be(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("u32be object field"))
}

fn assert_macho_ad_hoc_signature(bytes: &[u8]) {
    let command = macho_command(bytes, 0x1d);
    let signature = u32le(bytes, command + 8) as usize;
    let signature_bytes = u32le(bytes, command + 12) as usize;
    assert_eq!(signature + signature_bytes, bytes.len());
    assert_eq!(u32be(bytes, signature), 0xfade0cc0);
    assert_eq!(u32be(bytes, signature + 4) as usize, signature_bytes);
    assert_eq!(u32be(bytes, signature + 8), 1);
    let directory = signature + u32be(bytes, signature + 16) as usize;
    assert_eq!(u32be(bytes, directory), 0xfade0c02);
    assert_eq!(u32be(bytes, directory + 8), 0x0002_0200);
    assert_eq!(u32be(bytes, directory + 12), 2);
    let hash_offset = u32be(bytes, directory + 16) as usize;
    let identifier = u32be(bytes, directory + 20) as usize;
    let pages = u32be(bytes, directory + 28) as usize;
    let code_limit = u32be(bytes, directory + 32) as usize;
    assert_eq!(code_limit, signature);
    assert_eq!(
        object_string(bytes, directory + identifier),
        "ncc.mct.final"
    );
    assert_eq!(
        (
            bytes[directory + 36],
            bytes[directory + 37],
            bytes[directory + 39]
        ),
        (32, 2, 12)
    );
    assert_eq!(pages, code_limit.div_ceil(0x1000));
    for page in 0..pages {
        let start = page * 0x1000;
        let end = (start + 0x1000).min(code_limit);
        assert_eq!(
            &bytes[directory + hash_offset + page * 32..directory + hash_offset + (page + 1) * 32],
            Sha256::digest(&bytes[start..end]).as_slice(),
            "Mach-O CodeDirectory page {page}"
        );
    }
}

fn run_project_sync_json(
    executable: &Path,
    project: &Path,
    cache: &Path,
    target: &str,
    incremental: bool,
) -> Value {
    let mut arguments = vec![
        "project",
        "sync",
        project.to_str().expect("UTF-8 project path"),
        "--main",
        "main.n",
        "--target",
        target,
        "--content-cache-dir",
        cache.to_str().expect("UTF-8 cache path"),
        "--json",
    ];
    if !incremental {
        arguments.push("--no-incremental");
    }
    let output = run(executable, &arguments);
    assert!(
        output.status.success(),
        "native project cache sync failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("native project cache JSON")
}

fn kind_name(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(_) => "Identifier".to_string(),
        TokenKind::Number(_) => "Number".to_string(),
        TokenKind::String(_) => "String".to_string(),
        TokenKind::Char(_) => "Char".to_string(),
        other => format!("{other:?}"),
    }
}

fn payload(kind: &TokenKind) -> Vec<u8> {
    match kind {
        TokenKind::Identifier(value) | TokenKind::Number(value) | TokenKind::String(value) => {
            value.as_bytes().to_vec()
        }
        TokenKind::Char(value) => vec![*value as u8],
        _ => Vec::new(),
    }
}

fn payload_hash(bytes: &[u8]) -> i64 {
    bytes.iter().fold(5381_i64, |hash, byte| {
        (hash * 33 + i64::from(*byte) + 1) % 16_777_213
    })
}

#[test]
fn native_nc_lexer_matches_rust_stage0_token_contract() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native differential execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/surface.n");
    let source = fs::read_to_string(&fixture).expect("read lexer fixture");
    let expected = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "tokens",
            fixture.to_str().expect("UTF-8 fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native lexer failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native token JSON");
    let rows = document["tokens"].as_array().expect("native token rows");
    assert_eq!(rows.len(), expected.len());
    for (index, (row, token)) in rows.iter().zip(&expected).enumerate() {
        let material = payload(&token.kind);
        assert_eq!(row["kind"], kind_name(&token.kind), "token {index}");
        assert_eq!(row["line"], token.line, "token {index}");
        assert_eq!(row["column"], token.column, "token {index}");
        assert_eq!(row["payload_bytes"], material.len(), "token {index}");
        assert_eq!(
            row["payload_hash"],
            match &token.kind {
                TokenKind::Identifier(_)
                | TokenKind::Number(_)
                | TokenKind::String(_)
                | TokenKind::Char(_) => payload_hash(&material),
                _ => 0,
            },
            "token {index}"
        );
    }
}

#[test]
fn native_nc_parser_outline_matches_rust_stage0_program_shape() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native parser outline execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/surface.n");
    let source = fs::read_to_string(&fixture).expect("read parser fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts parser fixture");
    let token_count = tokens.len();
    let expected =
        ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts parser fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "outline",
            fixture.to_str().expect("UTF-8 fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native parser outline failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native outline JSON");
    assert_eq!(document["schema"], "nc.bootstrap.parser.outline.v1");
    assert_eq!(document["token_count"], token_count);
    assert_eq!(document["import_count"], expected.uses.len());
    assert_eq!(document["container_count"], expected.containers.len());
    let hashes = document["container_name_hashes"]
        .as_array()
        .expect("container hashes");
    let expected_hashes = expected
        .containers
        .iter()
        .map(|container| payload_hash(container.name.as_bytes()))
        .collect::<Vec<_>>();
    assert_eq!(
        hashes
            .iter()
            .map(|value| value.as_i64().expect("integer hash"))
            .collect::<Vec<_>>(),
        expected_hashes
    );
}

#[test]
fn native_nc_parser_outline_owns_structural_diagnostics() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native parser diagnostics: no host linker");
        return;
    };
    for (fixture, code) in [
        (
            "bootstrap/corpus/ncc-stage2-lexer/outline_mismatched.n",
            "E_PARSER_DELIMITER_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/outline_unterminated.n",
            "E_PARSER_DELIMITER_UNTERMINATED_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/outline_import_semicolon.n",
            "E_PARSER_IMPORT_TERMINATOR_001",
        ),
    ] {
        let output = run(&native, &["compiler", "outline", fixture, "--json"]);
        assert_eq!(output.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn native_nc_ast_materializes_declarations_signatures_and_visibility() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native AST declaration execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ast_declarations.n");
    let source = fs::read_to_string(&fixture).expect("read declaration fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts declaration fixture");
    ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts declaration fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "ast-head",
            fixture.to_str().expect("UTF-8 declaration fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native declaration AST failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native declaration AST");
    assert_eq!(document["schema"], "nc.bootstrap.parser.ast-head.v5");
    assert_eq!(document["node_count"], 55);
    assert_eq!(document["declarations"], 9);
    assert_eq!(document["parameters"], 2);
    assert_eq!(document["entries"], 1);
    assert_eq!(document["type_nodes"], 10);
    assert_eq!(document["expression_nodes"], 18);
    assert_eq!(document["statement_nodes"], 6);
    let rows = document["nodes"]
        .as_array()
        .expect("native declaration nodes");
    assert_eq!(
        rows.iter()
            .filter(|row| row["kind"] == "FunctionDecl")
            .count(),
        2
    );
    assert!(rows
        .iter()
        .any(|row| row["kind"] == "FieldDecl" && row["flags"] == 1));
    assert!(rows
        .iter()
        .any(|row| row["kind"] == "FieldDecl" && row["flags"] == 3));
}

#[test]
fn native_nc_ast_disambiguates_prefix_words_from_parenthesized_calls() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native expression disambiguation: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ast_expression_disambiguation.n");
    let actual = run(
        &native,
        &[
            "compiler",
            "ast-head",
            fixture.to_str().expect("UTF-8 expression fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native expression AST failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native expression AST");
    assert_eq!(document["node_count"], 46);
    let rows = document["nodes"]
        .as_array()
        .expect("native expression nodes");
    assert!(rows.iter().any(|row| row["kind"] == "ExprTernary"));
    assert!(rows.iter().any(|row| row["kind"] == "ExprCall"));
    assert!(rows.iter().any(|row| row["kind"] == "ExprPrefix"));
}

#[test]
fn native_nc_ast_materializes_composite_types_and_expression_precedence() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native material AST execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ast_material.n");
    let source = fs::read_to_string(&fixture).expect("read material AST fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts material fixture");
    ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts material fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "ast-head",
            fixture.to_str().expect("UTF-8 material fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native material AST failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native material AST");
    assert_eq!(document["schema"], "nc.bootstrap.parser.ast-head.v5");
    assert_eq!(document["node_count"], 87);
    assert_eq!(document["type_nodes"], 26);
    assert_eq!(document["expression_nodes"], 37);
    assert_eq!(document["statement_nodes"], 6);
    let rows = document["nodes"].as_array().expect("native material nodes");
    for retained in [
        "TypeApply",
        "TypePointer",
        "TypeArray",
        "TypeFunction",
        "ExprUnary",
        "ExprBinary",
        "ExprCall",
        "ExprIndex",
        "ExprTernary",
        "ExprArray",
    ] {
        assert!(
            rows.iter().any(|row| row["kind"] == retained),
            "material AST omitted {retained}"
        );
    }
    for (invalid, code) in [
        (
            "bootstrap/corpus/ncc-stage2-lexer/ast_invalid_type.n",
            "E_PARSER_TYPE_MATERIAL_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/ast_invalid_expression.n",
            "E_PARSER_EXPRESSION_MATERIAL_001",
        ),
    ] {
        let rejected = run(&native, &["compiler", "ast-head", invalid, "--json"]);
        assert_eq!(rejected.status.code(), Some(1), "{invalid}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{invalid}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_ast_materializes_core_statements_assignments_and_control_flow() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native statement AST execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ast_statements.n");
    let source = fs::read_to_string(&fixture).expect("read statement AST fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts statement fixture");
    ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts statement fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "ast-head",
            fixture.to_str().expect("UTF-8 statement fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native statement AST failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native statement AST");
    assert_eq!(document["schema"], "nc.bootstrap.parser.ast-head.v5");
    assert_eq!(document["node_count"], 89);
    assert_eq!(document["expression_nodes"], 46);
    assert_eq!(document["statement_nodes"], 28);
    let rows = document["nodes"]
        .as_array()
        .expect("native statement nodes");
    for retained in [
        "Block",
        "ExprStmt",
        "AssignStmt",
        "ReturnStmt",
        "IfStmt",
        "WhileStmt",
        "ForStmt",
        "GuardStmt",
        "DeferStmt",
        "BreakStmt",
        "ContinueStmt",
    ] {
        assert!(
            rows.iter().any(|row| row["kind"] == retained),
            "statement AST omitted {retained}"
        );
    }

    let invalid = "bootstrap/corpus/ncc-stage2-lexer/ast_invalid_statement.n";
    let rejected = run(&native, &["compiler", "ast-head", invalid, "--json"]);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&rejected.stdout).contains("E_PARSER_STATEMENT_MATERIAL_001"),
        "{invalid}: {}",
        String::from_utf8_lossy(&rejected.stdout)
    );
}

#[test]
fn native_nc_ast_materializes_patterns_closures_and_advanced_control_flow() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native advanced AST execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ast_advanced.n");
    let source = fs::read_to_string(&fixture).expect("read advanced AST fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts advanced fixture");
    ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts advanced fixture");
    let actual = run(
        &native,
        &[
            "compiler",
            "ast-head",
            fixture.to_str().expect("UTF-8 advanced fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native advanced AST failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native advanced AST");
    assert_eq!(document["schema"], "nc.bootstrap.parser.ast-head.v5");
    assert_eq!(document["node_count"], 224);
    assert_eq!(document["advanced_nodes"], 52);
    assert_eq!(document["expression_nodes"], 97);
    assert_eq!(document["statement_nodes"], 56);
    let rows = document["nodes"].as_array().expect("native advanced nodes");
    for retained in [
        "ExprStruct",
        "ExprClosure",
        "ClosureParameter",
        "ClosureCapture",
        "ExprTry",
        "ExprPrefix",
        "ExprIf",
        "MatchStmt",
        "MatchArm",
        "PatternVariant",
        "PatternStruct",
        "PatternArray",
        "PatternRange",
        "PatternRest",
        "ForInStmt",
        "ForRangeStmt",
        "RepeatStmt",
        "DoWhileStmt",
        "UnlessStmt",
        "LoopStmt",
        "YieldStmt",
    ] {
        assert!(
            rows.iter().any(|row| row["kind"] == retained),
            "advanced AST omitted {retained}"
        );
    }

    let invalid = "bootstrap/corpus/ncc-stage2-lexer/ast_invalid_advanced.n";
    let rejected = run(&native, &["compiler", "ast-head", invalid, "--json"]);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&rejected.stdout).contains("E_PARSER_ADVANCED_MATERIAL_001"),
        "{invalid}: {}",
        String::from_utf8_lossy(&rejected.stdout)
    );
}

#[test]
fn native_nc_sema_materializes_scopes_symbols_and_core_type_flow() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native semantic execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_core.n");
    let source = fs::read_to_string(&fixture).expect("read semantic fixture");
    let tokens = ncc_lexer::lex(&source).expect("Rust stage0 lexer accepts semantic fixture");
    let program =
        ncc_parser::parse_material_v2(tokens).expect("Rust stage0 parser accepts semantic fixture");
    ncc_sema::analyze(&program).expect("Rust stage0 sema accepts the retained valid fixture");

    let actual = run(
        &native,
        &[
            "compiler",
            "sema",
            fixture.to_str().expect("UTF-8 semantic fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native semantic analysis failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native semantic JSON");
    assert_eq!(document["schema"], "nc.bootstrap.sema.core.v1");
    assert_eq!(document["node_count"], 69);
    assert_eq!(document["scopes"], 9);
    assert_eq!(document["symbols"], 11);
    assert_eq!(document["resolved_names"], 11);
    assert_eq!(document["types_checked"], 10);
    assert_eq!(document["calls_checked"], 1);
    assert_eq!(document["conditions_checked"], 2);
    assert_eq!(document["returns_checked"], 2);
    assert_eq!(document["entries_checked"], 1);

    let shape = run(
        &native,
        &[
            "compiler",
            "sema-core-check",
            fixture.to_str().expect("UTF-8 semantic fixture path"),
            "9",
            "11",
            "11",
            "1",
        ],
    );
    assert!(shape.status.success(), "semantic shape contract failed");
    let drift = run(
        &native,
        &[
            "compiler",
            "sema-core-check",
            fixture.to_str().expect("UTF-8 semantic fixture path"),
            "8",
            "11",
            "11",
            "1",
        ],
    );
    assert_eq!(drift.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&drift.stdout).contains("E_SEMA_SHAPE_001"));

    for (invalid, code) in [
        ("sema_duplicate.n", "E_LOCAL_DUP_SYMBOL_001"),
        ("sema_unresolved.n", "E_SEMA_SYMBOL_UNRESOLVED_001"),
        ("sema_unknown_type.n", "E_TYPE_UNKNOWN_001"),
        ("sema_condition.n", "E_CONDITION_BOOL_REQUIRED_001"),
        ("sema_call_arity.n", "E_SEMA_CALL_ARITY_001"),
        ("sema_return.n", "E_TYPE_RETURN_001"),
        ("sema_entry.n", "E_SEMA_ENTRY_UNRESOLVED_001"),
        ("sema_break.n", "E_CONTROL_BREAK_SCOPE_001"),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(invalid);
        let rejected = run(
            &native,
            &[
                "compiler",
                "sema",
                path.to_str().expect("UTF-8 invalid semantic fixture path"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{invalid}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{invalid}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_sema_owns_canonical_import_runtime_and_modal_contracts() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native import semantic execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_import_runtime_modal.n");
    let actual = run(
        &native,
        &[
            "compiler",
            "sema-imports",
            fixture.to_str().expect("UTF-8 import fixture path"),
            "--json",
        ],
    );
    assert!(
        actual.status.success(),
        "native import semantics failed: {}",
        String::from_utf8_lossy(&actual.stdout)
    );
    let document: Value = serde_json::from_slice(&actual.stdout).expect("native import JSON");
    assert_eq!(
        document["schema"],
        "nc.bootstrap.sema.import-runtime-signatures.v3"
    );
    assert_eq!(document["import_groups"], 1);
    assert_eq!(document["import_items"], 3);
    assert_eq!(document["runtime_calls"], 2);
    assert_eq!(document["modal_checks"], 1);
    assert_eq!(document["capability_checks"], 2);
    assert_eq!(document["policy_checks"], 2);
    assert_eq!(document["context_checks"], 3);
    assert_eq!(document["authority_fingerprint"], 12_843);
    assert_eq!(document["container_contexts"], 1);
    assert_eq!(document["signature_checks"], 2);
    assert_eq!(document["signature_fingerprint"], 14_388_167);
    assert_eq!(document["catalog_families"], 96);
    assert_eq!(document["catalog_signatures"], 1_007);
    assert_eq!(document["calls_checked"], 2);

    let shape = run(
        &native,
        &[
            "compiler",
            "sema-imports-check",
            fixture.to_str().expect("UTF-8 import fixture path"),
            "1",
            "3",
            "2",
            "1",
        ],
    );
    assert!(
        shape.status.success(),
        "native import shape contract failed"
    );

    let catalog = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_import_catalog_all.n");
    let catalog_shape = run(
        &native,
        &[
            "compiler",
            "sema-imports-check",
            catalog.to_str().expect("UTF-8 import catalog fixture path"),
            "1",
            "273",
            "0",
            "1",
        ],
    );
    assert!(
        catalog_shape.status.success(),
        "native 273-item import catalog contract failed: {}",
        String::from_utf8_lossy(&catalog_shape.stdout)
    );
}

#[test]
fn native_nc_sema_owns_runtime_context_capability_and_policy_authority() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native runtime authority execution: no host linker");
        return;
    };
    for (fixture, expected) in [
        (
            "sema_import_catalog_all.n",
            ["270", "270", "273", "1009181", "1"],
        ),
        ("sema_authority_system_ok.n", ["5", "5", "5", "18545", "1"]),
        ("sema_authority_network_ok.n", ["6", "6", "6", "19577", "1"]),
        (
            "sema_authority_security_ok.n",
            ["8", "8", "8", "38054", "1"],
        ),
        (
            "sema_authority_compiler_ok.n",
            ["5", "5", "6", "31271", "1"],
        ),
        (
            "sema_authority_kernel_core_ok.n",
            ["3", "3", "3", "4807", "1"],
        ),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let path = path.to_str().expect("UTF-8 authority fixture path");
        let checked = run(
            &native,
            &[
                "compiler",
                "sema-authority-check",
                path,
                expected[0],
                expected[1],
                expected[2],
                expected[3],
                expected[4],
            ],
        );
        assert!(
            checked.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }

    for fixture in [
        "sema_authority_context_denied.n",
        "sema_authority_dom_denied.n",
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let rejected = run(
            &native,
            &[
                "compiler",
                "sema-authority",
                path.to_str().expect("UTF-8 authority rejection path"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains("E_USE_CONTEXT_AUTHORITY_001"),
            "{fixture}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_sema_owns_all_advanced_runtime_signatures() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native runtime signature execution: no host linker");
        return;
    };

    let catalog = run(&native, &["runtime-signatures"]);
    assert!(
        catalog.status.success(),
        "runtime signature authority failed: {}",
        String::from_utf8_lossy(&catalog.stdout)
    );
    let catalog_json: Value =
        serde_json::from_slice(&catalog.stdout).expect("runtime signature JSON");
    assert_eq!(catalog_json["schema"], "nc.bootstrap.runtime-signatures.v1");
    assert_eq!(catalog_json["families"], 96);
    assert_eq!(catalog_json["signatures"], 1_007);
    assert_eq!(catalog_json["fingerprint"], 5_926_350);
    assert_eq!(catalog_json["status"], "ok");

    let retained = run(
        &native,
        &[
            "compiler",
            "runtime-signatures-check",
            "96",
            "1007",
            "5926350",
        ],
    );
    assert!(retained.status.success(), "retained signature shape failed");

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_runtime_signatures_ok.n");
    let analyzed = run(
        &native,
        &[
            "compiler",
            "sema-imports",
            fixture.to_str().expect("UTF-8 runtime signature fixture"),
            "--json",
        ],
    );
    assert!(
        analyzed.status.success(),
        "typed runtime calls failed: {}",
        String::from_utf8_lossy(&analyzed.stdout)
    );
    let document: Value =
        serde_json::from_slice(&analyzed.stdout).expect("typed runtime call JSON");
    assert_eq!(
        document["schema"],
        "nc.bootstrap.sema.import-runtime-signatures.v3"
    );
    assert_eq!(document["signature_checks"], 7);
    assert_eq!(document["signature_fingerprint"], 37_466_667);
    assert_eq!(document["catalog_families"], 96);
    assert_eq!(document["catalog_signatures"], 1_007);
    assert_eq!(document["runtime_calls"], 7);

    for (fixture, code) in [
        ("sema_runtime_signature_arity.n", "E_SEMA_CALL_ARITY_001"),
        ("sema_runtime_signature_type.n", "E_TYPE_MISMATCH_001"),
        ("sema_runtime_signature_import.n", "E_RUNTIME_USE_001"),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let rejected = run(
            &native,
            &[
                "compiler",
                "sema-imports",
                path.to_str().expect("UTF-8 rejected signature fixture"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_sema_owns_generics_traits_static_dispatch_and_manual_abi() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native generic/trait/ABI execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_generic_trait_abi_ok.n");
    let analyzed = run(
        &native,
        &[
            "compiler",
            "sema-generic-abi",
            fixture.to_str().expect("UTF-8 generic/trait/ABI fixture"),
            "--json",
        ],
    );
    assert!(
        analyzed.status.success(),
        "native generic/trait/ABI authority failed: {}",
        String::from_utf8_lossy(&analyzed.stdout)
    );
    let document: Value = serde_json::from_slice(&analyzed.stdout).expect("generic/trait/ABI JSON");
    assert_eq!(document["schema"], "nc.bootstrap.sema.generic-trait-abi.v1");
    assert_eq!(document["traits"], 1);
    assert_eq!(document["trait_methods"], 1);
    assert_eq!(document["associated_types"], 1);
    assert_eq!(document["impls"], 1);
    assert_eq!(document["implemented_methods"], 1);
    assert_eq!(document["associated_bindings"], 1);
    assert_eq!(document["generic_templates"], 2);
    assert_eq!(document["generic_parameters"], 2);
    assert_eq!(document["const_parameters"], 1);
    assert_eq!(document["bounds"], 1);
    assert_eq!(document["monomorphizations"], 2);
    assert_eq!(document["static_dispatches"], 1);
    assert_eq!(document["associated_projections"], 1);
    assert_eq!(document["abi_contracts"], 1);
    assert_eq!(document["abi_fields"], 12);
    assert_eq!(document["abi_bindings"], 1);
    assert_eq!(document["dynamic_dispatches"], 0);
    assert_eq!(document["fingerprint"], 9_643_923);

    for (fixture, code) in [
        (
            "sema_generic_duplicate_param.n",
            "E_HIGH_GENERIC_DUP_PARAM_001",
        ),
        (
            "sema_generic_unknown_bound.n",
            "E_HIGH_BOUND_UNKNOWN_TRAIT_001",
        ),
        (
            "sema_const_generic_range.n",
            "E_HIGH_CONST_GENERIC_RANGE_001",
        ),
        (
            "sema_trait_impl_missing_member.n",
            "E_HIGH_TRAIT_METHOD_MISSING_001",
        ),
        ("sema_abi_incomplete.n", "E_V2_ABI_REQUIRED_001"),
        ("sema_abi_unknown_binding.n", "E_V2_ABI_BIND_UNKNOWN_001"),
        (
            "sema_dynamic_dispatch_implicit.n",
            "E_HIGH_DYN_IMPLICIT_001",
        ),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let rejected = run(
            &native,
            &[
                "sema",
                path.to_str()
                    .expect("UTF-8 rejected generic/trait/ABI fixture"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_sema_owns_patterns_async_captures_and_resource_cleanup() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native pattern/async/resource execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/sema_pattern_async_resource_ok.n");
    let analyzed = run(
        &native,
        &[
            "compiler",
            "sema-pattern-async-resource",
            fixture
                .to_str()
                .expect("UTF-8 pattern/async/resource fixture"),
            "--json",
        ],
    );
    assert!(
        analyzed.status.success(),
        "native pattern/async/resource authority failed: {}",
        String::from_utf8_lossy(&analyzed.stdout)
    );
    let document: Value =
        serde_json::from_slice(&analyzed.stdout).expect("pattern/async/resource JSON");
    assert_eq!(
        document["schema"],
        "nc.bootstrap.sema.pattern-async-resource.v1"
    );
    assert_eq!(document["matches"], 5);
    assert_eq!(document["arms"], 10);
    assert_eq!(document["patterns"], 10);
    assert_eq!(document["wildcards"], 3);
    assert_eq!(document["variants"], 2);
    assert_eq!(document["struct_patterns"], 1);
    assert_eq!(document["array_patterns"], 1);
    assert_eq!(document["range_patterns"], 1);
    assert_eq!(document["exhaustive_matches"], 5);
    assert_eq!(document["closures"], 1);
    assert_eq!(document["async_closures"], 1);
    assert_eq!(document["captures"], 1);
    assert_eq!(document["ref_captures"], 1);
    assert_eq!(document["awaits"], 1);
    assert_eq!(document["yields"], 1);
    assert_eq!(document["spawns"], 1);
    assert_eq!(document["defers"], 1);
    assert_eq!(document["try_propagations"], 1);
    assert_eq!(document["kernel_hidden_allocations"], 0);
    assert_eq!(document["kernel_hidden_schedulers"], 0);
    assert_eq!(document["fingerprint"], 16_557_412);

    for (fixture, code) in [
        (
            "sema_match_non_exhaustive.n",
            "E_HIGH_MATCH_NON_EXHAUSTIVE_001",
        ),
        (
            "sema_match_unreachable.n",
            "E_HIGH_MATCH_UNREACHABLE_ARM_001",
        ),
        ("sema_match_guard_type.n", "E_HIGH_GUARD_TYPE_001"),
        (
            "sema_pattern_range_empty.n",
            "E_HIGH_PATTERN_RANGE_EMPTY_001",
        ),
        ("sema_capture_duplicate.n", "E_HIGH_CAPTURE_ALIAS_001"),
        ("sema_capture_unknown.n", "E_HIGH_CAPTURE_UNKNOWN_001"),
        (
            "sema_capture_mut_required.n",
            "E_HIGH_CAPTURE_MUT_REQUIRED_001",
        ),
        ("sema_capture_moved.n", "E_HIGH_CAPTURE_MOVED_001"),
        ("sema_defer_control_flow.n", "E_HIGH_DEFER_CONTROL_FLOW_001"),
        ("sema_await_context.n", "E_HIGH_AWAIT_CONTEXT_001"),
        ("sema_await_non_future.n", "E_HIGH_AWAIT_NON_FUTURE_001"),
        ("sema_yield_context.n", "E_HIGH_YIELD_CONTEXT_001"),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let rejected = run(
            &native,
            &[
                "sema",
                path.to_str()
                    .expect("UTF-8 rejected pattern/async/resource fixture"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_ir_materializes_typed_graph_and_all_declared_target_contracts() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native IR execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture_text = fs::read_to_string(&fixture).expect("read native IR fixture");
    let ast = ncc_parser::parse(ncc_lexer::lex(&fixture_text).expect("IR fixture lex"))
        .expect("IR fixture parse");
    ncc_sema::analyze(&ast).expect("Rust Stage0 accepts IR fixture");
    let source_functions = ast
        .containers
        .iter()
        .flat_map(|container| container.sections.iter())
        .map(|section| match section {
            ncc_ast::Section::Fn(functions)
            | ncc_ast::Section::MoveFn(functions)
            | ncc_ast::Section::ConstFn(functions) => functions.len(),
            _ => 0,
        })
        .sum::<usize>();
    let stage0 =
        ncc_ir::lower_with_backend_profile(ast, ncc_ir::BackendLoweringProfileIR::nc_sysv64());
    let stage0_functions = stage0
        .containers
        .iter()
        .filter_map(|container| match container {
            ncc_ir::ContainerIR::Regular(regular) => Some(regular.functions.len()),
            _ => None,
        })
        .sum::<usize>();

    let emitted = run(
        &native,
        &[
            "compiler",
            "emit-ir",
            fixture.to_str().expect("UTF-8 IR fixture"),
            "--target",
            "x86_64-unknown-linux-gnu",
            "--json",
        ],
    );
    assert!(
        emitted.status.success(),
        "native IR authority failed: {}",
        String::from_utf8_lossy(&emitted.stdout)
    );
    let document: Value = serde_json::from_slice(&emitted.stdout).expect("native IR JSON");
    assert_eq!(document["schema"], "nc.bootstrap.ir.material.v2");
    assert_eq!(document["target"], "x86_64-unknown-linux-gnu");
    assert_eq!(document["pointer_bits"], 64);
    assert_eq!(document["object"], "elf64");
    assert_eq!(document["abi"], "sysv64");
    assert_eq!(document["header_bytes"], 128);
    assert_eq!(document["record_bytes"], 64);
    assert_eq!(document["identity_record_bytes"], 24);
    assert_eq!(document["spelling_bytes"], 1353);
    assert_eq!(document["nodes"], 91);
    assert_eq!(document["functions"], source_functions);
    assert!(stage0_functions >= source_functions);
    assert_eq!(document["operations"], 64);
    assert_eq!(document["terminators"], 4);
    assert_eq!(document["memory_operations"], 14);
    assert_eq!(document["machine_seed"], serde_json::json!([49, 192, 195]));
    assert_eq!(document["material_bytes"], 9489);
    assert_eq!(document["fingerprint"], 671_657_574);

    for (target, target_id, bits, object, abi, seed_bytes) in [
        ("x86_64-unknown-linux-gnu", "1", "64", "1", "1", "3"),
        ("x86_64-pc-windows-msvc", "2", "64", "3", "2", "3"),
        ("x86_64-apple-darwin", "3", "64", "4", "1", "3"),
        ("i386-unknown-linux-gnu", "4", "32", "2", "4", "3"),
        ("aarch64-unknown-linux-gnu", "5", "64", "1", "3", "8"),
        ("aarch64-apple-darwin", "6", "64", "4", "3", "8"),
        ("armv7-unknown-linux-gnueabihf", "7", "32", "2", "5", "8"),
        ("riscv32imac-unknown-elf", "8", "32", "2", "6", "8"),
        ("riscv64gc-unknown-linux-gnu", "9", "64", "1", "7", "8"),
        ("wasm32-wasi", "10", "32", "5", "8", "8"),
    ] {
        let checked = run(
            &native,
            &[
                "compiler",
                "ir-check",
                fixture.to_str().expect("UTF-8 IR fixture"),
                target,
                target_id,
                bits,
                object,
                abi,
                seed_bytes,
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }

    let rejected = run(
        &native,
        &[
            "compiler",
            "emit-ir",
            fixture.to_str().expect("UTF-8 IR fixture"),
            "--target",
            "m68k-unknown-none",
            "--json",
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("E_IR_TARGET_UNSUPPORTED_001"));
}

#[test]
fn native_nc_mct_materializes_instruction_bytes_relocations_and_binary_layout() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native MCT execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 MCT fixture");
    for (target, family, bits, object, abi, text_bytes, fingerprint, prefix) in [
        (
            "x86_64-unknown-linux-gnu",
            1,
            64,
            "elf64",
            "sysv64",
            270,
            208_070_673,
            &[72, 131, 236, 8, 184, 4, 0, 0][..],
        ),
        (
            "x86_64-pc-windows-msvc",
            1,
            64,
            "coff64",
            "win64",
            270,
            208_257_237,
            &[72, 131, 236, 8, 184, 4, 0, 0][..],
        ),
        (
            "x86_64-apple-darwin",
            1,
            64,
            "macho64",
            "sysv64",
            270,
            208_073_694,
            &[72, 131, 236, 8, 184, 4, 0, 0][..],
        ),
        (
            "i386-unknown-linux-gnu",
            2,
            32,
            "elf32",
            "sysv32",
            231,
            226_064_657,
            &[131, 236, 4, 184, 4, 0, 0, 0][..],
        ),
        (
            "aarch64-unknown-linux-gnu",
            3,
            64,
            "elf64",
            "aapcs64",
            304,
            221_029_965,
            &[255, 67, 0, 209, 128, 0, 128, 210][..],
        ),
        (
            "aarch64-apple-darwin",
            3,
            64,
            "macho64",
            "aapcs64",
            304,
            221_032_476,
            &[255, 67, 0, 209, 128, 0, 128, 210][..],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            4,
            32,
            "elf32",
            "aapcs32",
            288,
            231_951_623,
            &[8, 208, 77, 226, 4, 0, 0, 227][..],
        ),
        (
            "riscv32imac-unknown-elf",
            5,
            32,
            "elf32",
            "riscv32",
            328,
            204_277_052,
            &[19, 1, 1, 255, 19, 5, 64, 0][..],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            6,
            64,
            "elf64",
            "riscv64",
            328,
            205_824_652,
            &[19, 1, 1, 255, 19, 5, 64, 0][..],
        ),
        (
            "wasm32-wasi",
            7,
            32,
            "wasm",
            "wasm",
            156,
            227_860_321,
            &[1, 66, 4, 1, 1, 1, 66, 0][..],
        ),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler", "emit-mct", fixture, "--target", target, "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value = serde_json::from_slice(&emitted.stdout).expect("native MCT JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.material.v2");
        assert_eq!(document["target"], target);
        assert_eq!(document["family_id"], family);
        assert_eq!(document["pointer_bits"], bits);
        assert_eq!(document["object"], object);
        assert_eq!(document["abi"], abi);
        assert_eq!(document["header_bytes"], 160);
        assert_eq!(document["record_bytes"], 72);
        assert_eq!(document["identity_record_bytes"], 32);
        assert_eq!(document["source_map_record_bytes"], 8);
        assert_eq!(document["source_map_bytes"], 728);
        assert_eq!(document["source_map_lines"], 64);
        assert_eq!(document["identity_bytes"], 64);
        assert_eq!(document["identity_fingerprint"], 463_359);
        assert_eq!(document["ir_nodes"], 91);
        assert_eq!(document["instruction_records"], 64);
        assert_eq!(document["relocations"], 4);
        assert_eq!(document["targets"], 6);
        assert_eq!(document["local_targets"], 4);
        assert_eq!(document["external_targets"], 0);
        assert_eq!(document["control_targets"], 2);
        assert_eq!(document["unresolved_targets"], 0);
        assert_eq!(document["text_bytes"], text_bytes);
        assert_eq!(document["fingerprint"], fingerprint);
        let actual_prefix = document["text_prefix"]
            .as_array()
            .expect("MCT byte prefix")
            .iter()
            .take(prefix.len())
            .map(|byte| byte.as_u64().expect("MCT byte") as u8)
            .collect::<Vec<_>>();
        assert_eq!(actual_prefix, prefix, "{target} byte prefix");
    }

    let directory = root().join("target/tmp/ncc-stage2-mct-differential");
    fs::create_dir_all(&directory).expect("create MCT differential directory");
    let artifact = directory.join("x86_64.mct");
    let emitted = run(
        &native,
        &[
            "emit-mct",
            fixture,
            "--target",
            "x86_64-unknown-linux-gnu",
            "--out",
            artifact.to_str().expect("UTF-8 MCT artifact"),
            "--quiet",
        ],
    );
    assert!(emitted.status.success());
    let bytes = fs::read(&artifact).expect("read material MCT artifact");
    let word =
        |offset: usize| i32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("MCT word"));
    assert_eq!(bytes.len(), 10686);
    assert_eq!(word(0), 1_413_698_382);
    assert_eq!(word(4), 2);
    assert_eq!(word(8), 1);
    assert_eq!(word(12), 1);
    assert_eq!(word(28), 91);
    assert_eq!(word(32), 72);
    assert_eq!(word(36), 64);
    assert_eq!(word(40), 270);
    assert_eq!(word(44), 4);
    assert_eq!(word(88), 160);
    assert_eq!(word(92), 10416);
    assert_eq!(word(96), bytes.len() as i32);
    assert_eq!(word(100), 6);
    assert_eq!(word(104), 4);
    assert_eq!(word(108), 0);
    assert_eq!(word(112), 2);
    assert_eq!(word(116), 0);
    assert_eq!(word(120), 160 + 91 * 72);
    assert_eq!(word(124), 160 + 91 * 72 + 91 * 32);
    assert_eq!(word(128), 64);
    assert_eq!(word(132), 463_359);
    assert_eq!(word(136), 9688);
    assert_eq!(word(140), 728);
    assert_eq!(word(144), 8);
    assert_eq!(word(148), 64);
    assert_eq!(
        &bytes[word(92) as usize..word(92) as usize + 8],
        &[72, 131, 236, 8, 184, 4, 0, 0]
    );

    let rejected = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture,
            "--target",
            "m68k-unknown-none",
            "--json",
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("E_IR_TARGET_UNSUPPORTED_001"));
}

#[test]
fn native_nc_mct_materializes_scalar_immediates_for_every_target_family() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native scalar immediate execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_scalar_immediates.n");
    for (target, prefix) in [
        (
            "x86_64-unknown-linux-gnu",
            &[85, 72, 137, 229, 72, 131, 236, 16, 184, 120, 86, 52, 18][..],
        ),
        (
            "i386-unknown-linux-gnu",
            &[85, 137, 229, 131, 236, 4, 184, 120, 86, 52, 18][..],
        ),
        (
            "aarch64-unknown-linux-gnu",
            &[
                253, 123, 191, 169, 253, 3, 0, 145, 0, 207, 138, 210, 128, 70, 162, 242,
            ][..],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            &[
                0, 72, 45, 233, 0, 176, 141, 226, 120, 6, 5, 227, 52, 2, 65, 227,
            ][..],
        ),
        (
            "riscv32imac-unknown-elf",
            &[
                19, 1, 1, 255, 35, 38, 17, 0, 55, 85, 52, 18, 19, 5, 133, 103,
            ][..],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            &[
                19, 1, 1, 255, 35, 52, 17, 0, 55, 85, 52, 18, 19, 5, 133, 103,
            ][..],
        ),
        ("wasm32-wasi", &[1, 66, 248, 172, 209, 145, 1, 15][..]),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 scalar immediate fixture"),
                "--target",
                target,
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native scalar MCT JSON");
        let actual = document["text_prefix"]
            .as_array()
            .expect("scalar text prefix")
            .iter()
            .take(prefix.len())
            .map(|byte| byte.as_u64().expect("scalar byte") as u8)
            .collect::<Vec<_>>();
        assert_eq!(actual, prefix, "{target} 0x12345678 materialization");
        assert_eq!(
            document["intrinsic_targets_pending_selection"], 0,
            "{target}"
        );
    }
}

#[test]
fn native_nc_mct_preserves_exact_relocation_destination_identity() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native MCT target execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_link_targets.n");
    let fixture = fixture.to_str().expect("UTF-8 MCT target fixture");
    for (target, fingerprint) in [
        ("x86_64-unknown-linux-gnu", "341668409"),
        ("x86_64-pc-windows-msvc", "310838992"),
        ("x86_64-apple-darwin", "341671430"),
        ("i386-unknown-linux-gnu", "315331175"),
        ("aarch64-unknown-linux-gnu", "328783755"),
        ("aarch64-apple-darwin", "328786266"),
        ("armv7-unknown-linux-gnueabihf", "304341863"),
        ("riscv32imac-unknown-elf", "336951588"),
        ("riscv64gc-unknown-linux-gnu", "338785562"),
        ("wasm32-wasi", "331724983"),
    ] {
        let checked = run(
            &native,
            &[
                "compiler",
                "mct-target-check",
                fixture,
                target,
                "6",
                "4",
                "1",
                "1",
                "0",
                fingerprint,
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }
}

#[test]
fn native_nc_mct_classifies_link_targets_before_final_selection() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native MCT link-readiness execution: no host linker");
        return;
    };

    for (fixture, intrinsic, selected, runtime, project, ready) in [
        (
            "bootstrap/corpus/ncc-stage2-lexer/mct_link_readiness_raw.n",
            0,
            4,
            3,
            0,
            true,
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/mct_link_targets.n",
            0,
            0,
            1,
            0,
            true,
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/mct_pointer_inference.n",
            0,
            11,
            4,
            0,
            true,
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/mct_memory_layout_intrinsics.n",
            0,
            4,
            4,
            0,
            true,
        ),
    ] {
        let report = run(
            &native,
            &[
                "compiler",
                "mct-link-readiness",
                fixture,
                "--target",
                "x86_64-pc-windows-msvc",
                "--json",
            ],
        );
        assert!(
            report.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&report.stdout)
        );
        let document: Value =
            serde_json::from_slice(&report.stdout).expect("native MCT readiness JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.link-readiness.v1");
        assert_eq!(document["intrinsic_targets_pending_selection"], intrinsic);
        assert_eq!(document["selected_intrinsics"], selected);
        assert_eq!(document["runtime_import_targets"], runtime);
        assert_eq!(document["project_external_targets"], project);
        assert_eq!(document["unresolved_targets"], 0);
        assert_eq!(document["ready"], ready);

        let intrinsic = intrinsic.to_string();
        let runtime = runtime.to_string();
        let project = project.to_string();
        let checked = run(
            &native,
            &[
                "compiler",
                "mct-link-readiness-check",
                fixture,
                "x86_64-pc-windows-msvc",
                &intrinsic,
                &runtime,
                &project,
                "0",
            ],
        );
        assert!(
            checked.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }
}

#[test]
fn native_nc_mct_selects_all_typed_pointer_intrinsics_for_every_target() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native pointer intrinsic execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_pointer_intrinsics.n");
    let directory = root().join("target/tmp/ncc-stage2-pointer-intrinsics");
    fs::create_dir_all(&directory).expect("create pointer intrinsic directory");
    let targets: &[(&str, &[(i32, &str)])] = &[
        (
            "x86_64-unknown-linux-gnu",
            &[
                (1, "8B07"),
                (2, "8937B801000000"),
                (3, "488D04B7"),
                (4, "4889F84829F048C1F802"),
                (5, "4885FF0F94C00FB6C0"),
                (6, "4889F8"),
                (7, "4889F8"),
                (8, "4889F8"),
            ],
        ),
        (
            "x86_64-pc-windows-msvc",
            &[
                (1, "8B01"),
                (2, "8911B801000000"),
                (3, "488D0491"),
                (4, "4889C84829D048C1F802"),
                (5, "4885C90F94C00FB6C0"),
                (6, "4889C8"),
                (7, "4889C8"),
                (8, "4889C8"),
            ],
        ),
        (
            "i386-unknown-linux-gnu",
            &[
                (1, "8B4424048B00"),
                (2, "8B4424048B5424088910B801000000"),
                (3, "8B4424048B5424088D0490"),
                (4, "8B4424048B54240829D0C1F802"),
                (5, "8B44240485C00F94C00FB6C0"),
                (6, "8B442404"),
                (7, "8B442404"),
                (8, "8B442404"),
            ],
        ),
        (
            "aarch64-unknown-linux-gnu",
            &[
                (1, "000040B9"),
                (2, "010000B920008052"),
                (3, "0008018B"),
                (4, "000001CB00FC4293"),
                (5, "1F0000F1E0179F1A"),
                (6, "E00300AA"),
                (7, "E00300AA"),
                (8, "E00300AA"),
            ],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            &[
                (1, "000090E5"),
                (2, "001080E50100A0E3"),
                (3, "010180E0"),
                (4, "010040E04001A0E1"),
                (5, "000050E30100A0030000A013"),
                (6, "0000A0E1"),
                (7, "0000A0E1"),
                (8, "0000A0E1"),
            ],
        ),
        (
            "riscv32imac-unknown-elf",
            &[
                (1, "03250500"),
                (2, "2320B50013051000"),
                (3, "939525003305B500"),
                (4, "3305B54013552540"),
                (5, "13351500"),
                (6, "13050500"),
                (7, "13050500"),
                (8, "13050500"),
            ],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            &[
                (1, "03250500"),
                (2, "2320B50013051000"),
                (3, "939525003305B500"),
                (4, "3305B54013552540"),
                (5, "13351500"),
                (6, "13050500"),
                (7, "13050500"),
                (8, "13050500"),
            ],
        ),
        (
            "wasm32-wasi",
            &[
                (1, "2000280000"),
                (2, "200020013600004101"),
                (3, "2000200141046C6A"),
                (4, "200020016B41046D"),
                (5, "200045"),
                (6, "2000"),
                (7, "2000"),
                (8, "2000"),
            ],
        ),
    ];

    for (target, expected) in targets {
        let artifact = directory.join(format!("{}.ncmct", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 pointer fixture"),
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 pointer artifact"),
                "--quiet",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let bytes = fs::read(&artifact).expect("read pointer MCT artifact");
        let word = |offset: usize| {
            i32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("MCT word"))
        };
        let records = word(28) as usize;
        let text = word(92) as usize;
        assert_eq!(word(152), 0, "{target} pending pointer intrinsic");
        assert_eq!(word(156), 0, "{target} unexpected runtime import");

        let mut selected = BTreeMap::new();
        for index in 0..records {
            let row = 160 + index * 72;
            if word(row + 4) != 20 {
                continue;
            }
            let offset = word(row + 12) as usize;
            let length = word(row + 16) as usize;
            let id = (word(row + 44) / 2_097_152) % 128;
            assert_eq!(word(row + 20), 0, "{target} intrinsic relocation");
            assert_eq!(word(row + 64), 0, "{target} intrinsic target kind");
            selected.insert(id, bytes[text + offset..text + offset + length].to_vec());
        }
        assert_eq!(selected.len(), 8, "{target} selected pointer family");
        for (id, hex) in *expected {
            let expected_bytes = hex
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                selected.get(id),
                Some(&expected_bytes),
                "{target} intrinsic {id}"
            );
        }
    }
}

#[test]
fn native_nc_mct_materializes_copy_and_scalar_size_for_every_target() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native memory/layout intrinsic execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_memory_layout_intrinsics.n");
    let directory = root().join("target/tmp/ncc-stage2-memory-layout-intrinsics");
    fs::create_dir_all(&directory).expect("create memory/layout intrinsic directory");
    let targets = [
        ("x86_64-unknown-linux-gnu", "4889D1F3A4", "B804000000"),
        (
            "x86_64-pc-windows-msvc",
            "57564889CF4889D64C89C1F3A45E5F",
            "B804000000",
        ),
        (
            "i386-unknown-linux-gnu",
            "57568B7C240C8B7424108B4C2414F3A45E5F",
            "B804000000",
        ),
        (
            "aarch64-unknown-linux-gnu",
            "A20000B42314403803140038420400F1A1FFFF54",
            "80008052",
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            "000052E30300000A0130D1E40130C0E4012052E2FBFFFF1A",
            "0400A0E3",
        ),
        (
            "riscv32imac-unknown-elf",
            "630E060083C205002300550093851500130515001306F6FFE31606FE",
            "13054000",
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            "630E060083C205002300550093851500130515001306F6FFE31606FE",
            "13054000",
        ),
        ("wasm32-wasi", "200020012002FC0A0000", "4104"),
    ];

    let decode = |hex: &str| {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };

    for (target, copy_hex, size_hex) in targets {
        let artifact = directory.join(format!("{}.ncmct", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 memory/layout fixture"),
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 memory/layout artifact"),
                "--quiet",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: stdout={} stderr={}",
            String::from_utf8_lossy(&emitted.stdout),
            String::from_utf8_lossy(&emitted.stderr)
        );

        let bytes = fs::read(&artifact).expect("read memory/layout MCT artifact");
        let word = |offset: usize| {
            i32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("MCT word"))
        };
        assert_eq!(word(152), 0, "{target} pending memory/layout intrinsic");
        assert_eq!(word(156), 4, "{target} runtime imports");
        let records = word(28) as usize;
        let text = word(92) as usize;
        let mut selected = BTreeMap::new();
        for index in 0..records {
            let row = 160 + index * 72;
            if word(row + 4) != 20 {
                continue;
            }
            let offset = word(row + 12) as usize;
            let length = word(row + 16) as usize;
            let id = (word(row + 44) / 2_097_152) % 128;
            assert_eq!(word(row + 20), 0, "{target} intrinsic relocation");
            assert_eq!(word(row + 64), 0, "{target} intrinsic target kind");
            selected.insert(id, bytes[text + offset..text + offset + length].to_vec());
        }
        assert_eq!(selected.get(&20), Some(&decode(copy_hex)), "{target} cp");
        assert_eq!(
            selected.get(&24),
            Some(&decode(size_hex)),
            "{target} sizeOf(int)"
        );
    }
}

#[test]
fn native_nc_mct_serializes_portable_debug_and_unwind_for_all_targets() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native debug/unwind execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 debug/unwind fixture");

    for (target, family, pointer_bits, stack_alignment, cfa_register, text_bytes, hash) in [
        ("x86_64-unknown-linux-gnu", 1, 64, 16, 7, 270, 63_054_075),
        ("x86_64-pc-windows-msvc", 1, 64, 16, 7, 270, 63_054_084),
        ("x86_64-apple-darwin", 1, 64, 16, 7, 270, 63_054_093),
        ("i386-unknown-linux-gnu", 2, 32, 16, 4, 231, 62_956_193),
        ("aarch64-unknown-linux-gnu", 3, 64, 16, 31, 304, 63_129_314),
        ("aarch64-apple-darwin", 3, 64, 16, 31, 304, 63_129_323),
        (
            "armv7-unknown-linux-gnueabihf",
            4,
            32,
            8,
            13,
            288,
            63_116_557,
        ),
        ("riscv32imac-unknown-elf", 5, 32, 16, 2, 328, 63_138_277),
        ("riscv64gc-unknown-linux-gnu", 6, 64, 16, 2, 328, 63_138_843),
        ("wasm32-wasi", 7, 32, 1, 0, 156, 62_646_660),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-debug-unwind",
                fixture,
                "--target",
                target,
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native debug/unwind JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.debug-unwind.v1");
        assert_eq!(document["target"], target);
        assert_eq!(document["family"], family);
        assert_eq!(document["pointer_bits"], pointer_bits);
        assert_eq!(document["line_records"], 64);
        assert_eq!(document["function_records"], 2);
        assert_eq!(document["line_record_bytes"], 20);
        assert_eq!(document["function_record_bytes"], 32);
        assert_eq!(document["stack_alignment"], stack_alignment);
        assert_eq!(document["cfa_register"], cfa_register);
        assert_eq!(document["text_bytes"], text_bytes);
        assert_eq!(document["bytes"], 1408);
        assert_eq!(document["hash"], hash);

        let checked = run(
            &native,
            &[
                "compiler",
                "debug-unwind-check",
                fixture,
                target,
                "64",
                "2",
                "1408",
                &hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }
}

#[test]
fn native_nc_mct_writes_platform_debug_records_for_all_declared_targets() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native platform debug execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 platform debug fixture");
    for (target, format, abbrev, info, line, codeview, bytes, hash) in [
        (
            "x86_64-unknown-linux-gnu",
            1,
            33,
            112,
            414,
            0,
            628,
            57_866_030,
        ),
        ("x86_64-pc-windows-msvc", 2, 0, 0, 0, 76, 140, 50_597_677),
        ("x86_64-apple-darwin", 1, 33, 112, 414, 0, 628, 57_866_048),
        (
            "i386-unknown-linux-gnu",
            1,
            33,
            100,
            410,
            0,
            612,
            57_579_984,
        ),
        (
            "aarch64-unknown-linux-gnu",
            1,
            33,
            112,
            414,
            0,
            628,
            57_884_584,
        ),
        ("aarch64-apple-darwin", 1, 33, 112, 414, 0, 628, 57_884_593),
        (
            "armv7-unknown-linux-gnueabihf",
            1,
            33,
            100,
            410,
            0,
            612,
            57_568_038,
        ),
        (
            "riscv32imac-unknown-elf",
            1,
            33,
            100,
            410,
            0,
            612,
            57_537_772,
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            1,
            33,
            112,
            414,
            0,
            628,
            57_843_269,
        ),
        ("wasm32-wasi", 1, 33, 100, 409, 0, 612, 57_537_681),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-platform-debug",
                fixture,
                "--target",
                target,
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native platform debug JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.platform-debug.v1");
        assert_eq!(document["target"], target);
        assert_eq!(document["format"], format);
        assert_eq!(document["abbrev_bytes"], abbrev);
        assert_eq!(document["info_bytes"], info);
        assert_eq!(document["line_bytes"], line);
        assert_eq!(document["codeview_bytes"], codeview);
        assert_eq!(document["line_records"], 64);
        assert_eq!(document["function_records"], 2);
        assert_eq!(document["bytes"], bytes);
        assert_eq!(document["hash"], hash);

        let checked = run(
            &native,
            &[
                "compiler",
                "platform-debug-check",
                fixture,
                target,
                &hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }
}

#[test]
fn native_nc_mct_materializes_every_llvm_optimization_profile() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native LLVM optimization policy: no host linker");
        return;
    };
    let directory = root().join("target/tmp/ncc-stage2-llvm-optimization");
    fs::create_dir_all(&directory).expect("create LLVM optimization directory");

    for (level, profile, passes, bytes, hash, vectorization, lto, pgo) in [
        ("O0", "debug", 10, 224, 570_959, false, false, false),
        ("O1", "release-light", 15, 304, 940_367, false, false, false),
        ("O2", "release", 25, 464, 2_064_918, true, true, true),
        (
            "O3",
            "release-aggressive",
            27,
            496,
            2_354_302,
            true,
            true,
            true,
        ),
        ("Os", "release-size", 25, 464, 2_065_529, false, true, true),
        ("Oz", "release-tiny", 24, 448, 1_927_672, false, true, true),
    ] {
        let artifact = directory.join(format!("{level}.ncop"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-llvm-optimization",
                "--opt-level",
                level,
                "--out",
                artifact.to_str().expect("UTF-8 optimization artifact"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{level}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native optimization JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.llvm-optimization.v1");
        assert_eq!(document["level"], level);
        assert_eq!(document["profile"], profile);
        assert_eq!(document["pass_count"], passes);
        assert_eq!(document["record_bytes"], 16);
        assert_eq!(document["raw_may_alias_default"], true);
        assert_eq!(document["implicit_tbaa_noalias"], false);
        assert_eq!(document["observable_memory_preserved"], true);
        assert_eq!(document["vectorization"], vectorization);
        assert_eq!(document["lto"], lto);
        assert_eq!(document["pgo"], pgo);
        assert_eq!(document["debug_preserved"], true);
        assert_eq!(document["bytes"], bytes);
        assert_eq!(document["hash"], hash);

        let material = fs::read(&artifact).expect("read optimization artifact");
        assert_eq!(material.len(), bytes);
        assert_eq!(&material[..4], b"NCOP");

        let checked = run(
            &native,
            &[
                "compiler",
                "llvm-optimization-check",
                level,
                &passes.to_string(),
                &bytes.to_string(),
                &hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "optimization check failed for {level}"
        );
    }
}

#[test]
fn native_nc_mct_applies_material_cfg_transforms_before_instruction_selection() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native LLVM CFG transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_transform_cfg.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-transform");
    fs::create_dir_all(&directory).expect("create LLVM transform directory");
    let artifact = directory.join("cfg.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 transform fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            artifact.to_str().expect("UTF-8 transform artifact"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("native LLVM transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["level"], "O2");
    assert_eq!(transform["nodes"], 43);
    assert_eq!(transform["eliminated"], 23);
    assert_eq!(transform["constant_branches"], 2);
    assert_eq!(transform["dead_cfg_nodes"], 3);
    assert_eq!(transform["before_fingerprint"], 371_396_407);
    assert_eq!(transform["after_fingerprint"], 371_397_482);
    assert_eq!(transform["raw_may_alias"], true);
    assert_eq!(transform["observable_memory_preserved"], true);
    assert_eq!(transform["ssa_propagations"], 0);
    assert_eq!(transform["dead_scalar_writes"], 0);
    assert_eq!(transform["dead_functions"], 0);
    assert_eq!(transform["local_call_edges"], 0);
    assert_eq!(transform["loop_collapses"], 0);
    assert_eq!(transform["zero_trip_loops"], 1);
    assert_eq!(transform["strength_reductions"], 0);
    assert_eq!(transform["induction_variables"], 0);
    assert_eq!(transform["loop_invariants"], 0);
    assert_eq!(transform["derived_inductions"], 0);
    assert_eq!(transform["inlined_calls"], 0);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 1_136);
    assert_eq!(transform["hash"], 11_774_571);
    let material = fs::read(&artifact).expect("read LLVM transform artifact");
    assert_eq!(material.len(), 1_136);
    assert_eq!(&material[..4], b"NCOT");

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 transform fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "23",
            "2",
            "3",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "11774571",
        ],
    );
    assert!(checked.status.success());

    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 transform fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline: Value = serde_json::from_slice(&baseline.stdout).expect("baseline MCT JSON");
    assert_eq!(baseline["instruction_records"], 33);
    assert_eq!(baseline["text_bytes"], 151);
    assert_eq!(baseline["relocations"], 4);
    assert_eq!(baseline["branches"], 3);
    assert_eq!(baseline["ir_fingerprint"], 371_396_407);

    for target in [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "i386-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "armv7-unknown-linux-gnueabihf",
        "riscv32imac-unknown-elf",
        "riscv64gc-unknown-linux-gnu",
        "wasm32-wasi",
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 transform fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let mct: Value = serde_json::from_slice(&emitted.stdout).expect("optimized MCT JSON");
        assert_eq!(mct["instruction_records"], 13, "{target}");
        assert_eq!(mct["relocations"], 1, "{target}");
        assert_eq!(mct["branches"], 0, "{target}");
        assert_eq!(mct["stores"], 1, "{target}");
        assert_ne!(mct["ir_fingerprint"], 371_396_407, "{target}");
    }
}

#[test]
fn native_nc_mct_materializes_ssa_and_interprocedural_dce_before_selection() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native LLVM SSA/interprocedural transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_ssa_interprocedural.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-ssa-interprocedural");
    fs::create_dir_all(&directory).expect("create LLVM SSA transform directory");
    let artifact = directory.join("ssa-interprocedural.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 SSA transform fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            artifact.to_str().expect("UTF-8 SSA transform artifact"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("native SSA transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 51);
    assert_eq!(transform["eliminated"], 26);
    assert_eq!(transform["constant_branches"], 1);
    assert_eq!(transform["dead_cfg_nodes"], 3);
    assert_eq!(transform["ssa_propagations"], 1);
    assert_eq!(transform["dead_scalar_writes"], 1);
    assert_eq!(transform["dead_functions"], 2);
    assert_eq!(transform["local_call_edges"], 0);
    assert_eq!(transform["loop_collapses"], 0);
    assert_eq!(transform["zero_trip_loops"], 0);
    assert_eq!(transform["strength_reductions"], 0);
    assert_eq!(transform["induction_variables"], 0);
    assert_eq!(transform["loop_invariants"], 0);
    assert_eq!(transform["derived_inductions"], 0);
    assert_eq!(transform["inlined_calls"], 1);
    assert_eq!(transform["before_fingerprint"], 807_820_069);
    assert_eq!(transform["after_fingerprint"], 807_821_201);
    assert_eq!(transform["raw_may_alias"], true);
    assert_eq!(transform["observable_memory_preserved"], true);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 1_328);
    assert_eq!(transform["hash"], 15_798_917);
    let material = fs::read(&artifact).expect("read SSA transform artifact");
    assert_eq!(material.len(), 1_328);
    assert_eq!(&material[..4], b"NCOT");

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 SSA transform fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "26",
            "1",
            "3",
            "1",
            "1",
            "2",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "15798917",
        ],
    );
    assert!(checked.status.success());

    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 SSA transform fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline: Value = serde_json::from_slice(&baseline.stdout).expect("baseline MCT JSON");
    assert_eq!(baseline["instruction_records"], 36);
    assert_eq!(baseline["text_bytes"], 167);
    assert_eq!(baseline["relocations"], 3);
    assert_eq!(baseline["branches"], 1);
    assert_eq!(baseline["ir_fingerprint"], 807_820_069);

    for target in [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "i386-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "armv7-unknown-linux-gnueabihf",
        "riscv32imac-unknown-elf",
        "riscv64gc-unknown-linux-gnu",
        "wasm32-wasi",
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 SSA transform fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let mct: Value = serde_json::from_slice(&emitted.stdout).expect("optimized MCT JSON");
        assert_eq!(mct["instruction_records"], 16, "{target}");
        assert_eq!(mct["relocations"], 1, "{target}");
        assert_eq!(mct["branches"], 0, "{target}");
        assert_eq!(mct["returns"], 2, "{target}");
        assert_eq!(mct["stores"], 1, "{target}");
        assert_ne!(mct["ir_fingerprint"], 807_820_069, "{target}");
    }
}

#[test]
fn native_nc_mct_eliminates_static_trip_loops_without_dropping_compound_writes() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native static-trip loop transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_static_trip_loops.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-static-trip-loops");
    fs::create_dir_all(&directory).expect("create static-trip loop transform directory");
    let artifact = directory.join("static-trip.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 static-trip fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            artifact.to_str().expect("UTF-8 static-trip artifact"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("static-trip transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 55);
    assert_eq!(transform["eliminated"], 19);
    assert_eq!(transform["constant_branches"], 0);
    assert_eq!(transform["dead_cfg_nodes"], 0);
    assert_eq!(transform["dead_scalar_writes"], 0);
    assert_eq!(transform["loop_collapses"], 4);
    assert_eq!(transform["zero_trip_loops"], 1);
    assert_eq!(transform["strength_reductions"], 0);
    assert_eq!(transform["induction_variables"], 0);
    assert_eq!(transform["loop_invariants"], 0);
    assert_eq!(transform["derived_inductions"], 0);
    assert_eq!(transform["inlined_calls"], 0);
    assert_eq!(transform["before_fingerprint"], 428_982_491);
    assert_eq!(transform["after_fingerprint"], 428_983_540);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 1_424);
    assert_eq!(transform["hash"], 18_225_265);
    assert_eq!(
        fs::read(&artifact).expect("read static-trip NCOT").len(),
        1_424
    );

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 static-trip fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "19",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "4",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "18225265",
        ],
    );
    assert!(checked.status.success());

    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 static-trip fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--json",
        ],
    );
    let baseline: Value = serde_json::from_slice(&baseline.stdout).expect("baseline MCT JSON");
    assert_eq!(baseline["instruction_records"], 43);
    assert_eq!(baseline["text_bytes"], 193);
    assert_eq!(baseline["branches"], 7);
    assert_eq!(baseline["stores"], 7);

    for (target, text_bytes) in [
        ("x86_64-unknown-linux-gnu", 110),
        ("x86_64-pc-windows-msvc", 110),
        ("x86_64-apple-darwin", 110),
        ("i386-unknown-linux-gnu", 93),
        ("aarch64-unknown-linux-gnu", 128),
        ("aarch64-apple-darwin", 128),
        ("armv7-unknown-linux-gnueabihf", 116),
        ("riscv32imac-unknown-elf", 144),
        ("riscv64gc-unknown-linux-gnu", 144),
        ("wasm32-wasi", 60),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 static-trip fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let mct: Value = serde_json::from_slice(&emitted.stdout).expect("optimized MCT JSON");
        assert_eq!(mct["instruction_records"], 25, "{target}");
        assert_eq!(mct["text_bytes"], text_bytes, "{target}");
        assert_eq!(mct["relocations"], 1, "{target}");
        assert_eq!(mct["branches"], 0, "{target}");
        assert_eq!(mct["loads"], 6, "{target}");
        assert_eq!(mct["stores"], 5, "{target}");
    }
}

#[test]
fn native_nc_mct_reduces_strength_and_materializes_induction_metadata() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native induction/strength transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_induction_strength.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-induction-strength");
    fs::create_dir_all(&directory).expect("create induction/strength directory");
    let transform_path = directory.join("induction-strength.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 induction/strength fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            transform_path.to_str().expect("UTF-8 NCOT path"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("induction/strength JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 49);
    assert_eq!(transform["eliminated"], 0);
    assert_eq!(transform["strength_reductions"], 2);
    assert_eq!(transform["induction_variables"], 1);
    assert_eq!(transform["loop_invariants"], 0);
    assert_eq!(transform["derived_inductions"], 0);
    assert_eq!(transform["inlined_calls"], 0);
    assert_eq!(transform["before_fingerprint"], 746_127_814);
    assert_eq!(transform["after_fingerprint"], 746_437_759);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 1_280);
    assert_eq!(transform["hash"], 14_485_973);

    let ncot = fs::read(&transform_path).expect("read induction/strength NCOT");
    assert_eq!(&ncot[..4], b"NCOT");
    assert_eq!(i32le(&ncot, 4), 7);
    assert_eq!(i32le(&ncot, 44), 24);
    assert_eq!(i32le(&ncot, 48), 104);
    assert_eq!(i32le(&ncot, 80), 2);
    assert_eq!(i32le(&ncot, 84), 1);
    assert_eq!(i32le(&ncot, 88), 0);
    assert_eq!(i32le(&ncot, 92), 0);
    assert_eq!(i32le(&ncot, 96), 0);
    assert_eq!(i32le(&ncot, 100), 0);

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 induction/strength fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "2",
            "1",
            "0",
            "0",
            "0",
            "0",
            "14485973",
        ],
    );
    assert!(checked.status.success());

    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 induction/strength fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--json",
        ],
    );
    let baseline: Value =
        serde_json::from_slice(&baseline.stdout).expect("baseline strength MCT JSON");

    let targets: [(&str, usize, &[u8], &[u8]); 10] = [
        (
            "x86_64-unknown-linux-gnu",
            158,
            &[0xc1, 0xe0, 0x03],
            &[0xc1, 0xe8, 0x02],
        ),
        (
            "x86_64-pc-windows-msvc",
            158,
            &[0xc1, 0xe0, 0x03],
            &[0xc1, 0xe8, 0x02],
        ),
        (
            "x86_64-apple-darwin",
            158,
            &[0xc1, 0xe0, 0x03],
            &[0xc1, 0xe8, 0x02],
        ),
        (
            "i386-unknown-linux-gnu",
            136,
            &[0xc1, 0xe0, 0x03],
            &[0xc1, 0xe8, 0x02],
        ),
        (
            "aarch64-unknown-linux-gnu",
            180,
            &[0x00, 0x70, 0x1d, 0x53],
            &[0x00, 0x7c, 0x02, 0x53],
        ),
        (
            "aarch64-apple-darwin",
            180,
            &[0x00, 0x70, 0x1d, 0x53],
            &[0x00, 0x7c, 0x02, 0x53],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            164,
            &[0x80, 0x01, 0xa0, 0xe1],
            &[0x20, 0x01, 0xa0, 0xe1],
        ),
        (
            "riscv32imac-unknown-elf",
            204,
            &[0x13, 0x15, 0x35, 0x00],
            &[0x13, 0x55, 0x25, 0x00],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            204,
            &[0x13, 0x15, 0x35, 0x00],
            &[0x13, 0x55, 0x25, 0x00],
        ),
        ("wasm32-wasi", 89, &[0x41, 0x03, 0x74], &[0x41, 0x02, 0x76]),
    ];
    for (index, (target, expected_text_bytes, shl, shr_u)) in targets.iter().enumerate() {
        let output_path = directory.join(format!("strength-{index}.nbo"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 induction/strength fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                output_path.to_str().expect("UTF-8 MCT path"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("optimized strength MCT JSON");
        assert_eq!(document["instruction_records"], 36, "{target}");
        assert_eq!(document["text_bytes"], *expected_text_bytes, "{target}");
        assert_eq!(document["branches"], 1, "{target}");
        if *target == "x86_64-pc-windows-msvc" {
            assert_ne!(document["text_hash"], baseline["text_hash"]);
            assert_ne!(document["ir_fingerprint"], baseline["ir_fingerprint"]);
        }
        let artifact = fs::read(&output_path).expect("read optimized MCT artifact");
        let text_at = i32le(&artifact, 92) as usize;
        let text_bytes = i32le(&artifact, 40) as usize;
        let machine_text = &artifact[text_at..text_at + text_bytes];
        assert!(
            machine_text.windows(shl.len()).any(|bytes| bytes == *shl),
            "missing shl for {target}"
        );
        assert!(
            machine_text
                .windows(shr_u.len())
                .any(|bytes| bytes == *shr_u),
            "missing shr_u for {target}"
        );
    }
}

#[test]
fn native_nc_mct_hoists_invariants_and_tracks_derived_inductions() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native LICM transform: no host linker");
        return;
    };
    let fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_licm_derived_induction.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-licm-derived");
    fs::create_dir_all(&directory).expect("create LICM directory");
    let transform_path = directory.join("licm-derived.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 LICM fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            transform_path.to_str().expect("UTF-8 LICM NCOT path"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("LICM transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 47);
    assert_eq!(transform["eliminated"], 0);
    assert_eq!(transform["strength_reductions"], 0);
    assert_eq!(transform["induction_variables"], 1);
    assert_eq!(transform["loop_invariants"], 1);
    assert_eq!(transform["derived_inductions"], 1);
    assert_eq!(transform["inlined_calls"], 0);
    assert_eq!(transform["before_fingerprint"], 685_900_103);
    assert_eq!(transform["after_fingerprint"], 686_096_017);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 1_232);
    assert_eq!(transform["hash"], 13_705_510);

    let ncot = fs::read(&transform_path).expect("read LICM NCOT");
    assert_eq!(&ncot[..4], b"NCOT");
    assert_eq!(i32le(&ncot, 4), 7);
    assert_eq!(i32le(&ncot, 48), 104);
    assert_eq!(i32le(&ncot, 88), 1);
    assert_eq!(i32le(&ncot, 92), 1);
    assert_eq!(i32le(&ncot, 96), 0);
    assert_eq!(i32le(&ncot, 100), 0);

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 LICM fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "1",
            "1",
            "0",
            "0",
            "13705510",
        ],
    );
    assert!(checked.status.success());

    let baseline_path = directory.join("licm-derived.O0.nbo");
    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 LICM fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--out",
            baseline_path.to_str().expect("UTF-8 LICM baseline path"),
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline_json: Value =
        serde_json::from_slice(&baseline.stdout).expect("LICM baseline MCT JSON");
    let baseline_artifact = fs::read(&baseline_path).expect("read LICM O0 artifact");
    let record = |bytes: &[u8], node: usize, field: usize| i32le(bytes, 160 + node * 72 + field);
    assert!(record(&baseline_artifact, 24, 12) < record(&baseline_artifact, 33, 12));

    for (index, (target, instruction_bytes)) in [
        ("x86_64-unknown-linux-gnu", 3),
        ("x86_64-pc-windows-msvc", 3),
        ("x86_64-apple-darwin", 3),
        ("i386-unknown-linux-gnu", 3),
        ("aarch64-unknown-linux-gnu", 4),
        ("aarch64-apple-darwin", 4),
        ("armv7-unknown-linux-gnueabihf", 4),
        ("riscv32imac-unknown-elf", 4),
        ("riscv64gc-unknown-linux-gnu", 4),
        ("wasm32-wasi", 3),
    ]
    .iter()
    .enumerate()
    {
        let output_path = directory.join(format!("licm-{index}.nbo"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 LICM fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                output_path.to_str().expect("UTF-8 LICM MCT path"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("optimized LICM MCT JSON");
        assert_eq!(document["instruction_records"], 35, "{target}");
        assert_eq!(document["branches"], 1, "{target}");
        let artifact = fs::read(&output_path).expect("read optimized LICM artifact");
        let loop_offset = record(&artifact, 24, 12);
        let invariant_offset = record(&artifact, 33, 12);
        assert!(invariant_offset < loop_offset, "LICM order for {target}");
        assert_eq!(record(&artifact, 33, 16), *instruction_bytes, "{target}");
        if *target == "x86_64-pc-windows-msvc" {
            assert_ne!(document["text_hash"], baseline_json["text_hash"]);
            assert_eq!(document["text_bytes"], baseline_json["text_bytes"]);
        }
    }
}

#[test]
fn native_nc_mct_inlines_private_pure_leaf_calls_before_selection() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native leaf inlining transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_leaf.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-inline-leaf");
    fs::create_dir_all(&directory).expect("create inline-leaf directory");
    let transform_path = directory.join("inline-leaf.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 inline fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            transform_path.to_str().expect("UTF-8 inline NCOT path"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("inline transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 22);
    assert_eq!(transform["eliminated"], 8);
    assert_eq!(transform["dead_functions"], 1);
    assert_eq!(transform["local_call_edges"], 0);
    assert_eq!(transform["inlined_calls"], 1);
    assert_eq!(transform["before_fingerprint"], 183_023_875);
    assert_eq!(transform["after_fingerprint"], 183_024_104);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 632);
    assert_eq!(transform["hash"], 3_722_596);

    let ncot = fs::read(&transform_path).expect("read inline NCOT");
    assert_eq!(&ncot[..4], b"NCOT");
    assert_eq!(i32le(&ncot, 4), 7);
    assert_eq!(i32le(&ncot, 48), 104);
    assert_eq!(i32le(&ncot, 96), 1);
    assert_eq!(i32le(&ncot, 100), 0);

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 inline fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "8",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "3722596",
        ],
    );
    assert!(checked.status.success());

    let record = |bytes: &[u8], node: usize, field: usize| i32le(bytes, 160 + node * 72 + field);
    let baseline_path = directory.join("inline-leaf.O0.nbo");
    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 inline fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--out",
            baseline_path.to_str().expect("UTF-8 inline baseline path"),
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline_json: Value =
        serde_json::from_slice(&baseline.stdout).expect("inline baseline MCT JSON");
    assert_eq!(baseline_json["instruction_records"], 14);
    assert_eq!(baseline_json["text_bytes"], 72);
    assert_eq!(baseline_json["relocations"], 2);
    let baseline_artifact = fs::read(&baseline_path).expect("read inline O0 artifact");
    assert_eq!(record(&baseline_artifact, 3, 16), 8);
    assert_eq!(record(&baseline_artifact, 19, 16), 5);
    assert_ne!(record(&baseline_artifact, 19, 20), 0);

    for (index, (target, text_bytes, inline_bytes)) in [
        ("x86_64-unknown-linux-gnu", 53, 13),
        ("x86_64-pc-windows-msvc", 53, 13),
        ("x86_64-apple-darwin", 53, 13),
        ("i386-unknown-linux-gnu", 47, 13),
        ("aarch64-unknown-linux-gnu", 68, 12),
        ("aarch64-apple-darwin", 68, 12),
        ("armv7-unknown-linux-gnueabihf", 56, 12),
        ("riscv32imac-unknown-elf", 84, 12),
        ("riscv64gc-unknown-linux-gnu", 84, 12),
        ("wasm32-wasi", 22, 7),
    ]
    .iter()
    .enumerate()
    {
        let output_path = directory.join(format!("inline-{index}.nbo"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 inline fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                output_path.to_str().expect("UTF-8 inline MCT path"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("optimized inline MCT JSON");
        assert_eq!(document["instruction_records"], 8, "{target}");
        assert_eq!(document["text_bytes"], *text_bytes, "{target}");
        assert_eq!(document["relocations"], 1, "{target}");
        assert_eq!(document["local_targets"], 3, "{target}");
        let artifact = fs::read(&output_path).expect("read optimized inline artifact");
        assert_eq!(
            record(&artifact, 3, 16),
            0,
            "dead private leaf for {target}"
        );
        assert_eq!(
            record(&artifact, 19, 16),
            *inline_bytes,
            "inline bytes for {target}"
        );
        assert_eq!(
            record(&artifact, 19, 20),
            0,
            "inline relocation for {target}"
        );
    }
}

#[test]
fn native_nc_mct_substitutes_linear_rematerialized_and_temporary_private_parameters() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native parameter inlining transform: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_parameters.n");
    let rematerialize_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_parameters_guard.n");
    let guard_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_rematerialize_guard.n");
    let temporary_pair_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_temporary_pair.n");
    let effect_guard_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_temporary_effect_guard.n");
    let capacity_guard_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_inline_temporary_capacity_guard.n");
    let directory = root().join("target/tmp/ncc-stage2-llvm-inline-parameters");
    fs::create_dir_all(&directory).expect("create inline-parameters directory");
    let transform_path = directory.join("inline-parameters.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture.to_str().expect("UTF-8 inline-parameters fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            transform_path
                .to_str()
                .expect("UTF-8 inline-parameters NCOT path"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("inline-parameters transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 30);
    assert_eq!(transform["eliminated"], 16);
    assert_eq!(transform["dead_functions"], 1);
    assert_eq!(transform["local_call_edges"], 0);
    assert_eq!(transform["inlined_calls"], 1);
    assert_eq!(transform["before_fingerprint"], 299_937_825);
    assert_eq!(transform["after_fingerprint"], 299_938_342);
    assert_eq!(transform["gvn_reuses"], 0);
    assert_eq!(transform["bytes"], 824);
    assert_eq!(transform["hash"], 6_288_192);

    let ncot = fs::read(&transform_path).expect("read inline-parameters NCOT");
    assert_eq!(&ncot[..4], b"NCOT");
    assert_eq!(i32le(&ncot, 4), 7);
    assert_eq!(i32le(&ncot, 48), 104);
    assert_eq!(i32le(&ncot, 96), 1);
    assert_eq!(i32le(&ncot, 100), 0);

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture.to_str().expect("UTF-8 inline-parameters fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "16",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "6288192",
        ],
    );
    assert!(checked.status.success());

    let record = |bytes: &[u8], node: usize, field: usize| i32le(bytes, 160 + node * 72 + field);
    let record_bytes = |bytes: &[u8], node: usize| {
        let item = 160 + node * 72;
        let text = i32le(bytes, 92) as usize;
        let offset = i32le(bytes, item + 12) as usize;
        let length = i32le(bytes, item + 16) as usize;
        bytes[text + offset..text + offset + length].to_vec()
    };
    let baseline_path = directory.join("inline-parameters.O0.nbo");
    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            fixture.to_str().expect("UTF-8 inline-parameters fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--out",
            baseline_path
                .to_str()
                .expect("UTF-8 inline-parameters baseline path"),
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline_json: Value =
        serde_json::from_slice(&baseline.stdout).expect("inline-parameters baseline MCT JSON");
    assert_eq!(baseline_json["instruction_records"], 20);
    assert_eq!(baseline_json["text_bytes"], 96);
    assert_eq!(baseline_json["relocations"], 2);
    let baseline_artifact = fs::read(&baseline_path).expect("read inline-parameters O0 artifact");
    assert_eq!(record(&baseline_artifact, 3, 16), 8);
    assert_eq!(record(&baseline_artifact, 25, 16), 5);
    assert_ne!(record(&baseline_artifact, 25, 20), 0);
    assert_eq!(record(&baseline_artifact, 26, 16), 5);
    assert_eq!(record(&baseline_artifact, 27, 16), 5);

    for (index, (target, text_bytes, inline_bytes)) in [
        ("x86_64-unknown-linux-gnu", 61, 21),
        ("x86_64-pc-windows-msvc", 61, 21),
        ("x86_64-apple-darwin", 61, 21),
        ("i386-unknown-linux-gnu", 55, 21),
        ("aarch64-unknown-linux-gnu", 76, 20),
        ("aarch64-apple-darwin", 76, 20),
        ("armv7-unknown-linux-gnueabihf", 64, 20),
        ("riscv32imac-unknown-elf", 92, 20),
        ("riscv64gc-unknown-linux-gnu", 92, 20),
        ("wasm32-wasi", 27, 12),
    ]
    .iter()
    .enumerate()
    {
        let output_path = directory.join(format!("inline-parameters-{index}.nbo"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture.to_str().expect("UTF-8 inline-parameters fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                output_path
                    .to_str()
                    .expect("UTF-8 inline-parameters MCT path"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("optimized inline-parameters MCT JSON");
        assert_eq!(document["instruction_records"], 8, "{target}");
        assert_eq!(document["text_bytes"], *text_bytes, "{target}");
        assert_eq!(document["relocations"], 1, "{target}");
        assert_eq!(document["local_targets"], 3, "{target}");
        let artifact = fs::read(&output_path).expect("read optimized inline-parameters artifact");
        assert_eq!(
            record(&artifact, 3, 16),
            0,
            "dead private function for {target}"
        );
        assert_eq!(
            record(&artifact, 25, 16),
            *inline_bytes,
            "inline bytes for {target}"
        );
        assert_eq!(
            record(&artifact, 25, 20),
            0,
            "inline relocation for {target}"
        );
        assert_eq!(
            record(&artifact, 26, 16),
            0,
            "first consumed argument for {target}"
        );
        assert_eq!(
            record(&artifact, 27, 16),
            0,
            "second consumed argument for {target}"
        );
    }

    let rematerialize_transform_path = directory.join("inline-rematerialize.O2.ncot");
    let rematerialized = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            rematerialize_fixture
                .to_str()
                .expect("UTF-8 inline rematerialize fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            rematerialize_transform_path
                .to_str()
                .expect("UTF-8 inline rematerialize NCOT path"),
            "--json",
        ],
    );
    assert!(rematerialized.status.success());
    let rematerialize_transform: Value = serde_json::from_slice(&rematerialized.stdout)
        .expect("inline rematerialize transform JSON");
    assert_eq!(rematerialize_transform["nodes"], 25);
    assert_eq!(rematerialize_transform["eliminated"], 11);
    assert_eq!(rematerialize_transform["dead_functions"], 1);
    assert_eq!(rematerialize_transform["local_call_edges"], 0);
    assert_eq!(rematerialize_transform["inlined_calls"], 1);
    assert_eq!(rematerialize_transform["before_fingerprint"], 236_571_294);
    assert_eq!(rematerialize_transform["after_fingerprint"], 236_571_622);
    assert_eq!(rematerialize_transform["gvn_reuses"], 0);
    assert_eq!(rematerialize_transform["bytes"], 704);
    assert_eq!(rematerialize_transform["hash"], 4_623_881);

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            rematerialize_fixture
                .to_str()
                .expect("UTF-8 inline rematerialize fixture"),
            "x86_64-pc-windows-msvc",
            "O2",
            "11",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "4623881",
        ],
    );
    assert!(checked.status.success());

    let baseline_path = directory.join("inline-rematerialize.O0.nbo");
    let baseline = run(
        &native,
        &[
            "compiler",
            "emit-mct",
            rematerialize_fixture
                .to_str()
                .expect("UTF-8 inline rematerialize fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O0",
            "--out",
            baseline_path
                .to_str()
                .expect("UTF-8 inline rematerialize baseline path"),
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let baseline_json: Value =
        serde_json::from_slice(&baseline.stdout).expect("rematerialize baseline MCT JSON");
    assert_eq!(baseline_json["instruction_records"], 16);
    assert_eq!(baseline_json["text_bytes"], 79);
    assert_eq!(baseline_json["relocations"], 2);
    let baseline_artifact =
        fs::read(&baseline_path).expect("read rematerialize baseline MCT artifact");
    assert_eq!(record(&baseline_artifact, 3, 16), 8);
    assert_eq!(record(&baseline_artifact, 21, 16), 5);
    assert_ne!(record(&baseline_artifact, 21, 20), 0);
    assert_eq!(record(&baseline_artifact, 22, 16), 5);

    for (index, (target, text_bytes, inline_bytes)) in [
        ("x86_64-unknown-linux-gnu", 53, 13),
        ("x86_64-pc-windows-msvc", 53, 13),
        ("x86_64-apple-darwin", 53, 13),
        ("i386-unknown-linux-gnu", 47, 13),
        ("aarch64-unknown-linux-gnu", 68, 12),
        ("aarch64-apple-darwin", 68, 12),
        ("armv7-unknown-linux-gnueabihf", 56, 12),
        ("riscv32imac-unknown-elf", 84, 12),
        ("riscv64gc-unknown-linux-gnu", 84, 12),
        ("wasm32-wasi", 22, 7),
    ]
    .iter()
    .enumerate()
    {
        let output_path = directory.join(format!("inline-rematerialize-{index}.nbo"));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                rematerialize_fixture
                    .to_str()
                    .expect("UTF-8 inline rematerialize fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                output_path
                    .to_str()
                    .expect("UTF-8 inline rematerialize MCT path"),
                "--json",
            ],
        );
        assert!(emitted.status.success(), "{target}");
        let document: Value = serde_json::from_slice(&emitted.stdout)
            .expect("optimized inline rematerialize MCT JSON");
        assert_eq!(document["instruction_records"], 8, "{target}");
        assert_eq!(document["text_bytes"], *text_bytes, "{target}");
        assert_eq!(document["relocations"], 1, "{target}");
        let artifact = fs::read(&output_path).expect("read rematerialized MCT artifact");
        assert_eq!(
            record(&artifact, 3, 16),
            0,
            "dead private function for {target}"
        );
        assert_eq!(
            record(&artifact, 21, 16),
            *inline_bytes,
            "inline bytes for {target}"
        );
        assert_eq!(
            record(&artifact, 21, 20),
            0,
            "inline relocation for {target}"
        );
        assert_eq!(
            record(&artifact, 22, 16),
            0,
            "consumed literal for {target}"
        );
    }

    let guard_transform_path = directory.join("inline-rematerialize-guard.O2.ncot");
    let guarded = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            guard_fixture
                .to_str()
                .expect("UTF-8 inline rematerialize guard fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            guard_transform_path
                .to_str()
                .expect("UTF-8 inline rematerialize guard NCOT path"),
            "--json",
        ],
    );
    assert!(guarded.status.success());
    let guard_transform: Value =
        serde_json::from_slice(&guarded.stdout).expect("inline rematerialize guard transform JSON");
    assert_eq!(guard_transform["nodes"], 28);
    assert_eq!(guard_transform["eliminated"], 11);
    assert_eq!(guard_transform["dead_functions"], 1);
    assert_eq!(guard_transform["local_call_edges"], 0);
    assert_eq!(guard_transform["inlined_calls"], 1);
    assert_eq!(guard_transform["before_fingerprint"], 351_154_031);
    assert_eq!(guard_transform["after_fingerprint"], 351_154_365);
    assert_eq!(guard_transform["gvn_reuses"], 0);
    assert_eq!(guard_transform["bytes"], 776);
    assert_eq!(guard_transform["hash"], 5_562_456);
    let guard_ncot = fs::read(&guard_transform_path).expect("read inline temporary NCOT");
    assert_eq!(i32le(&guard_ncot, 104 + 23 * 24 + 4), 13);
    assert_eq!(i32le(&guard_ncot, 104 + 24 * 24 + 4), 12);
    assert_eq!(i32le(&guard_ncot, 104 + 25 * 24 + 4), 14);

    for (index, (target, text_bytes, inline_bytes, selected)) in [
        (
            "x86_64-unknown-linux-gnu",
            65,
            16,
            &[
                0x48, 0x8B, 0x04, 0x24, 0x49, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x83,
                0xC0, 0x01,
            ][..],
        ),
        (
            "x86_64-pc-windows-msvc",
            65,
            16,
            &[
                0x48, 0x8B, 0x04, 0x24, 0x49, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x83,
                0xC0, 0x01,
            ][..],
        ),
        (
            "x86_64-apple-darwin",
            65,
            16,
            &[
                0x48, 0x8B, 0x04, 0x24, 0x49, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x4C, 0x89, 0xC0, 0x83,
                0xC0, 0x01,
            ][..],
        ),
        (
            "i386-unknown-linux-gnu",
            54,
            12,
            &[
                0x8B, 0x04, 0x24, 0x89, 0xC1, 0x89, 0xC8, 0x89, 0xC8, 0x83, 0xC0, 0x01,
            ][..],
        ),
        (
            "aarch64-unknown-linux-gnu",
            84,
            20,
            &[
                0xE0, 0x03, 0x40, 0xF9, 0xE9, 0x03, 0x00, 0xAA, 0xE0, 0x03, 0x09, 0xAA, 0xE0, 0x03,
                0x09, 0xAA, 0x00, 0x04, 0x00, 0x11,
            ][..],
        ),
        (
            "aarch64-apple-darwin",
            84,
            20,
            &[
                0xE0, 0x03, 0x40, 0xF9, 0xE9, 0x03, 0x00, 0xAA, 0xE0, 0x03, 0x09, 0xAA, 0xE0, 0x03,
                0x09, 0xAA, 0x00, 0x04, 0x00, 0x11,
            ][..],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            72,
            20,
            &[
                0x00, 0x00, 0x9D, 0xE5, 0x00, 0x10, 0xA0, 0xE1, 0x01, 0x00, 0xA0, 0xE1, 0x01, 0x00,
                0xA0, 0xE1, 0x01, 0x00, 0x80, 0xE2,
            ][..],
        ),
        (
            "riscv32imac-unknown-elf",
            100,
            20,
            &[
                0x03, 0x25, 0x01, 0x00, 0x93, 0x02, 0x05, 0x00, 0x13, 0x85, 0x02, 0x00, 0x13, 0x85,
                0x02, 0x00, 0x13, 0x05, 0x15, 0x00,
            ][..],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            100,
            20,
            &[
                0x03, 0x35, 0x01, 0x00, 0x93, 0x02, 0x05, 0x00, 0x13, 0x85, 0x02, 0x00, 0x13, 0x85,
                0x02, 0x00, 0x13, 0x05, 0x15, 0x00,
            ][..],
        ),
        (
            "wasm32-wasi",
            30,
            12,
            &[
                0x28, 0x00, 0x00, 0x21, 0x00, 0x20, 0x00, 0x20, 0x00, 0x41, 0x01, 0x6A,
            ][..],
        ),
    ]
    .iter()
    .enumerate()
    {
        let guard_path = directory.join(format!("inline-temporary-{index}.nbo"));
        let guard_mct = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                guard_fixture
                    .to_str()
                    .expect("UTF-8 inline temporary fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                guard_path
                    .to_str()
                    .expect("UTF-8 inline temporary MCT path"),
                "--json",
            ],
        );
        assert!(guard_mct.status.success(), "{target}");
        let guard_json: Value =
            serde_json::from_slice(&guard_mct.stdout).expect("inline temporary MCT JSON");
        assert_eq!(guard_json["instruction_records"], 10, "{target}");
        assert_eq!(guard_json["text_bytes"], *text_bytes, "{target}");
        assert_eq!(guard_json["relocations"], 1, "{target}");
        let guard_artifact = fs::read(&guard_path).expect("read inline temporary MCT artifact");
        assert_eq!(record(&guard_artifact, 3, 16), 0, "{target}");
        assert_eq!(record(&guard_artifact, 24, 16), *inline_bytes, "{target}");
        assert_eq!(record(&guard_artifact, 24, 20), 0, "{target}");
        assert_eq!(record(&guard_artifact, 25, 16), 0, "{target}");
        assert_eq!(record_bytes(&guard_artifact, 24), *selected, "{target}");
    }

    let pair_transform_path = directory.join("inline-temporary-pair.O2.ncot");
    let paired = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            temporary_pair_fixture
                .to_str()
                .expect("UTF-8 inline temporary pair fixture"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            pair_transform_path
                .to_str()
                .expect("UTF-8 inline temporary pair NCOT path"),
            "--json",
        ],
    );
    assert!(paired.status.success());
    let pair_transform: Value =
        serde_json::from_slice(&paired.stdout).expect("inline temporary pair transform JSON");
    assert_eq!(pair_transform["nodes"], 38);
    assert_eq!(pair_transform["eliminated"], 18);
    assert_eq!(pair_transform["dead_functions"], 1);
    assert_eq!(pair_transform["local_call_edges"], 0);
    assert_eq!(pair_transform["inlined_calls"], 1);
    assert_eq!(pair_transform["before_fingerprint"], 490_580_176);
    assert_eq!(pair_transform["after_fingerprint"], 490_580_796);
    assert_eq!(pair_transform["gvn_reuses"], 0);
    assert_eq!(pair_transform["bytes"], 1_016);
    assert_eq!(pair_transform["hash"], 9_383_348);
    let pair_ncot = fs::read(&pair_transform_path).expect("read inline temporary pair NCOT");
    assert_eq!(i32le(&pair_ncot, 104 + 34 * 24 + 4), 14);
    assert_eq!(i32le(&pair_ncot, 104 + 35 * 24 + 4), 14);

    for (index, (target, text_bytes, inline_bytes)) in [
        ("x86_64-unknown-linux-gnu", 93, 35),
        ("x86_64-pc-windows-msvc", 93, 35),
        ("x86_64-apple-darwin", 93, 35),
        ("i386-unknown-linux-gnu", 77, 27),
        ("aarch64-unknown-linux-gnu", 116, 44),
        ("aarch64-apple-darwin", 116, 44),
        ("armv7-unknown-linux-gnueabihf", 104, 44),
        ("riscv32imac-unknown-elf", 132, 44),
        ("riscv64gc-unknown-linux-gnu", 132, 44),
        ("wasm32-wasi", 48, 27),
    ]
    .iter()
    .enumerate()
    {
        let pair_path = directory.join(format!("inline-temporary-pair-{index}.nbo"));
        let pair_mct = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                temporary_pair_fixture
                    .to_str()
                    .expect("UTF-8 inline temporary pair fixture"),
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                pair_path
                    .to_str()
                    .expect("UTF-8 inline temporary pair MCT path"),
                "--json",
            ],
        );
        assert!(pair_mct.status.success(), "{target}");
        let pair_json: Value =
            serde_json::from_slice(&pair_mct.stdout).expect("inline temporary pair MCT JSON");
        assert_eq!(pair_json["instruction_records"], 12, "{target}");
        assert_eq!(pair_json["text_bytes"], *text_bytes, "{target}");
        assert_eq!(pair_json["relocations"], 1, "{target}");
        let pair_artifact = fs::read(&pair_path).expect("read inline temporary pair MCT");
        assert_eq!(record(&pair_artifact, 33, 16), *inline_bytes, "{target}");
        assert_eq!(record(&pair_artifact, 33, 20), 0, "{target}");
        assert_eq!(record(&pair_artifact, 34, 16), 0, "{target}");
        assert_eq!(record(&pair_artifact, 35, 16), 0, "{target}");
    }

    for (fixture, nodes, edges, hash) in [
        (&effect_guard_fixture, 34, 2, 7_712_138),
        (&capacity_guard_fixture, 48, 1, 14_173_555),
    ] {
        let guarded = run(
            &native,
            &[
                "compiler",
                "emit-llvm-transform",
                fixture
                    .to_str()
                    .expect("UTF-8 inline temporary guard fixture"),
                "--target",
                "x86_64-pc-windows-msvc",
                "--opt-level",
                "O2",
                "--out",
                directory
                    .join(format!("inline-temporary-guard-{nodes}.ncot"))
                    .to_str()
                    .expect("UTF-8 inline temporary guard NCOT path"),
                "--json",
            ],
        );
        assert!(guarded.status.success());
        let document: Value =
            serde_json::from_slice(&guarded.stdout).expect("inline temporary guard transform JSON");
        assert_eq!(document["nodes"], nodes);
        assert_eq!(document["eliminated"], 0);
        assert_eq!(document["dead_functions"], 0);
        assert_eq!(document["local_call_edges"], edges);
        assert_eq!(document["inlined_calls"], 0);
        assert_eq!(document["hash"], hash);
    }
}

#[test]
fn native_nc_mct_reuses_dominating_scalar_values_on_every_target() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native linked-graph GVN: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_llvm_gvn_linked.n");
    let fixture = fixture.to_str().expect("UTF-8 GVN fixture");
    let directory = root().join("target/tmp/ncc-stage2-llvm-linked-gvn");
    fs::create_dir_all(&directory).expect("create linked GVN directory");
    let transform_path = directory.join("linked-gvn.O2.ncot");

    let transformed = run(
        &native,
        &[
            "compiler",
            "emit-llvm-transform",
            fixture,
            "--target",
            "x86_64-pc-windows-msvc",
            "--opt-level",
            "O2",
            "--out",
            transform_path.to_str().expect("UTF-8 GVN transform path"),
            "--json",
        ],
    );
    assert!(
        transformed.status.success(),
        "{}",
        String::from_utf8_lossy(&transformed.stdout)
    );
    let transform: Value =
        serde_json::from_slice(&transformed.stdout).expect("linked GVN transform JSON");
    assert_eq!(transform["schema"], "nc.bootstrap.mct.llvm-transform.v7");
    assert_eq!(transform["nodes"], 36);
    assert_eq!(transform["eliminated"], 4);
    assert_eq!(transform["local_call_edges"], 1);
    assert_eq!(transform["inlined_calls"], 0);
    assert_eq!(transform["gvn_reuses"], 1);
    assert_eq!(transform["before_fingerprint"], 350_457_628);
    assert_eq!(transform["after_fingerprint"], 351_330_866);
    assert_eq!(transform["raw_may_alias"], true);
    assert_eq!(transform["observable_memory_preserved"], true);
    assert_eq!(transform["bytes"], 968);
    assert_eq!(transform["hash"], 8_507_167);

    let ncot = fs::read(&transform_path).expect("read linked GVN NCOT");
    assert_eq!(ncot.len(), 968);
    assert_eq!(&ncot[..4], b"NCOT");
    assert_eq!(i32le(&ncot, 4), 7);
    assert_eq!(i32le(&ncot, 48), 104);
    assert_eq!(i32le(&ncot, 100), 1);
    let actions: Vec<(i32, i32)> = (0..36)
        .map(|node| {
            let record = 104 + node * 24;
            (i32le(&ncot, record + 4), i32le(&ncot, record + 20))
        })
        .collect();
    assert_eq!(
        actions
            .iter()
            .filter(|(action, role)| *action == 17 && *role == 12_288)
            .count(),
        1
    );
    assert_eq!(
        actions
            .iter()
            .filter(|(action, role)| *action == 15 && *role == 12_304)
            .count(),
        1
    );
    assert_eq!(
        actions.iter().filter(|(action, _)| *action == 16).count(),
        4
    );

    let checked = run(
        &native,
        &[
            "compiler",
            "llvm-transform-check",
            fixture,
            "x86_64-pc-windows-msvc",
            "O2",
            "4",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
            "1",
            "8507167",
        ],
    );
    assert!(checked.status.success());

    let targets: [(&str, u64, u64, usize, &str, &[u8], &[u8]); 10] = [
        (
            "x86_64-unknown-linux-gnu",
            119,
            105,
            2_784,
            "62e3a2051030dc7313010c7f2001d7a855dba118a7b43d979a6d0095721d6aa9",
            &[0x49, 0x89, 0xc2],
            &[0x4c, 0x89, 0xd0],
        ),
        (
            "x86_64-pc-windows-msvc",
            119,
            105,
            1_618,
            "cfb663da86ab8928b93eefa0d78a0879d900e35c2d1124e5a2f1971af0ec2d66",
            &[0x49, 0x89, 0xc2],
            &[0x4c, 0x89, 0xd0],
        ),
        (
            "x86_64-apple-darwin",
            119,
            105,
            2_463,
            "65ebbfe73abf89580c8999f8eeebaa766532a1c27eb4ece04de69fcd9e20493f",
            &[0x49, 0x89, 0xc2],
            &[0x4c, 0x89, 0xd0],
        ),
        (
            "i386-unknown-linux-gnu",
            105,
            90,
            2_332,
            "2aa36a5790a45f19e5b3e5e6f066c52ebae33f2e3789489923dd8761e522d61a",
            &[0x89, 0xc1],
            &[0x89, 0xc8],
        ),
        (
            "aarch64-unknown-linux-gnu",
            140,
            128,
            2_816,
            "a4e8bc2c5915894424a6c5c558ccfc03bd2a966b313d8aee16efa1fe44b4688a",
            &[0xe9, 0x03, 0x00, 0xaa],
            &[0xe0, 0x03, 0x09, 0xaa],
        ),
        (
            "aarch64-apple-darwin",
            140,
            128,
            2_495,
            "db0a7562bd61d9d5020f82cf0ad5bb74ce37202a30299dae18417103923c3b7d",
            &[0xe9, 0x03, 0x00, 0xaa],
            &[0xe0, 0x03, 0x09, 0xaa],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            124,
            112,
            2_352,
            "83dcbe6f6f159806b06e1c2c2a1c4bb7e35a60e5ba4960022c910b540146ac4b",
            &[0x00, 0x10, 0xa0, 0xe1],
            &[0x01, 0x00, 0xa0, 0xe1],
        ),
        (
            "riscv32imac-unknown-elf",
            164,
            152,
            2_368,
            "9e5e821b322170bb2273e00888d5596a233fa8c8aaf6fdad4e2925aeb3d1734c",
            &[0x93, 0x02, 0x05, 0x00],
            &[0x13, 0x85, 0x02, 0x00],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            164,
            152,
            2_816,
            "5e72c8c720d7a2748fd967dbc3437da6d3c935cd27b1d40c0bacd505957d15ed",
            &[0x93, 0x02, 0x05, 0x00],
            &[0x13, 0x85, 0x02, 0x00],
        ),
        (
            "wasm32-wasi",
            62,
            53,
            1_686,
            "0c0a6e28ea5713927f79c1dd769cda0a7438be25179e657fa43dab29945d19bc",
            &[0x22, 0x00],
            &[0x20, 0x00],
        ),
    ];

    for (index, (target, o0_text, o2_text, object_size, object_sha, save, reuse)) in
        targets.iter().enumerate()
    {
        let baseline = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture,
                "--target",
                target,
                "--opt-level",
                "O0",
                "--json",
            ],
        );
        assert!(baseline.status.success(), "{target}");
        let baseline: Value =
            serde_json::from_slice(&baseline.stdout).expect("baseline linked GVN MCT JSON");
        assert_eq!(baseline["text_bytes"], *o0_text, "{target}");
        assert_eq!(baseline["instruction_records"], 26, "{target}");
        assert_eq!(baseline["relocations"], 2, "{target}");

        let mct_path = directory.join(format!("linked-gvn-{index}.nbo"));
        let optimized = run(
            &native,
            &[
                "compiler",
                "emit-mct",
                fixture,
                "--target",
                target,
                "--opt-level",
                "O2",
                "--out",
                mct_path.to_str().expect("UTF-8 linked GVN MCT path"),
                "--json",
            ],
        );
        assert!(
            optimized.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&optimized.stdout)
        );
        let optimized: Value =
            serde_json::from_slice(&optimized.stdout).expect("optimized linked GVN MCT JSON");
        assert_eq!(optimized["text_bytes"], *o2_text, "{target}");
        assert_eq!(optimized["instruction_records"], 22, "{target}");
        assert_eq!(optimized["relocations"], 2, "{target}");
        assert!(*o2_text < *o0_text, "{target} did not reduce material text");
        let mct = fs::read(&mct_path).expect("read linked GVN MCT");
        let text_at = i32le(&mct, 92) as usize;
        let text_bytes = i32le(&mct, 40) as usize;
        let text = &mct[text_at..text_at + text_bytes];
        assert!(
            text.windows(save.len()).any(|window| window == *save),
            "{target} omitted the GVN source save"
        );
        assert!(
            text.windows(reuse.len()).any(|window| window == *reuse),
            "{target} omitted the GVN reuse load"
        );

        let object_path = directory.join(format!("linked-gvn-{index}.object"));
        let object = run(
            &native,
            &[
                "compiler",
                "emit-object",
                fixture,
                "--target",
                target,
                "--out",
                object_path.to_str().expect("UTF-8 linked GVN object path"),
                "--json",
            ],
        );
        assert!(object.status.success(), "{target}");
        let report: Value = serde_json::from_slice(&object.stdout).expect("linked GVN object JSON");
        assert_eq!(report["bytes"], *object_size, "{target}");
        assert_eq!(report["relocations"], 2, "{target}");
        let object = fs::read(&object_path).expect("read linked GVN object");
        assert_eq!(object.len(), *object_size, "{target}");
        assert_eq!(
            format!("{:x}", Sha256::digest(&object)),
            *object_sha,
            "{target}"
        );
    }
}

#[test]
fn native_nc_mct_writes_a_valid_msf7_pdb() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native PDB execution: no host linker");
        return;
    };
    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let directory = root().join("target/tmp/ncc-stage2-pdb-differential");
    fs::create_dir_all(&directory).expect("create PDB differential directory");
    let artifact = directory.join("ir_material_all.pdb");
    let emitted = run(
        &native,
        &[
            "compiler",
            "emit-pdb",
            fixture.to_str().expect("UTF-8 PDB fixture"),
            "--out",
            artifact.to_str().expect("UTF-8 PDB artifact"),
            "--json",
        ],
    );
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stdout)
    );
    let document: Value = serde_json::from_slice(&emitted.stdout).expect("native PDB JSON");
    assert_eq!(document["schema"], "nc.bootstrap.mct.pdb-msf7.v1");
    assert_eq!(document["format"], "pdb-msf7");
    assert_eq!(document["target"], "x86_64-pc-windows-msvc");
    assert_eq!(document["block_bytes"], 4096);
    assert_eq!(document["blocks"], 11);
    assert_eq!(document["streams"], 8);
    assert_eq!(document["modules"], 1);
    assert_eq!(document["signature"], 208_257_237);
    assert_eq!(document["bytes"], 45_056);
    assert_eq!(document["hash"], 733_420_645);

    let bytes = fs::read(&artifact).expect("read native PDB");
    assert_eq!(bytes.len(), 45_056);
    assert_eq!(&bytes[..24], b"Microsoft C/C++ MSF 7.00");
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("PDB u32"));
    assert_eq!(u32_at(32), 4096);
    assert_eq!(u32_at(40), 11);
    assert_eq!(u32_at(44), 60);
    assert_eq!(u32_at(52), 3);
    assert_eq!(u32_at(10 * 4096), 8);
    assert!(bytes.windows(9).any(|window| window == b"nC-stage2"));

    let checked = run(
        &native,
        &[
            "compiler",
            "pdb-check",
            fixture.to_str().expect("UTF-8 PDB fixture"),
            "208257237",
            "733420645",
        ],
    );
    assert!(checked.status.success());

    for tool in ["llvm-pdbutil-19", "llvm-pdbutil-18", "llvm-pdbutil"] {
        let Ok(inspected) = Command::new(tool)
            .args(["dump", "-summary", "-streams", "-modules", "-types"])
            .arg(&artifact)
            .output()
        else {
            continue;
        };
        assert!(
            inspected.status.success(),
            "{tool} rejected native PDB: {}",
            String::from_utf8_lossy(&inspected.stderr)
        );
        let report = String::from_utf8_lossy(&inspected.stdout);
        assert!(report.contains("Block Size: 4096"));
        assert!(report.contains("Module \"nC-stage2\""));
        break;
    }

    let rejected = run(
        &native,
        &[
            "compiler",
            "emit-pdb",
            fixture.to_str().expect("UTF-8 PDB fixture"),
            "--target",
            "x86_64-unknown-linux-gnu",
            "--json",
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("E_MCT_PDB_TARGET_001"));
}

#[test]
fn native_nc_mct_writes_platform_unwind_for_all_declared_targets() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native unwind execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 native unwind fixture");
    for (target, format, relocations, eh, pdata, xdata, compact, compact_relocs, bytes, hash) in [
        (
            "x86_64-unknown-linux-gnu",
            1,
            2,
            88,
            0,
            0,
            0,
            0,
            200,
            50_970_664,
        ),
        (
            "x86_64-pc-windows-msvc",
            2,
            6,
            0,
            24,
            8,
            0,
            0,
            208,
            50_857_054,
        ),
        (
            "x86_64-apple-darwin",
            4,
            2,
            88,
            0,
            0,
            64,
            2,
            264,
            51_517_433,
        ),
        (
            "i386-unknown-linux-gnu",
            1,
            2,
            72,
            0,
            0,
            0,
            0,
            184,
            50_826_459,
        ),
        (
            "aarch64-unknown-linux-gnu",
            1,
            2,
            92,
            0,
            0,
            0,
            0,
            204,
            51_064_798,
        ),
        (
            "aarch64-apple-darwin",
            4,
            2,
            92,
            0,
            0,
            64,
            2,
            272,
            51_665_538,
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            1,
            2,
            76,
            0,
            0,
            0,
            0,
            188,
            50_899_634,
        ),
        (
            "riscv32imac-unknown-elf",
            1,
            2,
            52,
            0,
            0,
            0,
            0,
            164,
            50_543_875,
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            1,
            2,
            68,
            0,
            0,
            0,
            0,
            180,
            50_652_060,
        ),
        ("wasm32-wasi", 3, 0, 0, 0, 0, 0, 0, 80, 50_104_012),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-native-unwind",
                fixture,
                "--target",
                target,
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value = serde_json::from_slice(&emitted.stdout).expect("native unwind JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.native-unwind.v2");
        assert_eq!(document["target"], target);
        assert_eq!(document["format"], format);
        assert_eq!(document["functions"], 2);
        assert_eq!(document["relocations"], relocations);
        assert_eq!(document["eh_frame_bytes"], eh);
        assert_eq!(document["pdata_bytes"], pdata);
        assert_eq!(document["xdata_bytes"], xdata);
        assert_eq!(document["compact_unwind_bytes"], compact);
        assert_eq!(document["compact_unwind_relocations"], compact_relocs);
        assert_eq!(document["bytes"], bytes);
        assert_eq!(document["hash"], hash);

        let checked = run(
            &native,
            &[
                "compiler",
                "native-unwind-check",
                fixture,
                target,
                &hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }
}

#[test]
fn native_nc_mct_writes_canonical_symbol_bundles_for_all_declared_targets() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native symbol bundle execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 symbol bundle fixture");
    let directory = root().join("target/tmp/ncc-stage2-symbol-bundle-differential");
    fs::create_dir_all(&directory).expect("create symbol bundle differential directory");

    for (target, target_id, expected_hash) in [
        ("x86_64-unknown-linux-gnu", 1u32, 3_243_542),
        ("x86_64-pc-windows-msvc", 2, 3_243_587),
        ("x86_64-apple-darwin", 3, 3_243_632),
        ("i386-unknown-linux-gnu", 4, 3_224_147),
        ("aarch64-unknown-linux-gnu", 5, 3_193_845),
        ("aarch64-apple-darwin", 6, 3_193_890),
        ("armv7-unknown-linux-gnueabihf", 7, 3_249_802),
        ("riscv32imac-unknown-elf", 8, 3_184_837),
        ("riscv64gc-unknown-linux-gnu", 9, 3_186_535),
        ("wasm32-wasi", 10, 3_193_010),
    ] {
        let artifact = directory.join(format!("{}.ncsym", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-symbol-bundle",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 symbol bundle artifact"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native symbol bundle JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.symbol-bundle.v1");
        assert_eq!(document["target"], target);
        assert_eq!(document["target_id"], target_id);
        assert_eq!(document["records"], 4);
        assert_eq!(document["local_records"], 3);
        assert_eq!(document["defined_records"], 4);
        assert_eq!(document["record_bytes"], 64);
        assert_eq!(document["string_bytes"], 40);
        assert_eq!(document["identity_fingerprint"], 463_359);
        assert_eq!(document["bytes"], 360);
        assert_eq!(document["hash"], expected_hash);

        let bytes = fs::read(&artifact).expect("read native symbol bundle");
        assert_eq!(bytes.len(), 360);
        assert_eq!(&bytes[..4], b"NCSY");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            target_id
        );
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 64);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 320);
    }
}

#[test]
fn native_nc_mct_writes_relocatable_objects_for_all_declared_targets() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native object execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/ir_material_all.n");
    let fixture = fixture.to_str().expect("UTF-8 object fixture");
    let directory = root().join("target/tmp/ncc-stage2-object-differential");
    fs::create_dir_all(&directory).expect("create object differential directory");

    for (target, format, expected_bytes, expected_hash, magic, symbols) in [
        (
            "x86_64-unknown-linux-gnu",
            "elf64",
            4136,
            124_323_055,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "x86_64-pc-windows-msvc",
            "coff64",
            2726,
            57_288_457,
            &[0x64, 0x86][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "x86_64-apple-darwin",
            "macho64",
            3771,
            103_937_170,
            &[0xCF, 0xFA, 0xED, 0xFE][..],
            &[
                "_nc$C$00000048",
                "_nc$C$0000001E",
                "_nc$S$R$13$irMaterialAll$3$add",
                "_nc$S$P$13$irMaterialAll$3$run",
                "_main",
            ][..],
        ),
        (
            "i386-unknown-linux-gnu",
            "elf32",
            3616,
            95_576_517,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "aarch64-unknown-linux-gnu",
            "elf64",
            4176,
            126_675_962,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "aarch64-apple-darwin",
            "macho64",
            3811,
            106_282_068,
            &[0xCF, 0xFA, 0xED, 0xFE][..],
            &[
                "_nc$C$00000048",
                "_nc$C$0000001E",
                "_nc$S$R$13$irMaterialAll$3$add",
                "_nc$S$P$13$irMaterialAll$3$run",
                "_main",
            ][..],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            "elf32",
            3676,
            99_085_907,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "riscv32imac-unknown-elf",
            "elf32",
            3692,
            98_243_192,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            "elf64",
            4176,
            124_072_819,
            &[0x7F, b'E', b'L', b'F'][..],
            &[
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
                "main",
            ][..],
        ),
        (
            "wasm32-wasi",
            "wasm",
            2937,
            64_251_883,
            &[0, b'a', b's', b'm'][..],
            &[
                "ncc.mct",
                "ncc.link",
                "ncc.symbols",
                "ncc.reloc",
                "nc$C$00000048",
                "nc$C$0000001E",
                "nc$S$R$13$irMaterialAll$3$add",
                "nc$S$P$13$irMaterialAll$3$run",
            ][..],
        ),
    ] {
        let artifact = directory.join(format!("{}.object", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-object",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 object artifact"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value = serde_json::from_slice(&emitted.stdout).expect("native object JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.object.v5");
        assert_eq!(document["target"], target);
        assert_eq!(document["format"], format);
        assert_eq!(document["bytes"], expected_bytes);
        assert_eq!(document["text_bytes"].as_i64().is_some(), true);
        assert_eq!(document["relocations"], 4);
        assert_eq!(document["symbols"], 4);
        assert_eq!(document["defined_symbols"], 4);
        assert_eq!(document["external_symbols"], 0);
        assert_eq!(document["control_symbols"], 2);
        assert_eq!(document["unresolved_symbols"], 0);
        assert_eq!(document["public_symbols"], 1);
        assert_eq!(document["private_symbols"], 1);
        assert_eq!(document["internal_symbols"], 0);
        assert_eq!(document["default_symbols"], 2);
        assert_eq!(document["identity_bytes"], 64);
        assert_eq!(document["identity_fingerprint"], 463_359);
        assert_eq!(document["mangle_bytes"], 88);
        assert_eq!(document["symbol_bundle_bytes"], 360);
        assert_eq!(document["symbol_bundle_records"], 4);
        assert!(document["symbol_bundle_hash"]
            .as_i64()
            .is_some_and(|hash| hash > 0));
        assert_eq!(document["debug_bytes"], 1408);
        assert_eq!(document["debug_lines"], 64);
        assert_eq!(document["unwind_functions"], 2);
        assert_eq!(
            document["platform_debug_format"],
            if target.contains("windows") { 2 } else { 1 }
        );
        assert_eq!(
            document["platform_abbrev_bytes"],
            if target.contains("windows") { 0 } else { 33 }
        );
        assert_eq!(
            document["platform_codeview_bytes"],
            if target.contains("windows") { 76 } else { 0 }
        );
        assert_eq!(
            document["native_unwind_format"],
            if target.contains("windows") {
                2
            } else if target.contains("apple-darwin") {
                4
            } else if target == "wasm32-wasi" {
                3
            } else {
                1
            }
        );
        assert_eq!(
            document["native_unwind_relocations"],
            if target.contains("windows") {
                6
            } else if target == "wasm32-wasi" {
                0
            } else {
                2
            }
        );
        assert_eq!(
            document["pdata_bytes"],
            if target.contains("windows") { 24 } else { 0 }
        );
        assert_eq!(
            document["xdata_bytes"],
            if target.contains("windows") { 8 } else { 0 }
        );
        assert_eq!(
            document["compact_unwind_bytes"],
            if target.contains("apple-darwin") {
                64
            } else {
                0
            }
        );
        assert_eq!(
            document["compact_unwind_relocations"],
            if target.contains("apple-darwin") {
                2
            } else {
                0
            }
        );
        assert!(document["native_unwind_hash"]
            .as_i64()
            .is_some_and(|hash| hash > 0));
        assert_eq!(document["object_hash"], expected_hash);

        let bytes = fs::read(&artifact).expect("read native object artifact");
        assert_eq!(bytes.len(), expected_bytes as usize);
        assert_eq!(&bytes[..magic.len()], magic, "{target} magic");
        assert!(bytes.windows(4).any(|window| window == b"NCSY"));
        for symbol in symbols {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "{target} omitted {symbol}"
            );
        }
        let debug_name = if target.contains("apple-darwin") {
            "__ncc_debug"
        } else if target.contains("windows") {
            ".nccdbg"
        } else {
            "ncc.debug"
        };
        assert!(
            bytes
                .windows(debug_name.len())
                .any(|window| window == debug_name.as_bytes()),
            "{target} omitted {debug_name}"
        );
        let platform_name = if target.contains("windows") {
            ".debug$S"
        } else if target.contains("apple-darwin") {
            "__debug_abbrev"
        } else {
            ".debug_abbrev"
        };
        assert!(
            bytes
                .windows(platform_name.len())
                .any(|window| window == platform_name.as_bytes()),
            "{target} omitted {platform_name}"
        );
        let unwind_name = if target.contains("windows") {
            ".pdata"
        } else if target.contains("apple-darwin") {
            "__eh_frame"
        } else if target == "wasm32-wasi" {
            "ncc.unwind"
        } else {
            ".eh_frame"
        };
        assert!(
            bytes
                .windows(unwind_name.len())
                .any(|window| window == unwind_name.as_bytes()),
            "{target} omitted {unwind_name}"
        );
        if target.contains("apple-darwin") {
            assert!(bytes
                .windows(16)
                .any(|window| window == b"__compact_unwind"));
            let compact_bytes = u64::from_le_bytes(
                bytes[624..632]
                    .try_into()
                    .expect("Mach-O compact unwind size"),
            );
            let compact_at = u32::from_le_bytes(
                bytes[632..636]
                    .try_into()
                    .expect("Mach-O compact unwind offset"),
            ) as usize;
            let compact_relocations = u32::from_le_bytes(
                bytes[644..648]
                    .try_into()
                    .expect("Mach-O compact unwind relocation count"),
            );
            assert_eq!(compact_bytes, 64);
            assert_eq!(compact_relocations, 2);
            let expected_encoding = if target.starts_with("x86_64") {
                0x0100_0000_u32
            } else {
                0x0400_0000_u32
            };
            for row in 0..2 {
                let encoding_at = compact_at + row * 32 + 12;
                assert_eq!(
                    u32::from_le_bytes(
                        bytes[encoding_at..encoding_at + 4]
                            .try_into()
                            .expect("compact unwind encoding"),
                    ),
                    expected_encoding
                );
            }
        }
    }

    for (target, relocation_name) in [
        ("x86_64-apple-darwin", "X86_64_RELOC_UNSIGNED"),
        ("aarch64-apple-darwin", "ARM64_RELOC_UNSIGNED"),
    ] {
        let artifact = directory.join(format!("{}.object", target.replace('-', "_")));
        for tool in ["llvm-readobj-18", "llvm-readobj"] {
            let Ok(inspected) = Command::new(tool)
                .args(["--sections", "--relocations"])
                .arg(&artifact)
                .output()
            else {
                continue;
            };
            assert!(inspected.status.success(), "{tool} rejected {target}");
            let report = String::from_utf8_lossy(&inspected.stdout);
            assert!(report.contains("__LD"));
            assert!(report.contains("__compact_unwind"));
            assert_eq!(report.matches(relocation_name).count(), 4);
            break;
        }
    }

    let rejected = run(
        &native,
        &[
            "compiler",
            "emit-object",
            fixture,
            "--target",
            "m68k-unknown-none",
            "--json",
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("E_IR_TARGET_UNSUPPORTED_001"));
}

#[test]
fn native_nc_objects_publish_runtime_abi_and_hosted_entry_symbols() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native runtime ABI symbols: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");
    let fixture = fixture.to_str().expect("UTF-8 runtime ABI fixture");
    let directory = root().join("target/tmp/ncc-stage2-runtime-abi-differential");
    fs::create_dir_all(&directory).expect("create runtime ABI directory");

    for (target, format) in [
        ("x86_64-unknown-linux-gnu", "elf"),
        ("x86_64-pc-windows-msvc", "coff"),
        ("x86_64-apple-darwin", "macho"),
        ("i386-unknown-linux-gnu", "elf"),
        ("aarch64-unknown-linux-gnu", "elf"),
        ("aarch64-apple-darwin", "macho"),
        ("armv7-unknown-linux-gnueabihf", "elf"),
        ("riscv32imac-unknown-elf", "elf"),
        ("riscv64gc-unknown-linux-gnu", "elf"),
    ] {
        let artifact = directory.join(format!("{}.o", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-object",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 runtime ABI object"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("runtime ABI JSON");
        assert_eq!(report["external_symbols"], 1, "{target}");
        assert_eq!(report["defined_symbols"], 1, "{target}");

        let bytes = fs::read(&artifact).expect("read runtime ABI object");
        let (entry, runtime) = match format {
            "elf" => (
                elf_symbol_sections(&bytes, "main"),
                elf_symbol_sections(&bytes, "nC.runtime.output.println"),
            ),
            "coff" => (
                coff_symbol_sections(&bytes, "main")
                    .into_iter()
                    .map(|section| section as u16)
                    .collect(),
                coff_symbol_sections(&bytes, "nC.runtime.output.println")
                    .into_iter()
                    .map(|section| section as u16)
                    .collect(),
            ),
            "macho" => (
                macho_symbol_sections(&bytes, "_main")
                    .into_iter()
                    .map(u16::from)
                    .collect(),
                macho_symbol_sections(&bytes, "_nC.runtime.output.println")
                    .into_iter()
                    .map(u16::from)
                    .collect(),
            ),
            _ => unreachable!(),
        };
        assert_eq!(entry, vec![1], "{target} hosted main definition");
        assert_eq!(runtime, vec![0], "{target} runtime import");
    }
}

#[test]
fn native_nc_final_link_plan_places_runtime_providers_and_target_thunks() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native final link plan: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");
    let fixture = fixture.to_str().expect("UTF-8 final link fixture");
    let directory = root().join("target/tmp/ncc-stage2-final-link-plan-differential");
    fs::create_dir_all(&directory).expect("create final link plan directory");

    for (target, hash, entry, mode, module, wasm_start) in [
        (
            "x86_64-unknown-linux-gnu",
            1_914_251_297,
            29,
            1,
            "libncruntime.so.2",
            -1,
        ),
        (
            "x86_64-pc-windows-msvc",
            770_104_110,
            29,
            1,
            "ncruntime.dll",
            -1,
        ),
        (
            "x86_64-apple-darwin",
            881_082_235,
            29,
            1,
            "libncruntime.2.dylib",
            -1,
        ),
        (
            "i386-unknown-linux-gnu",
            750_591_543,
            26,
            1,
            "libncruntime.so.2",
            -1,
        ),
        (
            "aarch64-unknown-linux-gnu",
            1_057_592_087,
            36,
            1,
            "libncruntime.so.2",
            -1,
        ),
        (
            "aarch64-apple-darwin",
            1_719_723_252,
            36,
            1,
            "libncruntime.2.dylib",
            -1,
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            1_296_466_938,
            32,
            1,
            "libncruntime.so.2",
            -1,
        ),
        (
            "riscv32imac-unknown-elf",
            1_593_109_107,
            44,
            2,
            "libncruntime-rv32.a",
            -1,
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            1_779_999_542,
            44,
            1,
            "libncruntime.so.2",
            -1,
        ),
        ("wasm32-wasi", 623_722_611, 18, 3, "per-family", 1),
    ] {
        let checked = run(
            &native,
            &[
                "compiler",
                "final-link-plan-check",
                fixture,
                target,
                "1",
                "2",
                "2",
                "0",
                "0",
                "1",
                &hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&checked.stdout)
        );

        let artifact = directory.join(format!("{}.nclink", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "final-link-plan",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 final link plan"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("final link JSON");
        assert_eq!(report["providers"], 1, "{target}");
        assert_eq!(report["relocations"], 2, "{target}");
        assert_eq!(report["applied"], 2, "{target}");
        assert_eq!(report["ready"], true, "{target}");
        assert_eq!(report["provider_module"], module, "{target}");
        assert_eq!(
            report["bindings"][0]["symbol"], "nC.runtime.output.println",
            "{target}"
        );
        assert_eq!(
            report["bindings"][0]["module"],
            if target == "wasm32-wasi" {
                "nC.runtime.output"
            } else {
                module
            },
            "{target}"
        );

        let bytes = fs::read(&artifact).expect("read final link plan");
        assert_eq!(u32le(&bytes, 0), 1_279_873_870, "{target} magic");
        assert_eq!(u32le(&bytes, 4), 1, "{target} version");
        assert_eq!(u32le(&bytes, 36), 1, "{target} providers");
        assert_eq!(u32le(&bytes, 60), 2, "{target} relocations");
        assert_eq!(u32le(&bytes, 64), 2, "{target} applied");
        assert_eq!(u32le(&bytes, 76), 0, "{target} project targets");
        assert_eq!(u32le(&bytes, 80), 0, "{target} unresolved targets");
        assert_eq!(i32le(&bytes, 84), entry, "{target} hosted entry");
        assert_eq!(i32le(&bytes, 88), mode, "{target} provider mode");
        assert_eq!(u32le(&bytes, 92), 1, "{target} readiness");
        assert_eq!(u32le(&bytes, 96), hash as u32, "{target} hash");
        assert_eq!(u32le(&bytes, 100) as usize, bytes.len(), "{target} size");
        assert_eq!(i32le(&bytes, 104), wasm_start, "{target} WASM start");

        let provider = u32le(&bytes, 32) as usize;
        assert_eq!(u32le(&bytes, provider + 20), 0, "{target} import index");
        assert_eq!(u32le(&bytes, provider + 40), 1, "{target} binding calls");
        let thunk = i32le(&bytes, provider + 24);
        if target == "wasm32-wasi" {
            assert_eq!(thunk, -1);
            assert_eq!(i32le(&bytes, provider + 28), -1);
            continue;
        }

        let text = u32le(&bytes, 20) as usize;
        let thunk = text + thunk as usize;
        if target.starts_with("x86_64") || target.starts_with("i386") {
            assert_eq!(&bytes[thunk..thunk + 2], &[0xFF, 0x25], "{target}");
        } else if target.starts_with("aarch64") {
            assert_eq!(u32le(&bytes, thunk) & 0x9F00_001F, 0x9000_0010, "{target}");
            assert_eq!(
                u32le(&bytes, thunk + 4) & 0xFFC0_03FF,
                0xF940_0210,
                "{target}"
            );
            assert_eq!(u32le(&bytes, thunk + 8), 0xD61F_0200, "{target}");
        } else if target.starts_with("armv7") {
            assert_eq!(u32le(&bytes, thunk), 0xE59F_C000, "{target}");
            assert_eq!(u32le(&bytes, thunk + 4), 0xE59C_F000, "{target}");
        } else {
            assert_eq!(u32le(&bytes, thunk) & 0x7F, 0x17, "{target}");
            assert_eq!(u32le(&bytes, thunk + 4) & 0x7F, 0x03, "{target}");
            assert_eq!(u32le(&bytes, thunk + 8), 0x0003_0067, "{target}");
        }
    }
}

#[test]
fn native_nc_pe64_final_image_has_executable_entry_and_canonical_iat() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native PE64 final image: no host linker");
        return;
    };

    let directory = root().join("target/tmp/ncc-stage2-pe64-final-image-differential");
    fs::create_dir_all(&directory).expect("create PE64 final image directory");
    let pure_fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_final_image_exit.n");
    let runtime_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");
    let pure = directory.join("mct_final_image_exit.exe");
    let runtime = directory.join("mct_runtime_abi_hosted_main.exe");

    for (fixture, output, expected_entry, expected_providers, expected_hash) in [
        (&pure_fixture, &pure, 4_112, 0, 1_601_442_192),
        (&runtime_fixture, &runtime, 4_125, 1, 597_914_816),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                fixture.to_str().expect("UTF-8 PE fixture"),
                "--target",
                "x86_64-pc-windows-msvc",
                "--out",
                output.to_str().expect("UTF-8 PE output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("PE64 image JSON");
        assert_eq!(report["schema"], "nc.mct.image.v1");
        assert_eq!(report["format"], "pe64");
        assert_eq!(report["format_id"], 1);
        assert_eq!(report["bytes"], 1_536);
        assert_eq!(report["entry"], expected_entry);
        assert_eq!(report["providers"], expected_providers);
        assert_eq!(report["hash"], expected_hash);

        let checked = run(
            &native,
            &[
                "compiler",
                "final-image-check",
                fixture.to_str().expect("UTF-8 PE fixture"),
                "x86_64-pc-windows-msvc",
                "1",
                "1536",
                &expected_entry.to_string(),
                &expected_providers.to_string(),
                &expected_hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stdout)
        );
    }

    let bytes = fs::read(&pure).expect("read pure PE64 image");
    assert_eq!(&bytes[..2], b"MZ");
    let pe = u32le(&bytes, 0x3c) as usize;
    assert_eq!(&bytes[pe..pe + 4], b"PE\0\0");
    let coff = pe + 4;
    assert_eq!(u16le(&bytes, coff), 0x8664);
    assert_eq!(u16le(&bytes, coff + 2), 2);
    let optional = coff + 20;
    assert_eq!(u16le(&bytes, optional), 0x020b);
    assert_eq!(u32le(&bytes, optional + 16), 0x1010);
    assert_eq!(u64le(&bytes, optional + 24), 0x0040_0000);
    assert_eq!(u32le(&bytes, optional + 120), 0);
    assert_eq!(u32le(&bytes, optional + 208), 0);
    let entry = pe_rva_offset(&bytes, u32le(&bytes, optional + 16) as usize);
    assert_eq!(&bytes[entry..entry + 3], &[0x31, 0xc0, 0xc3]);

    let bytes = fs::read(&runtime).expect("read runtime PE64 image");
    let pe = u32le(&bytes, 0x3c) as usize;
    let optional = pe + 4 + 20;
    let imports_rva = u32le(&bytes, optional + 120) as usize;
    let iat_rva = u32le(&bytes, optional + 208) as usize;
    assert_eq!(imports_rva, 0x2000);
    assert_eq!(iat_rva, 0x1050);
    let descriptor = pe_rva_offset(&bytes, imports_rva);
    let lookup_rva = u32le(&bytes, descriptor) as usize;
    let dll_rva = u32le(&bytes, descriptor + 12) as usize;
    assert_eq!(u32le(&bytes, descriptor + 16) as usize, iat_rva);
    assert_eq!(
        object_string(&bytes, pe_rva_offset(&bytes, dll_rva)),
        "ncruntime.dll"
    );
    let lookup = pe_rva_offset(&bytes, lookup_rva);
    let hint_name_rva = u64le(&bytes, lookup) as usize;
    let hint_name = pe_rva_offset(&bytes, hint_name_rva);
    assert_eq!(u16le(&bytes, hint_name), 0);
    assert_eq!(
        object_string(&bytes, hint_name + 2),
        "nC.runtime.output.println"
    );
    let iat = pe_rva_offset(&bytes, iat_rva);
    assert_eq!(u64le(&bytes, iat), 0);

    if cfg!(windows) {
        let executed = Command::new(&pure)
            .current_dir(root())
            .output()
            .expect("execute native PE64 loader smoke");
        assert!(
            executed.status.success(),
            "PE64 loader smoke exited with {:?}",
            executed.status.code()
        );
    }
}

#[test]
fn native_nc_final_images_use_canonical_prelude_runtime_symbols() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native runtime prelude ABI images: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_prelude_abi.n");
    let directory = root().join("target/tmp/ncc-stage2-runtime-prelude-abi");
    fs::create_dir_all(&directory).expect("create runtime prelude ABI directory");

    for target in [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "i386-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "armv7-unknown-linux-gnueabihf",
        "riscv32imac-unknown-elf",
        "riscv64gc-unknown-linux-gnu",
        "wasm32-wasi",
    ] {
        let plan = directory.join(format!("{}.nclink", target.replace('-', "_")));
        let planned = run(
            &native,
            &[
                "compiler",
                "final-link-plan",
                fixture.to_str().expect("UTF-8 runtime prelude fixture"),
                "--target",
                target,
                "--out",
                plan.to_str().expect("UTF-8 runtime prelude link plan"),
                "--json",
            ],
        );
        assert!(
            planned.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&planned.stdout)
        );
        let plan_report: Value =
            serde_json::from_slice(&planned.stdout).expect("runtime prelude final-link JSON");
        assert_eq!(plan_report["providers"], 3, "{target}");
        let mut symbols = plan_report["bindings"]
            .as_array()
            .expect("runtime prelude bindings")
            .iter()
            .map(|binding| binding["symbol"].as_str().expect("runtime symbol"))
            .collect::<Vec<_>>();
        symbols.sort_unstable();
        assert_eq!(
            symbols,
            vec![
                "nC.runtime.alloc",
                "nC.runtime.free",
                "nC.runtime.output.println"
            ],
            "{target}"
        );

        if target == "riscv32imac-unknown-elf" {
            assert_eq!(plan_report["provider_mode"], 2);
            assert_eq!(plan_report["provider_module"], "libncruntime-rv32.a");
            continue;
        }

        let output = directory.join(format!("{}.image", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                fixture.to_str().expect("UTF-8 runtime prelude fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 runtime prelude image"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value =
            serde_json::from_slice(&emitted.stdout).expect("runtime prelude final-image JSON");
        assert_eq!(report["providers"], 3, "{target}");

        let bytes = fs::read(&output).expect("read runtime prelude final image");
        assert!(
            bytes
                .windows(b"nC.runtime.alloc".len())
                .any(|window| window == b"nC.runtime.alloc"),
            "{target}: missing canonical alloc symbol"
        );
        assert!(
            bytes
                .windows(b"nC.runtime.free".len())
                .any(|window| window == b"nC.runtime.free"),
            "{target}: missing canonical free symbol"
        );
        assert!(
            bytes
                .windows(b"nC.runtime.output.println".len())
                .any(|window| window == b"nC.runtime.output.println"),
            "{target}: missing family-qualified println symbol"
        );
        assert!(
            !bytes
                .windows(b"nC.runtime..".len())
                .any(|window| window == b"nC.runtime.."),
            "{target}: double-dot runtime ABI escaped into final bytes"
        );
    }
}

#[test]
fn native_nc_elf64_final_images_cover_loader_entry_and_dynamic_link_contracts() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native ELF64 final image: no host linker");
        return;
    };

    let directory = root().join("target/tmp/ncc-stage2-elf64-final-image-differential");
    fs::create_dir_all(&directory).expect("create ELF64 final image directory");
    let pure_fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_final_image_exit.n");
    let runtime_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");

    let targets = [
        (
            "x86_64-unknown-linux-gnu",
            62_u16,
            0_u32,
            4_198_416_u64,
            830_827_203_i64,
            &[0x31, 0xff, 0xb8, 0x3c, 0, 0, 0, 0x0f, 0x05][..],
        ),
        (
            "aarch64-unknown-linux-gnu",
            183,
            0,
            4_198_416,
            1_676_112_321,
            &[
                0x00, 0x00, 0x80, 0xd2, 0xa8, 0x0b, 0x80, 0xd2, 0x01, 0x00, 0x00, 0xd4,
            ][..],
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            243,
            4,
            65_552,
            1_999_093_983,
            &[
                0x13, 0x05, 0x00, 0x00, 0x93, 0x08, 0xd0, 0x05, 0x73, 0x00, 0x00, 0x00,
            ][..],
        ),
    ];

    for (target, machine, flags, entry, hash, start) in targets {
        let output = directory.join(format!("{}.elf", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                pure_fixture.to_str().expect("UTF-8 ELF fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 ELF output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("ELF64 image JSON");
        assert_eq!(report["format"], "elf64");
        assert_eq!(report["format_id"], 2);
        assert_eq!(report["bytes"], 4_472);
        assert_eq!(report["entry"], entry);
        assert_eq!(report["providers"], 0);
        assert_eq!(report["flags"], 1);
        assert_eq!(report["hash"], hash);

        let bytes = fs::read(&output).expect("read pure ELF64 image");
        assert_eq!(&bytes[..7], b"\x7fELF\x02\x01\x01");
        assert_eq!(u16le(&bytes, 16), 2);
        assert_eq!(u16le(&bytes, 18), machine);
        assert_eq!(u32le(&bytes, 48), flags);
        assert_eq!(u64le(&bytes, 24), entry);
        assert_eq!(u16le(&bytes, 56), 1);
        assert_eq!(u16le(&bytes, 60), 4);
        let (start_at, start_bytes, start_va, _, _, _) = elf64_section(&bytes, ".text.start");
        assert_eq!(start_va, entry);
        assert_eq!(start_bytes, start.len());
        assert_eq!(
            &bytes[start_at..start_at + start.len()],
            start,
            "{target} _start"
        );
    }

    let dynamic_targets = [
        (
            "x86_64-unknown-linux-gnu",
            62_u16,
            0_u32,
            4_198_488_u64,
            4_294_967_303_u64,
            1_i64,
            13_408_i64,
            2_077_095_500_i64,
            "/lib64/ld-linux-x86-64.so.2",
        ),
        (
            "aarch64-unknown-linux-gnu",
            183,
            0,
            4_198_504,
            4_294_968_322,
            1,
            13_408,
            1_745_720_476,
            "/lib/ld-linux-aarch64.so.1",
        ),
        (
            "riscv64gc-unknown-linux-gnu",
            243,
            4,
            65_656,
            4_294_967_301,
            1,
            13_408,
            1_930_077_427,
            "/lib/ld-linux-riscv64-lp64d.so.1",
        ),
    ];

    for (target, machine, flags, entry, relocation_info, symbol, image_bytes, hash, interp) in
        dynamic_targets
    {
        let output = directory.join(format!("runtime_{}.elf", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                runtime_fixture.to_str().expect("UTF-8 runtime ELF fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 runtime ELF output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("dynamic ELF JSON");
        assert_eq!(report["bytes"], image_bytes);
        assert_eq!(report["entry"], entry);
        assert_eq!(report["providers"], 1);
        assert_eq!(report["flags"], 3);
        assert_eq!(report["hash"], hash);

        let bytes = fs::read(&output).expect("read dynamic ELF64 image");
        assert_eq!(u16le(&bytes, 18), machine);
        assert_eq!(u32le(&bytes, 48), flags);
        assert_eq!(u16le(&bytes, 56), 5);
        assert_eq!(u16le(&bytes, 60), 12);
        let (interp_at, _, _, _, _, _) = elf64_section(&bytes, ".interp");
        assert_eq!(object_string(&bytes, interp_at), interp);
        let (dynstr_at, _, _, _, _, _) = elf64_section(&bytes, ".dynstr");
        assert_eq!(object_string(&bytes, dynstr_at + 1), "libncruntime.so.2");
        assert_eq!(
            object_string(&bytes, dynstr_at + 19),
            "nC.runtime.output.println"
        );
        let (dynsym_at, dynsym_bytes, _, dynsym_link, dynsym_info, dynsym_stride) =
            elf64_section(&bytes, ".dynsym");
        assert_eq!(dynsym_bytes, 48);
        assert_eq!((dynsym_link, dynsym_info, dynsym_stride), (6, 1, 24));
        assert_eq!(u32le(&bytes, dynsym_at + 24), 19);
        assert_eq!(bytes[dynsym_at + 28], 0x12);
        let (got_at, got_bytes, got_va, _, _, _) = elf64_section(&bytes, ".got.plt");
        assert_eq!(got_bytes, 8);
        assert_eq!(u64le(&bytes, got_at), 0);
        let (rela_at, rela_bytes, _, rela_link, rela_info, rela_stride) =
            elf64_section(&bytes, ".rela.plt");
        assert_eq!(
            (rela_bytes, rela_link, rela_info, rela_stride),
            (24, 7, 4, 24)
        );
        assert_eq!(u64le(&bytes, rela_at), got_va);
        assert_eq!(u64le(&bytes, rela_at + 8), relocation_info);
        assert_eq!(u64le(&bytes, rela_at + 8) >> 32, symbol as u64);
        let (dynamic_at, dynamic_bytes, _, dynamic_link, _, dynamic_stride) =
            elf64_section(&bytes, ".dynamic");
        assert_eq!((dynamic_bytes, dynamic_link, dynamic_stride), (256, 6, 16));
        assert_eq!(u64le(&bytes, dynamic_at), 1);
        assert_eq!(u64le(&bytes, dynamic_at + 8), 1);
        assert_eq!(u64le(&bytes, dynamic_at + 13 * 16), 24);
        assert_eq!(u64le(&bytes, dynamic_at + 14 * 16), 30);
        assert_eq!(u64le(&bytes, dynamic_at + 15 * 16), 0);
    }

    let pure = directory.join("x86_64_unknown_linux_gnu.elf");
    if cfg!(windows) {
        let probe = Command::new("wsl.exe")
            .args(["-e", "sh", "-lc", "true"])
            .output();
        if probe.as_ref().is_ok_and(|output| output.status.success()) {
            let converted = Command::new("wsl.exe")
                .args(["-e", "wslpath", "-a"])
                .arg(&pure)
                .output()
                .expect("convert ELF smoke path through WSL");
            assert!(converted.status.success());
            let linux_path = String::from_utf8(converted.stdout)
                .expect("UTF-8 WSL path")
                .trim()
                .to_string();
            let command = format!(
                "cp '{}' /tmp/ncc-stage2-elf64-smoke && chmod 700 /tmp/ncc-stage2-elf64-smoke && exec /tmp/ncc-stage2-elf64-smoke",
                linux_path
            );
            let executed = Command::new("wsl.exe")
                .args(["-e", "sh", "-lc", &command])
                .output()
                .expect("execute ELF64 through the Linux system loader");
            assert!(executed.status.success(), "WSL ELF loader smoke failed");
        }
    } else {
        let chmod = Command::new("chmod")
            .arg("700")
            .arg(&pure)
            .output()
            .expect("chmod ELF64 loader smoke");
        assert!(chmod.status.success());
        let executed = Command::new(&pure)
            .output()
            .expect("execute ELF64 through the host loader");
        assert!(executed.status.success(), "host ELF loader smoke failed");
    }
}

#[test]
fn native_nc_elf32_final_images_cover_three_isas_and_hosted_dynamic_links() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native ELF32 final image: no host linker");
        return;
    };

    let directory = root().join("target/tmp/ncc-stage2-elf32-final-image-differential");
    fs::create_dir_all(&directory).expect("create ELF32 final image directory");
    let pure_fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_final_image_exit.n");
    let runtime_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");

    let targets = [
        (
            "i386-unknown-linux-gnu",
            3_u16,
            0_u32,
            134_516_743_i64,
            4_364_i64,
            1_531_133_552_i64,
            &[0x31, 0xdb, 0xb8, 0x01, 0, 0, 0, 0xcd, 0x80][..],
        ),
        (
            "armv7-unknown-linux-gnueabihf",
            40,
            0x0500_0000,
            65_544,
            4_368,
            704_722_474,
            &[
                0x00, 0x00, 0xa0, 0xe3, 0x01, 0x70, 0xa0, 0xe3, 0x00, 0x00, 0x00, 0xef,
            ][..],
        ),
        (
            "riscv32imac-unknown-elf",
            243,
            1,
            65_552,
            4_376,
            73_550_170,
            &[
                0x13, 0x05, 0x00, 0x00, 0x93, 0x08, 0xd0, 0x05, 0x73, 0x00, 0x00, 0x00,
            ][..],
        ),
    ];

    for (target, machine, flags, entry, image_bytes, hash, start) in targets {
        let output = directory.join(format!("pure-{}.elf", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                pure_fixture.to_str().expect("UTF-8 ELF32 fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 ELF32 output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("ELF32 image JSON");
        assert_eq!(report["format"], "elf32");
        assert_eq!(report["format_id"], 3);
        assert_eq!(report["bytes"], image_bytes);
        assert_eq!(report["entry"], entry);
        assert_eq!(report["providers"], 0);
        assert_eq!(report["flags"], 1);
        assert_eq!(report["hash"], hash);

        let bytes = fs::read(&output).expect("read pure ELF32 image");
        assert_eq!(&bytes[..7], b"\x7fELF\x01\x01\x01");
        assert_eq!(u16le(&bytes, 16), 2);
        assert_eq!(u16le(&bytes, 18), machine);
        assert_eq!(u32le(&bytes, 24), entry as u32);
        assert_eq!(u32le(&bytes, 36), flags);
        assert_eq!((u16le(&bytes, 44), u16le(&bytes, 48)), (1, 4));
        let (start_at, start_bytes, start_va, _, _, _) = elf32_section(&bytes, ".text.start");
        assert_eq!((start_bytes, start_va), (start.len(), entry as u32));
        assert_eq!(
            &bytes[start_at..start_at + start.len()],
            start,
            "{target} _start"
        );
        assert_eq!(
            u32le(&bytes, 52 + 24),
            5,
            "{target} text segment must be RX"
        );
    }

    let runtime = directory.join("runtime-i386.elf");
    let emitted = run(
        &native,
        &[
            "compiler",
            "emit-executable",
            runtime_fixture
                .to_str()
                .expect("UTF-8 runtime ELF32 fixture"),
            "--target",
            "i386-unknown-linux-gnu",
            "--out",
            runtime.to_str().expect("UTF-8 runtime ELF32 output"),
            "--json",
        ],
    );
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stdout)
    );
    let report: Value = serde_json::from_slice(&emitted.stdout).expect("dynamic ELF32 JSON");
    assert_eq!(report["format"], "elf32");
    assert_eq!(report["bytes"], 12_992);
    assert_eq!(report["entry"], 134_516_816);
    assert_eq!(report["providers"], 1);
    assert_eq!(report["flags"], 3);
    assert_eq!(report["hash"], 1_348_400_354);

    let bytes = fs::read(&runtime).expect("read dynamic ELF32 image");
    assert_eq!(u16le(&bytes, 18), 3);
    assert_eq!((u16le(&bytes, 44), u16le(&bytes, 48)), (5, 12));
    assert_eq!(
        (
            u32le(&bytes, 52 + 24),
            u32le(&bytes, 52 + 32 + 24),
            u32le(&bytes, 52 + 64 + 24)
        ),
        (5, 4, 6),
        "ELF32 dynamic load segments must remain RX/R/RW"
    );
    let (interp_at, interp_bytes, _, _, _, _) = elf32_section(&bytes, ".interp");
    assert_eq!(interp_bytes, 19);
    assert_eq!(object_string(&bytes, interp_at), "/lib/ld-linux.so.2");
    let (dynstr_at, _, _, _, _, _) = elf32_section(&bytes, ".dynstr");
    assert_eq!(object_string(&bytes, dynstr_at + 1), "libncruntime.so.2");
    assert_eq!(
        object_string(&bytes, dynstr_at + 19),
        "nC.runtime.output.println"
    );
    let (dynsym_at, dynsym_bytes, _, dynsym_link, dynsym_info, dynsym_stride) =
        elf32_section(&bytes, ".dynsym");
    assert_eq!(
        (dynsym_bytes, dynsym_link, dynsym_info, dynsym_stride),
        (32, 6, 1, 16)
    );
    assert_eq!(u32le(&bytes, dynsym_at + 16), 19);
    assert_eq!(bytes[dynsym_at + 28], 0x12);
    let (slot_at, slot_bytes, slot_va, _, _, slot_stride) = elf32_section(&bytes, ".got.plt");
    assert_eq!((slot_bytes, slot_va, slot_stride), (4, 0x0804_b000, 4));
    assert_eq!(u32le(&bytes, slot_at), 0);
    let (plt_at, plt_bytes, _, _, _, plt_stride) = elf32_section(&bytes, ".plt");
    assert_eq!((plt_bytes, plt_stride), (16, 16));
    assert_eq!(
        &bytes[plt_at..plt_at + 8],
        &[0xff, 0x25, 0x00, 0xb0, 0x04, 0x08, 0x66, 0x90]
    );
    let (rel_at, rel_bytes, _, rel_link, rel_info, rel_stride) = elf32_section(&bytes, ".rel.plt");
    assert_eq!((rel_bytes, rel_link, rel_info, rel_stride), (8, 7, 4, 8));
    assert_eq!(u32le(&bytes, rel_at), slot_va);
    assert_eq!(u32le(&bytes, rel_at + 4), (1 << 8) | 7);
    let (dynamic_at, dynamic_bytes, _, dynamic_link, _, dynamic_stride) =
        elf32_section(&bytes, ".dynamic");
    assert_eq!((dynamic_bytes, dynamic_link, dynamic_stride), (128, 6, 8));
    assert_eq!(
        (u32le(&bytes, dynamic_at), u32le(&bytes, dynamic_at + 4)),
        (1, 1)
    );
    assert_eq!(u32le(&bytes, dynamic_at + 13 * 8), 24);
    assert_eq!(u32le(&bytes, dynamic_at + 14 * 8), 30);
    assert_eq!(u32le(&bytes, dynamic_at + 15 * 8), 0);

    let checked = run(
        &native,
        &[
            "compiler",
            "final-image-check",
            runtime_fixture
                .to_str()
                .expect("UTF-8 runtime ELF32 fixture"),
            "i386-unknown-linux-gnu",
            "3",
            "12992",
            "134516816",
            "1",
            "1348400354",
        ],
    );
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );

    let arm_runtime = directory.join("runtime-armv7.elf");
    let emitted = run(
        &native,
        &[
            "compiler",
            "emit-executable",
            runtime_fixture
                .to_str()
                .expect("UTF-8 ARMv7 runtime fixture"),
            "--target",
            "armv7-unknown-linux-gnueabihf",
            "--out",
            arm_runtime.to_str().expect("UTF-8 ARMv7 runtime ELF32"),
            "--json",
        ],
    );
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stdout)
    );
    let report: Value = serde_json::from_slice(&emitted.stdout).expect("ARMv7 ELF32 JSON");
    assert_eq!(report["format"], "elf32");
    assert_eq!(report["bytes"], 12_992);
    assert_eq!(report["entry"], 65_632);
    assert_eq!(report["providers"], 1);
    assert_eq!(report["flags"], 3);
    assert_eq!(report["hash"], 300_964_810);

    let bytes = fs::read(&arm_runtime).expect("read ARMv7 dynamic ELF32 image");
    assert_eq!(u16le(&bytes, 18), 40);
    assert_eq!(u32le(&bytes, 36), 0x0500_0000);
    assert_eq!((u16le(&bytes, 44), u16le(&bytes, 48)), (5, 12));
    let (interp_at, interp_bytes, _, _, _, _) = elf32_section(&bytes, ".interp");
    assert_eq!(interp_bytes, 25);
    assert_eq!(object_string(&bytes, interp_at), "/lib/ld-linux-armhf.so.3");
    let (dynstr_at, _, _, _, _, _) = elf32_section(&bytes, ".dynstr");
    assert_eq!(object_string(&bytes, dynstr_at + 1), "libncruntime.so.2");
    assert_eq!(
        object_string(&bytes, dynstr_at + 19),
        "nC.runtime.output.println"
    );
    let (slot_at, slot_bytes, slot_va, _, _, slot_stride) = elf32_section(&bytes, ".got.plt");
    assert_eq!((slot_bytes, slot_va, slot_stride), (4, 0x0001_2000, 4));
    assert_eq!(u32le(&bytes, slot_at), 0);
    let (plt_at, plt_bytes, _, _, _, plt_stride) = elf32_section(&bytes, ".plt");
    assert_eq!((plt_bytes, plt_stride), (16, 16));
    assert_eq!(
        &bytes[plt_at..plt_at + 16],
        &[
            0x00, 0xc0, 0x9f, 0xe5, 0x00, 0xf0, 0x9c, 0xe5, 0x00, 0x20, 0x01, 0x00, 0x00, 0x00,
            0xa0, 0xe1,
        ]
    );
    let (rel_at, rel_bytes, _, rel_link, rel_info, rel_stride) = elf32_section(&bytes, ".rel.plt");
    assert_eq!((rel_bytes, rel_link, rel_info, rel_stride), (8, 7, 4, 8));
    assert_eq!(u32le(&bytes, rel_at), slot_va);
    assert_eq!(u32le(&bytes, rel_at + 4), (1 << 8) | 22);

    let checked = run(
        &native,
        &[
            "compiler",
            "final-image-check",
            runtime_fixture
                .to_str()
                .expect("UTF-8 ARMv7 runtime fixture"),
            "armv7-unknown-linux-gnueabihf",
            "3",
            "12992",
            "65632",
            "1",
            "300964810",
        ],
    );
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
}

#[test]
fn native_nc_macho64_final_images_cover_dyld_entry_bind_symbols_and_ad_hoc_signature() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native Mach-O64 final image: no host linker");
        return;
    };

    let directory = root().join("target/tmp/ncc-stage2-macho64-final-image-differential");
    fs::create_dir_all(&directory).expect("create Mach-O64 final image directory");
    let pure_fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_final_image_exit.n");
    let runtime_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");

    let targets = [
        (
            "x86_64-apple-darwin",
            0x0100_0007_u32,
            1_052_688_i64,
            6_187_006_i64,
            &[0x31, 0xff, 0xb8, 0x01, 0, 0, 0x02, 0x0f, 0x05][..],
        ),
        (
            "aarch64-apple-darwin",
            0x0100_000c,
            1_052_688,
            324_467_454,
            &[
                0x00, 0x00, 0x80, 0xd2, 0x30, 0x00, 0x80, 0xd2, 0x01, 0x10, 0x00, 0xd4,
            ][..],
        ),
    ];

    for (target, cpu, entry, hash, wrapper) in targets {
        let output = directory.join(format!("pure-{}.macho", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                pure_fixture.to_str().expect("UTF-8 Mach-O fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 Mach-O output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("Mach-O image JSON");
        assert_eq!(report["format"], "macho64");
        assert_eq!(report["format_id"], 4);
        assert_eq!(report["bytes"], 8_406);
        assert_eq!(report["entry"], entry);
        assert_eq!(report["providers"], 0);
        assert_eq!(report["flags"], 1);
        assert_eq!(report["hash"], hash);

        let bytes = fs::read(&output).expect("read pure Mach-O image");
        assert_eq!(u32le(&bytes, 0), 0xfeedfacf);
        assert_eq!(u32le(&bytes, 4), cpu);
        assert_eq!(u32le(&bytes, 12), 2);
        assert_eq!((u32le(&bytes, 16), u32le(&bytes, 20)), (10, 632));
        let (start_at, start_bytes, start_va, start_flags, _, _) = macho_section(&bytes, "__start");
        assert_eq!(
            (start_bytes, start_va, start_flags),
            (wrapper.len(), entry as u64, 0x80000400)
        );
        assert_eq!(&bytes[start_at..start_at + wrapper.len()], wrapper);
        assert_eq!(
            u64le(&bytes, macho_command(&bytes, 0x80000028) + 8),
            start_at as u64
        );
        assert_eq!(
            object_string(&bytes, macho_command(&bytes, 0x0e) + 24),
            "/usr/lib/dyld"
        );
        assert_eq!(macho_symbol_sections(&bytes, "_main"), vec![2]);
        assert_macho_ad_hoc_signature(&bytes);
    }

    let dynamic_targets = [
        (
            "x86_64-apple-darwin",
            0x0100_0007_u32,
            1_052_752_i64,
            854_950_660_i64,
            &[0xff, 0x25, 0xba, 0x0f, 0, 0, 0x66, 0x90][..],
            &[
                0xe8, 0xc8, 0xff, 0xff, 0xff, 0x89, 0xc7, 0xb8, 0x01, 0, 0, 0x02, 0x0f, 0x05,
            ][..],
        ),
        (
            "aarch64-apple-darwin",
            0x0100_000c,
            1_052_768,
            571_854_169,
            &[
                0x10, 0x00, 0x00, 0xb0, 0x10, 0x02, 0x40, 0xf9, 0x00, 0x02, 0x1f, 0xd6,
            ][..],
            &[
                0xf1, 0xff, 0xff, 0x97, 0x30, 0x00, 0x80, 0xd2, 0x01, 0x10, 0x00, 0xd4,
            ][..],
        ),
    ];

    for (target, cpu, entry, hash, stub, wrapper) in dynamic_targets {
        let output = directory.join(format!("runtime-{}.macho", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                runtime_fixture
                    .to_str()
                    .expect("UTF-8 runtime Mach-O fixture"),
                "--target",
                target,
                "--out",
                output.to_str().expect("UTF-8 runtime Mach-O output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("dynamic Mach-O JSON");
        assert_eq!(report["bytes"], 12_678);
        assert_eq!(report["entry"], entry);
        assert_eq!(report["providers"], 1);
        assert_eq!(report["flags"], 3);
        assert_eq!(report["hash"], hash);

        let bytes = fs::read(&output).expect("read dynamic Mach-O image");
        assert_eq!(u32le(&bytes, 4), cpu);
        assert_eq!((u32le(&bytes, 16), u32le(&bytes, 20)), (12, 928));
        let (stub_at, stub_bytes, _, stub_flags, stub_first, stub_stride) =
            macho_section(&bytes, "__stubs");
        assert_eq!(
            (stub_bytes, stub_flags, stub_first, stub_stride),
            (16, 0x80000408, 0, 16)
        );
        assert_eq!(&bytes[stub_at..stub_at + stub.len()], stub);
        let (start_at, start_bytes, start_va, _, _, _) = macho_section(&bytes, "__start");
        assert_eq!((start_bytes, start_va), (wrapper.len(), entry as u64));
        assert_eq!(&bytes[start_at..start_at + wrapper.len()], wrapper);
        let (slot_at, slot_bytes, slot_va, slot_flags, slot_first, _) =
            macho_section(&bytes, "__nl_symbol_ptr");
        assert_eq!(
            (slot_bytes, slot_va, slot_flags, slot_first),
            (8, 0x0010_2000, 6, 1)
        );
        assert_eq!(u64le(&bytes, slot_at), 0);
        assert_eq!(
            object_string(&bytes, macho_command(&bytes, 0x0c) + 24),
            "@loader_path/libncruntime.2.dylib"
        );
        let dyld_info = macho_command(&bytes, 0x80000022);
        let bind_at = u32le(&bytes, dyld_info + 16) as usize;
        let bind_bytes = u32le(&bytes, dyld_info + 20) as usize;
        assert_eq!(bind_bytes, 34);
        assert!(bytes[bind_at..bind_at + bind_bytes]
            .windows(b"_nC.runtime.output.println".len())
            .any(|window| window == b"_nC.runtime.output.println"));
        let dysymtab = macho_command(&bytes, 0x0b);
        assert_eq!(
            (u32le(&bytes, dysymtab + 20), u32le(&bytes, dysymtab + 28)),
            (2, 1)
        );
        assert_eq!(u32le(&bytes, dysymtab + 60), 2);
        assert_eq!(macho_symbol_sections(&bytes, "_main"), vec![3]);
        assert_eq!(
            macho_symbol_sections(&bytes, "_nC.runtime.output.println"),
            vec![0]
        );
        assert_macho_ad_hoc_signature(&bytes);
    }
}

#[test]
fn native_nc_wasm32_final_image_materializes_imports_names_mct_and_node_execution() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native WASM final image: no host linker");
        return;
    };

    let directory = root().join("target/tmp/ncc-stage2-wasm-final-image-differential");
    fs::create_dir_all(&directory).expect("create WASM final image directory");
    let pure_fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_final_image_exit.n");
    let runtime_fixture =
        root().join("bootstrap/corpus/ncc-stage2-lexer/mct_runtime_abi_hosted_main.n");
    let pure = directory.join("mct_final_image_exit.wasm");
    let runtime = directory.join("mct_runtime_abi_hosted_main.wasm");

    let mut reports = Vec::new();
    for (fixture, output, expected_bytes, expected_entry, expected_providers, expected_hash) in [
        (
            &pure_fixture,
            &pure,
            98_i64,
            0_i64,
            0_i64,
            1_996_057_186_i64,
        ),
        (&runtime_fixture, &runtime, 203, 1, 1, 1_918_190_602),
    ] {
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-executable",
                fixture.to_str().expect("UTF-8 WASM fixture"),
                "--target",
                "wasm32-wasi",
                "--out",
                output.to_str().expect("UTF-8 WASM output"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let report: Value = serde_json::from_slice(&emitted.stdout).expect("WASM image JSON");
        assert_eq!(report["schema"], "nc.mct.image.v1");
        assert_eq!(report["format"], "wasm");
        assert_eq!(report["format_id"], 5);
        assert_eq!(report["bytes"], expected_bytes);
        assert_eq!(report["entry"], expected_entry);
        assert_eq!(report["providers"], expected_providers);
        assert_eq!(report["flags"], if output == &pure { 1 } else { 3 });
        assert_eq!(report["hash"], expected_hash);

        let checked = run(
            &native,
            &[
                "compiler",
                "final-image-check",
                fixture.to_str().expect("UTF-8 WASM fixture"),
                "wasm32-wasi",
                "5",
                &expected_bytes.to_string(),
                &expected_entry.to_string(),
                &expected_providers.to_string(),
                &expected_hash.to_string(),
            ],
        );
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stdout)
        );
        reports.push(report);
    }

    let pure_bytes = fs::read(&pure).expect("read pure WASM image");
    assert_eq!(&pure_bytes[..8], b"\0asm\x01\0\0\0");
    assert_eq!(reports[0]["bytes"], pure_bytes.len());
    assert_eq!(reports[0]["entry"], 0);
    assert_eq!(reports[0]["providers"], 0);
    let (functions, _) = wasm_section(&pure_bytes, 3);
    let mut function_at = functions;
    assert_eq!(wasm_uleb(&pure_bytes, &mut function_at), 1);
    let (exports, _) = wasm_section(&pure_bytes, 7);
    let mut export_at = exports;
    assert_eq!(wasm_uleb(&pure_bytes, &mut export_at), 1);
    assert_eq!(wasm_name(&pure_bytes, &mut export_at), "_start");
    assert_eq!(pure_bytes[export_at], 0);
    export_at += 1;
    assert_eq!(wasm_uleb(&pure_bytes, &mut export_at), 0);
    let (mct_at, mct_bytes) = wasm_custom_section(&pure_bytes, "nC.mct");
    assert!(mct_bytes >= 32);
    assert_eq!(u32le(&pure_bytes, mct_at), 1);
    assert_eq!(u32le(&pure_bytes, mct_at + 4), 10);
    assert_eq!(u32le(&pure_bytes, mct_at + 8), 0);
    assert_eq!(u32le(&pure_bytes, mct_at + 12), 1);
    assert_eq!(u32le(&pure_bytes, mct_at + 16), 0);
    assert_eq!(u32le(&pure_bytes, mct_at + 28) as usize + 32, mct_bytes);

    let runtime_bytes = fs::read(&runtime).expect("read runtime WASM image");
    assert_eq!(reports[1]["bytes"], runtime_bytes.len());
    assert_eq!(reports[1]["entry"], 1);
    assert_eq!(reports[1]["providers"], 1);
    let (imports, _) = wasm_section(&runtime_bytes, 2);
    let mut import_at = imports;
    assert_eq!(wasm_uleb(&runtime_bytes, &mut import_at), 1);
    assert_eq!(
        wasm_name(&runtime_bytes, &mut import_at),
        "nC.runtime.output"
    );
    assert_eq!(wasm_name(&runtime_bytes, &mut import_at), "println");
    assert_eq!(runtime_bytes[import_at], 0);
    import_at += 1;
    assert_eq!(wasm_uleb(&runtime_bytes, &mut import_at), 0);
    let (names_at, names_bytes) = wasm_custom_section(&runtime_bytes, "name");
    assert!(runtime_bytes[names_at..names_at + names_bytes]
        .windows(b"nC.runtime.output.println".len())
        .any(|window| window == b"nC.runtime.output.println"));
    let (mct_at, mct_bytes) = wasm_custom_section(&runtime_bytes, "nC.mct");
    assert_eq!(u32le(&runtime_bytes, mct_at + 8), 1);
    assert_eq!(u32le(&runtime_bytes, mct_at + 12), 1);
    assert_eq!(u32le(&runtime_bytes, mct_at + 16), 1);
    assert_eq!(u32le(&runtime_bytes, mct_at + 28) as usize + 32, mct_bytes);

    let node = Command::new("node").arg("--version").output();
    if node.as_ref().is_ok_and(|output| output.status.success()) {
        for (path, imports) in [
            (&pure, "{}"),
            (
                &runtime,
                "{'nC.runtime.output':{println(){globalThis.called=(globalThis.called||0)+1}}}",
            ),
        ] {
            let script = format!(
                "const fs=require('fs');const b=fs.readFileSync(process.argv[1]);WebAssembly.instantiate(b,{imports}).then(r=>{{r.instance.exports._start();if(process.argv[2]==='runtime'&&globalThis.called!==1)process.exit(2)}}).catch(e=>{{console.error(e);process.exit(1)}})"
            );
            let executed = Command::new("node")
                .args(["-e", &script])
                .arg(path)
                .arg(if path == &runtime { "runtime" } else { "pure" })
                .output()
                .expect("execute WASM module with Node");
            assert!(
                executed.status.success(),
                "Node WASM smoke failed: {}",
                String::from_utf8_lossy(&executed.stderr)
            );
        }
    }
}

#[test]
fn native_nc_objects_bind_each_relocation_to_its_exact_target_symbol() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native object identity execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_link_targets.n");
    let fixture = fixture.to_str().expect("UTF-8 object identity fixture");
    let directory = root().join("target/tmp/ncc-stage2-object-identity-differential");
    fs::create_dir_all(&directory).expect("create object identity directory");

    for (target, expected_bytes, expected_hash) in [
        ("x86_64-unknown-linux-gnu", 3936, 113_159_738),
        ("x86_64-pc-windows-msvc", 2569, 51_420_332),
        ("x86_64-apple-darwin", 3562, 94_061_253),
        ("i386-unknown-linux-gnu", 3412, 86_738_170),
        ("aarch64-unknown-linux-gnu", 3968, 114_884_382),
        ("aarch64-apple-darwin", 3594, 96_311_936),
        ("armv7-unknown-linux-gnueabihf", 3468, 89_589_688),
        ("riscv32imac-unknown-elf", 3480, 88_573_562),
        ("riscv64gc-unknown-linux-gnu", 3968, 112_574_736),
        ("wasm32-wasi", 2751, 58_009_445),
    ] {
        let artifact = directory.join(format!("{}.object", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-object",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 object identity artifact"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("native object identity JSON");
        assert_eq!(document["bytes"], expected_bytes);
        assert_eq!(document["relocations"], 4);
        assert_eq!(document["symbols"], 4);
        assert_eq!(document["defined_symbols"], 3);
        assert_eq!(document["external_symbols"], 1);
        assert_eq!(document["control_symbols"], 1);
        assert_eq!(document["unresolved_symbols"], 0);
        assert_eq!(document["public_symbols"], 1);
        assert_eq!(document["private_symbols"], 1);
        assert_eq!(document["internal_symbols"], 0);
        assert_eq!(document["default_symbols"], 2);
        assert_eq!(document["identity_bytes"], 83);
        assert_eq!(document["identity_fingerprint"], 645_706);
        assert_eq!(document["mangle_bytes"], 103);
        assert_eq!(document["symbol_bundle_bytes"], 376);
        assert_eq!(document["symbol_bundle_records"], 4);
        assert_eq!(document["debug_bytes"], 1248);
        assert_eq!(document["debug_lines"], 56);
        assert_eq!(document["unwind_functions"], 2);
        assert_eq!(document["object_hash"], expected_hash);

        let bytes = fs::read(&artifact).expect("read object identity artifact");
        assert!(bytes.windows(4).any(|window| window == b"NCSY"));
        let prefix = if target.contains("apple-darwin") {
            "_"
        } else {
            ""
        };
        for symbol in [
            "nc$C$00000041",
            "nc$S$R$14$mctLinkTargets$4$step",
            "nc$S$P$14$mctLinkTargets$3$run",
            "nC.runtime.output.println",
        ] {
            let symbol = format!("{prefix}{symbol}");
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "{target} omitted exact target {symbol}"
            );
        }
        if target == "wasm32-wasi" {
            assert!(bytes.windows(11).any(|window| window == b"ncc.symbols"));
        }
    }

    let elf = directory.join("x86_64_unknown_linux_gnu.object");
    if let Ok(inspected) = Command::new("readelf")
        .args(["-Ws", "-r"])
        .arg(&elf)
        .output()
    {
        assert!(inspected.status.success());
        let report = String::from_utf8_lossy(&inspected.stdout);
        for symbol in [
            "nc$C$00000041",
            "nc$S$R$14$mctLinkTargets$4$step",
            "nc$S$P$14$mctLinkTargets$3$run",
            "nC.runtime.output.println",
        ] {
            assert!(report.contains(symbol), "readelf omitted {symbol}");
        }
        assert!(report.contains("LOCAL") && report.contains("GLOBAL"));
    }

    let coff = directory.join("x86_64_pc_windows_msvc.object");
    for tool in ["llvm-readobj-18", "llvm-readobj"] {
        if let Ok(inspected) = Command::new(tool)
            .args(["--symbols", "--relocations"])
            .arg(&coff)
            .output()
        {
            assert!(inspected.status.success());
            let report = String::from_utf8_lossy(&inspected.stdout);
            assert!(report.contains("nc$S$P$14$mctLinkTargets$3$run"));
            break;
        }
    }
}

#[test]
fn native_nc_source_spelling_container_and_visibility_survive_hash_collisions() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native source spelling execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-lexer/mct_symbol_spelling_collision.n");
    let fixture = fixture.to_str().expect("UTF-8 source spelling fixture");
    let directory = root().join("target/tmp/ncc-stage2-source-spelling-differential");
    fs::create_dir_all(&directory).expect("create source spelling directory");

    for (target, expected_bytes, expected_hash) in [
        ("x86_64-unknown-linux-gnu", 4592, 153_606_409),
        ("x86_64-pc-windows-msvc", 3117, 74_409_391),
        ("x86_64-apple-darwin", 4250, 132_250_548),
        ("i386-unknown-linux-gnu", 4036, 119_962_284),
        ("aarch64-unknown-linux-gnu", 4632, 156_436_064),
        ("aarch64-apple-darwin", 4290, 136_491_196),
        ("armv7-unknown-linux-gnueabihf", 4116, 125_763_363),
        ("riscv32imac-unknown-elf", 4128, 124_517_853),
        ("riscv64gc-unknown-linux-gnu", 4632, 152_348_081),
        ("wasm32-wasi", 3346, 84_708_833),
    ] {
        let artifact = directory.join(format!("{}.object", target.replace('-', "_")));
        let emitted = run(
            &native,
            &[
                "compiler",
                "emit-object",
                fixture,
                "--target",
                target,
                "--out",
                artifact.to_str().expect("UTF-8 source spelling artifact"),
                "--json",
            ],
        );
        assert!(
            emitted.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&emitted.stdout)
        );
        let document: Value =
            serde_json::from_slice(&emitted.stdout).expect("source spelling object JSON");
        assert_eq!(document["schema"], "nc.bootstrap.mct.object.v5");
        assert_eq!(document["bytes"], expected_bytes);
        assert_eq!(document["relocations"], 4);
        assert_eq!(document["symbols"], 4);
        assert_eq!(document["defined_symbols"], 3);
        assert_eq!(document["external_symbols"], 1);
        assert_eq!(document["control_symbols"], 0);
        assert_eq!(document["unresolved_symbols"], 0);
        assert_eq!(document["public_symbols"], 1);
        assert_eq!(document["private_symbols"], 1);
        assert_eq!(document["internal_symbols"], 1);
        assert_eq!(document["default_symbols"], 1);
        assert_eq!(document["identity_bytes"], 195);
        assert_eq!(document["identity_fingerprint"], 2_548_996);
        assert_eq!(document["mangle_bytes"], 159);
        assert_eq!(document["symbol_bundle_bytes"], 432);
        assert_eq!(document["symbol_bundle_records"], 4);
        assert_eq!(document["debug_bytes"], 1580);
        assert_eq!(document["debug_lines"], 71);
        assert_eq!(document["unwind_functions"], 3);
        assert_eq!(document["object_hash"], expected_hash);

        let bytes = fs::read(&artifact).expect("read source spelling object");
        assert!(bytes.windows(4).any(|window| window == b"NCSY"));
        let prefix = if target.contains("apple-darwin") {
            "_"
        } else {
            ""
        };
        for symbol in [
            "nc$S$R$26$mctSymbolSpellingCollision$5$sym1r",
            "nc$S$I$26$mctSymbolSpellingCollision$5$sym30",
            "nc$S$P$26$mctSymbolSpellingCollision$3$run",
            "nC.runtime.output.println",
        ] {
            let symbol = format!("{prefix}{symbol}");
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "{target} omitted collision-safe symbol {symbol}"
            );
        }
        if target == "wasm32-wasi" {
            assert!(bytes.windows(11).any(|window| window == b"ncc.symbols"));
        }
    }

    let elf = directory.join("x86_64_unknown_linux_gnu.object");
    if let Ok(inspected) = Command::new("readelf")
        .args(["-Ws", "-r"])
        .arg(&elf)
        .output()
    {
        assert!(inspected.status.success());
        let report = String::from_utf8_lossy(&inspected.stdout);
        assert!(report.contains("LOCAL") && report.contains("sym1r"));
        assert!(report.contains("GLOBAL") && report.contains("HIDDEN"));
        assert!(report.contains("sym30"));
        assert_eq!(report.matches("sym1r").count(), 2);
        assert_eq!(report.matches("sym30").count(), 2);
    }
}

#[test]
fn native_nc_project_owns_graph_identity_and_symbol_resolution() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native project execution: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-project");
    let project = root().join("target/tmp/ncc-stage2-project-differential");
    if project.exists() {
        fs::remove_dir_all(&project).expect("remove previous native project fixture");
    }
    fs::create_dir_all(project.join("src")).expect("create native project fixture");
    fs::copy(fixture.join("main.n"), project.join("main.n")).expect("copy project main");
    fs::copy(fixture.join("src/worker.n"), project.join("src/worker.n"))
        .expect("copy project worker");

    let sync = run(
        &native,
        &[
            "project",
            "sync",
            project.to_str().expect("UTF-8 project path"),
            "--main",
            "main.n",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--quiet",
        ],
    );
    assert!(
        sync.status.success(),
        "native project sync failed: {}",
        String::from_utf8_lossy(&sync.stdout)
    );

    let bridge: Value =
        serde_json::from_slice(&fs::read(project.join("bridge.json")).expect("read native bridge"))
            .expect("parse native bridge");
    assert_eq!(bridge["schema"], "ncc.bridge.v1");
    assert_eq!(bridge["linker"], "ncc.project.native-resolution.v1");
    assert_eq!(bridge["units"].as_array().map(Vec::len), Some(2));
    assert_eq!(bridge["edges"].as_array().map(Vec::len), Some(1));
    assert_eq!(bridge["link_order"][0], "worker");
    assert_eq!(bridge["link_order"][1], "main");
    assert_eq!(bridge["target"]["object_format"], "elf64");
    assert_eq!(bridge["linked_lto"]["status"], "materialized");
    assert_eq!(bridge["linked_lto"]["linked_units"], 2);
    assert_eq!(bridge["linked_lto"]["inlined_calls"], 1);
    assert_eq!(bridge["linked_lto"]["gvn_reuses"], 1);
    assert_eq!(bridge["linked_lto"]["retained_call_edges"], 1);
    assert_eq!(bridge["linked_lto"]["material_relocations"], 2);
    assert_eq!(bridge["linked_lto"]["owner_qualified_resolution"], true);
    assert_eq!(bridge["linked_lto"]["abi_uniform"], true);
    assert!(bridge["linked_lto"]["object_bytes"]
        .as_u64()
        .is_some_and(|bytes| bytes > 1000));
    assert!(project
        .join(
            bridge["linked_lto"]["object"]
                .as_str()
                .expect("linked LTO object path"),
        )
        .is_file());

    for unit in bridge["units"].as_array().expect("bridge units") {
        let hash = unit["source_hash"].as_str().expect("source hash");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
        let object_hash = unit["object_hash"].as_str().expect("object hash");
        assert!(object_hash.starts_with("sha256:"));
        assert_eq!(object_hash.len(), 71);
        assert!(unit["object_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 100));
        assert!(unit["mct_fingerprint"]
            .as_i64()
            .is_some_and(|fingerprint| fingerprint > 0));
        assert!(project
            .join(unit["code"].as_str().expect("code path"))
            .is_file());
        assert!(project
            .join(unit["object"].as_str().expect("object path"))
            .is_file());
    }

    let resolution: Value = serde_json::from_slice(
        &fs::read(project.join(".ncc/lowering/project.symbols.json"))
            .expect("read native resolution"),
    )
    .expect("parse native resolution");
    assert_eq!(resolution["schema"], "nc.bootstrap.project.resolution.v2");
    assert_eq!(resolution["status"], "resolved");
    assert_eq!(resolution["unit_count"], 2);
    assert_eq!(resolution["edge_count"], 1);
    assert_eq!(resolution["unresolved_count"], 0);
    assert_eq!(resolution["symbol_count"], 16);
    assert_eq!(resolution["reference_count"], 2);
    assert_eq!(resolution["resolved_reference_count"], 2);
    assert_eq!(resolution["visibility_denied_count"], 0);
    let bindings = resolution["bindings"].as_array().expect("project bindings");
    let symbols = resolution["symbols"].as_array().expect("project symbols");
    assert!(symbols.iter().all(|symbol| symbol["kind"] != "field"));
    assert!(bindings.iter().any(|binding| {
        binding["owner"] == "stage2ProjectWorker"
            && binding["name"] == "answer"
            && binding["visibility"] == "public"
    }));
    assert!(bindings.iter().any(|binding| {
        binding["owner"] == "stage2ProjectWorker"
            && binding["name"] == "projectOffset"
            && binding["visibility"] == "internal"
    }));

    let main_object =
        fs::read(project.join(".ncc/obj/l64/main.o")).expect("read native main object");
    assert_eq!(&main_object[..4], b"\x7fELF");
    assert!(main_object
        .windows(b"nc$S$P$19$stage2ProjectWorker$6$answer".len())
        .any(|window| window == b"nc$S$P$19$stage2ProjectWorker$6$answer"));
    assert!(main_object
        .windows(b"nc$S$I$19$stage2ProjectWorker$13$projectOffset".len())
        .any(|window| window == b"nc$S$I$19$stage2ProjectWorker$13$projectOffset"));
    let worker_object =
        fs::read(project.join(".ncc/obj/l64/worker.o")).expect("read native worker object");
    assert!(worker_object
        .windows(b"projectOffset".len())
        .any(|window| window == b"projectOffset"));
    assert!(worker_object
        .windows(b"unitSecret".len())
        .any(|window| window == b"unitSecret"));
    assert!(elf_symbol_sections(&worker_object, "unitSecret").is_empty());

    let resolve = run(
        &native,
        &[
            "project",
            "resolve",
            project.to_str().expect("UTF-8 project path"),
            "--main",
            "main.n",
            "--target",
            "riscv32imac-unknown-elf",
            "--quiet",
        ],
    );
    assert!(resolve.status.success());
    let riscv: Value =
        serde_json::from_slice(&fs::read(project.join("bridge.json")).expect("read RISC-V bridge"))
            .expect("parse RISC-V bridge");
    assert_eq!(riscv["target"]["pointer_width"], 32);
    assert_eq!(riscv["target"]["object_format"], "elf32");

    let private_fixture = root().join("bootstrap/corpus/ncc-stage2-project-private");
    let private_project = root().join("target/tmp/ncc-stage2-project-private-differential");
    if private_project.exists() {
        fs::remove_dir_all(&private_project).expect("remove previous private project fixture");
    }
    fs::create_dir_all(private_project.join("src")).expect("create private project fixture");
    fs::copy(
        private_fixture.join("main.n"),
        private_project.join("main.n"),
    )
    .expect("copy private project main");
    fs::copy(
        private_fixture.join("src/worker.n"),
        private_project.join("src/worker.n"),
    )
    .expect("copy private project worker");
    let denied = run(
        &native,
        &[
            "project",
            "resolve",
            private_project
                .to_str()
                .expect("UTF-8 private project path"),
            "--main",
            "main.n",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--quiet",
        ],
    );
    assert!(!denied.status.success());
    let denied_report: Value = serde_json::from_slice(
        &fs::read(private_project.join(".ncc/lowering/project.symbols.json"))
            .expect("read private resolution"),
    )
    .expect("parse private resolution");
    assert_eq!(denied_report["reference_count"], 1);
    assert_eq!(denied_report["resolved_reference_count"], 0);
    assert_eq!(denied_report["visibility_denied_count"], 1);
    assert_eq!(denied_report["unresolved_count"], 1);
}

#[test]
fn native_nc_project_link_writes_and_executes_a_self_contained_final_image() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native project final image: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-project");
    let project = root().join("target/tmp/ncc-stage2-project-native-final-image");
    if project.exists() {
        fs::remove_dir_all(&project).expect("remove previous native final-image fixture");
    }
    fs::create_dir_all(project.join("src")).expect("create native final-image fixture");
    fs::copy(fixture.join("main.n"), project.join("main.n")).expect("copy project main");
    fs::copy(fixture.join("src/worker.n"), project.join("src/worker.n"))
        .expect("copy project worker");

    let (target, file_name, magic): (&str, &str, &[u8]) = if cfg!(windows) {
        ("x86_64-pc-windows-msvc", "stage2-project.exe", b"MZ")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        (
            "aarch64-apple-darwin",
            "stage2-project",
            b"\xcf\xfa\xed\xfe",
        )
    } else if cfg!(target_os = "macos") {
        ("x86_64-apple-darwin", "stage2-project", b"\xcf\xfa\xed\xfe")
    } else {
        ("x86_64-unknown-linux-gnu", "stage2-project", b"\x7fELF")
    };
    let image = project.join(file_name);
    let linked = run(
        &native,
        &[
            "project",
            "link",
            project.to_str().expect("UTF-8 final-image project path"),
            "--main",
            "main.n",
            "--target",
            target,
            "--out",
            image.to_str().expect("UTF-8 final-image output path"),
            "--quiet",
        ],
    );
    assert!(
        linked.status.success(),
        "native project final-image link failed: {}",
        String::from_utf8_lossy(&linked.stdout)
    );
    let bytes = fs::read(&image).expect("read native project final image");
    assert!(bytes.starts_with(magic));
    let bridge: Value = serde_json::from_slice(
        &fs::read(project.join("bridge.json")).expect("read final-image bridge"),
    )
    .expect("parse final-image bridge");
    assert_eq!(bridge["linker"], "ncc.project.native-final-image.v1");
    assert_eq!(bridge["final_image"]["runtime_providers"], 0);
    assert_eq!(bridge["final_image"]["self_contained"], true);
    assert_eq!(
        bridge["final_image"]["bytes"].as_u64(),
        Some(bytes.len() as u64)
    );
    assert_eq!(
        bridge["final_image"]["sha256"].as_str().map(str::len),
        Some(64)
    );

    if cfg!(windows) {
        let executed = Command::new(&image)
            .output()
            .expect("execute self-contained native project PE");
        assert_eq!(
            executed.status.code(),
            Some(0),
            "native project PE failed: {}",
            String::from_utf8_lossy(&executed.stderr)
        );
    }

    let matrix_root = root().join("target/tmp/ncc-stage2-project-native-final-image-matrix");
    if matrix_root.exists() {
        fs::remove_dir_all(&matrix_root).expect("remove previous final-image matrix");
    }
    for (triple, expected_magic) in [
        ("x86_64-pc-windows-msvc", &b"MZ"[..]),
        ("x86_64-unknown-linux-gnu", &b"\x7fELF"[..]),
        ("x86_64-apple-darwin", &b"\xcf\xfa\xed\xfe"[..]),
        ("i386-unknown-linux-gnu", &b"\x7fELF"[..]),
        ("aarch64-unknown-linux-gnu", &b"\x7fELF"[..]),
        ("aarch64-apple-darwin", &b"\xcf\xfa\xed\xfe"[..]),
        ("armv7-unknown-linux-gnueabihf", &b"\x7fELF"[..]),
        ("riscv32imac-unknown-elf", &b"\x7fELF"[..]),
        ("riscv64gc-unknown-linux-gnu", &b"\x7fELF"[..]),
        ("wasm32-wasi", &b"\0asm"[..]),
    ] {
        let matrix_project = matrix_root.join(triple.replace('-', "_"));
        fs::create_dir_all(matrix_project.join("src")).expect("create target matrix project");
        fs::copy(fixture.join("main.n"), matrix_project.join("main.n"))
            .expect("copy target matrix main");
        fs::copy(
            fixture.join("src/worker.n"),
            matrix_project.join("src/worker.n"),
        )
        .expect("copy target matrix worker");
        let matrix_image = matrix_project.join("program.image");
        let linked = run(
            &native,
            &[
                "project",
                "link",
                matrix_project.to_str().expect("UTF-8 matrix project"),
                "--main",
                "main.n",
                "--target",
                triple,
                "--out",
                matrix_image.to_str().expect("UTF-8 matrix image"),
                "--no-incremental",
                "--quiet",
            ],
        );
        assert!(
            linked.status.success(),
            "native project image failed for {triple}: {}",
            String::from_utf8_lossy(&linked.stdout)
        );
        let bytes = fs::read(&matrix_image).expect("read target matrix image");
        assert!(
            bytes.starts_with(expected_magic),
            "{triple} image has the wrong loader magic"
        );
    }
}

#[test]
fn native_nc_project_content_cache_reuses_and_invalidates_material_artifacts() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native project content cache: no host linker");
        return;
    };

    let fixture = root().join("bootstrap/corpus/ncc-stage2-project");
    let project = root().join("target/tmp/ncc-stage2-content-cache-differential/project");
    let cache = root().join("target/tmp/ncc-stage2-content-cache-differential/cache");
    let parent = project.parent().expect("content cache test parent");
    if parent.exists() {
        fs::remove_dir_all(parent).expect("remove previous content cache fixture");
    }
    fs::create_dir_all(project.join("src")).expect("create content cache project");
    fs::copy(fixture.join("main.n"), project.join("main.n")).expect("copy cache main");
    fs::copy(fixture.join("src/worker.n"), project.join("src/worker.n"))
        .expect("copy cache worker");

    let cold = run_project_sync_json(&native, &project, &cache, "x86_64-unknown-linux-gnu", true);
    assert_eq!(cold["incremental"]["ast_hits"], 0);
    assert_eq!(cold["incremental"]["ast_misses"], 2);
    assert_eq!(cold["incremental"]["material_hits"], 0);
    assert_eq!(cold["incremental"]["material_misses"], 3);
    assert_eq!(cold["linked_lto"]["material_cache_hit"], false);
    assert_eq!(cold["linked_lto"]["inlined_calls"], 1);
    assert_eq!(cold["linked_lto"]["gvn_reuses"], 1);
    assert_eq!(cold["linked_lto"]["retained_call_edges"], 1);
    assert_eq!(cold["linked_lto"]["material_relocations"], 2);
    assert_eq!(
        cold["mct_link_readiness"]["schema"],
        "nc.bootstrap.mct.project-link-readiness.v1"
    );
    assert_eq!(
        cold["mct_link_readiness"]["intrinsic_targets_pending_selection"],
        0
    );
    assert_eq!(cold["mct_link_readiness"]["selected_intrinsics"], 0);
    assert_eq!(cold["mct_link_readiness"]["runtime_import_targets"], 0);
    assert_eq!(cold["mct_link_readiness"]["project_external_targets"], 2);
    assert_eq!(cold["mct_link_readiness"]["unresolved_targets"], 0);
    assert_eq!(cold["mct_link_readiness"]["ready"], true);

    let main_object = project.join(".ncc/obj/l64/main.o");
    let worker_object = project.join(".ncc/obj/l64/worker.o");
    let cold_main = fs::read(&main_object).expect("read cold main object");
    let cold_worker = fs::read(&worker_object).expect("read cold worker object");

    let warm = run_project_sync_json(&native, &project, &cache, "x86_64-unknown-linux-gnu", true);
    assert_eq!(warm["incremental"]["ast_hits"], 2);
    assert_eq!(warm["incremental"]["ast_misses"], 0);
    assert_eq!(warm["incremental"]["material_hits"], 3);
    assert_eq!(warm["incremental"]["material_misses"], 0);
    assert_eq!(warm["linked_lto"]["material_cache_hit"], true);
    assert_eq!(warm["linked_lto"]["gvn_reuses"], 1);
    assert_eq!(warm["mct_link_readiness"], cold["mct_link_readiness"]);
    assert!(warm["incremental"]["restored_bytes"]
        .as_u64()
        .is_some_and(|bytes| bytes > 0));
    assert_eq!(fs::read(&main_object).expect("read warm main"), cold_main);
    assert_eq!(
        fs::read(&worker_object).expect("read warm worker"),
        cold_worker
    );
    let warm_status: Value = serde_json::from_slice(
        &fs::read(project.join(".ncc/incremental/project-cache.json"))
            .expect("read warm cache status"),
    )
    .expect("parse warm cache status");
    let key_for = |status: &Value, wanted: &str| -> String {
        status["units"]
            .as_array()
            .expect("cache status units")
            .iter()
            .find(|unit| unit["unit"] == wanted)
            .and_then(|unit| unit["material_key"].as_str())
            .expect("unit material key")
            .to_string()
    };
    let warm_main_key = key_for(&warm_status, "main");
    let warm_worker_key = key_for(&warm_status, "worker");
    let warm_main_manifest: Value = serde_json::from_slice(
        &fs::read(
            cache
                .join("c2/m")
                .join(&warm_main_key[..20])
                .join("manifest.json"),
        )
        .expect("read warm main cache manifest"),
    )
    .expect("parse warm main cache manifest");
    assert_eq!(
        warm_main_manifest["compiler_identity"],
        "ncc-stage2-reserved-pointer-identity-v33"
    );

    let worker_source = project.join("src/worker.n");
    let changed = fs::read_to_string(&worker_source)
        .expect("read worker source")
        .replace("MATERIAL >> int <- 42;", "MATERIAL >> int <- 43;");
    fs::write(&worker_source, changed).expect("change worker implementation");
    let partial =
        run_project_sync_json(&native, &project, &cache, "x86_64-unknown-linux-gnu", true);
    assert_eq!(partial["incremental"]["ast_hits"], 1);
    assert_eq!(partial["incremental"]["ast_misses"], 1);
    assert_eq!(partial["incremental"]["material_hits"], 1);
    assert_eq!(partial["incremental"]["material_misses"], 2);
    assert_eq!(fs::read(&main_object).expect("read reused main"), cold_main);

    let status: Value = serde_json::from_slice(
        &fs::read(project.join(".ncc/incremental/project-cache.json"))
            .expect("read content cache status"),
    )
    .expect("parse content cache status");
    assert_eq!(key_for(&status, "main"), warm_main_key);
    let worker_key = key_for(&status, "worker");
    assert_ne!(worker_key, warm_worker_key);
    let cached_object = cache
        .join("c2/m")
        .join(&worker_key[..20])
        .join("object.bin");
    fs::write(&cached_object, b"corrupt").expect("corrupt cached worker object");
    let recovered =
        run_project_sync_json(&native, &project, &cache, "x86_64-unknown-linux-gnu", true);
    assert_eq!(recovered["incremental"]["ast_hits"], 2);
    assert_eq!(recovered["incremental"]["material_hits"], 2);
    assert_eq!(recovered["incremental"]["material_misses"], 1);
    assert!(
        fs::metadata(&cached_object)
            .expect("rebuilt cached object")
            .len()
            > b"corrupt".len() as u64
    );

    let other_target =
        run_project_sync_json(&native, &project, &cache, "aarch64-unknown-linux-gnu", true);
    assert_eq!(other_target["incremental"]["ast_hits"], 2);
    assert_eq!(other_target["incremental"]["material_hits"], 0);
    assert_eq!(other_target["incremental"]["material_misses"], 3);

    let disabled =
        run_project_sync_json(&native, &project, &cache, "x86_64-unknown-linux-gnu", false);
    assert_eq!(disabled["incremental"]["enabled"], false);
    assert_eq!(disabled["incremental"]["ast_hits"], 0);
    assert_eq!(disabled["incremental"]["material_hits"], 0);
    let disabled_status: Value = serde_json::from_slice(
        &fs::read(project.join(".ncc/incremental/project-cache.json"))
            .expect("read disabled cache status"),
    )
    .expect("parse disabled cache status");
    assert_eq!(disabled_status["probes"], 0);
    assert_eq!(disabled_status["stores"], 0);
}

#[test]
fn native_nc_final_relocation_engine_patches_every_target_family() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native relocation execution: no host linker");
        return;
    };

    for arguments in [
        [
            "x86_64-unknown-linux-gnu",
            "x86-pcrel32",
            "4608",
            "4097",
            "-4",
            "507",
            "0",
        ],
        [
            "x86_64-pc-windows-msvc",
            "x86-abs64",
            "305419896",
            "0",
            "16",
            "305419912",
            "0",
        ],
        [
            "x86_64-apple-darwin",
            "x86-branch32",
            "8192",
            "4096",
            "-4",
            "4092",
            "0",
        ],
        [
            "i386-unknown-linux-gnu",
            "x86-abs32",
            "270544960",
            "0",
            "4",
            "270544964",
            "0",
        ],
        [
            "aarch64-unknown-linux-gnu",
            "aarch64-call26",
            "4352",
            "4096",
            "0",
            "-1811939264",
            "0",
        ],
        [
            "aarch64-apple-darwin",
            "aarch64-adrp-page21",
            "305418240",
            "301989888",
            "0",
            "-1342170592",
            "0",
        ],
        [
            "armv7-unknown-linux-gnueabihf",
            "arm-call",
            "4352",
            "4096",
            "0",
            "-352321472",
            "0",
        ],
        [
            "riscv32imac-unknown-elf",
            "riscv-branch",
            "4352",
            "4096",
            "0",
            "268435555",
            "0",
        ],
        [
            "riscv64gc-unknown-linux-gnu",
            "riscv-call",
            "8756",
            "4096",
            "0",
            "4119",
            "591429863",
        ],
        [
            "wasm32-wasi",
            "wasm-function-index",
            "300",
            "0",
            "0",
            "-2139061588",
            "0",
        ],
    ] {
        let mut command = vec!["compiler", "relocation-check"];
        command.extend(arguments);
        let checked = run(&native, &command);
        assert!(
            checked.status.success(),
            "{} {}: {}",
            arguments[0],
            arguments[1],
            String::from_utf8_lossy(&checked.stdout)
        );
    }

    let rejected = run(
        &native,
        &[
            "compiler",
            "relocation-check",
            "x86_64-unknown-linux-gnu",
            "aarch64-call26",
            "4352",
            "4096",
            "0",
            "-1811939264",
            "0",
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("E_MACHINE_LINK_RELOCATION_001"));
}

#[test]
fn native_nc_sema_owns_import_runtime_and_modal_diagnostics() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native import semantic diagnostics: no host linker");
        return;
    };
    for (fixture, code) in [
        ("sema_import_unknown_domain.n", "E_USE_DOMAIN_UNKNOWN_001"),
        ("sema_import_unknown_family.n", "E_USE_FAMILY_UNKNOWN_001"),
        ("sema_import_unknown_item.n", "E_USE_ITEM_UNKNOWN_001"),
        ("sema_import_wildcard.n", "E_USE_WILDCARD_DENIED_001"),
        ("sema_import_machine.n", "E_USE_FORBIDDEN_IN_MACHINE_001"),
        ("sema_runtime_use_missing.n", "E_RUNTIME_USE_001"),
    ] {
        let path = root()
            .join("bootstrap/corpus/ncc-stage2-lexer")
            .join(fixture);
        let rejected = run(
            &native,
            &[
                "compiler",
                "sema-imports",
                path.to_str().expect("UTF-8 invalid import fixture path"),
                "--json",
            ],
        );
        assert_eq!(rejected.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&rejected.stdout)
        );
    }
}

#[test]
fn native_nc_lexer_owns_errors_and_marks_unmigrated_stage0_commands() {
    let Some(native) = build_native_lexer() else {
        eprintln!("skipping native lexer error execution: no host linker");
        return;
    };
    for (fixture, code) in [
        (
            "bootstrap/corpus/ncc-stage2-lexer/invalid_escape.n",
            "E_LEXER_ESCAPE_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/invalid_hex_escape.n",
            "E_LEXER_ESCAPE_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/invalid_unicode_escape.n",
            "E_LEXER_ESCAPE_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/invalid_multi_char.n",
            "E_LEXER_TOKEN_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/invalid_wide_char.n",
            "E_LEXER_TOKEN_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/unterminated_string.n",
            "E_LEXER_STRING_UNTERMINATED_001",
        ),
        (
            "bootstrap/corpus/ncc-stage2-lexer/unterminated_comment.n",
            "E_LEXER_COMMENT_UNTERMINATED_001",
        ),
    ] {
        let output = run(&native, &["compiler", "tokens", fixture, "--json"]);
        assert_eq!(output.status.code(), Some(1), "{fixture}");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(code),
            "{fixture}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    let boundary = run(&native, &["compiler", "ast", "unused.n"]);
    assert_eq!(boundary.status.code(), Some(69));
    assert!(String::from_utf8_lossy(&boundary.stdout).contains("E_BOOTSTRAP_STAGE0_REQUIRED_001"));
}
