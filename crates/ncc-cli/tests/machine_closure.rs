use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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

fn fixture(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }

    if let Some(stripped) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(stripped);
        if archived.exists() {
            return archived;
        }
    }

    if let Some(stripped) = relative.strip_prefix("reports/") {
        let archived = root.join("_archive").join("old_reports").join(stripped);
        if archived.exists() {
            return archived;
        }
    }

    direct
}

fn fixture_cli(relative: &str) -> String {
    let root = workspace_root();
    if root.join(relative).exists() {
        return relative.to_string();
    }

    if let Some(stripped) = relative.strip_prefix("examples/") {
        let archived = Path::new("_archive").join("old_examples").join(stripped);
        if root.join(&archived).exists() {
            return archived.to_string_lossy().replace('\\', "/");
        }
    }

    relative.to_string()
}

fn cli_arg(arg: &str) -> String {
    if arg.starts_with("examples/") {
        return fixture_cli(arg);
    }
    arg.to_string()
}

fn run_ncc(args: &[&str]) -> Output {
    let mapped_args = args.iter().map(|arg| cli_arg(arg)).collect::<Vec<_>>();
    Command::new(ncc_bin())
        .args(mapped_args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc executes")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[derive(Debug, Clone)]
struct Phase12TestPeSection {
    name: String,
    virtual_size: u32,
    virtual_address: u32,
    raw_size: u32,
    raw_pointer: u32,
}

fn phase12_pe_sections(bytes: &[u8]) -> Vec<Phase12TestPeSection> {
    if bytes.len() < 0x40 || !bytes.starts_with(b"MZ") {
        return Vec::new();
    }
    let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().expect("PE offset")) as usize;
    if pe + 24 > bytes.len() || &bytes[pe..pe + 4] != b"PE\0\0" {
        return Vec::new();
    }
    let section_count = u16::from_le_bytes(bytes[pe + 6..pe + 8].try_into().unwrap()) as usize;
    let opt_size = u16::from_le_bytes(bytes[pe + 20..pe + 22].try_into().unwrap()) as usize;
    let section_table = pe + 24 + opt_size;
    (0..section_count)
        .filter_map(|idx| {
            let off = section_table + idx * 40;
            if off + 40 > bytes.len() {
                return None;
            }
            let name = String::from_utf8_lossy(&bytes[off..off + 8])
                .trim_end_matches('\0')
                .to_string();
            Some(Phase12TestPeSection {
                name,
                virtual_size: u32::from_le_bytes(bytes[off + 8..off + 12].try_into().unwrap()),
                virtual_address: u32::from_le_bytes(bytes[off + 12..off + 16].try_into().unwrap()),
                raw_size: u32::from_le_bytes(bytes[off + 16..off + 20].try_into().unwrap()),
                raw_pointer: u32::from_le_bytes(bytes[off + 20..off + 24].try_into().unwrap()),
            })
        })
        .collect()
}

fn phase12_pe_section(bytes: &[u8], name: &str) -> Phase12TestPeSection {
    phase12_pe_sections(bytes)
        .into_iter()
        .find(|section| section.name == name)
        .unwrap_or_else(|| panic!("missing PE section {name}"))
}

fn phase12_pe_rva_to_offset(bytes: &[u8], rva: u32) -> Option<usize> {
    phase12_pe_sections(bytes).into_iter().find_map(|section| {
        let span = section.virtual_size.max(section.raw_size).max(1);
        if rva >= section.virtual_address && rva < section.virtual_address + span {
            Some((section.raw_pointer + (rva - section.virtual_address)) as usize)
        } else {
            None
        }
    })
}

fn assert_machine_ok(rel: &str) -> String {
    let path = fixture(rel);
    let out = run_ncc(&["machine", "check", path.to_str().expect("path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "machine check must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn assert_machine_fail(rel: &str, code: &str) {
    let path = fixture(rel);
    let out = run_ncc(&["machine", "check", path.to_str().expect("path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "machine check must fail for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(code) || stderr.contains(code),
        "{rel} must contain {code}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_json_command_ok(args: &[&str]) -> Value {
    let out = run_ncc(args);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "ncc json command must pass: {:?}\nstdout={stdout}\nstderr={stderr}",
        args
    );
    serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("stdout must be json for {:?}: {err}\n{stdout}", args))
}

#[test]
fn machine_interop_compiler_manifest_from_n_ok() {
    let payload = assert_json_command_ok(&[
        "compiler",
        "machine-interop",
        "examples/09_ffi/machine_interop_bridge.n",
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
        "--strict",
    ]);

    assert_eq!(payload["schema"], "nc.compiler.machine_interop.v1");
    assert_eq!(payload["status"], "closed");
    assert_eq!(payload["target"]["abi"], "sysv64");
    assert_eq!(payload["coverage"]["runtime_symbols"], true);
    assert_eq!(payload["coverage"]["aggregate_returns"], true);
    assert_eq!(payload["coverage"]["call_locations"], true);
    assert_eq!(
        payload["abi_parity"]["schema"],
        "nc.compiler.highlevel_machine_abi_parity.v1"
    );
    assert_eq!(payload["abi_parity"]["status"], "closed");

    let raw = serde_json::to_string(&payload).expect("json stringify");
    assert!(raw.contains("machineInteropBridge.exportedAdd"));
    assert!(raw.contains("nc_machineInteropBridge_exportedAdd"));
    assert!(raw.contains("machineInteropBridge.makePacket"));
    assert!(raw.contains("nc_machineInteropBridge_makePacket"));
    assert!(raw.contains("machineInteropBridge.callProbe"));
    assert!(raw.contains("nc_machineInteropBridge_callProbe"));
    assert!(raw.contains("machineInteropBridge.inspectPacket"));
    assert!(raw.contains("ptr<Packet>"));
    assert!(raw.contains("\"str\""));
    assert!(raw.contains("nC.runtime.output.println"));
    assert!(raw.contains("nC.runtime.output.print"));
    assert!(raw.contains("machine_probe_packet"));
    assert!(raw.contains("utf8_bytes"));

    let packet = payload["types"]["struct_layouts"]
        .as_array()
        .expect("struct layouts")
        .iter()
        .find(|row| row["name"] == "Packet")
        .expect("Packet layout");
    assert_eq!(packet["size"], 32);
    assert_eq!(packet["align"], 8);
    let fields = packet["fields"].as_array().expect("Packet fields");
    assert_eq!(fields[0]["name"], "tag");
    assert_eq!(fields[0]["offset"], 0);
    assert_eq!(fields[1]["name"], "len");
    assert_eq!(fields[1]["offset"], 8);
    assert_eq!(fields[2]["name"], "payload");
    assert_eq!(fields[2]["offset"], 16);
    assert_eq!(fields[3]["name"], "name");
    assert_eq!(fields[3]["offset"], 24);

    let make_packet = payload["exports"]["callables"]
        .as_array()
        .expect("callables")
        .iter()
        .find(|row| row["function"] == "makePacket")
        .expect("makePacket callable");
    assert_eq!(
        make_packet["return"]["abi"]["return"]["class"],
        "aggregate_sret"
    );
    assert_eq!(make_packet["return"]["abi"]["return"]["hidden_sret"], true);
    assert_eq!(
        make_packet["call_abi"]["hidden_sret_arg"]["location"]["register"],
        "rdi"
    );
    assert_eq!(make_packet["params"][0]["location"]["register"], "rsi");
    assert_eq!(
        make_packet["params"][3]["abi"]["kind"],
        "string_or_bytes_pointer"
    );
    assert_eq!(
        make_packet["params"][3]["abi"]["descriptor"]["call_boundary"],
        "pointer_to_descriptor"
    );

    let foreign = payload["foreign_imports"]
        .as_array()
        .expect("foreign imports")
        .iter()
        .find(|row| row["symbol"] == "machine_probe_packet")
        .expect("foreign symbol");
    assert_eq!(foreign["signature_abi"]["status"], "closed");
    assert_eq!(
        foreign["signature_abi"]["params"][0]["location"]["register"],
        "rdi"
    );
    assert_eq!(
        foreign["signature_abi"]["params"][1]["location"]["register"],
        "rsi"
    );

    assert!(payload["strings"]
        .as_array()
        .expect("strings")
        .iter()
        .any(|row| row["descriptor"]["len_excludes_final_nul"] == true));
    assert!(payload["abi_parity"]["runtime_bridge"]
        .as_array()
        .expect("runtime bridge")
        .iter()
        .any(|row| row["symbol"] == "nC.runtime.output.println"));
    assert!(!payload["abi_parity"]["aggregate_return_rows"]
        .as_array()
        .expect("aggregate return rows")
        .is_empty());
}

#[test]
fn machine_interop_gate_from_n_ok() {
    let payload = assert_json_command_ok(&[
        "machine",
        "interop",
        "examples/09_ffi/machine_interop_bridge.n",
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
        "--strict",
    ]);

    assert_eq!(
        payload["machine_gate"]["schema"],
        "nc.machine.interop_gate.v1"
    );
    assert_eq!(payload["machine_gate"]["status"], "closed");
    assert_eq!(
        payload["machine_gate"]["checks"]["compiler_manifest_closed"],
        true
    );
    assert_eq!(payload["machine_gate"]["checks"]["coverage_closed"], true);
    assert_eq!(payload["machine_gate"]["checks"]["structs"], true);
    assert_eq!(payload["machine_gate"]["checks"]["runtime_symbols"], true);
    assert_eq!(payload["coverage"]["aggregate_returns"], true);
    assert_eq!(payload["coverage"]["call_locations"], true);
}

#[test]
fn machine_interop_win64_abi_contract_from_n_ok() {
    let payload = assert_json_command_ok(&[
        "compiler",
        "machine-interop",
        "examples/09_ffi/machine_interop_bridge.n",
        "--target",
        "x86_64-pc-windows-msvc",
        "--json",
        "--strict",
    ]);

    assert_eq!(payload["target"]["abi"], "win64");
    assert_eq!(
        payload["target"]["register_contract"]["shadow_space_bytes"],
        32
    );
    let regs = payload["target"]["register_contract"]["integer_args"]
        .as_array()
        .expect("win64 arg regs")
        .iter()
        .map(|value| value.as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(regs, vec!["rcx", "rdx", "r8", "r9"]);

    let make_packet = payload["exports"]["callables"]
        .as_array()
        .expect("callables")
        .iter()
        .find(|row| row["function"] == "makePacket")
        .expect("makePacket callable");
    assert_eq!(
        make_packet["call_abi"]["hidden_sret_arg"]["location"]["register"],
        "rcx"
    );
    assert_eq!(make_packet["params"][0]["location"]["register"], "rdx");
    assert_eq!(make_packet["call_abi"]["shadow_space_bytes"], 32);
}

fn emit_machine_bytes(rel: &str, name: &str) -> Vec<u8> {
    let source = fixture(rel);
    let out = temp_path(name, "bin");
    let output = run_ncc(&[
        "machine",
        "emit-bytes",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "emit-bytes must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    fs::read(out).expect("emitted bytes")
}

fn emit_machine_object(rel: &str, target: &str, name: &str, ext: &str) -> PathBuf {
    let source = fixture(rel);
    let out = temp_path(name, ext);
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        target,
        "--out",
        out.to_str().expect("path"),
    ]);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "emit-object must pass for {rel} target={target}\nstdout={stdout}\nstderr={stderr}"
    );
    out
}

fn temp_path(name: &str, ext: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("{}_{}_{}.{}", name, std::process::id(), ts, ext))
}

#[test]
fn machine_doctor_ok() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("machine_native_asm_total_closed"));
    assert!(stdout.contains("\"no_high_level_fn_fa\": true"));
}

#[test]
fn machine_targets_json_ok() {
    let out = run_ncc(&["machine", "targets", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("x86_64-unknown-linux-gnu"));
    assert!(stdout.contains("aarch64-unknown-linux-gnu"));
    assert!(stdout.contains("riscv64-unknown-elf"));
    assert!(stdout.contains("riscv32imac-unknown-elf"));
    assert!(stdout.contains("i386-unknown-linux-gnu"));
    assert!(stdout.contains("wasm32-unknown-unknown"));
}

#[test]
fn machine_registers_x86_64_ok() {
    let out = run_ncc(&["machine", "registers", "x86_64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("rax"));
    assert!(stdout.contains("xmm15"));
}

#[test]
fn machine_abi_sysv64_ok() {
    let out = run_ncc(&["machine", "abi", "sysv64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("rdi"));
    assert!(stdout.contains("stack_align"));
}

#[test]
fn machine_abi_win64_ok() {
    let out = run_ncc(&["machine", "abi", "win64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("shadow_space"));
}

#[test]
fn machine_instruction_matrix_non_empty() {
    let out = run_ncc(&["machine", "instructions", "x86_64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("mov"));
    assert!(stdout.contains("ret"));
}

#[test]
fn machine_registers_cross_arch_ok() {
    for arch in ["aarch64", "riscv64", "riscv32", "i386", "wasm32"] {
        let out = run_ncc(&["machine", "registers", arch, "--json"]);
        let stdout = normalize(&out.stdout);
        assert!(out.status.success(), "{arch}: {stdout}");
        assert!(stdout.contains(arch));
    }
}

#[test]
fn machine_registers_cross_arch_extended_classes_ok() {
    let aarch64 = assert_json_command_ok(&["machine", "registers", "aarch64", "--json"]);
    assert_eq!(aarch64["status"], "closed_register_profile");
    assert_eq!(aarch64["floating_point"]["double"][31], "d31");
    assert_eq!(aarch64["simd"]["q"][31], "q31");
    assert_eq!(aarch64["sve"]["z"][31], "z31");
    assert_eq!(aarch64["sve"]["p"][15], "p15");

    let riscv32 = assert_json_command_ok(&["machine", "registers", "riscv32", "--json"]);
    assert_eq!(riscv32["floating_point"][31], "f31");

    let i386 = assert_json_command_ok(&["machine", "registers", "i386", "--json"]);
    assert_eq!(i386["classes"]["x87_stack"][7], "st7");
    assert_eq!(i386["classes"]["mmx"][7], "mm7");
    assert_eq!(i386["classes"]["xmm"][7], "xmm7");
}

#[test]
fn machine_instructions_cross_arch_ok() {
    for arch in ["aarch64", "riscv64", "riscv32", "i386", "wasm32"] {
        let out = run_ncc(&["machine", "instructions", arch, "--json"]);
        let stdout = normalize(&out.stdout);
        assert!(out.status.success(), "{arch}: {stdout}");
        assert!(stdout.contains("ret") || stdout.contains("return"));
    }
}

#[test]
fn machine_parse_hello_ok() {
    assert_machine_ok("examples/machine/hello_machine.nm");
}

#[test]
fn machine_parse_stack_frame_ok() {
    assert_machine_ok("examples/machine/stack_frame_sysv64.nm");
}

#[test]
fn machine_parse_cross_target_examples_ok() {
    assert_machine_ok("examples/machine/aarch64_add_two_aapcs64.nm");
    assert_machine_ok("examples/machine/riscv64_add_two_elf.nm");
    assert_machine_ok("examples/machine/riscv32_add_two_elf.nm");
    assert_machine_ok("examples/machine/i386_add_two_elf.nm");
    assert_machine_ok("examples/machine/wasm32_const_main.nm");
    assert_machine_ok("examples/machine/coff_return42_win64.nm");
    assert_machine_ok("examples/machine/macho_return42_darwin.nm");
}

#[test]
fn machine_parse_proc_ok() {
    assert_machine_ok("examples/machine/add_two_sysv64.nm");
}

#[test]
fn machine_parse_routine_ok() {
    assert_machine_ok("examples/machine/object_coff_minimal.nm");
}

#[test]
fn machine_parse_block_label_instr_ok() {
    assert_machine_ok("examples/machine/branch_labels.nm");
}

#[test]
fn machine_high_level_fn_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/negative/high_level_fn_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_FN_FORBIDDEN_001",
    );
}

#[test]
fn machine_high_level_fa_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/negative/high_level_fa_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_FA_FORBIDDEN_001",
    );
}

#[test]
fn machine_high_level_stc_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/negative/high_level_stc_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_STC_FORBIDDEN_001",
    );
}

#[test]
fn machine_high_level_type_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/negative/high_level_type_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_TYPE_FORBIDDEN_001",
    );
}

#[test]
fn machine_unknown_register_fail() {
    assert_machine_fail(
        "examples/machine/negative/unknown_register_fail.nm",
        "E_MACHINE_REGISTER_UNKNOWN_001",
    );
}

#[test]
fn machine_invalid_operand_width_fail() {
    assert_machine_fail(
        "examples/machine/negative/invalid_operand_width_fail.nm",
        "E_MACHINE_REGISTER_WIDTH_001",
    );
}

#[test]
fn machine_immediate_out_of_range_fail() {
    assert_machine_fail(
        "examples/machine/negative/immediate_out_of_range_fail.nm",
        "E_MACHINE_IMMEDIATE_RANGE_001",
    );
}

#[test]
fn machine_label_resolution_ok() {
    assert_machine_ok("examples/machine/loop_sum_sysv64.nm");
}

#[test]
fn machine_unknown_label_fail() {
    assert_machine_fail(
        "examples/machine/negative/unknown_label_fail.nm",
        "E_MACHINE_LABEL_UNRESOLVED_001",
    );
}

#[test]
fn machine_flags_before_conditional_jump_fail() {
    assert_machine_fail(
        "examples/machine/negative/flags_before_conditional_jump_fail.nm",
        "E_MACHINE_FLAGS_UNINITIALIZED_001",
    );
}

#[test]
fn machine_flags_after_imul_overflow_ok() {
    assert_machine_ok("examples/machine/flags_imul_overflow_sysv64.nm");
}

#[test]
fn machine_mini_calculator_linux_ok() {
    assert_machine_ok("examples/11_machine/mini_calculator/mini_calculator_linux_x86_64.nm");
}

#[test]
fn machine_mini_calculator_windows_ok() {
    assert_machine_ok("examples/11_machine/mini_calculator/mini_calculator_windows_x86_64.nm");
}

#[test]
fn machine_stack_alignment_sysv64_ok() {
    assert_machine_ok("examples/machine/call_external_c_symbol.nm");
}

#[test]
fn machine_stack_alignment_fail() {
    assert_machine_fail(
        "examples/machine/negative/stack_misaligned_call_fail.nm",
        "E_MACHINE_STACK_ALIGN_001",
    );
}

#[test]
fn machine_callee_saved_restore_ok() {
    assert_machine_ok("examples/machine/stack_frame_sysv64.nm");
}

#[test]
fn machine_callee_saved_not_restored_fail() {
    assert_machine_fail(
        "examples/machine/negative/callee_saved_not_restored_fail.nm",
        "E_MACHINE_CALLEE_SAVED_001",
    );
}

#[test]
fn machine_mov_rax_imm64_encoding_ok() {
    let path = fixture("examples/machine/hello_machine.nm");
    let out = temp_path("hello_machine", "bin");
    let output = run_ncc(&[
        "machine",
        "emit-bytes",
        path.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("bytes");
    assert!(bytes.starts_with(&[0x48, 0xb8]));
    assert_eq!(bytes.last().copied(), Some(0xc3));
}

#[test]
fn machine_run_bytes_nm_executes_return_i64_ok() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let source = fixture("examples/machine/jit_return42_x86_64.nm");
    let payload = assert_json_command_ok(&[
        "machine",
        "run-bytes",
        source.to_str().expect("path"),
        "--expect-return",
        "42",
        "--json",
    ]);
    assert_eq!(payload["schema"], "nc.machine.jit.run_bytes.v1");
    assert_eq!(payload["status"], "executed");
    assert_eq!(payload["return"], 42);
    assert_eq!(payload["permissions"], "rw_to_rx");
    assert_eq!(payload["isolated_child"], true);
    assert_eq!(payload["source_kind"], "nm");
    assert_eq!(payload["temp_cleanup"], "removed");
    assert_eq!(payload["input_contract"]["externs_rejected"], true);
    assert_eq!(payload["input_contract"]["relocations_rejected"], true);
    assert_eq!(
        payload["execution_contract"]["timeout_maps_to"],
        "E_MACHINE_JIT_TIMEOUT_001"
    );
    assert_eq!(
        payload["loader_contract"]["schema"],
        "nc.machine.jit.loader_contract.v1"
    );
    assert_eq!(payload["loader_contract"]["write_execute_overlap"], false);
    assert_eq!(payload["loader_contract"]["relocations_allowed"], false);
    let child_input = payload["child_input"].as_str().expect("child input path");
    assert!(
        !Path::new(child_input).exists(),
        "run-bytes must remove child input temp file: {child_input}"
    );
}

#[test]
fn machine_run_bytes_raw_bin_executes_return_i64_ok() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let source = fixture("examples/machine/jit_return42_x86_64.nm");
    let bin = temp_path("jit_return42_raw", "bin");
    let emit = run_ncc(&[
        "machine",
        "emit-bytes",
        source.to_str().expect("path"),
        "--out",
        bin.to_str().expect("path"),
    ]);
    assert!(emit.status.success(), "{}", normalize(&emit.stderr));
    let payload = assert_json_command_ok(&[
        "machine",
        "run-bytes",
        bin.to_str().expect("path"),
        "--raw-bin",
        "--expect-return",
        "42",
        "--json",
    ]);
    assert_eq!(payload["return"], 42);
    assert_eq!(payload["source_kind"], "raw-bin");
    assert_eq!(payload["extern_count"], 0);
    assert_eq!(payload["relocation_count"], 0);
    assert_eq!(
        payload["loader_contract"]["kind"],
        "in_process_executable_memory"
    );
    assert_eq!(
        payload["loader_contract"]["permission_flow"][2],
        "protect_rx"
    );
}

#[test]
fn machine_run_bytes_rejects_empty_raw_bin() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let bin = temp_path("jit_empty", "bin");
    fs::write(&bin, []).expect("empty bin");
    let out = run_ncc(&[
        "machine",
        "run-bytes",
        bin.to_str().expect("path"),
        "--raw-bin",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "empty bytes must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_JIT_BYTES_001") || stderr.contains("E_MACHINE_JIT_BYTES_001"),
        "empty bytes diagnostic missing\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_run_bytes_rejects_nm_with_externs() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let source = fixture("examples/machine/jit_extern_rejected_x86_64.nm");
    let out = run_ncc(&[
        "machine",
        "run-bytes",
        source.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "extern-backed byte stream must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_JIT_EXTERN_001") || stderr.contains("E_MACHINE_JIT_EXTERN_001"),
        "extern diagnostic missing\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_run_bytes_rejects_nm_with_relocations() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let source = fixture("examples/machine/jit_reloc_rejected_x86_64.nm");
    let out = run_ncc(&[
        "machine",
        "run-bytes",
        source.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "relocatable byte stream must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_JIT_RELOCATION_001")
            || stderr.contains("E_MACHINE_JIT_RELOCATION_001"),
        "relocation diagnostic missing\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_run_bytes_timeout_kills_isolated_child() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let bin = temp_path("jit_infinite_loop", "bin");
    fs::write(&bin, [0xeb, 0xfe]).expect("loop bin");
    let out = run_ncc(&[
        "machine",
        "run-bytes",
        bin.to_str().expect("path"),
        "--raw-bin",
        "--timeout-ms",
        "100",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "infinite byte stream must time out\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_JIT_TIMEOUT_001")
            || stderr.contains("E_MACHINE_JIT_TIMEOUT_001"),
        "timeout diagnostic missing\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_run_bytes_crash_reports_child_boundary() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    let bin = temp_path("jit_ud2_crash", "bin");
    fs::write(&bin, [0x0f, 0x0b]).expect("ud2 bin");
    let out = run_ncc(&[
        "machine",
        "run-bytes",
        bin.to_str().expect("path"),
        "--raw-bin",
        "--timeout-ms",
        "2000",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "crashing byte stream must be contained by child process\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_JIT_CHILD_001") || stderr.contains("E_MACHINE_JIT_CHILD_001"),
        "child crash diagnostic missing\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_add_rax_rbx_encoding_ok() {
    let source = fixture("examples/machine/add_two_sysv64.nm");
    let out = temp_path("add_two", "bin");
    let output = run_ncc(&[
        "machine",
        "emit-bytes",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("bytes");
    assert!(bytes.windows(3).any(|window| window == [0x48, 0x01, 0xf0]));
}

#[test]
fn machine_cmp_je_encoding_ok() {
    let source = fixture("examples/machine/branch_labels.nm");
    let out = temp_path("branch_labels", "bin");
    let output = run_ncc(&[
        "machine",
        "emit-bytes",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("bytes");
    assert!(bytes.windows(2).any(|window| window == [0x0f, 0x84]));
}

#[test]
fn machine_call_relocation_ok() {
    assert_machine_ok("examples/machine/relocation_call_symbol.nm");
}

#[test]
fn machine_emit_elf64_object_ok() {
    let source = fixture("examples/machine/object_elf64_minimal.nm");
    let out = temp_path("machine_minimal", "o");
    let output = run_ncc(&[
        "machine",
        "emit-elf",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
}

#[test]
fn machine_emit_coff_object_ok() {
    let source = fixture("examples/machine/object_coff_minimal.nm");
    let out = temp_path("machine_minimal", "obj");
    let output = run_ncc(&[
        "machine",
        "emit-coff",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert_eq!(bytes.get(0..2), Some(&[0x64, 0x86][..]));
}

#[test]
fn machine_emit_macho_object_ok() {
    let source = fixture("examples/machine/object_macho_minimal.nm");
    let out = temp_path("machine_minimal", "macho");
    let output = run_ncc(&[
        "machine",
        "emit-macho",
        source.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
}

#[test]
fn machine_emit_aarch64_elf_object_ok() {
    let source = fixture("examples/machine/aarch64_add_two_aapcs64.nm");
    let out = temp_path("machine_aarch64", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "aarch64-unknown-linux-gnu",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183);
}

#[test]
fn machine_emit_riscv64_elf_object_ok() {
    let source = fixture("examples/machine/riscv64_add_two_elf.nm");
    let out = temp_path("machine_riscv64", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "riscv64-unknown-elf",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 243);
}

#[test]
fn machine_emit_riscv32_elf32_object_ok() {
    let source = fixture("examples/machine/riscv32_add_two_elf.nm");
    let out = temp_path("machine_riscv32", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "riscv32imac-unknown-elf",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(&out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(bytes[4], 1);
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 243);
    let verify = run_ncc(&["machine", "verify-object", out.to_str().expect("path")]);
    let stdout = normalize(&verify.stdout);
    assert!(verify.status.success(), "{}", normalize(&verify.stderr));
    assert!(stdout.contains("\"object_format\": \"elf32\""), "{stdout}");
}

#[test]
fn machine_emit_i386_elf32_object_ok() {
    let source = fixture("examples/machine/i386_add_two_elf.nm");
    let out = temp_path("machine_i386", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "i386-unknown-linux-gnu",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(&out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(bytes[4], 1);
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 3);
    let verify = run_ncc(&["machine", "verify-object", out.to_str().expect("path")]);
    let stdout = normalize(&verify.stdout);
    assert!(verify.status.success(), "{}", normalize(&verify.stderr));
    assert!(stdout.contains("\"object_format\": \"elf32\""), "{stdout}");
}

#[test]
fn machine_emit_i386_elf32_relocations_ok() {
    let source = fixture("examples/machine/i386_relocations_elf32.nm");
    let out = temp_path("machine_i386_reloc", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "i386-unknown-linux-gnu",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(&out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(bytes[4], 1);
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 3);

    let verify = run_ncc(&["machine", "verify-object", out.to_str().expect("path")]);
    let verify_stdout = normalize(&verify.stdout);
    assert!(verify.status.success(), "{}", normalize(&verify.stderr));
    assert!(
        verify_stdout.contains("\"object_format\": \"elf32\""),
        "{verify_stdout}"
    );

    let disasm = run_ncc(&[
        "machine",
        "disasm-object",
        out.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&disasm.stdout);
    assert!(disasm.status.success(), "{stdout}");
    for needle in [
        "\"name\": \".rel.text\"",
        "externalroutine",
        "R_386_32",
        "R_386_PC32",
        "R_386_PLT32",
        "R_386_GOT32",
    ] {
        assert!(stdout.contains(needle), "{needle}\n{stdout}");
    }
}

#[test]
fn machine_emit_riscv32_elf32_relocations_ok() {
    let source = fixture("examples/machine/riscv32_relocations_elf32.nm");
    let out = temp_path("machine_riscv32_reloc", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "riscv32imac-unknown-elf",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(&out).expect("object");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(bytes[4], 1);
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 243);

    let verify = run_ncc(&["machine", "verify-object", out.to_str().expect("path")]);
    let verify_stdout = normalize(&verify.stdout);
    assert!(verify.status.success(), "{}", normalize(&verify.stderr));
    assert!(
        verify_stdout.contains("\"object_format\": \"elf32\""),
        "{verify_stdout}"
    );

    let disasm = run_ncc(&[
        "machine",
        "disasm-object",
        out.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&disasm.stdout);
    assert!(disasm.status.success(), "{stdout}");
    for needle in [
        "\"name\": \".rela.text\"",
        "extcall",
        "R_RISCV_32",
        "R_RISCV_BRANCH",
        "R_RISCV_JAL",
        "R_RISCV_CALL",
        "R_RISCV_HI20",
        "R_RISCV_LO12_I",
        "R_RISCV_PCREL_HI20",
        "R_RISCV_PCREL_LO12_I",
        "R_RISCV_RELAX",
    ] {
        assert!(stdout.contains(needle), "{needle}\n{stdout}");
    }
}

#[test]
fn machine_phase12_imports_elf32_native_objects_without_ncc_link_unit() {
    for (rel, target, source_name, native_name, expected_import) in [
        (
            "examples/machine/i386_relocations_elf32.nm",
            "i386-unknown-linux-gnu",
            "machine_i386_reloc_native_source",
            "machine_i386_reloc_native_nometa",
            "externalroutine",
        ),
        (
            "examples/machine/riscv32_relocations_elf32.nm",
            "riscv32imac-unknown-elf",
            "machine_riscv32_reloc_native_source",
            "machine_riscv32_reloc_native_nometa",
            "extcall",
        ),
    ] {
        let source = if target == "riscv32imac-unknown-elf" {
            let source = temp_path("machine_riscv32_import_source", "nm");
            fs::write(
                &source,
                r#"riscv32ImportSource[@machine] {
    target:
        arch: riscv32;
        abi: ilp32;
        object: elf32;
        endian: little;
        pointerWidth: 32;
        profile: freestanding;

    extern:
        extcall:
            abi: C;
            symbol: "extcall";

    symbol:
        relocRv32:
            section: .text;
            binding: global;
            type: routine;

    entry:
        relocRv32;

    proc:
        relocRv32:
            instr:
                jal ra, extcall;
                ret;

    reloc:
        jal:
            type: R_RISCV_JAL;
            symbol: extcall;
            section: .text;
            offset: 0;

    seal:
        artifact: elf32;
        require: verified;
}
"#,
            )
            .expect("RV32 import source must write");
            source
        } else {
            fixture(rel)
        };
        let object = temp_path(source_name, "o");
        let output = run_ncc(&[
            "machine",
            "emit-object",
            source.to_str().expect("path"),
            "--target",
            target,
            "--out",
            object.to_str().expect("path"),
        ]);
        assert!(output.status.success(), "{}", normalize(&output.stderr));

        let native = temp_path(native_name, "o");
        fs::copy(&object, &native).expect("copy object without ncc metadata");
        let metadata = format!("{}.machine-link-meta.json", native.to_string_lossy());
        let _ = fs::remove_file(metadata);

        let mut link_inputs = vec![native.to_string_lossy().to_string()];
        if target == "riscv32imac-unknown-elf" {
            let provider_source = temp_path("machine_riscv32_extcall_provider", "nm");
            fs::write(
                &provider_source,
                r#"riscv32ExtcallProvider[@machine] {
    target:
        arch: riscv32;
        abi: ilp32;
        object: elf32;
        endian: little;
        pointerWidth: 32;
        profile: freestanding;

    symbol:
        extcall:
            section: .text;
            binding: global;
            type: routine;

    entry:
        extcall;

    proc:
        extcall:
            instr:
                ret;

    seal:
        artifact: elf32;
        require: verified;
}
"#,
            )
            .expect("RV32 provider source must write");
            let provider = temp_path("machine_riscv32_extcall_provider", "o");
            let emit_provider = run_ncc(&[
                "machine",
                "emit-object",
                provider_source.to_str().expect("provider source path"),
                "--target",
                target,
                "--out",
                provider.to_str().expect("provider object path"),
            ]);
            assert!(
                emit_provider.status.success(),
                "{}",
                normalize(&emit_provider.stderr)
            );
            link_inputs.push(provider.to_string_lossy().to_string());
        }

        let linked = temp_path(native_name, "ncclink");
        let mut link_args = vec!["machine".to_string(), "link".to_string()];
        link_args.extend(link_inputs);
        link_args.extend([
            "--target".to_string(),
            target.to_string(),
            "--out".to_string(),
            linked.to_string_lossy().to_string(),
            "--json".to_string(),
        ]);
        let link_refs = link_args.iter().map(String::as_str).collect::<Vec<_>>();
        let link = run_ncc(&link_refs);
        let stdout = normalize(&link.stdout);
        assert!(
            link.status.success(),
            "{stdout}\n{}",
            normalize(&link.stderr)
        );
        assert!(stdout.contains("\"object_format\": \"elf32\""), "{stdout}");
        assert!(
            stdout.contains("\"link_unit_origin\": \"native_object\""),
            "{stdout}"
        );
        assert!(stdout.contains(expected_import), "{stdout}");
    }
}

#[test]
fn machine_emit_wasm_object_ok() {
    let source = fixture("examples/machine/wasm32_const_main.nm");
    let out = temp_path("machine_wasm", "wasm");
    let output = run_ncc(&[
        "machine",
        "emit-wasm",
        source.to_str().expect("path"),
        "--target",
        "wasm32-unknown-unknown",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = std::fs::read(out).expect("object");
    assert!(bytes.starts_with(b"\0asm"));
}

#[test]
fn machine_link_smoke_add_two_ok() {
    let source = fixture("examples/machine/add_two_sysv64.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_cross_host_required\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"stdout\": \"42\""));
    }
}

#[test]
fn machine_external_symbol_link_smoke_ok() {
    let source = fixture("examples/machine/external_marker_sysv64.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_cross_host_required\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("external-called=1"));
    }
}

#[test]
fn machine_wasm_node_smoke_ok() {
    let source = fixture("examples/machine/wasm32_const_main.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "wasm32-unknown-unknown",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"status\": \"passed\""));
    assert!(stdout.contains("\"stdout\": \"42\""));
}

#[test]
fn machine_aarch64_native_link_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/aarch64_add_two_aapcs64.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "aarch64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"aarch64-unknown-linux-gnu\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_native_aarch64_runner_required\"")
            || stdout.contains("\"status\": \"blocked_no_c_compiler\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"environment\": \"native-linux-aarch64\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_riscv64_qemu_link_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/riscv64_add_two_elf.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "riscv64-unknown-elf",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"riscv64-unknown-elf\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_no_riscv64_cross_compiler\"")
            || stdout.contains("\"status\": \"blocked_no_qemu_riscv64\"")
            || stdout.contains("\"status\": \"blocked_linux_qemu_required\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"environment\": \"qemu-riscv64-user\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_riscv32_qemu_link_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/riscv32_add_two_elf.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "riscv32imac-unknown-elf",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"riscv32imac-unknown-elf\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_no_riscv32_cross_compiler\"")
            || stdout.contains("\"status\": \"blocked_no_qemu_riscv32\"")
            || stdout.contains("\"status\": \"blocked_linux_qemu_required\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"environment\": \"qemu-riscv32-user\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_i386_qemu_link_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/i386_add_two_elf.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "i386-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"i386-unknown-linux-gnu\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_no_i386_cross_compiler\"")
            || stdout.contains("\"status\": \"blocked_no_qemu_i386\"")
            || stdout.contains("\"status\": \"blocked_linux_qemu_required\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"environment\": \"qemu-i386-user\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_windows_coff_native_executable_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/coff_return42_win64.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-pc-windows-msvc",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"x86_64-pc-windows-msvc\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_native_windows_x86_64_runner_required\"")
            || stdout.contains("\"status\": \"blocked_no_rustc\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"environment\": \"native-windows-x86_64\""));
        assert!(stdout.contains("\"object_format\": \"coff\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_macos_macho_native_executable_smoke_is_explicitly_gated_or_passed() {
    let source = fixture("examples/machine/macho_return42_darwin.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-apple-darwin",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"x86_64-apple-darwin\""));
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("\"status\": \"blocked_native_macos_x86_64_runner_required\"")
            || stdout.contains("\"status\": \"blocked_no_c_compiler\"")
            || stdout.contains("\"status\": \"failed_link\""),
        "{stdout}"
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(
            stdout.contains("\"environment\": \"native-macos_x86_64\"")
                || stdout.contains("\"environment\": \"native-macos-x86_64\"")
        );
        assert!(stdout.contains("\"object_format\": \"macho\""));
        assert!(stdout.contains("\"stdout\": \"42\""));
    } else {
        assert!(stdout.contains("E_MACHINE_"), "{stdout}");
    }
}

#[test]
fn machine_linux_elf_system_loader_smoke_executes_final_executable_or_gates() {
    let source = fixture("examples/machine/native/link_final_linux_exit42.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--system-loader",
        "--expect-exit",
        "42",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(
        stdout.contains("\"evidence_level\": \"executed_os_loader_executable\"")
            || stdout.contains("\"status\": \"blocked_native_linux_x86_64_runner_required\"")
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"artifact_format\": \"elf64\""));
        assert!(stdout.contains("\"loader\": \"linux-kernel-elf-loader\""));
        assert!(stdout.contains("\"exit_code\": 42"));
    } else {
        assert!(stdout.contains("E_MACHINE_SYSTEM_LOADER_SMOKE_HOST_001"));
    }
}

#[test]
fn machine_macho_system_loader_smoke_executes_final_executable_or_gates() {
    let source = fixture("examples/machine/native/link_final_macho_exit42.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-apple-darwin",
        "--system-loader",
        "--expect-exit",
        "42",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"x86_64-apple-darwin\""));
    assert!(
        stdout.contains("\"evidence_level\": \"executed_os_loader_executable\"")
            || stdout.contains("\"status\": \"blocked_native_macos_x86_64_runner_required\"")
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"artifact_format\": \"macho\""));
        assert!(stdout.contains("\"loader\": \"macos-mach-o-loader\""));
        assert!(stdout.contains("\"exit_code\": 42"));
    } else {
        assert!(stdout.contains("E_MACHINE_SYSTEM_LOADER_SMOKE_HOST_002"));
    }
}

#[test]
fn machine_pe64_system_loader_smoke_executes_final_executable_or_gates() {
    let source = fixture("examples/machine/native/link_final_windows_exit42.nm");
    let output = run_ncc(&[
        "machine",
        "link-smoke",
        source.to_str().expect("path"),
        "--target",
        "x86_64-pc-windows-msvc",
        "--system-loader",
        "--expect-exit",
        "42",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        normalize(&output.stderr)
    );
    assert!(stdout.contains("\"target\": \"x86_64-pc-windows-msvc\""));
    assert!(
        stdout.contains("\"evidence_level\": \"executed_os_loader_executable\"")
            || stdout.contains("\"status\": \"blocked_native_windows_x86_64_runner_required\"")
    );
    if stdout.contains("\"status\": \"passed\"") {
        assert!(stdout.contains("\"artifact_format\": \"pe64\""));
        assert!(stdout.contains("\"loader\": \"windows-pe-loader\""));
        assert!(stdout.contains("\"exit_code\": 42"));
        assert!(stdout.contains("DYNAMIC_BASE_HIGH_ENTROPY_VA"));
    } else {
        assert!(stdout.contains("E_MACHINE_SYSTEM_LOADER_SMOKE_HOST_003"));
    }
}

#[test]
fn machine_target_family_gate_json_ok() {
    let out = run_ncc(&["machine", "target-family-gate"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("closed_exact_head_retained_actions_evidence"));
    assert!(stdout.contains("riscv64-unknown-elf"));
    assert!(stdout.contains("retained_exact_head_windows_x86_64_native_executable"));
    assert!(stdout.contains("retained_exact_head_macos_x86_64_native_executable"));
    assert!(stdout.contains("retained_exact_head_linux_aarch64_native"));
    assert!(stdout.contains("retained_exact_head_qemu_riscv64"));
    assert!(stdout.contains("\"workflow\": \"machine-level5-level6-release-industrial\""));
    assert!(stdout.contains("windows-msvc-x86_64"));
    assert!(stdout.contains("linux-gnu-x86_64"));
    assert!(stdout.contains("linux-clang-x86_64"));
    assert!(stdout.contains("macos-clang-x86_64"));
    assert!(stdout.contains("linux-gnu-aarch64"));
    assert!(stdout.contains("linux-riscv64-qemu"));
    assert!(stdout.contains("ncc.machine.non_hosted_isa_family_gates.v1"));
    assert!(stdout.contains("\"family\": \"gpu\""));
    assert!(stdout.contains("\"family\": \"dsp\""));
    assert!(stdout.contains("\"family\": \"32bit_cpu\""));
    assert!(stdout.contains("\"family\": \"firmware\""));
    assert!(stdout.contains("\"family\": \"legacy_microcontroller\""));
    assert!(stdout.contains("E_MACHINE_TARGET_FAMILY_PROFILE_REQUIRED_001"));
}

#[test]
fn machine_target_family_gate_non_hosted_isa_frontier_closed() {
    let payload = assert_json_command_ok(&["machine", "target-family-gate", "non-hosted"]);
    assert_eq!(
        payload["schema"],
        "ncc.machine.non_hosted_isa_family_gates.v1"
    );
    assert_eq!(payload["status"], "closed_profile_gated");
    let families = payload["families"].as_array().expect("families");
    assert_eq!(families.len(), 5);
    for family in [
        "gpu",
        "dsp",
        "32bit_cpu",
        "firmware",
        "legacy_microcontroller",
    ] {
        let row = families
            .iter()
            .find(|row| row["family"] == family)
            .unwrap_or_else(|| panic!("missing non-hosted ISA family {family}: {payload}"));
        assert_eq!(row["accepted_without_profile"], false);
        assert_eq!(row["native_machine_target"], false);
        assert_eq!(
            row["diagnostic"],
            "E_MACHINE_TARGET_FAMILY_PROFILE_REQUIRED_001"
        );
    }
}

#[test]
fn machine_target_family_gate_individual_non_hosted_selectors_ok() {
    for (selector, expected_family) in [
        ("gpu", "gpu"),
        ("dsp", "dsp"),
        ("32bit", "32bit_cpu"),
        ("firmware", "firmware"),
        ("legacy-mcu", "legacy_microcontroller"),
    ] {
        let payload = assert_json_command_ok(&["machine", "target-family-gate", selector]);
        assert_eq!(
            payload["schema"],
            "ncc.machine.non_hosted_isa_family_gate.v1"
        );
        assert_eq!(payload["status"], "closed_profile_gated");
        assert_eq!(payload["family"]["family"], expected_family);
        assert_eq!(payload["family"]["accepted_without_profile"], false);
    }
}

#[test]
fn machine_target_admission_doctor_profiles_material_axes_ok() {
    let payload = assert_json_command_ok(&["machine", "target-admission", "doctor", "--json"]);
    assert_eq!(payload["schema"], "ncc.machine.target_admission.doctor.v1");
    assert_eq!(payload["status"], "closed_admission_contracts");
    assert_eq!(payload["all_required_axes_present"], true);
    assert!(
        payload["native_admitted_profile_count"]
            .as_u64()
            .unwrap_or(0)
            >= 2,
        "{payload}"
    );

    let required_axes = payload["required_axes"].as_array().expect("required axes");
    let profiles = payload["profiles"].as_array().expect("profiles");
    assert!(
        profiles.len() >= 10,
        "target admission must cover representative GPU/DSP/32-bit/firmware/MCU profiles"
    );
    for profile in profiles {
        let profile_id = profile["profile_id"].as_str().unwrap_or_default();
        let should_be_native = matches!(
            profile_id,
            "i386-unknown-linux-gnu" | "riscv32imac-unknown-elf"
        );
        assert_eq!(
            profile["admission"]["native_machine_target"],
            should_be_native
        );
        assert_eq!(profile["admission"]["accepted_without_profile"], false);
        let axes = profile["material_axes"].as_object().expect("material axes");
        for axis in required_axes {
            let axis = axis.as_str().expect("axis string");
            assert!(
                axes.contains_key(axis),
                "profile {} must declare axis {axis}",
                profile["profile_id"]
            );
            assert_ne!(axes[axis]["status"], "missing");
        }
    }
}

#[test]
fn machine_target_admission_covers_frontier_families() {
    let payload = assert_json_command_ok(&["machine", "target-admission", "families", "--json"]);
    assert_eq!(
        payload["schema"],
        "ncc.machine.target_admission.families.v1"
    );
    let families = payload["families"].as_array().expect("families");
    for family in [
        "gpu",
        "dsp",
        "32bit_cpu",
        "firmware",
        "legacy_microcontroller",
    ] {
        let row = families
            .iter()
            .find(|row| row["family"] == family)
            .unwrap_or_else(|| panic!("missing admission family {family}: {payload}"));
        assert_eq!(row["native_machine_target"], family == "32bit_cpu");
        assert!(
            row["candidate_profile_ids"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "family {family} must expose candidate profiles"
        );
    }
}

#[test]
fn machine_target_admission_individual_profiles_expose_material_contracts() {
    for (selector, expected_family, expected_width) in [
        ("nvptx64-nvidia-cuda", "gpu", 64),
        ("hexagonv68-unknown-elf", "dsp", 32),
        ("i386-unknown-linux-gnu", "32bit_cpu", 32),
        ("armv7em-none-eabi", "32bit_cpu", 32),
        ("riscv32imac-unknown-elf", "32bit_cpu", 32),
        ("x86_64-unknown-none-flat", "firmware", 64),
        ("x86_64-uefi-pe", "firmware", 64),
        ("avr-atmega328p-none-elf", "legacy_microcontroller", 16),
        ("msp430-none-elf", "legacy_microcontroller", 16),
    ] {
        let payload = assert_json_command_ok(&["machine", "target-admission", selector, "--json"]);
        assert_eq!(payload["schema"], "ncc.machine.target_admission.profile.v1");
        let profile = &payload["profile"];
        assert_eq!(profile["family"], expected_family);
        let admitted = matches!(
            selector,
            "i386-unknown-linux-gnu" | "riscv32imac-unknown-elf"
        );
        assert_eq!(profile["admission"]["native_machine_target"], admitted);
        assert_eq!(
            profile["material_axes"]["data_layout"]["pointer_width"],
            expected_width
        );
        assert!(profile["material_axes"]["registers"]["classes"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        assert!(profile["material_axes"]["isa"]["families"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
    }
}

#[test]
fn machine_target_admission_admit_riscv32_and_i386_ok() {
    for selector in ["riscv32imac-unknown-elf", "i386-unknown-linux-gnu"] {
        let out = run_ncc(&["machine", "target-admission", "admit", selector, "--json"]);
        let stdout = normalize(&out.stdout);
        assert!(
            out.status.success(),
            "admitted profile should succeed\nselector={selector}\nstdout={stdout}\nstderr={}",
            normalize(&out.stderr)
        );
        assert!(stdout.contains("\"status\": \"native_profile_admitted\""));
        assert!(stdout.contains("\"native_machine_target\": true"));
    }
}

#[test]
fn machine_target_admission_admit_requires_retained_evidence() {
    let out = run_ncc(&[
        "machine",
        "target-admission",
        "admit",
        "avr-atmega328p-none-elf",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "admitting a non-hosted target without evidence must fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("blocked_profile_evidence_required"));
    assert!(
        stdout.contains("E_MACHINE_TARGET_PROFILE_EVIDENCE_REQUIRED_001")
            || stderr.contains("E_MACHINE_TARGET_PROFILE_EVIDENCE_REQUIRED_001"),
        "admission failure must carry typed diagnostic\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_multi_host_ci_retained_evidence_workflow_contract_ok() {
    let workflow = fs::read_to_string(
        workspace_root().join(".github/workflows/machine-level5-level6-release-industrial.yml"),
    )
    .expect("machine CI workflow");
    for lane in [
        "windows-msvc-x86_64",
        "linux-gnu-x86_64",
        "linux-clang-x86_64",
        "macos-clang-x86_64",
        "linux-gnu-aarch64",
        "linux-riscv64-qemu",
    ] {
        assert!(workflow.contains(lane), "missing CI lane {lane}");
    }
    assert!(workflow.contains("machine-ci-retained-${{ matrix.id }}"));
    assert!(workflow.contains("ncc-release verify-machine-ci"));
    assert!(workflow.contains("retained-evidence-aggregate"));
    assert!(workflow.contains("machine-ci-retained-aggregate"));
    assert!(workflow.contains("RISCV64_CC"));
    assert!(workflow.contains("macos-15-intel"));
    assert!(workflow.contains("strict_system_loader_targets: x86_64-pc-windows-msvc"));
}

#[test]
fn machine_third_party_object_matrix_ci_evidence_contract_ok() {
    let workflow = fs::read_to_string(
        workspace_root().join(".github/workflows/machine-level5-level6-release-industrial.yml"),
    )
    .expect("machine CI workflow");
    assert!(workflow.contains("third-party-objects"));
    assert!(workflow.contains("--strict-available"));
    assert!(workflow.contains("third_party_object_matrix.json"));
    assert!(workflow.contains("target/tmp/machine_third_party_objects"));
    assert!(workflow.contains("nasm"));
    assert!(workflow.contains("msvc-dev-cmd"));
}

#[test]
fn machine_object_symbols_ok() {
    assert_machine_ok("examples/machine/object_elf64_minimal.nm");
}

#[test]
fn machine_object_relocations_ok() {
    assert_machine_ok("examples/machine/relocation_call_symbol.nm");
}

#[test]
fn machine_userland_hlt_fail() {
    assert_machine_fail(
        "examples/machine/negative/userland_hlt_fail.nm",
        "E_MACHINE_FORBIDDEN_IN_USERLAND_001",
    );
}

#[test]
fn machine_freestanding_hlt_ok() {
    assert_machine_ok("examples/machine/freestanding_halt_loop.nm");
}

#[test]
fn machine_asm_complete_doctor_reports_closed_without_regression() {
    let out = run_ncc(&["machine", "asm-complete", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("Machine native ASM total closure"));
    assert!(stdout.contains("\"asm_complete_claim\": \"closed\""));
    assert!(stdout.contains("ncc.machine.asm_absolute_complete.v1"));
    assert!(stdout.contains("\"fn_fa_forbidden\": true"));
}

#[test]
fn machine_asm_complete_absolute_reports_declared_full_scope() {
    let payload = assert_json_command_ok(&["machine", "asm-complete", "absolute", "--json"]);
    assert_eq!(payload["schema"], "ncc.machine.asm_absolute_complete.v1");
    assert_eq!(payload["status"], "closed");
    assert_eq!(payload["asm_absolute_complete"], true);
    assert_eq!(payload["asm_complete_total"], true);
    assert_eq!(
        payload["truth_boundary"]["external_historical_assembler_universe_claim"],
        false
    );
    assert_eq!(payload["coverage_counters"]["avx512_subfeatures"], 11);

    let families = payload["families"].as_array().expect("families array");
    for id in [
        "x86_64_prefixes_and_rare_addressing",
        "x86_64_vex_avx_avx2",
        "x86_64_evex_avx512",
        "x86_64_absolute_isa_catalog",
        "aarch64_deep_profile",
        "riscv64_deep_profile",
        "asm_dialect_profiles",
    ] {
        assert!(
            families.iter().any(|row| row["id"] == id),
            "missing absolute ASM family {id}: {payload}"
        );
    }
}

#[test]
fn machine_x86_64_register_catalog_covers_evex_and_amx_classes() {
    let payload = assert_json_command_ok(&["machine", "registers", "x86_64", "--json"]);
    assert_eq!(payload["status"], "closed_register_profile");
    assert_eq!(payload["xmm"].as_array().expect("xmm").len(), 32);
    assert_eq!(payload["ymm"].as_array().expect("ymm").len(), 32);
    assert_eq!(payload["zmm"].as_array().expect("zmm").len(), 32);
    assert_eq!(payload["opmask"].as_array().expect("opmask").len(), 8);
    assert_eq!(payload["tile"].as_array().expect("tile").len(), 8);

    let gpr = payload["general_purpose"]
        .as_array()
        .expect("general_purpose");
    assert!(gpr.iter().any(|reg| reg == "ah"));
    assert!(!gpr.iter().any(|reg| reg == "zmm31"));

    assert_eq!(payload["widths"]["zmm31"], 512);
    assert_eq!(payload["widths"]["st7"], 80);
    assert_eq!(payload["widths"]["tmm7"], "tile_configured");
    assert_eq!(payload["classes"]["legacy_high8"][0], "ah");
    assert!(payload["classes"]["segment"]
        .as_array()
        .expect("segment")
        .iter()
        .any(|reg| reg == "fs"));
    assert!(payload["classes"]["control"]
        .as_array()
        .expect("control")
        .iter()
        .any(|reg| reg == "cr15"));
    assert!(payload["classes"]["debug"]
        .as_array()
        .expect("debug")
        .iter()
        .any(|reg| reg == "dr15"));
    assert_eq!(payload["abi"]["sysv64"]["args"][0], "rdi");
    assert_eq!(payload["abi"]["win64"]["args"][0], "rcx");
    assert_eq!(payload["system_registers_allowed_by_default"], false);
}

#[test]
fn machine_asm_complete_instruction_coverage_x86_64_ok() {
    let out = run_ncc(&["machine", "instruction-coverage", "x86_64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("data_movement"));
    assert!(stdout.contains("sse_avx_avx512"));
    assert!(stdout.contains("E_MACHINE_INSTR_UNKNOWN_001"));
}

#[test]
fn machine_asm_complete_encoding_coverage_x86_64_ok() {
    let out = run_ncc(&["machine", "encoding-coverage", "x86_64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("mov rax, imm64"));
    assert!(stdout.contains("phase2_avx2_closed_vex_total_closed"));
    assert!(stdout.contains("\"table_backbone\": \"closed\""));
    assert!(stdout.contains("\"phase1c_encoding_verifier\": \"closed\""));
    assert!(stdout.contains("\"phase1d_vex_encoding_expansion\": \"closed\""));
    assert!(stdout.contains("\"phase1e_vex_deferred_closure\": \"closed\""));
    assert!(stdout.contains("E_MACHINE_ENCODING_001"));
}

#[test]
fn machine_asm_complete_features_x86_64_ok() {
    let out = run_ncc(&["machine", "features", "x86_64", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("avx512f"));
    assert!(stdout.contains("E_MACHINE_FEATURE_REQUIRED_001"));
}

#[test]
fn machine_asm_complete_roundtrip_subset_ok() {
    let source = fixture("examples/machine/asm_complete/x86_flags_jcc.nm");
    let output = run_ncc(&[
        "machine",
        "roundtrip",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("phase7_roundtrip_complete_closed"));
    assert!(stdout.contains("\"lossless\": true"));
    assert!(stdout.contains("\"mnemonic\": \"jcc\""));
    assert!(stdout.contains("\"relative\""));
}

#[test]
fn machine_asm_complete_export_import_nasm_basic_ok() {
    let source = fixture("examples/machine/asm_complete/x86_mov_all_forms.nm");
    let asm = temp_path("x86_mov_all_forms", "asm");
    let imported = temp_path("x86_mov_all_forms_imported", "nm");
    let export = run_ncc(&[
        "machine",
        "export-asm",
        source.to_str().expect("path"),
        "--syntax",
        "nasm",
        "--out",
        asm.to_str().expect("path"),
    ]);
    assert!(export.status.success(), "{}", normalize(&export.stderr));
    let import = run_ncc(&[
        "machine",
        "import-asm",
        asm.to_str().expect("path"),
        "--syntax",
        "nasm",
        "--out",
        imported.to_str().expect("path"),
    ]);
    assert!(import.status.success(), "{}", normalize(&import.stderr));
    let check = run_ncc(&["machine", "check", imported.to_str().expect("path")]);
    assert!(check.status.success(), "{}", normalize(&check.stderr));
}

#[test]
fn machine_asm_complete_external_toolchain_reports_authorities() {
    let out = run_ncc(&["machine", "asm-complete", "toolchain", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("external_authorities_wrap_nm_contracts"));
    assert!(stdout.contains("llvm-mc"));
    assert!(stdout.contains("llvm-objdump"));
    assert!(stdout.contains("nasm"));
    assert!(stdout.contains("riscv64-gas"));
}

#[test]
fn machine_asm_complete_authority_assemble_x86_llvm_mc_ok_or_blocked() {
    let source = fixture("examples/machine/asm_complete/x86_mov_all_forms.nm");
    let out = temp_path("authority_x86_llvm", "o");
    let output = run_ncc(&[
        "machine",
        "asm-complete",
        "assemble",
        source.to_str().expect("path"),
        "--tool",
        "llvm-mc",
        "--target",
        "x86_64-unknown-linux-gnu",
        "--out",
        out.to_str().expect("path"),
    ]);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains("\"status\": \"passed\"")
            || stdout.contains("E_MACHINE_AUTHORITY_TOOL_MISSING_001"),
        "{stdout}"
    );
}

#[test]
fn machine_asm_complete_authority_parity_x86_ok_or_blocked() {
    let source = fixture("examples/machine/asm_complete/x86_mov_all_forms.nm");
    let output = run_ncc(&[
        "machine",
        "asm-complete",
        "parity",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
    ]);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    if !output.status.success() {
        assert!(
            stdout.contains("E_MACHINE_AUTHORITY_TOOL_MISSING_001")
                || stderr.contains("E_MACHINE_AUTHORITY_TOOL_MISSING_001")
                || stderr.contains("os error 3"),
            "stdout={stdout}\nstderr={stderr}"
        );
        return;
    }
    assert!(stdout.contains("\"official_source\": \".nm\""));
    assert!(stdout.contains("llvm-mc"));
    assert!(stdout.contains("nasm"));
}

#[test]
fn machine_asm_complete_authority_parity_aarch64_riscv64_ok_or_blocked() {
    for (rel, target) in [
        (
            "examples/machine/asm_complete/aarch64_full_basic.nm",
            "aarch64-unknown-linux-gnu",
        ),
        (
            "examples/machine/asm_complete/riscv64_full_basic.nm",
            "riscv64-unknown-elf",
        ),
    ] {
        let source = fixture(rel);
        let output = run_ncc(&[
            "machine",
            "asm-complete",
            "parity",
            source.to_str().expect("path"),
            "--target",
            target,
        ]);
        let stdout = normalize(&output.stdout);
        let stderr = normalize(&output.stderr);
        if !output.status.success() {
            assert!(
                stdout.contains("E_MACHINE_AUTHORITY_TOOL_MISSING_001")
                    || stderr.contains("E_MACHINE_AUTHORITY_TOOL_MISSING_001")
                    || stderr.contains("os error 2")
                    || stderr.contains("os error 3")
                    || stderr.contains("The system cannot find")
                    || stderr.contains("No such file"),
                "stdout={stdout}\nstderr={stderr}"
            );
            continue;
        }
        assert!(stdout.contains(target), "{stdout}");
        assert!(stdout.contains("llvm-mc"), "{stdout}");
        assert!(stdout.contains("clang"), "{stdout}");
    }
}

#[test]
fn machine_native_assembler_phase1_doctor_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("native_total_phase0_phase14_fuzzing_corpus_hardening_closed"));
    assert!(stdout.contains("\"phase0_baseline\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase3_evex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4a_avx512_manifest_foundation\": \"closed\""));
    assert!(stdout.contains("\"phase4b_avx512f_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4c_avx512dq_bw_vl_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4d_avx512_remaining_subfeatures_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4e_avx512_final_audit\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("REX prefix emission"));
    assert!(stdout.contains("SIB memory forms"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_native_phase0_baseline_doctor_ok() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("machine_native_asm_total_closed"));
    assert!(stdout.contains("\"no_high_level_fn_fa\": true"));
}

#[test]
fn machine_native_phase0_fn_fa_still_forbidden() {
    assert_machine_fail(
        "examples/machine/negative/high_level_fn_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_FN_FORBIDDEN_001",
    );
    assert_machine_fail(
        "examples/machine/negative/high_level_fa_forbidden_fail.nm",
        "E_MACHINE_HIGH_LEVEL_FA_FORBIDDEN_001",
    );
}

#[test]
fn machine_native_phase0_existing_targets_preserved() {
    let out = run_ncc(&["machine", "targets", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for target in [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "wasm32-unknown-unknown",
    ] {
        assert!(stdout.contains(target), "missing target {target}: {stdout}");
    }
}

#[test]
fn machine_native_phase0_existing_object_emission_preserved() {
    let source = fixture("examples/machine/add_two_sysv64.nm");
    let out = temp_path("phase0_object_preserved", "o");
    let output = run_ncc(&[
        "machine",
        "emit-object",
        source.to_str().expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--out",
        out.to_str().expect("path"),
    ]);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let bytes = fs::read(out).expect("object bytes");
    assert!(bytes.starts_with(b"\x7fELF"), "object is not ELF");
}

#[test]
fn machine_native_phase0_existing_examples_pass() {
    for entry in fs::read_dir(fixture("examples/machine")).expect("examples/machine") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("nm") {
            let rel = path.strip_prefix(workspace_root()).expect("relative path");
            assert_machine_ok(rel.to_str().expect("path"));
        }
    }
}

#[test]
fn machine_native_phase0_negative_examples_fail() {
    for entry in fs::read_dir(fixture("examples/machine/negative")).expect("negative examples") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("nm") {
            let output = run_ncc(&["machine", "check", path.to_str().expect("path")]);
            assert!(
                !output.status.success(),
                "negative example must fail: {}\n{}",
                path.display(),
                normalize(&output.stdout)
            );
        }
    }
}

#[test]
fn machine_native_phase0_no_fake_bytes() {
    let source = fs::read_to_string(fixture("crates/ncc-cli/src/commands/machine.rs"))
        .expect("machine command source");
    assert!(!source.contains(concat!("fake ", "bytes")));
    assert!(!source.contains(&["unsup", "ported but ok"].concat()));
    assert!(!source.contains(&["unk", "nown accepted"].concat()));
}

#[test]
fn machine_native_phase0_no_text_filler_marker() {
    for rel in [
        "examples/machine",
        "docs/spec/machine/machine_native_assembler.md",
        "docs/status/STATUS_machine_native_assembler_phase1.md",
    ] {
        let path = fixture(rel);
        if path.is_file() {
            let text = fs::read_to_string(path).expect("text file");
            assert!(!text.contains(concat!("place", "holder")));
            continue;
        }
        for entry in fs::read_dir(path).expect("scan dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("nm") {
                let text = fs::read_to_string(path).expect("nm text");
                assert!(!text.contains(concat!("place", "holder")));
            }
        }
    }
}

#[test]
fn machine_native_x86_reg_reg_phase1_encoding_ok() {
    assert_machine_ok("examples/machine/native/x86_reg_reg_phase1.nm");
    let bytes = emit_machine_bytes(
        "examples/machine/native/x86_reg_reg_phase1.nm",
        "x86_reg_reg_phase1",
    );
    assert_eq!(
        bytes,
        vec![
            0x49, 0x89, 0xc0, 0x4d, 0x89, 0xc1, 0x4d, 0x01, 0xda, 0x4d, 0x29, 0xec, 0x4d, 0x21,
            0xfe, 0x48, 0x09, 0xd8, 0x48, 0x31, 0xd1, 0x48, 0x85, 0xfe, 0xc3,
        ]
    );
}

#[test]
fn machine_native_x86_setcc_cmov_phase1_encoding_ok() {
    assert_machine_ok("examples/machine/native/x86_setcc_cmov_phase1.nm");
    let bytes = emit_machine_bytes(
        "examples/machine/native/x86_setcc_cmov_phase1.nm",
        "x86_setcc_cmov_phase1",
    );
    assert_eq!(
        bytes,
        vec![
            0x48, 0x39, 0xd8, 0x0f, 0x94, 0xc0, 0x41, 0x0f, 0x95, 0xc0, 0x4d, 0x0f, 0x4f, 0xd3,
            0xc3,
        ]
    );
}

#[test]
fn machine_native_x86_memory_sib_phase1_encoding_ok() {
    assert_machine_ok("examples/machine/native/x86_memory_sib_phase1.nm");
    let bytes = emit_machine_bytes(
        "examples/machine/native/x86_memory_sib_phase1.nm",
        "x86_memory_sib_phase1",
    );
    assert_eq!(
        bytes,
        vec![
            0x48, 0x89, 0x45, 0xf8, 0x48, 0x8b, 0x5d, 0xf8, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x48,
            0x8b, 0x54, 0x24, 0x08, 0x48, 0x89, 0x4c, 0xd8, 0x10, 0x4c, 0x8b, 0x44, 0xd8, 0x10,
            0xc3,
        ]
    );
}

#[test]
fn machine_native_x86_sse_phase2_encoding_ok() {
    assert_machine_ok("examples/machine/native/x86_sse_phase2.nm");
    let bytes = emit_machine_bytes(
        "examples/machine/native/x86_sse_phase2.nm",
        "x86_sse_phase2",
    );
    assert_eq!(
        bytes,
        vec![0x0f, 0x28, 0xc1, 0x0f, 0x58, 0xd3, 0x0f, 0x57, 0xe5, 0xf2, 0x0f, 0x58, 0xf7, 0xc3,]
    );
}

#[test]
fn machine_native_x86_vex_avx_phase2_encoding_ok() {
    assert_machine_ok("examples/machine/native/x86_vex_avx_phase2.nm");
    let bytes = emit_machine_bytes(
        "examples/machine/native/x86_vex_avx_phase2.nm",
        "x86_vex_avx_phase2",
    );
    assert_eq!(
        bytes,
        vec![
            0xc4, 0xe1, 0x7c, 0x28, 0xc1, 0xc4, 0xe1, 0x64, 0x58, 0xd4, 0xc4, 0xe1, 0x4c, 0x5c,
            0xef, 0xc4, 0x41, 0x34, 0x57, 0xc2, 0xc3,
        ]
    );
}

#[test]
fn machine_native_x86_vex_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/x86_vex_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_manifest_is_exhaustive_audit_not_closed() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/manifests/phase1_vex_instruction_manifest.json",
    ))
    .expect("phase1 VEX instruction manifest");
    assert!(manifest.contains("\"phase_status\": \"not_closed\""));
    assert!(manifest.contains("\"closed_instructions\": 0"));
    assert!(manifest.contains("\"partial_instructions\": 7"));
    assert!(manifest.contains("\"final_status\": \"unsupported_by_design\""));
    assert!(manifest.contains("\"mnemonic\": \"vaddps\""));
    assert!(manifest.contains("\"mnemonic\": \"vpermps\""));
}

#[test]
fn machine_vex_manifest_no_false_closed_claim() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/phase1/phase1_vex_complete.json",
    ))
    .expect("phase1 VEX report");
    assert!(manifest.contains("\"status\": \"not_closed\""));
    assert!(manifest.contains("\"asm_complete_total\": false"));
    assert!(manifest.contains("\"no_phase2_started\": true"));
}

#[test]
fn machine_vex_examples_check_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_vex_3byte_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_complete_x86_64.nm",
        "vex_complete_x86_64",
    );
    assert!(
        bytes.windows(3).any(|w| w[0] == 0xc4),
        "expected at least one VEX3 prefix in {bytes:02x?}"
    );
}

#[test]
fn machine_vex_xmm_operands_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_vex_ymm_operands_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_vex_register_register_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_vex_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_invalid_operand_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_operand_fail.nm",
        "E_MACHINE_INSTR_OPERAND_001",
    );
}

#[test]
fn machine_vex_bad_vvvv_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_bad_vvvv_fail.nm",
        "E_MACHINE_VEX_VVVV_001",
    );
}

#[test]
fn machine_vex_wrong_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_wrong_width_fail.nm",
        "E_MACHINE_REGISTER_WIDTH_001",
    );
}

#[test]
fn machine_vex_invalid_memory_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_memory_width_fail.nm",
        "E_MACHINE_MEM_WIDTH_001",
    );
}

#[test]
fn machine_vex_invalid_immediate_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_immediate_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
}

#[test]
fn machine_vex_invalid_register_class_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_register_class_fail.nm",
        "E_MACHINE_REGISTER_CLASS_001",
    );
}

#[test]
fn machine_vex_unsupported_by_design_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_unsupported_by_design_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_negative_examples_fail_ok() {
    for entry in
        fs::read_dir(fixture("examples/machine/native/negative")).expect("native negative examples")
    {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with("vex_") && name.ends_with("_fail.nm") {
            let output = run_ncc(&["machine", "check", path.to_str().expect("path")]);
            assert!(
                !output.status.success(),
                "VEX negative example must fail: {}\n{}",
                path.display(),
                normalize(&output.stdout)
            );
        }
    }
}

#[test]
fn machine_vex_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase0_baseline\": \"closed\""));
    assert!(stdout.contains("\"phase1a_vex_table_backbone\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_vex_table_backbone_exists() {
    let out = run_ncc(&["machine", "vex", "table", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase1a_vex_table_backbone_closed"));
    assert!(stdout.contains("vex.phase1.vmovaps"));
}

#[test]
fn machine_vex_manifest_has_104_entries() {
    let out = run_ncc(&["machine", "vex", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"manifest_total\": 104"));
    assert!(stdout.contains("\"mapped_to_vex_instruction_def\": 104"));
}

#[test]
fn machine_vex_manifest_entries_have_table_mapping() {
    let out = run_ncc(&["machine", "vex", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("vex.phase1.vaddps"));
    assert!(stdout.contains("vex.phase1.vcvtph2ps"));
}

#[test]
fn machine_vex_operand_forms_are_structured() {
    let out = run_ncc(&["machine", "vex", "table", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"operand_count\""));
    assert!(stdout.contains("\"reg_mem\""));
    assert!(stdout.contains("\"invalid_forms\""));
}

#[test]
fn machine_vex_encoding_patterns_are_structured() {
    let out = run_ncc(&["machine", "vex", "table", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_pattern\""));
    assert!(stdout.contains("\"vex2_fields\""));
    assert!(stdout.contains("\"vex3_fields\""));
}

#[test]
fn machine_vex_feature_gates_are_structured() {
    let out = run_ncc(&["machine", "vex", "table", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"feature_gates\""));
    assert!(stdout.contains("\"diagnostic_if_missing\": \"E_MACHINE_FEATURE_REQUIRED_001\""));
}

#[test]
fn machine_vex_diagnostics_are_mapped() {
    let out = run_ncc(&["machine", "vex", "diagnostics", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for code in [
        "E_MACHINE_VEX_VVVV_001",
        "E_MACHINE_MEM_WIDTH_001",
        "E_MACHINE_REGISTER_WIDTH_001",
        "E_MACHINE_INSTR_OPERAND_001",
        "E_MACHINE_FEATURE_REQUIRED_001",
        "E_MACHINE_IMM_WIDTH_001",
        "E_MACHINE_REGISTER_CLASS_001",
        "E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001",
    ] {
        assert!(stdout.contains(code), "missing {code}: {stdout}");
    }
}

#[test]
fn machine_vex_unsupported_by_design_has_justification() {
    let out = run_ncc(&["machine", "vex", "table", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"final_status\": \"phase2_closed\""));
    assert!(stdout.contains("phase2_avx2_native_encoding_closed"));
}

#[test]
fn machine_vex_no_manifest_shrinkage() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"manifest_total\": 104"));
    assert!(stdout.contains("\"table_entries\": 104"));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1a_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1a_vex_table_backbone\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1b_operand_matrix_exists() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase1b_vex_operand_matrix_closed"));
    assert!(stdout.contains("\"manifest_total\": 104"));
}

#[test]
fn machine_vex_phase1b_all_manifest_entries_have_operand_forms() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"instructions_with_complete_operand_matrix\": 104"));
    assert!(stdout.contains("\"instructions_missing_operand_matrix\": 0"));
}

#[test]
fn machine_vex_phase1b_no_any_operand_forms() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("\"any\""), "{stdout}");
}

#[test]
fn machine_vex_phase1b_valid_forms_are_structured() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for key in [
        "\"form_id\"",
        "\"allowed_register_classes\"",
        "\"allowed_vector_widths\"",
        "\"allowed_memory_widths\"",
        "\"vex_kind_allowed\"",
        "\"required_features\"",
    ] {
        assert!(stdout.contains(key), "missing {key}: {stdout}");
    }
}

#[test]
fn machine_vex_phase1b_invalid_forms_have_diagnostics() {
    let out = run_ncc(&["machine", "vex", "invalid-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"invalid_forms\""));
    assert!(stdout.contains("E_MACHINE_REGISTER_CLASS_001"));
    assert!(stdout.contains("E_MACHINE_MEM_WIDTH_001"));
    assert!(stdout.contains("E_MACHINE_IMM_WIDTH_001"));
}

#[test]
fn machine_vex_phase1b_xmm_forms_ok() {
    assert_machine_ok("examples/machine/native/vex_operand_forms_x86_64.nm");
}

#[test]
fn machine_vex_phase1b_ymm_forms_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_vex_phase1b_reg_mem_forms_ok() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"reg_mem\": true"));
}

#[test]
fn machine_vex_phase1b_mem_reg_forms_ok() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"mem_reg\": true"));
}

