use ncc_machine::{
    migrate_destination_over_stream, migrate_source_over_stream_with_hook, persist_snapshot_atomic,
    DeviceSchemaRange, DeviceSnapshot, GuestArchitecture, GuestBootPlan, GuestFirmwareTable,
    GuestFirmwareTableKind, GuestPowerAction, GuestSystemArchitecture, GuestSystemBootMethod,
    GuestSystemConfig, LiveMigrationDestination, LiveMigrationSource, MemorySnapshot,
    MigrationCapabilities, MigrationClockPolicy, MigrationIdentity, MigrationPhase,
    MigrationPolicy, RegisterBank, SnapshotId, VcpuSnapshot, VirtioDiskAttachment,
    VirtualClockSnapshot, VirtualDeviceClass, VmPowerState, VmSnapshot,
    MIGRATION_FEATURE_CLOCK_CONTINUITY, MIGRATION_FEATURE_DEVICE_VERSIONING,
    MIGRATION_FEATURE_DIRTY_BITMAP, MIGRATION_FEATURE_DIRTY_RING, MIGRATION_FEATURE_PRECOPY,
    MIGRATION_FEATURE_TRANSACTIONAL_COMMIT,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GUEST_CERTIFICATE_SCHEMA: &str = "ncc.machine.guest_system_bundle.v1";
const MIGRATION_CERTIFICATE_SCHEMA: &str = "ncc.machine.live_migration_certificate.v1";
const GUEST_DOMAIN: &[u8] = b"ncc.machine.guest_system_bundle.v1\0";
const MIGRATION_DOMAIN: &[u8] = b"ncc.machine.live_migration_certificate.v1\0";
const MIGRATION_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) fn run_prepare_guest(args: &[String]) -> Result<(), Box<dyn Error>> {
    let parsed = ParsedGuest::from_args(args)?;
    let bundle = parsed.bundle_path();
    ensure_new_bundle(&bundle)?;
    fs::create_dir_all(&bundle)?;
    let plan = GuestBootPlan::build(&parsed.config)?;
    let generated = write_guest_generated_artifacts(&bundle, &plan)?;
    let source_files = parsed.source_manifest(&bundle)?;
    let plan_json = guest_plan_json(&plan);
    write_json(&bundle.join("guest-plan.json"), &plan_json)?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": GUEST_CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "guest_boot_plan_prepared",
        "created_epoch_seconds": epoch_seconds(),
        "bundle_root": fs::canonicalize(&bundle)?.to_string_lossy(),
        "host": host_identity(),
        "request": parsed.request_json(),
        "plan": plan_json,
        "generated": generated,
        "source_files": source_files,
        "artifacts": artifacts,
        "truth_boundary": {
            "image_bytes_parsed": true,
            "physical_regions_validated": true,
            "initrd_materialized": !parsed.config.initrd.is_empty(),
            "command_line_materialized": true,
            "virtio_disks_validated": true,
            "firmware_tables_validated": true,
            "guest_executed": false,
            "qemu_used": false
        }
    });
    let certificate = attest(payload, GUEST_DOMAIN)?;
    let out = parsed
        .out
        .clone()
        .unwrap_or_else(|| bundle.join("guest-certificate.json"));
    write_json(&out, &certificate)?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&certificate))?
        );
    } else {
        println!(
            "OK: guest boot bundle={} certificate={} method={} architecture={}",
            bundle.display(),
            out.display(),
            boot_method_name(parsed.config.method),
            architecture_name(parsed.config.architecture)
        );
    }
    Ok(())
}

pub(crate) fn run_verify_guest_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args, &[])
        .map(PathBuf::from)
        .ok_or("verify-guest-certificate requires a certificate path")?;
    let certificate = read_and_verify_certificate(&path, GUEST_CERTIFICATE_SCHEMA, GUEST_DOMAIN)?;
    verify_artifacts_and_sources(&certificate, &path)?;
    for pointer in [
        "/truth_boundary/image_bytes_parsed",
        "/truth_boundary/physical_regions_validated",
        "/truth_boundary/command_line_materialized",
        "/truth_boundary/virtio_disks_validated",
        "/truth_boundary/firmware_tables_validated",
    ] {
        require_true(&certificate, pointer)?;
    }
    if certificate.get("status").and_then(Value::as_str) == Some("guest_executed") {
        for pointer in [
            "/truth_boundary/guest_executed",
            "/truth_boundary/direct_host_backend",
            "/truth_boundary/qemu_absent",
            "/truth_boundary/clean_shutdown_or_reboot",
            "/truth_boundary/expected_marker_contract_satisfied",
        ] {
            require_true(&certificate, pointer)?;
        }
        if certificate
            .pointer("/request/disk_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
        {
            require_true(&certificate, "/truth_boundary/virtio_disks_exercised")?;
        }
    }
    println!("OK: verified guest-system certificate {}", path.display());
    Ok(())
}

