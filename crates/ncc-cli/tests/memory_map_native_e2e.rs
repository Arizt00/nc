#![cfg(all(any(unix, windows), target_arch = "x86_64"))]

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

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-memory-map-native-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("binary parent").to_path_buf()];
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
        .expect("ncc command")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn native_profile() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("nC.win64", "x86_64-pc-windows-msvc")
    } else {
        ("nC.sysv64", "x86_64-unknown-linux-gnu")
    }
}

#[test]
fn memory_map_family_builds_and_executes_through_the_normal_native_route() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }

    let root = workspace_root();
    let out_dir = unique_temp_dir();
    let (cc, target) = native_profile();
    let fixtures = [
        "memoryMapNativeSafe.n",
        "memoryMapNativeError.n",
        "memoryMapVariants.n",
        "memoryMapFileVariants.n",
    ];
    let mut inspected_runtime = false;

    for fixture in fixtures {
        let source = root.join("examples/06_memory").join(fixture);
        let build = run_ncc(&[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--cc".into(),
            cc.into(),
            "--target".into(),
            target.into(),
            "--profile".into(),
            "release-fast".into(),
            "--json".into(),
        ]);
        assert!(build.status.success(), "{}", rendered(&build));
        let json: serde_json::Value = serde_json::from_slice(&build.stdout)
            .unwrap_or_else(|err| panic!("native build JSON: {err}\n{}", rendered(&build)));
        assert_eq!(json["status"], "native_build_passed");
        assert_eq!(json["optimization"]["route"], "standard-mct-machine");

        let executable = PathBuf::from(
            json["paths"]["executable"]
                .as_str()
                .expect("executable path"),
        );
        let execution = Command::new(&executable)
            .current_dir(&root)
            .output()
            .expect("external memory-map program");
        assert!(
            execution.status.success(),
            "{} exited with {}\n{}",
            executable.display(),
            execution.status,
            rendered(&execution)
        );

        if !inspected_runtime {
            let llvm = PathBuf::from(
                json["paths"]["mct_llvm_ir"]
                    .as_str()
                    .expect("MCT LLVM path"),
            );
            let llvm = std::fs::read_to_string(llvm).expect("MCT LLVM module");
            for symbol in [
                "__ncc_rt_map_default",
                "__ncc_rt_map_anonymous",
                "__ncc_rt_map_at",
                "__ncc_rt_map_shared",
                "__ncc_rt_map_private",
                "__ncc_rt_map_file",
                "__ncc_rt_map_file_shared",
                "__ncc_rt_map_file_private",
                "__ncc_rt_map_unmap",
                "__ncc_rt_map_remap",
                "__ncc_rt_map_remap_at",
                "__ncc_rt_map_protect",
                "__ncc_rt_map_sync",
                "__ncc_rt_map_advise",
                "__ncc_rt_map_lock",
                "__ncc_rt_map_unlock",
                "__ncc_rt_map_len",
                "__ncc_rt_map_address",
                "__ncc_rt_map_is_mapped",
            ] {
                assert!(llvm.contains(&format!("@{symbol}")), "missing {symbol}");
            }
            inspected_runtime = true;
        }
    }

    let raw = root.join("examples/06_memory/memoryMapComplete.n");
    let interpreted = run_ncc(&[
        "run".into(),
        raw.to_string_lossy().to_string(),
        "--show-return".into(),
    ]);
    assert!(interpreted.status.success(), "{}", rendered(&interpreted));
    assert!(rendered(&interpreted).contains("[return: true"));

    std::fs::remove_dir_all(out_dir).expect("temporary build tree clean");
}
