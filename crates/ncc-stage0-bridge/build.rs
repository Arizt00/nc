use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let source = manifest.join("../../bootstrap/nc/stage0_policy.n");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let target = env::var("TARGET").expect("TARGET");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-env-changed=NC_STAGE0_POLICY_COMPILER");
    println!("cargo:rerun-if-env-changed=NC_STAGE0_POLICY_ARCHIVER");
    let compiler = env::var_os("NC_STAGE0_POLICY_COMPILER")
        .expect("set NC_STAGE0_POLICY_COMPILER to a verified native nC compiler; no Rust policy fallback exists");
    let windows = target.contains("windows-msvc");
    let object = out.join(if windows { "policy.obj" } else { "policy.o" });
    let status = Command::new(&compiler)
        .args(["compiler", "emit-object"])
        .arg(&source)
        .args(["--target", &target, "--out"])
        .arg(&object)
        .arg("--quiet")
        .status()
        .expect("execute the native nC compiler for Stage0 integration");
    assert!(
        status.success(),
        "nC Stage0 policy compilation failed: {status}"
    );
    assert!(
        fs::metadata(&object).is_ok_and(|f| f.len() > 0),
        "missing native nC policy object"
    );
    let archive = out.join(if windows {
        "nc_stage0_policy.lib"
    } else {
        "libnc_stage0_policy.a"
    });
    let archiver = env::var_os("NC_STAGE0_POLICY_ARCHIVER").unwrap_or_else(|| {
        if windows {
            "llvm-lib.exe".into()
        } else {
            "ar".into()
        }
    });
    let mut command = Command::new(archiver);
    if windows {
        command
            .arg(format!("/OUT:{}", archive.display()))
            .arg(&object);
    } else {
        command.arg("crs").arg(&archive).arg(&object);
    }
    let status = command.status().expect("archive native nC policy object");
    assert!(status.success(), "native policy archiving failed: {status}");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=nc_stage0_policy");
}
