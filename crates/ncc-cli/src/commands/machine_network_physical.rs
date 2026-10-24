use hmac::{Hmac, Mac};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const REPORT_SCHEMA: &str = "ncc.machine.kernel_network_sync_physical_report.v1";
const CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_network_sync_physical_certificate.v1";
const CAPTURE_SCHEMA: &str = "ncc.machine.kernel_network_sync_physical_capture.v1";
const HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_network_sync_physical.v1\0";
const HMAC_DOMAIN: &[u8] = b"ncc.machine.kernel_network_sync_physical.attestation.v1\0";
const MAX_CAPTURE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BUNDLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureContract {
    board_id: String,
    board_serial: String,
    target: String,
    bdf: String,
    vendor: String,
    device: String,
    nic_mac: [u8; 6],
    peer_mac: [u8; 6],
    ether_type: u16,
    tx_payload: Vec<u8>,
    rx_payload: Vec<u8>,
    minimum_tx_frames: u64,
    minimum_rx_frames: u64,
    capture_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureResult {
    format: &'static str,
    link_type: u32,
    total_frames: u64,
    tx_frames: u64,
    rx_frames: u64,
    captured_bytes: u64,
}

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn required_option<'a>(args: &'a [String], flag: &str) -> Result<&'a str, Box<dyn Error>> {
    option_value(args, flag)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!("[E_MACHINE_NETWORK_PHYSICAL_ARGS_001] missing {flag} <path>").into()
        })
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

fn canonical_bytes(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&canonical_json(value))?)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn payload_sha256(value: &Value) -> Result<String, Box<dyn Error>> {
    let mut bytes = HASH_DOMAIN.to_vec();
    bytes.extend(canonical_bytes(value)?);
    Ok(sha256(&bytes))
}

fn signing_key(args: &[String]) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let environment = required_option(args, "--signing-key-env")?;
    if !environment
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_ATTESTATION_001] signing environment name is invalid"
                .into(),
        );
    }
    let key = std::env::var(environment).map_err(|_| {
        format!(
            "[E_MACHINE_NETWORK_PHYSICAL_ATTESTATION_001] signing environment '{environment}' is unavailable"
        )
    })?;
    if key.as_bytes().len() < 32 {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_ATTESTATION_001] signing key must contain at least 32 bytes".into());
    }
    Ok((environment.to_string(), key.into_bytes()))
}

fn hmac_sha256(key: &[u8], payload_hash: &str) -> Result<String, Box<dyn Error>> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(HMAC_DOMAIN);
    mac.update(payload_hash.as_bytes());
    Ok(mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&canonical_json(value))?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<u64, Box<dyn Error>> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_FILE_001] '{}' is not a regular file",
            source.display()
        )
        .into());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(metadata.len())
}

fn copy_tree(source: &Path, destination: &Path, total: &mut u64) -> Result<(), Box<dyn Error>> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_FILE_001] symlink '{}' is forbidden in retained evidence",
            source.display()
        )
        .into());
    }
    if metadata.is_file() {
        *total = total
            .checked_add(copy_regular_file(source, destination)?)
            .ok_or("physical evidence byte count overflow")?;
        if *total > MAX_BUNDLE_BYTES {
            return Err(
                "[E_MACHINE_NETWORK_PHYSICAL_LIMIT_001] retained evidence exceeds 2 GiB".into(),
            );
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_FILE_001] unsupported evidence node '{}'",
            source.display()
        )
        .into());
    }
    fs::create_dir_all(destination)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        copy_tree(&entry.path(), &destination.join(entry.file_name()), total)?;
    }
    Ok(())
}

fn parse_hex_bytes(value: &str, label: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let clean = value
        .chars()
        .filter(|character| !matches!(character, ':' | '-' | '_' | ' '))
        .collect::<String>();
    if clean.is_empty() || clean.len() % 2 != 0 {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] {label} is not even non-empty hex"
        )
        .into());
    }
    (0..clean.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&clean[index..index + 2], 16).map_err(Into::into))
        .collect()
}

fn parse_mac(value: &str, label: &str) -> Result<[u8; 6], Box<dyn Error>> {
    let bytes = parse_hex_bytes(value, label)?;
    bytes.try_into().map_err(|_| {
        format!("[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] {label} must contain 6 bytes").into()
    })
}