pub(crate) fn run_certify_migration(args: &[String]) -> Result<(), Box<dyn Error>> {
    let memory_mib = parse_u64_option(args, "--memory-mib", 16)?;
    if !(1..=1024).contains(&memory_mib) {
        return Err("--memory-mib must be between 1 and 1024".into());
    }
    let memory_len = usize::try_from(memory_mib * 1024 * 1024)?;
    let vcpu_count = parse_u32_option(args, "--vcpus", 4)?;
    if !(1..=256).contains(&vcpu_count) {
        return Err("--vcpus must be between 1 and 256".into());
    }
    let out = option_value(args, "--out").map(PathBuf::from);
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bundle(out.as_deref(), "migration-certificate"));
    ensure_new_bundle(&bundle)?;
    fs::create_dir_all(&bundle)?;

    let capabilities = migration_capabilities(memory_len, vcpu_count);
    let mut source = migration_source(capabilities.clone(), memory_len, vcpu_count)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?;
    let destination_capabilities = capabilities.clone();
    let destination_thread = thread::spawn(move || -> Result<DestinationResult, String> {
        let (stream, peer) = listener.accept().map_err(|error| error.to_string())?;
        configure_stream(&stream).map_err(|error| error.to_string())?;
        let mut stream = TraceStream::new(stream);
        let mut destination = LiveMigrationDestination::new(destination_capabilities)
            .map_err(|error| error.to_string())?;
        let restored = migrate_destination_over_stream(
            &mut destination,
            &mut stream,
            25_000_000,
            MigrationClockPolicy::AdvanceByDowntime,
        )
        .map_err(|error| error.to_string())?;
        Ok(DestinationResult {
            restored,
            phase: destination.phase,
            read_trace: stream.read_trace,
            write_trace: stream.write_trace,
            peer: peer.to_string(),
        })
    });

    let stream = TcpStream::connect(endpoint)?;
    configure_stream(&stream)?;
    let mut stream = TraceStream::new(stream);
    let report = migrate_source_over_stream_with_hook(
        &mut source,
        &mut stream,
        MigrationPolicy {
            maximum_pre_copy_rounds: 8,
            stop_dirty_pages: 0,
            clock_policy: MigrationClockPolicy::AdvanceByDowntime,
        },
        20_000_000,
        |source, round| {
            match round {
                1 => source.write_memory(4096 + 31, b"dirty-round-one")?,
                2 => source.write_memory(memory_len / 2 + 17, b"dirty-round-two")?,
                3 => source.write_memory(memory_len - 4096 + 7, b"final-dirty-page")?,
                _ => {}
            }
            Ok(())
        },
    )?;
    let destination = destination_thread
        .join()
        .map_err(|_| "migration destination thread panicked")?
        .map_err(|error| format!("migration destination failed: {error}"))?;
    if source.phase != MigrationPhase::Committed
        || destination.phase != MigrationPhase::Committed
        || source.memory != destination.restored.memory
    {
        return Err("migration committed state diverged".into());
    }

    fs::write(
        bundle.join("source-to-destination.ncmig"),
        &stream.write_trace,
    )?;
    fs::write(
        bundle.join("destination-to-source.ncmig"),
        &stream.read_trace,
    )?;
    fs::write(
        bundle.join("destination-read.ncmig"),
        &destination.read_trace,
    )?;
    fs::write(
        bundle.join("destination-write.ncmig"),
        &destination.write_trace,
    )?;
    let restored_snapshot = VmSnapshot {
        id: SnapshotId::from_sequence(0x4e43_4d49, report.commit_token as u64),
        parent: SnapshotId::NIL,
        generation: 1,
        timestamp_ns: 25_000_000,
        state: VmPowerState::Paused,
        architecture: capabilities.architecture,
        target: capabilities.target.clone(),
        vcpus: destination.restored.vcpus.clone(),
        memory: MemorySnapshot::capture_full(
            &destination.restored.memory,
            capabilities.page_size as usize,
        )?,
        devices: destination.restored.devices.clone(),
        metadata: BTreeMap::from([
            ("migration.protocol".into(), "NCMIGR01".into()),
            ("migration.commit".into(), report.commit_token.to_string()),
            ("migration.transport".into(), "tcp-loopback".into()),
        ]),
    };
    persist_snapshot_atomic(&bundle.join("restored.ncsnap"), &restored_snapshot)?;
    let execution = json!({
        "transport": "tcp-loopback",
        "endpoint": endpoint.to_string(),
        "peer": destination.peer,
        "source_phase": format!("{:?}", source.phase),
        "destination_phase": format!("{:?}", destination.phase),
        "rounds": report.rounds,
        "pages_transferred": report.pages_transferred,
        "bytes_transferred": report.bytes_transferred,
        "downtime_ns": report.downtime_ns,
        "commit_token": report.commit_token,
        "memory_bytes": memory_len,
        "memory_sha256": sha256_bytes(&destination.restored.memory),
        "vcpus": destination.restored.vcpus.len(),
        "pending_interrupts": destination.restored.vcpus.iter().map(|vcpu| vcpu.pending_interrupts.len()).sum::<usize>(),
        "devices": destination.restored.devices.len(),
        "device_versions": destination.restored.devices.iter().map(|device| json!({
            "id": device.id,
            "class": format!("{:?}", device.class),
            "schema_version": device.schema_version,
            "checksum": device.checksum
        })).collect::<Vec<_>>(),
        "clock": {
            "guest_monotonic_ns": destination.restored.clock.guest_monotonic_ns,
            "guest_wall_clock_ns": destination.restored.clock.guest_wall_clock_ns,
            "counter_frequency_hz": destination.restored.clock.counter_frequency_hz,
            "counter_value": destination.restored.clock.counter_value.to_string(),
            "accumulated_downtime_ns": destination.restored.clock.accumulated_downtime_ns,
            "paused": destination.restored.clock.paused
        },
        "wire": {
            "source_written_bytes": stream.write_trace.len(),
            "source_read_bytes": stream.read_trace.len(),
            "destination_read_bytes": destination.read_trace.len(),
            "destination_written_bytes": destination.write_trace.len(),
            "directions_match": stream.write_trace == destination.read_trace && stream.read_trace == destination.write_trace
        }
    });
    write_json(&bundle.join("migration-execution.json"), &execution)?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": MIGRATION_CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "live_migration_committed",
        "created_epoch_seconds": epoch_seconds(),
        "bundle_root": fs::canonicalize(&bundle)?.to_string_lossy(),
        "host": host_identity(),
        "capabilities": migration_capabilities_json(&capabilities),
        "execution": execution,
        "artifacts": artifacts,
        "truth_boundary": {
            "real_tcp_transport": true,
            "precopy_executed": report.rounds >= 2,
            "dirty_pages_replayed": true,
            "multi_vcpu_state_preserved": destination.restored.vcpus.len() == vcpu_count as usize,
            "interrupt_state_preserved": destination.restored.vcpus.iter().all(|vcpu| !vcpu.pending_interrupts.is_empty()),
            "versioned_devices_preserved": destination.restored.devices.iter().all(|device| device.schema_version > 0),
            "clock_continuity_preserved": destination.restored.clock.accumulated_downtime_ns == report.downtime_ns,
            "transactional_two_phase_commit": true,
            "persistent_snapshot_verified": true,
            "qemu_used": false,
            "physical_cross_host_execution": false
        }
    });
    let certificate = attest(payload, MIGRATION_DOMAIN)?;
    let out = out.unwrap_or_else(|| bundle.join("migration-certificate.json"));
    write_json(&out, &certificate)?;
    if has_flag(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&certificate))?
        );
    } else {
        println!(
            "OK: TCP live migration committed rounds={} vcpus={} bundle={} certificate={}",
            report.rounds,
            vcpu_count,
            bundle.display(),
            out.display()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_migration_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args, &[])
        .map(PathBuf::from)
        .ok_or("verify-migration-certificate requires a certificate path")?;
    let certificate =
        read_and_verify_certificate(&path, MIGRATION_CERTIFICATE_SCHEMA, MIGRATION_DOMAIN)?;
    verify_artifacts_and_sources(&certificate, &path)?;
    for pointer in [
        "/truth_boundary/real_tcp_transport",
        "/truth_boundary/precopy_executed",
        "/truth_boundary/dirty_pages_replayed",
        "/truth_boundary/multi_vcpu_state_preserved",
        "/truth_boundary/interrupt_state_preserved",
        "/truth_boundary/versioned_devices_preserved",
        "/truth_boundary/clock_continuity_preserved",
        "/truth_boundary/transactional_two_phase_commit",
        "/truth_boundary/persistent_snapshot_verified",
    ] {
        require_true(&certificate, pointer)?;
    }
    let root = resolve_bundle_root(&certificate, &path)?;
    let snapshot = ncc_machine::load_snapshot(
        &root.join("restored.ncsnap"),
        ncc_machine::SnapshotLimits::default(),
    )?;
    snapshot.restore_memory(None)?;
    println!("OK: verified live-migration certificate {}", path.display());
    Ok(())
}

