use hmac::{Hmac, Mac};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const BOARD_MANIFEST_SCHEMA: &str = "ncc.machine.physical_board_manifest.v1";
const BOARD_CERTIFICATE_SCHEMA: &str = "ncc.machine.physical_board_certificate.v1";
const BOARD_BUNDLE_SCHEMA: &str = "ncc.machine.physical_board_bundle.v1";
const BOARD_SESSION_SCHEMA: &str = "ncc.machine.physical_board_session.v1";
const BOARD_VERIFICATION_SCHEMA: &str = "ncc.machine.physical_board_certificate_verification.v1";
const BOARD_FORMAT_VERSION: u64 = 1;
const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
const DEFAULT_MAX_TIMEOUT_MS: u64 = 10 * 60 * 1000;
const HARD_MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const HARD_MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
const HARD_MAX_TIMEOUT_MS: u64 = 60 * 60 * 1000;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const HMAC_DOMAIN: &[u8] = b"ncc.machine.physical_board_certificate.v1\0";
const EMULATOR_TOKENS: &[&str] = &[
    "qemu",
    "tcg",
    "emulator",
    "emulated",
    "simulator",
    "bochs",
    "renode",
    "gem5",
    "unicorn",
    "riscv-spike",
];

#[derive(Debug, Clone, Copy)]
struct BoardLimits {
    max_artifact_bytes: u64,
    max_log_bytes: u64,
    max_timeout_ms: u64,
}

#[derive(Debug, Clone)]
struct BoardFramebufferContract {
    width: u64,
    height: u64,
    stride: u64,
    pixel_format: String,
    expected_bytes: u64,
}

#[derive(Debug)]
struct CapturedLog {
    observed_bytes: u64,
    retained_bytes: u64,
    sha256: String,
    truncated: bool,
}

#[derive(Debug)]
struct BoardCommandOutcome {
    report: Value,
    transcript: String,
    stdout: String,
    success: bool,
    emulator_detected: bool,
}

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn first_non_flag(args: &[String]) -> Option<&str> {
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--manifest" | "--bundle" | "--out" | "--target" | "--signing-key-env"
        ) {
            skip = true;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg);
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn epoch_seconds() -> u64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or(0)
        })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let mut ordered = object.keys().collect::<Vec<_>>();
            ordered.sort();
            let mut result = Map::new();
            for key in ordered {
                result.insert(key.clone(), canonical_json(&object[key]));
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&canonical_json(value))?)
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut encoded = serde_json::to_vec_pretty(&canonical_json(value))?;
    encoded.push(b'\n');
    fs::write(path, encoded)?;
    Ok(())
}

fn required_str<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "[E_MACHINE_BOARD_MANIFEST_001] required non-empty string '{pointer}' is missing"
            )
            .into()
        })
}

fn optional_str<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
}

fn reject_template_placeholder(value: &str, label: &str) -> Result<(), Box<dyn Error>> {
    if value.to_ascii_uppercase().contains("EDIT_ME") {
        return Err(format!(
            "[E_MACHINE_BOARD_MANIFEST_001] '{label}' still contains an EDIT_ME placeholder"
        )
        .into());
    }
    Ok(())
}

fn emulator_token(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    EMULATOR_TOKENS
        .iter()
        .copied()
        .find(|token| lower.contains(token))
}

fn reject_emulator_value(value: &str, label: &str) -> Result<(), Box<dyn Error>> {
    if let Some(token) = emulator_token(value) {
        return Err(format!(
            "[E_MACHINE_BOARD_EMULATOR_001] physical certification rejects emulator token '{token}' in {label}"
        )
        .into());
    }
    Ok(())
}

fn reject_implicit_shell_program(program: &str, label: &str) -> Result<(), Box<dyn Error>> {
    let extension = Path::new(program)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "bat" | "cmd") {
        return Err(format!(
            "[E_MACHINE_BOARD_COMMAND_001] {label} points to '.{extension}', which requires an implicit Windows command shell; invoke an explicit executable argv instead"
        )
        .into());
    }
    Ok(())
}

fn json_string_array(value: Option<&Value>, label: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let rows = value.as_array().ok_or_else(|| {
        format!("[E_MACHINE_BOARD_MANIFEST_001] '{label}' must be an array of strings")
    })?;
    rows.iter()
        .map(|row| {
            row.as_str()
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    format!(
                        "[E_MACHINE_BOARD_MANIFEST_001] '{label}' contains an empty or non-string value"
                    )
                    .into()
                })
        })
        .collect()
}

fn resolve_relative(base: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn board_arch_for_target(target: &str) -> Result<&'static str, Box<dyn Error>> {
    match target {
        "x86_64-unknown-none" | "x86_64-unknown-none-elf" => Ok("x86_64"),
        "i386-unknown-none" | "i386-unknown-none-elf" => Ok("i386"),
        "aarch64-unknown-none" | "aarch64-unknown-none-elf" => Ok("aarch64"),
        "riscv64-unknown-elf" | "riscv64gc-unknown-none-elf" => Ok("riscv64"),
        "riscv32-unknown-elf" | "riscv32imac-unknown-elf" => Ok("riscv32"),
        _ => Err(format!(
            "[E_MACHINE_BOARD_TARGET_001] target '{target}' is not a declared freestanding physical-board target"
        )
        .into()),
    }
}

fn board_template(target: &str) -> Result<Value, Box<dyn Error>> {
    let arch = board_arch_for_target(target)?;
    Ok(json!({
        "schema": BOARD_MANIFEST_SCHEMA,
        "manifest_version": BOARD_FORMAT_VERSION,
        "board": {
            "id": format!("{arch}-lab-board-01"),
            "vendor": "EDIT_ME",
            "model": "EDIT_ME",
            "hardware_revision": "EDIT_ME",
            "serial": "EDIT_ME_UNIQUE_BOARD_SERIAL",
            "architecture": arch,
            "target": target,
        },
        "artifact": {
            "path": "EDIT_ME_BOOT_IMAGE_OR_ELF",
            "sha256": "auto",
        },
        "transport": {
            "kind": "uart+jtag",
            "physical": true,
            "port": "/dev/ttyUSB0",
            "baud": 115200,
            "probe_serial": "EDIT_ME_UNIQUE_DEBUG_PROBE_SERIAL",
        },
        "limits": {
            "max_artifact_bytes": DEFAULT_MAX_ARTIFACT_BYTES,
            "max_log_bytes": DEFAULT_MAX_LOG_BYTES,
            "max_timeout_ms": DEFAULT_MAX_TIMEOUT_MS,
        },
        "commands": {
            "probe": {
                "argv": ["openocd", "-f", "EDIT_ME_BOARD_CONFIG.cfg", "-c", "adapter serial {probe_serial}; init; shutdown"],
                "timeout_ms": 30000,
                "success_exit_codes": [0],
            },
            "flash": {
                "argv": ["openocd", "-f", "EDIT_ME_BOARD_CONFIG.cfg", "-c", "program {artifact} verify reset exit"],
                "timeout_ms": 180000,
                "success_exit_codes": [0],
            },
            "reset": {
                "argv": ["openocd", "-f", "EDIT_ME_BOARD_CONFIG.cfg", "-c", "reset run; shutdown"],
                "timeout_ms": 30000,
                "success_exit_codes": [0],
            },
            "capture": {
                "argv": ["picocom", "--baud", "{baud}", "--quiet", "{transport_port}"],
                "timeout_ms": 30000,
                "allow_timeout": true,
                "success_exit_codes": [0],
            },
            "framebuffer": {
                "argv": ["EDIT_ME_FRAMEBUFFER_CAPTURE_TOOL", "--output", "{framebuffer_path}"],
                "timeout_ms": 30000,
                "success_exit_codes": [0],
            },
        },
        "framebuffer": {
            "width": 1024,
            "height": 768,
            "stride": 4096,
            "pixel_format": "xrgb8888",
        },
        "identity": {
            "probe_markers": ["{probe_serial}"],
            "required_markers": ["NCC_BOOT_START", "NCC_BOOT_OK"],
            "ordered_markers": ["NCC_BOOT_START", "NCC_BOOT_OK"],
            "forbidden_markers": ["QEMU", "TCG", "EMULATOR", "SIMULATOR", "RENODE", "GEM5"],
        },
        "policy": {
            "require_clean_git": false,
            "require_framebuffer_capture": true,
            "reject_emulated_transport": true,
            "retain_full_command_logs_within_limits": true,
        }
    }))
}

pub(crate) fn run_board_template(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("aarch64-unknown-none");
    let payload = board_template(target)?;
    if let Some(out) = option_value(args, "--out") {
        write_pretty_json(Path::new(out), &payload)?;
    }
    if has_flag(args, "--json") || option_value(args, "--out").is_none() {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&payload))?
        );
    } else {
        println!(
            "OK: physical board manifest template written to {}",
            option_value(args, "--out").unwrap_or_default()
        );
    }
    Ok(())
}

