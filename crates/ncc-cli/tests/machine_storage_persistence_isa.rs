use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/machine")
        .join(name)
}

fn output(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("ncc-storage-isa-{name}-{nonce}.bin"))
}

fn emit(name: &str) -> Vec<u8> {
    let destination = output(name);
    let result = Command::new(ncc())
        .args(["machine", "emit-bytes"])
        .arg(example(name))
        .arg("--out")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(&destination).unwrap();
    let _ = fs::remove_file(destination);
    bytes
}

#[test]
fn storage_cache_maintenance_examples_validate_and_emit_exact_bytes() {
    assert_eq!(
        emit("kernel_storage_persistence_aarch64.nm"),
        [
            0xd50b_7a20_u32,
            0xd50b_7e21,
            0xd50b_7422,
            0xd503_3b9f,
            0xd50b_7520,
            0xd503_3fdf,
            0xd65f_03c0,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>()
    );
    assert_eq!(
        emit("kernel_storage_persistence_riscv64.nm"),
        [
            0x0015_200f_u32,
            0x0025_a00f,
            0x0006_200f,
            0x0046_a00f,
            0x0ff0_000f,
            0x0000_8067,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>()
    );
}
