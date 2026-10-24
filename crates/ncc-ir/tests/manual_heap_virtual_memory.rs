use ncc_ir::{
    lower_with_backend_profile, mct_emit_llvm_module, BackendLoweringProfileIR, MctLlvmModuleIR,
};

fn source() -> &'static str {
    r#"
manualHeapVirtualMemory {
    fn:
        run() -> bool {
            page >> int = vmPageSize();
            block >> p<byte> = alignedAlloc(128, 64);
            resized >> p<byte> = alignedRealloc(block, 128, 256, 64);
            alignedFree(resized);
            reservation >> p<byte> = vmReserve(null, page * 3, 0);
            vmCommit(reservation, page, 3);
            vmProtect(reservation, page, 1);
            vmProtect(reservation, page, 3);
            vmExecutable(reservation, page);
            return vmRelease(reservation, page * 3);
        }

    ref main ->> @fn(run);
}
"#
}

fn module(profile: BackendLoweringProfileIR) -> MctLlvmModuleIR {
    let tokens = ncc_lexer::lex(source()).expect("manual memory source must lex");
    let program = ncc_parser::parse(tokens).expect("manual memory source must parse");
    let ir = lower_with_backend_profile(program, profile);
    mct_emit_llvm_module(&ir)
}

fn clang_object(llvm: &str, target: &str, suffix: &str) -> Result<u64, String> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        return Ok(0);
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let stem = format!("ncc_manual_vm_{}_{}", std::process::id(), nonce);
    let llvm_path = std::env::temp_dir().join(format!("{stem}.ll"));
    let object_path = std::env::temp_dir().join(format!("{stem}.{suffix}"));
    std::fs::write(&llvm_path, llvm).map_err(|error| error.to_string())?;
    let output = std::process::Command::new("clang")
        .arg(format!("--target={target}"))
        .args(["-Wno-override-module", "-x", "ir", "-c"])
        .arg(&llvm_path)
        .arg("-o")
        .arg(&object_path)
        .output()
        .map_err(|error| error.to_string())?;
    let object_len = std::fs::metadata(&object_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let _ = std::fs::remove_file(&llvm_path);
    let _ = std::fs::remove_file(&object_path);
    if !output.status.success() {
        return Err(format!(
            "clang {target} rejected raw memory LLVM:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(object_len)
}

#[test]
fn raw_memory_llvm_has_no_registry_or_tagged_result() {
    let module = module(BackendLoweringProfileIR::nc_sysv64());
    let llvm = &module.llvm_ir;
    assert!(llvm.contains("call ptr @__ncc_raw_vm_reserve"), "{llvm}");
    assert!(llvm.contains("call i1 @__ncc_raw_vm_executable"), "{llvm}");
    assert!(
        llvm.contains("call ptr @__ncc_raw_aligned_realloc"),
        "{llvm}"
    );
    assert!(!llvm.contains("@__ncc_map_head"), "{llvm}");
    assert!(!llvm.contains("__ncc_tagged_sum"), "{llvm}");
    assert!(module
        .diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.is_error()));
}

#[test]
fn raw_memory_llvm_compiles_to_elf_coff_and_macho_objects() {
    let linux = module(BackendLoweringProfileIR::nc_sysv64());
    let windows = module(BackendLoweringProfileIR::nc_win64());
    let darwin = module(BackendLoweringProfileIR::nc_x86_64_darwin());

    let elf = clang_object(&linux.llvm_ir, "x86_64-unknown-linux-gnu", "o")
        .expect("ELF raw memory object");
    let coff = clang_object(&windows.llvm_ir, "x86_64-pc-windows-msvc", "obj")
        .expect("COFF raw memory object");
    let macho = clang_object(&darwin.llvm_ir, "x86_64-apple-darwin", "macho.o")
        .expect("Mach-O raw memory object");
    if elf != 0 {
        assert!(elf > 0 && coff > 0 && macho > 0);
    }
}