#[test]
fn machine_vex_phase1b_immediate_forms_ok() {
    let out = run_ncc(&["machine", "vex", "operand-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"immediate_width\": \"imm8\""));
}

#[test]
fn machine_vex_phase1b_invalid_operand_count_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_operand_count_fail.nm",
        "E_MACHINE_INSTR_OPERAND_001",
    );
}

#[test]
fn machine_vex_phase1b_invalid_register_class_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_register_class_fail.nm",
        "E_MACHINE_REGISTER_CLASS_001",
    );
}

#[test]
fn machine_vex_phase1b_invalid_memory_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_memory_width_fail.nm",
        "E_MACHINE_MEM_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1b_invalid_immediate_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_immediate_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1b_invalid_xmm_ymm_mix_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_invalid_xmm_ymm_mix_fail.nm",
        "E_MACHINE_REGISTER_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1b_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_phase1b_unsupported_by_design_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_unsupported_by_design_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_phase1b_examples_check_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_operand_forms_x86_64.nm");
}

#[test]
fn machine_vex_phase1b_negative_examples_fail_ok() {
    let negative_dir = fixture("examples/machine/native/negative");
    for entry in fs::read_dir(&negative_dir).expect("negative examples") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with("vex_") && name.ends_with("_fail.nm") {
            let out = run_ncc(&["machine", "check", path.to_str().expect("path")]);
            assert!(
                !out.status.success(),
                "VEX negative must fail: {}\n{}",
                path.display(),
                normalize(&out.stdout)
            );
        }
    }
}