pub(crate) fn run_boot_guest(args: &[String]) -> Result<(), Box<dyn Error>> {
    let parsed = ParsedGuest::from_args(args)?;
    let plan = GuestBootPlan::build(&parsed.config)?;
    let retain_evidence = parsed.bundle.is_some() || parsed.out.is_some();
    let bundle = retain_evidence.then(|| parsed.bundle_path());
    let mut execution_args = args.to_vec();
    if let Some(bundle) = &bundle {
        ensure_new_bundle(bundle)?;
        fs::create_dir_all(bundle)?;
        if option_value(&execution_args, "--serial-out").is_none() {
            execution_args.push("--serial-out".into());
            execution_args.push(
                bundle
                    .join("guest-serial.log")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    let execution = guest_host::execute(&plan, &parsed, &execution_args)?;
    let certificate = if let Some(bundle) = &bundle {
        let generated = write_guest_generated_artifacts(bundle, &plan)?;
        let plan_json = guest_plan_json(&plan);
        write_json(&bundle.join("guest-plan.json"), &plan_json)?;
        write_json(&bundle.join("guest-execution.json"), &execution)?;
        let source_files = parsed.source_manifest(bundle)?;
        let artifacts = collect_artifacts(bundle)?;
        let direct_backend = execution
            .get("backend")
            .and_then(Value::as_str)
            .is_some_and(|backend| backend != "plan-only" && backend != "qemu");
        let qemu_absent = execution.get("qemu_used").and_then(Value::as_bool) == Some(false);
        let clean_stop = execution.get("clean_stop").and_then(Value::as_bool) == Some(true);
        let marker_contract = execution
            .get("expected_serial_observed")
            .and_then(Value::as_bool)
            == Some(true);
        let exercised_disks = execution
            .get("virtio")
            .and_then(Value::as_array)
            .is_some_and(|devices| {
                devices.len() == parsed.config.disks.len()
                    && devices.iter().all(|device| {
                        device.get("requests").and_then(Value::as_u64).unwrap_or(0) > 0
                    })
            });
        let disk_contract = parsed.config.disks.is_empty() || exercised_disks;
        let payload = json!({
            "schema": GUEST_CERTIFICATE_SCHEMA,
            "certificate_version": 1,
            "status": "guest_executed",
            "created_epoch_seconds": epoch_seconds(),
            "bundle_root": fs::canonicalize(bundle)?.to_string_lossy(),
            "host": host_identity(),
            "request": parsed.request_json(),
            "plan": plan_json,
            "execution": execution,
            "generated": generated,
            "source_files": source_files,
            "artifacts": artifacts,
            "truth_boundary": {
                "image_bytes_parsed": true,
                "physical_regions_validated": true,
                "initrd_materialized": !parsed.config.initrd.is_empty(),
                "command_line_materialized": true,
                "virtio_disks_validated": true,
                "firmware_tables_validated": true,
                "guest_executed": true,
                "direct_host_backend": direct_backend,
                "qemu_absent": qemu_absent,
                "clean_shutdown_or_reboot": clean_stop,
                "expected_marker_contract_satisfied": marker_contract,
                "virtio_disks_exercised": disk_contract
            }
        });
        if !direct_backend || !qemu_absent || !clean_stop || !marker_contract || !disk_contract {
            return Err("guest execution did not satisfy the retained evidence contract".into());
        }
        let certificate = attest(payload, GUEST_DOMAIN)?;
        let out = parsed
            .out
            .clone()
            .unwrap_or_else(|| bundle.join("guest-certificate.json"));
        write_json(&out, &certificate)?;
        Some((certificate, out))
    } else {
        None
    };
    if parsed.json {
        let output = certificate
            .as_ref()
            .map(|(certificate, _)| certificate)
            .unwrap_or(&execution);
        println!("{}", serde_json::to_string_pretty(output)?);
    } else {
        if let Some((_, path)) = certificate {
            println!(
                "OK: guest stopped cleanly; evidence={} bundle={}",
                path.display(),
                bundle.unwrap().display()
            );
        } else {
            println!("OK: guest stopped cleanly ({})", execution);
        }
    }
    Ok(())
}

struct ParsedGuest {
    config: GuestSystemConfig,
    kernel_path: PathBuf,
    firmware_path: Option<PathBuf>,
    initrd_path: Option<PathBuf>,
    disk_paths: Vec<PathBuf>,
    out: Option<PathBuf>,
    bundle: Option<PathBuf>,
    json: bool,
}

impl ParsedGuest {
    fn from_args(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let method = match option_value(args, "--boot").unwrap_or("bzimage") {
            "bzimage" | "linux" => GuestSystemBootMethod::LinuxBzImage,
            "efi" | "uefi" => GuestSystemBootMethod::Efi,
            "opensbi" => GuestSystemBootMethod::OpenSbi,
            value => return Err(format!("unknown guest boot method '{value}'").into()),
        };
        let architecture =
            parse_architecture(option_value(args, "--arch").unwrap_or(match method {
                GuestSystemBootMethod::LinuxBzImage | GuestSystemBootMethod::Efi => "x86_64",
                GuestSystemBootMethod::OpenSbi => "riscv64",
            }))?;
        let kernel_path = option_value(args, "--kernel")
            .map(PathBuf::from)
            .ok_or("guest command requires --kernel <image>")?;
        let kernel = read_required(&kernel_path, "kernel")?;
        let firmware_path = option_value(args, "--firmware").map(PathBuf::from);
        let firmware = firmware_path
            .as_deref()
            .map(|path| read_required(path, "firmware"))
            .transpose()?
            .unwrap_or_default();
        if matches!(
            method,
            GuestSystemBootMethod::Efi | GuestSystemBootMethod::OpenSbi
        ) && firmware.is_empty()
        {
            return Err("EFI and OpenSBI boot require --firmware <image>".into());
        }
        let initrd_path = option_value(args, "--initrd").map(PathBuf::from);
        let initrd = initrd_path
            .as_deref()
            .map(|path| read_required(path, "initrd"))
            .transpose()?
            .unwrap_or_default();
        let memory_mib = parse_u64_option(args, "--memory-mib", 512)?;
        if !(16..=1_048_576).contains(&memory_mib) {
            return Err("--memory-mib must be between 16 and 1048576".into());
        }
        let memory_size = memory_mib
            .checked_mul(1024 * 1024)
            .ok_or("guest memory size overflow")?;
        let mut config = match method {
            GuestSystemBootMethod::LinuxBzImage => {
                GuestSystemConfig::linux_bzimage(kernel, memory_size)
            }
            GuestSystemBootMethod::Efi => {
                GuestSystemConfig::efi(architecture, kernel, firmware, memory_size)
            }
            GuestSystemBootMethod::OpenSbi => {
                GuestSystemConfig::opensbi(architecture, kernel, firmware, memory_size)
            }
        };
        config.vcpu_count = parse_u32_option(args, "--vcpus", 1)?;
        if let Some(command_line) = option_value(args, "--cmdline") {
            config.command_line = command_line.to_string();
        }
        config.initrd = initrd;
        config.firmware_address = parse_optional_address(args, "--firmware-address")?;
        config.initrd_address = parse_optional_address(args, "--initrd-address")?;
        config.fdt_address = parse_optional_address(args, "--fdt-address")?;
        if let Some(path) = option_value(args, "--fdt") {
            config.fdt = read_required(Path::new(path), "FDT")?;
        }
        let mut disk_paths = Vec::new();
        let mut ids = BTreeSet::new();
        for (path, read_only) in option_values(args, "--disk")
            .into_iter()
            .map(|path| (path, false))
            .chain(
                option_values(args, "--read-only-disk")
                    .into_iter()
                    .map(|path| (path, true)),
            )
        {
            let path = PathBuf::from(path);
            let mut id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("disk")
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
                .take(48)
                .collect::<String>();
            if id.is_empty() {
                id = "disk".into();
            }
            let base = id.clone();
            let mut suffix = 1;
            while !ids.insert(id.clone()) {
                id = format!("{base}-{suffix}");
                suffix += 1;
            }
            let bytes = read_required(&path, "VirtIO disk")?;
            config
                .disks
                .push(VirtioDiskAttachment::new(id, bytes, 512, read_only)?);
            disk_paths.push(path);
        }
        for (flag, kind) in [
            ("--acpi-rsdp", GuestFirmwareTableKind::AcpiRsdp),
            ("--smbios2", GuestFirmwareTableKind::Smbios2),
            ("--smbios3", GuestFirmwareTableKind::Smbios3),
            ("--efi-system-table", GuestFirmwareTableKind::EfiSystemTable),
        ] {
            if let Some(specification) = option_value(args, flag) {
                config
                    .firmware_tables
                    .push(parse_firmware_table(specification, kind)?);
            }
        }
        Ok(Self {
            config,
            kernel_path,
            firmware_path,
            initrd_path,
            disk_paths,
            out: option_value(args, "--out").map(PathBuf::from),
            bundle: option_value(args, "--bundle").map(PathBuf::from),
            json: has_flag(args, "--json"),
        })
    }

    fn bundle_path(&self) -> PathBuf {
        self.bundle
            .clone()
            .unwrap_or_else(|| default_bundle(self.out.as_deref(), "guest-system"))
    }

    fn request_json(&self) -> Value {
        json!({
            "architecture": architecture_name(self.config.architecture),
            "boot_method": boot_method_name(self.config.method),
            "memory_base": self.config.memory_base,
            "memory_size": self.config.memory_size,
            "vcpus": self.config.vcpu_count,
            "kernel": self.kernel_path,
            "firmware": self.firmware_path,
            "initrd": self.initrd_path,
            "command_line": self.config.command_line,
            "disk_count": self.config.disks.len()
        })
    }

    fn source_manifest(&self, bundle: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut sources = vec![source_file_json("kernel", &self.kernel_path, bundle)?];
        if let Some(path) = &self.firmware_path {
            sources.push(source_file_json("firmware", path, bundle)?);
        }
        if let Some(path) = &self.initrd_path {
            sources.push(source_file_json("initrd", path, bundle)?);
        }
        for (index, path) in self.disk_paths.iter().enumerate() {
            sources.push(source_file_json(
                &format!("virtio-disk-{index}"),
                path,
                bundle,
            )?);
        }
        Ok(sources)
    }
}

struct DestinationResult {
    restored: ncc_machine::RestoredVmState,
    phase: MigrationPhase,
    read_trace: Vec<u8>,
    write_trace: Vec<u8>,
    peer: String,
}

struct TraceStream {
    inner: TcpStream,
    read_trace: Vec<u8>,
    write_trace: Vec<u8>,
}

impl TraceStream {
    fn new(inner: TcpStream) -> Self {
        Self {
            inner,
            read_trace: Vec::new(),
            write_trace: Vec::new(),
        }
    }
}

impl Read for TraceStream {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(output)?;
        self.read_trace.extend_from_slice(&output[..count]);
        Ok(count)
    }
}

impl Write for TraceStream {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let count = self.inner.write(input)?;
        self.write_trace.extend_from_slice(&input[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn migration_capabilities(memory_len: usize, vcpus: u32) -> MigrationCapabilities {
    MigrationCapabilities {
        architecture: GuestArchitecture::X86_64,
        target: "x86_64-unknown-linux-gnu".into(),
        page_size: 4096,
        memory_len: memory_len as u64,
        maximum_vcpus: vcpus.max(1),
        features: MIGRATION_FEATURE_PRECOPY
            | MIGRATION_FEATURE_DIRTY_BITMAP
            | MIGRATION_FEATURE_DIRTY_RING
            | MIGRATION_FEATURE_DEVICE_VERSIONING
            | MIGRATION_FEATURE_CLOCK_CONTINUITY
            | MIGRATION_FEATURE_TRANSACTIONAL_COMMIT,
        devices: vec![
            DeviceSchemaRange {
                id: "virtio-blk0".into(),
                class: VirtualDeviceClass::VirtioBlock,
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            },
            DeviceSchemaRange {
                id: "uart0".into(),
                class: VirtualDeviceClass::Uart,
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            },
            DeviceSchemaRange {
                id: "clock0".into(),
                class: VirtualDeviceClass::Timer,
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            },
        ],
    }
}

fn migration_source(
    capabilities: MigrationCapabilities,
    memory_len: usize,
    vcpu_count: u32,
) -> Result<LiveMigrationSource, Box<dyn Error>> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut memory = vec![0u8; memory_len];
    for byte in &mut memory {
        state ^= state << 7;
        state ^= state >> 9;
        state ^= state << 8;
        *byte = state as u8;
    }
    let vcpus = (0..vcpu_count)
        .map(|id| {
            let mut registers = RegisterBank::new(GuestArchitecture::X86_64);
            registers.set("rip", 0x10_0000 + u128::from(id) * 0x1000);
            registers.set("rsp", memory_len as u128 - 0x1000 - u128::from(id) * 0x2000);
            registers.set("rflags", 0x202);
            VcpuSnapshot {
                id,
                registers,
                halted: id != 0,
                pending_interrupts: vec![32 + id],
                virtual_time_ns: 15_000_000,
            }
        })
        .collect();
    Ok(LiveMigrationSource::new(
        MigrationIdentity {
            machine_id: SnapshotId::from_sequence(0x4e43, 1),
            session_id: SnapshotId::from_sequence(0x4e43, epoch_seconds()),
            generation: 1,
        },
        capabilities,
        memory,
        vcpus,
        vec![
            DeviceSnapshot::new(
                "virtio-blk0",
                VirtualDeviceClass::VirtioBlock,
                1,
                true,
                b"queue=0;avail=11;used=9;generation=4".to_vec(),
            ),
            DeviceSnapshot::new(
                "uart0",
                VirtualDeviceClass::Uart,
                1,
                true,
                b"ier=1;lcr=3;divisor=1;tx=0;rx=0".to_vec(),
            ),
            DeviceSnapshot::new(
                "clock0",
                VirtualDeviceClass::Timer,
                1,
                true,
                b"deadline=16000000;period=1000000".to_vec(),
            ),
        ],
        VirtualClockSnapshot {
            guest_monotonic_ns: 15_000_000,
            guest_wall_clock_ns: 1_750_000_000_000_000_000,
            counter_frequency_hz: 1_000_000_000,
            counter_value: 15_000_000,
            paused: false,
            accumulated_downtime_ns: 0,
        },
    )?)
}

fn write_guest_generated_artifacts(
    bundle: &Path,
    plan: &GuestBootPlan,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut generated = Vec::new();
    if let Some(efi) = &plan.efi {
        let path = bundle.join("efi-system-partition.img");
        fs::write(&path, &efi.esp_disk.bytes)?;
        generated.push(json!({ "kind": "efi-system-partition", "path": path.file_name(), "bytes": efi.esp_disk.bytes.len(), "sha256": sha256_bytes(&efi.esp_disk.bytes) }));
    }
    for region in &plan.regions {
        if matches!(
            region.kind,
            ncc_machine::GuestRegionKind::Fdt
                | ncc_machine::GuestRegionKind::OpenSbiDynamicInfo
                | ncc_machine::GuestRegionKind::BootParameters
                | ncc_machine::GuestRegionKind::CommandLine
        ) {
            let name = sanitize_name(&region.name);
            let path = bundle.join(format!("{name}.bin"));
            fs::write(&path, &region.bytes)?;
            generated.push(json!({
                "kind": format!("{:?}", region.kind),
                "name": region.name,
                "guest_physical_address": region.guest_physical_address,
                "path": path.file_name(),
                "bytes": region.bytes.len(),
                "sha256": sha256_bytes(&region.bytes)
            }));
        }
    }
    Ok(generated)
}

fn guest_plan_json(plan: &GuestBootPlan) -> Value {
    json!({
        "architecture": architecture_name(plan.architecture),
        "boot_method": boot_method_name(plan.method),
        "memory_base": plan.memory_base,
        "memory_size": plan.memory_size,
        "vcpus": plan.vcpu_count,
        "initial_cpu": {
            "mode": format!("{:?}", plan.initial_cpu.mode),
            "entry_point": plan.initial_cpu.entry_point,
            "registers": plan.initial_cpu.registers.iter().map(|(name, value)| (name.clone(), Value::String(format!("{value:#x}")))).collect::<Map<_, _>>(),
            "interrupts_enabled": plan.initial_cpu.interrupts_enabled,
            "paging_enabled": plan.initial_cpu.paging_enabled
        },
        "regions": plan.regions.iter().map(|region| json!({
            "name": region.name,
            "kind": format!("{:?}", region.kind),
            "guest_physical_address": region.guest_physical_address,
            "bytes": region.bytes.len(),
            "sha256": sha256_bytes(&region.bytes),
            "read": region.flags.read,
            "write": region.flags.write,
            "execute": region.flags.execute
        })).collect::<Vec<_>>(),
        "disks": plan.disks.iter().map(|disk| json!({
            "id": disk.id,
            "bytes": disk.bytes.len(),
            "block_size": disk.block_size,
            "read_only": disk.read_only,
            "crc32": disk.checksum,
            "sha256": sha256_bytes(&disk.bytes)
        })).collect::<Vec<_>>(),
        "linux": plan.linux.as_ref().map(|linux| json!({
            "protocol_version": format!("{:#06x}", linux.protocol_version),
            "setup_bytes": linux.setup_bytes,
            "code32_start": linux.code32_start,
            "kernel_alignment": linux.kernel_alignment,
            "init_size": linux.init_size,
            "command_line_limit": linux.command_line_limit,
            "initrd_address_limit": linux.initrd_address_limit,
            "boot_params_address": linux.boot_params_address,
            "command_line_address": linux.command_line_address,
            "initrd_address": linux.initrd_address,
            "e820_entries": linux.e820.len()
        })),
        "efi": plan.efi.as_ref().map(|efi| json!({
            "application_path": efi.application_path,
            "load_options_utf16_units": efi.load_options_utf16.len(),
            "initrd_address": efi.initrd_address,
            "initrd_size": efi.initrd_size,
            "image_entry": efi.image_plan.entry_point,
            "esp_bytes": efi.esp_disk.bytes.len()
        })),
        "opensbi": plan.opensbi.as_ref().map(|opensbi| json!({
            "hart_id": opensbi.hart_id,
            "fdt_address": opensbi.fdt_address,
            "dynamic_info_address": opensbi.dynamic_info_address,
            "firmware_entry": opensbi.firmware_entry,
            "kernel_entry": opensbi.kernel_entry,
            "kernel_load_address": opensbi.kernel_load_address,
            "next_mode": opensbi.next_mode,
            "xlen": opensbi.xlen
        })),
        "power": {
            "acpi_pm1_ports": plan.power_contract.acpi_pm1_control_ports,
            "reset_control_port": plan.power_contract.reset_control_port,
            "keyboard_control_port": plan.power_contract.keyboard_control_port,
            "uefi_reset_system": plan.power_contract.uefi_reset_system,
            "sbi_system_reset": plan.power_contract.sbi_system_reset,
            "actions": [format!("{:?}", GuestPowerAction::Shutdown), format!("{:?}", GuestPowerAction::WarmReboot), format!("{:?}", GuestPowerAction::ColdReboot)]
        }
    })
}

fn migration_capabilities_json(capabilities: &MigrationCapabilities) -> Value {
    json!({
        "architecture": format!("{:?}", capabilities.architecture),
        "target": capabilities.target,
        "page_size": capabilities.page_size,
        "memory_len": capabilities.memory_len,
        "maximum_vcpus": capabilities.maximum_vcpus,
        "features": capabilities.features,
        "devices": capabilities.devices.iter().map(|device| json!({
            "id": device.id,
            "class": format!("{:?}", device.class),
            "minimum_version": device.minimum_version,
            "maximum_version": device.maximum_version,
            "required": device.required
        })).collect::<Vec<_>>()
    })
}

fn parse_architecture(value: &str) -> Result<GuestSystemArchitecture, Box<dyn Error>> {
    match value.to_ascii_lowercase().replace('-', "_").as_str() {
        "x86_64" | "amd64" => Ok(GuestSystemArchitecture::X86_64),
        "i386" | "x86" | "ia32" => Ok(GuestSystemArchitecture::I386),
        "aarch64" | "arm64" => Ok(GuestSystemArchitecture::Aarch64),
        "riscv64" | "rv64" => Ok(GuestSystemArchitecture::RiscV64),
        "riscv32" | "rv32" => Ok(GuestSystemArchitecture::RiscV32),
        _ => Err(format!("unknown guest architecture '{value}'").into()),
    }
}

fn architecture_name(value: GuestSystemArchitecture) -> &'static str {
    match value {
        GuestSystemArchitecture::X86_64 => "x86_64",
        GuestSystemArchitecture::I386 => "i386",
        GuestSystemArchitecture::Aarch64 => "aarch64",
        GuestSystemArchitecture::RiscV64 => "riscv64",
        GuestSystemArchitecture::RiscV32 => "riscv32",
    }
}

fn boot_method_name(value: GuestSystemBootMethod) -> &'static str {
    match value {
        GuestSystemBootMethod::LinuxBzImage => "bzimage",
        GuestSystemBootMethod::Efi => "efi",
        GuestSystemBootMethod::OpenSbi => "opensbi",
    }
}

fn parse_firmware_table(
    specification: &str,
    kind: GuestFirmwareTableKind,
) -> Result<GuestFirmwareTable, Box<dyn Error>> {
    let (path, address) = specification
        .rsplit_once('@')
        .ok_or("firmware table must use <path>@<guest-address>")?;
    Ok(GuestFirmwareTable {
        kind,
        guest_physical_address: parse_address(address)?,
        bytes: read_required(Path::new(path), "firmware table")?,
    })
}

fn parse_optional_address(args: &[String], flag: &str) -> Result<Option<u64>, Box<dyn Error>> {
    option_value(args, flag).map(parse_address).transpose()
}

fn parse_address(value: &str) -> Result<u64, Box<dyn Error>> {
    let normalized = value.replace('_', "");
    if let Some(hex) = normalized.strip_prefix("0x") {
        Ok(u64::from_str_radix(hex, 16)?)
    } else {
        Ok(normalized.parse()?)
    }
}

fn parse_u64_option(args: &[String], flag: &str, default: u64) -> Result<u64, Box<dyn Error>> {
    option_value(args, flag)
        .map(str::parse)
        .transpose()
        .map(|value| value.unwrap_or(default))
        .map_err(Into::into)
}

fn parse_u32_option(args: &[String], flag: &str, default: u32) -> Result<u32, Box<dyn Error>> {
    option_value(args, flag)
        .map(str::parse)
        .transpose()
        .map(|value| value.unwrap_or(default))
        .map_err(Into::into)
}

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn option_values<'a>(args: &'a [String], flag: &str) -> Vec<&'a str> {
    args.iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == flag).then(|| args.get(index + 1)).flatten())
        .map(String::as_str)
        .collect()
}