fn limits_from_manifest(manifest: &Value) -> Result<BoardLimits, Box<dyn Error>> {
    let limits = BoardLimits {
        max_artifact_bytes: manifest
            .pointer("/limits/max_artifact_bytes")
            .and_then(Value::as_u64)
            .ok_or(
                "[E_MACHINE_BOARD_LIMIT_001] limits.max_artifact_bytes is required and must be an unsigned integer",
            )?,
        max_log_bytes: manifest
            .pointer("/limits/max_log_bytes")
            .and_then(Value::as_u64)
            .ok_or(
                "[E_MACHINE_BOARD_LIMIT_001] limits.max_log_bytes is required and must be an unsigned integer",
            )?,
        max_timeout_ms: manifest
            .pointer("/limits/max_timeout_ms")
            .and_then(Value::as_u64)
            .ok_or(
                "[E_MACHINE_BOARD_LIMIT_001] limits.max_timeout_ms is required and must be an unsigned integer",
            )?,
    };
    if limits.max_artifact_bytes == 0 || limits.max_artifact_bytes > HARD_MAX_ARTIFACT_BYTES {
        return Err(format!(
            "[E_MACHINE_BOARD_LIMIT_001] limits.max_artifact_bytes must be in 1..={HARD_MAX_ARTIFACT_BYTES}"
        )
        .into());
    }
    if limits.max_log_bytes == 0 || limits.max_log_bytes > HARD_MAX_LOG_BYTES {
        return Err(format!(
            "[E_MACHINE_BOARD_LIMIT_001] limits.max_log_bytes must be in 1..={HARD_MAX_LOG_BYTES}"
        )
        .into());
    }
    if limits.max_timeout_ms == 0 || limits.max_timeout_ms > HARD_MAX_TIMEOUT_MS {
        return Err(format!(
            "[E_MACHINE_BOARD_LIMIT_001] limits.max_timeout_ms must be in 1..={HARD_MAX_TIMEOUT_MS}"
        )
        .into());
    }
    Ok(limits)
}

fn framebuffer_contract(
    manifest: &Value,
    limits: BoardLimits,
) -> Result<Option<BoardFramebufferContract>, Box<dyn Error>> {
    let required = manifest
        .pointer("/policy/require_framebuffer_capture")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !required {
        return Ok(None);
    }
    let width = manifest
        .pointer("/framebuffer/width")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer.width is required")?;
    let height = manifest
        .pointer("/framebuffer/height")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer.height is required")?;
    let stride = manifest
        .pointer("/framebuffer/stride")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer.stride is required")?;
    let pixel_format = required_str(manifest, "/framebuffer/pixel_format")?.to_ascii_lowercase();
    let bytes_per_pixel = match pixel_format.as_str() {
        "rgb565" | "bgr565" => 2,
        "rgb888" | "bgr888" => 3,
        "xrgb8888" | "argb8888" | "bgra8888" | "rgba8888" => 4,
        _ => {
            return Err(format!(
                "[E_MACHINE_BOARD_FRAMEBUFFER_001] unsupported framebuffer.pixel_format '{pixel_format}'"
            )
            .into())
        }
    };
    let minimum_stride = width
        .checked_mul(bytes_per_pixel)
        .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer row byte size overflows u64")?;
    let expected_bytes = stride
        .checked_mul(height)
        .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer byte size overflows u64")?;
    if width == 0 || height == 0 || stride < minimum_stride || expected_bytes == 0 {
        return Err(format!(
            "[E_MACHINE_BOARD_FRAMEBUFFER_001] invalid framebuffer geometry width={width} height={height} stride={stride} minimum_stride={minimum_stride}"
        )
        .into());
    }
    if expected_bytes > limits.max_artifact_bytes {
        return Err(format!(
            "[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer requires {expected_bytes} bytes, above limits.max_artifact_bytes={}",
            limits.max_artifact_bytes
        )
        .into());
    }
    Ok(Some(BoardFramebufferContract {
        width,
        height,
        stride,
        pixel_format,
        expected_bytes,
    }))
}

fn expand_tokens(raw: &str, values: &BTreeMap<String, String>) -> String {
    values.iter().fold(raw.to_string(), |text, (key, value)| {
        text.replace(&format!("{{{key}}}"), value)
    })
}

fn expanded_string(
    raw: &str,
    values: &BTreeMap<String, String>,
    label: &str,
) -> Result<String, Box<dyn Error>> {
    let expanded = expand_tokens(raw, values);
    if expanded.contains('{') || expanded.contains('}') {
        return Err(format!(
            "[E_MACHINE_BOARD_COMMAND_001] unresolved placeholder in {label}: '{expanded}'"
        )
        .into());
    }
    if expanded.contains('\0') {
        return Err(format!("[E_MACHINE_BOARD_COMMAND_001] NUL byte in {label}").into());
    }
    reject_emulator_value(&expanded, label)?;
    Ok(expanded)
}

fn command_argv(
    spec: &Value,
    label: &str,
    values: &BTreeMap<String, String>,
) -> Result<Vec<String>, Box<dyn Error>> {
    if spec.get("program").is_some()
        || spec.get("args").is_some()
        || spec.get("shell").is_some()
        || spec.get("command_line").is_some()
    {
        return Err(format!(
            "[E_MACHINE_BOARD_COMMAND_001] commands.{label} must use one structured argv array; program/args/shell/command_line are rejected"
        )
        .into());
    }
    let raw = json_string_array(spec.get("argv"), &format!("commands.{label}.argv"))?;
    if raw.is_empty() {
        return Err(format!(
            "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.argv must contain a program"
        )
        .into());
    }
    raw.iter()
        .enumerate()
        .map(|(index, item)| {
            expanded_string(item, values, &format!("commands.{label}.argv[{index}]"))
        })
        .collect()
}

fn command_exit_codes(spec: &Value, label: &str) -> Result<Vec<i64>, Box<dyn Error>> {
    let Some(rows) = spec.get("success_exit_codes") else {
        return Ok(vec![0]);
    };
    let parsed = rows
        .as_array()
        .ok_or_else(|| {
            format!(
                "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.success_exit_codes must be an array"
            )
        })?
        .iter()
        .map(|row| {
            row.as_i64().ok_or_else(|| {
                format!(
                    "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.success_exit_codes must contain integers"
                )
                .into()
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    if parsed.is_empty() {
        return Err(format!(
            "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.success_exit_codes cannot be empty"
        )
        .into());
    }
    Ok(parsed)
}

fn validate_command_spec(
    spec: &Value,
    label: &str,
    values: &BTreeMap<String, String>,
    limits: BoardLimits,
    signing_key_env: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let argv = command_argv(spec, label, values)?;
    reject_implicit_shell_program(&argv[0], &format!("commands.{label}.argv[0]"))?;
    command_exit_codes(spec, label)?;
    let timeout_ms = spec
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label}.timeout_ms is required")
        })?;
    if timeout_ms == 0 || timeout_ms > limits.max_timeout_ms {
        return Err(format!(
            "[E_MACHINE_BOARD_LIMIT_001] commands.{label}.timeout_ms must be in 1..={}",
            limits.max_timeout_ms
        )
        .into());
    }
    if let Some(value) = spec.get("allow_timeout") {
        let allow_timeout = value.as_bool().ok_or_else(|| {
            format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label}.allow_timeout must be boolean")
        })?;
        if allow_timeout && label != "capture" {
            return Err(format!(
                "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.allow_timeout is forbidden; only capture may complete by controlled timeout"
            )
            .into());
        }
    }
    if let Some(value) = spec.get("inherit_env") {
        if !value.is_boolean() {
            return Err(format!(
                "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.inherit_env must be boolean"
            )
            .into());
        }
    }
    if let Some(cwd) = spec.get("cwd") {
        let cwd = cwd.as_str().ok_or_else(|| {
            format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label}.cwd must be a string")
        })?;
        expanded_string(cwd, values, &format!("commands.{label}.cwd"))?;
    }
    if let Some(env) = spec.get("env") {
        let env = env.as_object().ok_or_else(|| {
            format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label}.env must be an object")
        })?;
        for (key, value) in env {
            if key.is_empty() || key.contains('=') || key.contains('\0') {
                return Err(format!(
                    "[E_MACHINE_BOARD_COMMAND_001] commands.{label}.env contains invalid key '{key}'"
                )
                .into());
            }
            if signing_key_env == Some(key.as_str()) {
                return Err(format!(
                    "[E_MACHINE_BOARD_ATTESTATION_001] commands.{label}.env cannot receive the HMAC signing key"
                )
                .into());
            }
            let value = value.as_str().ok_or_else(|| {
                format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label}.env values must be strings")
            })?;
            expanded_string(value, values, &format!("commands.{label}.env.{key}"))?;
        }
    }
    Ok(())
}

