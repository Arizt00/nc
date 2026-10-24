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
        .expect("workspace root")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ncc-{label}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn command_available(command: &str) -> bool {
    Command::new(command).arg("--version").output().is_ok()
}

fn run(command: &str, args: &[String], cwd: &Path) -> Output {
    Command::new(command)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("failed to start {command}: {error}"))
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn require_tool(command: &str) -> bool {
    let available = command_available(command);
    if !available && std::env::var_os("NCC_ABI_LAYOUT_REQUIRE_HOST").is_some() {
        panic!("required ABI certification tool is unavailable: {command}");
    }
    available
}

const GNU_LAYOUT_PROBE: &str = r#"
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long long u64;
#define OFF(type, field) __builtin_offsetof(type, field)

struct __attribute__((packed, aligned(8))) Header {
    u32 flags;
    u8 code;
    u8 tail[];
};
union Value { u32 word; u64 wide; };
struct Wire { u32 value; };
struct __attribute__((aligned(16))) CacheLine { u64 key; u8 pad[24]; };
struct Pair { u16 left; u16 right; };
struct Frame { struct Pair pair; u16 lanes[2]; };
struct Cell { u16 mark; };
struct Grid { struct Cell cells[2]; };
struct Matrix { u16 values[2][2]; };

_Static_assert(sizeof(struct Header) == 8, "Header size");
_Static_assert(_Alignof(struct Header) == 8, "Header align");
_Static_assert(OFF(struct Header, code) == 4, "Header.code offset");
_Static_assert(sizeof(union Value) == 8, "Value size");
_Static_assert(sizeof(struct Wire) == 4, "Wire size");
_Static_assert(sizeof(struct CacheLine) == 32, "CacheLine size");
_Static_assert(_Alignof(struct CacheLine) == 16, "CacheLine align");
_Static_assert(sizeof(struct Pair) == 4, "Pair size");
_Static_assert(sizeof(struct Frame) == 8, "Frame size");
_Static_assert(OFF(struct Frame, lanes) == 4, "Frame.lanes offset");
_Static_assert(sizeof(struct Grid) == 4, "Grid size");
_Static_assert(sizeof(struct Matrix) == 8, "Matrix size");

struct Pair nc_abi_pair_roundtrip(struct Pair value) { return value; }
u64 nc_abi_layout_fingerprint(void) {
    return (u64)sizeof(struct Header) << 48 |
           (u64)sizeof(union Value) << 32 |
           (u64)sizeof(struct Frame) << 16 |
           (u64)sizeof(struct Matrix);
}
int main(void) {
    struct Pair input = { 7, 11 };
    struct Pair output = nc_abi_pair_roundtrip(input);
    return output.left == 7 && output.right == 11 ? 0 : 91;
}
"#;

#[cfg(target_os = "windows")]
const MSVC_LAYOUT_PROBE: &str = r#"
#include <stddef.h>
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned __int64 u64;
#define OFF(type, field) offsetof(type, field)

#pragma pack(push, 1)
typedef __declspec(align(8)) struct Header { u32 flags; u8 code; u8 tail[1]; } Header;
#pragma pack(pop)
typedef union Value { u32 word; u64 wide; } Value;
typedef struct Wire { u32 value; } Wire;
typedef __declspec(align(16)) struct CacheLine { u64 key; u8 pad[24]; } CacheLine;
typedef struct Pair { u16 left; u16 right; } Pair;
typedef struct Frame { Pair pair; u16 lanes[2]; } Frame;
typedef struct Cell { u16 mark; } Cell;
typedef struct Grid { Cell cells[2]; } Grid;
typedef struct Matrix { u16 values[2][2]; } Matrix;

_Static_assert(sizeof(Header) == 8, "Header size");
_Static_assert(__alignof(Header) == 8, "Header align");
_Static_assert(OFF(Header, code) == 4, "Header.code offset");
_Static_assert(sizeof(Value) == 8, "Value size");
_Static_assert(sizeof(Wire) == 4, "Wire size");
_Static_assert(sizeof(CacheLine) == 32, "CacheLine size");
_Static_assert(__alignof(CacheLine) == 16, "CacheLine align");
_Static_assert(sizeof(Pair) == 4, "Pair size");
_Static_assert(sizeof(Frame) == 8, "Frame size");
_Static_assert(OFF(Frame, lanes) == 4, "Frame.lanes offset");
_Static_assert(sizeof(Grid) == 4, "Grid size");
_Static_assert(sizeof(Matrix) == 8, "Matrix size");

Pair nc_abi_pair_roundtrip(Pair value) { return value; }
u64 nc_abi_layout_fingerprint(void) {
    return (u64)sizeof(Header) << 48 |
           (u64)sizeof(Value) << 32 |
           (u64)sizeof(Frame) << 16 |
           (u64)sizeof(Matrix);
}
int main(void) {
    Pair input = { 7, 11 };
    Pair output = nc_abi_pair_roundtrip(input);
    return output.left == 7 && output.right == 11 ? 0 : 91;
}
"#;

fn assert_elf_machine(path: &Path, expected: u16) {
    let bytes = std::fs::read(path).expect("ELF object bytes");
    assert!(
        bytes.len() >= 20,
        "truncated ELF object: {}",
        path.display()
    );
    assert_eq!(&bytes[..4], b"\x7fELF", "not ELF: {}", path.display());
    let machine = if bytes[5] == 1 {
        u16::from_le_bytes([bytes[18], bytes[19]])
    } else {
        u16::from_be_bytes([bytes[18], bytes[19]])
    };
    assert_eq!(machine, expected, "wrong ELF machine: {}", path.display());
}