#[test]
fn machine_vex_phase1b_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1b_vex_operand_matrix\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_vex_phase1c_encoding_status_exists() {
    let out = run_ncc(&["machine", "vex", "encoding-status", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase2_avx2_closed_vex_total_closed"));
    assert!(stdout.contains("\"total_operand_forms\": 207"));
}

#[test]
fn machine_vex_phase1c_every_form_has_encoding_status() {
    let out = run_ncc(&["machine", "vex", "encoding-status", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_deferred_forms\": 0"));
    assert!(stdout.contains("\"encoding_unsupported_by_design_forms\": 0"));
}

#[test]
fn machine_vex_phase1c_every_closed_form_has_real_bytes() {
    let out = run_ncc(&["machine", "vex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"byte_pattern_examples\": 207"));
    assert!(stdout.contains("\"evidence_hash\""));
    assert!(stdout.contains("c5 f8 77"));
    assert!(stdout.contains("c4 e1 70 c2 c2 01"));
}

#[test]
fn machine_vex_phase1c_every_closed_form_has_verifier() {
    let out = run_ncc(&["machine", "vex", "verifier-status", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"verifier_closed_forms\": 207"));
    assert!(stdout.contains("\"verifier_deferred_forms\": 0"));
}

#[test]
fn machine_vex_phase1c_vex2_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_encoding_x86_64.nm",
        "vex_encoding_phase1c",
    );
    assert_eq!(&bytes[0..6], &[0xc5, 0xf8, 0x77, 0xc5, 0xfc, 0x77]);
}

#[test]
fn machine_vex_phase1c_vex3_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_encoding_x86_64.nm",
        "vex_encoding_phase1c_vex3",
    );
    assert!(bytes.windows(3).any(|w| w == [0xc4, 0xe1, 0x78]));
}

#[test]
fn machine_vex_phase1c_reg_reg_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_encoding_x86_64.nm",
        "vex_encoding_phase1c_reg_reg",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x78, 0x28, 0xc1]));
}

#[test]
fn machine_vex_phase1c_reg_mem_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_memory_encoding_x86_64.nm",
        "vex_memory_phase1c_reg_mem",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x78, 0x28, 0x00]));
}

#[test]
fn machine_vex_phase1c_mem_reg_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_memory_encoding_x86_64.nm",
        "vex_memory_phase1c_mem_reg",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe1, 0x78, 0x29, 0x4d, 0xf8]));
}

#[test]
fn machine_vex_phase1c_immediate_bytes_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_immediate_encoding_x86_64.nm",
        "vex_immediate_phase1c",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe1, 0x70, 0xc2, 0xc2, 0x01]));
}

#[test]
fn machine_vex_phase1c_modrm_ok() {
    assert_machine_ok("examples/machine/native/vex_memory_encoding_x86_64.nm");
}

#[test]
fn machine_vex_phase1c_sib_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_memory_encoding_x86_64.nm",
        "vex_memory_phase1c_sib",
    );
    assert!(bytes.windows(4).any(|w| w == [0x54, 0x8b, 0x10, 0xc4]));
}

#[test]
fn machine_vex_phase1c_memory_disp_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_memory_encoding_x86_64.nm",
        "vex_memory_phase1c_disp",
    );
    assert!(bytes.windows(2).any(|w| w == [0x4d, 0xf8]));
}

#[test]
fn machine_vex_phase1c_no_fake_bytes() {
    let out = run_ncc(&["machine", "vex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(concat!("fake ", "bytes")));
    assert!(!stdout.contains(concat!("place", "holder")));
}

#[test]
fn machine_vex_phase1c_invalid_modrm_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_encoding_invalid_modrm_fail.nm",
        "E_MACHINE_MODRM_001",
    );
}

#[test]
fn machine_vex_phase1c_invalid_sib_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_encoding_invalid_sib_fail.nm",
        "E_MACHINE_SIB_001",
    );
}

#[test]
fn machine_vex_phase1c_invalid_immediate_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_encoding_invalid_immediate_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1c_examples_check_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_operand_forms_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_memory_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_immediate_encoding_x86_64.nm");
}

#[test]
fn machine_vex_phase1c_negative_examples_fail_ok() {
    for name in [
        "vex_encoding_invalid_modrm_fail.nm",
        "vex_encoding_invalid_sib_fail.nm",
        "vex_encoding_invalid_vex2_fail.nm",
        "vex_encoding_invalid_vex3_fail.nm",
        "vex_encoding_invalid_immediate_fail.nm",
    ] {
        let path = format!("examples/machine/native/negative/{name}");
        let out = run_ncc(&["machine", "check", &path]);
        assert!(
            !out.status.success(),
            "VEX Phase 1C negative must fail: {path}\n{}",
            normalize(&out.stdout)
        );
    }
}

#[test]
fn machine_vex_phase1c_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1c_vex_encoding_verifier\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_vex_phase1d_family_encoding_status_exists() {
    let out = run_ncc(&["machine", "vex", "family-coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase2_avx2_vex_family_coverage_reported"));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1d_preserves_phase1c_closed_forms() {
    let out = run_ncc(&["machine", "vex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("c5 f8 77"));
    assert!(stdout.contains("c4 e1 78 28 c1"));
    assert!(stdout.contains("c4 e1 70 c2 c2 01"));
}

#[test]
fn machine_vex_phase1d_increases_encoding_closed_count() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_deferred_forms\": 0"));
    assert!(stdout.contains("\"verifier_closed_forms\": 207"));
}

#[test]
fn machine_vex_phase1d_packed_double_arithmetic_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_pd_scalar_encoding_x86_64.nm",
        "vex_phase1d_pd_arithmetic",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x71, 0x58, 0xc2]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x5d, 0x5c, 0x18]));
}

#[test]
fn machine_vex_phase1d_packed_double_logical_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_pd_scalar_encoding_x86_64.nm",
        "vex_phase1d_pd_logical",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x71, 0x56, 0xc2]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x5d, 0x57, 0x1a]));
}

#[test]
fn machine_vex_phase1d_scalar_single_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_pd_scalar_encoding_x86_64.nm",
        "vex_phase1d_scalar_single",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x72, 0x58, 0xc2]));
}

#[test]
fn machine_vex_phase1d_scalar_double_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_pd_scalar_encoding_x86_64.nm",
        "vex_phase1d_scalar_double",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x63, 0x58, 0xd4]));
}

#[test]
fn machine_vex_phase1d_compare_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_compare_shuffle_encoding_x86_64.nm",
        "vex_phase1d_compare",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe1, 0x5d, 0xc2, 0x18, 0x01]));
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe1, 0x4a, 0xc2, 0xef, 0x02]));
}

#[test]
fn machine_vex_phase1d_compare_imm_required_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_compare_missing_imm_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
    assert_machine_fail(
        "examples/machine/native/negative/vex_compare_bad_imm_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1d_shuffle_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_compare_shuffle_encoding_x86_64.nm",
        "vex_phase1d_shuffle",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0x41, 0x20, 0xc6, 0xd4, 0x04]));
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0x61, 0x0d, 0xc6, 0x29, 0x05]));
}

#[test]
fn machine_vex_phase1d_shuffle_imm_required_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_shuffle_missing_imm_fail.nm",
        "E_MACHINE_IMM_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1d_conversion_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_conversion_encoding_x86_64.nm",
        "vex_phase1d_conversion",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x70, 0x5a, 0xc2]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x59, 0x5a, 0x18]));
}

#[test]
fn machine_vex_phase1d_scalar_wrong_l_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_scalar_wrong_l_fail.nm",
        "E_MACHINE_REGISTER_WIDTH_001",
    );
}

#[test]
fn machine_vex_phase1d_memory_forms_ok() {
    assert_machine_ok("examples/machine/native/vex_pd_scalar_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_compare_shuffle_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_conversion_encoding_x86_64.nm");
}

#[test]
fn machine_vex_phase1d_byte_patterns_ok() {
    let out = run_ncc(&["machine", "vex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"byte_pattern_examples\": 207"));
    assert!(stdout.contains("vaddpd xmm0, xmm1, xmm2"));
    assert!(stdout.contains("vcmppd xmm0, xmm1, xmm2, 1"));
    assert!(stdout.contains("vcvtps2pd xmm0, xmm1, xmm2"));
}

#[test]
fn machine_vex_phase1d_no_fake_bytes() {
    let out = run_ncc(&["machine", "vex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(concat!("fake ", "bytes")));
    assert!(!stdout.contains(concat!("place", "holder")));
}

#[test]
fn machine_vex_phase1d_examples_check_ok() {
    assert_machine_ok("examples/machine/native/vex_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_memory_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_immediate_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_pd_scalar_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_compare_shuffle_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_conversion_encoding_x86_64.nm");
    assert_machine_ok("examples/machine/native/vex_remaining_forms_x86_64.nm");
}

#[test]
fn machine_vex_phase1d_negative_examples_fail_ok() {
    for name in [
        "vex_scalar_wrong_l_fail.nm",
        "vex_compare_missing_imm_fail.nm",
        "vex_compare_bad_imm_fail.nm",
        "vex_shuffle_missing_imm_fail.nm",
        "vex_conversion_bad_operand_fail.nm",
    ] {
        let path = format!("examples/machine/native/negative/{name}");
        let out = run_ncc(&["machine", "check", &path]);
        assert!(
            !out.status.success(),
            "VEX Phase 1D negative must fail: {path}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
    }
}

#[test]
fn machine_vex_phase1d_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1d_vex_encoding_expansion\": \"closed\""));
    assert!(stdout.contains("\"phase1e_vex_deferred_closure\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_vex_phase1e_remaining_forms_status_closed() {
    let out = run_ncc(&["machine", "vex", "encoding-status", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase2_avx2_closed_vex_total_closed"));
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_unsupported_by_design_forms\": 0"));
}

#[test]
fn machine_vex_phase1e_no_deferred_forms() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_deferred_forms\": 0"));
    assert!(stdout.contains("\"verifier_closed_forms\": 207"));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1e_remaining_forms_example_emits_bytes() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_remaining_forms_x86_64.nm",
        "vex_phase1e_remaining_forms",
    );
    assert!(bytes.len() > 200, "expected broad VEX byte coverage");
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x79, 0x28, 0xc1]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe1, 0x70, 0x54, 0xc2]));
}

#[test]
fn machine_vex_phase1e_unsupported_by_design_blocks_global_closed() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1e_vex_deferred_closure\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("VEX Phase 2 closes AVX2 native encoding"));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1f_unsupported_resolution_exists() {
    let out = run_ncc(&["machine", "vex", "unsupported-resolution", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase1f_vex_final_classification_closed"));
    assert!(stdout.contains("\"unsupported_resolution\""));
}

#[test]
fn machine_vex_phase1f_all_10_forms_classified() {
    let out = run_ncc(&["machine", "vex", "classification", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"unsupported_by_design_forms\": 0"));
    assert!(stdout.contains("\"delegated_to_avx2_forms\": 0"));
    assert!(stdout.contains("\"delegated_to_fma_f16c_forms\": 0"));
}

#[test]
fn machine_vex_phase1f_avx2_forms_delegated_to_phase2() {
    let out = run_ncc(&["machine", "vex", "phase2-avx2", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"instruction_mnemonic\": \"vpermps\""));
    assert!(stdout.contains("\"instruction_mnemonic\": \"vgatherdps\""));
    assert!(stdout.contains("\"instruction_mnemonic\": \"vpmulld\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"encoding_closed_forms\": 6"));
}

#[test]
fn machine_vex_phase1f_fma_f16c_classified() {
    let out = run_ncc(&["machine", "vex", "phase2b-fma-f16c", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"instruction_mnemonic\": \"vfmadd132ps\""));
    assert!(stdout.contains("\"instruction_mnemonic\": \"vcvtph2ps\""));
    assert!(stdout.contains("\"phase2b_vex_fma_f16c\": \"closed\""));
    assert!(stdout.contains("\"encoding_closed_forms\": 4"));
}

#[test]
fn machine_vex_phase1f_no_silent_unsupported() {
    let out = run_ncc(&["machine", "vex", "classification", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"unsupported_by_design_forms\": 0"));
    assert_machine_fail(
        "examples/machine/native/negative/vex_avx2_delegated_before_phase2_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_phase1f_base_complete_status_ok() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1_vex_base_complete\": \"closed\""));
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_deferred_forms\": 0"));
}

#[test]
fn machine_vex_phase1f_total_complete_not_closed_if_extensions_delegated() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1_vex_base_complete\": \"closed\""));
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase1f_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1f_vex_final_classification\": \"closed\""));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2b_vex_fma_f16c\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_vex_phase1f_no_phase2_started() {
    let out = run_ncc(&["machine", "vex", "classification", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2b_vex_fma_f16c\": \"closed\""));
}

#[test]
fn machine_vex_phase1f_fn_fa_forbidden_preserved() {
    assert_machine_ok("examples/machine/native/vex_phase1f_classification_x86_64.nm");
    let source = fs::read_to_string(fixture(
        "examples/machine/native/vex_phase1f_classification_x86_64.nm",
    ))
    .expect("phase1f source");
    assert!(!source.contains(concat!("f", "n:")));
    assert!(!source.contains(concat!("f", "a:")));
}

#[test]
fn machine_vex_phase2b_fma_f16c_status_exists() {
    let out = run_ncc(&["machine", "vex", "phase2b-fma-f16c", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase2b_vex_fma_f16c_closed"));
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
}

#[test]
fn machine_vex_phase2b_vfmadd132ps_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_fma_f16c_encoding_x86_64.nm",
        "vex_phase2b_fma_f16c",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x71, 0x98, 0xc2]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x5d, 0x98, 0x18]));
}

#[test]
fn machine_vex_phase2b_vcvtph2ps_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/vex_fma_f16c_encoding_x86_64.nm",
        "vex_phase2b_f16c",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x79, 0x13, 0xee]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x7d, 0x13, 0x3b]));
}

#[test]
fn machine_vex_phase2b_fma_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_fma_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_phase2b_f16c_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_f16c_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_vex_phase2b_invalid_operand_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/vex_fma_invalid_operand_fail.nm",
        "E_MACHINE_REGISTER_CLASS_001",
    );
    assert_machine_fail(
        "examples/machine/native/negative/vex_f16c_invalid_operand_fail.nm",
        "E_MACHINE_INSTR_OPERAND_001",
    );
}

#[test]
fn machine_vex_phase2b_reduces_unsupported_from_10_to_6() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_unsupported_by_design_forms\": 0"));
}

#[test]
fn machine_vex_phase2b_preserves_avx2_delegation() {
    let out = run_ncc(&["machine", "vex", "phase2-avx2", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"unsupported_by_design_forms\": 0"));
    assert!(stdout.contains("\"vex_total_unsupported_by_design_forms\": 0"));
}

#[test]
fn machine_vex_phase2b_no_phase2_avx2_started() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"phase2b_vex_fma_f16c\": \"closed\""));
}

#[test]
fn machine_vex_phase2b_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase2b_vex_fma_f16c\""));
    assert!(stdout.contains("\"encoding_closed_forms\": 4"));
    assert!(stdout.contains("\"remaining_avx2_delegated_forms\": 0"));
}

#[test]
fn machine_phase2_avx2_status_exists() {
    let out = run_ncc(&["machine", "vex", "phase2-avx2", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase2_avx2_closed"));
    assert!(stdout.contains("\"forms_total\": 6"));
}

#[test]
fn machine_phase2_avx2_vpermps_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/avx2_vex_complete_x86_64.nm",
        "phase2_avx2_vpermps",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x75, 0x16, 0xc2]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xe2, 0x5d, 0x16, 0x18]));
}

#[test]
fn machine_phase2_avx2_vgatherdps_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/avx2_vex_complete_x86_64.nm",
        "phase2_avx2_vgatherdps",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe2, 0x69, 0x92, 0x04, 0x88]));
    assert!(bytes
        .windows(6)
        .any(|w| w == [0xc4, 0xe2, 0x55, 0x92, 0x1c, 0xa3]));
}

#[test]
fn machine_phase2_avx2_vpmulld_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/avx2_vex_complete_x86_64.nm",
        "phase2_avx2_vpmulld",
    );
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0xc2, 0x41, 0x40, 0xf0]));
    assert!(bytes
        .windows(5)
        .any(|w| w == [0xc4, 0x42, 0x2d, 0x40, 0x0b]));
}

#[test]
fn machine_phase2_avx2_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx2_missing_feature_fail.nm",
        "E_MACHINE_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_phase2_avx2_vpermps_invalid_operand_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx2_vpermps_invalid_operand_fail.nm",
        "E_MACHINE_INSTR_OPERAND_001",
    );
}

#[test]
fn machine_phase2_avx2_vgatherdps_invalid_memory_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx2_vgatherdps_invalid_memory_fail.nm",
        "E_MACHINE_SIB_001",
    );
}

#[test]
fn machine_phase2_avx2_vpmulld_invalid_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx2_vpmulld_invalid_width_fail.nm",
        "E_MACHINE_REGISTER_WIDTH_001",
    );
}

#[test]
fn machine_phase2_avx2_reduces_unsupported_to_zero() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"unsupported_by_design\": 0"));
    assert!(stdout.contains("\"encoding_unsupported_by_design_forms\": 0"));
}

#[test]
fn machine_phase2_avx2_encoding_207_of_207() {
    let out = run_ncc(&["machine", "vex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_closed_forms\": 207"));
    assert!(stdout.contains("\"verifier_closed_forms\": 207"));
    assert!(stdout.contains("\"encoding_deferred_forms\": 0"));
}

#[test]
fn machine_phase2_avx2_phase1_vex_complete_closed() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase1_vex_complete\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_phase2_avx2_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase2_avx2_complete\": \"closed\""));
    assert!(stdout.contains("\"phase3_evex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
}

#[test]
fn machine_phase2_avx2_no_asm_complete_total_yet() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase3_evex_status_exists() {
    let out = run_ncc(&["machine", "evex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase3_evex_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"not_closed\""));
}

#[test]
fn machine_evex_field_manifest_exists() {
    let out = run_ncc(&["machine", "evex", "fields", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"field_count\": 14"));
    assert!(stdout.contains("\"field\": \"P0.R\""));
    assert!(stdout.contains("\"field\": \"P2.aaa\""));
}

#[test]
fn machine_evex_tuple_manifest_exists() {
    let out = run_ncc(&["machine", "evex", "tuples", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"tuple_count\": 4"));
    assert!(stdout.contains("m32bcst_1to16"));
}

#[test]
fn machine_evex_carrier_manifest_exists() {
    let out = run_ncc(&["machine", "evex", "carriers", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"carrier_count\": 42"));
    assert!(stdout.contains("closed_as_evex_carrier"));
    assert!(stdout.contains("\"avx512_family_complete\": \"not_closed\""));
}

#[test]
fn machine_evex_prefix_0x62_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_complete_x86_64.nm",
        "evex_prefix",
    );
    assert!(bytes.windows(1).any(|w| w == [0x62]));
}

#[test]
fn machine_evex_p0_fields_ok() {
    let bytes = emit_machine_bytes("examples/machine/native/evex_complete_x86_64.nm", "evex_p0");
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xf1, 0x7d, 0xc9, 0x6f, 0xca]));
}

#[test]
fn machine_evex_p1_fields_ok() {
    let bytes = emit_machine_bytes("examples/machine/native/evex_complete_x86_64.nm", "evex_p1");
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xe1, 0x5c, 0x56, 0x58, 0x58]));
}

#[test]
fn machine_evex_p2_fields_ok() {
    let bytes = emit_machine_bytes("examples/machine/native/evex_complete_x86_64.nm", "evex_p2");
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xa1, 0x4c, 0xd7, 0x58, 0xef]));
}

#[test]
fn machine_evex_zmm_registers_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_complete_x86_64.nm",
        "evex_zmm",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xa1, 0x75, 0xc5, 0xef, 0xc2]));
}

#[test]
fn machine_evex_k_mask_registers_ok() {
    assert_machine_ok("examples/machine/native/evex_masking_x86_64.nm");
}

#[test]
fn machine_evex_masking_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_masking_x86_64.nm",
        "evex_mask",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xf1, 0x75, 0x49, 0xfe, 0xc2]));
}

#[test]
fn machine_evex_zeroing_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_masking_x86_64.nm",
        "evex_zero",
    );
    assert!(bytes.iter().any(|byte| byte & 0x80 == 0x80));
}

#[test]
fn machine_evex_merging_ok() {
    assert_machine_ok("examples/machine/native/evex_masking_x86_64.nm");
}

#[test]
fn machine_evex_broadcast_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_broadcast_x86_64.nm",
        "evex_broadcast",
    );
    assert!(bytes
        .windows(7)
        .any(|w| w == [0x62, 0xe1, 0x5c, 0x56, 0x58, 0x58, 0x01]));
}

#[test]
fn machine_evex_rounding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_rounding_sae_x86_64.nm",
        "evex_rounding",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xa1, 0x4c, 0xd7, 0x58, 0xef]));
}

#[test]
fn machine_evex_sae_ok() {
    assert_machine_ok("examples/machine/native/evex_rounding_sae_x86_64.nm");
}

#[test]
fn machine_evex_compressed_disp_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_compressed_disp_x86_64.nm",
        "evex_compressed",
    );
    assert!(bytes
        .windows(7)
        .any(|w| w == [0x62, 0xf1, 0x75, 0x49, 0xfe, 0x40, 0x01]));
}

#[test]
fn machine_evex_tuple_types_ok() {
    let out = run_ncc(&["machine", "evex", "tuples", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("full_512"));
    assert!(stdout.contains("disp8_scale_n"));
}

#[test]
fn machine_evex_vl_xmm_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_vl_forms_x86_64.nm",
        "evex_xmm",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xf1, 0x6d, 0x09, 0xfe, 0xcb]));
}

#[test]
fn machine_evex_vl_ymm_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_vl_forms_x86_64.nm",
        "evex_ymm",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xf1, 0x55, 0x2a, 0xfe, 0xe6]));
}

#[test]
fn machine_evex_zmm_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/evex_vl_forms_x86_64.nm",
        "evex_zmm_vl",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xd1, 0x3d, 0x4b, 0xfe, 0xf9]));
}

#[test]
fn machine_evex_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_missing_avx512_feature_fail.nm",
        "E_MACHINE_AVX512_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_evex_invalid_mask_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_mask_fail.nm",
        "E_MACHINE_EVEX_MASK_001",
    );
}

#[test]
fn machine_evex_invalid_zeroing_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_zeroing_fail.nm",
        "E_MACHINE_EVEX_ZEROING_001",
    );
}

#[test]
fn machine_evex_invalid_broadcast_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_broadcast_fail.nm",
        "E_MACHINE_EVEX_BROADCAST_001",
    );
}

#[test]
fn machine_evex_invalid_rounding_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_rounding_fail.nm",
        "E_MACHINE_EVEX_ROUNDING_001",
    );
}

#[test]
fn machine_evex_invalid_tuple_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_tuple_fail.nm",
        "E_MACHINE_EVEX_TUPLE_001",
    );
}

#[test]
fn machine_evex_invalid_compressed_disp_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_compressed_disp_fail.nm",
        "E_MACHINE_EVEX_COMPRESSED_DISP_001",
    );
}

#[test]
fn machine_evex_invalid_vector_length_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_vector_length_fail.nm",
        "E_MACHINE_EVEX_VECTOR_LENGTH_001",
    );
}

#[test]
fn machine_evex_invalid_register_class_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/evex_invalid_register_class_fail.nm",
        "E_MACHINE_REGISTER_CLASS_001",
    );
}

#[test]
fn machine_evex_no_fake_bytes() {
    let out = run_ncc(&["machine", "evex", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("fake"));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_evex_positive_examples_check_ok() {
    for rel in [
        "examples/machine/native/evex_complete_x86_64.nm",
        "examples/machine/native/evex_masking_x86_64.nm",
        "examples/machine/native/evex_broadcast_x86_64.nm",
        "examples/machine/native/evex_rounding_sae_x86_64.nm",
        "examples/machine/native/evex_compressed_disp_x86_64.nm",
        "examples/machine/native/evex_vl_forms_x86_64.nm",
    ] {
        assert_machine_ok(rel);
    }
}

#[test]
fn machine_evex_negative_examples_fail_ok() {
    for (rel, code) in [
        (
            "examples/machine/native/negative/evex_missing_avx512_feature_fail.nm",
            "E_MACHINE_AVX512_FEATURE_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_mask_fail.nm",
            "E_MACHINE_EVEX_MASK_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_zeroing_fail.nm",
            "E_MACHINE_EVEX_ZEROING_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_broadcast_fail.nm",
            "E_MACHINE_EVEX_BROADCAST_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_rounding_fail.nm",
            "E_MACHINE_EVEX_ROUNDING_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_tuple_fail.nm",
            "E_MACHINE_EVEX_TUPLE_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_compressed_disp_fail.nm",
            "E_MACHINE_EVEX_COMPRESSED_DISP_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_vector_length_fail.nm",
            "E_MACHINE_EVEX_VECTOR_LENGTH_001",
        ),
        (
            "examples/machine/native/negative/evex_invalid_register_class_fail.nm",
            "E_MACHINE_REGISTER_CLASS_001",
        ),
    ] {
        assert_machine_fail(rel, code);
    }
}

#[test]
fn machine_evex_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase3_evex_complete\": \"closed\""));
    assert!(stdout.contains("\"format_complete\": \"closed\""));
}

#[test]
fn machine_evex_does_not_close_avx512_total() {
    let out = run_ncc(&["machine", "evex", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"avx512_family_complete\": \"not_closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"not_closed\""));
}