fn validate_manifest(
    manifest: &Value,
    signing_key_env: Option<&str>,
) -> Result<BoardLimits, Box<dyn Error>> {
    if manifest.get("schema").and_then(Value::as_str) != Some(BOARD_MANIFEST_SCHEMA)
        || manifest.get("manifest_version").and_then(Value::as_u64) != Some(BOARD_FORMAT_VERSION)
    {
        return Err(format!(
            "[E_MACHINE_BOARD_MANIFEST_001] expected schema '{BOARD_MANIFEST_SCHEMA}' and manifest_version {BOARD_FORMAT_VERSION}"
        )
        .into());
    }
    let identity_fields = [
        ("/board/id", "board.id"),
        ("/board/vendor", "board.vendor"),
        ("/board/model", "board.model"),
        ("/board/hardware_revision", "board.hardware_revision"),
        ("/board/serial", "board.serial"),
        ("/board/architecture", "board.architecture"),
        ("/board/target", "board.target"),
        ("/transport/kind", "transport.kind"),
        ("/transport/port", "transport.port"),
        ("/transport/probe_serial", "transport.probe_serial"),
        ("/artifact/path", "artifact.path"),
    ];
    for (pointer, label) in identity_fields {
        let value = required_str(manifest, pointer)?;
        reject_template_placeholder(value, label)?;
        reject_emulator_value(value, label)?;
    }
    let artifact_hash = required_str(manifest, "/artifact/sha256")?;
    if artifact_hash != "auto"
        && (artifact_hash.len() != 64
            || !artifact_hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(
            "[E_MACHINE_BOARD_ARTIFACT_001] artifact.sha256 must be 'auto' or a 64-digit SHA-256 value"
                .into(),
        );
    }
    if manifest
        .pointer("/transport/physical")
        .and_then(Value::as_bool)
        != Some(true)
        || manifest
            .pointer("/policy/reject_emulated_transport")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(
            "[E_MACHINE_BOARD_PHYSICAL_REQUIRED_001] transport.physical and policy.reject_emulated_transport must both be true"
                .into(),
        );
    }
    let target = required_str(manifest, "/board/target")?;
    let expected_arch = board_arch_for_target(target)?;
    let arch = required_str(manifest, "/board/architecture")?;
    if arch != expected_arch {
        return Err(format!(
            "[E_MACHINE_BOARD_TARGET_001] board architecture '{arch}' does not match target '{target}' ({expected_arch})"
        )
        .into());
    }
    let baud = manifest
        .pointer("/transport/baud")
        .and_then(Value::as_u64)
        .ok_or(
            "[E_MACHINE_BOARD_MANIFEST_001] transport.baud is required and must be an unsigned integer",
        )?;
    if baud == 0 || baud > 100_000_000 {
        return Err(
            "[E_MACHINE_BOARD_MANIFEST_001] transport.baud must be in 1..=100000000".into(),
        );
    }
    if let Some(require_clean) = manifest.pointer("/policy/require_clean_git") {
        if !require_clean.is_boolean() {
            return Err(
                "[E_MACHINE_BOARD_MANIFEST_001] policy.require_clean_git must be boolean".into(),
            );
        }
    }
    if let Some(require_framebuffer) = manifest.pointer("/policy/require_framebuffer_capture") {
        if !require_framebuffer.is_boolean() {
            return Err(
                "[E_MACHINE_BOARD_FRAMEBUFFER_001] policy.require_framebuffer_capture must be boolean"
                    .into(),
            );
        }
    }
    if manifest
        .pointer("/policy/retain_full_command_logs_within_limits")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "[E_MACHINE_BOARD_MANIFEST_001] policy.retain_full_command_logs_within_limits must be true"
                .into(),
        );
    }
    let limits = limits_from_manifest(manifest)?;
    let framebuffer = framebuffer_contract(manifest, limits)?;
    let commands = manifest
        .get("commands")
        .and_then(Value::as_object)
        .ok_or("[E_MACHINE_BOARD_COMMAND_001] manifest requires a commands object")?;
    let mut placeholder_values = BTreeMap::new();
    for key in [
        "artifact",
        "artifact_sha256",
        "board_id",
        "board_serial",
        "target",
        "arch",
        "transport_port",
        "probe_serial",
        "baud",
        "bundle",
        "framebuffer_path",
    ] {
        placeholder_values.insert(key.to_string(), format!("NCC_{key}"));
    }
    for label in ["probe", "flash", "capture"] {
        let spec = commands
            .get(label)
            .ok_or_else(|| format!("[E_MACHINE_BOARD_COMMAND_001] commands.{label} is required"))?;
        validate_command_spec(spec, label, &placeholder_values, limits, signing_key_env)?;
    }
    if let Some(reset) = commands.get("reset") {
        validate_command_spec(reset, "reset", &placeholder_values, limits, signing_key_env)?;
    }
    if framebuffer.is_some() {
        let spec = commands.get("framebuffer").ok_or(
            "[E_MACHINE_BOARD_FRAMEBUFFER_001] commands.framebuffer is required when policy.require_framebuffer_capture is true",
        )?;
        validate_command_spec(
            spec,
            "framebuffer",
            &placeholder_values,
            limits,
            signing_key_env,
        )?;
    }
    let probe_markers = json_string_array(
        manifest.pointer("/identity/probe_markers"),
        "identity.probe_markers",
    )?;
    let ordered_markers = json_string_array(
        manifest.pointer("/identity/ordered_markers"),
        "identity.ordered_markers",
    )?;
    let required_markers = json_string_array(
        manifest.pointer("/identity/required_markers"),
        "identity.required_markers",
    )?;
    let forbidden_markers = json_string_array(
        manifest.pointer("/identity/forbidden_markers"),
        "identity.forbidden_markers",
    )?;
    if probe_markers.is_empty()
        || required_markers.is_empty()
        || ordered_markers.is_empty()
        || forbidden_markers.is_empty()
    {
        return Err(
            "[E_MACHINE_BOARD_MANIFEST_001] probe, required, ordered and forbidden marker arrays must be non-empty"
                .into(),
        );
    }
    for (label, markers) in [
        ("identity.probe_markers", probe_markers),
        ("identity.required_markers", required_markers),
        ("identity.ordered_markers", ordered_markers),
    ] {
        for marker in markers {
            expanded_string(&marker, &placeholder_values, label)?;
        }
    }
    Ok(limits)
}

fn resolve_program(program: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(program);
    if direct.is_file() {
        return fs::canonicalize(direct).ok();
    }
    let path = std::env::var_os("PATH")?;
    #[cfg(windows)]
    let extensions = std::env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .filter(|item| !item.is_empty())
                .filter(|item| !matches!(item.to_ascii_lowercase().as_str(), ".bat" | ".cmd"))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![".EXE".into(), ".COM".into()]);
    #[cfg(not(windows))]
    let extensions = vec![String::new()];
    for directory in std::env::split_paths(&path) {
        if Path::new(program).extension().is_some() {
            let candidate = directory.join(program);
            if candidate.is_file() {
                return fs::canonicalize(candidate).ok();
            }
        }
        for extension in &extensions {
            let candidate = directory.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return fs::canonicalize(candidate).ok();
            }
        }
    }
    None
}

fn spawn_log_reader<R: Read + Send + 'static>(
    mut reader: R,
    path: PathBuf,
    limit: u64,
    limit_hit: Arc<AtomicBool>,
) -> thread::JoinHandle<Result<CapturedLog, String>> {
    thread::spawn(move || {
        let mut output = File::create(&path).map_err(|error| error.to_string())?;
        let mut digest = Sha256::new();
        let mut observed = 0_u64;
        let mut retained = 0_u64;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
            observed = observed.saturating_add(read as u64);
            let remaining = limit.saturating_sub(retained) as usize;
            let keep = remaining.min(read);
            if keep > 0 {
                output
                    .write_all(&buffer[..keep])
                    .map_err(|error| error.to_string())?;
                retained += keep as u64;
            }
            if observed > limit {
                limit_hit.store(true, Ordering::Release);
            }
        }
        output.flush().map_err(|error| error.to_string())?;
        Ok(CapturedLog {
            observed_bytes: observed,
            retained_bytes: retained,
            sha256: digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            truncated: observed > retained,
        })
    })
}

fn captured_file(path: &Path) -> Result<CapturedLog, Box<dyn Error>> {
    Ok(CapturedLog {
        observed_bytes: fs::metadata(path)?.len(),
        retained_bytes: fs::metadata(path)?.len(),
        sha256: sha256_file(path)?,
        truncated: false,
    })
}

fn transcript_preview(text: &str) -> String {
    text.chars().take(4096).collect()
}

fn command_log_json(path: &str, capture: &CapturedLog, preview: &str) -> Value {
    json!({
        "path": path,
        "observed_bytes": capture.observed_bytes,
        "retained_bytes": capture.retained_bytes,
        "sha256": capture.sha256,
        "truncated": capture.truncated,
        "preview": transcript_preview(preview),
    })
}