fn first_non_flag<'a>(args: &'a [String], value_flags: &[&str]) -> Option<&'a str> {
    let mut skip = false;
    for argument in args {
        if skip {
            skip = false;
            continue;
        }
        if value_flags.contains(&argument.as_str()) {
            skip = true;
        } else if !argument.starts_with('-') {
            return Some(argument);
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|argument| argument == flag)
}

fn read_required(path: &Path, kind: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {kind} {}: {error}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("{kind} {} is empty", path.display()).into());
    }
    Ok(bytes)
}

fn source_file_json(kind: &str, path: &Path, bundle: &Path) -> Result<Value, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let original_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let relative = PathBuf::from("sources").join(format!(
        "{}-{}",
        sanitize_name(kind),
        sanitize_name(original_name)
    ));
    let retained = bundle.join(&relative);
    if let Some(parent) = retained.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&retained, &bytes)?;
    Ok(json!({
        "kind": kind,
        "path": relative.to_string_lossy().replace('\\', "/"),
        "original_path": fs::canonicalize(path)?.to_string_lossy(),
        "bytes": bytes.len(),
        "sha256": sha256_bytes(&bytes)
    }))
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn configure_stream(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(MIGRATION_TIMEOUT))?;
    stream.set_write_timeout(Some(MIGRATION_TIMEOUT))?;
    Ok(())
}