fn required_string(value: &Value, pointer: &str) -> Result<String, Box<dyn Error>> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            format!("[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] missing string '{pointer}'").into()
        })
}

fn parse_u16(value: &Value, pointer: &str) -> Result<u16, Box<dyn Error>> {
    if let Some(number) = value.pointer(pointer).and_then(Value::as_u64) {
        return u16::try_from(number).map_err(Into::into);
    }
    let raw = required_string(value, pointer)?;
    let raw = raw.replace('_', "");
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        Ok(u16::from_str_radix(hex, 16)?)
    } else {
        Ok(raw.parse()?)
    }
}

fn load_capture_contract(path: &Path) -> Result<CaptureContract, Box<dyn Error>> {
    let value = read_json(path)?;
    if value.get("schema").and_then(Value::as_str) != Some(CAPTURE_SCHEMA) {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] invalid capture contract schema".into(),
        );
    }
    let tx_payload = parse_hex_bytes(
        &required_string(&value, "/frames/tx_payload_hex")?,
        "frames.tx_payload_hex",
    )?;
    let rx_payload = parse_hex_bytes(
        &required_string(&value, "/frames/rx_payload_hex")?,
        "frames.rx_payload_hex",
    )?;
    if tx_payload.len() > 65_535 || rx_payload.len() > 65_535 {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] marker payload exceeds Ethernet jumbo limit"
                .into(),
        );
    }
    let minimum_tx_frames = value
        .pointer("/frames/minimum_tx_frames")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let minimum_rx_frames = value
        .pointer("/frames/minimum_rx_frames")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if minimum_tx_frames == 0 || minimum_rx_frames == 0 {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] both frame minima must be non-zero".into(),
        );
    }
    Ok(CaptureContract {
        board_id: required_string(&value, "/board/id")?,
        board_serial: required_string(&value, "/board/serial")?,
        target: required_string(&value, "/board/target")?,
        bdf: required_string(&value, "/nic/bdf")?.to_ascii_lowercase(),
        vendor: required_string(&value, "/nic/vendor")?.to_ascii_lowercase(),
        device: required_string(&value, "/nic/device")?.to_ascii_lowercase(),
        nic_mac: parse_mac(&required_string(&value, "/nic/mac")?, "nic.mac")?,
        peer_mac: parse_mac(&required_string(&value, "/peer/mac")?, "peer.mac")?,
        ether_type: parse_u16(&value, "/frames/ether_type")?,
        tx_payload,
        rx_payload,
        minimum_tx_frames,
        minimum_rx_frames,
        capture_sha256: required_string(&value, "/capture/sha256")?.to_ascii_lowercase(),
    })
}

fn read_u16(bytes: &[u8], offset: usize, endian: Endian) -> Result<u16, Box<dyn Error>> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or("truncated capture u16")?
        .try_into()?;
    Ok(match endian {
        Endian::Little => u16::from_le_bytes(raw),
        Endian::Big => u16::from_be_bytes(raw),
    })
}

fn read_u32(bytes: &[u8], offset: usize, endian: Endian) -> Result<u32, Box<dyn Error>> {
    let raw: [u8; 4] = bytes
        .get(offset..offset + 4)
        .ok_or("truncated capture u32")?
        .try_into()?;
    Ok(match endian {
        Endian::Little => u32::from_le_bytes(raw),
        Endian::Big => u32::from_be_bytes(raw),
    })
}

fn inspect_ethernet(frame: &[u8], contract: &CaptureContract, result: &mut CaptureResult) {
    if frame.len() < 14 {
        return;
    }
    let destination = &frame[..6];
    let source = &frame[6..12];
    let mut ether_type = u16::from_be_bytes([frame[12], frame[13]]);
    let mut payload_offset = 14;
    while matches!(ether_type, 0x8100 | 0x88a8 | 0x9100) {
        if frame.len() < payload_offset + 4 {
            return;
        }
        ether_type = u16::from_be_bytes([frame[payload_offset + 2], frame[payload_offset + 3]]);
        payload_offset += 4;
    }
    if ether_type != contract.ether_type {
        return;
    }
    let payload = &frame[payload_offset..];
    if source == contract.nic_mac
        && destination == contract.peer_mac
        && payload.starts_with(&contract.tx_payload)
    {
        result.tx_frames += 1;
    }
    if source == contract.peer_mac
        && destination == contract.nic_mac
        && payload.starts_with(&contract.rx_payload)
    {
        result.rx_frames += 1;
    }
}