fn run_board_command(
    label: &str,
    spec: &Value,
    values: &BTreeMap<String, String>,
    manifest_dir: &Path,
    bundle_dir: &Path,
    limits: BoardLimits,
    signing_key_env: Option<&str>,
) -> Result<BoardCommandOutcome, Box<dyn Error>> {
    validate_command_spec(spec, label, values, limits, signing_key_env)?;
    let argv = command_argv(spec, label, values)?;
    let timeout_ms = spec
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_BOARD_COMMAND_001] timeout_ms disappeared after validation")?;
    let allow_timeout = spec
        .get("allow_timeout")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let accepted_exit_codes = command_exit_codes(spec, label)?;
    let stdout_relative = format!("logs/{label}.stdout.log");
    let stderr_relative = format!("logs/{label}.stderr.log");
    let stdout_path = bundle_dir.join(&stdout_relative);
    let stderr_path = bundle_dir.join(&stderr_relative);
    fs::create_dir_all(
        stdout_path
            .parent()
            .ok_or("[E_MACHINE_BOARD_BUNDLE_001] log path has no parent")?,
    )?;
    let resolved_program = resolve_program(&argv[0]);
    if let Some(path) = resolved_program.as_ref() {
        reject_emulator_value(&path.display().to_string(), "resolved command program")?;
    }
    let executable_sha256_before = resolved_program
        .as_ref()
        .and_then(|path| sha256_file(path).ok());
    let mut command = Command::new(
        resolved_program
            .as_deref()
            .unwrap_or_else(|| Path::new(&argv[0])),
    );
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let inherit_env = spec
        .get("inherit_env")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !inherit_env {
        command.env_clear();
    }
    let mut explicit_env_keys = Vec::new();
    if let Some(env) = spec.get("env").and_then(Value::as_object) {
        for (key, value) in env {
            explicit_env_keys.push(key.clone());
            command.env(
                key,
                expanded_string(
                    value.as_str().unwrap_or_default(),
                    values,
                    &format!("commands.{label}.env.{key}"),
                )?,
            );
        }
    }
    explicit_env_keys.sort();
    if let Some(secret_env) = signing_key_env {
        command.env_remove(secret_env);
    }
    if let Some(cwd) = spec.get("cwd").and_then(Value::as_str) {
        let cwd = expanded_string(cwd, values, &format!("commands.{label}.cwd"))?;
        command.current_dir(resolve_relative(manifest_dir, &cwd));
    }
    let started = Instant::now();
    let spawn = command.spawn();
    let mut spawn_error = None::<String>;
    let mut exit_code = None::<i64>;
    let mut timed_out = false;
    let mut log_limit_exceeded = false;
    let (stdout_capture, stderr_capture) = match spawn {
        Ok(mut child) => {
            let limit_hit = Arc::new(AtomicBool::new(false));
            let stdout_handle = spawn_log_reader(
                child
                    .stdout
                    .take()
                    .ok_or("[E_MACHINE_BOARD_COMMAND_001] child stdout pipe is missing")?,
                stdout_path.clone(),
                limits.max_log_bytes,
                Arc::clone(&limit_hit),
            );
            let stderr_handle = spawn_log_reader(
                child
                    .stderr
                    .take()
                    .ok_or("[E_MACHINE_BOARD_COMMAND_001] child stderr pipe is missing")?,
                stderr_path.clone(),
                limits.max_log_bytes,
                Arc::clone(&limit_hit),
            );
            loop {
                if let Some(status) = child.try_wait()? {
                    exit_code = status.code().map(i64::from);
                    break;
                }
                if limit_hit.load(Ordering::Acquire) {
                    log_limit_exceeded = true;
                    let _ = child.kill();
                    let status = child.wait()?;
                    exit_code = status.code().map(i64::from);
                    break;
                }
                if started.elapsed() >= Duration::from_millis(timeout_ms) {
                    timed_out = true;
                    let _ = child.kill();
                    let status = child.wait()?;
                    exit_code = status.code().map(i64::from);
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            let stdout_capture = stdout_handle
                .join()
                .map_err(|_| "[E_MACHINE_BOARD_TRANSCRIPT_001] stdout reader panicked")?
                .map_err(|error| {
                    format!("[E_MACHINE_BOARD_TRANSCRIPT_001] stdout reader failed: {error}")
                })?;
            let stderr_capture = stderr_handle
                .join()
                .map_err(|_| "[E_MACHINE_BOARD_TRANSCRIPT_001] stderr reader panicked")?
                .map_err(|error| {
                    format!("[E_MACHINE_BOARD_TRANSCRIPT_001] stderr reader failed: {error}")
                })?;
            log_limit_exceeded |= stdout_capture.truncated || stderr_capture.truncated;
            (stdout_capture, stderr_capture)
        }
        Err(error) => {
            spawn_error = Some(error.to_string());
            fs::write(&stdout_path, b"")?;
            fs::write(&stderr_path, error.to_string().as_bytes())?;
            (captured_file(&stdout_path)?, captured_file(&stderr_path)?)
        }
    };
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_else(|_| {
        String::from_utf8_lossy(&fs::read(&stdout_path).unwrap_or_default()).into_owned()
    });
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_else(|_| {
        String::from_utf8_lossy(&fs::read(&stderr_path).unwrap_or_default()).into_owned()
    });
    let transcript = format!("{stdout}\n{stderr}");
    let emulator_detected = emulator_token(&transcript).is_some();
    let executable_sha256_after = resolved_program
        .as_ref()
        .and_then(|path| sha256_file(path).ok());
    let executable_identity_verified = resolved_program.is_some()
        && executable_sha256_before.is_some()
        && executable_sha256_before == executable_sha256_after;
    let executable_changed_during_execution = executable_sha256_before != executable_sha256_after;
    let success = spawn_error.is_none()
        && !log_limit_exceeded
        && !emulator_detected
        && executable_identity_verified
        && !executable_changed_during_execution
        && ((!timed_out && exit_code.is_some_and(|code| accepted_exit_codes.contains(&code)))
            || (timed_out && allow_timeout));
    let argv_sha256 = sha256_bytes(&canonical_json_bytes(&json!(argv.clone()))?);
    Ok(BoardCommandOutcome {
        report: json!({
            "label": label,
            "argv": argv,
            "argv_sha256": argv_sha256,
            "shell_implicit": false,
            "resolved_program": resolved_program.map(|path| path.display().to_string()),
            "resolved_program_sha256_before": executable_sha256_before,
            "resolved_program_sha256_after": executable_sha256_after,
            "resolved_program_identity_verified": executable_identity_verified,
            "resolved_program_changed_during_execution": executable_changed_during_execution,
            "inherit_env": inherit_env,
            "explicit_env_keys": explicit_env_keys,
            "hmac_secret_removed_from_child_environment": signing_key_env.is_some(),
            "timeout_ms": timeout_ms,
            "allow_timeout": allow_timeout,
            "accepted_exit_codes": accepted_exit_codes,
            "exit_code": exit_code,
            "timed_out": timed_out,
            "log_limit_bytes_per_stream": limits.max_log_bytes,
            "log_limit_exceeded": log_limit_exceeded,
            "spawn_error": spawn_error,
            "emulator_marker_observed": emulator_detected,
            "success": success,
            "duration_ms": started.elapsed().as_millis(),
            "stdout": command_log_json(&stdout_relative, &stdout_capture, &stdout),
            "stderr": command_log_json(&stderr_relative, &stderr_capture, &stderr),
        }),
        transcript,
        stdout,
        success,
        emulator_detected,
    })
}

fn ordered_markers_present(transcript: &str, markers: &[String]) -> bool {
    let mut cursor = 0;
    for marker in markers {
        let Some(found) = transcript[cursor..].find(marker) else {
            return false;
        };
        cursor += found + marker.len();
    }
    true
}

fn command_output_text(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn prepare_bundle_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        if !path.is_dir() {
            return Err(format!(
                "[E_MACHINE_BOARD_BUNDLE_001] bundle path '{}' is not a directory",
                path.display()
            )
            .into());
        }
        if fs::read_dir(path)?.next().is_some() {
            return Err(format!(
                "[E_MACHINE_BOARD_BUNDLE_001] bundle directory '{}' must be new or empty",
                path.display()
            )
            .into());
        }
    }
    fs::create_dir_all(path.join("inputs"))?;
    fs::create_dir_all(path.join("logs"))?;
    fs::create_dir_all(path.join("metadata"))?;
    Ok(())
}

fn normalize_relative(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn safe_bundle_relative(raw: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = PathBuf::from(raw);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(
            format!("[E_MACHINE_BOARD_BUNDLE_001] unsafe bundle-relative path '{raw}'").into(),
        );
    }
    if path.components().count() == 0 {
        return Err("[E_MACHINE_BOARD_BUNDLE_001] empty bundle-relative path".into());
    }
    Ok(path)
}

fn collect_bundle_files(
    root: &Path,
    current: &Path,
    excluded: &BTreeSet<String>,
    out: &mut Vec<Value>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "[E_MACHINE_BOARD_BUNDLE_001] symlinks are forbidden inside bundles: '{}'",
                path.display()
            )
            .into());
        }
        if metadata.is_dir() {
            collect_bundle_files(root, &path, excluded, out)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(format!(
                "[E_MACHINE_BOARD_BUNDLE_001] non-regular bundle entry '{}'",
                path.display()
            )
            .into());
        }
        let relative = normalize_relative(path.strip_prefix(root)?);
        if excluded.contains(&relative) {
            continue;
        }
        let role = if relative == "certificate.json" {
            "certificate"
        } else if relative.starts_with("inputs/") {
            "input"
        } else if relative.starts_with("logs/") {
            "evidence_log"
        } else if relative.starts_with("metadata/") {
            "metadata"
        } else {
            "evidence"
        };
        out.push(json!({
            "path": relative,
            "role": role,
            "bytes": metadata.len(),
            "sha256": sha256_file(&path)?,
        }));
    }
    out.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    Ok(())
}

fn bundle_files(root: &Path, excluded: &[&str]) -> Result<Vec<Value>, Box<dyn Error>> {
    let excluded = excluded
        .iter()
        .map(|item| item.to_string())
        .collect::<BTreeSet<_>>();
    let mut files = Vec::new();
    collect_bundle_files(root, root, &excluded, &mut files)?;
    Ok(files)
}

fn certificate_payload_bytes(certificate: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut unsigned = certificate.clone();
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("integrity");
        object.remove("attestation");
    }
    canonical_json_bytes(&unsigned)
}

fn hmac_tag_from_env(env_name: &str, payload: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut key = std::env::var(env_name)
        .map_err(|_| {
            format!(
                "[E_MACHINE_BOARD_ATTESTATION_001] HMAC key environment variable '{env_name}' is not set"
            )
        })?
        .into_bytes();
    if key.len() < 32 {
        key.fill(0);
        return Err(
            "[E_MACHINE_BOARD_ATTESTATION_001] HMAC keys must contain at least 32 bytes".into(),
        );
    }
    let mut mac = HmacSha256::new_from_slice(&key)?;
    mac.update(HMAC_DOMAIN);
    mac.update(payload);
    let tag = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    key.fill(0);
    Ok(tag)
}

fn verify_hmac_from_env(
    env_name: &str,
    payload: &[u8],
    expected_tag: &[u8],
) -> Result<bool, Box<dyn Error>> {
    let mut key = std::env::var(env_name)
        .map_err(|_| {
            format!(
                "[E_MACHINE_BOARD_ATTESTATION_001] HMAC key environment variable '{env_name}' is not set"
            )
        })?
        .into_bytes();
    if key.len() < 32 {
        key.fill(0);
        return Err(
            "[E_MACHINE_BOARD_ATTESTATION_001] HMAC keys must contain at least 32 bytes".into(),
        );
    }
    let mut mac = HmacSha256::new_from_slice(&key)?;
    mac.update(HMAC_DOMAIN);
    mac.update(payload);
    let verified = mac.verify_slice(expected_tag).is_ok();
    key.fill(0);
    Ok(verified)
}

fn seal_certificate(
    certificate: &mut Value,
    signing_key_env: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let payload = certificate_payload_bytes(certificate)?;
    certificate["integrity"] = json!({
        "algorithm": "sha256",
        "payload_sha256": sha256_bytes(&payload),
        "canonicalization": "recursive-key-sort;compact-json;exclude-integrity-and-attestation",
    });
    if let Some(env_name) = signing_key_env {
        certificate["attestation"] = json!({
            "algorithm": "hmac-sha256",
            "domain": "ncc.machine.physical_board_certificate.v1",
            "key_id": env_name,
            "tag": hmac_tag_from_env(env_name, &payload)?,
            "secret_persisted": false,
        });
    }
    Ok(())
}