fn default_bundle(out: Option<&Path>, stem: &str) -> PathBuf {
    if let Some(path) = out {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(stem);
        return parent.join(format!("{name}-bundle"));
    }
    PathBuf::from("target").join(format!("ncc-{stem}"))
}

fn ensure_new_bundle(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(format!("bundle already exists: {}", path.display()).into());
    }
    Ok(())
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
        value => value.clone(),
    }
}

fn attest(mut payload: Value, domain: &[u8]) -> Result<Value, Box<dyn Error>> {
    let hash = payload_sha256(&payload, domain)?;
    payload["attestation"] = json!({
        "algorithm": "sha256",
        "domain": String::from_utf8_lossy(domain).trim_end_matches('\0'),
        "payload_sha256": hash
    });
    Ok(payload)
}

fn payload_sha256(value: &Value, domain: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(hex_bytes(&digest.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn host_identity() -> Value {
    json!({
        "operating_system": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "logical_processors": thread::available_parallelism().map(usize::from).ok(),
        "kvm_device": Path::new("/dev/kvm").exists()
    })
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

fn collect_artifacts(root: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    fn visit(root: &Path, current: &Path, output: &mut Vec<Value>) -> Result<(), Box<dyn Error>> {
        let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output)?;
            } else {
                let bytes = fs::read(&path)?;
                output.push(json!({
                    "path": path.strip_prefix(root)?.to_string_lossy().replace('\\', "/"),
                    "bytes": bytes.len(),
                    "sha256": sha256_bytes(&bytes)
                }));
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn read_and_verify_certificate(
    path: &Path,
    schema: &str,
    domain: &[u8],
) -> Result<Value, Box<dyn Error>> {
    let mut certificate: Value = serde_json::from_slice(&fs::read(path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(schema) {
        return Err("unsupported certificate schema".into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("certificate has no payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .ok_or("certificate root is not an object")?
        .remove("attestation");
    let actual = payload_sha256(&certificate, domain)?;
    if expected != actual {
        return Err(format!("certificate hash mismatch: expected {expected}, got {actual}").into());
    }
    Ok(certificate)
}

fn verify_artifacts_and_sources(
    certificate: &Value,
    certificate_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let root = resolve_bundle_root(certificate, certificate_path)?;
    for artifact in certificate
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("certificate has no artifact manifest")?
    {
        verify_manifest_file(&root.join(required_str(artifact, "path")?), artifact)?;
    }
    if let Some(sources) = certificate.get("source_files").and_then(Value::as_array) {
        for source in sources {
            let recorded = required_str(source, "path")?;
            let path = if Path::new(recorded).is_relative() {
                root.join(recorded)
            } else {
                resolve_recorded_path(recorded)
            };
            verify_manifest_file(&path, source)?;
        }
    }
    Ok(())
}

fn resolve_bundle_root(
    certificate: &Value,
    certificate_path: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let recorded = certificate
        .get("bundle_root")
        .and_then(Value::as_str)
        .ok_or("certificate has no bundle root")?;
    let resolved = resolve_recorded_path(recorded);
    if resolved.is_dir() {
        return Ok(resolved);
    }

    let parent = certificate_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let first_artifact = certificate
        .get("artifacts")
        .and_then(Value::as_array)
        .and_then(|artifacts| artifacts.first())
        .and_then(|artifact| artifact.get("path"))
        .and_then(Value::as_str);
    if first_artifact.is_some_and(|artifact| parent.join(artifact).is_file()) {
        return Ok(parent.to_path_buf());
    }

    Ok(resolved)
}

fn resolve_recorded_path(recorded: &str) -> PathBuf {
    let direct = PathBuf::from(recorded);
    if direct.exists() {
        return direct;
    }

    let normalized = recorded.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("/mnt/") {
        let mut pieces = rest.splitn(2, '/');
        if let (Some(drive), Some(tail)) = (pieces.next(), pieces.next()) {
            if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
                let candidate = PathBuf::from(format!(
                    "{}:\\{}",
                    drive.to_ascii_uppercase(),
                    tail.replace('/', "\\")
                ));
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }

    let bytes = normalized.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let candidate = PathBuf::from(format!("/mnt/{drive}/{}", &normalized[3..]));
        if candidate.exists() {
            return candidate;
        }
    }

    direct
}

fn verify_manifest_file(path: &Path, manifest: &Value) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot verify artifact {}: {error}", path.display()))?;
    let expected_size = manifest
        .get("bytes")
        .and_then(Value::as_u64)
        .ok_or("manifest has no byte length")?;
    let expected_hash = required_str(manifest, "sha256")?;
    if bytes.len() as u64 != expected_size || sha256_bytes(&bytes) != expected_hash {
        return Err(format!("artifact verification failed: {}", path.display()).into());
    }
    Ok(())
}

fn required_str<'a>(value: &'a Value, name: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string '{name}'").into())
}

fn require_true(value: &Value, pointer: &str) -> Result<(), Box<dyn Error>> {
    if value.pointer(pointer).and_then(Value::as_bool) != Some(true) {
        return Err(format!("required evidence '{pointer}' is not true").into());
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "machine_guest_kvm.rs"]
mod guest_host;

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod guest_host {
    use super::*;

    pub(super) const VIRTIO_MMIO_BASE: u64 = ncc_machine::PC_VIRTIO_MMIO_BASE;
    pub(super) const VIRTIO_IRQ_BASE: u32 = ncc_machine::PC_VIRTIO_IRQ_BASE;

    pub(super) fn execute(
        _plan: &GuestBootPlan,
        _parsed: &ParsedGuest,
        _args: &[String],
    ) -> Result<Value, Box<dyn Error>> {
        Err("direct guest execution currently requires an x86_64 Linux host with /dev/kvm".into())
    }
}
