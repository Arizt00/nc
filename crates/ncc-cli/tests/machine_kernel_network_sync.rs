use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PHYSICAL_HMAC_ENV: &str = "NCC_TEST_NETWORK_PHYSICAL_HMAC";
const PHYSICAL_HMAC_KEY: &str = "0123456789abcdef0123456789abcdef";

fn ncc() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn temporary_bundle(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-machine-kernel-network-sync-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let mut result = Map::new();
            for key in keys {
                result.insert(key.clone(), canonical_json(&object[key]));
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn iommu_payload_sha256(payload: &Value) -> String {
    let bytes = serde_json::to_vec(&canonical_json(payload)).unwrap();
    let mut digest = Sha256::new();
    digest.update(b"ncc.machine.iommu_vfio_certificate.v1\0");
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn create_contract_board_bundle(root: &std::path::Path) -> PathBuf {
    let artifact = root.join("kernel.elf");
    fs::write(&artifact, b"NCC-CONTRACT-ONLY-NOT-PHYSICAL-EVIDENCE").unwrap();
    let manifest = root.join("board.json");
    let test_binary = std::env::current_exe().unwrap();
    let command = |_label: &str| {
        json!({
            "argv": [
                test_binary,
                "--exact",
                "board_command_marker_helper",
                "--nocapture"
            ],
            "env": {
                "NCC_BOARD_CONTRACT_MARKERS": "CONTRACT-PROBE\nnetwork\nstatus\nNCC_PHYSICAL_BOARD=1\nNCC_BOARD_ID={board_id}\nNCC_BOARD_SERIAL={board_serial}\nNCC_TARGET={target}\nNCC_FIRMWARE_SHA256={artifact_sha256}"
            },
            "timeout_ms": 5_000,
            "success_exit_codes": [0]
        })
    };
    fs::write(
        &manifest,
        serde_json::to_vec_pretty(&json!({
            "schema": "ncc.machine.physical_board_manifest.v1",
            "manifest_version": 1,
            "board": {
                "id": "contract-board",
                "vendor": "Contract",
                "model": "CI",
                "hardware_revision": "1",
                "serial": "CONTRACT-SERIAL",
                "architecture": "x86_64",
                "target": "x86_64-unknown-none"
            },
            "artifact": {"path": artifact, "sha256": "auto"},
            "transport": {
                "kind": "contract-test-probe",
                "physical": true,
                "port": "CONTRACT-PORT",
                "baud": 115200,
                "probe_serial": "CONTRACT-PROBE"
            },
            "limits": {
                "max_artifact_bytes": 1_048_576,
                "max_log_bytes": 65_536,
                "max_timeout_ms": 10_000
            },
            "commands": {
                "probe": command("probe"),
                "flash": command("flash"),
                "reset": command("reset"),
                "capture": command("capture")
            },
            "identity": {
                "probe_markers": ["network", "status"],
                "required_markers": ["network", "status"],
                "ordered_markers": ["network", "status"],
                "forbidden_markers": ["QEMU", "TCG", "EMULATOR", "SIMULATOR"]
            },
            "policy": {
                "require_clean_git": false,
                "reject_emulated_transport": true,
                "retain_full_command_logs_within_limits": true
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let bundle = root.join("board-bundle");
    let output = Command::new(ncc())
        .args(["machine", "certify-board", "--manifest"])
        .arg(&manifest)
        .arg("--bundle")
        .arg(&bundle)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    bundle
}

fn create_contract_vfio_certificate(root: &std::path::Path) -> PathBuf {
    let flags = json!({
        "software_viommu_executed": true,
        "physical_iommu_executed": true,
        "vfio_passthrough_executed": true,
        "iommu_dma_map_unmap_executed": true,
        "device_generated_dma_executed": true,
        "device_reset_performed": true,
        "msix_eventfd_delivered": true,
        "vfio_interrupt_remapping_path_executed": true,
        "iova_isolation_executed": true,
        "iommu_group_isolation_verified": true,
        "hardware_iommu_architecture_identified": true,
        "driver_recovery_performed": true,
        "device_rebinding_performed": false
    });
    let payload = json!({
        "schema": "ncc.machine.iommu_vfio_certificate.v1",
        "certificate_version": 1,
        "kind": "vfio_physical_passthrough",
        "status": "vfio_physical_data_plane_passed",
        "source": {"contract_test_only": true},
        "host": {"contract_test_only": true},
        "model": {"material_execution": true},
        "vfio": {
            "provider_api_executed": true,
            "dma_map_unmap_executed": true,
            "device_generated_dma_executed": true,
            "device_reset_performed": true,
            "msix_eventfd_delivered": true,
            "vfio_interrupt_remapping_path_executed": true,
            "iova_isolation_executed": true,
            "iommu_group_isolation_verified": true,
            "hardware_iommu_architecture_identified": true,
            "driver_recovery_performed": true,
            "teardown_complete": true,
            "device": {"bdf": "0000:03:00.0", "vendor": "0x10ec", "device": "0x8125"},
            "host_iommu": {"architecture": "intel_vtd"},
            "dma": {"bytes": 16384}
        },
        "exercise": {"schema": "ncc.machine.vfio_dma_exercise.v1"},
        "truth_boundary": flags
    });
    let certificate = json!({
        "payload_sha256": iommu_payload_sha256(&payload),
        "payload": payload
    });
    let path = root.join("vfio-physical.json");
    fs::write(&path, serde_json::to_vec_pretty(&certificate).unwrap()).unwrap();
    path
}

fn ethernet_frame(source: [u8; 6], destination: [u8; 6], payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend(destination);
    frame.extend(source);
    frame.extend(0x88b5u16.to_be_bytes());
    frame.extend(payload);
    frame
}

fn create_contract_capture(root: &std::path::Path) -> (PathBuf, PathBuf) {
    let tx = ethernet_frame(
        [0x02, 0, 0, 0, 0, 1],
        [0x02, 0, 0, 0, 0, 2],
        b"NCC-PHYSICAL-TX",
    );
    let rx = ethernet_frame(
        [0x02, 0, 0, 0, 0, 2],
        [0x02, 0, 0, 0, 0, 1],
        b"NCC-PHYSICAL-RX",
    );
    let mut pcap = vec![
        0xd4, 0xc3, 0xb2, 0xa1, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0, 0, 1, 0, 0, 0,
    ];
    for frame in [tx, rx] {
        pcap.extend(1u32.to_le_bytes());
        pcap.extend(0u32.to_le_bytes());
        pcap.extend((frame.len() as u32).to_le_bytes());
        pcap.extend((frame.len() as u32).to_le_bytes());
        pcap.extend(frame);
    }
    let capture = root.join("wire.pcap");
    fs::write(&capture, &pcap).unwrap();
    let contract = root.join("capture-contract.json");
    fs::write(
        &contract,
        serde_json::to_vec_pretty(&json!({
            "schema": "ncc.machine.kernel_network_sync_physical_capture.v1",
            "board": {"id": "contract-board", "serial": "CONTRACT-SERIAL", "target": "x86_64-unknown-none"},
            "nic": {"bdf": "0000:03:00.0", "vendor": "10ec", "device": "8125", "mac": "02:00:00:00:00:01"},
            "peer": {"mac": "02:00:00:00:00:02"},
            "frames": {
                "ether_type": "0x88b5",
                "tx_payload_hex": "4e43432d504859534943414c2d5458",
                "rx_payload_hex": "4e43432d504859534943414c2d5258",
                "minimum_tx_frames": 1,
                "minimum_rx_frames": 1
            },
            "capture": {"sha256": sha256(&pcap)}
        }))
        .unwrap(),
    )
    .unwrap();
    (capture, contract)
}

#[test]
fn network_and_synchronization_certificate_is_material_and_independently_verified() {
    let bundle = temporary_bundle("valid");
    let output = Command::new(ncc())
        .args(["machine", "kernel-system", "network-sync", "--bundle"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(bundle.join("kernel-network-sync-report.json")).unwrap())
            .unwrap();
    assert!(report["network"]["nic_families"].as_u64().unwrap() >= 14);
    assert_eq!(report["network"]["recovered_nic_faults"], 1);
    assert_eq!(report["network"]["nic_rx_descriptors"], 8);
    assert_eq!(report["network"]["nic_tx_bytes"], 96);
    assert_eq!(report["network"]["ethernet_vlan_tags"], 2);
    assert_eq!(report["network"]["ipv4_fragments"], 3);
    assert_eq!(report["network"]["ipv4_reassembled_bytes"], 240);
    assert_eq!(report["network"]["ipv6_extension_bytes"], 8);
    assert_eq!(report["network"]["ipv6_reassembled_bytes"], 24);
    assert_eq!(report["network"]["tcp_retransmits"], 1);
    assert_eq!(report["synchronization"]["targets"], 6);
    assert_eq!(report["synchronization"]["barrier_encodings"], 36);
    assert_eq!(report["synchronization"]["contended_increments"], 8_000);
    assert_eq!(report["synchronization"]["queued_mutex_handoff"], 9);
    assert_eq!(report["synchronization"]["hidden_scheduler"], false);
    assert_eq!(report["synchronization"]["hidden_allocation"], false);
    assert_eq!(report["physical_execution"]["performed"], false);

    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-network-sync"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let _ = fs::remove_dir_all(bundle);
}

#[test]
fn network_and_synchronization_certificate_rejects_tampering() {
    let bundle = temporary_bundle("tamper");
    assert!(Command::new(ncc())
        .args(["machine", "kernel-system", "network-sync", "--bundle"])
        .arg(&bundle)
        .status()
        .unwrap()
        .success());
    let report = bundle.join("kernel-network-sync-report.json");
    let mut bytes = fs::read(&report).unwrap();
    let index = bytes.iter().position(|byte| *byte == b'8').unwrap();
    bytes[index] = b'9';
    fs::write(&report, bytes).unwrap();
    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-network-sync"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(!verified.status.success());
    assert!(String::from_utf8_lossy(&verified.stderr).contains("report hash mismatch"));
    let _ = fs::remove_dir_all(bundle);
}

#[test]
fn physical_network_certificate_never_degrades_to_host_only_evidence() {
    let output = Command::new(ncc())
        .args(["machine", "kernel-system", "network-sync-physical"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing --host-bundle"), "{stderr}");
}

#[test]
fn failed_physical_inputs_leave_no_partial_certificate_bundle() {
    let root = temporary_bundle("physical-missing");
    let host = root.join("host");
    let output_bundle = root.join("physical");
    assert!(Command::new(ncc())
        .args(["machine", "kernel-system", "network-sync", "--bundle"])
        .arg(&host)
        .status()
        .unwrap()
        .success());
    let output = Command::new(ncc())
        .args([
            "machine",
            "kernel-system",
            "network-sync-physical",
            "--host-bundle",
        ])
        .arg(&host)
        .arg("--board-bundle")
        .arg(root.join("missing-board"))
        .arg("--vfio-certificate")
        .arg(root.join("missing-vfio.json"))
        .arg("--capture")
        .arg(root.join("missing.pcapng"))
        .arg("--capture-contract")
        .arg(root.join("missing-contract.json"))
        .arg("--bundle")
        .arg(&output_bundle)
        .arg("--signing-key-env")
        .arg(PHYSICAL_HMAC_ENV)
        .env(PHYSICAL_HMAC_ENV, PHYSICAL_HMAC_KEY)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("board bundle"), "{stderr}");
    assert!(!output_bundle.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn physical_composition_pipeline_reverifies_every_retained_contract() {
    let root = temporary_bundle("physical-contract-pipeline");
    fs::create_dir_all(&root).unwrap();
    let host = root.join("host");
    assert!(Command::new(ncc())
        .args(["machine", "kernel-system", "network-sync", "--bundle"])
        .arg(&host)
        .status()
        .unwrap()
        .success());
    let board = create_contract_board_bundle(&root);
    let vfio = create_contract_vfio_certificate(&root);
    let (capture, contract) = create_contract_capture(&root);
    let bundle = root.join("physical-bundle");
    let certified = Command::new(ncc())
        .args([
            "machine",
            "kernel-system",
            "network-sync-physical",
            "--host-bundle",
        ])
        .arg(&host)
        .arg("--board-bundle")
        .arg(&board)
        .arg("--vfio-certificate")
        .arg(&vfio)
        .arg("--capture")
        .arg(&capture)
        .arg("--capture-contract")
        .arg(&contract)
        .arg("--bundle")
        .arg(&bundle)
        .arg("--signing-key-env")
        .arg(PHYSICAL_HMAC_ENV)
        .env(PHYSICAL_HMAC_ENV, PHYSICAL_HMAC_KEY)
        .output()
        .unwrap();
    assert!(
        certified.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&certified.stdout),
        String::from_utf8_lossy(&certified.stderr)
    );
    let report: Value = serde_json::from_slice(
        &fs::read(bundle.join("kernel-network-sync-physical-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["physical_execution"]["performed"], true);
    assert_eq!(
        report["physical_execution"]["capture"]["tx_marker_frames"],
        1
    );
    assert_eq!(
        report["physical_execution"]["capture"]["rx_marker_frames"],
        1
    );
    assert_eq!(report["physical_execution"]["nic"]["iommu"], "intel_vtd");

    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-network-sync-physical"])
        .arg(&bundle)
        .arg("--signing-key-env")
        .arg(PHYSICAL_HMAC_ENV)
        .env(PHYSICAL_HMAC_ENV, PHYSICAL_HMAC_KEY)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );

    let wrong_key = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-network-sync-physical"])
        .arg(&bundle)
        .arg("--signing-key-env")
        .arg(PHYSICAL_HMAC_ENV)
        .env(PHYSICAL_HMAC_ENV, "fedcba9876543210fedcba9876543210")
        .output()
        .unwrap();
    assert!(!wrong_key.status.success());
    assert!(String::from_utf8_lossy(&wrong_key.stderr).contains("HMAC mismatch"));

    let retained_capture = bundle.join("inputs/wire.pcap");
    let mut bytes = fs::read(&retained_capture).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&retained_capture, bytes).unwrap();
    let rejected = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-network-sync-physical"])
        .arg(&bundle)
        .arg("--signing-key-env")
        .arg(PHYSICAL_HMAC_ENV)
        .env(PHYSICAL_HMAC_ENV, PHYSICAL_HMAC_KEY)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("capture SHA-256 differs"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn board_command_marker_helper() {
    if let Ok(markers) = std::env::var("NCC_BOARD_CONTRACT_MARKERS") {
        println!("{markers}");
    }
}