fn inspect_pcap(bytes: &[u8], contract: &CaptureContract) -> Result<CaptureResult, Box<dyn Error>> {
    if bytes.len() < 24 {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] truncated PCAP header".into());
    }
    let endian = match &bytes[..4] {
        [0xd4, 0xc3, 0xb2, 0xa1] | [0x4d, 0x3c, 0xb2, 0xa1] => Endian::Little,
        [0xa1, 0xb2, 0xc3, 0xd4] | [0xa1, 0xb2, 0x3c, 0x4d] => Endian::Big,
        _ => return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] unsupported PCAP magic".into()),
    };
    let link_type = read_u32(bytes, 20, endian)?;
    if link_type != 1 {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] PCAP link type {link_type} is not Ethernet"
        )
        .into());
    }
    let mut result = CaptureResult {
        format: "pcap",
        link_type,
        total_frames: 0,
        tx_frames: 0,
        rx_frames: 0,
        captured_bytes: 0,
    };
    let mut offset = 24usize;
    while offset < bytes.len() {
        if bytes.len() - offset < 16 {
            return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] truncated PCAP record".into());
        }
        let captured = usize::try_from(read_u32(bytes, offset + 8, endian)?)?;
        let original = usize::try_from(read_u32(bytes, offset + 12, endian)?)?;
        if captured > original || bytes.len() - offset - 16 < captured {
            return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] invalid PCAP record length".into());
        }
        let frame = &bytes[offset + 16..offset + 16 + captured];
        result.total_frames += 1;
        result.captured_bytes += captured as u64;
        inspect_ethernet(frame, contract, &mut result);
        offset += 16 + captured;
    }
    Ok(result)
}

fn inspect_pcapng(
    bytes: &[u8],
    contract: &CaptureContract,
) -> Result<CaptureResult, Box<dyn Error>> {
    if bytes.len() < 28 || bytes[..4] != [0x0a, 0x0d, 0x0d, 0x0a] {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] invalid PCAPNG section".into());
    }
    let endian = match &bytes[8..12] {
        [0x4d, 0x3c, 0x2b, 0x1a] => Endian::Little,
        [0x1a, 0x2b, 0x3c, 0x4d] => Endian::Big,
        _ => {
            return Err(
                "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] invalid PCAPNG byte-order magic".into(),
            )
        }
    };
    let mut result = CaptureResult {
        format: "pcapng",
        link_type: 0,
        total_frames: 0,
        tx_frames: 0,
        rx_frames: 0,
        captured_bytes: 0,
    };
    let mut interfaces = Vec::<u16>::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] truncated PCAPNG block".into());
        }
        let block_type = read_u32(bytes, offset, endian)?;
        let length = usize::try_from(read_u32(bytes, offset + 4, endian)?)?;
        if length < 12 || length % 4 != 0 || bytes.len() - offset < length {
            return Err("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] invalid PCAPNG block length".into());
        }
        if read_u32(bytes, offset + length - 4, endian)? as usize != length {
            return Err(
                "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] PCAPNG trailing length mismatch".into(),
            );
        }
        match block_type {
            1 if length >= 20 => interfaces.push(read_u16(bytes, offset + 8, endian)?),
            6 if length >= 32 => {
                let interface = usize::try_from(read_u32(bytes, offset + 8, endian)?)?;
                let captured = usize::try_from(read_u32(bytes, offset + 20, endian)?)?;
                let original = usize::try_from(read_u32(bytes, offset + 24, endian)?)?;
                let link_type = *interfaces.get(interface).ok_or(
                    "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] PCAPNG packet references an unknown interface",
                )?;
                if link_type != 1 {
                    return Err(format!("[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] PCAPNG link type {link_type} is not Ethernet").into());
                }
                if captured > original || offset + 28 + captured > offset + length - 4 {
                    return Err(
                        "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] invalid PCAPNG packet length".into(),
                    );
                }
                let frame = &bytes[offset + 28..offset + 28 + captured];
                result.link_type = 1;
                result.total_frames += 1;
                result.captured_bytes += captured as u64;
                inspect_ethernet(frame, contract, &mut result);
            }
            _ => {}
        }
        offset += length;
    }
    if result.link_type != 1 {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_PCAP_001] PCAPNG contains no Ethernet packet".into(),
        );
    }
    Ok(result)
}