fn bundle_id(files: &[Value], certificate_sha256: &str) -> Result<String, Box<dyn Error>> {
    Ok(sha256_bytes(&canonical_json_bytes(&json!({
        "schema": BOARD_BUNDLE_SCHEMA,
        "bundle_version": BOARD_FORMAT_VERSION,
        "certificate_sha256": certificate_sha256,
        "files": files,
    }))?))
}

fn print_result(payload: &Value, json_output: bool, summary: &str) -> Result<(), Box<dyn Error>> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(payload))?
        );
    } else {
        println!("{summary}");
    }
    Ok(())
}

pub(crate) fn run_certify_board(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest_raw = option_value(args, "--manifest")
        .or_else(|| first_non_flag(args))
        .ok_or("machine certify-board expects --manifest <board.json>")?;
    let manifest_path = PathBuf::from(manifest_raw);
    let manifest_metadata = fs::metadata(&manifest_path)?;
    if !manifest_metadata.is_file()
        || manifest_metadata.len() == 0
        || manifest_metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(format!(
            "[E_MACHINE_BOARD_MANIFEST_001] manifest '{}' must be a non-empty regular file no larger than {MAX_MANIFEST_BYTES} bytes",
            manifest_path.display()
        )
        .into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let signing_key_env = option_value(args, "--signing-key-env");
    let limits = validate_manifest(&manifest, signing_key_env)?;
    let framebuffer = framebuffer_contract(&manifest, limits)?;
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let artifact_path = resolve_relative(manifest_dir, required_str(&manifest, "/artifact/path")?);
    let artifact_metadata = fs::metadata(&artifact_path).map_err(|error| {
        format!(
            "[E_MACHINE_BOARD_ARTIFACT_001] cannot read artifact '{}': {error}",
            artifact_path.display()
        )
    })?;
    if !artifact_metadata.is_file()
        || artifact_metadata.len() == 0
        || artifact_metadata.len() > limits.max_artifact_bytes
    {
        return Err(format!(
            "[E_MACHINE_BOARD_ARTIFACT_001] artifact '{}' must be non-empty and no larger than {} bytes",
            artifact_path.display(),
            limits.max_artifact_bytes
        )
        .into());
    }
    let artifact_sha256 = sha256_file(&artifact_path)?;
    if let Some(expected_hash) = optional_str(&manifest, "/artifact/sha256") {
        if expected_hash != "auto" && !expected_hash.eq_ignore_ascii_case(&artifact_sha256) {
            return Err(format!(
                "[E_MACHINE_BOARD_ARTIFACT_001] artifact hash mismatch: expected {expected_hash}, got {artifact_sha256}"
            )
            .into());
        }
    }
    let bundle_dir = PathBuf::from(
        option_value(args, "--bundle")
            .or_else(|| option_value(args, "--out"))
            .unwrap_or("target/physical-board-certification"),
    );
    prepare_bundle_dir(&bundle_dir)?;
    let framebuffer_path = bundle_dir.join("evidence/framebuffer.raw");
    if framebuffer.is_some() {
        fs::create_dir_all(
            framebuffer_path
                .parent()
                .ok_or("[E_MACHINE_BOARD_FRAMEBUFFER_001] framebuffer path has no parent")?,
        )?;
    }
    fs::write(
        bundle_dir.join("inputs/board-manifest.json"),
        &manifest_bytes,
    )?;
    fs::copy(&artifact_path, bundle_dir.join("inputs/artifact.bin"))?;
    if sha256_file(&bundle_dir.join("inputs/artifact.bin"))? != artifact_sha256 {
        return Err("[E_MACHINE_BOARD_ARTIFACT_001] bundled artifact copy hash mismatch".into());
    }

    let board_id = required_str(&manifest, "/board/id")?.to_string();
    let board_serial = required_str(&manifest, "/board/serial")?.to_string();
    let vendor = required_str(&manifest, "/board/vendor")?.to_string();
    let model = required_str(&manifest, "/board/model")?.to_string();
    let hardware_revision = required_str(&manifest, "/board/hardware_revision")?.to_string();
    let arch = required_str(&manifest, "/board/architecture")?.to_string();
    let target = required_str(&manifest, "/board/target")?.to_string();
    let transport_kind = required_str(&manifest, "/transport/kind")?.to_string();
    let transport_port = required_str(&manifest, "/transport/port")?.to_string();
    let probe_serial = required_str(&manifest, "/transport/probe_serial")?.to_string();
    let baud = manifest
        .pointer("/transport/baud")
        .and_then(Value::as_u64)
        .unwrap_or(115_200);
    let mut values = BTreeMap::new();
    values.insert("artifact".into(), artifact_path.display().to_string());
    values.insert("artifact_sha256".into(), artifact_sha256.clone());
    values.insert("board_id".into(), board_id.clone());
    values.insert("board_serial".into(), board_serial.clone());
    values.insert("target".into(), target.clone());
    values.insert("arch".into(), arch.clone());
    values.insert("transport_port".into(), transport_port.clone());
    values.insert("probe_serial".into(), probe_serial.clone());
    values.insert("baud".into(), baud.to_string());
    values.insert("bundle".into(), bundle_dir.display().to_string());
    values.insert(
        "framebuffer_path".into(),
        framebuffer_path.display().to_string(),
    );
    let commands = manifest
        .get("commands")
        .and_then(Value::as_object)
        .ok_or("[E_MACHINE_BOARD_COMMAND_001] commands disappeared after validation")?;
    let validate_only = has_flag(args, "--validate-only") || has_flag(args, "--dry-run");
    let require_clean = has_flag(args, "--require-clean")
        || manifest
            .pointer("/policy/require_clean_git")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let git_head = command_output_text("git", &["rev-parse", "HEAD"]);
    let git_status = command_output_text("git", &["status", "--porcelain"]).unwrap_or_default();
    let git_dirty = !git_status.trim().is_empty();
    let clean_ok = !require_clean || !git_dirty;

    let probe_markers = json_string_array(
        manifest.pointer("/identity/probe_markers"),
        "identity.probe_markers",
    )?
    .iter()
    .map(|marker| expanded_string(marker, &values, "identity.probe_markers"))
    .collect::<Result<Vec<_>, _>>()?;
    let mut required_markers = vec![
        "NCC_PHYSICAL_BOARD=1".to_string(),
        format!("NCC_BOARD_ID={board_id}"),
        format!("NCC_BOARD_SERIAL={board_serial}"),
        format!("NCC_TARGET={target}"),
        format!("NCC_FIRMWARE_SHA256={artifact_sha256}"),
    ];
    required_markers.extend(
        json_string_array(
            manifest.pointer("/identity/required_markers"),
            "identity.required_markers",
        )?
        .iter()
        .map(|marker| expanded_string(marker, &values, "identity.required_markers"))
        .collect::<Result<Vec<_>, _>>()?,
    );
    required_markers.sort();
    required_markers.dedup();
    let ordered_markers = json_string_array(
        manifest.pointer("/identity/ordered_markers"),
        "identity.ordered_markers",
    )?
    .iter()
    .map(|marker| expanded_string(marker, &values, "identity.ordered_markers"))
    .collect::<Result<Vec<_>, _>>()?;
    let mut forbidden_markers = EMULATOR_TOKENS
        .iter()
        .map(|token| token.to_ascii_uppercase())
        .collect::<Vec<_>>();
    forbidden_markers.extend(json_string_array(
        manifest.pointer("/identity/forbidden_markers"),
        "identity.forbidden_markers",
    )?);
    forbidden_markers.sort();
    forbidden_markers.dedup();

    let mut command_reports = Vec::new();
    let mut capture_transcript = String::new();
    let mut board_present = false;
    let mut emulator_detected = false;
    let mut execution_failed = false;
    let mut not_executed_reason = if validate_only {
        Some("validation_only_requested".to_string())
    } else {
        None
    };
    if !validate_only {
        let probe = run_board_command(
            "probe",
            &commands["probe"],
            &values,
            manifest_dir,
            &bundle_dir,
            limits,
            signing_key_env,
        )?;
        emulator_detected |= probe.emulator_detected;
        let probe_identity_ok = probe.success
            && probe.transcript.contains(&probe_serial)
            && probe_markers
                .iter()
                .all(|marker| probe.transcript.contains(marker));
        command_reports.push(probe.report);
        if emulator_detected {
            execution_failed = true;
        } else if !probe_identity_ok {
            not_executed_reason = Some("physical_board_probe_or_identity_unavailable".into());
        } else {
            board_present = true;
        }
    }
    if board_present && !execution_failed {
        let mut stages = vec!["flash", "reset", "capture"];
        if framebuffer.is_some() {
            stages.push("framebuffer");
        }
        for label in stages {
            let Some(spec) = commands.get(label) else {
                continue;
            };
            let outcome = run_board_command(
                label,
                spec,
                &values,
                manifest_dir,
                &bundle_dir,
                limits,
                signing_key_env,
            )?;
            emulator_detected |= outcome.emulator_detected;
            if label == "capture" {
                capture_transcript = outcome.stdout.clone();
            }
            execution_failed |= !outcome.success;
            command_reports.push(outcome.report);
            if execution_failed || emulator_detected {
                break;
            }
        }
    }
    let missing_markers = if capture_transcript.is_empty() {
        required_markers.clone()
    } else {
        required_markers
            .iter()
            .filter(|marker| !capture_transcript.contains(marker.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    };
    let capture_upper = capture_transcript.to_ascii_uppercase();
    let observed_forbidden_markers = forbidden_markers
        .iter()
        .filter(|marker| capture_upper.contains(&marker.to_ascii_uppercase()))
        .cloned()
        .collect::<Vec<_>>();
    emulator_detected |= !observed_forbidden_markers.is_empty();
    let ordered = !capture_transcript.is_empty()
        && ordered_markers_present(&capture_transcript, &ordered_markers);
    let framebuffer_evidence = if let Some(contract) = &framebuffer {
        let metadata = fs::symlink_metadata(&framebuffer_path).ok();
        let regular = metadata
            .as_ref()
            .is_some_and(|meta| meta.is_file() && !meta.file_type().is_symlink());
        let bytes = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
        let size_matches = regular && bytes == contract.expected_bytes;
        let sha256 = if size_matches {
            sha256_file(&framebuffer_path).ok()
        } else {
            None
        };
        json!({
            "required": true,
            "captured": size_matches && sha256.is_some(),
            "bundle_path": "evidence/framebuffer.raw",
            "width": contract.width,
            "height": contract.height,
            "stride": contract.stride,
            "pixel_format": contract.pixel_format,
            "expected_bytes": contract.expected_bytes,
            "bytes": bytes,
            "sha256": sha256,
            "regular_non_symlink_file": regular,
        })
    } else {
        json!({
            "required": false,
            "captured": false,
        })
    };
    let framebuffer_ok = framebuffer_evidence["required"] != Value::Bool(true)
        || framebuffer_evidence["captured"] == Value::Bool(true);
    let certified = board_present
        && !execution_failed
        && !emulator_detected
        && clean_ok
        && missing_markers.is_empty()
        && ordered
        && framebuffer_ok;
    let status = if certified {
        "certified"
    } else if emulator_detected {
        "rejected_emulator"
    } else if validate_only || !board_present {
        "validated_not_executed"
    } else {
        "certification_failed"
    };
    if !certified && not_executed_reason.is_none() && status == "validated_not_executed" {
        not_executed_reason = Some("physical_board_not_available".into());
    }
    let commands_successful = command_reports
        .iter()
        .all(|report| report.get("success").and_then(Value::as_bool) == Some(true));
    let session = json!({
        "schema": BOARD_SESSION_SCHEMA,
        "session_version": BOARD_FORMAT_VERSION,
        "status": status,
        "started_unix": epoch_seconds(),
        "validate_only": validate_only,
        "board_present": board_present,
        "not_executed_reason": not_executed_reason.clone(),
        "commands": command_reports.clone(),
        "framebuffer": framebuffer_evidence.clone(),
        "limits": {
            "max_artifact_bytes": limits.max_artifact_bytes,
            "max_log_bytes_per_stream": limits.max_log_bytes,
            "max_timeout_ms": limits.max_timeout_ms,
        },
    });
    write_pretty_json(&bundle_dir.join("metadata/session.json"), &session)?;
    let evidence_files = bundle_files(&bundle_dir, &["bundle.json", "certificate.json"])?;
    let evidence_set_sha256 = sha256_bytes(&canonical_json_bytes(&Value::Array(
        evidence_files.clone(),
    ))?);
    let mut certificate = json!({
        "schema": BOARD_CERTIFICATE_SCHEMA,
        "certificate_version": BOARD_FORMAT_VERSION,
        "status": status,
        "physical_board": certified,
        "execution_class": if certified {
            "physical_hardware"
        } else if status == "rejected_emulator" {
            "emulator_rejected"
        } else if status == "validated_not_executed" {
            if validate_only {
                "validation_only"
            } else {
                "physical_hardware_unavailable"
            }
        } else {
            "physical_hardware_failed"
        },
        "issued_unix": epoch_seconds(),
        "board": {
            "id": board_id,
            "vendor": vendor,
            "model": model,
            "hardware_revision": hardware_revision,
            "serial": board_serial,
            "architecture": arch,
            "target": target,
        },
        "transport": {
            "kind": transport_kind,
            "physical": true,
            "port": transport_port,
            "baud": baud,
            "probe_serial": probe_serial,
        },
        "artifact": {
            "bundle_path": "inputs/artifact.bin",
            "bytes": artifact_metadata.len(),
            "sha256": artifact_sha256,
        },
        "provenance": {
            "manifest_bundle_path": "inputs/board-manifest.json",
            "manifest_sha256": sha256_bytes(&manifest_bytes),
            "git_head": git_head,
            "git_dirty": git_dirty,
            "require_clean_git": require_clean,
            "host_os": std::env::consts::OS,
            "host_arch": std::env::consts::ARCH,
            "github_repository": std::env::var("GITHUB_REPOSITORY").ok(),
            "github_run_id": std::env::var("GITHUB_RUN_ID").ok(),
            "github_sha": std::env::var("GITHUB_SHA").ok(),
        },
        "identity": {
            "probe_markers": probe_markers,
            "required_markers": required_markers,
            "missing_markers": missing_markers,
            "ordered_markers": ordered_markers,
            "ordered_markers_present": ordered,
            "forbidden_markers": forbidden_markers,
            "observed_forbidden_markers": observed_forbidden_markers,
            "capture_retained_sha256": sha256_bytes(capture_transcript.as_bytes()),
        },
        "framebuffer": framebuffer_evidence,
        "execution": {
            "board_present": board_present,
            "validate_only": validate_only,
            "not_executed_reason": not_executed_reason,
            "commands_successful": commands_successful,
            "framebuffer_requirement_satisfied": framebuffer_ok,
            "clean_source_requirement_satisfied": clean_ok,
            "emulator_detected": emulator_detected,
        },
        "evidence": {
            "files": evidence_files,
            "evidence_set_sha256": evidence_set_sha256,
            "all_paths_bundle_relative": true,
            "qemu_evidence_accepted_as_physical": false,
        },
        "bundle": {
            "schema": BOARD_BUNDLE_SCHEMA,
            "certificate_path": "certificate.json",
            "index_path": "bundle.json",
        },
        "truth_boundary": {
            "physical_claim_requires_probe_flash_capture_and_identity": true,
            "framebuffer_claim_requires_retained_exact_size_capture": true,
            "absence_of_connected_hardware_is_never_certified": true,
            "known_emulator_tokens_are_rejected": true,
            "hmac_attests_retained_evidence_not_the_board_silicon": true,
        }
    });
    seal_certificate(&mut certificate, signing_key_env)?;
    let certificate_path = bundle_dir.join("certificate.json");
    write_pretty_json(&certificate_path, &certificate)?;
    let files = bundle_files(&bundle_dir, &["bundle.json"])?;
    let certificate_sha256 = sha256_file(&certificate_path)?;
    let file_count = files.len();
    let generated_bundle_id = bundle_id(&files, &certificate_sha256)?;
    let index = json!({
        "schema": BOARD_BUNDLE_SCHEMA,
        "bundle_version": BOARD_FORMAT_VERSION,
        "layout": "portable-relative-paths-v1",
        "certificate": "certificate.json",
        "certificate_sha256": certificate_sha256,
        "file_count": file_count,
        "files": files,
        "bundle_id": generated_bundle_id,
    });
    write_pretty_json(&bundle_dir.join("bundle.json"), &index)?;
    print_result(
        &certificate,
        has_flag(args, "--json"),
        &format!("machine certify-board: {status} ({})", bundle_dir.display()),
    )?;
    if matches!(status, "rejected_emulator" | "certification_failed") {
        return Err(format!(
            "[E_MACHINE_BOARD_CERTIFICATION_001] {status}; retained bundle: {}",
            bundle_dir.display()
        )
        .into());
    }
    Ok(())
}

fn decode_hex(raw: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if raw.len() % 2 != 0 {
        return Err("[E_MACHINE_BOARD_ATTESTATION_001] odd-length hexadecimal tag".into());
    }
    (0..raw.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&raw[index..index + 2], 16)
                .map_err(|error| -> Box<dyn Error> { error.into() })
        })
        .collect()
}

fn verify_file_entry(root: &Path, entry: &Value) -> Result<bool, Box<dyn Error>> {
    let object = entry
        .as_object()
        .ok_or("[E_MACHINE_BOARD_BUNDLE_001] every file entry must be an object")?;
    let expected_keys = ["path", "role", "bytes", "sha256"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let actual_keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual_keys != expected_keys {
        return Err(
            "[E_MACHINE_BOARD_BUNDLE_001] file entry fields differ from the v1 schema".into(),
        );
    }
    let relative = required_str(entry, "/path")?;
    safe_bundle_relative(relative)?;
    let role = required_str(entry, "/role")?;
    if !matches!(
        role,
        "certificate" | "input" | "evidence_log" | "metadata" | "evidence"
    ) {
        return Err(format!(
            "[E_MACHINE_BOARD_BUNDLE_001] unknown file role '{role}' for '{relative}'"
        )
        .into());
    }
    let expected_role = if relative == "certificate.json" {
        "certificate"
    } else if relative.starts_with("inputs/") {
        "input"
    } else if relative.starts_with("logs/") {
        "evidence_log"
    } else if relative.starts_with("metadata/") {
        "metadata"
    } else {
        "evidence"
    };
    if role != expected_role {
        return Err(format!(
            "[E_MACHINE_BOARD_BUNDLE_001] file role '{role}' does not match path '{relative}' (expected '{expected_role}')"
        )
        .into());
    }
    let expected_bytes = entry.get("bytes").and_then(Value::as_u64).ok_or_else(|| {
        format!("[E_MACHINE_BOARD_BUNDLE_001] file entry '{relative}' requires unsigned bytes")
    })?;
    let expected_hash = required_str(entry, "/sha256")?;
    if expected_hash.len() != 64
        || !expected_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "[E_MACHINE_BOARD_BUNDLE_001] file entry '{relative}' requires a lowercase SHA-256 digest"
        )
        .into());
    }
    let path = root.join(safe_bundle_relative(relative)?);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => metadata,
        _ => return Ok(false),
    };
    Ok(metadata.len() == expected_bytes && sha256_file(&path)?.eq_ignore_ascii_case(expected_hash))
}