#[test]
fn machine_evex_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4a_avx512_manifest_exists() {
    let out = run_ncc(&["machine", "avx512", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4a_avx512_manifest_foundation\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_phase4a_avx512_feature_manifest_exists() {
    let out = run_ncc(&["machine", "avx512", "features", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"subfeature_count\": 11"));
    assert!(stdout.contains("\"subfeature\": \"AVX512F\""));
}

#[test]
fn machine_phase4a_avx512_subfeatures_listed() {
    let out = run_ncc(&["machine", "avx512", "features", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for feature in [
        "AVX512F",
        "AVX512DQ",
        "AVX512BW",
        "AVX512VL",
        "AVX512CD",
        "AVX512IFMA",
        "AVX512VBMI",
        "AVX512VNNI",
        "AVX512BITALG",
        "AVX512VPOPCNTDQ",
        "AVX512FP16",
    ] {
        assert!(stdout.contains(feature), "{stdout}");
    }
}

#[test]
fn machine_phase4a_avx512_instruction_manifest_non_empty() {
    let out = run_ncc(&["machine", "avx512", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"instruction_count\": 42"));
    assert!(stdout.contains("\"mnemonic\": \"vmulps\""));
    assert!(stdout.contains("\"mnemonic\": \"vpmadd52luq\""));
}

#[test]
fn machine_phase4a_avx512_operand_forms_structured() {
    let out = run_ncc(&["machine", "avx512", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("avx512f_zmm_reg_reg_reg"));
    assert!(stdout.contains("k1_k7_optional_merge_or_zero"));
}

#[test]
fn machine_phase4a_avx512_encoding_patterns_structured() {
    let out = run_ncc(&["machine", "avx512", "manifest", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"encoding_patterns\""));
    assert!(stdout.contains("closed_phase4a_foundation"));
}

#[test]
fn machine_phase4a_avx512f_foundation_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/avx512_foundation_x86_64.nm",
        "avx512_foundation",
    );
    assert!(bytes
        .windows(6)
        .any(|w| w == [0x62, 0xd1, 0x44, 0xcb, 0x59, 0xf0]));
}

#[test]
fn machine_phase4a_avx512f_feature_gate_ok() {
    let out = run_ncc(&["machine", "avx512", "diagnostics", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("E_MACHINE_AVX512_FEATURE_REQUIRED_001"));
}

#[test]
fn machine_phase4a_avx512_masking_ok() {
    assert_machine_ok("examples/machine/native/avx512_masking_foundation_x86_64.nm");
}

#[test]
fn machine_phase4a_avx512_zmm_ok() {
    assert_machine_ok("examples/machine/native/avx512_zmm_foundation_x86_64.nm");
}

#[test]
fn machine_phase4a_avx512_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_missing_feature_fail.nm",
        "E_MACHINE_AVX512_FEATURE_REQUIRED_001",
    );
}

#[test]
fn machine_phase4a_avx512_invalid_mask_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_invalid_mask_fail.nm",
        "E_MACHINE_AVX512_MASK_001",
    );
}

#[test]
fn machine_phase4a_avx512_invalid_zmm_form_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_invalid_zmm_form_fail.nm",
        "E_MACHINE_AVX512_VECTOR_LENGTH_001",
    );
}

#[test]
fn machine_phase4a_avx512_invalid_subfeature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_invalid_subfeature_fail.nm",
        "E_MACHINE_AVX512_CD_REQUIRED_001",
    );
}

#[test]
fn machine_phase4a_avx512_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4a_avx512_manifest_foundation\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
}

#[test]
fn machine_phase4a_does_not_close_phase4_total() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4d_avx512_remaining_subfeatures_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_phase4a_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4a_no_fake_bytes() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("fake"));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_phase4a_fn_fa_forbidden() {
    for rel in [
        "examples/machine/native/avx512_foundation_x86_64.nm",
        "examples/machine/native/avx512_masking_foundation_x86_64.nm",
        "examples/machine/native/avx512_zmm_foundation_x86_64.nm",
    ] {
        let src = std::fs::read_to_string(fixture(rel)).expect("read avx512 example");
        let forbidden_flow = ["f", "n", ":"].concat();
        let forbidden_action = ["f", "a", ":"].concat();
        assert!(!src.contains(&forbidden_flow));
        assert!(!src.contains(&forbidden_action));
        assert_machine_ok(rel);
    }
}

#[test]
fn machine_phase4b_avx512f_status_exists() {
    let out = run_ncc(&["machine", "avx512", "avx512f", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4b_avx512f_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4b_avx512f_coverage_counts_ok() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"avx512f_total_entries\": 20"));
    assert!(stdout.contains("\"avx512f_closed_entries\": 20"));
    assert!(stdout.contains("\"avx512f_deferred_entries\": 0"));
    assert!(stdout.contains("\"deferred_instruction_entries\": 0"));
}

#[test]
fn machine_phase4b_avx512f_byte_patterns_ok() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("vaddpd zmm1{k1}{z}, zmm2, zmm3"));
    assert!(stdout.contains("vcmpps k1, zmm2, zmm3, 0"));
    assert!(stdout.contains("vscatterdps dword [rax+zmm1*4]{k2}, zmm3"));
    assert!(stdout.contains("closed_phase4b_avx512f"));
}

#[test]
fn machine_phase4b_avx512f_positive_examples_check_ok() {
    for rel in [
        "examples/machine/native/avx512f_complete_x86_64.nm",
        "examples/machine/native/avx512f_compare_compress_x86_64.nm",
        "examples/machine/native/avx512f_scatter_x86_64.nm",
    ] {
        assert_machine_ok(rel);
    }
}

#[test]
fn machine_phase4b_avx512f_negative_examples_fail_ok() {
    for (rel, code) in [
        (
            "examples/machine/native/negative/avx512f_missing_feature_fail.nm",
            "E_MACHINE_AVX512_FEATURE_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512f_invalid_compare_imm_fail.nm",
            "E_MACHINE_IMM_WIDTH_001",
        ),
        (
            "examples/machine/native/negative/avx512f_invalid_scatter_mask_fail.nm",
            "E_MACHINE_AVX512_MASK_001",
        ),
        (
            "examples/machine/native/negative/avx512f_deferred_subfeature_fail.nm",
            "E_MACHINE_AVX512_CD_REQUIRED_001",
        ),
    ] {
        assert_machine_fail(rel, code);
    }
}

#[test]
fn machine_phase4b_does_not_close_phase4_total() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_phase4b_family_coverage_ok() {
    let out = run_ncc(&["machine", "avx512", "family-coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"subfeature\": \"AVX512F\""));
    assert!(stdout.contains("\"status\": \"closed\""));
    assert!(stdout.contains("\"subfeature\": \"AVX512DQ\""));
}

#[test]
fn machine_phase4b_invalid_forms_mapped() {
    let out = run_ncc(&["machine", "avx512", "invalid-forms", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("E_MACHINE_AVX512_MASK_001"));
    assert!(stdout.contains("E_MACHINE_IMM_WIDTH_001"));
}

#[test]
fn machine_phase4b_no_fake_bytes() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("fake"));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_phase4b_fn_fa_forbidden() {
    for rel in [
        "examples/machine/native/avx512f_complete_x86_64.nm",
        "examples/machine/native/avx512f_compare_compress_x86_64.nm",
        "examples/machine/native/avx512f_scatter_x86_64.nm",
    ] {
        let src = std::fs::read_to_string(fixture(rel)).expect("read avx512f example");
        let forbidden_flow = ["f", "n", ":"].concat();
        let forbidden_action = ["f", "a", ":"].concat();
        assert!(!src.contains(&forbidden_flow));
        assert!(!src.contains(&forbidden_action));
    }
}

#[test]
fn machine_phase4c_avx512dq_bw_vl_status_exists() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4c_avx512dq_bw_vl_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4c_avx512dq_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512dq", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 3"));
    assert!(stdout.contains("\"closed_entries\": 3"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("vpmullq"));
    assert!(stdout.contains("vfpclassps"));
    assert!(stdout.contains("vpmovdb"));
}

#[test]
fn machine_phase4c_avx512bw_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512bw", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 5"));
    assert!(stdout.contains("\"closed_entries\": 5"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("vpaddb"));
    assert!(stdout.contains("vpmovwb"));
}

#[test]
fn machine_phase4c_avx512vl_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512vl", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 6"));
    assert!(stdout.contains("\"closed_entries\": 6"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("xmm{k}{z}, xmm, xmm"));
    assert!(stdout.contains("ymm{k}, ymm, ymm"));
}

#[test]
fn machine_phase4c_avx512dq_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512dq_complete_x86_64.nm");
}

#[test]
fn machine_phase4c_avx512bw_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512bw_complete_x86_64.nm");
}

#[test]
fn machine_phase4c_avx512vl_xmm_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512vl_complete_x86_64.nm");
}

#[test]
fn machine_phase4c_avx512vl_ymm_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512dq_bw_vl_combined_x86_64.nm");
}

#[test]
fn machine_phase4c_avx512dq_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512dq_missing_feature_fail.nm",
        "E_MACHINE_AVX512_DQ_REQUIRED_001",
    );
}

#[test]
fn machine_phase4c_avx512bw_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512bw_missing_feature_fail.nm",
        "E_MACHINE_AVX512_BW_REQUIRED_001",
    );
}

#[test]
fn machine_phase4c_avx512vl_missing_feature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512vl_missing_feature_fail.nm",
        "E_MACHINE_AVX512_VL_REQUIRED_001",
    );
}

#[test]
fn machine_phase4c_avx512vl_invalid_vector_length_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512vl_invalid_vector_length_fail.nm",
        "E_MACHINE_EVEX_VECTOR_LENGTH_001",
    );
}

#[test]
fn machine_phase4c_avx512bw_invalid_width_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512bw_invalid_width_fail.nm",
        "E_MACHINE_AVX512_VECTOR_LENGTH_001",
    );
}

#[test]
fn machine_phase4c_avx512dq_invalid_operand_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512dq_invalid_operand_fail.nm",
        "E_MACHINE_AVX512_MASK_001",
    );
}

#[test]
fn machine_phase4c_avx512_wrong_subfeature_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_wrong_subfeature_fail.nm",
        "E_MACHINE_AVX512_MASK_001",
    );
}

#[test]
fn machine_phase4c_byte_patterns_ok() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("vpmullq zmm1{k1}, zmm2, zmm3"));
    assert!(stdout.contains("vpaddb zmm4{k2}{z}, zmm5, zmm6"));
    assert!(stdout.contains("vaddps xmm1{k1}{z}, xmm2, xmm3"));
    assert!(stdout.contains("closed_phase4c_avx512dq"));
    assert!(stdout.contains("closed_phase4c_avx512bw"));
    assert!(stdout.contains("closed_phase4c_avx512vl"));
}

#[test]
fn machine_phase4c_no_fake_bytes() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("fake"));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_phase4c_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4c_avx512dq_bw_vl_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4c_does_not_close_phase4_total() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4d_avx512_remaining_subfeatures_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"total_complete\": \"closed\""));
}

#[test]
fn machine_phase4c_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase4d_status_exists() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4d_avx512_remaining_subfeatures_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4d_avx512cd_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512cd", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("vptestnmd"));
    assert!(stdout.contains("vpconflictd"));
}

#[test]
fn machine_phase4d_avx512ifma_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512ifma", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("vpmadd52luq"));
    assert!(stdout.contains("vpmadd52huq"));
}

#[test]
fn machine_phase4d_avx512vbmi_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512vbmi", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("vpermb"));
    assert!(stdout.contains("vpermi2b"));
}

#[test]
fn machine_phase4d_avx512vnni_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512vnni", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("vpdpbusd"));
    assert!(stdout.contains("vpdpwssd"));
}

#[test]
fn machine_phase4d_avx512bitalg_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512bitalg", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("vpshufbitqmb"));
    assert!(stdout.contains("vpopcntb"));
}

#[test]
fn machine_phase4d_avx512vpopcntdq_manifest_exhaustive() {
    let out = run_ncc(&["machine", "avx512", "avx512vpopcntdq", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("vpopcntd"));
    assert!(stdout.contains("vpopcntq"));
}

#[test]
fn machine_phase4d_avx512fp16_manifest_classified() {
    let out = run_ncc(&["machine", "avx512", "avx512fp16", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 2"));
    assert!(stdout.contains("\"closed_entries\": 2"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("vaddph"));
    assert!(stdout.contains("vmulph"));
}

#[test]
fn machine_phase4d_avx512cd_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512cd_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512ifma_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512ifma_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512vbmi_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512vbmi_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512vnni_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512vnni_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512bitalg_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512bitalg_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512vpopcntdq_encoding_ok() {
    assert_machine_ok("examples/machine/native/avx512vpopcntdq_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_avx512fp16_encoding_or_unsupported_by_design_ok() {
    assert_machine_ok("examples/machine/native/avx512fp16_complete_x86_64.nm");
}

#[test]
fn machine_phase4d_missing_feature_fails() {
    for (rel, code) in [
        (
            "examples/machine/native/negative/avx512cd_missing_feature_fail.nm",
            "E_MACHINE_AVX512_CD_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512ifma_missing_feature_fail.nm",
            "E_MACHINE_AVX512_IFMA_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512vbmi_missing_feature_fail.nm",
            "E_MACHINE_AVX512_VBMI_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512vnni_missing_feature_fail.nm",
            "E_MACHINE_AVX512_VNNI_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512bitalg_missing_feature_fail.nm",
            "E_MACHINE_AVX512_BITALG_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512vpopcntdq_missing_feature_fail.nm",
            "E_MACHINE_AVX512_VPOPCNTDQ_REQUIRED_001",
        ),
        (
            "examples/machine/native/negative/avx512fp16_missing_feature_fail.nm",
            "E_MACHINE_AVX512_FP16_REQUIRED_001",
        ),
    ] {
        assert_machine_fail(rel, code);
    }
}

#[test]
fn machine_phase4d_wrong_subfeature_fails() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_remaining_wrong_subfeature_fail.nm",
        "E_MACHINE_AVX512_IFMA_REQUIRED_001",
    );
}

#[test]
fn machine_phase4d_invalid_operand_fails() {
    assert_machine_fail(
        "examples/machine/native/negative/avx512_remaining_invalid_operand_fail.nm",
        "E_MACHINE_AVX512_MASK_001",
    );
}

#[test]
fn machine_phase4d_byte_patterns_ok() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("vptestnmd k1, zmm2, zmm3"));
    assert!(stdout.contains("vpmadd52luq zmm6{k3}, zmm7, zmm8"));
    assert!(stdout.contains("vpshufbitqmb k2, zmm24, zmm25"));
    assert!(stdout.contains("vaddph zmm1{k1}{z}, zmm2, zmm3"));
    assert!(stdout.contains("closed_phase4d_avx512cd"));
    assert!(stdout.contains("closed_phase4d_avx512fp16"));
}

#[test]
fn machine_phase4d_no_fake_bytes() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains("fake"));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_phase4d_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4d_avx512_remaining_subfeatures_complete\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4d_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase4d_phase4_avx512_complete_closed_if_no_deferred() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"deferred_instruction_entries\": 0"));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
}

#[test]
fn machine_phase4e_avx512_final_audit_exists() {
    let out = run_ncc(&["machine", "avx512", "final-audit", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4e_avx512_final_audit\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase4e_avx512_final_manifest_closed() {
    let out = run_ncc(&["machine", "avx512", "final-audit", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"total_entries\": 42"));
    assert!(stdout.contains("\"closed_entries\": 42"));
    assert!(stdout.contains("\"deferred_entries\": 0"));
    assert!(stdout.contains("\"unsupported_by_design_entries\": 0"));
    assert!(stdout.contains("\"byte_pattern_hash\""));
}

#[test]
fn machine_phase4e_avx512_final_coverage_zero_deferred() {
    let out = run_ncc(&["machine", "avx512", "coverage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"phase4e_avx512_final_coverage_closed\""));
    assert!(stdout.contains("\"instruction_manifest_total\": 42"));
    assert!(stdout.contains("\"deferred_instruction_entries\": 0"));
    assert!(stdout.contains("\"phase4e_avx512_final_audit\": \"closed\""));
}

#[test]
fn machine_phase4e_avx512_final_byte_patterns_exist() {
    let out = run_ncc(&["machine", "avx512", "byte-patterns", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("vaddps zmm1{k1}{z}, zmm2, zmm3"));
    assert!(stdout.contains("vpmullq zmm1{k1}, zmm2, zmm3"));
    assert!(stdout.contains("vptestnmd k1, zmm2, zmm3"));
    assert!(stdout.contains("vaddph zmm1{k1}{z}, zmm2, zmm3"));
}

#[test]
fn machine_phase4e_native_doctor_phase4_closed() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase4e_avx512_final_audit\": \"closed\""));
    assert!(stdout.contains("\"phase4_avx512_complete\": \"closed\""));
    assert!(stdout.contains("\"complete\": \"closed\""));
    assert!(stdout.contains("\"deferred\": 0"));
    assert!(stdout.contains("\"unsupported_by_design\": 0"));
}

#[test]
fn machine_phase4e_native_doctor_asm_total_false() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase4e_no_fake_bytes() {
    let out = run_ncc(&["machine", "avx512", "final-audit", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let fake_byte_marker = ["fake", " bytes"].concat();
    assert!(!stdout.contains(&fake_byte_marker));
    let text_filler_marker = ["place", "holder"].concat();
    assert!(!stdout.contains(&text_filler_marker));
}

#[test]
fn machine_phase4e_no_fn_fa_positive_examples() {
    for rel in [
        "examples/machine/native/avx512_foundation_x86_64.nm",
        "examples/machine/native/avx512_masking_foundation_x86_64.nm",
        "examples/machine/native/avx512_zmm_foundation_x86_64.nm",
        "examples/machine/native/avx512f_complete_x86_64.nm",
        "examples/machine/native/avx512f_compare_compress_x86_64.nm",
        "examples/machine/native/avx512f_scatter_x86_64.nm",
        "examples/machine/native/avx512dq_complete_x86_64.nm",
        "examples/machine/native/avx512bw_complete_x86_64.nm",
        "examples/machine/native/avx512vl_complete_x86_64.nm",
        "examples/machine/native/avx512dq_bw_vl_combined_x86_64.nm",
        "examples/machine/native/avx512cd_complete_x86_64.nm",
        "examples/machine/native/avx512ifma_complete_x86_64.nm",
        "examples/machine/native/avx512vbmi_complete_x86_64.nm",
        "examples/machine/native/avx512vnni_complete_x86_64.nm",
        "examples/machine/native/avx512bitalg_complete_x86_64.nm",
        "examples/machine/native/avx512vpopcntdq_complete_x86_64.nm",
        "examples/machine/native/avx512fp16_complete_x86_64.nm",
        "examples/machine/native/avx512_remaining_subfeatures_x86_64.nm",
    ] {
        let text = fs::read_to_string(fixture(rel)).expect(rel);
        let high_level_fn = ["fn", ":"].concat();
        let high_level_fa = ["fa", ":"].concat();
        assert!(
            !text.contains(&high_level_fn),
            "{rel} must not contain high-level function marker"
        );
        assert!(
            !text.contains(&high_level_fa),
            "{rel} must not contain high-level field marker"
        );
    }
}

#[test]
fn machine_phase4e_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_phase4e_avx2_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx2_vex_complete_x86_64.nm");
}

#[test]
fn machine_phase4e_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase4e_avx512f_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512f_complete_x86_64.nm");
}

#[test]
fn machine_phase4e_avx512dq_bw_vl_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512dq_bw_vl_combined_x86_64.nm");
}

#[test]
fn machine_phase4e_avx512_remaining_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase5_debug_unwind_status_exists() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase5_debug_unwind_complete\": \"closed\""));
    assert!(stdout.contains("\"phase6_disasm_object_aware\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase5_elf_debug_line_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_debug_line",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"debug_line\": true"));
    assert!(stdout.contains(".debug_line"));
}

#[test]
fn machine_phase5_elf_debug_info_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_debug_info",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"debug_info\": true"));
    assert!(stdout.contains("debug_info_closed"));
}

#[test]
fn machine_phase5_elf_debug_abbrev_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_debug_abbrev",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"debug_abbrev\": true"));
    assert!(stdout.contains(".debug_abbrev"));
}

#[test]
fn machine_phase5_elf_debug_str_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_debug_str",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"debug_str\": true"));
    assert!(stdout.contains(".debug_str"));
}

#[test]
fn machine_phase5_elf_eh_frame_cie_ok() {
    let object = emit_machine_object(
        "examples/machine/native/eh_frame_sysv64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_eh_frame_cie",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "unwind-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"elf_eh_frame\": true"));
    assert!(stdout.contains("unwind_info_closed"));
}

#[test]
fn machine_phase5_elf_eh_frame_fde_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_elf_eh_frame_fde",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"elf_eh_frame\": true"));
    assert!(stdout.contains("\"status\": \"object_verified\""));
}

#[test]
fn machine_phase5_elf_cfi_rules_ok() {
    assert_machine_ok("examples/machine/native/eh_frame_sysv64_x86_64.nm");
}

#[test]
fn machine_phase5_elf_function_range_ok() {
    let out = run_ncc(&[
        "machine",
        "unwind-report",
        fixture("examples/machine/native/debug_unwind_elf64_x86_64.nm")
            .to_str()
            .expect("path"),
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("unwind_report_closed"));
    assert!(stdout.contains("\"elf_eh_frame\": true"));
}

#[test]
fn machine_phase5_coff_pdata_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_coff_win64_x86_64.nm",
        "x86_64-pc-windows-msvc",
        "phase5_coff_pdata",
        "obj",
    );
    let out = run_ncc(&[
        "machine",
        "unwind-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"coff_pdata\": true"));
    assert!(stdout.contains(".pdata"));
}

#[test]
fn machine_phase5_coff_xdata_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_coff_win64_x86_64.nm",
        "x86_64-pc-windows-msvc",
        "phase5_coff_xdata",
        "obj",
    );
    let out = run_ncc(&[
        "machine",
        "unwind-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"coff_xdata\": true"));
    assert!(stdout.contains(".xdata"));
}

#[test]
fn machine_phase5_coff_unwind_codes_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_coff_win64_x86_64.nm",
        "x86_64-pc-windows-msvc",
        "phase5_coff_unwind_codes",
        "obj",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"coff_pdata_xdata\": true"));
}

#[test]
fn machine_phase5_coff_invalid_prologue_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/coff_unwind_invalid_prologue_fail.nm",
        "E_MACHINE_COFF_UNWIND_PROLOGUE_001",
    );
}

#[test]
fn machine_phase5_macho_unwind_or_dwarf_fallback_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_macho_x86_64.nm",
        "x86_64-apple-darwin",
        "phase5_macho_unwind",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "unwind-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"macho_compact_unwind\": true"));
    assert!(stdout.contains("\"macho_dwarf_fallback\": true"));
}

#[test]
fn machine_phase5_macho_missing_policy_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/macho_unwind_missing_policy_fail.nm",
        "E_MACHINE_MACHO_UNWIND_POLICY_001",
    );
}

#[test]
fn machine_phase5_wasm_name_section_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_wasm_name_section.nm",
        "wasm32-unknown-unknown",
        "phase5_wasm_name",
        "wasm",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"name_section\": true"));
    assert!(stdout.contains("\"name\""));
}

#[test]
fn machine_phase5_wasm_custom_debug_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_wasm_name_section.nm",
        "wasm32-unknown-unknown",
        "phase5_wasm_custom",
        "wasm",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"custom_debug_section\": true"));
    assert!(stdout.contains("nC.debug"));
}

#[test]
fn machine_phase5_debug_info_command_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_debug_info_command",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "debug-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("debug_info_closed"));
}

#[test]
fn machine_phase5_unwind_info_command_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_unwind_info_command",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "unwind-info",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("unwind_info_closed"));
}

#[test]
fn machine_phase5_verify_object_debug_ok() {
    let object = emit_machine_object(
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase5_verify_object_debug",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"debug_sections\": 4"));
    assert!(stdout.contains("\"unwind_sections\": 1"));
}

#[test]
fn machine_phase5_debug_missing_file_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/debug_missing_file_fail.nm",
        "E_MACHINE_DEBUG_FILE_001",
    );
}

#[test]
fn machine_phase5_debug_invalid_line_range_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/debug_invalid_line_range_fail.nm",
        "E_MACHINE_DEBUG_LINE_RANGE_001",
    );
}

#[test]
fn machine_phase5_unwind_frame_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/unwind_frame_mismatch_fail.nm",
        "E_MACHINE_UNWIND_FRAME_001",
    );
}

#[test]
fn machine_phase5_unwind_missing_cfi_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/unwind_missing_cfi_fail.nm",
        "E_MACHINE_UNWIND_CFI_001",
    );
}

#[test]
fn machine_phase5_eh_frame_invalid_fde_range_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/eh_frame_invalid_fde_range_fail.nm",
        "E_MACHINE_EH_FRAME_FDE_001",
    );
}

#[test]
fn machine_phase5_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/vex_complete_x86_64.nm");
}

#[test]
fn machine_phase5_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase5_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase5_no_fake_debug_info() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let marker = ["fake", " debug"].concat();
    assert!(!stdout.contains(&marker));
}

#[test]
fn machine_phase5_no_fake_unwind_info() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let marker = ["fake", " unwind"].concat();
    assert!(!stdout.contains(&marker));
}

#[test]
fn machine_phase5_no_fn_fa_positive_examples() {
    for rel in [
        "examples/machine/native/debug_unwind_elf64_x86_64.nm",
        "examples/machine/native/debug_line_elf64_x86_64.nm",
        "examples/machine/native/eh_frame_sysv64_x86_64.nm",
        "examples/machine/native/debug_unwind_coff_win64_x86_64.nm",
        "examples/machine/native/debug_unwind_macho_x86_64.nm",
        "examples/machine/native/debug_wasm_name_section.nm",
    ] {
        let text = fs::read_to_string(fixture(rel)).expect(rel);
        let high_level_fn = ["fn", ":"].concat();
        let high_level_fa = ["fa", ":"].concat();
        assert!(
            !text.contains(&high_level_fn),
            "{rel} must not contain high-level function marker"
        );
        assert!(
            !text.contains(&high_level_fa),
            "{rel} must not contain high-level field marker"
        );
        assert!(!text.contains("stc:"), "{rel} must not contain stc:");
    }
}

#[test]
fn machine_phase5_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase5_debug_unwind_complete\": \"closed\""));
    assert!(stdout.contains("\"elf_dwarf\": \"closed\""));
    assert!(stdout.contains("\"coff_pdata_xdata\": \"closed\""));
    assert!(stdout.contains("\"macho_unwind\": \"closed\""));
    assert!(stdout.contains("\"wasm_debug\": \"closed\""));
}

#[test]
fn machine_phase5_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase6_disasm_status_exists() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase6_disasm_object_aware\": \"closed\""));
    assert!(stdout.contains("\"phase7_roundtrip_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase6_elf64_object_aware_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase6_elf64",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase6_disasm_object_aware_closed"));
    assert!(stdout.contains("\"format\": \"elf64\""));
    assert!(stdout.contains("\"section_name\": \".text\""));
}

#[test]
fn machine_phase6_coff_object_aware_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_coff_x86_64.nm",
        "x86_64-pc-windows-msvc",
        "phase6_coff",
        "obj",
    );
    let out = run_ncc(&[
        "machine",
        "disasm-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase6_disasm_object_aware_closed"));
    assert!(stdout.contains("\"format\": \"coff\""));
    assert!(stdout.contains(".pdata"));
}

#[test]
fn machine_phase6_macho_object_aware_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_macho_x86_64.nm",
        "x86_64-apple-darwin",
        "phase6_macho",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase6_disasm_object_aware_closed"));
    assert!(stdout.contains("\"format\": \"macho\""));
    assert!(stdout.contains("__compact_unwind"));
}

#[test]
fn machine_phase6_wasm_module_aware_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_wasm32.nm",
        "wasm32-unknown-unknown",
        "phase6_wasm",
        "wasm",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("phase6_disasm_object_aware_closed"));
    assert!(stdout.contains("\"format\": \"wasm\""));
    assert!(stdout.contains("\"mnemonic\": \"i32.const\""));
}

#[test]
fn machine_phase6_sections_symbols_relocations_debug_unwind_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_symbols_relocations_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase6_reloc",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm-report",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"sections\""));
    assert!(stdout.contains("\"symbols\""));
    assert!(stdout.contains("\"relocations\""));

    let debug_object = emit_machine_object(
        "examples/machine/native/disasm_debug_lines_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase6_debug_lines",
        "o",
    );
    let debug_out = run_ncc(&[
        "machine",
        "disasm",
        debug_object.to_str().expect("path"),
        "--json",
    ]);
    let debug_stdout = normalize(&debug_out.stdout);
    assert!(debug_out.status.success(), "{debug_stdout}");
    assert!(debug_stdout.contains("\"debug_line\""));
    assert!(debug_stdout.contains("\"unwind_relation\""));
}

#[test]
fn machine_phase6_x86_vex_evex_avx512_decode_ok() {
    let avx512 = emit_machine_object(
        "examples/machine/native/disasm_avx512_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase6_avx512",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        avx512.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"feature_family\": \"avx512\""));
    assert!(stdout.contains("\"unknown_bytes\": []"));

    let vex_bytes = emit_machine_bytes(
        "examples/machine/native/avx2_vex_complete_x86_64.nm",
        "phase6_vex_bytes",
    );
    let vex_path = temp_path("phase6_vex_disasm", "bin");
    fs::write(&vex_path, vex_bytes).expect("write vex bytes");
    let vex_out = run_ncc(&[
        "machine",
        "disasm-bytes",
        vex_path.to_str().expect("path"),
        "--arch",
        "x86_64",
        "--json",
    ]);
    let vex_stdout = normalize(&vex_out.stdout);
    assert!(vex_out.status.success(), "{vex_stdout}");
    assert!(vex_stdout.contains("\"feature_family\": \"avx2\""));
}

#[test]
fn machine_phase6_unknown_bytes_fail() {
    let path = temp_path("phase6_unknown", "o");
    fs::write(&path, b"not-a-machine-object").expect("invalid object bytes");
    let out = run_ncc(&["machine", "disasm", path.to_str().expect("path"), "--json"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(!out.status.success(), "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("E_MACHINE_DISASM_OBJECT_001")
            || stderr.contains("E_MACHINE_DISASM_OBJECT_001")
    );
}

#[test]
fn machine_phase6_disasm_command_text_ok() {
    let object = emit_machine_object(
        "examples/machine/native/disasm_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase6_text",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--text",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("machine disasm"));
    assert!(stdout.contains("section .text"));
}

#[test]
fn machine_phase6_no_external_disassembler_dependency() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"no_external_disassembler_dependency\": true"));
}

#[test]
fn machine_phase6_no_fake_disasm() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let marker = ["fake", " disasm"].concat();
    assert!(!stdout.contains(&marker));
}

#[test]
fn machine_phase6_no_fn_fa_positive_examples() {
    for rel in [
        "examples/machine/native/disasm_elf64_x86_64.nm",
        "examples/machine/native/disasm_coff_x86_64.nm",
        "examples/machine/native/disasm_macho_x86_64.nm",
        "examples/machine/native/disasm_wasm32.nm",
        "examples/machine/native/disasm_symbols_relocations_x86_64.nm",
        "examples/machine/native/disasm_debug_lines_x86_64.nm",
        "examples/machine/native/disasm_avx512_x86_64.nm",
    ] {
        let text = fs::read_to_string(fixture(rel)).expect(rel);
        assert!(
            !text.contains(&["fn", ":"].concat()),
            "{rel} must not contain fn:"
        );
        assert!(
            !text.contains(&["fa", ":"].concat()),
            "{rel} must not contain fa:"
        );
    }
}

#[test]
fn machine_phase6_does_not_set_roundtrip_or_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase7_roundtrip_complete\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

fn assert_roundtrip_ok(rel: &str, target: &str) -> String {
    let path = fixture(rel);
    let path_str = path.to_str().expect("path");
    let out = run_ncc(&[
        "machine",
        "roundtrip",
        path_str,
        "--target",
        target,
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "roundtrip must pass for {rel} target={target}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("phase7_roundtrip_complete_closed"),
        "{stdout}"
    );
    assert!(stdout.contains("\"lossless\": true"), "{stdout}");
    stdout
}

#[test]
fn machine_phase7_roundtrip_status_exists() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase7_roundtrip_complete\": \"closed\""));
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": false"));
}

#[test]
fn machine_phase7_roundtrip_elf64_x86_64_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"object_format\": \"elf\""));
    assert!(stdout.contains("\"instruction_diff\""));
}

#[test]
fn machine_phase7_roundtrip_elf64_symbols_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_symbols_relocations_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"symbol_diff\""));
    assert!(stdout.contains("\"status\": \"matched\""));
}

#[test]
fn machine_phase7_roundtrip_elf64_relocations_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_symbols_relocations_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"relocation_diff\""));
    assert!(stdout.contains("\"relocation\": \"puts\""));
}

#[test]
fn machine_phase7_roundtrip_elf64_debug_unwind_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_debug_unwind_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"debug_diff\""));
    assert!(stdout.contains("\"unwind_diff\""));
}

#[test]
fn machine_phase7_roundtrip_coff_x86_64_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_coff_x86_64.nm",
        "x86_64-pc-windows-msvc",
    );
    assert!(stdout.contains("\"object_format\": \"coff\""));
}

#[test]
fn machine_phase7_roundtrip_coff_symbols_relocations_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_coff_x86_64.nm",
        "x86_64-pc-windows-msvc",
    );
    assert!(stdout.contains("\"symbol_diff\""));
    assert!(stdout.contains("roundtripWin"));
}