fn inspect_capture(
    path: &Path,
    contract: &CaptureContract,
) -> Result<CaptureResult, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_CAPTURE_BYTES {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_LIMIT_001] capture size is outside 1..512 MiB".into(),
        );
    }
    let bytes = fs::read(path)?;
    if sha256(&bytes) != contract.capture_sha256 {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] capture SHA-256 differs from contract".into(),
        );
    }
    let result = if bytes.starts_with(&[0x0a, 0x0d, 0x0d, 0x0a]) {
        inspect_pcapng(&bytes, contract)?
    } else {
        inspect_pcap(&bytes, contract)?
    };
    require_bidirectional_capture(&result, contract)?;
    Ok(result)
}

fn require_bidirectional_capture(
    result: &CaptureResult,
    contract: &CaptureContract,
) -> Result<(), Box<dyn Error>> {
    if result.tx_frames < contract.minimum_tx_frames
        || result.rx_frames < contract.minimum_rx_frames
    {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_CAPTURE_001] capture markers incomplete: tx={}/{} rx={}/{}",
            result.tx_frames,
            contract.minimum_tx_frames,
            result.rx_frames,
            contract.minimum_rx_frames
        )
        .into());
    }
    Ok(())
}

fn normalize_pci_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .strip_prefix("0x")
        .unwrap_or(value.trim())
        .trim_start_matches('0')
        .to_string()
}

fn run_self_json(args: &[String]) -> Result<Value, Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    let output = Command::new(&executable)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_VERIFY_001] '{}' rejected retained evidence: {}",
            executable.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

fn verify_component_certificates(
    host_bundle: &Path,
    board_bundle: &Path,
    vfio_certificate: &Path,
    board_signing_key_env: Option<&str>,
) -> Result<(Value, Value, Value, Value, Value), Box<dyn Error>> {
    let host_verification = run_self_json(&[
        "machine".into(),
        "kernel-system".into(),
        "verify-network-sync".into(),
        host_bundle.to_string_lossy().into_owned(),
    ])?;
    if host_verification.get("status").and_then(Value::as_str) != Some("verified") {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_VERIFY_001] host network certificate is not verified"
                .into(),
        );
    }
    let mut board_args = vec![
        "machine".into(),
        "verify-board-certificate".into(),
        board_bundle.to_string_lossy().into_owned(),
        "--json".into(),
    ];
    if let Some(environment) = board_signing_key_env {
        board_args.extend([
            "--signing-key-env".into(),
            environment.into(),
            "--require-attestation".into(),
        ]);
    }
    let board_verification = run_self_json(&board_args)?;
    if board_verification.get("status").and_then(Value::as_str) != Some("verified")
        || board_verification
            .get("physical_board_claim")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_VERIFY_001] board bundle lacks a verified physical claim"
                .into(),
        );
    }
    let iommu_verification = run_self_json(&[
        "machine".into(),
        "virtualization".into(),
        "verify-iommu-certificate".into(),
        vfio_certificate.to_string_lossy().into_owned(),
        "--json".into(),
    ])?;
    if iommu_verification.get("status").and_then(Value::as_str) != Some("verified")
        || iommu_verification.get("kind").and_then(Value::as_str)
            != Some("vfio_physical_passthrough")
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_VERIFY_001] IOMMU certificate is not a physical VFIO dataplane certificate".into());
    }
    let board_certificate = read_json(&board_bundle.join("certificate.json"))?;
    let vfio = read_json(vfio_certificate)?;
    Ok((
        host_verification,
        board_verification,
        iommu_verification,
        board_certificate,
        vfio,
    ))
}

