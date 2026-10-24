use super::*;
use crate::commands::primitive::{
    primitive_default_entry_for_program, primitive_emit_witness_json,
    primitive_firmware_boot_model_json, primitive_read_program,
    primitive_reject_runtime_stdout_fallbacks, run_primitive_command,
};
use std::sync::{Mutex, OnceLock};

static PRIMITIVE_QEMU_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn primitive_qemu_test_guard() -> std::sync::MutexGuard<'static, ()> {
    PRIMITIVE_QEMU_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn primitive_fixture_path(rel: &str) -> String {
    let root = workspace_repo_root();
    let direct = root.join("examples").join("primitive").join(rel);
    let path = if direct.exists() {
        direct
    } else {
        let archived = root
            .join("_archive")
            .join("old_examples")
            .join("primitive")
            .join(rel);
        if archived.exists() {
            archived
        } else {
            root.join("tests")
                .join("fixtures")
                .join("old_examples")
                .join("primitive")
                .join(rel)
        }
    };
    path.to_string_lossy().to_string()
}

fn read_program(path: &str) -> ncc_ast::Program {
    let src = std::fs::read_to_string(path).expect("fixture read");
    let tokens = ncc_lexer::lex(&src).expect("lex");
    ncc_parser::parse(tokens).expect("parse")
}

fn write_temp_primitive(name: &str, source: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "ncc_primitive_test_{}_{}_{}.ncp",
        name,
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, source.as_bytes()).expect("write temp source");
    path.to_string_lossy().to_string()
}

fn write_temp_target(name: &str, body: &str) -> String {
    let root = workspace_repo_root();
    let path = root.join("targets").join(format!("{name}.nctarget"));
    std::fs::write(&path, body.as_bytes()).expect("write temp target");
    name.to_string()
}

fn advanced_physical_semantics_fixture() -> String {
    write_temp_primitive(
        "advanced_physical_semantics",
        "primitive[@] target x86_64:\n    voltage:\n        vcore;\n    current:\n        rail0;\n    noise:\n        emi0;\n    clock:\n        core_clk;\n    edge:\n        rising;\n    metastability:\n        cdc0;\n    power:\n        pstate0;\n    powerGate:\n        pg0;\n    temperature:\n        sensor0;\n    thermalState:\n        thermal_state0;\n    throttleState:\n        throttle_state0;\n    watchdog:\n        wd0;\n    interruptLine:\n        irq0;\n    dmaChannel:\n        dma0;\n    cacheState:\n        l1_clean;\n    tlbState:\n        tlb_hot;\n    firmwareRegion:\n        fw0;\n    bootRomRegion:\n        rom0;\n    biosRegion:\n        bios0;\n    uefiHandoff:\n        handoff0;\n    resetVector:\n        reset0;\n    bootWindow:\n        bootwin0;\n    bootAnchor:\n        boot0;\n    firmwareTable:\n        fwtable0;\n    deviceTree:\n        dt0;\n    acpiTable:\n        acpi0;\n    microcodeState:\n        patch_state0;\n    controller:\n        uart_ctrl;\n    peripheral:\n        uart0;\n    phy:\n        uart_phy;\n    lane:\n        lane0;\n    channel:\n        ch0;\n    pipelineState:\n        fetch_decode;\n    branchPredictorState:\n        bp0;\n    reorderState:\n        rob0;\n    outOfOrder:\n        ooo_enabled;\n    physicalFault:\n        thermal_trip;\n    timingViolation:\n        missed_deadline;\n    thermalViolation:\n        overheat;\n    metastabilityRisk:\n        cdc_risk;\n    dmaViolation:\n        dma_race;\n    cacheCoherenceViolation:\n        stale_line;\n    mmioSideEffectViolation:\n        read_clears_flag;\n    capability hw:\n        allow port.write;\n        allow protocol.uart;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol UART;\n        writes uart0.data;\n        event physicalFault overheat;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    rupture:\n        thermal_trip;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    entry kernelEntry:\n        anchor boot0;\n        mode real16;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    )
}

fn event_contracts_fixture() -> String {
    write_temp_primitive(
        "event_contracts_semantics",
        "primitive[@] target x86_64:\n    interruptLine:\n        irq0;\n    dmaChannel:\n        dma0;\n    watchdog:\n        wd0;\n    peripheral:\n        uart0;\n    physicalFault:\n        thermal_trip;\n    window status:\n        cache uncached;\n    capability hw:\n        allow port.write;\n        allow protocol.uart;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        mmioRead mmioaddr(0x0040) volatile;\n        read side_effect clears_status_flag;\n        event interrupt irq0;\n        event dmaCompletion dma0;\n        event watchdogTrip wd0;\n        event mmioReadClearsFlag status.ready;\n        event peripheralStateChange uart0;\n        event physicalFault thermal_trip;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    rupture:\n        thermal_trip;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    entry kernelEntry:\n        anchor boot0;\n        mode real16;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    )
}

fn run_primitive_check(
    source_path: String,
    target: &str,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let args = vec![
        "check".to_string(),
        source_path,
        "--target".to_string(),
        target.to_string(),
        "--environment".to_string(),
        environment.to_string(),
    ];
    run_primitive_command(&args)
}

fn protocol_contract_fixture(name: &str, protocol: &str) -> String {
    write_temp_primitive(
        name,
        format!(
            "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol {protocol};\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n"
        )
        .as_str(),
    )
}

fn protocol_deep_lowering_fixture(name: &str) -> String {
    write_temp_primitive(
        name,
        "primitive[@] target x86_64:\n    controller:\n        ctrl0;\n    peripheral:\n        per0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    channel:\n        ch0;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    window usbwin0:\n        addr mmioaddr(0x2100);\n    window satawin0:\n        addr mmioaddr(0x2200);\n    window nvmewin0:\n        addr mmioaddr(0x2300);\n    window ethwin0:\n        addr mmioaddr(0x2400);\n    window hdmiwin0:\n        addr mmioaddr(0x2500);\n    window dpwin0:\n        addr mmioaddr(0x2600);\n    capability hw:\n        allow port.write;\n    operation kernelEntry():\n        capability hw;\n        protocol UART controller ctrl0 peripheral per0 dataPort dev.data;\n        protocol PCIe controller ctrl0 lane lane0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data;\n        protocol USB controller ctrl0 peripheral per0 window usbwin0;\n        protocol SATA controller ctrl0 peripheral per0 channel ch0 window satawin0;\n        protocol NVMe controller ctrl0 peripheral per0 channel ch0 window nvmewin0;\n        protocol I2C controller ctrl0 channel ch0 dataPort dev.data;\n        protocol SPI controller ctrl0 channel ch0 dataPort dev.data;\n        protocol Ethernet controller ctrl0 peripheral per0 phy phy0 window ethwin0;\n        protocol HDMI controller ctrl0 phy phy0 window hdmiwin0;\n        protocol DisplayPort controller ctrl0 phy phy0 lane lane0 window dpwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    )
}

fn protocol_matrix_clause(protocol: &str) -> &'static str {
    match protocol {
        "UART" => "protocol UART controller ctrl0 peripheral per0 dataPort dev.data readyMask 0x20 writeMask 0x01;",
        "PCIe" => "protocol PCIe controller ctrl0 lane lane0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data configCommand 0x80 compareValue 0x32;",
        "USB" => "protocol USB controller ctrl0 peripheral per0 window usbwin0 statusOffset 0x04 commandOffset 0x00;",
        "SATA" => "protocol SATA controller ctrl0 peripheral per0 channel ch0 window satawin0 statusOffset 0x10 commandOffset 0x18;",
        "NVMe" => "protocol NVMe controller ctrl0 peripheral per0 channel ch0 window nvmewin0 statusOffset 0x1C commandOffset 0x20;",
        "I2C" => "protocol I2C controller ctrl0 channel ch0 dataPort dev.data readyMask 0x02 writeMask 0x0F;",
        "SPI" => "protocol SPI controller ctrl0 channel ch0 dataPort dev.data readyMask 0x01 writeMask 0x1F;",
        "Ethernet" => "protocol Ethernet controller ctrl0 peripheral per0 phy phy0 window ethwin0 statusOffset 0x08 commandOffset 0x14 readyMask 0x10;",
        "HDMI" => "protocol HDMI controller ctrl0 phy phy0 window hdmiwin0 statusOffset 0x0C commandOffset 0x10 readyMask 0x40;",
        "DisplayPort" => "protocol DisplayPort controller ctrl0 phy phy0 lane lane0 window dpwin0 statusOffset 0x14 commandOffset 0x18 readyMask 0x40;",
        _ => panic!("unknown protocol matrix fixture protocol '{}'", protocol),
    }
}

fn protocol_matrix_fixture(name: &str, protocol: &str) -> String {
    let clause = protocol_matrix_clause(protocol);
    write_temp_primitive(
        name,
        format!(
            "primitive[@] target x86_64:\n    controller:\n        ctrl0;\n    peripheral:\n        per0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    channel:\n        ch0;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    window usbwin0:\n        addr mmioaddr(0x2100);\n    window satawin0:\n        addr mmioaddr(0x2200);\n    window nvmewin0:\n        addr mmioaddr(0x2300);\n    window ethwin0:\n        addr mmioaddr(0x2400);\n    window hdmiwin0:\n        addr mmioaddr(0x2500);\n    window dpwin0:\n        addr mmioaddr(0x2600);\n    capability hw:\n        allow port.write;\n    operation kernelEntry():\n        capability hw;\n        {clause}\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n"
        )
        .as_str(),
    )
}

#[test]
fn primitive_hardware_identity_uart_ok() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    ncc_sema::analyze(&program).expect("primitive hardware identity fixture must pass sema");
}

#[test]
fn primitive_multi_isa_contract_survives_parser_sema_and_ir() {
    let path = workspace_repo_root()
        .join("examples")
        .join("12_primitive")
        .join("multi_isa_machine_contract.ncp");
    let program = read_program(path.to_string_lossy().as_ref());
    ncc_sema::analyze(&program).expect("multi-ISA primitive contract must pass sema");

    let ir = format!("{:#?}", ncc_ir::lower(program));
    for kind in ["Isa", "Opcode", "Encoding", "Relocation", "Abi"] {
        assert!(
            ir.contains(&format!("kind: {kind}")),
            "primitive directive {kind} must survive typed IR lowering\n{ir}"
        );
    }
}

#[test]
fn primitive_material_witness_uart_ok() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    assert_eq!(
        witness.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.material.witness.v1")
    );
    assert_eq!(
        witness.get("evidence_level").and_then(|v| v.as_str()),
        Some("lowered")
    );
    assert_eq!(
        witness
            .get("physical_hardware_certified")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
fn primitive_material_witness_reports_advanced_physical_semantics() {
    let path = advanced_physical_semantics_fixture();
    let program = read_program(&path);
    ncc_sema::analyze(&program).expect("advanced physical fixture must pass sema");

    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    let semantics = witness
        .get("material_semantics")
        .and_then(|v| v.as_array())
        .expect("material_semantics must be present in witness");
    let labels: std::collections::BTreeSet<String> = semantics
        .iter()
        .filter_map(|v| v.as_str())
        .map(|v| v.to_string())
        .collect();
    for required in [
        "voltage",
        "current",
        "noise",
        "metastability",
        "power",
        "powerGate",
        "temperature",
        "thermalState",
        "throttleState",
        "watchdog",
        "interruptLine",
        "dmaChannel",
        "cacheState",
        "tlbState",
        "firmwareRegion",
        "bootRomRegion",
        "biosRegion",
        "uefiHandoff",
        "resetVector",
        "bootWindow",
        "bootAnchor",
        "firmwareTable",
        "deviceTree",
        "acpiTable",
        "microcodeState",
        "controller",
        "peripheral",
        "phy",
        "lane",
        "channel",
        "pipelineState",
        "branchPredictorState",
        "reorderState",
        "outOfOrder",
        "physicalFault",
        "timingViolation",
        "thermalViolation",
        "metastabilityRisk",
        "dmaViolation",
        "cacheCoherenceViolation",
        "mmioSideEffectViolation",
    ] {
        assert!(
            labels.contains(required),
            "witness material_semantics must include '{}'",
            required
        );
    }
    let has_event_lowering_bytes = witness
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("kind").and_then(|v| v.as_str()) == Some("event_hook")
                    && row
                        .get("lowering_machine_bytes_hex")
                        .and_then(|v| v.as_str())
                        .map(|hex| !hex.is_empty())
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        has_event_lowering_bytes,
        "advanced witness must include machine lowering bytes for event hooks"
    );
    let has_protocol_hook_semantics = witness
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("kind")
                    .and_then(|v| v.as_str())
                    .map(|kind| kind.starts_with("protocol_") && kind.ends_with("_contract"))
                    .unwrap_or(false)
                    && row
                        .get("lowering_semantics")
                        .and_then(|v| v.as_array())
                        .map(|cols| !cols.is_empty())
                        .unwrap_or(false)
                    && row
                        .get("lowering_mode")
                        .and_then(|v| v.as_str())
                        .map(|mode| mode.starts_with("protocol_"))
                        .unwrap_or(false)
                    && row
                        .get("transport")
                        .and_then(|v| v.as_str())
                        .map(|transport| matches!(transport, "port_io" | "mmio"))
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        has_protocol_hook_semantics,
        "advanced witness must include protocol hook lowering semantics"
    );
}