fn value_contains_emulator(value: &Value) -> bool {
    match value {
        Value::String(text) => emulator_token(text).is_some(),
        Value::Array(items) => items.iter().any(value_contains_emulator),
        Value::Object(object) => object.values().any(value_contains_emulator),
        _ => false,
    }
}

fn manifest_contains_emulator(manifest: &Value) -> bool {
    let direct_fields = [
        "/board/id",
        "/board/vendor",
        "/board/model",
        "/board/hardware_revision",
        "/board/serial",
        "/transport/kind",
        "/transport/port",
        "/transport/probe_serial",
        "/artifact/path",
    ];
    if direct_fields.iter().any(|pointer| {
        manifest
            .pointer(pointer)
            .and_then(Value::as_str)
            .and_then(emulator_token)
            .is_some()
    }) {
        return true;
    }
    manifest
        .get("commands")
        .and_then(Value::as_object)
        .is_some_and(|commands| commands.values().any(value_contains_emulator))
}

pub(crate) fn run_verify_board_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let input_raw = first_non_flag(args)
        .ok_or("machine verify-board-certificate expects <bundle-directory>")?;
    let input = PathBuf::from(input_raw);
    let bundle_dir = if input.is_dir() {
        input
    } else {
        input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    let bundle_path = bundle_dir.join("bundle.json");
    let bundle: Value = serde_json::from_slice(&fs::read(&bundle_path)?)?;
    let mut failures = Vec::<String>::new();
    if bundle.get("schema").and_then(Value::as_str) != Some(BOARD_BUNDLE_SCHEMA)
        || bundle.get("bundle_version").and_then(Value::as_u64) != Some(BOARD_FORMAT_VERSION)
    {
        failures.push("bundle schema/version mismatch".into());
    }
    let expected_bundle_keys = [
        "schema",
        "bundle_version",
        "layout",
        "certificate",
        "certificate_sha256",
        "file_count",
        "files",
        "bundle_id",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let actual_bundle_keys = bundle
        .as_object()
        .map(|object| object.keys().map(String::as_str).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if actual_bundle_keys != expected_bundle_keys {
        failures.push("bundle index fields differ from the v1 schema".into());
    }
    if bundle.get("layout").and_then(Value::as_str) != Some("portable-relative-paths-v1")
        || bundle.get("certificate").and_then(Value::as_str) != Some("certificate.json")
    {
        failures.push("bundle layout/certificate path mismatch".into());
    }
    let indexed_files = bundle
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut indexed_paths = BTreeSet::new();
    for entry in &indexed_files {
        match required_str(entry, "/path") {
            Ok(path) if indexed_paths.insert(path.to_string()) => {
                match verify_file_entry(&bundle_dir, entry) {
                    Ok(true) => {}
                    Ok(false) => failures.push(format!("file hash/size mismatch: {path}")),
                    Err(error) => failures.push(format!("invalid file entry '{path}': {error}")),
                }
            }
            Ok(path) => failures.push(format!("duplicate indexed path: {path}")),
            Err(error) => failures.push(error.to_string()),
        }
    }
    if bundle.get("file_count").and_then(Value::as_u64) != Some(indexed_files.len() as u64) {
        failures.push("bundle file_count mismatch".into());
    }
    if indexed_files.windows(2).any(|window| {
        window[0]["path"].as_str().unwrap_or_default()
            >= window[1]["path"].as_str().unwrap_or_default()
    }) {
        failures.push("bundle file index is not strictly path-sorted".into());
    }
    let actual_files = bundle_files(&bundle_dir, &["bundle.json"])?;
    let actual_paths = actual_files
        .iter()
        .filter_map(|entry| entry["path"].as_str().map(str::to_string))
        .collect::<BTreeSet<_>>();
    if actual_paths != indexed_paths {
        failures.push("bundle file set differs from the signed index".into());
    }
    let certificate_relative = bundle
        .get("certificate")
        .and_then(Value::as_str)
        .unwrap_or("certificate.json");
    let certificate_path = bundle_dir.join(safe_bundle_relative(certificate_relative)?);
    let certificate_bytes = fs::read(&certificate_path)?;
    let certificate: Value = serde_json::from_slice(&certificate_bytes)?;
    let certificate_sha256 = sha256_bytes(&certificate_bytes);
    if bundle.get("certificate_sha256").and_then(Value::as_str) != Some(certificate_sha256.as_str())
    {
        failures.push("certificate hash differs from bundle index".into());
    }
    let expected_bundle_id = bundle_id(&indexed_files, &certificate_sha256)?;
    if bundle.get("bundle_id").and_then(Value::as_str) != Some(expected_bundle_id.as_str()) {
        failures.push("bundle_id mismatch".into());
    }
    if certificate.get("schema").and_then(Value::as_str) != Some(BOARD_CERTIFICATE_SCHEMA)
        || certificate
            .get("certificate_version")
            .and_then(Value::as_u64)
            != Some(BOARD_FORMAT_VERSION)
    {
        failures.push("certificate schema/version mismatch".into());
    }
    let payload = certificate_payload_bytes(&certificate)?;
    let payload_hash = sha256_bytes(&payload);
    if certificate
        .pointer("/integrity/algorithm")
        .and_then(Value::as_str)
        != Some("sha256")
        || certificate
            .pointer("/integrity/canonicalization")
            .and_then(Value::as_str)
            != Some("recursive-key-sort;compact-json;exclude-integrity-and-attestation")
        || certificate
            .pointer("/integrity/payload_sha256")
            .and_then(Value::as_str)
            != Some(payload_hash.as_str())
    {
        failures.push("certificate payload SHA-256 mismatch".into());
    }
    let attestation_present = certificate.get("attestation").is_some();
    let mut attestation_ok = None::<bool>;
    if let Some(attestation) = certificate.get("attestation") {
        let env_name = option_value(args, "--signing-key-env").ok_or(
            "[E_MACHINE_BOARD_ATTESTATION_001] signed bundles require --signing-key-env for verification",
        )?;
        if attestation.get("algorithm").and_then(Value::as_str) != Some("hmac-sha256")
            || attestation.get("domain").and_then(Value::as_str)
                != Some("ncc.machine.physical_board_certificate.v1")
            || attestation.get("key_id").and_then(Value::as_str) != Some(env_name)
            || attestation.get("secret_persisted").and_then(Value::as_bool) != Some(false)
        {
            failures.push("HMAC attestation metadata mismatch".into());
            attestation_ok = Some(false);
        } else {
            let expected = decode_hex(required_str(attestation, "/tag")?)?;
            let comparison_is_constant_time = verify_hmac_from_env(env_name, &payload, &expected)?;
            attestation_ok = Some(comparison_is_constant_time);
            if !comparison_is_constant_time {
                failures.push("HMAC attestation mismatch".into());
            }
        }
    } else if has_flag(args, "--require-attestation") {
        failures.push("HMAC attestation is required but absent".into());
    }
    let evidence_files = certificate
        .pointer("/evidence/files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let evidence_set_sha256 = sha256_bytes(&canonical_json_bytes(&Value::Array(
        evidence_files.clone(),
    ))?);
    if certificate
        .pointer("/evidence/evidence_set_sha256")
        .and_then(Value::as_str)
        != Some(evidence_set_sha256.as_str())
    {
        failures.push("certificate evidence-set hash mismatch".into());
    }
    for entry in &evidence_files {
        let path = entry["path"].as_str().unwrap_or("<missing>");
        match verify_file_entry(&bundle_dir, entry) {
            Ok(true) => {}
            Ok(false) => failures.push(format!("certificate evidence mismatch: {path}")),
            Err(error) => failures.push(format!(
                "invalid certificate evidence entry '{path}': {error}"
            )),
        }
    }
    let mut evidence_paths = BTreeSet::new();
    for entry in &evidence_files {
        match required_str(entry, "/path") {
            Ok(path) if evidence_paths.insert(path.to_string()) => {}
            Ok(path) => failures.push(format!("duplicate certificate evidence path: {path}")),
            Err(error) => failures.push(error.to_string()),
        }
    }
    if evidence_files.windows(2).any(|window| {
        window[0]["path"].as_str().unwrap_or_default()
            >= window[1]["path"].as_str().unwrap_or_default()
    }) {
        failures.push("certificate evidence index is not strictly path-sorted".into());
    }
    let expected_evidence_paths = indexed_paths
        .iter()
        .filter(|path| path.as_str() != certificate_relative)
        .cloned()
        .collect::<BTreeSet<_>>();
    if evidence_paths != expected_evidence_paths {
        failures.push("certificate evidence set differs from bundle index".into());
    }
    for evidence in &evidence_files {
        if let Some(path) = evidence["path"].as_str() {
            if !indexed_files.iter().any(|indexed| indexed == evidence) {
                failures.push(format!(
                    "certificate evidence metadata differs from bundle index: {path}"
                ));
            }
        }
    }
    let manifest_path = bundle_dir.join("inputs/board-manifest.json");
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let framebuffer_manifest_contract = match validate_manifest(&manifest, None) {
        Ok(limits) => match framebuffer_contract(&manifest, limits) {
            Ok(contract) => contract,
            Err(error) => {
                failures.push(format!("bundled framebuffer contract rejected: {error}"));
                None
            }
        },
        Err(error) => {
            failures.push(format!("bundled manifest rejected: {error}"));
            None
        }
    };
    let bundled_manifest_sha256 = sha256_file(&manifest_path)?;
    if certificate
        .pointer("/provenance/manifest_sha256")
        .and_then(Value::as_str)
        != Some(bundled_manifest_sha256.as_str())
    {
        failures.push("bundled manifest hash differs from certificate".into());
    }
    let artifact_path = bundle_dir.join("inputs/artifact.bin");
    let bundled_artifact_sha256 = sha256_file(&artifact_path)?;
    if certificate
        .pointer("/artifact/sha256")
        .and_then(Value::as_str)
        != Some(bundled_artifact_sha256.as_str())
    {
        failures.push("bundled artifact hash differs from certificate".into());
    }
    if manifest
        .pointer("/artifact/sha256")
        .and_then(Value::as_str)
        .is_some_and(|expected| {
            expected != "auto" && !expected.eq_ignore_ascii_case(&bundled_artifact_sha256)
        })
    {
        failures.push("bundled artifact hash differs from manifest".into());
    }
    for (manifest_pointer, certificate_pointer) in [
        ("/board/id", "/board/id"),
        ("/board/vendor", "/board/vendor"),
        ("/board/model", "/board/model"),
        ("/board/hardware_revision", "/board/hardware_revision"),
        ("/board/serial", "/board/serial"),
        ("/board/architecture", "/board/architecture"),
        ("/board/target", "/board/target"),
        ("/transport/kind", "/transport/kind"),
        ("/transport/port", "/transport/port"),
        ("/transport/probe_serial", "/transport/probe_serial"),
    ] {
        if manifest.pointer(manifest_pointer) != certificate.pointer(certificate_pointer) {
            failures.push(format!(
                "manifest/certificate identity mismatch: {manifest_pointer}"
            ));
        }
    }
    let status = certificate
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("invalid");
    let physical = certificate
        .get("physical_board")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let certificate_validate_only = certificate
        .pointer("/execution/validate_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let expected_execution_class = match status {
        "certified" => Some("physical_hardware"),
        "validated_not_executed" if certificate_validate_only => Some("validation_only"),
        "validated_not_executed" => Some("physical_hardware_unavailable"),
        "rejected_emulator" => Some("emulator_rejected"),
        "certification_failed" => Some("physical_hardware_failed"),
        _ => None,
    };
    if expected_execution_class.is_none() {
        failures.push(format!("unknown certificate status: {status}"));
    }
    if certificate.get("execution_class").and_then(Value::as_str) != expected_execution_class {
        failures.push("certificate status/execution_class mismatch".into());
    }
    if physical != (status == "certified") {
        failures.push("physical_board must be true exactly for status=certified".into());
    }
    let certificate_framebuffer_required = certificate
        .pointer("/framebuffer/required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if certificate_framebuffer_required != framebuffer_manifest_contract.is_some() {
        failures.push("manifest/certificate framebuffer requirement mismatch".into());
    }
    if let Some(contract) = &framebuffer_manifest_contract {
        for (pointer, expected) in [
            ("/framebuffer/width", contract.width),
            ("/framebuffer/height", contract.height),
            ("/framebuffer/stride", contract.stride),
            ("/framebuffer/expected_bytes", contract.expected_bytes),
        ] {
            if certificate.pointer(pointer).and_then(Value::as_u64) != Some(expected) {
                failures.push(format!("certificate framebuffer mismatch: {pointer}"));
            }
        }
        if certificate
            .pointer("/framebuffer/pixel_format")
            .and_then(Value::as_str)
            != Some(contract.pixel_format.as_str())
            || certificate
                .pointer("/framebuffer/bundle_path")
                .and_then(Value::as_str)
                != Some("evidence/framebuffer.raw")
        {
            failures.push("certificate framebuffer format/path mismatch".into());
        }
        if status == "certified" {
            let framebuffer_path = bundle_dir.join("evidence/framebuffer.raw");
            let metadata = fs::symlink_metadata(&framebuffer_path).ok();
            let valid_file = metadata.as_ref().is_some_and(|meta| {
                meta.is_file()
                    && !meta.file_type().is_symlink()
                    && meta.len() == contract.expected_bytes
            });
            let captured = certificate
                .pointer("/framebuffer/captured")
                .and_then(Value::as_bool)
                == Some(true);
            let certificate_hash = certificate
                .pointer("/framebuffer/sha256")
                .and_then(Value::as_str);
            let observed_hash = valid_file
                .then(|| sha256_file(&framebuffer_path).ok())
                .flatten();
            if !captured
                || !valid_file
                || observed_hash.as_deref() != certificate_hash
                || certificate
                    .pointer("/execution/framebuffer_requirement_satisfied")
                    .and_then(Value::as_bool)
                    != Some(true)
            {
                failures.push("certified framebuffer evidence is absent or inconsistent".into());
            }
        }
    }
    if status == "validated_not_executed" {
        if physical
            || certificate.get("execution_class").and_then(Value::as_str)
                != expected_execution_class
        {
            failures.push("validated_not_executed cannot carry a physical claim".into());
        }
        let labels = certificate
            .pointer("/evidence/files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["path"].as_str())
            .collect::<Vec<_>>();
        if labels
            .iter()
            .any(|path| path.contains("/flash.") || path.contains("/capture."))
        {
            failures.push("validated_not_executed bundle contains flash/capture evidence".into());
        }
    }
    let session: Value =
        serde_json::from_slice(&fs::read(bundle_dir.join("metadata/session.json"))?)?;
    if session.get("schema").and_then(Value::as_str) != Some(BOARD_SESSION_SCHEMA)
        || session.get("session_version").and_then(Value::as_u64) != Some(BOARD_FORMAT_VERSION)
    {
        failures.push("session schema/version mismatch".into());
    }
    if session.get("status").and_then(Value::as_str) != Some(status) {
        failures.push("session/certificate status mismatch".into());
    }
    for (session_pointer, certificate_pointer, label) in [
        (
            "/validate_only",
            "/execution/validate_only",
            "validate_only",
        ),
        (
            "/board_present",
            "/execution/board_present",
            "board_present",
        ),
        (
            "/not_executed_reason",
            "/execution/not_executed_reason",
            "not_executed_reason",
        ),
    ] {
        if session.pointer(session_pointer) != certificate.pointer(certificate_pointer) {
            failures.push(format!("session/certificate execution mismatch: {label}"));
        }
    }
    let session_commands_successful = session
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| {
            commands
                .iter()
                .all(|command| command.get("success").and_then(Value::as_bool) == Some(true))
        });
    if certificate
        .pointer("/execution/commands_successful")
        .and_then(Value::as_bool)
        != Some(session_commands_successful)
    {
        failures.push("session/certificate commands_successful mismatch".into());
    }
    let emulator_evidence = manifest_contains_emulator(&manifest)
        || session.get("commands").is_some_and(value_contains_emulator)
        || indexed_files.iter().any(|entry| {
            entry["path"].as_str().is_some_and(|path| {
                path.starts_with("logs/")
                    && fs::read(bundle_dir.join(path)).ok().is_some_and(|bytes| {
                        emulator_token(&String::from_utf8_lossy(&bytes)).is_some()
                    })
            })
        });
    if emulator_evidence != (status == "rejected_emulator") {
        if emulator_evidence {
            failures.push("known emulator evidence has a non-rejected certificate status".into());
        } else if status == "rejected_emulator" {
            failures.push("rejected_emulator status lacks retained emulator evidence".into());
        }
    }
    if emulator_evidence && physical {
        failures.push("known emulator evidence detected in physical bundle".into());
    }
    if status == "certified" {
        let commands = session
            .get("commands")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for label in ["probe", "flash", "capture"] {
            let command = commands
                .iter()
                .find(|command| command["label"].as_str() == Some(label));
            if !command.is_some_and(|command| {
                command["success"] == true
                    && command["shell_implicit"] == false
                    && command["log_limit_exceeded"] == false
                    && command["emulator_marker_observed"] == false
                    && command["resolved_program_identity_verified"] == true
                    && command["resolved_program_changed_during_execution"] == false
            }) {
                failures.push(format!(
                    "certified session lacks a successful structured {label} command"
                ));
            }
        }
        let capture_bytes = fs::read(bundle_dir.join("logs/capture.stdout.log"))?;
        let capture = String::from_utf8_lossy(&capture_bytes);
        let capture_sha256 = sha256_bytes(&capture_bytes);
        if certificate
            .pointer("/identity/capture_retained_sha256")
            .and_then(Value::as_str)
            != Some(capture_sha256.as_str())
        {
            failures.push("certified capture digest mismatch".into());
        }
        let required = json_string_array(
            certificate.pointer("/identity/required_markers"),
            "certificate.identity.required_markers",
        )?;
        let ordered = json_string_array(
            certificate.pointer("/identity/ordered_markers"),
            "certificate.identity.ordered_markers",
        )?;
        if required.iter().any(|marker| !capture.contains(marker))
            || !ordered_markers_present(&capture, &ordered)
            || certificate
                .pointer("/execution/board_present")
                .and_then(Value::as_bool)
                != Some(true)
            || certificate
                .pointer("/execution/commands_successful")
                .and_then(Value::as_bool)
                != Some(true)
            || certificate
                .pointer("/execution/clean_source_requirement_satisfied")
                .and_then(Value::as_bool)
                != Some(true)
        {
            failures
                .push("certified capture does not independently satisfy identity markers".into());
        }
    }
    let verified = failures.is_empty();
    let report = json!({
        "schema": BOARD_VERIFICATION_SCHEMA,
        "status": if verified { "verified" } else { "verification_failed" },
        "bundle": bundle_dir.display().to_string(),
        "bundle_id": bundle.get("bundle_id"),
        "certificate_status": status,
        "physical_board_claim": physical,
        "payload_sha256_ok": !failures.iter().any(|failure| failure.contains("payload SHA-256")),
        "attestation_present": attestation_present,
        "attestation_ok": attestation_ok,
        "file_count": indexed_files.len(),
        "failures": failures,
    });
    print_result(
        &report,
        has_flag(args, "--json"),
        &format!(
            "machine verify-board-certificate: {}",
            report["status"].as_str().unwrap_or("verification_failed")
        ),
    )?;
    if verified {
        Ok(())
    } else {
        Err("[E_MACHINE_BOARD_CERTIFICATE_001] bundle verification failed".into())
    }
}