fn validate_identity(
    contract: &CaptureContract,
    board: &Value,
    vfio: &Value,
) -> Result<Value, Box<dyn Error>> {
    for (pointer, expected) in [
        ("/board/id", contract.board_id.as_str()),
        ("/board/serial", contract.board_serial.as_str()),
        ("/board/target", contract.target.as_str()),
    ] {
        if board.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "[E_MACHINE_NETWORK_PHYSICAL_IDENTITY_001] board identity mismatch at {pointer}"
            )
            .into());
        }
    }
    let payload = vfio
        .get("payload")
        .ok_or("[E_MACHINE_NETWORK_PHYSICAL_IDENTITY_001] VFIO certificate has no payload")?;
    for pointer in [
        "/truth_boundary/device_generated_dma_executed",
        "/truth_boundary/device_reset_performed",
        "/truth_boundary/msix_eventfd_delivered",
        "/truth_boundary/vfio_interrupt_remapping_path_executed",
        "/truth_boundary/iova_isolation_executed",
        "/truth_boundary/iommu_group_isolation_verified",
        "/truth_boundary/hardware_iommu_architecture_identified",
        "/truth_boundary/driver_recovery_performed",
        "/vfio/teardown_complete",
    ] {
        if payload.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!("[E_MACHINE_NETWORK_PHYSICAL_VFIO_001] required VFIO evidence '{pointer}' is not true").into());
        }
    }
    let device = payload
        .pointer("/vfio/device")
        .ok_or("[E_MACHINE_NETWORK_PHYSICAL_IDENTITY_001] VFIO device identity is missing")?;
    let actual_bdf = required_string(device, "/bdf")?.to_ascii_lowercase();
    let actual_vendor = normalize_pci_id(&required_string(device, "/vendor")?);
    let actual_device = normalize_pci_id(&required_string(device, "/device")?);
    if actual_bdf != contract.bdf
        || actual_vendor != normalize_pci_id(&contract.vendor)
        || actual_device != normalize_pci_id(&contract.device)
    {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_IDENTITY_001] capture NIC {} {}/{} differs from VFIO NIC {} {}/{}",
            contract.bdf,
            contract.vendor,
            contract.device,
            actual_bdf,
            actual_vendor,
            actual_device
        )
        .into());
    }
    let dma_bytes = payload
        .pointer("/vfio/dma/bytes")
        .and_then(Value::as_u64)
        .filter(|bytes| *bytes != 0)
        .ok_or("[E_MACHINE_NETWORK_PHYSICAL_VFIO_001] VFIO DMA byte count is zero or missing")?;
    let iommu = required_string(payload, "/vfio/host_iommu/architecture")?;
    Ok(json!({
        "bdf": actual_bdf,
        "vendor": actual_vendor,
        "device": actual_device,
        "mac": contract.nic_mac.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(":"),
        "iommu": iommu,
        "dma_bytes": dma_bytes,
        "msix_delivered": true,
        "interrupt_remapping_executed": true,
        "iova_isolation_executed": true,
        "driver_recovery_performed": true,
    }))
}

fn ensure_output_available(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() && fs::read_dir(path)?.next().is_some() {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_BUNDLE_001] output bundle '{}' is not empty",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn require_input(path: &Path, label: &str, directory: bool) -> Result<(), Box<dyn Error>> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "[E_MACHINE_NETWORK_PHYSICAL_INPUT_001] {label} '{}' is unavailable: {error}",
            path.display()
        )
    })?;
    let valid = if directory {
        metadata.is_dir()
    } else {
        metadata.is_file()
    };
    if !valid {
        return Err(format!(
            "[E_MACHINE_NETWORK_PHYSICAL_INPUT_001] {label} '{}' has the wrong file type",
            path.display()
        )
        .into());
    }
    Ok(())
}

struct StagingBundle {
    path: PathBuf,
    committed: bool,
}

impl StagingBundle {
    fn create(destination: &Path) -> Result<Self, Box<dyn Error>> {
        let name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("network-physical");
        let staging = destination.with_file_name(format!(
            ".{name}.partial-{}-{}",
            std::process::id(),
            epoch_seconds()
        ));
        if staging.exists() {
            return Err(format!(
                "[E_MACHINE_NETWORK_PHYSICAL_BUNDLE_001] staging path '{}' already exists",
                staging.display()
            )
            .into());
        }
        fs::create_dir_all(&staging)?;
        Ok(Self {
            path: staging,
            committed: false,
        })
    }