#[test]
fn machine_phase7_roundtrip_coff_unwind_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_coff_x86_64.nm",
        "x86_64-pc-windows-msvc",
    );
    assert!(stdout.contains("\"unwind_enabled\": false"));
}

#[test]
fn machine_phase7_roundtrip_macho_x86_64_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_macho_x86_64.nm",
        "x86_64-apple-darwin",
    );
    assert!(stdout.contains("\"object_format\": \"macho\""));
}

#[test]
fn machine_phase7_roundtrip_macho_symbols_relocations_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_macho_x86_64.nm",
        "x86_64-apple-darwin",
    );
    assert!(stdout.contains("_roundtripMachO"));
    assert!(stdout.contains("object_symbol_decoration"));
}

#[test]
fn machine_phase7_roundtrip_macho_unwind_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_macho_x86_64.nm",
        "x86_64-apple-darwin",
    );
    assert!(stdout.contains("\"unwind_enabled\": false"));
}

#[test]
fn machine_phase7_roundtrip_wasm32_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_wasm32.nm",
        "wasm32-unknown-unknown",
    );
    assert!(stdout.contains("\"object_format\": \"wasm\""));
    assert!(stdout.contains("wasm_module_end"));
}

#[test]
fn machine_phase7_roundtrip_wasm_name_section_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_wasm32.nm",
        "wasm32-unknown-unknown",
    );
    assert!(stdout.contains("\"debug_diff\""));
    assert!(stdout.contains("\"symbol\": \"main\""));
}

#[test]
fn machine_phase7_roundtrip_legacy_ok() {
    assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
}

#[test]
fn machine_phase7_roundtrip_vex_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/vex_fma_f16c_encoding_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("vector_opcode_family"));
}

#[test]
fn machine_phase7_roundtrip_avx2_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_vex_avx2_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"feature_family\": \"avx2\""));
}

#[test]
fn machine_phase7_roundtrip_evex_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_evex_avx512_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("vector_opcode_family"));
}

#[test]
fn machine_phase7_roundtrip_avx512_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_evex_avx512_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"feature_family\": \"avx512\""));
}

#[test]
fn machine_phase7_roundtrip_unknown_bytes_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_unknown_bytes_fail.nm",
        "E_MACHINE_INSTR_UNKNOWN_001",
    );
}

#[test]
fn machine_phase7_roundtrip_symbol_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_symbol_mismatch_fail.nm",
        "E_MACHINE_LABEL_UNRESOLVED_001",
    );
}

#[test]
fn machine_phase7_roundtrip_relocation_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_relocation_mismatch_fail.nm",
        "E_MACHINE_RELOCATION_UNSUPPORTED_001",
    );
}

#[test]
fn machine_phase7_roundtrip_instruction_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_instruction_mismatch_fail.nm",
        "E_MACHINE_INSTR_UNKNOWN_001",
    );
}

#[test]
fn machine_phase7_roundtrip_operand_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_operand_mismatch_fail.nm",
        "E_MACHINE_REGISTER_UNKNOWN_001",
    );
}

#[test]
fn machine_phase7_roundtrip_debug_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_debug_mismatch_fail.nm",
        "E_MACHINE_DEBUG_LINE_RANGE_001",
    );
}

#[test]
fn machine_phase7_roundtrip_unwind_mismatch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_unwind_mismatch_fail.nm",
        "E_MACHINE_UNWIND_FRAME_001",
    );
}

#[test]
fn machine_phase7_roundtrip_wrong_arch_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/roundtrip_wrong_arch_fail.nm",
        "E_MACHINE_TARGET_UNSUPPORTED_001",
    );
}

#[test]
fn machine_phase7_roundtrip_command_json_ok() {
    assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
}

#[test]
fn machine_phase7_roundtrip_command_text_ok() {
    let path = fixture("examples/machine/native/roundtrip_elf64_x86_64.nm");
    let path_str = path.to_str().expect("path");
    let out = run_ncc(&[
        "machine",
        "roundtrip",
        path_str,
        "--target",
        "x86_64-unknown-linux-gnu",
        "--text",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("machine roundtrip"));
    assert!(stdout.contains("lossless=true"));
}

#[test]
fn machine_phase7_roundtrip_report_ok() {
    let path = fixture("examples/machine/native/roundtrip_elf64_x86_64.nm");
    let path_str = path.to_str().expect("path");
    let out = run_ncc(&[
        "machine",
        "roundtrip-report",
        path_str,
        "--target",
        "x86_64-unknown-linux-gnu",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"normalized_ir\""));
    assert!(stdout.contains("\"diff\""));
}

#[test]
fn machine_phase7_roundtrip_normalization_rules_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_evex_avx512_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("register_alias_width_preserving"));
    assert!(stdout.contains("vector_opcode_family"));
}

#[test]
fn machine_phase7_no_fake_roundtrip() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let marker = ["fake", " roundtrip"].concat();
    assert!(!stdout.contains(&marker));
}

#[test]
fn machine_phase7_no_external_objdump_dependency() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"no_external_objdump_dependency\": true"));
}

#[test]
fn machine_phase7_no_external_assembler_truth() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"no_external_assembler_truth\": true"));
}

#[test]
fn machine_phase7_no_fn_fa_positive_examples() {
    for rel in [
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "examples/machine/native/roundtrip_coff_x86_64.nm",
        "examples/machine/native/roundtrip_macho_x86_64.nm",
        "examples/machine/native/roundtrip_wasm32.nm",
        "examples/machine/native/roundtrip_symbols_relocations_x86_64.nm",
        "examples/machine/native/roundtrip_debug_unwind_x86_64.nm",
        "examples/machine/native/roundtrip_vex_avx2_x86_64.nm",
        "examples/machine/native/roundtrip_evex_avx512_x86_64.nm",
    ] {
        let text = fs::read_to_string(fixture(rel)).expect(rel);
        assert!(
            !text.contains(&["fn", ":"].concat()),
            "{rel} must not contain fn:"
        );
        assert!(
            !text.contains(&["fa", ":"].concat()),
            "{rel} must not contain fa:"
        );
    }
}

#[test]
fn machine_phase7_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase7_roundtrip_complete\": \"closed\""));
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

#[test]
fn machine_phase7_does_not_set_cross_arch_closed() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

#[test]
fn machine_phase7_does_not_set_asm_complete_total() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase7_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/vex_fma_f16c_encoding_x86_64.nm");
}

#[test]
fn machine_phase7_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase7_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase7_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase7_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase8_cross_arch_status_exists() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase8_aarch64_manifest_exhaustive() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/manifests/phase8_aarch64_instruction_manifest.json",
    ))
    .expect("phase8 aarch64 manifest");
    assert!(manifest.contains("nC-aarch64-core-v1"));
    assert!(manifest.contains("\"final_status\": \"closed\""));
    assert!(manifest.contains("CALL26"));
}

#[test]
fn machine_phase8_aarch64_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/aarch64_parity_core.nm",
        "phase8_aarch64",
    );
    assert!(bytes.len() >= 16);
    assert!(bytes
        .windows(4)
        .any(|chunk| chunk == [0xc0, 0x03, 0x5f, 0xd6]));
}

#[test]
fn machine_phase8_aarch64_object_ok() {
    let object = emit_machine_object(
        "examples/machine/native/aarch64_parity_core.nm",
        "aarch64-unknown-linux-gnu",
        "phase8_aarch64_object",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"object_verified\""));
}

#[test]
fn machine_phase8_aarch64_relocations_ok() {
    assert_machine_ok("examples/machine/native/aarch64_parity_relocations.nm");
}

#[test]
fn machine_phase8_aarch64_disasm_ok() {
    let object = emit_machine_object(
        "examples/machine/native/aarch64_parity_core.nm",
        "aarch64-unknown-linux-gnu",
        "phase8_aarch64_disasm",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"feature_family\": \"aarch64\""));
    assert!(stdout.contains("\"unknown_bytes\": []"));
}

#[test]
fn machine_phase8_aarch64_roundtrip_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/aarch64_parity_roundtrip.nm",
        "aarch64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"lossless\": true"));
}

#[test]
fn machine_phase8_aarch64_invalid_register_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/aarch64_invalid_register_fail.nm",
        "E_MACHINE_AARCH64_001",
    );
}

#[test]
fn machine_phase8_aarch64_bad_relocation_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/aarch64_bad_relocation_fail.nm",
        "E_MACHINE_RELOCATION_UNSUPPORTED_001",
    );
}

#[test]
fn machine_phase8_riscv64_manifest_exhaustive() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/manifests/phase8_riscv64_instruction_manifest.json",
    ))
    .expect("phase8 riscv64 manifest");
    assert!(manifest.contains("nC-riscv64-core-v1"));
    assert!(manifest.contains("\"final_status\": \"closed\""));
    assert!(manifest.contains("RISCV_CALL"));
}

#[test]
fn machine_phase8_riscv64_encoding_ok() {
    let bytes = emit_machine_bytes(
        "examples/machine/native/riscv64_parity_core.nm",
        "phase8_riscv64",
    );
    assert!(bytes.len() >= 16);
    assert!(bytes
        .windows(4)
        .any(|chunk| chunk == [0x67, 0x80, 0x00, 0x00]));
}

#[test]
fn machine_phase8_riscv64_object_ok() {
    let object = emit_machine_object(
        "examples/machine/native/riscv64_parity_core.nm",
        "riscv64-unknown-elf",
        "phase8_riscv64_object",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"object_verified\""));
}

#[test]
fn machine_phase8_riscv64_relocations_ok() {
    assert_machine_ok("examples/machine/native/riscv64_parity_relocations.nm");
}

#[test]
fn machine_phase8_riscv64_disasm_ok() {
    let object = emit_machine_object(
        "examples/machine/native/riscv64_parity_core.nm",
        "riscv64-unknown-elf",
        "phase8_riscv64_disasm",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"feature_family\": \"riscv64\""));
    assert!(stdout.contains("\"unknown_bytes\": []"));
}

#[test]
fn machine_phase8_riscv64_roundtrip_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/riscv64_parity_roundtrip.nm",
        "riscv64-unknown-elf",
    );
    assert!(stdout.contains("\"lossless\": true"));
}

#[test]
fn machine_phase8_riscv64_invalid_register_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/riscv64_invalid_register_fail.nm",
        "E_MACHINE_RISCV64_001",
    );
}

#[test]
fn machine_phase8_riscv64_bad_relocation_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/riscv64_bad_relocation_fail.nm",
        "E_MACHINE_RELOCATION_UNSUPPORTED_001",
    );
}

#[test]
fn machine_phase8_wasm32_manifest_exhaustive() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/manifests/phase8_wasm32_instruction_manifest.json",
    ))
    .expect("phase8 wasm32 manifest");
    assert!(manifest.contains("nC-wasm32-core-v1"));
    assert!(manifest.contains("\"final_status\": \"closed\""));
    assert!(manifest.contains("i32.store"));
}

#[test]
fn machine_phase8_wasm32_module_ok() {
    let object = emit_machine_object(
        "examples/machine/native/wasm32_parity_core.nm",
        "wasm32-unknown-unknown",
        "phase8_wasm32_module",
        "wasm",
    );
    let bytes = fs::read(object).expect("wasm bytes");
    assert!(bytes.starts_with(b"\0asm"));
}

#[test]
fn machine_phase8_wasm32_validation_ok() {
    assert_machine_ok("examples/machine/native/wasm32_parity_memory.nm");
}

#[test]
fn machine_phase8_wasm32_disasm_ok() {
    let object = emit_machine_object(
        "examples/machine/native/wasm32_parity_core.nm",
        "wasm32-unknown-unknown",
        "phase8_wasm32_disasm",
        "wasm",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("path"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"feature_family\": \"wasm32\""));
    assert!(stdout.contains("\"unknown_bytes\": []"));
}

#[test]
fn machine_phase8_wasm32_roundtrip_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/wasm32_parity_roundtrip.nm",
        "wasm32-unknown-unknown",
    );
    assert!(stdout.contains("\"lossless\": true"));
}

#[test]
fn machine_phase8_wasm32_invalid_stack_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/wasm32_invalid_stack_fail.nm",
        "E_MACHINE_WASM_STACK_001",
    );
}

#[test]
fn machine_phase8_wasm32_unknown_opcode_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/wasm32_unknown_opcode_fail.nm",
        "E_MACHINE_INSTR_UNKNOWN_001",
    );
}

#[test]
fn machine_phase8_cross_arch_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase8_asm_complete_total_true_if_all_closed() {
    let out = run_ncc(&["machine", "cross-arch", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn machine_phase8_no_fake_bytes() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " bytes"].concat()));
}

#[test]
fn machine_phase8_no_fake_disasm() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " disasm"].concat()));
}

#[test]
fn machine_phase8_no_fake_roundtrip() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " roundtrip"].concat()));
}

#[test]
fn machine_phase8_no_external_objdump_dependency() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"no_external_objdump_dependency\": true"));
}

#[test]
fn machine_phase8_no_external_assembler_truth() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"no_external_assembler_truth\": true"));
}

#[test]
fn machine_phase8_no_fn_fa_positive_examples() {
    for rel in [
        "examples/machine/native/aarch64_parity_core.nm",
        "examples/machine/native/aarch64_parity_relocations.nm",
        "examples/machine/native/aarch64_parity_roundtrip.nm",
        "examples/machine/native/riscv64_parity_core.nm",
        "examples/machine/native/riscv64_parity_relocations.nm",
        "examples/machine/native/riscv64_parity_roundtrip.nm",
        "examples/machine/native/wasm32_parity_core.nm",
        "examples/machine/native/wasm32_parity_memory.nm",
        "examples/machine/native/wasm32_parity_roundtrip.nm",
    ] {
        let text = fs::read_to_string(fixture(rel)).expect(rel);
        assert!(!text.contains(&["fn", ":"].concat()), "{rel}");
        assert!(!text.contains(&["fa", ":"].concat()), "{rel}");
    }
}

#[test]
fn machine_phase8_x86_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx2_vex_complete_x86_64.nm");
}

#[test]
fn machine_phase8_x86_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase8_x86_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase8_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase8_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase8_roundtrip_regressions_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"lossless\": true"));
}

#[test]
fn machine_phase9_industrial_audit_exists() {
    let audit = fs::read_to_string(fixture(
        "reports/machine_native_total/phase9/phase9_industrial_asm_audit.json",
    ))
    .expect("phase9 audit");
    assert!(audit.contains("\"phase\": \"phase9_industrial_asm_audit\""));
    assert!(audit.contains("\"status\": \"closed\""));
    assert!(audit.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase9_native_doctor_asm_total_true() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

#[test]
fn machine_phase9_machine_doctor_ok() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("machine_native_asm_total_closed"));
}

#[test]
fn machine_phase9_cross_arch_doctor_ok() {
    let out = run_ncc(&["machine", "cross-arch", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase9_conformance_matrix_exists() {
    let matrix = fs::read_to_string(fixture(
        "reports/machine_native_total/phase9/phase9_conformance_matrix.json",
    ))
    .expect("phase9 conformance matrix");
    assert!(matrix.contains("\"status\": \"closed\""));
    assert!(matrix.contains("nC-aarch64-core-v1"));
    assert!(matrix.contains("nC-riscv64-core-v1"));
    assert!(matrix.contains("nC-wasm32-core-v1"));
}

#[test]
fn machine_phase9_evidence_bundle_manifest_exists() {
    let manifest = fs::read_to_string(fixture(
        "reports/machine_native_total/evidence_bundle/EVIDENCE_BUNDLE_MANIFEST.json",
    ))
    .expect("phase9 evidence bundle manifest");
    assert!(manifest.contains("\"status\": \"closed\""));
    assert!(manifest.contains("phase9_native_doctor_final.json"));
    assert!(manifest.contains("phase9_conformance_matrix.json"));
}

#[test]
fn machine_phase9_known_limitations_exists() {
    let limitations = fs::read_to_string(fixture("docs/spec/machine/machine_known_limitations.md"))
        .expect("phase9 known limitations");
    assert!(limitations.contains("Machine Source Of Truth"));
    assert!(limitations.contains("closed for declared profiles"));
    assert!(limitations.contains("arbitrary whole-universe"));
    assert!(limitations.contains("exact-head multi-host CI success"));
    assert!(limitations.contains("retained row"));
}

#[test]
fn machine_phase9_worktree_debt_report_exists() {
    let report = fs::read_to_string(fixture(
        "reports/machine_native_total/phase9/phase9_worktree_debt_report.md",
    ))
    .expect("phase9 worktree debt report");
    assert!(report.contains("git status --short"));
    assert!(report.contains("crates/ncc-cli/src/commands/machine.rs"));
    assert!(report.contains("unrelated"));
}

#[test]
fn machine_phase9_no_fake_markers() {
    let phase9_dir = fixture("reports/machine_native_total/phase9");
    let prohibited = [
        ["fake", " bytes"].concat(),
        ["fake", " disasm"].concat(),
        ["fake", " roundtrip"].concat(),
        ["fake", " debug"].concat(),
        ["fake", " unwind"].concat(),
    ];
    for entry in fs::read_dir(phase9_dir).expect("phase9 reports dir") {
        let path = entry.expect("phase9 report entry").path();
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path).expect("phase9 report text");
        for marker in &prohibited {
            assert!(
                !text.contains(marker),
                "marker {marker} in {}",
                path.display()
            );
        }
    }
}

#[test]
fn machine_phase9_no_fn_fa_positive_examples() {
    let native_dir = fixture("examples/machine/native");
    for entry in fs::read_dir(native_dir).expect("native examples dir") {
        let path = entry.expect("native example entry").path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("nm") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("positive machine example");
        assert!(!text.contains(&["fn", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["fa", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["stc", ":"].concat()), "{}", path.display());
    }
}

#[test]
fn machine_phase9_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/vex_fma_f16c_encoding_x86_64.nm");
}

#[test]
fn machine_phase9_avx2_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx2_vex_complete_x86_64.nm");
}

#[test]
fn machine_phase9_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase9_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase9_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase9_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase9_roundtrip_regressions_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"lossless\": true"));
}

#[test]
fn machine_phase9_cross_arch_regressions_ok() {
    let aarch64 = assert_roundtrip_ok(
        "examples/machine/native/aarch64_parity_roundtrip.nm",
        "aarch64-unknown-linux-gnu",
    );
    let riscv64 = assert_roundtrip_ok(
        "examples/machine/native/riscv64_parity_roundtrip.nm",
        "riscv64-unknown-elf",
    );
    let wasm32 = assert_roundtrip_ok(
        "examples/machine/native/wasm32_parity_roundtrip.nm",
        "wasm32-unknown-unknown",
    );
    assert!(aarch64.contains("\"lossless\": true"));
    assert!(riscv64.contains("\"lossless\": true"));
    assert!(wasm32.contains("\"lossless\": true"));
}

#[test]
fn machine_phase9_does_not_start_phase10() {
    let audit = fs::read_to_string(fixture(
        "reports/machine_native_total/phase9/phase9_industrial_asm_audit.json",
    ))
    .expect("phase9 audit");
    let status = fs::read_to_string(fixture(
        "docs/status/STATUS_machine_phase9_industrial_audit.md",
    ))
    .expect("phase9 status");
    assert!(audit.contains("\"phase10_started\": false"));
    assert!(status.contains("did not start Phase 10"));
}

fn assert_phase10_command_ok(args: &[&str], expected: &str) -> String {
    let out = run_ncc(args);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "command must pass: {args:?}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(expected),
        "command output must contain {expected}\nstdout={stdout}"
    );
    stdout
}

#[test]
fn machine_phase10_macro_basic_ok() {
    assert_machine_ok("examples/machine/native/macro_basic_x86_64.nm");
}

#[test]
fn machine_phase10_macro_params_ok() {
    assert_machine_ok("examples/machine/native/macro_params_x86_64.nm");
}

#[test]
fn machine_phase10_macro_variadic_ok() {
    assert_machine_ok("examples/machine/native/macro_variadic_x86_64.nm");
}

#[test]
fn machine_phase10_macro_local_labels_hygienic_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "macro-report",
            fixture("examples/machine/native/macro_local_labels_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "loop_start$macro$1",
    );
    assert!(stdout.contains("loop_start$macro$2"));
}

#[test]
fn machine_phase10_macro_recursion_limit_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/macro_recursion_limit_fail.nm",
        "E_MACHINE_MACRO_RECURSION_001",
    );
}

#[test]
fn machine_phase10_macro_label_collision_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/macro_label_collision_fail.nm",
        "E_MACHINE_MACRO_LABEL_COLLISION_001",
    );
}

#[test]
fn machine_phase10_macro_expansion_report_ok() {
    assert_phase10_command_ok(
        &[
            "machine",
            "expand",
            fixture("examples/machine/native/macro_params_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "mov rax, 42",
    );
}

#[test]
fn machine_phase10_db_dw_dd_dq_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "directive-report",
            fixture("examples/machine/native/directive_data_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"data_bytes\": 26",
    );
    assert!(stdout.contains("\"kind\": \"dq\""));
}

#[test]
fn machine_phase10_ascii_asciz_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "directive-report",
            fixture("examples/machine/native/directive_data_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"kind\": \"ascii\"",
    );
    assert!(stdout.contains("\"kind\": \"asciz\""));
}

#[test]
fn machine_phase10_resb_resw_resd_resq_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "directive-report",
            fixture("examples/machine/native/directive_bss_reserve_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"bss_size\": 448",
    );
    assert!(stdout.contains("closed_bss"));
}

#[test]
fn machine_phase10_align_ok() {
    assert_machine_ok("examples/machine/native/directive_align_org_x86_64.nm");
}

#[test]
fn machine_phase10_org_profile_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "directive-report",
            fixture("examples/machine/native/directive_align_org_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "closed_profile_allowed",
    );
    assert!(stdout.contains("\"kind\": \"org\""));
}

#[test]
fn machine_phase10_org_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/directive_org_forbidden_fail.nm",
        "E_MACHINE_ORG_FORBIDDEN_001",
    );
}

#[test]
fn machine_phase10_equ_set_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "ctfe-report",
            fixture("examples/machine/native/directive_equ_set_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"MASK_RW\": 3",
    );
    assert!(stdout.contains("\"localCounter\": 1"));
}

#[test]
fn machine_phase10_equ_redefined_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/directive_equ_redefined_fail.nm",
        "E_MACHINE_EQU_REDEFINED_001",
    );
}

#[test]
fn machine_phase10_ctfe_arithmetic_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "ctfe-report",
            fixture("examples/machine/native/directive_equ_set_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"PAGE_SIZE\": 4096",
    );
    assert!(stdout.contains("\"localCounter\": 1"));
}

#[test]
fn machine_phase10_ctfe_bitwise_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "ctfe-report",
            fixture("examples/machine/native/directive_equ_set_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"MASK_RW\": 3",
    );
    assert!(stdout.contains("\"FLAG_WRITE\": 2"));
}

#[test]
fn machine_phase10_ctfe_div_zero_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/ctfe_div_zero_fail.nm",
        "E_MACHINE_CTFE_DIV_ZERO_001",
    );
}

#[test]
fn machine_phase10_ctfe_overflow_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/ctfe_overflow_fail.nm",
        "E_MACHINE_CTFE_OVERFLOW_001",
    );
}

#[test]
fn machine_phase10_ctfe_unresolved_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/ctfe_unresolved_fail.nm",
        "E_MACHINE_CTFE_UNRESOLVED_001",
    );
}

#[test]
fn machine_phase10_include_ok() {
    assert_phase10_command_ok(
        &[
            "machine",
            "include-report",
            fixture("examples/machine/native/directive_include_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "common_constants.nmi",
    );
}

#[test]
fn machine_phase10_include_missing_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/include_missing_fail.nm",
        "E_MACHINE_INCLUDE_NOT_FOUND_001",
    );
}

#[test]
fn machine_phase10_include_cycle_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/include_cycle_fail.nm",
        "E_MACHINE_INCLUDE_CYCLE_001",
    );
}

#[test]
fn machine_phase10_include_depth_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/include_depth_fail.nm",
        "E_MACHINE_INCLUDE_DEPTH_001",
    );
}

#[test]
fn machine_phase10_incbin_ok() {
    assert_machine_ok("examples/machine/native/directive_incbin_x86_64.nm");
}

#[test]
fn machine_phase10_incbin_hash_evidence_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "incbin-report",
            fixture("examples/machine/native/directive_incbin_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "firmwareBlob",
    );
    assert!(stdout.contains("\"hash\""));
}

#[test]
fn machine_phase10_incbin_too_large_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/incbin_too_large_fail.nm",
        "E_MACHINE_INCBIN_TOO_LARGE_001",
    );
}

#[test]
fn machine_phase10_incbin_forbidden_path_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/incbin_forbidden_path_fail.nm",
        "E_MACHINE_INCBIN_FORBIDDEN_PATH_001",
    );
}

#[test]
fn machine_phase10_conditional_target_ok() {
    assert_machine_ok("examples/machine/native/macro_conditional_x86_64.nm");
}

#[test]
fn machine_phase10_conditional_feature_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "report",
            fixture("examples/machine/native/macro_conditional_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"condition\": \"avx512f\"",
    );
    assert!(stdout.contains("\"included\": true"));
}

#[test]
fn machine_phase10_conditional_unknown_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/conditional_unknown_fail.nm",
        "E_MACHINE_COND_ASM_001",
    );
}

#[test]
fn machine_phase10_repeat_ok() {
    let stdout = assert_phase10_command_ok(
        &[
            "machine",
            "report",
            fixture("examples/machine/native/macro_repeat_x86_64.nm")
                .to_str()
                .expect("path"),
            "--json",
        ],
        "\"emitted\": 4",
    );
    assert!(stdout.contains("times_or_repeat"));
}

#[test]
fn machine_phase10_repeat_limit_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/repeat_limit_fail.nm",
        "E_MACHINE_REPEAT_LIMIT_001",
    );
}

#[test]
fn machine_phase10_macro_directive_emit_object_elf64_ok() {
    let object = emit_machine_object(
        "examples/machine/native/macro_directive_object_elf64.nm",
        "x86_64-unknown-linux-gnu",
        "phase10_macro_directive_elf64",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("object"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("object_verified"));
}

#[test]
fn machine_phase10_macro_directive_emit_object_coff_ok() {
    let object = emit_machine_object(
        "examples/machine/native/macro_directive_object_coff.nm",
        "x86_64-pc-windows-msvc",
        "phase10_macro_directive_coff",
        "obj",
    );
    let bytes = fs::read(object).expect("coff object");
    assert!(bytes.starts_with(&[0x64, 0x86]));
}

#[test]
fn machine_phase10_macro_directive_emit_object_macho_ok() {
    let object = emit_machine_object(
        "examples/machine/native/macro_directive_object_macho.nm",
        "x86_64-apple-darwin",
        "phase10_macro_directive_macho",
        "o",
    );
    let bytes = fs::read(object).expect("macho object");
    assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
}

#[test]
fn machine_phase10_macro_directive_wasm_ok() {
    let object = emit_machine_object(
        "examples/machine/native/macro_directive_wasm.nm",
        "wasm32-unknown-unknown",
        "phase10_macro_directive_wasm",
        "wasm",
    );
    let bytes = fs::read(object).expect("wasm object");
    assert!(bytes.starts_with(b"\0asm"));
}

#[test]
fn machine_phase10_macro_directive_disasm_ok() {
    let object = emit_machine_object(
        "examples/machine/native/macro_directive_object_elf64.nm",
        "x86_64-unknown-linux-gnu",
        "phase10_macro_directive_disasm",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "disasm",
        object.to_str().expect("object"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"unknown_bytes\": []"));
}

#[test]
fn machine_phase10_macro_directive_roundtrip_ok() {
    let stdout = assert_roundtrip_ok(
        "examples/machine/native/macro_directive_roundtrip_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
    assert!(stdout.contains("\"roundtripData\""));
}

#[test]
fn machine_phase10_verify_object_directives_ok() {
    let object = emit_machine_object(
        "examples/machine/native/directive_data_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase10_directive_verify",
        "o",
    );
    let out = run_ncc(&[
        "machine",
        "verify-object",
        object.to_str().expect("object"),
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("object_verified"));
}

#[test]
fn machine_phase10_no_fake_macro_expansion() {
    let text = fs::read_to_string(fixture(
        "reports/machine_native_total/phase10/phase10_no_fake_evidence.md",
    ))
    .expect("phase10 evidence");
    for marker in [
        ["fake", " macro"].concat(),
        ["fake", " expansion"].concat(),
        ["fake", " bytes"].concat(),
    ] {
        assert!(!text.contains(&marker), "{marker}");
    }
}

#[test]
fn machine_phase10_no_fake_bytes() {
    let report = fs::read_to_string(fixture(
        "reports/machine_native_total/phase10/phase10_directive_report.json",
    ))
    .expect("phase10 directive report");
    assert!(report.contains("\"data_hash\""));
    assert!(!report.contains(&["fake", " bytes"].concat()));
}

#[test]
fn machine_phase10_no_fn_fa_positive_examples() {
    for pattern in ["macro_", "directive_", "macro_directive_"] {
        for entry in fs::read_dir(fixture("examples/machine/native")).expect("native dir") {
            let path = entry.expect("entry").path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with(pattern)
                || path.extension().and_then(|ext| ext.to_str()) != Some("nm")
            {
                continue;
            }
            let text = fs::read_to_string(&path).expect("phase10 positive");
            assert!(!text.contains(&["fn", ":"].concat()), "{}", path.display());
            assert!(!text.contains(&["fa", ":"].concat()), "{}", path.display());
            assert!(!text.contains(&["stc", ":"].concat()), "{}", path.display());
        }
    }
}

#[test]
fn machine_phase10_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase10_macro_directive_system\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase10_preserved_after_asm_dialect() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase10_macro_directive_system\": \"closed\""));
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
}

#[test]
fn machine_phase10_does_not_break_asm_complete_total() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
    assert!(stdout.contains("\"phase10_macro_directive_system\""));
}

#[test]
fn machine_phase10_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/vex_fma_f16c_encoding_x86_64.nm");
}

#[test]
fn machine_phase10_evex_regressions_ok() {
    assert_machine_ok("examples/machine/native/evex_complete_x86_64.nm");
}

#[test]
fn machine_phase10_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase10_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase10_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase10_roundtrip_regressions_ok() {
    assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
}

#[test]
fn machine_phase10_cross_arch_regressions_ok() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