#[test]
fn primitive_material_witness_reports_firmware_boot_model_contracts() {
    let path = advanced_physical_semantics_fixture();
    let program = read_program(&path);
    ncc_sema::analyze(&program).expect("advanced physical fixture must pass sema");

    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    let model = witness
        .get("firmware_boot_model")
        .expect("firmware_boot_model must be present");
    assert_eq!(model.get("verified").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        model
            .get("resetVector")
            .and_then(|v| v.as_array())
            .map(|rows| rows.len()),
        Some(1)
    );
    assert_eq!(
        model
            .get("bootAnchor")
            .and_then(|v| v.as_array())
            .and_then(|rows| rows.first())
            .and_then(|v| v.as_str()),
        Some("boot0")
    );
    assert_eq!(
        model
            .get("contract_checks")
            .and_then(|v| v.get("uefi_requires_firmware_table"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        model
            .get("contract_checks")
            .and_then(|v| v.get("bios_requires_reset_and_window"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        model
            .get("contract_checks")
            .and_then(|v| v.get("microcode_requires_firmware_table"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn primitive_material_witness_contains_non_linear_event_contracts() {
    let path = event_contracts_fixture();
    let program = read_program(&path);
    ncc_sema::analyze(&program).expect("event contract fixture must pass sema");

    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    let events = witness
        .get("event_contracts")
        .and_then(|v| v.as_array())
        .expect("event_contracts must exist");
    let event_kinds: std::collections::BTreeSet<String> = events
        .iter()
        .filter_map(|row| row.get("kind"))
        .filter_map(|v| v.as_str())
        .map(|v| v.to_string())
        .collect();
    for required in [
        "interrupt",
        "dmaCompletion",
        "watchdogTrip",
        "mmioReadClearsFlag",
        "peripheralStateChange",
        "physicalFault",
    ] {
        assert!(
            event_kinds.contains(required),
            "witness event_contracts must include '{}'",
            required
        );
    }
    assert!(
        events.iter().all(|row| {
            row.get("lowering_prefix_hex")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)
                && row
                    .get("lowering_suffix_hex")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                && row
                    .get("lowering_semantics")
                    .and_then(|v| v.as_array())
                    .map(|cols| !cols.is_empty())
                    .unwrap_or(false)
        }),
        "witness event_contracts must include lowering prefix/suffix and semantic tags"
    );
}

#[test]
fn primitive_boot_contract_collects_non_linear_event_contracts() {
    let path = event_contracts_fixture();
    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extraction should pass");
    assert_eq!(contract.event_contracts.len(), 6);
    let kinds: std::collections::BTreeSet<&'static str> = contract
        .event_contracts
        .iter()
        .map(|event| event.kind.as_str())
        .collect();
    for required in [
        "interrupt",
        "dmaCompletion",
        "watchdogTrip",
        "mmioReadClearsFlag",
        "peripheralStateChange",
        "physicalFault",
    ] {
        assert!(
            kinds.contains(required),
            "boot contract must include event kind '{}'",
            required
        );
    }
}

#[test]
fn primitive_target_declares_isa_ok() {
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    assert_eq!(target.field("isa"), Some("x86_64"));
}

#[test]
fn primitive_target_declares_environment_ok() {
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    assert_eq!(target.field("environment"), Some("freestanding"));
}

#[test]
fn primitive_cli_check_environment_missing_fail() {
    let target_name = write_temp_target(
        &format!(
            "tmp-env-missing-{}",
            TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
        ),
        "name = \"tmp-env-missing\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nprotocols = [\"UART\"]\n",
    );
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        target_name.clone(),
    ];
    let err = run_primitive_command(&args).expect_err("missing environment must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_ENV_MISSING_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_cli_check_environment_mismatch_fail() {
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("environment mismatch must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_ENV_UNSUPPORTED_001"));
}

#[test]
fn primitive_target_declares_protocol_uart_ok() {
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    assert!(target.supports_protocol("UART"));
}

#[test]
fn primitive_port_window_ok() {
    let src = std::fs::read_to_string(primitive_fixture_path("hardware_identity_uart.ncp"))
        .expect("fixture read");
    assert!(src.contains("window com1_port_window"));
}

#[test]
fn primitive_signal_tx_ready_ok() {
    let src = std::fs::read_to_string(primitive_fixture_path("hardware_identity_uart.ncp"))
        .expect("fixture read");
    assert!(src.contains("signal uart.com1.txReady"));
}

#[test]
fn primitive_timing_clock_ok() {
    let src = std::fs::read_to_string(primitive_fixture_path("hardware_identity_uart.ncp"))
        .expect("fixture read");
    assert!(src.contains("timing await"));
}

#[test]
fn primitive_mmio_side_effect_declared_ok() {
    let src = std::fs::read_to_string(primitive_fixture_path("hardware_identity_uart.ncp"))
        .expect("fixture read");
    assert!(src.contains("read side_effect none"));
}

#[test]
fn primitive_firmware_boot_anchor_ok() {
    let src = std::fs::read_to_string(primitive_fixture_path("hardware_identity_uart.ncp"))
        .expect("fixture read");
    assert!(src.contains("anchor boot0"));
    assert!(src.contains("entry kernelEntry"));
}

#[test]
fn primitive_emit_lowering_witness_ok() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let image = build_primitive_boot_image_from_program(
        &program,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("lowering call must pass")
    .expect("image must be produced");
    assert_eq!(image.len(), 512);
}

#[test]
fn primitive_qemu_uart_smoke_ok_if_qemu_available() {
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let _guard = primitive_qemu_test_guard();
    let args = vec![
        "qemu".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--expect".to_string(),
        "nC primitive alive".to_string(),
    ];
    run_primitive_command(&args).expect("qemu smoke should pass when qemu is available");
}

#[test]
fn primitive_operation_without_material_witness_fail() {
    let temp = write_temp_primitive(
        "missing_witness",
        "primitive[@] target x86_64:\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let (src, program) = primitive_read_program(&temp).expect("parse temp source");
    primitive_reject_runtime_stdout_fallbacks(&src).expect("source must pass runtime guard");
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target load");
    let err = primitive_emit_witness_json(&temp, &program, &target, "freestanding")
        .expect_err("witness without operations must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_WITNESS_001"));
}

#[test]
fn primitive_signal_without_bus_fail() {
    let temp = write_temp_primitive(
        "signal_without_bus",
        "primitive[@] target x86_64:\n    signal s0:\n        source missing.port;\n    anchor:\n        boot0;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let program = read_program(&temp);
    let err = ncc_sema::analyze(&program).expect_err("signal source without port must fail");
    assert!(
        err.contains("E_PRIMITIVE_SIGNAL_UNDECLARED_001")
            || err.contains("references unknown port")
    );
}

#[test]
fn primitive_timing_without_clock_fail() {
    let target_id = format!(
        "tmp-timing-no-clock-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = false\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "timing_without_clock_contract",
        "primitive[@] target x86_64:\n    timing:\n        clock boot_clock;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor:\n        boot0;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("timing env violation must fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_TIMING_ENV_001"),
        "unexpected diagnostic: {}",
        rendered
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_latency_contract_without_clock_model_fail() {
    let target_id = format!(
        "tmp-latency-no-clock-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_clock_model = false\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "latency_without_clock_model",
        "primitive[@] target x86_64:\n    latency:\n        lat2;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("latency without clock model must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_LATENCY_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_mmio_environment_missing_mapping_fail() {
    let target_id = format!(
        "tmp-mmio-env-missing-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = false\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "mmio_environment_missing_mapping",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        mmioRead mmioaddr(0x0040) volatile;\n        read side_effect none;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing mmio mapping must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_MMIO_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_MMIO_UNSUPPORTED_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_mmio_without_side_effect_contract_fail() {
    let temp = write_temp_primitive(
        "mmio_without_volatile",
        "primitive[@] target x86_64:\n    capability c0:\n        allow port.write;\n    operation kernelEntry():\n        capability c0;\n        mmioWrite 0x0040 <- 0x01;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let program = read_program(&temp);
    let err = ncc_sema::analyze(&program).expect_err("mmio without volatile must fail");
    assert!(err.contains("E_PRIMITIVE_VOLATILE_REQUIRED_001"));
}

#[test]
fn primitive_mmio_event_without_side_effect_contract_fail() {
    let temp = write_temp_primitive(
        "mmio_event_without_side_effect_contract",
        "primitive[@] target x86_64:\n    interruptLine:\n        irq0;\n    window status:\n        cache uncached;\n    capability c0:\n        allow port.write;\n    operation kernelEntry():\n        capability c0;\n        mmioRead 0x0040 volatile;\n        event mmioReadClearsFlag status.ready;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let program = read_program(&temp);
    let err =
        ncc_sema::analyze(&program).expect_err("mmio event without side-effect contract must fail");
    assert!(err.contains("E_PRIMITIVE_MMIO_SIDE_EFFECT_001"));
}

#[test]
fn primitive_cli_check_dma_completion_without_cache_contract_fails() {
    let temp = write_temp_primitive(
        "dma_completion_cache_contract_missing",
        "primitive[@] target x86_64:\n    dmaChannel:\n        dma0;\n    dmaViolation:\n        dma_race;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event dmaCompletion dma0;\n        event physicalFault dma_race;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing dma cache contract must fail");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_CACHE_COHERENCE_VIOLATION_001"));
}

#[test]
fn primitive_dma_without_coherence_fail() {
    let temp = write_temp_primitive(
        "dma_without_coherence_fail",
        "primitive[@] target x86_64:\n    dmaChannel:\n        dma0;\n    dmaViolation:\n        dma_race;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event dmaCompletion dma0;\n        event physicalFault dma_race;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("dma without coherence contract must fail");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_CACHE_COHERENCE_VIOLATION_001"));
}

#[test]
fn primitive_cache_contract_missing_fail() {
    let temp = write_temp_primitive(
        "cache_contract_missing_fail",
        "primitive[@] target x86_64:\n    dmaChannel:\n        dma0;\n    dmaViolation:\n        dma_race;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event dmaCompletion dma0;\n        event physicalFault dma_race;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("cache contract missing must fail");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_CACHE_COHERENCE_VIOLATION_001"));
}

#[test]
fn primitive_cli_check_watchdog_without_physical_fault_contract_fails() {
    let temp = write_temp_primitive(
        "watchdog_fault_contract_missing",
        "primitive[@] target x86_64:\n    watchdog:\n        wd0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event watchdogTrip wd0;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing watchdog fault contract");
    assert!(err.to_string().contains("E_PRIMITIVE_WATCHDOG_TRIP_001"));
}

#[test]
fn primitive_cli_check_bus_arbitration_contract_missing_fails() {
    let temp = write_temp_primitive(
        "bus_arbitration_contract_missing",
        "primitive[@] target x86_64:\n    busFault:\n        bus_busy;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing bus arbitration fault contract");
    assert!(err.to_string().contains("E_PRIMITIVE_BUS_ARBITRATION_001"));
}

#[test]
fn primitive_cli_check_thermal_violation_requires_power_state_contract() {
    let temp = write_temp_primitive(
        "thermal_violation_without_power_contract",
        "primitive[@] target x86_64:\n    thermalViolation:\n        overheat;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event physicalFault overheat;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("thermal/power contract must fail");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_POWER_STATE_INVALID_001"));
}

#[test]
fn primitive_pcie_unsupported_fail() {
    let temp = write_temp_primitive(
        "pcie_unsupported",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("pcie unsupported check must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(err.to_string().contains("E_PRIMITIVE_PCIE_001"));
}

#[test]
fn primitive_pcie_transport_requires_port_io_fail() {
    let target_id = format!(
        "tmp-pcie-no-port-io-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = false\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"PCIE\"]\n"
        )
        .as_str(),
    );
    let temp = protocol_contract_fixture("pcie_transport_requires_port_io", "PCIe");
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("pcie lowering must require port-io transport");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PORT_IO_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_PORT_UNSUPPORTED_001"));
    assert!(rendered.contains("requires port I/O transport"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_pcie_explicit_config_ports_lowering_ok() {
    let target_id = format!(
        "tmp-pcie-explicit-config-ports-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_dma = true\nprotocols = [\"PCIE\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "pcie_explicit_config_ports_lowering",
        "primitive[@] target x86_64:\n    controller:\n        pcie0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe controller pcie0 lane lane0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp.clone(), &target_name, "freestanding")
        .expect("pcie explicit config-port lowering fixture must pass check");

    let program = read_program(&temp);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extraction must pass");

    let mut saw_pcie = false;
    for op in &contract.ops {
        let PrimitiveMaterialOp::ProtocolPcie {
            config_addr_port,
            config_data_port,
            ..
        } = op
        else {
            continue;
        };
        saw_pcie = true;
        assert_eq!(*config_addr_port, 0x0CF8);
        assert_eq!(*config_data_port, 0x0CFC);
        let lowered = primitive_material_op_lowering_bytes(op, "kernelEntry")
            .expect("pcie lowering bytes must materialize");
        let lowered_hex = primitive_bytes_to_hex(&lowered);
        assert!(
            lowered_hex.contains("baf80cb080eebafc0cec"),
            "pcie lowering must materialize explicit cfg-addr/cfg-data IO cycle"
        );
    }
    assert!(saw_pcie, "expected ProtocolPcie op in boot contract");
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_pcie_config_ports_pair_required_fail() {
    let target_id = format!(
        "tmp-pcie-config-pair-required-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_dma = true\nprotocols = [\"PCIE\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "pcie_config_ports_pair_required",
        "primitive[@] target x86_64:\n    controller:\n        pcie0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe controller pcie0 lane lane0 configAddrPort pcie.cfg_addr;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("pcie configAddrPort without configDataPort must fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_PROTOCOL_001"),
        "expected protocol diagnostic, got: {}",
        rendered
    );
    assert!(rendered.contains("configAddrPort"));
    assert!(rendered.contains("configDataPort"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_pcie_config_ports_cannot_mix_with_data_port_fail() {
    let target_id = format!(
        "tmp-pcie-config-mix-data-port-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_dma = true\nprotocols = [\"PCIE\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "pcie_config_mix_data_port",
        "primitive[@] target x86_64:\n    controller:\n        pcie0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe controller pcie0 lane lane0 dataPort dev.data configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("pcie dataPort/configAddrPort mix must fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_PROTOCOL_001"),
        "expected protocol diagnostic, got: {}",
        rendered
    );
    assert!(rendered.contains("cannot mix"));
    assert!(rendered.contains("configAddrPort"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_config_ports_qualifier_forbidden_fail() {
    let target_id = format!(
        "tmp-usb-config-qualifier-forbidden-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_dma = true\nsupports_cache_model = true\nprotocols = [\"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_config_qualifier_forbidden",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    peripheral:\n        usbdev0;\n    window usbwin0:\n        addr mmioaddr(0x1100);\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller usb0 peripheral usbdev0 window usbwin0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data;\n        write pcie.cfg_addr <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb protocol must reject pcie config-port qualifiers");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_PROTOCOL_001"),
        "expected protocol diagnostic, got: {}",
        rendered
    );
    assert!(
        rendered.contains("only allows configAddrPort/configDataPort"),
        "expected config-qualifier restriction, got: {}",
        rendered
    );
    assert!(
        rendered.contains("PCIE"),
        "expected PCIE reference in diagnostic, got: {}",
        rendered
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_pcie_explicit_config_unknown_path_fail() {
    let target_id = format!(
        "tmp-pcie-config-unknown-path-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_dma = true\nprotocols = [\"PCIE\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "pcie_explicit_config_unknown_path",
        "primitive[@] target x86_64:\n    controller:\n        pcie0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe controller pcie0 lane lane0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data;\n        write pcie.cfg_addr <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("pcie configDataPort unknown path must fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_PROTOCOL_001"),
        "expected protocol diagnostic, got: {}",
        rendered
    );
    assert!(
        rendered.contains("unknown port path"),
        "expected unknown-port-path message, got: {}",
        rendered
    );
    assert!(
        rendered.contains("pcie.cfg_data"),
        "expected missing configData path in diagnostic, got: {}",
        rendered
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_uart_without_protocol_fail() {
    let target_id = format!(
        "tmp-no-uart-protocol-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nprotocols = []\n"
        )
        .as_str(),
    );
    let temp = protocol_contract_fixture("uart_without_protocol", "UART");
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("uart protocol missing on target must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_UART_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_unsupported_fail() {
    let temp = protocol_contract_fixture("usb_unsupported", "USB");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("usb unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_USB_001"));
}

#[test]
fn primitive_usb_without_controller_fail() {
    let target_id = format!(
        "tmp-usb-protocol-no-controller-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_cache_model = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = protocol_contract_fixture("usb_without_controller", "USB");
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb without controller declaration must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
    assert!(rendered.contains("requires explicit controller declaration"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_without_peripheral_fail() {
    let target_id = format!(
        "tmp-usb-protocol-no-peripheral-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_cache_model = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_without_peripheral",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb without peripheral declaration must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
    assert!(rendered.contains("requires explicit peripheral declaration"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_transport_requires_mmio_fail() {
    let target_id = format!(
        "tmp-usb-no-mmio-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = false\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_transport_requires_mmio",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    peripheral:\n        usbdev0;\n    window usbwin0:\n        cache uncached;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller usb0 peripheral usbdev0 window usbwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb lowering must require mmio transport");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_MMIO_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_MMIO_UNSUPPORTED_001"));
    assert!(rendered.contains("requires MMIO transport"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_window_without_mmio_addr_fail() {
    let target_id = format!(
        "tmp-usb-window-no-mmio-addr-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_cache_model = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_window_without_mmio_addr",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    peripheral:\n        usbdev0;\n    window usbwin0:\n        cache uncached;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller usb0 peripheral usbdev0 window usbwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb protocol must require window addr mmio contract");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_MMIO_ENV_001"),
        "expected MMIO env diagnostic, got: {}",
        rendered
    );
    assert!(
        rendered.contains("window 'usbwin0'"),
        "expected window symbol in diagnostic, got: {}",
        rendered
    );
    assert!(
        rendered.contains("addr mmioaddr"),
        "expected addr mmioaddr contract hint, got: {}",
        rendered
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_usb_window_with_physaddr_contract_fail() {
    let target_id = format!(
        "tmp-usb-window-physaddr-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_window_with_physaddr_contract",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    peripheral:\n        usbdev0;\n    window usbwin0:\n        addr physaddr(0x1100);\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller usb0 peripheral usbdev0 window usbwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("usb protocol must reject non-mmio window addr helper");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_MMIO_ENV_001"),
        "expected MMIO env diagnostic, got: {}",
        rendered
    );
    assert!(
        rendered.contains("addr mmioaddr"),
        "expected mmioaddr contract hint, got: {}",
        rendered
    );
    assert!(
        rendered.contains("non-MMIO"),
        "expected explicit non-MMIO contract explanation, got: {}",
        rendered
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_i2c_transport_requires_port_io_fail() {
    let target_id = format!(
        "tmp-i2c-no-port-io-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = false\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"I2C\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "i2c_transport_requires_port",
        "primitive[@] target x86_64:\n    controller:\n        i2c0;\n    channel:\n        ch0;\n    capability hw:\n        allow protocol.i2c;\n    operation kernelEntry():\n        capability hw;\n        protocol I2C;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("i2c lowering must require port-io transport");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PORT_IO_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_PORT_UNSUPPORTED_001"));
    assert!(rendered.contains("requires port I/O transport"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_sata_unsupported_fail() {
    let temp = protocol_contract_fixture("sata_unsupported", "SATA");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("sata unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_SATA_001"));
}

#[test]
fn primitive_nvme_unsupported_fail() {
    let temp = protocol_contract_fixture("nvme_unsupported", "NVMe");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("nvme unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_NVME_001"));
}

#[test]
fn primitive_i2c_unsupported_fail() {
    let temp = protocol_contract_fixture("i2c_unsupported", "I2C");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("i2c unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_I2C_001"));
}

#[test]
fn primitive_spi_unsupported_fail() {
    let temp = protocol_contract_fixture("spi_unsupported", "SPI");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("spi unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_SPI_001"));
}

#[test]
fn primitive_ethernet_unsupported_fail() {
    let temp = protocol_contract_fixture("ethernet_unsupported", "Ethernet");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("ethernet unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_ETHERNET_001"));
}

#[test]
fn primitive_hdmi_unsupported_fail() {
    let temp = protocol_contract_fixture("hdmi_unsupported", "HDMI");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("hdmi unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_DISPLAY_001"));
}

#[test]
fn primitive_displayport_unsupported_fail() {
    let temp = protocol_contract_fixture("displayport_unsupported", "DisplayPort");
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("displayport unsupported must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_ENV_001"));
    assert!(rendered.contains("E_PRIMITIVE_DISPLAY_001"));
}

#[test]
fn primitive_sensor_without_peripheral_fail() {
    let temp = write_temp_primitive(
        "sensor_without_peripheral",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port sensor0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes sensor0.data;\n        write sensor0.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("sensor surface without peripheral declaration must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
    assert!(rendered.contains("sensor surface requires explicit peripheral declaration"));
}

#[test]
fn primitive_motor_without_controller_fail() {
    let temp = write_temp_primitive(
        "motor_without_controller",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port motor0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes motor0.data;\n        write motor0.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("motor surface without controller declaration must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
    assert!(rendered.contains("motor surface requires explicit controller declaration"));
}

#[test]
fn primitive_radio_without_rf_capability_fail() {
    let temp = write_temp_primitive(
        "radio_without_rf_capability",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port radio0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes radio0.data;\n        write radio0.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("radio surface without RF capability must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_CAPABILITY_001"));
    assert!(rendered.contains("radio surface requires target RF capability"));
}

#[test]
fn primitive_screen_without_display_link_fail() {
    let temp = write_temp_primitive(
        "screen_without_display_link",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port screen0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes screen0.data;\n        write screen0.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("screen surface without display link protocol must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_DISPLAY_001"));
    assert!(
        rendered.contains("display surface requires explicit HDMI/DisplayPort protocol contract")
    );
}

#[test]
fn primitive_usb_with_controller_ok_when_target_supports_usb() {
    let target_id = format!(
        "tmp-usb-supported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"USB\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "usb_supported_with_controller",
        "primitive[@] target x86_64:\n    controller:\n        usb0;\n    peripheral:\n        usbdev0;\n    window usbwin0:\n        addr mmioaddr(0x1100);\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller usb0 peripheral usbdev0 window usbwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, &target_name, "freestanding")
        .expect("usb must pass when target capability + controller contract are declared");
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_radio_with_rf_capability_ok() {
    let target_id = format!(
        "tmp-rf-supported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_rf = true\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "radio_surface_supported",
        "primitive[@] target x86_64:\n    signal:\n        radio_ready;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, &target_name, "freestanding")
        .expect("radio surface must pass when target declares RF capability");
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_screen_with_displayport_protocol_ok() {
    let target_id = format!(
        "tmp-displayport-supported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"DISPLAYPORT\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "screen_surface_supported",
        "primitive[@] target x86_64:\n    controller:\n        display0;\n    peripheral:\n        screen0;\n    phy:\n        dp_phy0;\n    lane:\n        lane0;\n    window dpwin0:\n        addr mmioaddr(0x1200);\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol DisplayPort controller display0 phy dp_phy0 lane lane0 window dpwin0;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, &target_name, "freestanding")
        .expect("display surface must pass when display protocol + controller are declared");
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_displayport_without_lane_fail() {
    let target_id = format!(
        "tmp-displayport-no-lane-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\", \"DISPLAYPORT\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "displayport_without_lane",
        "primitive[@] target x86_64:\n    controller:\n        display0;\n    phy:\n        dp_phy0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol DisplayPort;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, &target_name, "freestanding")
        .expect_err("displayport without lane declaration must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
    assert!(rendered.contains("requires explicit lane declaration"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_lane_phy_controller_chain_ok() {
    let temp = write_temp_primitive(
        "lane_phy_controller_chain",
        "primitive[@] target x86_64:\n    controller:\n        net0;\n    phy:\n        phy0;\n    lane:\n        lane0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect("controller->phy->lane chain must pass");
}

#[test]
fn primitive_channel_with_peripheral_ok() {
    let temp = write_temp_primitive(
        "channel_with_peripheral",
        "primitive[@] target x86_64:\n    peripheral:\n        uart0;\n    channel:\n        ch0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect("channel with peripheral declaration must pass");
}

#[test]
fn primitive_sensor_with_peripheral_ok() {
    let temp = write_temp_primitive(
        "sensor_with_peripheral",
        "primitive[@] target x86_64:\n    peripheral:\n        sensor0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect("sensor surface must pass when explicit peripheral exists");
}

#[test]
fn primitive_motor_with_controller_ok() {
    let temp = write_temp_primitive(
        "motor_with_controller",
        "primitive[@] target x86_64:\n    controller:\n        motor0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect("motor surface must pass when explicit controller exists");
}

#[test]
fn primitive_electrical_model_unsupported_fail() {
    let temp = write_temp_primitive(
        "electrical_model_unsupported",
        "primitive[@] target x86_64:\n    voltage:\n        vcore;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("electrical model unsupported must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_ELECTRICAL_001"));
}

#[test]
fn primitive_dma_without_target_dma_capability_fail() {
    let temp = write_temp_primitive(
        "dma_capability_missing",
        "primitive[@] target x86_64:\n    dmaChannel:\n        ch0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("dma capability missing must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_DMA_001"));
}

#[test]
fn primitive_pipeline_without_target_capability_fail() {
    let target_id = format!(
        "tmp-pipeline-no-model-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_pipeline_model = false\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "pipeline_without_target_capability",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        pipeline stage0;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("pipeline capability missing must fail");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_TARGET_CAPABILITY_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_thermal_state_without_target_thermal_model_fail() {
    let temp = write_temp_primitive(
        "thermal_state_without_target_model",
        "primitive[@] target x86_64:\n    thermalState:\n        t0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("thermalState must require thermal model");
    assert!(err.to_string().contains("E_PRIMITIVE_THERMAL_001"));
}

#[test]
fn primitive_power_gate_without_target_power_model_fail() {
    let temp = write_temp_primitive(
        "power_gate_without_target_model",
        "primitive[@] target x86_64:\n    powerGate:\n        pg0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("powerGate must require power model");
    assert!(err.to_string().contains("E_PRIMITIVE_POWER_001"));
}

#[test]
fn primitive_cache_state_without_target_cache_model_fail() {
    let temp = write_temp_primitive(
        "cache_state_without_target_model",
        "primitive[@] target x86_64:\n    cacheState:\n        l1_clean;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("cacheState must require cache model");
    assert!(err.to_string().contains("E_PRIMITIVE_CACHE_001"));
}

#[test]
fn primitive_tlb_state_without_target_tlb_model_fail() {
    let temp = write_temp_primitive(
        "tlb_state_without_target_model",
        "primitive[@] target x86_64:\n    tlbState:\n        hot0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("tlbState must require tlb model");
    assert!(err.to_string().contains("E_PRIMITIVE_TLB_001"));
}

#[test]
fn primitive_pipeline_state_without_target_pipeline_model_fail() {
    let temp = write_temp_primitive(
        "pipeline_state_without_target_model",
        "primitive[@] target x86_64:\n    pipelineState:\n        decode0;\n    branchPredictorState:\n        bp0;\n    reorderState:\n        rob0;\n    outOfOrder:\n        ooo0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("pipelineState family must require pipeline model");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_TARGET_CAPABILITY_001"));
}

#[test]
fn primitive_firmware_region_without_target_firmware_model_fail() {
    let temp = write_temp_primitive(
        "firmware_model_missing",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("firmware model missing must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_firmware_missing_fail() {
    let temp = write_temp_primitive(
        "firmware_missing_fail",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("firmware model missing must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_firmware_environment_capability_missing_fail() {
    let temp = write_temp_primitive(
        "firmware_environment_capability_missing",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        firmware bios;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("firmware env capability must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_ENV_001"));
}

#[test]
fn primitive_bios_region_without_support_fail() {
    let temp = write_temp_primitive(
        "bios_region_without_support",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    biosRegion:\n        bios0;\n    resetVector:\n        reset0;\n    bootWindow:\n        bootwin0;\n    bootAnchor:\n        boot0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("bios region unsupported must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_ENV_001"));
}

#[test]
fn primitive_uefi_handoff_without_support_fail() {
    let temp = write_temp_primitive(
        "uefi_handoff_without_support",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    uefiHandoff:\n        handoff0;\n    firmwareTable:\n        fwtable0;\n    deviceTree:\n        dt0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("uefi handoff unsupported must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_ENV_001"));
}

#[test]
fn primitive_firmware_table_without_firmware_model_fail() {
    let temp = write_temp_primitive(
        "firmware_table_without_model",
        "primitive[@] target x86_64:\n    firmwareTable:\n        fwtable0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("firmware table without model must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_microcode_state_without_target_declared_fail() {
    let target_id = format!(
        "tmp-microcode-opaque-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\npointer_width = 64\npage_size = 4096\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_firmware_model = true\nprotocols = [\"UART\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"supported\"\nfirmware_microcode = \"opaque\"\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "microcode_state_without_target_declared",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    firmwareTable:\n        fwtable0;\n    deviceTree:\n        dt0;\n    microcodeState:\n        patch_state0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("microcode state must fail on opaque target");
    assert!(err.to_string().contains("E_PRIMITIVE_MICROCODE_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_firmware_boot_entities_ok_with_supported_target() {
    let target_id = format!(
        "tmp-firmware-boot-supported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\npointer_width = 64\npage_size = 4096\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_firmware_model = true\nprotocols = [\"UART\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"supported\"\nfirmware_microcode = \"target_declared\"\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "firmware_boot_entities_supported",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    bootRomRegion:\n        rom0;\n    biosRegion:\n        bios0;\n    uefiHandoff:\n        handoff0;\n    resetVector:\n        reset0;\n    bootWindow:\n        bootwin0;\n    bootAnchor:\n        boot0;\n    firmwareTable:\n        fwtable0;\n    deviceTree:\n        dt0;\n    acpiTable:\n        acpi0;\n    microcodeState:\n        patch_state0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    run_primitive_check(temp, &target_name, "freestanding")
        .expect("firmware/boot directives should pass on a fully-declared target");
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_cli_check_reset_vector_without_boot_window_fails() {
    let temp = write_temp_primitive(
        "reset_vector_without_boot_window",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    resetVector:\n        reset0;\n    bootAnchor:\n        boot0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing bootWindow must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_cli_check_firmware_table_requires_platform_table_fail() {
    let temp = write_temp_primitive(
        "firmware_table_requires_platform_table",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    firmwareTable:\n        fwtable0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err =
        run_primitive_command(&args).expect_err("firmwareTable without platform table must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_cli_check_uefi_requires_firmware_table_fail() {
    let temp = write_temp_primitive(
        "uefi_requires_firmware_table",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    uefiHandoff:\n        handoff0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err =
        run_primitive_command(&args).expect_err("uefiHandoff without firmwareTable must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_cli_check_bios_requires_reset_and_boot_window_fail() {
    let temp = write_temp_primitive(
        "bios_requires_reset_and_boot_window",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    biosRegion:\n        bios0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args)
        .expect_err("biosRegion without resetVector/bootWindow must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_FIRMWARE_001"));
}

#[test]
fn primitive_cli_check_microcode_requires_firmware_table_fail() {
    let temp = write_temp_primitive(
        "microcode_requires_firmware_table",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    microcodeState:\n        patch0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err =
        run_primitive_command(&args).expect_err("microcodeState without firmwareTable must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_FIRMWARE_001"));
    assert!(rendered.contains("microcodeState"));
}

#[test]
fn primitive_bootrom_without_support_fail() {
    let target_id = format!(
        "tmp-bootrom-unsupported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\npointer_width = 64\npage_size = 4096\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_firmware_model = true\nprotocols = [\"UART\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"unsupported\"\nfirmware_microcode = \"opaque\"\n"
        )
        .as_str(),
    );
    let temp = write_temp_primitive(
        "bootrom_unsupported",
        "primitive[@] target x86_64:\n    firmwareRegion:\n        fw0;\n    bootRomRegion:\n        rom0;\n    resetVector:\n        reset0;\n    bootWindow:\n        bootwin0;\n    bootAnchor:\n        boot0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("bootrom unsupported must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_BOOTROM_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_microcode_control_forbidden_fail() {
    let temp = write_temp_primitive(
        "microcode_control_forbidden",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        microcode patch0;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("microcode control must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_MICROCODE_001"));
}

#[test]
fn primitive_physical_fault_without_rupture_fail() {
    let temp = write_temp_primitive(
        "physical_fault_without_rupture",
        "primitive[@] target x86_64:\n    physicalFault:\n        thermal_trip;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("physical fault without rupture must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_PHYSICAL_FAULT_001"));
}

#[test]
fn primitive_interrupt_line_without_interrupt_capability_fail() {
    let target_name = write_temp_target(
        "tmp-interrupt-line-no-cap",
        "name = \"tmp-interrupt-line-no-cap\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = false\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_clock_model = true\nprotocols = [\"UART\"]\n",
    );
    let temp = write_temp_primitive(
        "interrupt_line_without_capability",
        "primitive[@] target x86_64:\n    interruptLine:\n        irq0;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("interrupt capability must fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_INTERRUPT_001"),
        "unexpected diagnostic: {rendered}"
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_cli_check_rejects_unknown_event_interrupt_line() {
    let temp = write_temp_primitive(
        "unknown_event_interrupt_line",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event interrupt irq_missing;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    entry kernelEntry:\n        anchor boot0;\n        mode real16;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("unknown interrupt event must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_INTERRUPT_001"));
}

#[test]
fn primitive_port_io_on_aarch64_without_support_fail() {
    let target_id = format!(
        "tmp-aarch64-no-portio-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"aarch64\"\nisa = \"aarch64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = false\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("aarch64 port io should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_PORT_IO_ENV_001")
            || rendered.contains("E_PRIMITIVE_TARGET_MATERIAL_001"),
        "unexpected diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("supports_port_io")
            || rendered.contains("E_PRIMITIVE_PORT_UNSUPPORTED_001"),
        "unexpected diagnostic: {rendered}"
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_qemu_physical_certification_fail() {
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu check must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_witness_strict_qemu_certification_fail() {
    let args = vec![
        "witness".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu witness must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_map_strict_qemu_certification_fail() {
    let args = vec![
        "map".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu map must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_map_rejects_invalid_non_linear_event_contracts() {
    let temp = write_temp_primitive(
        "map_rejects_invalid_non_linear_event_contracts",
        "primitive[@] target x86_64:\n    interruptLine:\n        irq0;\n    timingViolation:\n        missed_deadline;\n    physicalFault:\n        thermal_trip;\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port uart0.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        event interrupt irq0;\n        event physicalFault thermal_trip;\n        write uart0.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "map".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args)
        .expect_err("map must fail when non-linear event contracts are semantically incomplete");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_INTERRUPT_001"));
    assert!(rendered.contains("timingViolation"));
}

#[test]
fn primitive_cli_lower_strict_qemu_certification_fail() {
    let args = vec![
        "lower".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu lower must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_build_strict_qemu_certification_fail() {
    let args = vec![
        "build".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu build must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_report_strict_qemu_certification_fail() {
    let args = vec![
        "report".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict qemu report must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_map_without_primitive_container_fail() {
    let args = vec![
        "map".to_string(),
        primitive_fixture_path("../root_at/root_at_final_keyword_ok.n"),
    ];
    let err = run_primitive_command(&args).expect_err("non-primitive source must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
}

#[test]
fn primitive_unsupported_target_material_fail() {
    let args = vec![
        "map".to_string(),
        primitive_fixture_path("../root_at/root_at_final_keyword_ok.n"),
    ];
    let err = run_primitive_command(&args).expect_err("non-primitive source must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_TARGET_MATERIAL_001"));
}

#[test]
fn primitive_runtime_fallback_fail() {
    let temp = write_temp_primitive(
        "runtime_fallback_fail",
        "primitive[@] target x86_64:\n    operation kernelEntry():\n        runtime.output(\"x\");\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("runtime fallback must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_RUNTIME_001"));
}

#[test]
fn primitive_stdout_simulation_fail() {
    let temp = write_temp_primitive(
        "stdout_simulation_fail",
        "primitive[@] target x86_64:\n    operation kernelEntry():\n        println(\"fake\");\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(temp, "x86_64-unknown-none", "freestanding")
        .expect_err("stdout fallback must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_STDOUT_FAKE_001"));
}

#[test]
fn primitive_cli_doctor_runs() {
    run_primitive_command(&["doctor".to_string()]).expect("primitive doctor should run");
}

#[test]
fn primitive_cli_doctor_json_runs() {
    run_primitive_command(&["doctor".to_string(), "--json".to_string()])
        .expect("primitive doctor --json should run");
}

#[test]
fn primitive_cli_doctor_reports_qemu_missing_without_panic() {
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_ok()
    {
        return;
    }
    let err = run_primitive_command(&["doctor".to_string(), "--strict".to_string()])
        .expect_err("strict doctor must report missing qemu");
    assert!(err.to_string().contains("E_PRIMITIVE_CLI_QEMU_MISSING_001"));
}

#[test]
fn primitive_cli_doctor_strict_qemu_certification_fail() {
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let args = vec![
        "doctor".to_string(),
        "--strict".to_string(),
        "--target".to_string(),
        "x86_64-qemu".to_string(),
        "--environment".to_string(),
        "emulated-qemu".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict doctor qemu must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_check_hardware_identity_uart_ok() {
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    run_primitive_command(&args).expect("primitive check should pass");
}

#[test]
fn primitive_cli_check_rejects_runtime() {
    let temp = write_temp_primitive(
        "cli_check_rejects_runtime",
        "primitive[@] target x86_64:\n    operation kernelEntry():\n        runtime.output(\"x\");\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("check must reject runtime");
    assert!(err.to_string().contains("E_PRIMITIVE_RUNTIME_001"));
}

#[test]
fn primitive_cli_check_rejects_stdout_fallback() {
    let temp = write_temp_primitive(
        "cli_check_rejects_stdout_fallback",
        "primitive[@] target x86_64:\n    operation kernelEntry():\n        println(\"fake\");\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "check".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("check must reject stdout fallback");
    assert!(err.to_string().contains("E_PRIMITIVE_STDOUT_FAKE_001"));
}

#[test]
fn primitive_cli_witness_missing_material_fails() {
    let temp = write_temp_primitive(
        "cli_witness_missing_material",
        "primitive[@] target x86_64:\n    anchor:\n        boot0;\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec!["witness".to_string(), temp, "--json".to_string()];
    let err = run_primitive_command(&args).expect_err("witness without material must fail");
    assert!(
        err.to_string().contains("E_PRIMITIVE_WITNESS_001")
            || err
                .to_string()
                .contains("E_PRIMITIVE_LOWERING_UNAVAILABLE_001")
    );
}

#[test]
fn primitive_cli_report_writes_markdown() {
    let source = event_contracts_fixture();
    let md_out = std::env::temp_dir().join(format!(
        "ncc_primitive_report_md_{}_{}.md",
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let json_out = std::env::temp_dir().join(format!(
        "ncc_primitive_report_json_{}_{}.json",
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let args = vec![
        "report".to_string(),
        source,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--md-out".to_string(),
        md_out.to_string_lossy().to_string(),
        "--json-out".to_string(),
        json_out.to_string_lossy().to_string(),
    ];
    run_primitive_command(&args).expect("primitive report should succeed");
    let md_raw = std::fs::read_to_string(&md_out).expect("markdown report must be readable");
    assert!(md_raw.contains("non_linear_event_contracts: 6"));
    assert!(md_raw.contains("event_contracts_verified: true"));
    assert!(md_raw.contains("mmioReadClearsFlag: 1"));
    let _ = std::fs::remove_file(md_out);
    let _ = std::fs::remove_file(json_out);
}

#[test]
fn primitive_cli_report_writes_json() {
    let source = event_contracts_fixture();
    let md_out = std::env::temp_dir().join(format!(
        "ncc_primitive_report_md_{}_{}.md",
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let json_out = std::env::temp_dir().join(format!(
        "ncc_primitive_report_json_{}_{}.json",
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let args = vec![
        "report".to_string(),
        source,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--md-out".to_string(),
        md_out.to_string_lossy().to_string(),
        "--json-out".to_string(),
        json_out.to_string_lossy().to_string(),
    ];
    run_primitive_command(&args).expect("primitive report should succeed");
    let json_raw = std::fs::read_to_string(&json_out).expect("json report must be readable");
    let value: serde_json::Value =
        serde_json::from_str(&json_raw).expect("json report must be valid json");
    assert_eq!(
        value.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.report.v1")
    );
    assert_eq!(
        value
            .get("event_audit")
            .and_then(|v| v.get("total"))
            .and_then(|v| v.as_u64()),
        Some(6)
    );
    assert_eq!(
        value
            .get("event_audit")
            .and_then(|v| v.get("async_count"))
            .and_then(|v| v.as_u64()),
        Some(6)
    );
    assert_eq!(
        value
            .get("material_contract_checks")
            .and_then(|v| v.get("event_contracts_verified"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        value
            .get("witness")
            .and_then(|v| v.get("event_contracts"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.len()),
        Some(6)
    );
    let _ = std::fs::remove_file(md_out);
    let _ = std::fs::remove_file(json_out);
}

#[test]
fn primitive_cli_witness_uart_contains_port_window() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    let has_port_window = witness
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("material_entity").and_then(|v| v.as_str()) == Some("portWindow")
                    && row.get("port").and_then(|v| v.as_str()) == Some("0x03F8")
                    && row
                        .get("lowering_machine_bytes_hex")
                        .and_then(|v| v.as_str())
                        .map(|hex| !hex.is_empty())
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        has_port_window,
        "witness must include COM1 portWindow operation"
    );
}

#[test]
fn primitive_cli_witness_json_is_valid() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let witness = primitive_emit_witness_json(&path, &program, &target, "freestanding")
        .expect("witness generation must pass");
    let raw = serde_json::to_string_pretty(&witness).expect("witness json serialize");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("witness json parse");
    assert_eq!(
        parsed.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.material.witness.v1")
    );
}

#[test]
fn primitive_cli_map_contains_com1_port_window() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let mut windows = Vec::new();
    for section in &primitive.sections {
        if let ncc_ast::PrimitiveAtSection::WindowBlock(s) = section {
            windows.push(s.name.join("."));
        }
    }
    assert!(
        windows.iter().any(|name| name == "com1_port_window"),
        "map projection must include com1_port_window"
    );
}

#[test]
fn primitive_cli_map_json_is_valid() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let mut anchors = Vec::new();
    let mut windows = Vec::new();
    let mut signals = Vec::new();
    let mut ports = Vec::new();
    let mut entries = Vec::new();
    for section in &primitive.sections {
        match section {
            ncc_ast::PrimitiveAtSection::AnchorBlock(s) => anchors.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::WindowBlock(s) => windows.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::SignalBlock(s) => signals.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::Port(s) => ports.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::Entry(s) => entries.push(s.name.join(".")),
            _ => {}
        }
    }
    let payload = serde_json::json!({
        "schema": "nc.primitive.map.v1",
        "source": path,
        "anchors": anchors,
        "windows": windows,
        "signals": signals,
        "ports": ports,
        "entries": entries,
        "directive_kinds": primitive_directive_kind_labels(&program),
        "material_semantics": primitive_material_semantic_labels(&program),
        "firmware_boot_model": primitive_firmware_boot_model_json(&primitive),
        "protocols_required": primitive_collect_protocol_requirements(&program).into_iter().collect::<Vec<_>>(),
    });
    let raw = serde_json::to_string_pretty(&payload).expect("map json serialize");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("map json parse");
    assert_eq!(
        parsed.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.map.v1")
    );
    assert_eq!(
        parsed
            .get("firmware_boot_model")
            .and_then(|v| v.get("verified"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn primitive_cli_lower_contains_port_write_lowering() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extract");
    assert!(
        contract
            .ops
            .iter()
            .any(|op| matches!(op, PrimitiveMaterialOp::PortWrite { .. })),
        "lowering contract must include at least one port write"
    );
}

#[test]
fn primitive_cli_lower_json_is_valid() {
    let path = primitive_fixture_path("hardware_identity_uart.ncp");
    let program = read_program(&path);
    let target = load_primitive_target_spec("x86_64-unknown-none").expect("target spec load");
    let entry_name =
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".into());
    validate_primitive_material_build_requirements(
        &program,
        Some(target.id.as_str()),
        Some("freestanding"),
        Some("link/x86_64_baremetal.ld"),
        Some(entry_name.as_str()),
    )
    .expect("validation must pass");
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extraction must pass");
    let raw = emit_primitive_boot_sector(&contract).expect("emit boot image");
    let event_contracts_json = primitive_event_contracts_to_json(
        &contract.event_contracts,
        &contract.side_effect_contracts,
    );
    let event_contract_total = event_contracts_json.len();
    let event_contract_async = event_contracts_json
        .iter()
        .filter(|row| {
            row.get("async_hardware_event")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .count();
    let event_contract_verified = event_contracts_json.iter().all(|row| {
        let side_effects_ok = row
            .get("side_effects")
            .and_then(|v| v.as_array())
            .map(|cols| !cols.is_empty())
            .unwrap_or(false);
        let mmio_shape_ok =
            if row.get("kind").and_then(|v| v.as_str()) == Some("mmioReadClearsFlag") {
                row.get("subject")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains('.'))
                    .unwrap_or(false)
            } else {
                true
            };
        side_effects_ok && mmio_shape_ok
    });
    let payload = serde_json::json!({
        "schema": "nc.primitive.lower.v1",
        "source": path,
        "target": target.id,
        "environment": "freestanding",
        "entry": entry_name,
        "directive_kinds": primitive_directive_kind_labels(&program),
        "material_semantics": primitive_material_semantic_labels(&program),
        "firmware_boot_model": primitive_firmware_boot_model_json(&primitive),
        "event_contracts": event_contracts_json,
        "non_linear_event_contracts_audit": {
            "total": event_contract_total,
            "async_count": event_contract_async,
            "verified": event_contract_verified
        },
        "path": ["primitive[@]", "primitive IR", "machine IR", "backend", "artifact"],
        "artifact_kind": "raw_boot_image",
        "artifact_size": raw.len(),
    });
    let json = serde_json::to_string_pretty(&payload).expect("lower json serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("lower json parse");
    assert_eq!(
        parsed.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.lower.v1")
    );
    assert_eq!(
        parsed
            .get("firmware_boot_model")
            .and_then(|v| v.get("verified"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        parsed
            .get("non_linear_event_contracts_audit")
            .and_then(|v| v.get("verified"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn primitive_cli_lower_advanced_material_json_reports_entity_contract_hook_ops() {
    let target_name = format!(
        "tmp-advanced-lower-json-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_name,
        format!(
            "name = \"{target_name}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_serial_com1 = true\nsupports_dma = true\nsupports_cache_model = true\nsupports_tlb_model = true\nsupports_pipeline_model = true\nsupports_firmware_model = true\nsupports_electrical_model = true\nsupports_clock_model = true\nsupports_edge_model = true\nsupports_metastability_model = true\nsupports_thermal_model = true\nsupports_power_model = true\nprotocols = [\"UART\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"supported\"\nfirmware_microcode = \"target_declared\"\n"
        )
        .as_str(),
    );
    let path = advanced_physical_semantics_fixture();
    let program = read_program(&path);
    let target = load_primitive_target_spec(&target_name).expect("target spec load");
    let entry_name =
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".into());
    validate_primitive_material_build_requirements(
        &program,
        Some(target.id.as_str()),
        Some("freestanding"),
        Some("link/x86_64_baremetal.ld"),
        Some(entry_name.as_str()),
    )
    .expect("validation must pass");
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extraction must pass");
    let raw = emit_primitive_boot_sector(&contract).expect("emit boot image");
    let lowered_ops: Vec<String> = contract
        .ops
        .iter()
        .map(|op| primitive_material_op_kind(op).to_string())
        .collect();
    let mut lowered_op_sequences = Vec::new();
    let mut protocol_contracts = Vec::new();
    let mut event_hooks = Vec::new();
    let mut material_entity_contract_hooks = Vec::new();
    let mut semantic_probe_phases: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut semantic_probe_labels: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut probe_profile_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for op in &contract.ops {
        let lowered = primitive_material_op_lowering_bytes(op, entry_name.as_str())
            .expect("advanced lowering bytes must be available");
        let lowered_hex = primitive_bytes_to_hex(&lowered);
        lowered_op_sequences.push(serde_json::json!({
            "kind": primitive_material_op_kind(op),
            "machine_bytes_hex": lowered_hex.clone(),
            "machine_bytes_len": lowered.len(),
        }));
        match op {
            _ if primitive_protocol_op_view(op).is_some() => {
                let view = primitive_protocol_op_view(op)
                    .expect("protocol view must exist for protocol material op");
                protocol_contracts.push(serde_json::json!({
                    "kind": primitive_material_op_kind(op),
                    "protocol": view.protocol,
                    "lowering_mode": primitive_protocol_lowering_mode(view.protocol),
                    "transport": primitive_protocol_transport(view.protocol),
                    "protocol_marker": format!("0x{:02X}", primitive_protocol_marker_byte(view.protocol)),
                    "controller": view.controller,
                    "peripheral": view.peripheral,
                    "phy": view.phy,
                    "lane": view.lane,
                    "channel": view.channel,
                    "window": view.window,
                    "window_addr": view.window_addr.map(|addr| format!("0x{addr:08X}")),
                    "data_port": view.data_port.map(|port| format!("0x{port:04X}")),
                    "status_port": view.status_port.map(|port| format!("0x{port:04X}")),
                    "control_port": view.control_port.map(|port| format!("0x{port:04X}")),
                    "config_addr_port": view.config_addr_port.map(|port| format!("0x{port:04X}")),
                    "config_data_port": view.config_data_port.map(|port| format!("0x{port:04X}")),
                    "lowering_prefix_hex": primitive_bytes_to_hex(primitive_protocol_lowering_prefix_bytes(view.protocol)),
                    "lowering_suffix_hex": primitive_bytes_to_hex(primitive_protocol_lowering_suffix_bytes(view.protocol)),
                    "lowering_semantics": primitive_protocol_lowering_semantics(view.protocol),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            event_op @ (PrimitiveMaterialOp::LegacyEventContract { .. }
            | PrimitiveMaterialOp::PhysicalEventContract { .. }
            | PrimitiveMaterialOp::AsyncHardwareEventContract { .. }) => {
                let view =
                    primitive_event_op_view(event_op).expect("event view must exist for event op");
                event_hooks.push(serde_json::json!({
                    "construct_kind": view.construct.as_str(),
                    "nominal_event_contract": view.construct.is_nominal(),
                    "async_hardware_event_construct": view.construct.is_async_nominal(),
                    "event_kind": view.kind.as_str(),
                    "material_entity": view.kind.material_entity(),
                    "subject": view.subject,
                    "async_hardware_event": view.kind.async_hardware_event(),
                    "marker": format!("0x{:02X}", view.kind.marker_byte()),
                    "lowering_prefix_hex": primitive_bytes_to_hex(view.kind.lowering_prefix_bytes()),
                    "lowering_suffix_hex": primitive_bytes_to_hex(view.kind.lowering_suffix_bytes()),
                    "lowering_semantics": view.kind.lowering_semantics(),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            PrimitiveMaterialOp::MaterialEntityContractHook {
                kind,
                semantic,
                symbol,
            } => {
                let profile = primitive_material_probe_profile_from_op(op).unwrap_or_else(|| {
                    primitive_material_probe_profile_for_semantic(semantic.as_str())
                });
                let semantic_phase = primitive_material_semantic_phase(semantic.as_str());
                *semantic_probe_phases
                    .entry(semantic_phase.to_string())
                    .or_insert(0) += 1;
                semantic_probe_labels.insert(semantic.clone());
                *probe_profile_counts
                    .entry(profile.profile_key().to_string())
                    .or_insert(0) += 1;
                material_entity_contract_hooks.push(serde_json::json!({
                    "kind_family": kind.as_str(),
                    "probe_profile": profile.profile_key(),
                    "semantic": semantic,
                    "semantic_phase": semantic_phase,
                    "symbol": symbol,
                    "marker": format!("0x{:02X}", kind.marker_byte()),
                    "semantic_marker": format!("0x{:02X}", primitive_material_semantic_marker_byte(semantic.as_str())),
                    "lowering_mode": primitive_material_probe_semantic_lowering_mode(semantic.as_str()),
                    "required_capability": primitive_material_probe_semantic_required_capability(semantic.as_str()),
                    "unsupported_diagnostic_code": primitive_material_probe_semantic_unsupported_diag(semantic.as_str()),
                    "lowering_prefix_hex": primitive_bytes_to_hex(primitive_material_probe_semantic_prefix_bytes(semantic.as_str()).unwrap_or_else(|| kind.lowering_prefix_bytes())),
                    "lowering_suffix_hex": primitive_bytes_to_hex(primitive_material_probe_semantic_suffix_bytes(semantic.as_str()).unwrap_or_else(|| kind.lowering_suffix_bytes())),
                    "lowering_semantics": kind.lowering_semantics(),
                    "semantic_lowering_semantics": primitive_material_semantic_lowering_semantics(semantic.as_str()),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            _ => {}
        }
    }
    let event_hook_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_event_op_view(op).is_some())
        .count();
    let protocol_contract_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_protocol_op_view(op).is_some())
        .count();
    let material_entity_contract_hook_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_material_probe_profile_from_op(op).is_some())
        .count();
    let event_contracts_json = primitive_event_contracts_to_json(
        &contract.event_contracts,
        &contract.side_effect_contracts,
    );
    let event_contract_total = event_contracts_json.len();
    let event_contract_async = event_contracts_json
        .iter()
        .filter(|row| {
            row.get("async_hardware_event")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .count();
    let event_contract_verified = event_contracts_json.iter().all(|row| {
        let side_effects_ok = row
            .get("side_effects")
            .and_then(|v| v.as_array())
            .map(|cols| !cols.is_empty())
            .unwrap_or(false);
        let mmio_shape_ok =
            if row.get("kind").and_then(|v| v.as_str()) == Some("mmioReadClearsFlag") {
                row.get("subject")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains('.'))
                    .unwrap_or(false)
            } else {
                true
            };
        side_effects_ok && mmio_shape_ok
    });
    let mut event_contract_kind_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for row in &event_contracts_json {
        if let Some(kind) = row.get("kind").and_then(|v| v.as_str()) {
            *event_contract_kind_counts
                .entry(kind.to_string())
                .or_insert(0) += 1;
        }
    }
    let payload = serde_json::json!({
        "schema": "nc.primitive.lower.v1",
        "source": path,
        "target": target.id,
        "environment": "freestanding",
        "entry": entry_name,
        "directive_kinds": primitive_directive_kind_labels(&program),
        "material_semantics": primitive_material_semantic_labels(&program),
        "firmware_boot_model": primitive_firmware_boot_model_json(&primitive),
        "event_contracts": event_contracts_json,
        "lowered_ops": lowered_ops,
        "lowered_op_sequences": lowered_op_sequences,
        "advanced_material_lowering": {
            "protocol_contract_ops": protocol_contract_ops,
            "event_hook_ops": event_hook_ops,
            "material_entity_contract_hook_ops": material_entity_contract_hook_ops,
            "event_contracts_total": event_contract_total,
            "event_contracts_async": event_contract_async,
            "event_contracts_verified": event_contract_verified,
            "event_contract_kind_counts": event_contract_kind_counts,
            "semantic_probe_phases": semantic_probe_phases,
            "probe_profile_counts": probe_profile_counts,
            "semantic_probe_labels": semantic_probe_labels.into_iter().collect::<Vec<_>>(),
            "protocol_contracts": protocol_contracts,
            "event_hooks": event_hooks,
            "material_entity_contract_hooks": material_entity_contract_hooks,
        },
        "non_linear_event_contracts_audit": {
            "total": event_contract_total,
            "async_count": event_contract_async,
            "verified": event_contract_verified
        },
        "path": ["primitive[@]", "primitive IR", "machine IR", "backend", "artifact"],
        "artifact_kind": "raw_boot_image",
        "artifact_size": raw.len(),
    });
    let json = serde_json::to_string_pretty(&payload).expect("lower json serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("lower json parse");
    assert_eq!(
        parsed.get("schema").and_then(|v| v.as_str()),
        Some("nc.primitive.lower.v1")
    );
    assert!(
        parsed
            .get("lowered_ops")
            .and_then(|v| v.as_array())
            .map(|ops| ops.iter().any(|op| op
                .as_str()
                .map(|kind| kind.starts_with("protocol_") && kind.ends_with("_contract"))
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json must include protocol_*_contract ops"
    );
    assert!(
        parsed
            .get("lowered_ops")
            .and_then(|v| v.as_array())
            .map(|ops| ops.iter().any(|op| op.as_str() == Some("event_hook")))
            .unwrap_or(false),
        "advanced lower json must include event_hook ops"
    );
    assert!(
        parsed
            .get("lowered_ops")
            .and_then(|v| v.as_array())
            .map(|ops| ops
                .iter()
                .any(|op| op.as_str() == Some("material_entity_contract_hook")))
            .unwrap_or(false),
        "advanced lower json must include material_entity_contract_hook ops"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("protocol_contract_ops"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0,
        "advanced lower json must report protocol contract op count"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hook_ops"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0,
        "advanced lower json must report event hook op count"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("material_entity_contract_hook_ops"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0,
        "advanced lower json must report material contract probe op count"
    );
    assert!(
        parsed
            .get("lowered_op_sequences")
            .and_then(|v| v.as_array())
            .map(|rows| rows
                .iter()
                .any(|row| row.get("kind").and_then(|v| v.as_str())
                    == Some("material_entity_contract_hook")))
            .unwrap_or(false),
        "advanced lower json must include machine sequence rows for material probes"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("protocol_contracts"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("lowering_semantics")
                .and_then(|v| v.as_array())
                .map(|cols| !cols.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json protocol hooks must include lowering semantics tags"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("protocol_contracts"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| {
                row.get("lowering_mode")
                    .and_then(|v| v.as_str())
                    .map(|mode| mode.starts_with("protocol_"))
                    .unwrap_or(false)
                    && row
                        .get("transport")
                        .and_then(|v| v.as_str())
                        .map(|transport| matches!(transport, "port_io" | "mmio"))
                        .unwrap_or(false)
            }))
            .unwrap_or(false),
        "advanced lower json protocol hooks must include lowering mode and transport"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("machine_bytes_hex")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json event hooks must include machine byte sequences"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("material_entity_contract_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("lowering_prefix_hex")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json probes must include lowering prefix sequences"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("lowering_suffix_hex")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json event hooks must include lowering suffix sequences"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("lowering_semantics")
                .and_then(|v| v.as_array())
                .map(|cols| !cols.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json event hooks must include lowering semantics tags"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| {
                let kinds = rows
                    .iter()
                    .filter_map(|row| row.get("event_kind").and_then(|v| v.as_str()))
                    .collect::<std::collections::BTreeSet<_>>();
                kinds.contains("physicalFault")
            })
            .unwrap_or(false),
        "advanced lower json must include event hook kind labels from non-linear contracts"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("async_hardware_event")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json event hooks must explicitly mark async_hardware_event=true"
    );
    assert_eq!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_contracts_verified"))
            .and_then(|v| v.as_bool()),
        Some(true),
        "advanced lower json must mark non-linear event contracts as verified"
    );
    assert_eq!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("event_contracts_total"))
            .and_then(|v| v.as_u64()),
        parsed
            .get("non_linear_event_contracts_audit")
            .and_then(|v| v.get("total"))
            .and_then(|v| v.as_u64()),
        "advanced lower json must keep event contract totals consistent with audit totals"
    );
    assert_eq!(
        parsed
            .get("non_linear_event_contracts_audit")
            .and_then(|v| v.get("verified"))
            .and_then(|v| v.as_bool()),
        Some(true),
        "lower json must expose a verified non-linear event contract audit"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("material_entity_contract_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| rows.iter().all(|row| row
                .get("lowering_semantics")
                .and_then(|v| v.as_array())
                .map(|cols| !cols.is_empty())
                .unwrap_or(false)))
            .unwrap_or(false),
        "advanced lower json probes must include lowering semantics tags"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("semantic_probe_labels"))
            .and_then(|v| v.as_array())
            .map(|labels| {
                labels
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .map(|labels| {
                labels.contains("thermalState")
                    && labels.contains("throttleState")
                    && labels.contains("cacheState")
                    && labels.contains("tlbState")
                    && labels.contains("pipelineState")
                    && labels.contains("branchPredictorState")
                    && labels.contains("reorderState")
                    && labels.contains("outOfOrder")
                    && labels.contains("powerGate")
                    && labels.contains("firmwareRegion")
                    && labels.contains("bootRomRegion")
                    && labels.contains("biosRegion")
                    && labels.contains("uefiHandoff")
                    && labels.contains("resetVector")
                    && labels.contains("bootWindow")
                    && labels.contains("bootAnchor")
                    && labels.contains("firmwareTable")
                    && labels.contains("deviceTree")
                    && labels.contains("acpiTable")
                    && labels.contains("microcodeState")
            })
            .unwrap_or(false),
        "advanced lower json must expose high-detail semantic probe labels"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("semantic_probe_phases"))
            .and_then(|v| v.as_object())
            .map(|phases| {
                phases.get("microarchitecture").and_then(|v| v.as_u64()).unwrap_or(0) > 0
                    && phases.get("firmware_boot").and_then(|v| v.as_u64()).unwrap_or(0) > 0
                    && phases.get("thermal").and_then(|v| v.as_u64()).unwrap_or(0) > 0
                    && phases.get("power").and_then(|v| v.as_u64()).unwrap_or(0) > 0
            })
            .unwrap_or(false),
        "advanced lower json must include semantic probe phase coverage for microarchitecture/firmware/thermal/power"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("probe_profile_counts"))
            .and_then(|v| v.as_object())
            .map(|profiles| {
                profiles
                    .get("power_thermal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    > 0
                    && profiles
                        .get("microarchitecture")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        > 0
                    && profiles
                        .get("firmware_boot")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        > 0
                    && profiles
                        .get("family_fallback")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        > 0
            })
            .unwrap_or(false),
        "advanced lower json must include probe profile coverage for fallback/power_thermal/microarchitecture/firmware_boot"
    );
    assert!(
        parsed
            .get("advanced_material_lowering")
            .and_then(|v| v.get("material_entity_contract_hooks"))
            .and_then(|v| v.as_array())
            .map(|rows| {
                let mut by_semantic = std::collections::BTreeMap::new();
                for row in rows {
                    if let Some(semantic) = row.get("semantic").and_then(|v| v.as_str()) {
                        by_semantic.insert(semantic.to_string(), row);
                    }
                }
                let required = [
                    (
                        "powerGate",
                        "power_thermal",
                        "entity_opcode_plan_power_gate",
                        "supports_power_gate_model",
                        "E_PRIMITIVE_POWER_001",
                    ),
                    (
                        "thermalState",
                        "power_thermal",
                        "entity_opcode_plan_thermal_state",
                        "supports_thermal_state_model",
                        "E_PRIMITIVE_THERMAL_001",
                    ),
                    (
                        "throttleState",
                        "power_thermal",
                        "entity_opcode_plan_throttle_state",
                        "supports_throttle_state_model",
                        "E_PRIMITIVE_THERMAL_001",
                    ),
                    (
                        "cacheState",
                        "microarchitecture",
                        "entity_opcode_plan_cache_state",
                        "supports_cache_state_model",
                        "E_PRIMITIVE_CACHE_001",
                    ),
                    (
                        "tlbState",
                        "microarchitecture",
                        "entity_opcode_plan_tlb_state",
                        "supports_tlb_state_model",
                        "E_PRIMITIVE_TLB_001",
                    ),
                    (
                        "pipelineState",
                        "microarchitecture",
                        "entity_opcode_plan_pipeline_state",
                        "supports_pipeline_state_model",
                        "E_PRIMITIVE_TARGET_CAPABILITY_001",
                    ),
                    (
                        "branchPredictorState",
                        "microarchitecture",
                        "entity_opcode_plan_branch_predictor_state",
                        "supports_branch_predictor_model",
                        "E_PRIMITIVE_TARGET_CAPABILITY_001",
                    ),
                    (
                        "reorderState",
                        "microarchitecture",
                        "entity_opcode_plan_reorder_state",
                        "supports_reorder_model",
                        "E_PRIMITIVE_TARGET_CAPABILITY_001",
                    ),
                    (
                        "outOfOrder",
                        "microarchitecture",
                        "entity_opcode_plan_out_of_order",
                        "supports_out_of_order_model",
                        "E_PRIMITIVE_TARGET_CAPABILITY_001",
                    ),
                    (
                        "firmwareRegion",
                        "firmware_boot",
                        "entity_opcode_plan_firmware_region",
                        "supports_firmware_region_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "bootRomRegion",
                        "firmware_boot",
                        "entity_opcode_plan_bootrom_region",
                        "supports_boot_rom_region_model",
                        "E_PRIMITIVE_BOOTROM_001",
                    ),
                    (
                        "biosRegion",
                        "firmware_boot",
                        "entity_opcode_plan_bios_region",
                        "supports_bios_region_model",
                        "E_PRIMITIVE_FIRMWARE_ENV_001",
                    ),
                    (
                        "uefiHandoff",
                        "firmware_boot",
                        "entity_opcode_plan_uefi_handoff",
                        "supports_uefi_handoff_model",
                        "E_PRIMITIVE_FIRMWARE_ENV_001",
                    ),
                    (
                        "resetVector",
                        "firmware_boot",
                        "entity_opcode_plan_reset_vector",
                        "supports_reset_vector_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "bootWindow",
                        "firmware_boot",
                        "entity_opcode_plan_boot_window",
                        "supports_boot_window_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "bootAnchor",
                        "firmware_boot",
                        "entity_opcode_plan_boot_anchor",
                        "supports_boot_anchor_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "firmwareTable",
                        "firmware_boot",
                        "entity_opcode_plan_firmware_table",
                        "supports_firmware_table_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "deviceTree",
                        "firmware_boot",
                        "entity_opcode_plan_device_tree",
                        "supports_device_tree_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "acpiTable",
                        "firmware_boot",
                        "entity_opcode_plan_acpi_table",
                        "supports_acpi_table_model",
                        "E_PRIMITIVE_FIRMWARE_001",
                    ),
                    (
                        "microcodeState",
                        "firmware_boot",
                        "entity_opcode_plan_microcode_state",
                        "supports_microcode_state_model",
                        "E_PRIMITIVE_MICROCODE_001",
                    ),
                ];
                required.iter().all(
                    |(semantic, expected_profile, expected_mode, expected_capability, expected_diag)| {
                        let Some(row) = by_semantic.get(*semantic) else {
                            return false;
                        };
                        let prefix_ok = row
                            .get("lowering_prefix_hex")
                            .and_then(|v| v.as_str())
                            .map(|hex| !hex.is_empty() && hex != "90")
                            .unwrap_or(false);
                        let profile_ok = row
                            .get("probe_profile")
                            .and_then(|v| v.as_str())
                            .map(|profile| profile == *expected_profile)
                            .unwrap_or(false);
                        let lowering_mode_ok = row
                            .get("lowering_mode")
                            .and_then(|v| v.as_str())
                            .map(|mode| mode == *expected_mode)
                            .unwrap_or(false);
                        let capability_ok = row
                            .get("required_capability")
                            .and_then(|v| v.as_str())
                            .map(|capability| capability == *expected_capability)
                            .unwrap_or(false);
                        let diag_ok = row
                            .get("unsupported_diagnostic_code")
                            .and_then(|v| v.as_str())
                            .map(|diag| diag == *expected_diag)
                            .unwrap_or(false);
                        prefix_ok && profile_ok && lowering_mode_ok && capability_ok && diag_ok
                    },
                )
            })
            .unwrap_or(false),
        "advanced lower json must expose entity-specific lowering profile/capability/diag for high-detail semantics"
    );
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_cli_lower_missing_target_fails() {
    let args = vec![
        "lower".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "missing-target-does-not-exist".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("lower with missing target must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_CLI_TARGET_001"));
}

#[test]
fn primitive_cli_build_wraps_validation_errors_with_cli_code() {
    let target_id = format!(
        "tmp-runtime-managed-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"managed\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nprotocols = [\"UART\"]\n"
        )
        .as_str(),
    );
    let args = vec![
        "build".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        target_name.clone(),
    ];
    let err = run_primitive_command(&args).expect_err("runtime-managed target must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_CLI_BUILD_001"));
    assert!(rendered.contains("E_PRIMITIVE_TARGET_UNSUPPORTED_001"));
    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_cli_build_requires_target() {
    let args = vec![
        "build".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "missing-target-does-not-exist".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("build should require valid target");
    assert!(err.to_string().contains("E_PRIMITIVE_CLI_TARGET_001"));
}

#[test]
fn primitive_cli_build_requires_entry() {
    let temp = write_temp_primitive(
        "cli_build_requires_entry",
        "primitive[@] target x86_64:\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port p0:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation bootSomethingElse():\n        capability hw;\n        write p0 <- 0x41;\n        barrier io;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "build".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--entry".to_string(),
        "missingEntry".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing entry symbol should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("E_PRIMITIVE_ENTRY_MISSING_001")
            || rendered.contains("E_PRIMITIVE_CLI_BUILD_001"),
        "unexpected diagnostic: {}",
        rendered
    );
}

#[test]
fn primitive_cli_build_rejects_runtime() {
    let temp = write_temp_primitive(
        "cli_build_rejects_runtime",
        "primitive[@] target x86_64:\n    operation kernelEntry():\n        runtime.output(\"x\");\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let args = vec![
        "build".to_string(),
        temp,
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("build must reject runtime fallback");
    assert!(err.to_string().contains("E_PRIMITIVE_RUNTIME_001"));
}

#[test]
fn primitive_cli_build_wraps_linker_failures_with_cli_linker_code() {
    let args = vec![
        "build".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--linker".to_string(),
        "/tmp/does_not_exist.ld".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("missing linker path must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_CLI_BUILD_001"));
    assert!(rendered.contains("E_PRIMITIVE_CLI_LINKER_001"));
}

#[test]
fn primitive_cli_report_wraps_write_failures_with_cli_code() {
    let json_out = std::env::temp_dir().join(format!(
        "ncc_primitive_report_json_{}_{}.json",
        std::process::id(),
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let args = vec![
        "report".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--md-out".to_string(),
        std::env::temp_dir().to_string_lossy().to_string(),
        "--json-out".to_string(),
        json_out.to_string_lossy().to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("report write failure must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_CLI_REPORT_001"));
    let _ = std::fs::remove_file(json_out);
}

#[test]
fn primitive_cli_qemu_reports_missing_qemu_cleanly() {
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_ok()
    {
        return;
    }
    let args = vec![
        "qemu".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--expect".to_string(),
        "nC primitive alive".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("qemu command must report missing qemu");
    assert!(err.to_string().contains("E_PRIMITIVE_CLI_QEMU_MISSING_001"));
}

#[test]
fn primitive_cli_qemu_smoke_if_qemu_available() {
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let _guard = primitive_qemu_test_guard();
    let args = vec![
        "qemu".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--expect".to_string(),
        "nC primitive alive".to_string(),
    ];
    run_primitive_command(&args).expect("qemu smoke should pass when qemu is available");
}

#[test]
fn primitive_cli_target_lists_x86_64_unknown_none() {
    let path = primitive_target_path("x86_64-unknown-none");
    assert!(
        path.exists(),
        "target registry must include x86_64-unknown-none.nctarget"
    );
}

#[test]
fn primitive_cli_target_lists_x86_64_protocol_lab() {
    let path = primitive_target_path("x86_64-protocol-lab");
    assert!(
        path.exists(),
        "target registry must include x86_64-protocol-lab.nctarget"
    );
}

#[test]
fn primitive_doctor_real_reports_environment() {
    let args = vec![
        "doctor".to_string(),
        "--real".to_string(),
        "--json".to_string(),
    ];
    run_primitive_command(&args).expect("primitive doctor --real --json must report environment");
}

#[test]
fn primitive_doctor_real_does_not_claim_hardware_without_verified_device() {
    let args = vec!["doctor".to_string(), "--hardware".to_string()];
    run_primitive_command(&args).expect("primitive doctor --hardware must return blocked report");
}

#[test]
fn primitive_target_matrix_has_real_and_emulated_levels() {
    for target in [
        "x86_64-unknown-none",
        "x86_64-protocol-lab",
        "x86_64-linux-host-serial",
        "x86_64-linux-portio",
        "generic-mmio-lab",
    ] {
        let path = primitive_target_path(target);
        assert!(path.exists(), "missing primitive target {}", target);
    }
}

#[test]
fn primitive_cli_target_json_is_valid() {
    let spec = load_primitive_target_spec("x86_64-unknown-none").expect("target spec");
    let payload = serde_json::json!({
        "name": spec.id,
        "path": spec.path.to_string_lossy(),
        "arch": spec.field("arch"),
        "isa": spec.field("isa"),
        "environment": spec.field("environment"),
        "runtime": spec.field("runtime"),
        "protocols": spec.protocols,
        "capabilities": spec.bool_fields,
    });
    let raw = serde_json::to_string_pretty(&payload).expect("target json serialize");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("target json parse");
    assert_eq!(
        parsed.get("name").and_then(|v| v.as_str()),
        Some("x86_64-unknown-none")
    );
}

#[test]
fn primitive_cli_target_protocols_are_reported() {
    let spec = load_primitive_target_spec("x86_64-unknown-none").expect("target spec");
    assert!(
        spec.protocols.iter().any(|p| p == "UART"),
        "x86_64-unknown-none target must report UART protocol"
    );
}

#[test]
fn primitive_cli_target_protocol_lab_reports_advanced_protocols() {
    let spec = load_primitive_target_spec("x86_64-protocol-lab").expect("target spec");
    for protocol in [
        "UART",
        "PCIE",
        "USB",
        "SATA",
        "NVME",
        "I2C",
        "SPI",
        "ETHERNET",
        "HDMI",
        "DISPLAYPORT",
    ] {
        assert!(
            spec.supports_protocol(protocol),
            "x86_64-protocol-lab target must report '{}' protocol",
            protocol
        );
    }
}

#[test]
fn primitive_cli_target_environment_reported() {
    let spec = load_primitive_target_spec("x86_64-unknown-none").expect("target spec");
    assert_eq!(spec.field("environment"), Some("freestanding"));
}

#[test]
fn primitive_lower_includes_backend_bytes() {
    let args = vec![
        "lower".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--emit-bytes".to_string(),
        "--json".to_string(),
    ];
    run_primitive_command(&args).expect("primitive lower --emit-bytes must pass");
}

#[test]
fn primitive_real_hardware_missing_is_blocked_not_success() {
    let args = vec![
        "run".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-linux-host-serial".to_string(),
        "--device".to_string(),
        "/dev/nc-missing-primitive-test-device".to_string(),
        "--json".to_string(),
    ];
    run_primitive_command(&args)
        .expect("missing real hardware must be environmental blocked output");
}

#[test]
fn primitive_portio_permission_fail() {
    let args = vec![
        "check".to_string(),
        primitive_fixture_path("hardware_identity_uart.ncp"),
        "--target".to_string(),
        "x86_64-linux-portio".to_string(),
        "--environment".to_string(),
        "privileged_host".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("privileged port I/O must be blocked");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_PORT_IO_PERMISSION_001"));
}

#[test]
fn primitive_mmio_unsafe_address_fail() {
    let args = vec![
        "lower".to_string(),
        primitive_fixture_path("industrial/unsafe_mmio_write_fail.ncp"),
        "--json".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("unsafe MMIO address must be rejected");
    assert!(err
        .to_string()
        .contains("E_PRIMITIVE_MMIO_UNSAFE_ADDRESS_001"));
}

#[test]
fn primitive_directives_docs_exist() {
    let root = workspace_repo_root().join("docs/spec/primitive/directives");
    for directive in [
        "addr", "barrier", "read", "volatile", "width", "write", "anchor", "slot", "pattern",
        "mask", "window", "signal", "bus", "state", "timing", "emit", "sample", "latch", "assert",
        "rupture", "bridge", "seal",
    ] {
        assert!(
            root.join(format!("{directive}.md")).exists(),
            "missing directive doc {}",
            directive
        );
    }
}

#[test]
fn primitive_cli_target_strict_qemu_certification_fail() {
    let args = vec![
        "target".to_string(),
        "x86_64-qemu".to_string(),
        "--strict".to_string(),
    ];
    let err = run_primitive_command(&args).expect_err("strict target qemu must fail");
    assert!(err.to_string().contains("E_PRIMITIVE_QEMU_NOT_REAL_HW_001"));
}

#[test]
fn primitive_cli_target_strict_non_qemu_ok() {
    let args = vec![
        "target".to_string(),
        "x86_64-unknown-none".to_string(),
        "--strict".to_string(),
        "--json".to_string(),
    ];
    run_primitive_command(&args).expect("strict target physical declaration must pass");
}

#[test]
fn existing_ncc_commands_still_work() {
    run_nfmt_command(&["--help".to_string()]).expect("nfmt --help must pass");
    run_nlint_command(&["--help".to_string()]).expect("nlint --help must pass");
    run_nbuild_command(&["--help".to_string()]).expect("nbuild --help must pass");
    run_npkg_command(&["--help".to_string()]).expect("npkg --help must pass");
    run_nide_command(&["--help".to_string()]).expect("nide --help must pass");
}

#[test]
fn primitive_advanced_physical_directives_check_and_lower_ok_with_supported_target() {
    let target_name = format!(
        "tmp-advanced-physical-supported-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_name,
        format!(
            "name = \"{target_name}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_serial_com1 = true\nsupports_dma = true\nsupports_cache_model = true\nsupports_tlb_model = true\nsupports_pipeline_model = true\nsupports_firmware_model = true\nsupports_electrical_model = true\nsupports_clock_model = true\nsupports_edge_model = true\nsupports_metastability_model = true\nsupports_thermal_model = true\nsupports_power_model = true\nprotocols = [\"UART\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"supported\"\nfirmware_microcode = \"target_declared\"\n"
        )
        .as_str(),
    );
    let path = advanced_physical_semantics_fixture();

    let check_args = vec![
        "check".to_string(),
        path.clone(),
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
    ];
    run_primitive_command(&check_args)
        .expect("advanced physical fixture should pass check on fully-capable target");

    let lower_args = vec![
        "lower".to_string(),
        path,
        "--target".to_string(),
        target_name.clone(),
        "--environment".to_string(),
        "freestanding".to_string(),
        "--json".to_string(),
    ];
    run_primitive_command(&lower_args)
        .expect("advanced physical fixture should lower on fully-capable target");

    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_protocol_deep_lowering_uses_declared_mmio_window_addresses() {
    let target_id = format!(
        "tmp-protocol-deep-lowering-{}",
        TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let target_name = write_temp_target(
        &target_id,
        format!(
            "name = \"{target_id}\"\narch = \"x86_64\"\nisa = \"x86_64\"\nbits = 64\nendian = \"little\"\nos = \"none\"\nenvironment = \"freestanding\"\nbinary_format = \"elf\"\nruntime = \"none\"\nstdlib = \"core-only\"\nsupports_port_io = true\nsupports_mmio = true\nsupports_interrupts = true\nsupports_volatile = true\nsupports_fence = true\nsupports_atomic = true\nsupports_halt = true\nsupports_serial_com1 = true\nsupports_dma = true\nsupports_cache_model = true\nsupports_tlb_model = true\nsupports_pipeline_model = true\nsupports_firmware_model = true\nsupports_electrical_model = true\nsupports_clock_model = true\nsupports_edge_model = true\nsupports_metastability_model = true\nsupports_thermal_model = true\nsupports_power_model = true\nprotocols = [\"UART\", \"PCIE\", \"USB\", \"SATA\", \"NVME\", \"I2C\", \"SPI\", \"ETHERNET\", \"HDMI\", \"DISPLAYPORT\"]\nfirmware_bios = \"supported\"\nfirmware_uefi = \"supported\"\nfirmware_bootrom = \"supported\"\nfirmware_microcode = \"target_declared\"\n"
        )
        .as_str(),
    );
    let path = protocol_deep_lowering_fixture("protocol_deep_lowering");
    run_primitive_check(path.clone(), &target_name, "freestanding")
        .expect("deep protocol fixture must pass check");

    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("boot contract extraction must pass for deep protocol fixture");

    let mut seen_protocols = std::collections::BTreeSet::new();
    for op in &contract.ops {
        let Some(view) = primitive_protocol_op_view(op) else {
            continue;
        };
        seen_protocols.insert(view.protocol.to_string());
        assert_ne!(
            primitive_protocol_lowering_mode(view.protocol),
            "protocol_contract_probe",
            "protocol '{}' must use dedicated lowering mode",
            view.protocol
        );
        let lowered = primitive_material_op_lowering_bytes(op, "kernelEntry")
            .expect("protocol lowering bytes must be available");
        let lowered_hex = primitive_bytes_to_hex(&lowered);
        if let Some(window_addr) = view.window_addr {
            let base = primitive_addr_u16(window_addr, "kernelEntry", "protocol mmio window")
                .expect("protocol window address must lower to real16 range");
            let status_offset = match view.protocol {
                "USB" => Some(0x04u16),
                "SATA" => Some(0x10u16),
                "NVME" => Some(0x1Cu16),
                "ETHERNET" => Some(0x08u16),
                "HDMI" => Some(0x0Cu16),
                "DISPLAYPORT" => Some(0x14u16),
                _ => None,
            };
            if let Some(offset) = status_offset {
                let status_addr = base.saturating_add(offset);
                let status_mov_hex = format!(
                    "bb{:02x}{:02x}",
                    status_addr & 0x00FF,
                    (status_addr >> 8) & 0x00FF
                );
                assert!(
                    lowered_hex.contains(&status_mov_hex),
                    "protocol '{}' lowering must read status at window-derived address {:#06X}",
                    view.protocol,
                    status_addr
                );
            }
        }
    }

    for required in [
        "UART",
        "PCIE",
        "USB",
        "SATA",
        "NVME",
        "I2C",
        "SPI",
        "ETHERNET",
        "HDMI",
        "DISPLAYPORT",
    ] {
        assert!(
            seen_protocols.contains(required),
            "deep protocol fixture must include protocol '{}'",
            required
        );
    }

    let _ = std::fs::remove_file(primitive_target_path(&target_name));
}

#[test]
fn primitive_protocol_matrix_cli_e2e_check_map_witness_lower_build_ok() {
    let target_name = "x86_64-protocol-lab";
    let environment = "emulated-qemu";
    for protocol in [
        "UART",
        "PCIe",
        "USB",
        "SATA",
        "NVMe",
        "I2C",
        "SPI",
        "Ethernet",
        "HDMI",
        "DisplayPort",
    ] {
        let source = protocol_matrix_fixture(
            &format!("protocol_matrix_e2e_{}", protocol.to_ascii_lowercase()),
            protocol,
        );

        let check_args = vec![
            "check".to_string(),
            source.clone(),
            "--target".to_string(),
            target_name.to_string(),
            "--environment".to_string(),
            environment.to_string(),
        ];
        run_primitive_command(&check_args)
            .expect("protocol matrix fixture should pass primitive check");

        let map_args = vec![
            "map".to_string(),
            source.clone(),
            "--target".to_string(),
            target_name.to_string(),
            "--environment".to_string(),
            environment.to_string(),
            "--json".to_string(),
        ];
        run_primitive_command(&map_args)
            .expect("protocol matrix fixture should pass primitive map");

        let witness_args = vec![
            "witness".to_string(),
            source.clone(),
            "--target".to_string(),
            target_name.to_string(),
            "--environment".to_string(),
            environment.to_string(),
            "--json".to_string(),
        ];
        run_primitive_command(&witness_args)
            .expect("protocol matrix fixture should pass primitive witness");

        let lower_args = vec![
            "lower".to_string(),
            source.clone(),
            "--target".to_string(),
            target_name.to_string(),
            "--environment".to_string(),
            environment.to_string(),
            "--json".to_string(),
        ];
        run_primitive_command(&lower_args)
            .expect("protocol matrix fixture should pass primitive lower");

        let out_path = std::env::temp_dir().join(format!(
            "ncc_primitive_protocol_matrix_{}_{}.sov",
            protocol.to_ascii_lowercase(),
            TEMP_PATH_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let build_args = vec![
            "build".to_string(),
            source,
            "--target".to_string(),
            target_name.to_string(),
            "--environment".to_string(),
            environment.to_string(),
            "--entry".to_string(),
            "kernelEntry".to_string(),
            "--linker".to_string(),
            "link/x86_64_baremetal.ld".to_string(),
            "--out".to_string(),
            out_path.to_string_lossy().to_string(),
        ];
        run_primitive_command(&build_args)
            .expect("protocol matrix fixture should pass primitive build");
        let built = std::fs::metadata(&out_path).expect("protocol matrix build output metadata");
        assert!(
            built.len() > 0,
            "protocol matrix build output must be non-empty for protocol '{}'",
            protocol
        );
        let _ = std::fs::remove_file(out_path);
    }
}

#[test]
fn primitive_protocol_material_register_profiles_are_configurable() {
    let path = write_temp_primitive(
        "protocol_material_register_profiles",
        "primitive[@] target x86_64:\n    controller:\n        ctrl0;\n    peripheral:\n        per0;\n    lane:\n        lane0;\n    channel:\n        ch0;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    port pcie.cfg_addr:\n        bus io8;\n        addr ioaddr(0x0CF8);\n    port pcie.cfg_data:\n        bus io8;\n        addr ioaddr(0x0CFC);\n    window usbwin0:\n        addr mmioaddr(0x2100);\n    capability hw:\n        allow port.write;\n    operation kernelEntry():\n        capability hw;\n        protocol PCIe controller ctrl0 lane lane0 configAddrPort pcie.cfg_addr configDataPort pcie.cfg_data configCommand 0x91 compareValue 0x44;\n        protocol USB controller ctrl0 peripheral per0 window usbwin0 statusOffset 0x24 commandOffset 0x2A;\n        protocol I2C controller ctrl0 channel ch0 dataPort dev.data readyMask 0x04 writeMask 0x3F;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let program = read_program(&path);
    let primitive = primitive_find_container(&program).expect("primitive container");
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some("kernelEntry"),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )
    .expect("custom protocol register profiles must extract");

    let mut saw_pcie = false;
    let mut saw_usb = false;
    let mut saw_i2c = false;
    for op in &contract.ops {
        match op {
            PrimitiveMaterialOp::ProtocolPcie {
                config_command,
                compare_value,
                ..
            } => {
                saw_pcie = true;
                assert_eq!(*config_command, 0x91);
                assert_eq!(*compare_value, 0x44);
                let lowered = primitive_material_op_lowering_bytes(op, "kernelEntry")
                    .expect("pcie lowering bytes");
                let hex = primitive_bytes_to_hex(&lowered);
                assert!(
                    hex.contains("b091ee"),
                    "pcie config command must be lowered"
                );
                assert!(hex.contains("3c44"), "pcie compare value must be lowered");
            }
            PrimitiveMaterialOp::ProtocolUsb {
                status_offset,
                command_offset,
                ..
            } => {
                saw_usb = true;
                assert_eq!(*status_offset, 0x24);
                assert_eq!(*command_offset, 0x2A);
                let lowered = primitive_material_op_lowering_bytes(op, "kernelEntry")
                    .expect("usb lowering bytes");
                let hex = primitive_bytes_to_hex(&lowered);
                assert!(
                    hex.contains("bb2421"),
                    "usb status offset must lower to window+0x24"
                );
                assert!(
                    hex.contains("bb2a21"),
                    "usb command offset must lower to window+0x2A"
                );
            }
            PrimitiveMaterialOp::ProtocolI2c {
                ready_mask,
                write_mask,
                ..
            } => {
                saw_i2c = true;
                assert_eq!(*ready_mask, 0x04);
                assert_eq!(*write_mask, 0x3F);
                let lowered = primitive_material_op_lowering_bytes(op, "kernelEntry")
                    .expect("i2c lowering bytes");
                let hex = primitive_bytes_to_hex(&lowered);
                assert!(hex.contains("a804"), "i2c ready mask must be lowered");
                assert!(hex.contains("243f"), "i2c write mask must be lowered");
            }
            _ => {}
        }
    }
    assert!(saw_pcie, "custom fixture must include PCIe op");
    assert!(saw_usb, "custom fixture must include USB op");
    assert!(saw_i2c, "custom fixture must include I2C op");
}

#[test]
fn primitive_protocol_status_offset_for_uart_fail() {
    let source = write_temp_primitive(
        "protocol_status_offset_for_uart_fail",
        "primitive[@] target x86_64:\n    controller:\n        ctrl0;\n    peripheral:\n        per0;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    capability hw:\n        allow port.write;\n    operation kernelEntry():\n        capability hw;\n        protocol UART controller ctrl0 peripheral per0 dataPort dev.data statusOffset 0x04;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(source, "x86_64-protocol-lab", "emulated-qemu")
        .expect_err("uart statusOffset qualifier must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_001"));
    assert!(rendered.contains("statusOffset/commandOffset qualifiers for MMIO protocols"));
}

#[test]
fn primitive_protocol_write_mask_for_usb_fail() {
    let source = write_temp_primitive(
        "protocol_write_mask_for_usb_fail",
        "primitive[@] target x86_64:\n    controller:\n        ctrl0;\n    peripheral:\n        per0;\n    window usbwin0:\n        addr mmioaddr(0x2100);\n    capability hw:\n        allow port.write;\n    bus io8:\n        width 8;\n    port dev.data:\n        bus io8;\n        addr ioaddr(0x03F8);\n    operation kernelEntry():\n        capability hw;\n        protocol USB controller ctrl0 peripheral per0 window usbwin0 writeMask 0x1F;\n        writes dev.data;\n        write dev.data <- 0x41;\n        barrier io;\n        haltLoop;\n    anchor boot0:\n        addr physaddr(0x0000_7C00);\n    bridge:\n        root.policy.bind;\n        root.policy.guard;\n        root.memory.resolve;\n        machine.machineCore;\n    seal:\n        primitiveClosed;\n",
    );
    let err = run_primitive_check(source, "x86_64-protocol-lab", "emulated-qemu")
        .expect_err("usb writeMask qualifier must fail");
    let rendered = err.to_string();
    assert!(rendered.contains("E_PRIMITIVE_PROTOCOL_001"));
    assert!(rendered.contains("writeMask qualifier for UART/I2C/SPI protocols"));
}