    fn commit(mut self, destination: &Path) -> Result<(), Box<dyn Error>> {
        if destination.exists() {
            fs::remove_dir(destination)?;
        }
        fs::rename(&self.path, destination)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagingBundle {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub(crate) fn run_certify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source_host = PathBuf::from(required_option(args, "--host-bundle")?);
    let source_board = PathBuf::from(required_option(args, "--board-bundle")?);
    let source_vfio = PathBuf::from(required_option(args, "--vfio-certificate")?);
    let source_capture = PathBuf::from(required_option(args, "--capture")?);
    let source_contract = PathBuf::from(required_option(args, "--capture-contract")?);
    let bundle = PathBuf::from(required_option(args, "--bundle")?);
    let (signing_environment, signing_key) = signing_key(args)?;
    let board_signing_key_env = option_value(args, "--board-signing-key-env");
    require_input(&source_host, "host bundle", true)?;
    require_input(&source_board, "board bundle", true)?;
    require_input(&source_vfio, "VFIO certificate", false)?;
    require_input(&source_capture, "packet capture", false)?;
    require_input(&source_contract, "capture contract", false)?;
    ensure_output_available(&bundle)?;
    let staging = StagingBundle::create(&bundle)?;

    let retained = staging.path.join("inputs");
    let retained_host = retained.join("host");
    let retained_board = retained.join("board");
    let retained_vfio = retained.join("vfio-certificate.json");
    let retained_capture = retained.join(
        source_capture
            .file_name()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("capture.pcap")),
    );
    let retained_contract = retained.join("capture-contract.json");
    let mut retained_bytes = 0;
    copy_tree(&source_host, &retained_host, &mut retained_bytes)?;
    copy_tree(&source_board, &retained_board, &mut retained_bytes)?;
    retained_bytes += copy_regular_file(&source_vfio, &retained_vfio)?;
    retained_bytes += copy_regular_file(&source_capture, &retained_capture)?;
    retained_bytes += copy_regular_file(&source_contract, &retained_contract)?;
    if retained_bytes > MAX_BUNDLE_BYTES {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_LIMIT_001] retained evidence exceeds 2 GiB".into(),
        );
    }

    let contract = load_capture_contract(&retained_contract)?;
    let capture = inspect_capture(&retained_capture, &contract)?;
    let (host_verify, board_verify, iommu_verify, board, vfio) = verify_component_certificates(
        &retained_host,
        &retained_board,
        &retained_vfio,
        board_signing_key_env,
    )?;
    let nic = validate_identity(&contract, &board, &vfio)?;
    let report = json!({
        "schema": REPORT_SCHEMA,
        "status": "physical_network_certified",
        "issued_unix": epoch_seconds(),
        "physical_execution": {
            "performed": true,
            "board": {
                "id": contract.board_id,
                "serial": contract.board_serial,
                "target": contract.target,
            },
            "nic": nic,
            "capture": {
                "path": format!("inputs/{}", retained_capture.file_name().unwrap_or_default().to_string_lossy()),
                "format": capture.format,
                "sha256": contract.capture_sha256,
                "link_type": capture.link_type,
                "total_frames": capture.total_frames,
                "tx_marker_frames": capture.tx_frames,
                "rx_marker_frames": capture.rx_frames,
                "captured_bytes": capture.captured_bytes,
                "ether_type": format!("0x{:04x}", contract.ether_type),
            },
            "retained_bytes": retained_bytes,
            "component_verification": {
                "host": host_verify,
                "board": board_verify,
                "iommu": iommu_verify,
            },
        },
        "truth_boundary": {
            "windows_host_driver_counter_traffic_is_not_accepted": true,
            "emulator_or_traced_mmio_is_not_accepted": true,
            "vfio_device_generated_dma_required": true,
            "physical_msix_and_interrupt_remapping_required": true,
            "bidirectional_ethernet_capture_required": true,
            "all_evidence_copied_into_bundle": true,
        },
    });
    let report_path = staging
        .path
        .join("kernel-network-sync-physical-report.json");
    write_json(&report_path, &report)?;
    let report_bytes = fs::read(&report_path)?;
    let payload_hash = payload_sha256(&report)?;
    let certificate = json!({
        "schema": CERTIFICATE_SCHEMA,
        "report": "kernel-network-sync-physical-report.json",
        "report_sha256": sha256(&report_bytes),
        "payload_sha256": payload_hash,
        "attestation": {
            "algorithm": "HMAC-SHA256",
            "domain": "ncc.machine.kernel_network_sync_physical.attestation.v1",
            "key_environment": signing_environment,
            "hmac_sha256": hmac_sha256(&signing_key, &payload_hash)?,
            "secret_persisted": false,
        },
    });
    write_json(
        &staging
            .path
            .join("kernel-network-sync-physical-certificate.json"),
        &certificate,
    )?;
    staging.commit(&bundle)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "certified",
            "physical_execution": {"performed": true},
            "bundle": bundle,
            "payload_sha256": certificate["payload_sha256"],
            "capture": report["physical_execution"]["capture"],
            "nic": report["physical_execution"]["nic"],
        }))?
    );
    Ok(())
}