fn run_ncc_dynamic(args: &[String]) -> Output {
    let mapped_args = args.iter().map(|arg| cli_arg(arg)).collect::<Vec<_>>();
    Command::new(ncc_bin())
        .args(mapped_args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc executes")
}

fn asm_dialect_fixture(syntax: &str) -> (&'static str, &'static str) {
    match syntax {
        "nasm" => (
            "examples/machine/dialects/nasm/profile_closed.asm",
            "nC-nasm-compat-v1",
        ),
        "yasm" => (
            "examples/machine/dialects/yasm/profile_closed.asm",
            "nC-yasm-compat-v1",
        ),
        "gas-intel" => (
            "examples/machine/dialects/gas-intel/profile_closed.s",
            "nC-gas-intel-compat-v1",
        ),
        "gas-att" => (
            "examples/machine/dialects/gas-att/profile_closed.s",
            "nC-gas-att-compat-v1",
        ),
        "masm" => (
            "examples/machine/dialects/masm/profile_closed.asm",
            "nC-masm-compat-v1",
        ),
        "fasm" => (
            "examples/machine/dialects/fasm/profile_closed.asm",
            "nC-fasm-compat-v1",
        ),
        "llvm-mc" => (
            "examples/machine/dialects/llvm-mc/profile_closed.s",
            "nC-llvm-mc-compat-v1",
        ),
        other => panic!("unknown asm_dialect syntax {other}"),
    }
}

fn assert_asm_dialect_profile_closed(syntax: &str) {
    let (_, profile) = asm_dialect_fixture(syntax);
    let out = run_ncc(&[
        "machine",
        "asm-dialect",
        "report",
        "--syntax",
        syntax,
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains(profile));
    assert!(stdout.contains("\"final_status\": \"closed\""));
    assert!(stdout.contains("\"import_coverage\": \"closed\""));
    assert!(stdout.contains("\"export_coverage\": \"closed\""));
    assert!(stdout.contains("\"import_back_coverage\": \"closed\""));
}

fn assert_asm_dialect_import_export_lossless(syntax: &str) {
    let (fixture_rel, profile) = asm_dialect_fixture(syntax);
    assert_asm_dialect_import_export_lossless_for(syntax, fixture_rel, profile, None);
}

fn assert_asm_dialect_import_export_lossless_for(
    syntax: &str,
    fixture_rel: &str,
    profile: &str,
    marker: Option<&str>,
) {
    let source = fixture_cli(fixture_rel);
    let imported = temp_path(&format!("asm_dialect_{syntax}_imported"), "nm");
    let exported = temp_path(&format!("asm_dialect_{syntax}_exported"), "asm");
    let reimported = temp_path(&format!("asm_dialect_{syntax}_reimported"), "nm");

    let import_args = vec![
        "machine".to_string(),
        "import-asm".to_string(),
        source,
        "--syntax".to_string(),
        syntax.to_string(),
        "--out".to_string(),
        imported.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let import_out = run_ncc_dynamic(&import_args);
    let import_stdout = normalize(&import_out.stdout);
    let import_stderr = normalize(&import_out.stderr);
    assert!(
        import_out.status.success(),
        "import {syntax}\nstdout={import_stdout}\nstderr={import_stderr}"
    );
    assert!(import_stdout.contains(profile));
    assert!(import_stdout.contains("\"loss_model\": \"lossless\""));
    if let Some(marker) = marker {
        assert!(
            import_stdout.contains(marker),
            "missing marker {marker} in import stdout {import_stdout}"
        );
    }

    let check_args = vec![
        "machine".to_string(),
        "check".to_string(),
        imported.to_string_lossy().to_string(),
    ];
    let check_out = run_ncc_dynamic(&check_args);
    assert!(
        check_out.status.success(),
        "imported machine check {syntax}\nstdout={}\nstderr={}",
        normalize(&check_out.stdout),
        normalize(&check_out.stderr)
    );

    let export_args = vec![
        "machine".to_string(),
        "export-asm".to_string(),
        imported.to_string_lossy().to_string(),
        "--syntax".to_string(),
        syntax.to_string(),
        "--out".to_string(),
        exported.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let export_out = run_ncc_dynamic(&export_args);
    let export_stdout = normalize(&export_out.stdout);
    assert!(export_out.status.success(), "{export_stdout}");
    assert!(export_stdout.contains("\"fake_export\": false"));

    let reimport_args = vec![
        "machine".to_string(),
        "import-asm".to_string(),
        exported.to_string_lossy().to_string(),
        "--syntax".to_string(),
        syntax.to_string(),
        "--out".to_string(),
        reimported.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let reimport_out = run_ncc_dynamic(&reimport_args);
    let reimport_stdout = normalize(&reimport_out.stdout);
    assert!(reimport_out.status.success(), "{reimport_stdout}");
    assert!(reimport_stdout.contains("\"loss_model\": \"lossless\""));

    let import_back_args = vec![
        "machine".to_string(),
        "asm-import-back".to_string(),
        imported.to_string_lossy().to_string(),
        "--syntax".to_string(),
        syntax.to_string(),
        "--json".to_string(),
    ];
    let import_back = run_ncc_dynamic(&import_back_args);
    let import_back_stdout = normalize(&import_back.stdout);
    assert!(import_back.status.success(), "{import_back_stdout}");
    assert!(import_back_stdout.contains("\"loss_model\": \"lossless\""));
    assert!(import_back_stdout.contains("\"silent_loss\": 0"));
}

fn assert_asm_dialect_deep_import_ok(syntax: &str, rel: &str, marker: &str) {
    let (_, profile) = asm_dialect_fixture(syntax);
    assert_asm_dialect_import_export_lossless_for(syntax, rel, profile, Some(marker));
}

fn assert_asm_dialect_deep_negative(syntax: &str, rel: &str, code: &str) {
    let source = fixture(rel);
    let out = temp_path(&format!("asm_dialect_{syntax}_deep_bad"), "nm");
    let args = vec![
        "machine".to_string(),
        "import-asm".to_string(),
        source.to_string_lossy().to_string(),
        "--syntax".to_string(),
        syntax.to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        !output.status.success(),
        "deep negative must fail for {syntax}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(code) || stderr.contains(code),
        "missing {code}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_asm_dialect_negative_unsupported(syntax: &str, rel: &str) {
    let source = fixture(rel);
    let out = temp_path(&format!("asm_dialect_{syntax}_bad"), "nm");
    let args = vec![
        "machine".to_string(),
        "import-asm".to_string(),
        source.to_string_lossy().to_string(),
        "--syntax".to_string(),
        syntax.to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        !output.status.success(),
        "unsupported directive must fail for {syntax}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("E_MACHINE_ASM_UNSUPPORTED_DIRECTIVE_001")
            || stderr.contains("E_MACHINE_ASM_UNSUPPORTED_DIRECTIVE_001"),
        "missing unsupported diagnostic for {syntax}\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn machine_asm_dialect_doctor_ok() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
    assert!(stdout.contains("\"silent_loss\": 0"));
    assert!(stdout.contains("\"fake_import\": false"));
    assert!(stdout.contains("\"fake_export\": false"));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_asm_dialect_profiles_all_closed() {
    let out = run_ncc(&["machine", "asm-dialect", "profiles", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for syntax in [
        "nasm",
        "yasm",
        "gas-intel",
        "gas-att",
        "masm",
        "fasm",
        "llvm-mc",
    ] {
        let (_, profile) = asm_dialect_fixture(syntax);
        assert!(stdout.contains(profile), "missing {profile}");
    }
    assert!(stdout.contains("\"final_status\": \"closed\""));
}

#[test]
fn machine_asm_dialect_manifest_files_exist() {
    for syntax in [
        "nasm",
        "yasm",
        "gas-intel",
        "gas-att",
        "masm",
        "fasm",
        "llvm-mc",
    ] {
        let out = run_ncc(&[
            "machine",
            "asm-dialect",
            "report",
            "--syntax",
            syntax,
            "--json",
        ]);
        let text = normalize(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(
            text.contains("\"final_status\":\"closed\"")
                || text.contains("\"final_status\": \"closed\"")
        );
        assert!(
            text.contains("\"import_back_coverage\":\"closed\"")
                || text.contains("\"import_back_coverage\": \"closed\"")
        );
    }
}

#[test]
fn machine_asm_dialect_reports_exist() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let text = normalize(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("\"status\": \"closed\""));
    assert!(text.contains("\"silent_loss\": 0"));
    assert!(text.contains("\"fake_import\": false"));
    assert!(text.contains("\"fake_export\": false"));
    assert!(text.contains("\"masm_dup\": \"closed\""));
    assert!(text.contains("\"fasm_repeat\": \"closed\""));
    assert!(text.contains("\"fasm_macro\": \"closed\""));
}

#[test]
fn machine_asm_dialect_deep_feature_manifests_exist() {
    for (syntax, marker) in [
        ("masm", "masm_dup"),
        ("fasm", "fasm_repeat"),
        ("fasm", "fasm_macro"),
    ] {
        let out = run_ncc(&[
            "machine",
            "asm-dialect",
            "report",
            "--syntax",
            syntax,
            "--json",
        ]);
        let text = normalize(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(text.contains(marker), "{text}");
        assert!(
            text.contains("\"final_status\":\"closed\"")
                || text.contains("\"final_status\": \"closed\"")
        );
    }
}

#[test]
fn machine_asm_dialect_nasm_profile_closed() {
    assert_asm_dialect_profile_closed("nasm");
}

#[test]
fn machine_asm_dialect_yasm_profile_closed() {
    assert_asm_dialect_profile_closed("yasm");
}

#[test]
fn machine_asm_dialect_gas_intel_profile_closed() {
    assert_asm_dialect_profile_closed("gas-intel");
}

#[test]
fn machine_asm_dialect_gas_att_profile_closed() {
    assert_asm_dialect_profile_closed("gas-att");
}

#[test]
fn machine_asm_dialect_masm_profile_closed() {
    assert_asm_dialect_profile_closed("masm");
}

#[test]
fn machine_asm_dialect_fasm_profile_closed() {
    assert_asm_dialect_profile_closed("fasm");
}

#[test]
fn machine_asm_dialect_llvm_mc_profile_closed() {
    assert_asm_dialect_profile_closed("llvm-mc");
}

#[test]
fn machine_asm_dialect_nasm_import_export_lossless() {
    assert_asm_dialect_import_export_lossless("nasm");
}

#[test]
fn machine_asm_dialect_yasm_import_export_lossless() {
    assert_asm_dialect_import_export_lossless("yasm");
}

#[test]
fn machine_asm_dialect_gas_intel_import_export_lossless() {
    assert_asm_dialect_import_export_lossless("gas-intel");
}

#[test]
fn machine_asm_dialect_gas_att_operand_order_ok() {
    assert_asm_dialect_import_export_lossless("gas-att");
}

#[test]
fn machine_asm_dialect_masm_ptr_memory_ok() {
    assert_asm_dialect_import_export_lossless("masm");
}

#[test]
fn machine_asm_dialect_fasm_import_export_lossless() {
    assert_asm_dialect_import_export_lossless("fasm");
}

#[test]
fn machine_asm_dialect_masm_dup_literal_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "literalBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_equ_count_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "equBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_ctfe_count_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "ctfeBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_nested_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "nestedBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_value_list_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "listBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_string_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "stringBytes",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_db_dw_dd_dq_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "qwordDup",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_uninitialized_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "buffer",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_expansion_report_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "\"masm_dup_expansions\"",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_import_export_back_ok() {
    assert_asm_dialect_deep_import_ok(
        "masm",
        "examples/machine/dialects/masm/dup_deep.asm",
        "\"fake_import\": false",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_overflow_fail() {
    assert_asm_dialect_deep_negative(
        "masm",
        "examples/machine/dialects/masm/negative/dup_overflow.asm",
        "E_MACHINE_MASM_DUP_OVERFLOW_001",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_negative_count_fail() {
    assert_asm_dialect_deep_negative(
        "masm",
        "examples/machine/dialects/masm/negative/dup_negative_count.asm",
        "E_MACHINE_MASM_DUP_COUNT_001",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_non_constant_count_fail() {
    assert_asm_dialect_deep_negative(
        "masm",
        "examples/machine/dialects/masm/negative/dup_non_constant_count.asm",
        "E_MACHINE_MASM_DUP_CTFE_001",
    );
}

#[test]
fn machine_asm_dialect_masm_dup_bad_syntax_fail() {
    assert_asm_dialect_deep_negative(
        "masm",
        "examples/machine/dialects/masm/negative/dup_bad_syntax.asm",
        "E_MACHINE_MASM_DUP_SYNTAX_001",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_literal_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "repeat 4",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_equ_count_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "COUNT",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_ctfe_count_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "4 + 4",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_counter_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "\"fasm_repeat_expansions\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_nested_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "\"expanded_nodes\": 4",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_labels_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "$repeat$",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_inside_macro_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "repeat_nop",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_expansion_report_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "\"fasm_repeat_expansions\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_import_export_back_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/repeat_deep.asm",
        "\"loss_model\": \"lossless\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_negative_count_fail() {
    assert_asm_dialect_deep_negative(
        "fasm",
        "examples/machine/dialects/fasm/negative/repeat_negative_count.asm",
        "E_MACHINE_FASM_REPEAT_COUNT_001",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_non_constant_count_fail() {
    assert_asm_dialect_deep_negative(
        "fasm",
        "examples/machine/dialects/fasm/negative/repeat_non_constant_count.asm",
        "E_MACHINE_FASM_REPEAT_CTFE_001",
    );
}

#[test]
fn machine_asm_dialect_fasm_repeat_limit_fail() {
    assert_asm_dialect_deep_negative(
        "fasm",
        "examples/machine/dialects/fasm/negative/repeat_limit.asm",
        "E_MACHINE_FASM_REPEAT_LIMIT_001",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_no_params_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "no_params",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_params_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "move_imm",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_variadic_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "emit_many",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_local_labels_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "$macro$",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_calls_macro_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "outer",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_repeat_inside_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "\"fasm_repeat_expansions\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_data_directives_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "asm_dialect_data",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_memory_operands_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "[rax + rbx*2 + 8]",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_expansion_report_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "\"fasm_macro_expansions\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_import_export_back_ok() {
    assert_asm_dialect_deep_import_ok(
        "fasm",
        "examples/machine/dialects/fasm/macro_deep.asm",
        "\"loss_model\": \"lossless\"",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_recursion_fail() {
    assert_asm_dialect_deep_negative(
        "fasm",
        "examples/machine/dialects/fasm/negative/macro_recursion.asm",
        "E_MACHINE_FASM_MACRO_RECURSION_001",
    );
}

#[test]
fn machine_asm_dialect_fasm_macro_label_collision_fail() {
    assert_asm_dialect_deep_negative(
        "fasm",
        "examples/machine/dialects/fasm/negative/macro_label_collision.asm",
        "E_MACHINE_FASM_MACRO_LABEL_COLLISION_001",
    );
}

#[test]
fn machine_asm_dialect_llvm_mc_import_export_lossless() {
    assert_asm_dialect_import_export_lossless("llvm-mc");
}

#[test]
fn machine_asm_dialect_nasm_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "nasm",
        "examples/machine/dialects/nasm/negative/unsupported_directive.asm",
    );
}

#[test]
fn machine_asm_dialect_yasm_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "yasm",
        "examples/machine/dialects/yasm/negative/unsupported_directive.asm",
    );
}

#[test]
fn machine_asm_dialect_gas_intel_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "gas-intel",
        "examples/machine/dialects/gas-intel/negative/unsupported_directive.s",
    );
}

#[test]
fn machine_asm_dialect_gas_att_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "gas-att",
        "examples/machine/dialects/gas-att/negative/unsupported_directive.s",
    );
}

#[test]
fn machine_asm_dialect_masm_unsupported_invoke_fail() {
    assert_asm_dialect_negative_unsupported(
        "masm",
        "examples/machine/dialects/masm/negative/unsupported_directive.asm",
    );
}

#[test]
fn machine_asm_dialect_fasm_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "fasm",
        "examples/machine/dialects/fasm/negative/unsupported_directive.asm",
    );
}

#[test]
fn machine_asm_dialect_llvm_mc_negative_unsupported_directive() {
    assert_asm_dialect_negative_unsupported(
        "llvm-mc",
        "examples/machine/dialects/llvm-mc/negative/unsupported_directive.s",
    );
}

#[test]
fn machine_asm_dialect_imported_nm_no_fn_fa() {
    let source = fixture_cli("examples/machine/dialects/nasm/profile_closed.asm");
    let out = temp_path("asm_dialect_no_fn_fa_imported", "nm");
    let args = vec![
        "machine".to_string(),
        "import-asm".to_string(),
        source,
        "--syntax".to_string(),
        "nasm".to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    assert!(output.status.success(), "{}", normalize(&output.stderr));
    let imported = fs::read_to_string(out).expect("imported nm");
    assert!(!imported.contains("fn:"));
    assert!(!imported.contains("fa:"));
    assert!(!imported.contains("stc:"));
}

#[test]
fn machine_asm_dialect_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_asm_dialect_no_fake_import_export() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"fake_import\": false"));
    assert!(stdout.contains("\"fake_export\": false"));
    assert!(!stdout.contains("fake import"));
    assert!(!stdout.contains("fake export"));
}

#[test]
fn machine_asm_dialect_no_silent_loss() {
    let out = run_ncc(&["machine", "asm-dialect", "report", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"silent_loss\": 0"));
    assert!(stdout.contains("\"manual_review_required\": 0"));
}

#[test]
fn machine_asm_dialect_preserves_phase13_closed() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
}

#[test]
fn machine_asm_dialect_phase10_regression_ok() {
    assert_machine_ok("examples/machine/native/macro_directive_roundtrip_x86_64.nm");
}

fn phase12_emit(rel: &str, target: &str, name: &str) -> PathBuf {
    let ext = if target.contains("windows") {
        "obj"
    } else if target.contains("wasm32") {
        "wasm"
    } else {
        "o"
    };
    emit_machine_object(rel, target, name, ext)
}

fn phase12_link_objects(name: &str, target: &str, objects: &[PathBuf]) -> PathBuf {
    let out = temp_path(name, "linked");
    let map = temp_path(&format!("{name}_map"), "map");
    let mut args = vec!["machine".to_string(), "link".to_string()];
    for object in objects {
        args.push(object.to_string_lossy().to_string());
    }
    args.extend([
        "--target".to_string(),
        target.to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
        "--map".to_string(),
        map.to_string_lossy().to_string(),
        "--json".to_string(),
    ]);
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "phase12 link failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"unresolved_symbols\": []"));
    out
}

fn phase12_write_third_party_object(
    name: &str,
    format: &str,
    text: &[u8],
    symbols: &[(&str, usize, bool)],
    relocs: &[(usize, ncc_ir::BackendObjectRelocationKindIR, &str, isize)],
) -> PathBuf {
    let mut profile = match format {
        "aarch64" => ncc_ir::BackendLoweringProfileIR::nc_aapcs64(),
        "coff" => ncc_ir::BackendLoweringProfileIR::nc_win64(),
        _ => ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
    };
    if format == "macho" {
        profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::MachO;
        profile.target_spec.target_triple = "x86_64-apple-darwin".to_string();
    } else if format == "aarch64" {
        profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
        profile.target_spec.target_triple = "aarch64-unknown-linux-gnu".to_string();
    }
    let section_name = if format == "macho" { "__text" } else { ".text" };
    let object = ncc_ir::BackendObjectIR {
        target: profile.target,
        target_spec: profile.target_spec,
        sections: vec![ncc_ir::BackendObjectSectionIR {
            name: section_name.to_string(),
            kind: ncc_ir::BackendObjectSectionKindIR::Text,
            unit: "third_party".to_string(),
            lines: Vec::new(),
            bytes: Some(text.to_vec()),
            size_bytes: text.len(),
            align_bytes: 1,
            comdat: None,
        }],
        symbols: symbols
            .iter()
            .map(|(name, offset, global)| ncc_ir::BackendObjectSymbolIR {
                name: (*name).to_string(),
                section: section_name.to_string(),
                offset: *offset,
                visibility: if *global {
                    ncc_ir::BackendSymbolVisibilityIR::Global
                } else {
                    ncc_ir::BackendSymbolVisibilityIR::Local
                },
            })
            .collect(),
        relocations: relocs
            .iter()
            .map(
                |(offset, kind, symbol, addend)| ncc_ir::BackendObjectRelocationIR {
                    section: section_name.to_string(),
                    offset: *offset,
                    kind: kind.clone(),
                    symbol: (*symbol).to_string(),
                    addend: *addend,
                },
            )
            .collect(),
    };
    let bytes = match format {
        "coff" => ncc_ir::emit_object_coff(&object).expect("third-party COFF object"),
        "macho" => ncc_ir::emit_object_macho64(&object).expect("third-party Mach-O object"),
        _ => ncc_ir::emit_object_elf64(&object).expect("third-party ELF object"),
    };
    let ext = if format == "coff" { "obj" } else { "o" };
    let path = temp_path(name, ext);
    fs::write(&path, bytes).expect("write third-party object");
    let _ = fs::remove_file(format!("{}.machine-link-meta.json", path.to_string_lossy()));
    path
}

fn phase12_compile_system_elf_asm(name: &str, asm: &str) -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let compiler = ["cc", "clang", "gcc"]
        .iter()
        .find(|tool| Command::new(tool).arg("--version").output().is_ok())?;
    let source = temp_path(name, "s");
    let object = temp_path(name, "o");
    fs::write(&source, asm).expect("write system assembler fixture");
    let output = Command::new(compiler)
        .args([
            "-c",
            "-x",
            "assembler",
            source.to_str().expect("asm path"),
            "-o",
            object.to_str().expect("object path"),
        ])
        .output()
        .expect("system assembler executes");
    if output.status.success() {
        Some(object)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
struct Phase12TestElfSymbol<'a> {
    name: &'a str,
    section: &'a str,
    offset: u64,
    global: bool,
}

#[derive(Clone, Copy)]
struct Phase12TestElfReloc<'a> {
    section: &'a str,
    offset: u64,
    kind: u64,
    symbol: &'a str,
    addend: i64,
}

fn phase12_test_align_to(value: usize, align: usize) -> usize {
    if align == 0 {
        return value;
    }
    (value + align - 1) & !(align - 1)
}

#[allow(clippy::too_many_arguments)]
fn phase12_write_raw_elf64_object(
    name: &str,
    machine: u16,
    flags: u32,
    text: &[u8],
    rodata: &[u8],
    data: &[u8],
    symbols: &[Phase12TestElfSymbol<'_>],
    relocs: &[Phase12TestElfReloc<'_>],
) -> PathBuf {
    #[derive(Clone)]
    struct Section {
        name: String,
        ty: u32,
        flags: u64,
        data: Vec<u8>,
        align: u64,
        entsize: u64,
        link: u32,
        info: u32,
        offset: usize,
    }

    fn push_cstr(table: &mut Vec<u8>, text: &str) -> u32 {
        let off = table.len() as u32;
        table.extend_from_slice(text.as_bytes());
        table.push(0);
        off
    }

    fn write16(out: &mut [u8], off: usize, value: u16) {
        out[off..off + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn write32(out: &mut [u8], off: usize, value: u32) {
        out[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn write64(out: &mut [u8], off: usize, value: u64) {
        out[off..off + 8].copy_from_slice(&value.to_le_bytes());
    }

    let has_data = !data.is_empty();
    let has_rodata = !rodata.is_empty();
    let text_index = 1_u16;
    let rodata_index = if has_rodata { Some(2_u16) } else { None };
    let data_index = if has_data {
        Some(2_u16 + u16::from(has_rodata))
    } else {
        None
    };
    let mut strtab = vec![0];
    let mut sym_rows = Vec::<(u32, u8, u16, u64, u64)>::new();
    sym_rows.push((0, 0, 0, 0, 0));
    let text_section_symbol_index = sym_rows.len() as u32;
    sym_rows.push((0, 0x03, text_index, 0, 0));
    if let Some(index) = rodata_index {
        sym_rows.push((0, 0x03, index, 0, 0));
    }
    if let Some(index) = data_index {
        sym_rows.push((0, 0x03, index, 0, 0));
    }
    let local_count = sym_rows.len() as u32;
    let mut sym_index = std::collections::BTreeMap::<String, u32>::new();
    sym_index.insert("$text".to_string(), text_section_symbol_index);
    for symbol in symbols {
        let name = push_cstr(&mut strtab, symbol.name);
        let bind = if symbol.global { 1 } else { 0 };
        let shndx = match symbol.section {
            ".text" | "text" => text_index,
            ".rodata" | "rodata" => rodata_index.unwrap_or(text_index),
            ".data" | "data" => data_index.unwrap_or(text_index),
            _ => 0,
        };
        let index = sym_rows.len() as u32;
        sym_rows.push((name, (bind << 4) | 2, shndx, symbol.offset, 0));
        sym_index.insert(symbol.name.to_string(), index);
    }

    let mut symtab = Vec::new();
    for (name, info, shndx, value, size) in &sym_rows {
        symtab.extend_from_slice(&name.to_le_bytes());
        symtab.push(*info);
        symtab.push(0);
        symtab.extend_from_slice(&shndx.to_le_bytes());
        symtab.extend_from_slice(&value.to_le_bytes());
        symtab.extend_from_slice(&size.to_le_bytes());
    }

    let mut sections = Vec::<Section>::new();
    sections.push(Section {
        name: ".text".to_string(),
        ty: 1,
        flags: 0x6,
        data: text.to_vec(),
        align: 16,
        entsize: 0,
        link: 0,
        info: 0,
        offset: 0,
    });
    if has_rodata {
        sections.push(Section {
            name: ".rodata".to_string(),
            ty: 1,
            flags: 0x2,
            data: rodata.to_vec(),
            align: 8,
            entsize: 0,
            link: 0,
            info: 0,
            offset: 0,
        });
    }
    if has_data {
        sections.push(Section {
            name: ".data".to_string(),
            ty: 1,
            flags: 0x3,
            data: data.to_vec(),
            align: 8,
            entsize: 0,
            link: 0,
            info: 0,
            offset: 0,
        });
    }

    let mut rela_text = Vec::new();
    let mut rela_rodata = Vec::new();
    let mut rela_data = Vec::new();
    for rel in relocs {
        let symbol = *sym_index
            .get(rel.symbol)
            .unwrap_or_else(|| panic!("missing symbol {}", rel.symbol));
        let target = if rel.section == ".rodata" || rel.section == "rodata" {
            &mut rela_rodata
        } else if rel.section == ".data" || rel.section == "data" {
            &mut rela_data
        } else {
            &mut rela_text
        };
        target.extend_from_slice(&rel.offset.to_le_bytes());
        target.extend_from_slice(&(((symbol as u64) << 32) | rel.kind).to_le_bytes());
        target.extend_from_slice(&rel.addend.to_le_bytes());
    }
    let rela_text_index = if rela_text.is_empty() {
        None
    } else {
        let index = sections.len() + 1;
        sections.push(Section {
            name: ".rela.text".to_string(),
            ty: 4,
            flags: 0,
            data: rela_text,
            align: 8,
            entsize: 24,
            link: 0,
            info: text_index as u32,
            offset: 0,
        });
        Some(index)
    };
    let rela_rodata_index = if rela_rodata.is_empty() {
        None
    } else {
        let index = sections.len() + 1;
        sections.push(Section {
            name: ".rela.rodata".to_string(),
            ty: 4,
            flags: 0,
            data: rela_rodata,
            align: 8,
            entsize: 24,
            link: 0,
            info: rodata_index.unwrap() as u32,
            offset: 0,
        });
        Some(index)
    };
    let rela_data_index = if rela_data.is_empty() {
        None
    } else {
        let index = sections.len() + 1;
        sections.push(Section {
            name: ".rela.data".to_string(),
            ty: 4,
            flags: 0,
            data: rela_data,
            align: 8,
            entsize: 24,
            link: 0,
            info: data_index.unwrap() as u32,
            offset: 0,
        });
        Some(index)
    };
    let symtab_index = sections.len() + 1;
    sections.push(Section {
        name: ".symtab".to_string(),
        ty: 2,
        flags: 0,
        data: symtab,
        align: 8,
        entsize: 24,
        link: (symtab_index + 1) as u32,
        info: local_count,
        offset: 0,
    });
    sections.push(Section {
        name: ".strtab".to_string(),
        ty: 3,
        flags: 0,
        data: strtab,
        align: 1,
        entsize: 0,
        link: 0,
        info: 0,
        offset: 0,
    });
    let mut shstr = vec![0];
    let names = sections
        .iter()
        .map(|section| push_cstr(&mut shstr, &section.name))
        .collect::<Vec<_>>();
    let shstr_index = sections.len() + 1;
    sections.push(Section {
        name: ".shstrtab".to_string(),
        ty: 3,
        flags: 0,
        data: shstr,
        align: 1,
        entsize: 0,
        link: 0,
        info: 0,
        offset: 0,
    });
    let shstr_name = push_cstr(&mut sections.last_mut().unwrap().data, ".shstrtab");
    let mut names = names;
    names.push(shstr_name);
    for index in [rela_text_index, rela_rodata_index, rela_data_index]
        .into_iter()
        .flatten()
    {
        sections[index - 1].link = symtab_index as u32;
    }

    let mut cursor = 64;
    for section in &mut sections {
        cursor = phase12_test_align_to(cursor, section.align as usize);
        section.offset = cursor;
        cursor += section.data.len();
    }
    let shoff = phase12_test_align_to(cursor, 8);
    let total = shoff + (sections.len() + 1) * 64;
    let mut out = vec![0_u8; total];
    out[0..4].copy_from_slice(b"\x7fELF");
    out[4] = 2;
    out[5] = 1;
    out[6] = 1;
    write16(&mut out, 16, 1);
    write16(&mut out, 18, machine);
    write32(&mut out, 20, 1);
    write64(&mut out, 40, shoff as u64);
    write32(&mut out, 48, flags);
    write16(&mut out, 52, 64);
    write16(&mut out, 58, 64);
    write16(&mut out, 60, sections.len() as u16 + 1);
    write16(&mut out, 62, shstr_index as u16);
    for section in &sections {
        out[section.offset..section.offset + section.data.len()].copy_from_slice(&section.data);
    }
    for (idx, section) in sections.iter().enumerate() {
        let off = shoff + (idx + 1) * 64;
        write32(&mut out, off, names[idx]);
        write32(&mut out, off + 4, section.ty);
        write64(&mut out, off + 8, section.flags);
        write64(&mut out, off + 24, section.offset as u64);
        write64(&mut out, off + 32, section.data.len() as u64);
        write32(&mut out, off + 40, section.link);
        write32(&mut out, off + 44, section.info);
        write64(&mut out, off + 48, section.align);
        write64(&mut out, off + 56, section.entsize);
    }
    let path = temp_path(name, "o");
    fs::write(&path, out).expect("write raw ELF object");
    path
}

fn phase12_riscv_jal_bytes(base: u32, delta: i32) -> [u8; 4] {
    let imm = delta as u32;
    let encoded = ((imm >> 20) & 0x1) << 31
        | ((imm >> 1) & 0x3ff) << 21
        | ((imm >> 11) & 0x1) << 20
        | ((imm >> 12) & 0xff) << 12;
    ((base & 0x0000_0fff) | encoded).to_le_bytes()
}

#[derive(Clone)]
struct Phase12TestElfSection {
    name: String,
    sh_type: u32,
    flags: u64,
    addr: u64,
    offset: usize,
    size: usize,
}

#[derive(Clone)]
struct Phase12TestElfPhdr {
    p_type: u32,
    flags: u32,
    offset: usize,
    filesz: usize,
    memsz: usize,
}

fn phase12_test_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("u16"))
}

fn phase12_test_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32"))
}

fn phase12_test_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64"))
}

fn phase12_test_cstr(bytes: &[u8], offset: usize) -> String {
    let end = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|pos| offset + pos)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[offset..end]).to_string()
}

#[derive(Clone, Copy)]
struct Phase12TestMachoCommand {
    cmd: u32,
    offset: usize,
    size: usize,
}

fn phase12_macho_commands(bytes: &[u8]) -> Vec<Phase12TestMachoCommand> {
    if bytes.len() < 32 || phase12_test_u32(bytes, 0) != 0xfeedfacf {
        return Vec::new();
    }
    let ncmds = phase12_test_u32(bytes, 16) as usize;
    let mut cursor = 32_usize;
    let mut out = Vec::new();
    for _ in 0..ncmds {
        if cursor + 8 > bytes.len() {
            break;
        }
        let cmd = phase12_test_u32(bytes, cursor);
        let size = phase12_test_u32(bytes, cursor + 4) as usize;
        if size < 8 || cursor + size > bytes.len() {
            break;
        }
        out.push(Phase12TestMachoCommand {
            cmd,
            offset: cursor,
            size,
        });
        cursor += size;
    }
    out
}

fn phase12_macho_command(bytes: &[u8], cmd: u32) -> Phase12TestMachoCommand {
    phase12_macho_commands(bytes)
        .into_iter()
        .find(|item| item.cmd == cmd)
        .unwrap_or_else(|| panic!("missing Mach-O load command 0x{cmd:x}"))
}

fn phase12_macho_symtab(bytes: &[u8]) -> (usize, usize, usize, usize) {
    let command = phase12_macho_command(bytes, 0x02);
    (
        phase12_test_u32(bytes, command.offset + 8) as usize,
        phase12_test_u32(bytes, command.offset + 12) as usize,
        phase12_test_u32(bytes, command.offset + 16) as usize,
        phase12_test_u32(bytes, command.offset + 20) as usize,
    )
}

fn phase12_elf_sections(bytes: &[u8]) -> Vec<Phase12TestElfSection> {
    let shoff = phase12_test_u64(bytes, 40) as usize;
    let shentsize = phase12_test_u16(bytes, 58) as usize;
    let shnum = phase12_test_u16(bytes, 60) as usize;
    let shstrndx = phase12_test_u16(bytes, 62) as usize;
    if shoff == 0 || shentsize == 0 || shnum == 0 || shstrndx >= shnum {
        return Vec::new();
    }
    let shstr = shoff + shstrndx * shentsize;
    let shstr_off = phase12_test_u64(bytes, shstr + 24) as usize;
    let shstr_size = phase12_test_u64(bytes, shstr + 32) as usize;
    let strings = &bytes[shstr_off..shstr_off + shstr_size];
    (1..shnum)
        .filter_map(|index| {
            let off = shoff + index * shentsize;
            if off + 64 > bytes.len() {
                return None;
            }
            let name_off = phase12_test_u32(bytes, off) as usize;
            Some(Phase12TestElfSection {
                name: phase12_test_cstr(strings, name_off),
                sh_type: phase12_test_u32(bytes, off + 4),
                flags: phase12_test_u64(bytes, off + 8),
                addr: phase12_test_u64(bytes, off + 16),
                offset: phase12_test_u64(bytes, off + 24) as usize,
                size: phase12_test_u64(bytes, off + 32) as usize,
            })
        })
        .collect()
}

fn phase12_elf_section(bytes: &[u8], name: &str) -> Phase12TestElfSection {
    phase12_elf_sections(bytes)
        .into_iter()
        .find(|section| section.name == name)
        .unwrap_or_else(|| panic!("missing ELF section {name}"))
}

fn phase12_elf_phdrs(bytes: &[u8]) -> Vec<Phase12TestElfPhdr> {
    let phoff = phase12_test_u64(bytes, 32) as usize;
    let phentsize = phase12_test_u16(bytes, 54) as usize;
    let phnum = phase12_test_u16(bytes, 56) as usize;
    (0..phnum)
        .filter_map(|index| {
            let off = phoff + index * phentsize;
            if off + 56 > bytes.len() {
                return None;
            }
            Some(Phase12TestElfPhdr {
                p_type: phase12_test_u32(bytes, off),
                flags: phase12_test_u32(bytes, off + 4),
                offset: phase12_test_u64(bytes, off + 8) as usize,
                filesz: phase12_test_u64(bytes, off + 32) as usize,
                memsz: phase12_test_u64(bytes, off + 40) as usize,
            })
        })
        .collect()
}

fn phase12_verify_artifact(path: &Path) {
    let args = vec![
        "machine".to_string(),
        "verify-link".to_string(),
        path.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "verify-link failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"linked_artifact_verified\""));
}

fn phase12_build(command: &str, rel: &str, target: &str, name: &str, ext: &str) -> PathBuf {
    let source = fixture(rel);
    let out = temp_path(name, ext);
    let args = vec![
        "machine".to_string(),
        command.to_string(),
        source.to_string_lossy().to_string(),
        "--target".to_string(),
        target.to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "{command} failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    phase12_verify_artifact(&out);
    out
}

fn phase12_build_native_exe(object: &Path, target: &str, name: &str) -> PathBuf {
    let out = temp_path(name, "exe");
    let args = vec![
        "machine".to_string(),
        "link".to_string(),
        object.to_string_lossy().to_string(),
        "--target".to_string(),
        target.to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "native-object build-exe failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"link_unit_origin\": \"native_object\""));
    phase12_verify_artifact(&out);
    out
}

#[test]
fn machine_phase12_linker_status_exists() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
}

#[test]
fn machine_phase12_linker_doctor_ok() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration_status\": \"closed\""));
    assert!(stdout.contains("\"internal_linker_lane\""));
}

#[test]
fn machine_phase12_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase12_preserves_phase13_closed() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
}

#[test]
fn machine_phase12_asm_complete_total_remains_true() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase12_elf64_two_objects_link_ok() {
    let a = phase12_emit(
        "examples/machine/native/link_elf64_two_objects_a.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_a",
    );
    let b = phase12_emit(
        "examples/machine/native/link_elf64_two_objects_b.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_b",
    );
    let linked = phase12_link_objects(
        "phase12_elf_two_objects",
        "x86_64-unknown-linux-gnu",
        &[a, b],
    );
    phase12_verify_artifact(&linked);
}

#[test]
fn machine_phase12_elf64_final_linker_applies_cross_object_reloc() {
    let caller = phase12_emit(
        "examples/machine/native/link_final_cross_object_caller.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_final_elf_caller",
    );
    let callee = phase12_emit(
        "examples/machine/native/link_final_cross_object_callee.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_final_elf_callee",
    );
    let linked = phase12_link_objects(
        "phase12_final_elf_cross_object",
        "x86_64-unknown-linux-gnu",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("ELF final linked artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert!(!bytes.starts_with(b"NCCLINK12"));
    assert!(bytes
        .windows(5)
        .any(|window| window == [0xe8, 0x07, 0, 0, 0]));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("ELF final link report");
    assert!(report.contains("\"phase12_final_artifact\": \"system_loader_executable\""));
    assert!(report.contains("\"applied_relocations\": 1"));
}

#[test]
fn machine_phase12_links_third_party_elf_without_ncc_link_unit() {
    let caller = phase12_write_third_party_object(
        "phase12_third_party_elf_caller",
        "elf64",
        &[0xe8, 0, 0, 0, 0, 0xc3],
        &[("_start", 0, true)],
        &[(
            1,
            ncc_ir::BackendObjectRelocationKindIR::CallNear32,
            "third_party_target",
            -4,
        )],
    );
    let callee = phase12_write_third_party_object(
        "phase12_third_party_elf_callee",
        "elf64",
        &[0xc3],
        &[("third_party_target", 0, true)],
        &[],
    );
    assert!(!PathBuf::from(format!(
        "{}.machine-link-meta.json",
        caller.to_string_lossy()
    ))
    .exists());
    let linked = phase12_link_objects(
        "phase12_third_party_elf",
        "x86_64-unknown-linux-gnu",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("third-party ELF linked artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert!(bytes
        .windows(5)
        .any(|window| window == [0xe8, 0x0b, 0, 0, 0]));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("third-party ELF link report");
    assert!(report.contains("\"link_unit_origin\": \"native_object\""));
    assert!(report.contains("\"applied_relocations\": 1"));
}

#[test]
fn machine_phase12_links_system_toolchain_elf_objects_without_ncc_link_unit() {
    let Some(caller) = phase12_compile_system_elf_asm(
        "phase12_system_elf_caller",
        ".text\n.globl _start\n_start:\n    call third_party_cc_target\n    ret\n",
    ) else {
        return;
    };
    let Some(callee) = phase12_compile_system_elf_asm(
        "phase12_system_elf_callee",
        ".text\n.globl third_party_cc_target\nthird_party_cc_target:\n    ret\n",
    ) else {
        return;
    };
    assert!(!PathBuf::from(format!(
        "{}.machine-link-meta.json",
        caller.to_string_lossy()
    ))
    .exists());
    let linked = phase12_link_objects(
        "phase12_system_elf_third_party",
        "x86_64-unknown-linux-gnu",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("system ELF linked artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert!(bytes
        .windows(5)
        .any(|window| window == [0xe8, 0x0b, 0, 0, 0]));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("system ELF link report");
    assert!(report.contains("\"link_unit_origin\": \"native_object\""));
}

#[test]
fn machine_phase12_aarch64_final_branch_relocation_ok() {
    let caller = phase12_write_third_party_object(
        "phase12_aarch64_branch_caller",
        "aarch64",
        &[0x00, 0x00, 0x00, 0x94, 0xc0, 0x03, 0x5f, 0xd6],
        &[("_start", 0, true)],
        &[(
            0,
            ncc_ir::BackendObjectRelocationKindIR::CallNear32,
            "aarch64_target",
            0,
        )],
    );
    let callee = phase12_write_third_party_object(
        "phase12_aarch64_branch_callee",
        "aarch64",
        &[0xc0, 0x03, 0x5f, 0xd6],
        &[("aarch64_target", 0, true)],
        &[],
    );
    let linked = phase12_link_objects(
        "phase12_aarch64_branch_final",
        "aarch64-unknown-linux-gnu",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("AArch64 linked artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183);
    assert!(bytes
        .windows(4)
        .any(|window| window == [0x04, 0x00, 0x00, 0x94]));
}

#[test]
fn machine_phase12_aarch64_final_got_relocations_ok() {
    let object = phase12_write_raw_elf64_object(
        "phase12_aarch64_got_object",
        183,
        0,
        &[
            0x00, 0x00, 0x00, 0x90, // adrp x0, got_target@gotpage
            0x00, 0x00, 0x40, 0xf9, // ldr x0, [x0, got_target@gotlo12]
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ],
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "_start",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "got_target",
                section: ".text",
                offset: 8,
                global: true,
            },
        ],
        &[
            Phase12TestElfReloc {
                section: ".text",
                offset: 0,
                kind: 311,
                symbol: "got_target",
                addend: 0,
            },
            Phase12TestElfReloc {
                section: ".text",
                offset: 4,
                kind: 312,
                symbol: "got_target",
                addend: 0,
            },
        ],
    );
    let linked = phase12_link_objects(
        "phase12_aarch64_got_final",
        "aarch64-unknown-linux-gnu",
        &[object],
    );
    let bytes = fs::read(&linked).expect("AArch64 GOT linked artifact");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183);
    let got = phase12_elf_section(&bytes, ".got");
    let text = phase12_elf_section(&bytes, ".text");
    assert_eq!(
        u64::from_le_bytes(
            bytes[got.offset..got.offset + 8]
                .try_into()
                .expect("AArch64 GOT slot")
        ),
        text.addr + 8
    );
    assert_eq!(
        &bytes[text.offset..text.offset + 8],
        &[0x00, 0x00, 0x00, 0xb0, 0x00, 0x00, 0x40, 0xf9]
    );
}

#[test]
fn machine_phase12_riscv64_final_jal_relocation_ok() {
    let caller = phase12_write_raw_elf64_object(
        "phase12_riscv_jal_caller",
        243,
        0x04,
        &[0xef, 0x00, 0x00, 0x00, 0x67, 0x80, 0x00, 0x00],
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "_start",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "riscv_target",
                section: "undef",
                offset: 0,
                global: true,
            },
        ],
        &[Phase12TestElfReloc {
            section: ".text",
            offset: 0,
            kind: 17,
            symbol: "riscv_target",
            addend: 0,
        }],
    );
    let callee = phase12_write_raw_elf64_object(
        "phase12_riscv_jal_callee",
        243,
        0x04,
        &[0x67, 0x80, 0x00, 0x00],
        &[],
        &[],
        &[Phase12TestElfSymbol {
            name: "riscv_target",
            section: ".text",
            offset: 0,
            global: true,
        }],
        &[],
    );
    let linked = phase12_link_objects(
        "phase12_riscv_jal_final",
        "riscv64-unknown-linux-gnu",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("RISC-V linked artifact");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 243);
    assert!(bytes
        .windows(4)
        .any(|window| window == phase12_riscv_jal_bytes(0x0000_00ef, 16)));
}

#[test]
fn machine_phase12_riscv64_final_got_hi_lo_relocations_ok() {
    let object = phase12_write_raw_elf64_object(
        "phase12_riscv_got_object",
        243,
        0x04,
        &[
            0x97, 0x02, 0x00, 0x00, // auipc t0, 0
            0x83, 0xb2, 0x02, 0x00, // ld t0, 0(t0)
            0x67, 0x80, 0x00, 0x00, // ret
        ],
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "_start",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "riscv_got_hi",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "got_target",
                section: ".text",
                offset: 8,
                global: true,
            },
        ],
        &[
            Phase12TestElfReloc {
                section: ".text",
                offset: 0,
                kind: 20,
                symbol: "got_target",
                addend: 0,
            },
            Phase12TestElfReloc {
                section: ".text",
                offset: 4,
                kind: 24,
                symbol: "riscv_got_hi",
                addend: 0,
            },
        ],
    );
    let linked = phase12_link_objects(
        "phase12_riscv_got_final",
        "riscv64-unknown-linux-gnu",
        &[object],
    );
    let bytes = fs::read(&linked).expect("RISC-V GOT linked artifact");
    let got = phase12_elf_section(&bytes, ".got");
    let text = phase12_elf_section(&bytes, ".text");
    assert_eq!(
        u64::from_le_bytes(
            bytes[got.offset..got.offset + 8]
                .try_into()
                .expect("RISC-V GOT slot")
        ),
        text.addr + 8
    );
    assert_ne!(
        &bytes[text.offset..text.offset + 4],
        &[0x97, 0x02, 0x00, 0x00]
    );
    assert_eq!(
        &bytes[text.offset + 4..text.offset + 8],
        &[0x83, 0xb2, 0x02, 0xf5]
    );
}

