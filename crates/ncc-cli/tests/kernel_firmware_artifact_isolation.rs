use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-kernel-firmware-artifact-isolation-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp directory");
    path
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!(
            "cannot read retained artifact '{}': {error}",
            path.display()
        )
    });
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn kernel_firmware_targets_retain_isolated_canonical_artifacts() {
    let temp = unique_temp_dir();
    let source = temp.join("firmware_isolation.n");
    let artifact_root = temp.join(".ncc");
    fs::write(
        &source,
        r#"kernelFirmwareArtifactIsolation {
    inspect() -> bool {
        guard bootValid() else return false;
        guard bootMemoryCount() > 0 else return false;
        framebuffer = bootFramebuffer();
        guard framebuffer == null || (
            bootFramebufferBase() != null &&
            bootFramebufferWidth() > 0 &&
            bootFramebufferHeight() > 0 &&
            bootFramebufferPitch() > 0
        ) else return false;
        return bootMemorySize(0) > 0;
    }

    main: inspect;
}
"#,
    )
    .expect("kernel source");

    let targets = [
        "x86_64-unknown-none",
        "aarch64-unknown-none",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
        "i386-unknown-none-elf",
    ];
    let mut retained = Vec::new();

    for target in targets {
        let output = Command::new(ncc_bin())
            .args([
                "build",
                source.to_string_lossy().as_ref(),
                "--profile",
                "kernel",
                "--target",
                target,
                "--out-dir",
                artifact_root.to_string_lossy().as_ref(),
                "--json",
            ])
            .current_dir(&temp)
            .env("NC_NO_COLOR", "1")
            .output()
            .expect("ncc executes");
        assert!(
            output.status.success(),
            "kernel build failed for {target}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stem = "firmware_isolation";
        let paths = [
            artifact_root
                .join("code")
                .join(format!("{stem}.{target}.kernel.nm")),
            artifact_root
                .join("obj")
                .join(format!("{stem}.{target}.kernel.obj")),
            artifact_root
                .join("obj")
                .join(format!("{stem}.{target}.kernel.body.o")),
            artifact_root
                .join("lowering")
                .join(format!("{stem}.{target}.kernel.body.ll")),
            artifact_root
                .join("lowering")
                .join(format!("{stem}.{target}.kernel.lowering.json")),
            artifact_root
                .join("run")
                .join(format!("{stem}.{target}.kernel.map")),
            artifact_root
                .join("run")
                .join(format!("{stem}.{target}.kernel-build.json")),
        ];
        for path in paths {
            let metadata = fs::metadata(&path).unwrap_or_else(|error| {
                panic!("missing canonical artifact '{}': {error}", path.display())
            });
            assert!(
                metadata.is_file() && metadata.len() > 0,
                "{}",
                path.display()
            );
            retained.push((path.clone(), sha256(&path)));
        }

        let manifest_path = artifact_root
            .join("run")
            .join(format!("{stem}.{target}.kernel-build.json"));
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(&manifest_path).expect("target-qualified manifest must read"),
        )
        .expect("target-qualified manifest must be JSON");
        assert_eq!(manifest["target"]["triple"], target);

        for (path, expected) in &retained {
            assert_eq!(
                sha256(path),
                *expected,
                "later target build overwrote '{}'",
                path.display()
            );
        }
    }

    assert!(artifact_root
        .join("run/firmware_isolation.kernel-build.json")
        .is_file());
    fs::remove_dir_all(temp).expect("temp cleanup");
}