pub(crate) fn run_verify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (_, signing_key) = signing_key(args)?;
    let bundle = PathBuf::from(
        args.iter()
            .find(|arg| !arg.starts_with('-'))
            .ok_or("verify-network-sync-physical expects a bundle path")?,
    );
    let certificate_path = bundle.join("kernel-network-sync-physical-certificate.json");
    let certificate = read_json(&certificate_path)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(CERTIFICATE_SCHEMA) {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] invalid certificate schema".into(),
        );
    }
    let report_path = bundle.join(
        certificate
            .get("report")
            .and_then(Value::as_str)
            .ok_or("physical certificate has no report path")?,
    );
    let report_bytes = fs::read(&report_path)?;
    if certificate.get("report_sha256").and_then(Value::as_str)
        != Some(sha256(&report_bytes).as_str())
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] report SHA-256 mismatch".into());
    }
    let report: Value = serde_json::from_slice(&report_bytes)?;
    if report.get("schema").and_then(Value::as_str) != Some(REPORT_SCHEMA)
        || report
            .pointer("/physical_execution/performed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] report does not claim a physical execution".into());
    }
    if certificate.get("payload_sha256").and_then(Value::as_str)
        != Some(payload_sha256(&report)?.as_str())
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] payload SHA-256 mismatch".into());
    }
    let payload_hash = certificate
        .get("payload_sha256")
        .and_then(Value::as_str)
        .ok_or("physical certificate has no payload SHA-256")?;
    if certificate
        .pointer("/attestation/algorithm")
        .and_then(Value::as_str)
        != Some("HMAC-SHA256")
        || certificate
            .pointer("/attestation/domain")
            .and_then(Value::as_str)
            != Some("ncc.machine.kernel_network_sync_physical.attestation.v1")
        || certificate
            .pointer("/attestation/secret_persisted")
            .and_then(Value::as_bool)
            != Some(false)
        || certificate
            .pointer("/attestation/hmac_sha256")
            .and_then(Value::as_str)
            != Some(hmac_sha256(&signing_key, payload_hash)?.as_str())
    {
        return Err(
            "[E_MACHINE_NETWORK_PHYSICAL_ATTESTATION_001] physical certificate HMAC mismatch"
                .into(),
        );
    }
    let retained_host = bundle.join("inputs/host");
    let retained_board = bundle.join("inputs/board");
    let retained_vfio = bundle.join("inputs/vfio-certificate.json");
    let retained_contract = bundle.join("inputs/capture-contract.json");
    let contract = load_capture_contract(&retained_contract)?;
    let capture_relative = report
        .pointer("/physical_execution/capture/path")
        .and_then(Value::as_str)
        .ok_or("physical report has no capture path")?;
    if Path::new(capture_relative).is_absolute()
        || Path::new(capture_relative)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] unsafe capture path".into());
    }
    let capture = inspect_capture(&bundle.join(capture_relative), &contract)?;
    let board_signing_key_env = option_value(args, "--board-signing-key-env");
    let (_, _, _, board, vfio) = verify_component_certificates(
        &retained_host,
        &retained_board,
        &retained_vfio,
        board_signing_key_env,
    )?;
    let nic = validate_identity(&contract, &board, &vfio)?;
    if report.pointer("/physical_execution/nic") != Some(&nic)
        || report
            .pointer("/physical_execution/capture/tx_marker_frames")
            .and_then(Value::as_u64)
            != Some(capture.tx_frames)
        || report
            .pointer("/physical_execution/capture/rx_marker_frames")
            .and_then(Value::as_u64)
            != Some(capture.rx_frames)
    {
        return Err("[E_MACHINE_NETWORK_PHYSICAL_CERTIFICATE_001] recomputed physical evidence differs from report".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_network_sync_physical_verification.v1",
            "status": "verified",
            "physical_execution": {"performed": true},
            "certificate": certificate_path,
            "payload_sha256": certificate["payload_sha256"],
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(capture_sha256: String) -> CaptureContract {
        CaptureContract {
            board_id: "board".into(),
            board_serial: "serial".into(),
            target: "x86_64-unknown-none".into(),
            bdf: "0000:03:00.0".into(),
            vendor: "10ec".into(),
            device: "8125".into(),
            nic_mac: [0x02, 0, 0, 0, 0, 1],
            peer_mac: [0x02, 0, 0, 0, 0, 2],
            ether_type: 0x88b5,
            tx_payload: b"NCC-PHYSICAL-TX".to_vec(),
            rx_payload: b"NCC-PHYSICAL-RX".to_vec(),
            minimum_tx_frames: 1,
            minimum_rx_frames: 1,
            capture_sha256,
        }
    }

    fn ethernet(source: [u8; 6], destination: [u8; 6], payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend(destination);
        frame.extend(source);
        frame.extend(0x88b5u16.to_be_bytes());
        frame.extend(payload);
        frame
    }

    fn pcap(frames: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![
            0xd4, 0xc3, 0xb2, 0xa1, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0, 0, 1, 0, 0,
            0,
        ];
        for frame in frames {
            bytes.extend(1u32.to_le_bytes());
            bytes.extend(0u32.to_le_bytes());
            bytes.extend((frame.len() as u32).to_le_bytes());
            bytes.extend((frame.len() as u32).to_le_bytes());
            bytes.extend(frame);
        }
        bytes
    }

    fn pcapng(frames: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(0x0a0d0d0au32.to_le_bytes());
        bytes.extend(28u32.to_le_bytes());
        bytes.extend(0x1a2b3c4du32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(u64::MAX.to_le_bytes());
        bytes.extend(28u32.to_le_bytes());

        bytes.extend(1u32.to_le_bytes());
        bytes.extend(20u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(65_535u32.to_le_bytes());
        bytes.extend(20u32.to_le_bytes());

        for frame in frames {
            let padded = (frame.len() + 3) & !3;
            let block_length = 32 + padded;
            bytes.extend(6u32.to_le_bytes());
            bytes.extend((block_length as u32).to_le_bytes());
            bytes.extend(0u32.to_le_bytes());
            bytes.extend(0u32.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            bytes.extend((frame.len() as u32).to_le_bytes());
            bytes.extend((frame.len() as u32).to_le_bytes());
            bytes.extend(frame);
            bytes.resize(bytes.len() + padded - frame.len(), 0);
            bytes.extend((block_length as u32).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn classic_pcap_requires_bidirectional_marker_frames() {
        let tx = ethernet(
            [0x02, 0, 0, 0, 0, 1],
            [0x02, 0, 0, 0, 0, 2],
            b"NCC-PHYSICAL-TX",
        );
        let rx = ethernet(
            [0x02, 0, 0, 0, 0, 2],
            [0x02, 0, 0, 0, 0, 1],
            b"NCC-PHYSICAL-RX",
        );
        let bytes = pcap(&[tx, rx]);
        let result = inspect_pcap(&bytes, &contract(sha256(&bytes))).unwrap();
        assert_eq!(result.tx_frames, 1);
        assert_eq!(result.rx_frames, 1);
        assert_eq!(result.link_type, 1);
    }

    #[test]
    fn capture_contract_rejects_host_only_or_one_way_evidence() {
        let tx = ethernet(
            [0x02, 0, 0, 0, 0, 1],
            [0x02, 0, 0, 0, 0, 2],
            b"NCC-PHYSICAL-TX",
        );
        let bytes = pcap(&[tx]);
        let contract = contract(sha256(&bytes));
        let result = inspect_pcap(&bytes, &contract).unwrap();
        assert_eq!(result.rx_frames, 0);
        let error = require_bidirectional_capture(&result, &contract).unwrap_err();
        assert!(error.to_string().contains("capture markers incomplete"));
    }

    #[test]
    fn pcapng_enhanced_packets_bind_the_same_physical_nic_and_peer() {
        let tx = ethernet(
            [0x02, 0, 0, 0, 0, 1],
            [0x02, 0, 0, 0, 0, 2],
            b"NCC-PHYSICAL-TX",
        );
        let rx = ethernet(
            [0x02, 0, 0, 0, 0, 2],
            [0x02, 0, 0, 0, 0, 1],
            b"NCC-PHYSICAL-RX",
        );
        let bytes = pcapng(&[tx, rx]);
        let result = inspect_pcapng(&bytes, &contract(sha256(&bytes))).unwrap();
        assert_eq!(result.format, "pcapng");
        assert_eq!((result.tx_frames, result.rx_frames), (1, 1));
    }
}
