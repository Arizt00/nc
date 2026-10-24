use super::*;

fn primitive_detect_serial_devices() -> Vec<String> {
    let mut out = Vec::new();
    for prefix in ["/dev/ttyS", "/dev/ttyUSB", "/dev/ttyACM"] {
        for idx in 0..64 {
            let path = format!("{prefix}{idx}");
            if PathBuf::from(&path).exists() {
                out.push(path);
            }
        }
    }
    let by_id = PathBuf::from("/dev/serial/by-id");
    if let Ok(entries) = fs::read_dir(by_id) {
        for entry in entries.flatten() {
            out.push(entry.path().to_string_lossy().to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn primitive_detect_sysfs_devices(path: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten().take(limit) {
            out.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    out.sort();
    out
}

fn primitive_real_allowlist_path() -> PathBuf {
    workspace_repo_root().join("primitive.hardware.allowlist.json")
}

fn primitive_current_uid() -> Option<u32> {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|raw| raw.trim().parse::<u32>().ok())
}

pub(super) fn handle_doctor(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, _) = primitive_parse_standard_options(sub_args)?;
    let root = workspace_repo_root();
    let workspace_ok =
        root.join("Cargo.toml").exists() && root.join("crates/ncc-cli/src/main.rs").exists();
    if !workspace_ok {
        return Err("E_PRIMITIVE_CLI_WORKSPACE_001: workspace is not a valid nC workspace".into());
    }
    let qemu_available = Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_ok();
    if opts.strict && !qemu_available {
        return Err("E_PRIMITIVE_CLI_QEMU_MISSING_001: qemu-system-x86_64 not found".into());
    }
    if opts.strict {
        if let Some(target_name) = opts.target.as_deref() {
            let spec = load_primitive_target_spec(target_name)?;
            let env_name = if let Some(value) = opts.environment.as_deref() {
                value.to_string()
            } else {
                spec.field("environment")
                    .ok_or("E_PRIMITIVE_ENV_MISSING_001: primitive[@] requires an explicit material environment")?
                    .to_string()
            };
            primitive_enforce_strict_qemu_certification_guard(&opts, &spec, &env_name)?;
        } else if let Some(env_name) = opts.environment.as_deref() {
            if env_name.to_ascii_lowercase().contains("qemu")
                || env_name.to_ascii_lowercase().contains("emulated")
            {
                return Err("E_PRIMITIVE_QEMU_NOT_REAL_HW_001: QEMU environment cannot certify this operation as physical hardware".into());
            }
        }
    }
    let targets = list_primitive_target_specs()?;
    let serial_devices = primitive_detect_serial_devices();
    let usb_devices = primitive_detect_sysfs_devices("/sys/bus/usb/devices", 128);
    let pci_devices = primitive_detect_sysfs_devices("/sys/bus/pci/devices", 128);
    let allowlist_path = primitive_real_allowlist_path();
    let allowlist_exists = allowlist_path.exists();
    let uid = primitive_current_uid();
    let port_io_allowed = uid == Some(0) && PathBuf::from("/dev/port").exists();
    let mmio_allowed = false;
    let real_hardware_available = false;
    let status = if opts.real || opts.hardware {
        if real_hardware_available {
            "ok_real_hardware"
        } else {
            "blocked_no_hardware"
        }
    } else if qemu_available {
        "ok_emulated"
    } else {
        "ok_freestanding"
    };
    let report = serde_json::json!({
        "schema": if opts.real || opts.hardware || opts.evidence { "nc.primitive.doctor.real.v1" } else { "nc.primitive.doctor.v1" },
        "workspace": root.to_string_lossy(),
        "workspace_ok": workspace_ok,
        "qemu_available": qemu_available,
        "real_hardware_available": real_hardware_available,
        "real_hardware_targets_detected": [],
        "serial_devices_detected": serial_devices,
        "usb_devices_detected": usb_devices,
        "pci_devices_detected": pci_devices,
        "permissions_ok": port_io_allowed,
        "udev_rules_ok": PathBuf::from("/etc/udev/rules.d").exists(),
        "port_io_allowed": port_io_allowed,
        "mmio_allowed": mmio_allowed,
        "required_capabilities": [
            "primitive.hardware.allowlisted-device",
            "primitive.portio.privileged",
            "primitive.mmio.safe-map"
        ],
        "required_policies": [
            "primitive.no-false-hardware-claims",
            "primitive.no-unsafe-mmio-write",
            "primitive.real-write-requires-allowlist"
        ],
        "targets_count": targets.len(),
        "targets": targets.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
        "protocols": ["UART", "PCIE", "USB", "SATA", "NVME", "I2C", "SPI", "ETHERNET", "HDMI", "DISPLAYPORT"],
        "evidence_root": "reports/primitive_real",
        "artifact_root": "reports/primitive_real/artifacts",
        "allowlist": {
            "path": allowlist_path.to_string_lossy(),
            "exists": allowlist_exists,
            "default_policy": "real writes blocked unless exact or safe pseudo-terminal device is allowlisted"
        },
        "evidence_level": if real_hardware_available { "hardware_detected" } else if qemu_available { "qemu_emulated" } else { "freestanding_built" },
        "claim": if real_hardware_available { "hardware_observed" } else if qemu_available { "qemu_emulated" } else { "freestanding_artifact" },
        "status": status,
        "diagnostics": if status == "blocked_no_hardware" {
            serde_json::json!([{
                "code": "E_PRIMITIVE_REAL_HARDWARE_MISSING_001",
                "message": "no allowlisted real hardware device has been verified on this host"
            }])
        } else {
            serde_json::json!([])
        },
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("primitive doctor");
        println!("workspace: {}", root.to_string_lossy());
        println!("workspace_ok: {}", workspace_ok);
        println!("qemu_available: {}", qemu_available);
        println!("real_hardware_available: {}", real_hardware_available);
        println!(
            "serial_devices_detected: {}",
            report["serial_devices_detected"]
                .as_array()
                .map(|v| v.len())
                .unwrap_or(0)
        );
        println!("allowlist: {}", allowlist_path.to_string_lossy());
        println!("status: {}", status);
        println!("targets: {}", targets.len());
    }
    Ok(())
}