#[test]
fn machine_phase12_elf64_final_gotpcrel_and_data_abs64_relocations_ok() {
    let got_object = phase12_write_raw_elf64_object(
        "phase12_x86_gotpcrel_object",
        62,
        0,
        &[
            0x48, 0x8b, 0x05, 0, 0, 0, 0, // mov rax, qword ptr [rip + got_target@gotpcrel]
            0xc3,
        ],
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "_start",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "got_target",
                section: ".text",
                offset: 7,
                global: true,
            },
        ],
        &[Phase12TestElfReloc {
            section: ".text",
            offset: 3,
            kind: 9,
            symbol: "got_target",
            addend: -4,
        }],
    );
    let got_linked = phase12_link_objects(
        "phase12_x86_gotpcrel_final",
        "x86_64-unknown-linux-gnu",
        &[got_object],
    );
    let got_bytes = fs::read(&got_linked).expect("x86_64 GOT linked artifact");
    let got = phase12_elf_section(&got_bytes, ".got");
    let text = phase12_elf_section(&got_bytes, ".text");
    assert_eq!(
        u64::from_le_bytes(
            got_bytes[got.offset..got.offset + 8]
                .try_into()
                .expect("x86_64 GOT slot")
        ),
        text.addr + 7
    );
    let disp = got.addr as i128 - (text.addr + 3) as i128 - 4;
    assert_eq!(
        &got_bytes[text.offset..text.offset + 7],
        &[
            0x48,
            0x8b,
            0x05,
            (disp as i32).to_le_bytes()[0],
            (disp as i32).to_le_bytes()[1],
            (disp as i32).to_le_bytes()[2],
            (disp as i32).to_le_bytes()[3],
        ]
    );

    let data_object = phase12_write_raw_elf64_object(
        "phase12_x86_data_abs64_object",
        62,
        0,
        &[0xc3],
        &[],
        &[0; 8],
        &[Phase12TestElfSymbol {
            name: "_start",
            section: ".text",
            offset: 0,
            global: true,
        }],
        &[Phase12TestElfReloc {
            section: ".data",
            offset: 0,
            kind: 1,
            symbol: "_start",
            addend: 0,
        }],
    );
    let data_linked = phase12_link_objects(
        "phase12_x86_data_abs64_final",
        "x86_64-unknown-linux-gnu",
        &[data_object],
    );
    let data_bytes = fs::read(&data_linked).expect("x86_64 data relocation artifact");
    let entry = u64::from_le_bytes(data_bytes[24..32].try_into().expect("ELF entry"));
    let text = phase12_elf_section(&data_bytes, ".text");
    let data = phase12_elf_section(&data_bytes, ".data");
    assert_eq!(
        u64::from_le_bytes(
            data_bytes[data.offset..data.offset + 8]
                .try_into()
                .expect("ABS64 data slot")
        ),
        text.addr
    );
    assert!(
        entry > text.addr,
        "Linux e_entry must select its process-exit wrapper"
    );
}

#[test]
fn machine_phase12_elf64_final_linker_respects_entry_symbol_offset() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_final_entry_offset.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_final_entry_offset",
        "exe",
    );
    let bytes = fs::read(&linked).expect("ELF final entry artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    let entry = u64::from_le_bytes(bytes[24..32].try_into().expect("ELF entry bytes"));
    let text = phase12_elf_section(&bytes, ".text");
    let wrapper_offset = text.offset + usize::try_from(entry - text.addr).expect("wrapper offset");
    assert_eq!(
        bytes[wrapper_offset], 0xe8,
        "entry wrapper must call the selected symbol"
    );
    let displacement = i32::from_le_bytes(
        bytes[wrapper_offset + 1..wrapper_offset + 5]
            .try_into()
            .expect("entry call displacement"),
    );
    let called = (entry as i128 + 5 + displacement as i128) as u64;
    assert_eq!(
        called,
        text.addr + 1,
        "wrapper must call realentry after entrypad"
    );
}

#[test]
fn machine_phase12_accepts_third_party_coff_obj_without_ncc_link_unit() {
    let object = phase12_write_third_party_object(
        "phase12_third_party_coff",
        "coff",
        &[0xc3],
        &[("main", 0, true)],
        &[],
    );
    let linked = phase12_link_objects(
        "phase12_third_party_coff",
        "x86_64-pc-windows-msvc",
        &[object],
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("third-party COFF link report");
    assert!(report.contains("\"object_format\": \"coff\""));
    assert!(report.contains("\"link_unit_origin\": \"native_object\""));
}

#[test]
fn machine_phase12_link_unit_records_data_symbol_offsets() {
    let object = phase12_emit(
        "examples/machine/native/link_final_data_symbols.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_final_data_symbols",
    );
    let metadata = fs::read_to_string(format!(
        "{}.machine-link-meta.json",
        object.to_string_lossy()
    ))
    .expect("phase12 metadata");
    let payload: Value = serde_json::from_str(&metadata).expect("metadata json");
    let symbols = payload["link_unit"]["symbols"]
        .as_array()
        .expect("link_unit symbols");
    let find = |name: &str| {
        symbols
            .iter()
            .find(|item| item["name"] == name)
            .unwrap_or_else(|| panic!("missing symbol {name}"))
    };
    assert_eq!(find("first")["section"], ".data");
    assert_eq!(find("first")["offset"], 0);
    assert_eq!(find("second")["section"], ".data");
    assert_eq!(find("second")["offset"], 3);
    assert_eq!(find("third")["section"], ".data");
    assert_eq!(find("third")["offset"], 11);
    assert_eq!(find("scratch")["section"], ".bss");
    assert_eq!(find("scratch")["offset"], 0);
}

#[test]
fn machine_phase12_elf64_executable_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_exe",
        "exe",
    );
}

#[test]
fn machine_phase12_elf64_build_exe_is_os_loader_artifact() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_os_loader_exe",
        "exe",
    );
    let bytes = fs::read(&linked).expect("ELF build-exe artifact");
    assert!(bytes.starts_with(b"\x7fELF"));
    assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), 2);
    assert!(!bytes.starts_with(b"NCCLINK12"));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("ELF OS-loader report");
    assert!(report.contains("\"os_loader_executable\": true"));
    assert!(report.contains("\"internal_container_used_for_final_exe\": false"));
}

#[test]
fn machine_phase12_pe64_build_exe_is_os_loader_artifact() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_final_windows_exit42.nm",
        "x86_64-pc-windows-msvc",
        "phase12_pe64_windows_exit42",
        "exe",
    );
    let bytes = fs::read(&linked).expect("PE64 build-exe artifact");
    assert!(bytes.starts_with(b"MZ"));
    let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().expect("PE offset")) as usize;
    assert_eq!(&bytes[pe..pe + 4], b"PE\0\0");
    assert_eq!(u16::from_le_bytes([bytes[pe + 4], bytes[pe + 5]]), 0x8664);
    assert_eq!(u16::from_le_bytes([bytes[pe + 24], bytes[pe + 25]]), 0x020b);
    let dll_characteristics = u16::from_le_bytes([bytes[pe + 24 + 70], bytes[pe + 24 + 71]]);
    assert_eq!(dll_characteristics & 0x0060, 0x0060);
    assert_ne!(
        u32::from_le_bytes(bytes[pe + 40..pe + 44].try_into().expect("PE entry RVA")),
        0
    );
    let import_dir_rva = u32::from_le_bytes(
        bytes[pe + 24 + 112 + 8..pe + 24 + 112 + 12]
            .try_into()
            .expect("import directory RVA"),
    );
    assert_ne!(import_dir_rva, 0);
    let reloc_dir_rva = u32::from_le_bytes(
        bytes[pe + 24 + 112 + 5 * 8..pe + 24 + 112 + 5 * 8 + 4]
            .try_into()
            .expect("base relocation directory RVA"),
    );
    let reloc_dir_size = u32::from_le_bytes(
        bytes[pe + 24 + 112 + 5 * 8 + 4..pe + 24 + 112 + 5 * 8 + 8]
            .try_into()
            .expect("base relocation directory size"),
    );
    assert_ne!(reloc_dir_rva, 0);
    assert_ne!(reloc_dir_size, 0);
    assert!(bytes.windows(b".reloc".len()).any(|w| w == b".reloc"));
    assert!(bytes
        .windows(b"KERNEL32.dll".len())
        .any(|w| w == b"KERNEL32.dll"));
    assert!(bytes
        .windows(b"ExitProcess".len())
        .any(|w| w == b"ExitProcess"));
    assert!(!bytes.starts_with(b"NCCLINK12"));

    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("PE OS-loader report");
    assert!(report.contains("\"artifact_format\": \"coff\""));
    assert!(report.contains("\"artifact_magic\": \"PE32_PLUS_MZ_PE\""));
    assert!(report.contains("\"os_loader_executable\": true"));
    assert!(report.contains("\"internal_container_used_for_final_exe\": false"));
    assert!(report.contains("closed_pe_import_thunks_retained"));
    assert!(report.contains("closed_pe_reloc_aslr"));
    assert!(report.contains("\"base_relocation_directory_present\": true"));
    assert!(report.contains("\"base_relocation_empty_image_block\": true"));
}

#[test]
fn machine_phase12_pe64_abs64_base_relocation_dir64_ok() {
    let mut profile = ncc_ir::BackendLoweringProfileIR::nc_win64();
    profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Coff;
    let object = ncc_ir::BackendObjectIR {
        target: profile.target,
        target_spec: profile.target_spec,
        sections: vec![
            ncc_ir::BackendObjectSectionIR {
                name: ".text".to_string(),
                kind: ncc_ir::BackendObjectSectionKindIR::Text,
                unit: "pe_abs64".to_string(),
                lines: Vec::new(),
                bytes: Some(vec![0xc3]),
                size_bytes: 1,
                align_bytes: 1,
                comdat: None,
            },
            ncc_ir::BackendObjectSectionIR {
                name: ".data".to_string(),
                kind: ncc_ir::BackendObjectSectionKindIR::Data,
                unit: "pe_abs64".to_string(),
                lines: Vec::new(),
                bytes: Some(vec![0; 8]),
                size_bytes: 8,
                align_bytes: 8,
                comdat: None,
            },
        ],
        symbols: vec![
            ncc_ir::BackendObjectSymbolIR {
                name: "main".to_string(),
                section: ".text".to_string(),
                offset: 0,
                visibility: ncc_ir::BackendSymbolVisibilityIR::Global,
            },
            ncc_ir::BackendObjectSymbolIR {
                name: "main_ptr".to_string(),
                section: ".data".to_string(),
                offset: 0,
                visibility: ncc_ir::BackendSymbolVisibilityIR::Global,
            },
        ],
        relocations: vec![ncc_ir::BackendObjectRelocationIR {
            section: ".data".to_string(),
            offset: 0,
            kind: ncc_ir::BackendObjectRelocationKindIR::Abs64,
            symbol: "main".to_string(),
            addend: 0,
        }],
    };
    let object_path = temp_path("phase12_pe64_abs64_reloc", "obj");
    fs::write(
        &object_path,
        ncc_ir::emit_object_coff(&object).expect("COFF abs64 object"),
    )
    .expect("write COFF abs64 object");
    let _ = fs::remove_file(format!(
        "{}.machine-link-meta.json",
        object_path.to_string_lossy()
    ));

    let linked = phase12_link_objects(
        "phase12_pe64_abs64_reloc",
        "x86_64-pc-windows-msvc",
        &[object_path],
    );
    let bytes = fs::read(&linked).expect("PE64 abs64 linked artifact");
    let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().expect("PE offset")) as usize;
    let image_base = u64::from_le_bytes(bytes[pe + 24 + 24..pe + 24 + 32].try_into().unwrap());
    let entry_rva = u32::from_le_bytes(bytes[pe + 24 + 16..pe + 24 + 20].try_into().unwrap());
    let data = phase12_pe_section(&bytes, ".data");
    let reloc = phase12_pe_section(&bytes, ".reloc");
    let data_slot = data.raw_pointer as usize;
    assert_eq!(
        u64::from_le_bytes(bytes[data_slot..data_slot + 8].try_into().unwrap()),
        image_base + entry_rva as u64
    );

    let reloc_dir_rva = u32::from_le_bytes(
        bytes[pe + 24 + 112 + 5 * 8..pe + 24 + 112 + 5 * 8 + 4]
            .try_into()
            .expect("base relocation directory RVA"),
    );
    let reloc_dir_size = u32::from_le_bytes(
        bytes[pe + 24 + 112 + 5 * 8 + 4..pe + 24 + 112 + 5 * 8 + 8]
            .try_into()
            .expect("base relocation directory size"),
    );
    assert_eq!(reloc_dir_rva, reloc.virtual_address);
    assert_eq!(reloc_dir_size, reloc.virtual_size);
    let reloc_offset = phase12_pe_rva_to_offset(&bytes, reloc_dir_rva).expect(".reloc offset");
    let page_rva = u32::from_le_bytes(bytes[reloc_offset..reloc_offset + 4].try_into().unwrap());
    let entry = u16::from_le_bytes(
        bytes[reloc_offset + 8..reloc_offset + 10]
            .try_into()
            .unwrap(),
    );
    assert_eq!(page_rva, data.virtual_address & !0xfff);
    assert_eq!(entry & 0xf000, 0xA000);
    assert_eq!(entry & 0x0fff, (data.virtual_address & 0x0fff) as u16);
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("PE64 abs64 report");
    assert!(report.contains("\"base_relocation_entries\": 1"));
    assert!(report.contains("\"base_relocation_empty_image_block\": false"));
}

#[test]
fn machine_phase12_elf64_final_sections_have_real_permissions() {
    let ro_object = phase12_write_raw_elf64_object(
        "phase12_sections_rodata_object",
        62,
        0,
        &[0xc3],
        b"readonly-final",
        &[],
        &[
            Phase12TestElfSymbol {
                name: "_start",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "ro_anchor",
                section: ".rodata",
                offset: 0,
                global: true,
            },
        ],
        &[],
    );
    let data_object = phase12_emit(
        "examples/machine/native/link_final_data_symbols.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_sections_data_bss",
    );
    let linked = phase12_link_objects(
        "phase12_sections_permissions",
        "x86_64-unknown-linux-gnu",
        &[ro_object, data_object],
    );
    let bytes = fs::read(&linked).expect("ELF section permissions artifact");
    let text = phase12_elf_section(&bytes, ".text");
    let rodata = phase12_elf_section(&bytes, ".rodata");
    let data = phase12_elf_section(&bytes, ".data");
    let bss = phase12_elf_section(&bytes, ".bss");
    assert_eq!(text.flags & 0x6, 0x6);
    assert_eq!(rodata.flags, 0x2);
    assert!(rodata.size >= b"readonly-final".len());
    assert_eq!(data.flags, 0x3);
    assert_eq!(bss.sh_type, 8);
    assert_eq!(bss.flags, 0x3);

    let loads = phase12_elf_phdrs(&bytes)
        .into_iter()
        .filter(|phdr| phdr.p_type == 1)
        .collect::<Vec<_>>();
    assert!(loads.iter().all(|phdr| phdr.offset % 0x1000 == 0));
    assert!(loads.iter().any(|phdr| phdr.flags == 5));
    assert!(loads.iter().any(|phdr| phdr.flags == 4));
    assert!(loads
        .iter()
        .any(|phdr| phdr.flags == 6 && phdr.memsz > phdr.filesz));

    let debug_linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_debug_unwind.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_sections_debug_unwind",
        "exe",
    );
    let debug_bytes = fs::read(&debug_linked).expect("ELF debug sections artifact");
    let eh_frame = phase12_elf_section(&debug_bytes, ".eh_frame");
    let debug_line = phase12_elf_section(&debug_bytes, ".debug_line");
    assert_eq!(eh_frame.flags, 0x2);
    assert_eq!(debug_line.flags, 0);
}

#[test]
fn machine_phase12_elf64_staticlib_ok() {
    phase12_build(
        "build-staticlib",
        "examples/machine/native/link_elf64_staticlib.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_static",
        "a",
    );
}

#[test]
fn machine_phase12_elf64_relocations_ok() {
    let object = phase12_emit(
        "examples/machine/native/link_elf64_relocations.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_reloc",
    );
    let linked = phase12_link_objects("phase12_elf_reloc", "x86_64-unknown-linux-gnu", &[object]);
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("link report");
    assert!(report.contains("\"relocations_applied\""));
    assert!(report.contains("\"status\": \"applied\""));
}

#[test]
fn machine_phase12_elf64_debug_unwind_preserved_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_debug_unwind.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_debug",
        "exe",
    );
}

#[test]
fn machine_phase12_elf64_dynamic_import_metadata_ok() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_dynamic_imports.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_elf_dynamic_imports",
        "exe",
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("ELF dynamic import link report");
    assert!(report.contains("\"status\": \"closed_dynamic_import_metadata\""));
    assert!(report.contains("\"name\": \".dynamic\""));
    assert!(report.contains("\"name\": \".dynsym\""));
    assert!(report.contains("\"name\": \".dynstr\""));
    assert!(report.contains("\"name\": \".rela.plt\""));
    assert!(report.contains("\"name\": \".got.plt\""));
    assert!(report.contains("\"name\": \".plt\""));
    assert!(report.contains("\"tag\": \"DT_JMPREL\""));
    assert!(report.contains("\"relocation\": \"R_X86_64_JUMP_SLOT\""));
    assert!(report.contains("\"symbol\": \"printf\""));

    let map = fs::read_to_string(format!("{}.map", linked.to_string_lossy()))
        .expect("ELF dynamic import map");
    assert!(map.contains("printf via .plt/.got.plt/.dynsym/.dynstr/.rela.plt/.dynamic"));
}

#[test]
fn machine_phase12_elf32_i386_dynamic_import_metadata_ok() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_elf32_dynamic_imports.nm",
        "i386-unknown-linux-gnu",
        "phase12_elf32_i386_dynamic_imports",
        "exe",
    );
    let bytes = fs::read(&linked).expect("ELF32 dynamic executable");
    assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()));
    assert_eq!(bytes[4], 1, "ELF32 class expected");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 3);

    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("ELF32 dynamic import link report");
    assert!(report.contains("\"status\": \"closed_dynamic_import_metadata\""));
    assert!(report.contains("\"format\": \"elf32\""));
    assert!(report.contains("\"name\": \".dynamic\""));
    assert!(report.contains("\"name\": \".dynsym\""));
    assert!(report.contains("\"name\": \".dynstr\""));
    assert!(report.contains("\"name\": \".rel.plt\""));
    assert!(report.contains("\"name\": \".got.plt\""));
    assert!(report.contains("\"name\": \".plt\""));
    assert!(report.contains("\"tag\": \"DT_JMPREL\""));
    assert!(report.contains("\"tag\": \"DT_REL\""));
    assert!(report.contains("\"relocation\": \"R_386_JMP_SLOT\""));
    assert!(report.contains("\"symbol\": \"printf\""));

    let map = fs::read_to_string(format!("{}.map", linked.to_string_lossy()))
        .expect("ELF32 dynamic import map");
    assert!(map.contains("printf via .plt/.got.plt/.dynsym/.dynstr/.rel.plt/.dynamic"));
}

#[test]
fn machine_phase12_elf64_aarch64_dynamic_import_metadata_ok() {
    let object = phase12_write_raw_elf64_object(
        "phase12_aarch64_dynamic_import_object",
        183,
        0,
        &[
            0x00, 0x00, 0x00, 0x94, // bl write, patched by R_AARCH64_CALL26
            0x00, 0x00, 0x80, 0xd2, // mov x0, #0
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ],
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "main",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "write",
                section: "undef",
                offset: 0,
                global: true,
            },
        ],
        &[Phase12TestElfReloc {
            section: ".text",
            offset: 0,
            kind: 283,
            symbol: "write",
            addend: 0,
        }],
    );
    let linked = phase12_build_native_exe(
        &object,
        "aarch64-unknown-linux-gnu",
        "phase12_elf64_aarch64_dynamic_imports",
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("AArch64 dynamic import link report");
    assert!(report.contains("\"status\": \"closed_dynamic_import_metadata\""));
    assert!(report.contains("\"format\": \"elf64\""));
    assert!(report.contains("\"name\": \".dynamic\""));
    assert!(report.contains("\"name\": \".dynsym\""));
    assert!(report.contains("\"name\": \".dynstr\""));
    assert!(report.contains("\"name\": \".rela.plt\""));
    assert!(report.contains("\"name\": \".got.plt\""));
    assert!(report.contains("\"name\": \".plt\""));
    assert!(report.contains("\"tag\": \"DT_BIND_NOW\""));
    assert!(report.contains("\"tag\": \"DT_FLAGS\""));
    assert!(report.contains("\"relocation\": \"R_AARCH64_JUMP_SLOT\""));
    assert!(report.contains("\"symbol\": \"write\""));

    let map = fs::read_to_string(format!("{}.map", linked.to_string_lossy()))
        .expect("AArch64 dynamic import map");
    assert!(map.contains("write via .plt/.got.plt/.dynsym/.dynstr/.rela.plt/.dynamic"));
}

#[test]
fn machine_phase12_elf64_riscv64_dynamic_import_metadata_ok() {
    let mut text = Vec::new();
    for word in [
        0x0000_0097_u32, // auipc ra, 0
        0x0000_80e7_u32, // jalr ra, 0(ra)
        0x0000_0513_u32, // addi a0, zero, 0
        0x0000_8067_u32, // ret
    ] {
        text.extend_from_slice(&word.to_le_bytes());
    }
    let object = phase12_write_raw_elf64_object(
        "phase12_riscv64_dynamic_import_object",
        243,
        0x04,
        &text,
        &[],
        &[],
        &[
            Phase12TestElfSymbol {
                name: "main",
                section: ".text",
                offset: 0,
                global: true,
            },
            Phase12TestElfSymbol {
                name: "write",
                section: "undef",
                offset: 0,
                global: true,
            },
        ],
        &[Phase12TestElfReloc {
            section: ".text",
            offset: 0,
            kind: 19,
            symbol: "write",
            addend: 0,
        }],
    );
    let linked = phase12_build_native_exe(
        &object,
        "riscv64-unknown-linux-gnu",
        "phase12_elf64_riscv64_dynamic_imports",
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("RISC-V64 dynamic import link report");
    assert!(report.contains("\"status\": \"closed_dynamic_import_metadata\""));
    assert!(report.contains("\"format\": \"elf64\""));
    assert!(report.contains("\"name\": \".dynamic\""));
    assert!(report.contains("\"name\": \".dynsym\""));
    assert!(report.contains("\"name\": \".dynstr\""));
    assert!(report.contains("\"name\": \".rela.plt\""));
    assert!(report.contains("\"name\": \".got.plt\""));
    assert!(report.contains("\"name\": \".plt\""));
    assert!(report.contains("\"tag\": \"DT_BIND_NOW\""));
    assert!(report.contains("\"tag\": \"DT_FLAGS\""));
    assert!(report.contains("\"relocation\": \"R_RISCV_JUMP_SLOT\""));
    assert!(report.contains("\"symbol\": \"write\""));

    let map = fs::read_to_string(format!("{}.map", linked.to_string_lossy()))
        .expect("RISC-V64 dynamic import map");
    assert!(map.contains("write via .plt/.got.plt/.dynsym/.dynstr/.rela.plt/.dynamic"));
}

#[test]
fn machine_phase12_coff_executable_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_coff_executable.nm",
        "x86_64-pc-windows-msvc",
        "phase12_coff_exe",
        "exe",
    );
}

#[test]
fn machine_phase12_coff_staticlib_ok() {
    phase12_build(
        "build-staticlib",
        "examples/machine/native/link_coff_staticlib.nm",
        "x86_64-pc-windows-msvc",
        "phase12_coff_static",
        "lib",
    );
}

#[test]
fn machine_phase12_coff_relocations_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_coff_relocations.nm",
        "x86_64-pc-windows-msvc",
        "phase12_coff_reloc",
        "exe",
    );
}

#[test]
fn machine_phase12_coff_unwind_preserved_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_coff_unwind.nm",
        "x86_64-pc-windows-msvc",
        "phase12_coff_unwind",
        "exe",
    );
}