#[cfg(target_os = "windows")]
fn assert_coff_machine(path: &Path, expected: u16) {
    let bytes = std::fs::read(path).expect("COFF object bytes");
    assert!(
        bytes.len() >= 20,
        "truncated COFF object: {}",
        path.display()
    );
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        expected,
        "wrong COFF machine: {}",
        path.display()
    );
}

#[test]
fn c_abi_layout_is_certified_by_host_and_cross_compilers() {
    let temp = unique_temp_dir("abi-layout-c");
    let source = temp.join("layout_probe.c");
    std::fs::write(&source, GNU_LAYOUT_PROBE).expect("GNU layout probe");

    if require_tool("clang") {
        for (target, machine) in [
            ("x86_64-unknown-linux-gnu", 62),
            ("aarch64-unknown-linux-gnu", 183),
            ("riscv64-unknown-linux-gnu", 243),
        ] {
            let object = temp.join(format!("clang-{}.o", target.replace('-', "_")));
            let output = run(
                "clang",
                &[
                    format!("--target={target}"),
                    "-std=c11".into(),
                    "-ffreestanding".into(),
                    "-c".into(),
                    source.to_string_lossy().to_string(),
                    "-o".into(),
                    object.to_string_lossy().to_string(),
                ],
                &temp,
            );
            assert!(output.status.success(), "{target}: {}", rendered(&output));
            assert_elf_machine(&object, machine);
        }
    }

    #[cfg(target_os = "linux")]
    for (compiler, machine) in [
        ("gcc", 62),
        ("aarch64-linux-gnu-gcc", 183),
        ("riscv64-linux-gnu-gcc", 243),
    ] {
        if !require_tool(compiler) {
            continue;
        }
        let object = temp.join(format!("{compiler}.o"));
        let output = run(
            compiler,
            &[
                "-std=c11".into(),
                "-ffreestanding".into(),
                "-c".into(),
                source.to_string_lossy().to_string(),
                "-o".into(),
                object.to_string_lossy().to_string(),
            ],
            &temp,
        );
        assert!(output.status.success(), "{compiler}: {}", rendered(&output));
        assert_elf_machine(&object, machine);
    }

    #[cfg(target_os = "windows")]
    {
        let source = temp.join("layout_probe_msvc.c");
        let object = temp.join("layout_probe_msvc.obj");
        std::fs::write(&source, MSVC_LAYOUT_PROBE).expect("MSVC layout probe");
        if require_tool("cl.exe") {
            let output = run(
                "cl.exe",
                &[
                    "/nologo".into(),
                    "/std:c11".into(),
                    "/c".into(),
                    source.to_string_lossy().to_string(),
                    format!("/Fo{}", object.to_string_lossy()),
                ],
                &temp,
            );
            assert!(output.status.success(), "MSVC: {}", rendered(&output));
            assert_coff_machine(&object, 0x8664);
        }
    }

    std::fs::remove_dir_all(temp).expect("temporary C ABI tree clean");
}

#[test]
fn nc_explicit_layout_emits_real_multitarget_objects() {
    let root = workspace_root();
    let source = root.join("examples/03_types/explicitBinaryLayout.n");
    let check = Command::new(ncc_bin())
        .current_dir(&root)
        .args(["check", source.to_string_lossy().as_ref()])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc check");
    assert!(check.status.success(), "{}", rendered(&check));

    let temp = unique_temp_dir("abi-layout-nc");
    let targets: &[(&str, u16)] = if cfg!(target_os = "linux") {
        &[
            ("x86_64-unknown-linux-gnu", 62),
            ("aarch64-unknown-linux-gnu", 183),
            ("riscv64-unknown-elf", 243),
        ]
    } else {
        &[]
    };
    for (target, machine) in targets {
        let out_dir = temp.join(target.replace('-', "_"));
        let output = Command::new(ncc_bin())
            .current_dir(&root)
            .args([
                "build",
                source.to_string_lossy().as_ref(),
                "--profile",
                "kernel",
                "--target",
                target,
                "--out-dir",
                out_dir.to_string_lossy().as_ref(),
                "--json",
            ])
            .env("NC_NO_COLOR", "1")
            .output()
            .expect("ncc kernel layout build");
        assert!(output.status.success(), "{target}: {}", rendered(&output));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("{target} report: {error}\n{}", rendered(&output)));
        assert_eq!(report["status"], "kernel_build_passed", "{target}");
        let object = PathBuf::from(
            report["paths"]["high_level_body_object"]
                .as_str()
                .expect("high-level object path"),
        );
        assert_elf_machine(&object, *machine);
    }

    #[cfg(target_os = "windows")]
    {
        let out_dir = temp.join("windows_msvc");
        let output = Command::new(ncc_bin())
            .current_dir(&root)
            .args([
                "build",
                source.to_string_lossy().as_ref(),
                "--profile",
                "kernel",
                "--target",
                "x86_64-pc-windows-msvc",
                "--out-dir",
                out_dir.to_string_lossy().as_ref(),
                "--json",
            ])
            .env("NC_NO_COLOR", "1")
            .output()
            .expect("ncc Windows layout build");
        assert!(output.status.success(), "{}", rendered(&output));
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("Windows build report");
        let object = PathBuf::from(
            report["paths"]["high_level_body_object"]
                .as_str()
                .expect("high-level COFF path"),
        );
        assert_coff_machine(&object, 0x8664);
    }

    std::fs::remove_dir_all(temp).expect("temporary nC ABI tree clean");
}