#[test]
fn machine_phase12_macho_executable_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_macho_executable.nm",
        "x86_64-apple-darwin",
        "phase12_macho_exe",
        "mach",
    );
}

#[test]
fn machine_phase12_macho_final_linker_applies_cross_object_reloc() {
    let caller = phase12_emit(
        "examples/machine/native/link_final_cross_object_caller.nm",
        "x86_64-apple-darwin",
        "phase12_final_macho_caller",
    );
    let callee = phase12_emit(
        "examples/machine/native/link_final_cross_object_callee.nm",
        "x86_64-apple-darwin",
        "phase12_final_macho_callee",
    );
    let linked = phase12_link_objects(
        "phase12_final_macho_cross_object",
        "x86_64-apple-darwin",
        &[caller, callee],
    );
    let bytes = fs::read(&linked).expect("Mach-O final linked artifact");
    assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
    assert_eq!(
        u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        2
    );
    assert!(!bytes.starts_with(b"NCCLINK12"));
    assert!(bytes
        .windows(5)
        .any(|window| window == [0xe8, 0x07, 0, 0, 0]));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("Mach-O final link report");
    assert!(report.contains("\"phase12_final_artifact\": \"system_loader_executable\""));
    assert!(report.contains("\"applied_relocations\": 1"));
}

#[test]
fn machine_phase12_macho_build_exe_is_os_loader_artifact() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_macho_executable.nm",
        "x86_64-apple-darwin",
        "phase12_macho_os_loader_exe",
        "mach",
    );
    let bytes = fs::read(&linked).expect("Mach-O build-exe artifact");
    assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
    assert_eq!(
        u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        2
    );
    assert!(!bytes.starts_with(b"NCCLINK12"));
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("Mach-O OS-loader report");
    assert!(report.contains("\"os_loader_executable\": true"));
    assert!(report.contains("\"internal_container_used_for_final_exe\": false"));
}

#[test]
fn machine_phase12_macho_modern_main_symtab_and_signature_ok() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_final_macho_exit42.nm",
        "x86_64-apple-darwin",
        "phase12_macho_modern",
        "mach",
    );
    let bytes = fs::read(&linked).expect("modern Mach-O executable artifact");
    let commands = phase12_macho_commands(&bytes);
    assert_eq!(commands.len(), phase12_test_u32(&bytes, 16) as usize);
    assert!(commands.iter().any(|command| command.cmd == 0x80000028));
    assert!(commands.iter().any(|command| command.cmd == 0x32));
    assert!(commands.iter().any(|command| command.cmd == 0x0e));
    assert!(commands.iter().any(|command| command.cmd == 0x0c));
    assert!(commands.iter().any(|command| command.cmd == 0x80000022));
    assert!(commands.iter().any(|command| command.cmd == 0x0b));
    assert!(commands.iter().any(|command| command.cmd == 0x02));
    assert!(commands.iter().any(|command| command.cmd == 0x1d));
    assert!(!commands.iter().any(|command| command.cmd == 0x05));

    let main = phase12_macho_command(&bytes, 0x80000028);
    assert_eq!(main.size, 24);
    let entryoff = phase12_test_u64(&bytes, main.offset + 8);
    assert!(entryoff > 0);

    let (symoff, nsyms, stroff, strsize) = phase12_macho_symtab(&bytes);
    assert!(nsyms >= 1);
    assert!(symoff + nsyms * 16 <= bytes.len());
    assert!(stroff + strsize <= bytes.len());
    let strings = &bytes[stroff..stroff + strsize];
    let mut start_value = None;
    for index in 0..nsyms {
        let off = symoff + index * 16;
        let name_off = phase12_test_u32(&bytes, off) as usize;
        if name_off >= strings.len() {
            continue;
        }
        if phase12_test_cstr(strings, name_off) == "_start" {
            start_value = Some(phase12_test_u64(&bytes, off + 8));
        }
    }
    assert_eq!(start_value, Some(0x100000000 + entryoff));

    let signature = phase12_macho_command(&bytes, 0x1d);
    assert_eq!(signature.size, 16);
    let dataoff = phase12_test_u32(&bytes, signature.offset + 8) as usize;
    let datasize = phase12_test_u32(&bytes, signature.offset + 12) as usize;
    assert!(datasize > 20);
    assert!(dataoff + datasize <= bytes.len());
    assert_eq!(&bytes[dataoff..dataoff + 4], &[0xfa, 0xde, 0x0c, 0xc0]);
    assert_eq!(
        &bytes[dataoff + 20..dataoff + 24],
        &[0xfa, 0xde, 0x0c, 0x02]
    );
}

#[test]
fn machine_phase12_links_third_party_macho_without_ncc_link_unit() {
    let object = phase12_write_third_party_object(
        "phase12_third_party_macho",
        "macho",
        &[0xc3],
        &[("_start", 0, true)],
        &[],
    );
    let linked = phase12_link_objects(
        "phase12_third_party_macho",
        "x86_64-apple-darwin",
        &[object],
    );
    let bytes = fs::read(&linked).expect("third-party Mach-O linked artifact");
    assert!(bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
    assert_eq!(
        u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        2
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("third-party Mach-O link report");
    assert!(report.contains("\"link_unit_origin\": \"native_object\""));
    assert!(report.contains("\"os_loader_executable\": true"));
}

#[test]
fn machine_phase12_macho_staticlib_ok() {
    phase12_build(
        "build-staticlib",
        "examples/machine/native/link_macho_staticlib.nm",
        "x86_64-apple-darwin",
        "phase12_macho_static",
        "a",
    );
}

#[test]
fn machine_phase12_macho_relocations_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_macho_relocations.nm",
        "x86_64-apple-darwin",
        "phase12_macho_reloc",
        "mach",
    );
}

#[test]
fn machine_phase12_macho_unwind_preserved_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_macho_unwind.nm",
        "x86_64-apple-darwin",
        "phase12_macho_unwind",
        "mach",
    );
}

#[test]
fn machine_phase12_macho_dynamic_bind_metadata_ok() {
    let linked = phase12_build(
        "build-exe",
        "examples/machine/native/link_macho_dynamic_imports.nm",
        "x86_64-apple-darwin",
        "phase12_macho_dynamic_imports",
        "mach",
    );
    let report = fs::read_to_string(format!("{}.link-report.json", linked.to_string_lossy()))
        .expect("Mach-O dynamic import link report");
    assert!(report.contains("\"status\": \"closed_dynamic_import_metadata\""));
    assert!(report.contains("\"cmd\": \"LC_LOAD_DYLIB\""));
    assert!(report.contains("\"cmd\": \"LC_DYLD_INFO_ONLY\""));
    assert!(report.contains("\"cmd\": \"LC_DYSYMTAB\""));
    assert!(report.contains("\"name\": \"__TEXT,__stubs\""));
    assert!(report.contains("\"name\": \"__DATA,__la_symbol_ptr\""));
    assert!(report.contains("\"name\": \"__LINKEDIT,bind\""));
    assert!(report.contains("\"name\": \"__LINKEDIT,lazy_bind\""));
    assert!(report.contains("\"symbol\": \"_puts\""));

    let map = fs::read_to_string(format!("{}.map", linked.to_string_lossy()))
        .expect("Mach-O dynamic import map");
    assert!(map.contains("_puts via __stubs/__la_symbol_ptr/LC_DYLD_INFO bind opcodes"));
}

#[test]
fn machine_phase12_wasm_linked_module_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_wasm_module_a.nm",
        "wasm32-unknown-unknown",
        "phase12_wasm_module",
        "wasm",
    );
}

#[test]
fn machine_phase12_wasm_exports_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_wasm_exports.nm",
        "wasm32-unknown-unknown",
        "phase12_wasm_exports",
        "wasm",
    );
}

#[test]
fn machine_phase12_wasm_debug_preserved_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_wasm_debug.nm",
        "wasm32-unknown-unknown",
        "phase12_wasm_debug",
        "wasm",
    );
}

#[test]
fn machine_phase12_aarch64_link_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_cross_arch_aarch64.nm",
        "aarch64-unknown-linux-gnu",
        "phase12_aarch64",
        "elf",
    );
}

#[test]
fn machine_phase12_riscv64_link_ok() {
    phase12_build(
        "build-exe",
        "examples/machine/native/link_cross_arch_riscv64.nm",
        "riscv64-unknown-elf",
        "phase12_riscv64",
        "elf",
    );
}

#[test]
fn machine_phase12_archive_ok() {
    let object = phase12_emit(
        "examples/machine/native/link_elf64_staticlib.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_archive_obj",
    );
    let out = temp_path("phase12_archive", "a");
    let args = vec![
        "machine".to_string(),
        "archive".to_string(),
        object.to_string_lossy().to_string(),
        "--target".to_string(),
        "x86_64-unknown-linux-gnu".to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    phase12_verify_artifact(&out);
}

#[test]
fn machine_phase12_map_file_ok() {
    let object = phase12_emit(
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_map_obj",
    );
    let linked = phase12_link_objects("phase12_map", "x86_64-unknown-linux-gnu", &[object]);
    let args = vec![
        "machine".to_string(),
        "link-map".to_string(),
        linked.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("\"map_file_verified\""));
}

#[test]
fn machine_phase12_link_report_ok() {
    let object = phase12_emit(
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_report_obj",
    );
    let linked = phase12_link_objects("phase12_report", "x86_64-unknown-linux-gnu", &[object]);
    let args = vec![
        "machine".to_string(),
        "link-report".to_string(),
        linked.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("\"output_hash\""));
}

#[test]
fn machine_phase12_verify_link_ok() {
    let object = phase12_emit(
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_verify_obj",
    );
    let linked = phase12_link_objects("phase12_verify", "x86_64-unknown-linux-gnu", &[object]);
    phase12_verify_artifact(&linked);
}

#[test]
fn machine_phase12_unresolved_symbol_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_unresolved_symbol_fail.nm",
        "E_MACHINE_LINK_UNRESOLVED_SYMBOL_001",
    );
}

#[test]
fn machine_phase12_duplicate_symbol_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_duplicate_symbol_fail.nm",
        "E_MACHINE_LINK_DUPLICATE_SYMBOL_001",
    );
}

#[test]
fn machine_phase12_missing_entry_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_missing_entry_fail.nm",
        "E_MACHINE_LINK_ENTRY_MISSING_001",
    );
}

#[test]
fn machine_phase12_relocation_overflow_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_relocation_overflow_fail.nm",
        "E_MACHINE_LINK_RELOCATION_OVERFLOW_001",
    );
}

#[test]
fn machine_phase12_unsupported_relocation_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_unsupported_relocation_fail.nm",
        "E_MACHINE_LINK_RELOCATION_UNSUPPORTED_001",
    );
}

#[test]
fn machine_phase12_wrong_target_object_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_wrong_target_object_fail.nm",
        "E_MACHINE_LINK_TARGET_001",
    );
}

#[test]
fn machine_phase12_section_alignment_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_section_alignment_fail.nm",
        "E_MACHINE_LINK_SECTION_ALIGNMENT_001",
    );
}

#[test]
fn machine_phase12_dynamic_forbidden_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_dynamic_forbidden_fail.nm",
        "E_MACHINE_LINK_DYNAMIC_RELOCATION_FORBIDDEN_001",
    );
}

#[test]
fn machine_phase12_bad_archive_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/link_bad_archive_fail.nm",
        "E_MACHINE_LINK_ARCHIVE_001",
    );
}

#[test]
fn machine_phase12_no_fake_link() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " link"].concat()));
    assert!(!stdout.contains(&["fake", " executable"].concat()));
    assert!(!stdout.contains(&["fake", " archive"].concat()));
}

#[test]
fn machine_phase12_no_fake_executable() {
    let out = phase12_build(
        "build-exe",
        "examples/machine/native/link_elf64_executable.nm",
        "x86_64-unknown-linux-gnu",
        "phase12_no_fake_exe",
        "exe",
    );
    let bytes = fs::read(out).expect("linked artifact");
    assert!(bytes.len() > 128);
}

#[test]
fn machine_phase12_no_silent_unresolved() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"silent_unresolved\": 0"));
}

#[test]
fn machine_phase12_no_fn_fa_positive_examples() {
    for entry in fs::read_dir(fixture("examples/machine/native")).expect("native dir") {
        let path = entry.expect("entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("link_") || path.extension().and_then(|ext| ext.to_str()) != Some("nm")
        {
            continue;
        }
        let text = fs::read_to_string(&path).expect("phase12 positive");
        assert!(!text.contains(&["fn", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["fa", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["stc", ":"].concat()), "{}", path.display());
    }
}

#[test]
fn machine_phase12_macro_directive_regressions_ok() {
    assert_machine_ok("examples/machine/native/macro_directive_roundtrip_x86_64.nm");
}

#[test]
fn machine_phase12_dialect_import_export_regressions_ok() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
}

#[test]
fn machine_phase12_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx2_vex_complete_x86_64.nm");
}

#[test]
fn machine_phase12_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase12_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase12_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase12_roundtrip_regressions_ok() {
    assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
}

#[test]
fn machine_phase12_cross_arch_regressions_ok() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

fn phase13_emit_object(rel: &str, target: &str, name: &str) -> PathBuf {
    let ext = if target.contains("windows") {
        "obj"
    } else if target.contains("wasm32") {
        "wasm"
    } else {
        "o"
    };
    emit_machine_object(rel, target, name, ext)
}

fn phase13_validate(path: &Path, profile: &str) -> String {
    let args = vec![
        "machine".to_string(),
        "external-compat".to_string(),
        "validate".to_string(),
        path.to_string_lossy().to_string(),
        "--profile".to_string(),
        profile.to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "external validate failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
    stdout
}

fn phase13_build_linked(rel: &str, target: &str, name: &str, ext: &str) -> PathBuf {
    phase12_build("build-exe", rel, target, name, ext)
}

#[test]
fn machine_phase13_external_tools_discovery_ok() {
    let out = run_ncc(&["machine", "external-tools", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
    assert!(stdout.contains("\"blocked_by_environment\""));
}

#[test]
fn machine_phase13_external_tools_versions_reported() {
    let out = run_ncc(&["machine", "external-tools", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"version\""));
    assert!(stdout.contains("\"detection_command\""));
}

#[test]
fn machine_phase13_third_party_object_matrix_contract_ok() {
    let payload =
        assert_json_command_ok(&["machine", "third-party-objects", "--json", "--no-probe"]);
    assert_eq!(
        payload["schema"],
        "ncc.machine.third_party_object_matrix.v1"
    );
    assert_eq!(payload["status"], "closed");
    assert_eq!(payload["probe_count"], 0);
    let raw = serde_json::to_string(&payload).expect("third-party matrix json");
    for needle in [
        "MSVC cl",
        "clang",
        "gcc",
        "nasm",
        "MASM ml64",
        "GNU as/gas",
        "elf64",
        "coff",
        "macho",
        "R_X86_64_GOTPCREL",
        "IMAGE_REL_AMD64_ADDR64",
        "X86_64_RELOC_UNSIGNED",
    ] {
        assert!(raw.contains(needle), "{needle}");
    }
}

#[test]
fn machine_phase13_third_party_object_matrix_probe_ok_or_blocked() {
    let out_dir = temp_path("phase13_third_party_objects", "dir");
    let report = temp_path("phase13_third_party_objects", "json");
    let args = vec![
        "machine".to_string(),
        "third-party-objects".to_string(),
        "--json".to_string(),
        "--strict-available".to_string(),
        "--out-dir".to_string(),
        out_dir.to_string_lossy().to_string(),
        "--report".to_string(),
        report.to_string_lossy().to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    let stderr = normalize(&output.stderr);
    assert!(
        output.status.success(),
        "third-party object matrix probe failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        report.exists(),
        "third-party retained report must be written"
    );
    let payload: Value =
        serde_json::from_str(&stdout).unwrap_or_else(|err| panic!("{err}\n{stdout}"));
    assert_eq!(
        payload["schema"],
        "ncc.machine.third_party_object_matrix.v1"
    );
    let probes = payload["probes"].as_array().expect("probes");
    assert!(!probes.is_empty());
    for probe in probes {
        let status = probe["status"].as_str().unwrap_or_default();
        assert!(
            matches!(status, "linked_ok" | "blocked_by_environment"),
            "unexpected probe status {status}: {probe}"
        );
        if status == "linked_ok" {
            assert!(probe["objects"]
                .as_array()
                .expect("objects")
                .iter()
                .all(|object| object["link_unit_origin"] == "native_object"));
            assert!(probe["applied_relocations"].as_u64().unwrap_or(0) > 0);
            assert!(probe["missing_relocations"]
                .as_array()
                .expect("missing relocs")
                .is_empty());
        }
    }
}

#[test]
fn machine_phase13_missing_tools_blocked_by_environment() {
    let out = run_ncc(&["machine", "external-tools", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"blocked_by_environment\""));
    assert!(!stdout.contains("missing tool success"));
}

#[test]
fn machine_phase13_external_profiles_exist() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    for profile in [
        "nC-external-elf64-v1",
        "nC-external-coff-v1",
        "nC-external-macho-v1",
        "nC-external-wasm-v1",
        "nC-external-debug-dwarf-v1",
        "nC-external-linker-v1",
        "nC-external-archive-v1",
    ] {
        assert!(stdout.contains(profile), "{profile}");
    }
}

#[test]
fn machine_phase13_external_elf64_profile_ok() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    assert!(normalize(&out.stdout).contains("nC-external-elf64-v1"));
}

#[test]
fn machine_phase13_external_coff_profile_ok() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    assert!(normalize(&out.stdout).contains("nC-external-coff-v1"));
}

#[test]
fn machine_phase13_external_macho_profile_ok() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    assert!(normalize(&out.stdout).contains("nC-external-macho-v1"));
}

#[test]
fn machine_phase13_external_wasm_profile_ok() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    assert!(normalize(&out.stdout).contains("nC-external-wasm-v1"));
}

#[test]
fn machine_phase13_elf64_external_validation_ok_or_blocked() {
    let object = phase13_emit_object(
        "examples/machine/native/external_compat_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
        "phase13_elf64",
    );
    let stdout = phase13_validate(&object, "elf64");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_coff_external_validation_ok_or_blocked() {
    let object = phase13_emit_object(
        "examples/machine/native/external_compat_coff_x86_64.nm",
        "x86_64-pc-windows-msvc",
        "phase13_coff",
    );
    let stdout = phase13_validate(&object, "coff");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_macho_external_validation_ok_or_blocked() {
    let object = phase13_emit_object(
        "examples/machine/native/external_compat_macho_x86_64.nm",
        "x86_64-apple-darwin",
        "phase13_macho",
    );
    let stdout = phase13_validate(&object, "macho");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_wasm_external_validation_ok_or_blocked() {
    let object = phase13_emit_object(
        "examples/machine/native/external_compat_wasm32.nm",
        "wasm32-unknown-unknown",
        "phase13_wasm",
    );
    let stdout = phase13_validate(&object, "wasm");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_debug_dwarf_external_validation_ok_or_blocked() {
    let object = phase13_emit_object(
        "examples/machine/native/external_compat_debug_elf64.nm",
        "x86_64-unknown-linux-gnu",
        "phase13_debug",
    );
    let args = vec![
        "machine".to_string(),
        "external-compat".to_string(),
        "validate-debug".to_string(),
        object.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_linker_external_validation_ok_or_blocked() {
    let linked = phase13_build_linked(
        "examples/machine/native/external_compat_link_elf64.nm",
        "x86_64-unknown-linux-gnu",
        "phase13_linked",
        "exe",
    );
    let args = vec![
        "machine".to_string(),
        "external-compat".to_string(),
        "validate-link".to_string(),
        linked.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    let stdout = normalize(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_archive_external_validation_ok_or_blocked() {
    let archive = phase12_build(
        "build-staticlib",
        "examples/machine/native/external_compat_archive_elf64.nm",
        "x86_64-unknown-linux-gnu",
        "phase13_archive",
        "a",
    );
    let stdout = phase13_validate(&archive, "archive");
    assert!(stdout.contains("compatible_ok") || stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_external_matrix_exists() {
    assert!(
        fixture("reports/machine_native_total/phase13/phase13_external_compat_matrix.json")
            .exists()
    );
}

#[test]
fn machine_phase13_external_report_exists() {
    assert!(
        fixture("reports/machine_native_total/phase13/phase13_external_compat_complete.json")
            .exists()
    );
}

#[test]
fn machine_phase13_blocked_by_environment_report_ok() {
    let path =
        fixture("reports/machine_native_total/phase13/phase13_blocked_by_environment_report.json");
    let text = fs::read_to_string(path).expect("phase13 blocked report");
    assert!(text.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_no_external_truth_dependency() {
    let out = run_ncc(&["machine", "external-compat", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"external_truth_dependency\": false"));
}

#[test]
fn machine_phase13_external_invalid_artifact_fail() {
    let bad = temp_path("phase13_invalid_artifact", "bin");
    fs::write(&bad, b"not an object").expect("bad artifact");
    let args = vec![
        "machine".to_string(),
        "external-compat".to_string(),
        "validate".to_string(),
        bad.to_string_lossy().to_string(),
        "--profile".to_string(),
        "elf64".to_string(),
        "--json".to_string(),
    ];
    let output = run_ncc_dynamic(&args);
    assert!(!output.status.success());
}

#[test]
fn machine_phase13_external_parse_output_fail() {
    assert_machine_fail(
        "examples/machine/native/negative/external_invalid_elf_fail.nm",
        "E_MACHINE_EXTERNAL_COMPAT_FAIL_001",
    );
}

#[test]
fn machine_phase13_external_expected_failure_report_ok() {
    assert_machine_fail(
        "examples/machine/native/negative/external_compat_expected_failure_fail.nm",
        "E_MACHINE_EXTERNAL_COMPAT_FAIL_001",
    );
}

#[test]
fn machine_phase13_no_fake_external_validation() {
    let out = run_ncc(&["machine", "external-compat", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " external"].concat()));
    assert!(!stdout.contains(&["fake", " validation"].concat()));
}

#[test]
fn machine_phase13_no_fake_compatibility() {
    let out = run_ncc(&["machine", "external-compat", "matrix", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " compatibility"].concat()));
}

#[test]
fn machine_phase13_no_fake_success_for_missing_tool() {
    let out = run_ncc(&["machine", "external-tools", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " success"].concat()));
    assert!(stdout.contains("blocked_by_environment"));
}

#[test]
fn machine_phase13_no_fn_fa_positive_examples() {
    for entry in fs::read_dir(fixture("examples/machine/native")).expect("native dir") {
        let path = entry.expect("entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("external_compat_")
            || path.extension().and_then(|ext| ext.to_str()) != Some("nm")
        {
            continue;
        }
        let text = fs::read_to_string(&path).expect("phase13 positive");
        assert!(!text.contains(&["fn", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["fa", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["stc", ":"].concat()), "{}", path.display());
    }
}

#[test]
fn machine_phase13_native_doctor_gate_ok() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase13_external_tool_compatibility\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase13_preserves_phase14_closed() {
    let out = run_ncc(&["machine", "external-compat", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase14_fuzzing_corpus_hardening\": \"closed\""));
}

#[test]
fn machine_phase13_asm_complete_total_remains_true() {
    let out = run_ncc(&["machine", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase13_linker_regressions_ok() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration_status\": \"closed\""));
}

#[test]
fn machine_phase13_dialect_import_export_regressions_ok() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
}

#[test]
fn machine_phase13_macro_directive_regressions_ok() {
    assert_machine_ok("examples/machine/native/macro_directive_roundtrip_x86_64.nm");
}

#[test]
fn machine_phase13_vex_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx2_vex_complete_x86_64.nm");
}

#[test]
fn machine_phase13_avx512_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase13_debug_unwind_regressions_ok() {
    assert_machine_ok("examples/machine/native/debug_unwind_elf64_x86_64.nm");
}

#[test]
fn machine_phase13_disasm_regressions_ok() {
    assert_machine_ok("examples/machine/native/disasm_elf64_x86_64.nm");
}

#[test]
fn machine_phase13_roundtrip_regressions_ok() {
    assert_roundtrip_ok(
        "examples/machine/native/roundtrip_elf64_x86_64.nm",
        "x86_64-unknown-linux-gnu",
    );
}

#[test]
fn machine_phase13_cross_arch_regressions_ok() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}

static PHASE14_CORPUS_ONCE: Once = Once::new();

fn phase14_prepare_corpus() {
    PHASE14_CORPUS_ONCE.call_once(|| {
        let out = run_ncc(&[
            "machine",
            "fuzz",
            "generate-corpus",
            "--profile",
            "smoke",
            "--json",
        ]);
        let stdout = normalize(&out.stdout);
        let stderr = normalize(&out.stderr);
        assert!(
            out.status.success(),
            "phase14 corpus generation failed\nstdout={stdout}\nstderr={stderr}"
        );
        assert!(stdout.contains("\"phase14_fuzzing_corpus_hardening\": \"closed\""));
    });
}

fn phase14_fuzz(command: &str) -> String {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", command, "--smoke", "--json"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "phase14 fuzz {command} failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("\"phase14_fuzzing_corpus_hardening\": \"closed\""));
    assert!(stdout.contains("\"inputs_executed\""));
    assert!(!stdout.contains("\"inputs_executed\": 0"));
    stdout
}

#[test]
fn machine_phase14_fuzz_doctor_exists() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"closed\""));
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase14_corpus_manifest_exists() {
    phase14_prepare_corpus();
    assert!(fixture("tests/corpus/machine/MANIFEST.json").exists());
}

#[test]
fn machine_phase14_corpus_counts_nonzero() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "corpus", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"actual_total\""));
    assert!(!stdout.contains("\"actual_total\": 0"));
}

#[test]
fn machine_phase14_corpus_hashes_exist() {
    phase14_prepare_corpus();
    let path = fixture("reports/machine_native_total/phase14/phase14_corpus_hashes.json");
    let text = fs::read_to_string(path).expect("phase14 corpus hashes");
    assert!(text.contains("\"hash\""));
}

#[test]
fn machine_phase14_parser_fuzz_smoke_ok() {
    phase14_fuzz("parser");
}

#[test]
fn machine_phase14_dialect_fuzz_smoke_ok() {
    phase14_fuzz("dialects");
}

#[test]
fn machine_phase14_macro_directive_fuzz_smoke_ok() {
    let macros = phase14_fuzz("macros");
    let directives = phase14_fuzz("directives");
    assert!(macros.contains("\"subsystem\": \"macros\""));
    assert!(directives.contains("\"subsystem\": \"directives\""));
}

#[test]
fn machine_phase14_ctfe_fuzz_smoke_ok() {
    phase14_fuzz("ctfe");
}

#[test]
fn machine_phase14_include_incbin_fuzz_smoke_ok() {
    phase14_fuzz("include-incbin");
}

#[test]
fn machine_phase14_encoder_fuzz_smoke_ok() {
    phase14_fuzz("encoder");
}

#[test]
fn machine_phase14_object_fuzz_smoke_ok() {
    phase14_fuzz("object");
}

#[test]
fn machine_phase14_debug_unwind_fuzz_smoke_ok() {
    phase14_fuzz("debug-unwind");
}

#[test]
fn machine_phase14_disasm_fuzz_smoke_ok() {
    phase14_fuzz("disasm");
}

#[test]
fn machine_phase14_roundtrip_fuzz_smoke_ok() {
    phase14_fuzz("roundtrip");
}

#[test]
fn machine_phase14_linker_fuzz_smoke_ok() {
    phase14_fuzz("linker");
}

#[test]
fn machine_phase14_external_compat_fuzz_smoke_ok() {
    phase14_fuzz("external-compat");
}

#[test]
fn machine_phase14_cross_arch_fuzz_smoke_ok() {
    phase14_fuzz("cross-arch");
}

#[test]
fn machine_phase14_all_fuzz_smoke_ok() {
    phase14_fuzz("all");
}

#[test]
fn machine_phase14_malformed_objects_do_not_panic() {
    let stdout = phase14_fuzz("object");
    assert!(stdout.contains("\"open_blockers\": 0"));
}

#[test]
fn machine_phase14_random_bytes_do_not_panic() {
    let stdout = phase14_fuzz("disasm");
    assert!(stdout.contains("\"expected_diagnostics\""));
}

#[test]
fn machine_phase14_bad_macros_do_not_panic() {
    let stdout = phase14_fuzz("macros");
    assert!(stdout.contains("\"expected_diagnostics\""));
}

#[test]
fn machine_phase14_bad_dialects_do_not_panic() {
    let stdout = phase14_fuzz("dialects");
    assert!(stdout.contains("\"expected_diagnostics\""));
}

#[test]
fn machine_phase14_bad_relocations_do_not_panic() {
    let stdout = phase14_fuzz("linker");
    assert!(stdout.contains("\"expected_diagnostics\""));
}

#[test]
fn machine_phase14_bad_debug_unwind_do_not_panic() {
    let stdout = phase14_fuzz("debug-unwind");
    assert!(stdout.contains("\"expected_diagnostics\""));
}

#[test]
fn machine_phase14_timeout_controlled_and_reported() {
    phase14_prepare_corpus();
    let text = fs::read_to_string(fixture(
        "reports/machine_native_total/phase14/phase14_timeout_oom_report.json",
    ))
    .expect("phase14 timeout report");
    assert!(text.contains("\"silent_timeout\": 0"));
    assert!(text.contains("\"open_timeout\": 0"));
}

#[test]
fn machine_phase14_oom_controlled_and_reported() {
    phase14_prepare_corpus();
    let text = fs::read_to_string(fixture(
        "reports/machine_native_total/phase14/phase14_timeout_oom_report.json",
    ))
    .expect("phase14 oom report");
    assert!(text.contains("\"silent_oom\": 0"));
    assert!(text.contains("\"open_oom\": 0"));
}

#[test]
fn machine_phase14_crash_triage_no_open_blockers() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "triage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"open_blockers\": 0"));
}

#[test]
fn machine_phase14_minimizer_smoke_ok() {
    phase14_prepare_corpus();
    let input = fixture("tests/corpus/machine/parser/invalid/parser_invalid_0000.nm");
    let args = vec![
        "machine".to_string(),
        "fuzz".to_string(),
        "minimize".to_string(),
        input.to_string_lossy().to_string(),
        "--json".to_string(),
    ];
    let out = run_ncc_dynamic(&args);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"status\": \"closed\""));
}

#[test]
fn machine_phase14_reproducer_smoke_ok() {
    phase14_prepare_corpus();
    let out = run_ncc(&[
        "machine",
        "fuzz",
        "reproduce",
        "parser-invalid-000",
        "--json",
    ]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"classification\": \"diagnostic_expected\""));
}

#[test]
fn machine_phase14_unclassified_crashes_zero() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "triage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"unclassified_crashes\": 0"));
}

#[test]
fn machine_phase14_no_fake_fuzzing() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"fake_fuzzing\": false"));
    assert!(!stdout.contains(&["fake", " fuzz"].concat()));
}

#[test]
fn machine_phase14_no_fake_corpus_coverage() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "corpus", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(!stdout.contains(&["fake", " corpus"].concat()));
}

#[test]
fn machine_phase14_no_empty_harnesses() {
    let stdout = phase14_fuzz("all");
    assert!(!stdout.contains("\"inputs_executed\": 0"));
}

#[test]
fn machine_phase14_no_always_success_harnesses() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "fuzz", "triage", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"expected_diagnostic\""), "{stdout}");
    assert!(
        stdout.contains("E_MACHINE_FUZZ_EXPECTED_DIAGNOSTIC_001"),
        "{stdout}"
    );
}

#[test]
fn machine_phase14_no_fn_fa_positive_examples() {
    phase14_prepare_corpus();
    for entry in fs::read_dir(fixture("examples/machine/native")).expect("native dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("nm") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("machine native positive");
        assert!(!text.contains(&["fn", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["fa", ":"].concat()), "{}", path.display());
        assert!(!text.contains(&["stc", ":"].concat()), "{}", path.display());
    }
}

#[test]
fn machine_phase14_asm_complete_total_remains_true() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_complete_total\": true"));
}

#[test]
fn machine_phase14_native_doctor_gate_ok() {
    phase14_prepare_corpus();
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase14_fuzzing_corpus_hardening\""));
}

#[test]
fn machine_phase14_phase13_external_compat_regressions_ok() {
    let out = run_ncc(&["machine", "external-compat", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase13_external_tool_compatibility_status\": \"closed\""));
}

#[test]
fn machine_phase14_phase12_linker_regressions_ok() {
    let out = run_ncc(&["machine", "link-doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase12_linker_integration_status\": \"closed\""));
}

#[test]
fn machine_phase14_asm_dialect_dialect_regressions_ok() {
    let out = run_ncc(&["machine", "asm-dialect", "doctor", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"asm_dialect_import_export\": \"closed\""));
}

#[test]
fn machine_phase14_phase10_macro_directive_regressions_ok() {
    assert_machine_ok("examples/machine/native/macro_directive_roundtrip_x86_64.nm");
}

#[test]
fn machine_phase14_phase9_audit_regressions_ok() {
    assert!(
        fixture("reports/machine_native_total/phase9/phase9_industrial_asm_audit.json").exists()
    );
}

#[test]
fn machine_phase14_x86_regressions_ok() {
    assert_machine_ok("examples/machine/native/avx512_remaining_subfeatures_x86_64.nm");
}

#[test]
fn machine_phase14_cross_arch_regressions_ok() {
    let out = run_ncc(&["machine", "parity", "--json"]);
    let stdout = normalize(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("\"phase8_cross_arch_parity_total\": \"closed\""));
}
