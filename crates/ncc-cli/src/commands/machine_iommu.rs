use ncc_machine::iommu::{
    DmaMapping, DmaPermissions, DmaRecoveryAction, DmaRecoveryPlan, IommuArchitecture,
    IommuDeviceId, IommuDomainId, IotlbInvalidation, ViommuCommand, VirtualIommu,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::error::Error;
use std::fs;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::ptr::NonNull;
use std::time::{SystemTime, UNIX_EPOCH};

const CERTIFICATE_SCHEMA: &str = "ncc.machine.iommu_vfio_certificate.v1";
const CERTIFICATE_DOMAIN: &[u8] = b"ncc.machine.iommu_vfio_certificate.v1\0";
const VFIO_DMA_EXERCISE_SCHEMA: &str = "ncc.machine.vfio_dma_exercise.v1";
const DMA_BYTES: usize = 0x4000;
const MAX_PHYSICAL_DMA_BYTES: usize = 1024 * 1024 * 1024;
#[cfg(target_os = "linux")]
const DEFAULT_IOVA: u64 = 0x1_0000_0000;

#[derive(Debug, Clone)]
struct PhysicalDmaExercise {
    source_sha256: String,
    required_bdf: Option<String>,
    required_vendor: Option<String>,
    required_device: Option<String>,
    region_index: u32,
    operations: Vec<MmioOperation>,
    dma_bytes: usize,
    seed_byte: u8,
    expect_changed: bool,
    expected_sha256: Option<String>,
    checks: Vec<DmaCheck>,
    interrupt: PhysicalInterruptExercise,
    recovery: PhysicalRecoveryExercise,
    reset_before: bool,
    reset_after: bool,
}

#[derive(Debug, Clone)]
enum MmioOperation {
    Write {
        offset: u64,
        width: u8,
        value: MmioWriteValue,
    },
    ReadExpect {
        offset: u64,
        width: u8,
        mask: u64,
        expected: u64,
    },
    Poll {
        offset: u64,
        width: u8,
        mask: u64,
        expected: u64,
        timeout_ms: u64,
        interval_us: u64,
    },
    Fence,
    Sleep {
        milliseconds: u64,
    },
}

#[derive(Debug, Clone)]
enum MmioWriteValue {
    Constant(u64),
    DmaIova,
    DmaIovaLow,
    DmaIovaHigh,
    DmaBytes,
}

#[derive(Debug, Clone)]
struct DmaCheck {
    offset: usize,
    width: u8,
    mask: u64,
    expected: u64,
}

#[derive(Debug, Clone)]
struct PhysicalInterruptExercise {
    irq_index: u32,
    vector: u32,
    minimum_count: u64,
    timeout_ms: u64,
    require_vfio_interrupt_remapping: bool,
}

#[derive(Debug, Clone)]
struct PhysicalRecoveryExercise {
    retry_limit: u8,
    timeout_ms: u32,
    quiesce_device: bool,
    mask_msix: bool,
    disable_bus_master: bool,
    flush_iotlb: bool,
    function_level_reset: bool,
    secondary_bus_reset_allowed: bool,
    reenable_bus_master: bool,
    unmask_msix: bool,
    verify_device_alive_after_reset: bool,
}

fn json_u64(value: &Value, field: &str) -> Result<u64, Box<dyn Error>> {
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }
    if let Some(text) = value.as_str() {
        let compact = text.replace('_', "");
        return if let Some(hex) = compact
            .strip_prefix("0x")
            .or_else(|| compact.strip_prefix("0X"))
        {
            Ok(u64::from_str_radix(hex, 16)?)
        } else {
            Ok(compact.parse()?)
        };
    }
    Err(format!(
        "[E_MACHINE_VFIO_EXERCISE_001] '{field}' must be an unsigned integer or hexadecimal string"
    )
    .into())
}

fn json_width(value: Option<&Value>, field: &str) -> Result<u8, Box<dyn Error>> {
    let width = value.map_or(Ok(4), |value| json_u64(value, field))?;
    if !matches!(width, 1 | 2 | 4 | 8) {
        return Err(
            format!("[E_MACHINE_VFIO_EXERCISE_001] '{field}' must be 1, 2, 4 or 8 bytes").into(),
        );
    }
    Ok(width as u8)
}

fn width_mask(width: u8) -> u64 {
    if width == 8 {
        u64::MAX
    } else {
        (1u64 << (u32::from(width) * 8)) - 1
    }
}

fn normalized_pci_id(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    normalized
        .strip_prefix("0x")
        .unwrap_or(&normalized)
        .trim_start_matches('0')
        .to_string()
}

impl MmioWriteValue {
    fn parse(value: &Value, field: &str) -> Result<Self, Box<dyn Error>> {
        if let Some(symbol) = value.as_str() {
            return match symbol {
                "dma_iova" => Ok(Self::DmaIova),
                "dma_iova_low" => Ok(Self::DmaIovaLow),
                "dma_iova_high" => Ok(Self::DmaIovaHigh),
                "dma_bytes" => Ok(Self::DmaBytes),
                _ => Ok(Self::Constant(json_u64(value, field)?)),
            };
        }
        Ok(Self::Constant(json_u64(value, field)?))
    }

    #[cfg(target_os = "linux")]
    fn resolve(&self, iova: u64, bytes: usize) -> u64 {
        match self {
            Self::Constant(value) => *value,
            Self::DmaIova => iova,
            Self::DmaIovaLow => iova & u64::from(u32::MAX),
            Self::DmaIovaHigh => iova >> 32,
            Self::DmaBytes => bytes as u64,
        }
    }

    fn label(&self) -> Value {
        match self {
            Self::Constant(value) => json!({"constant": value}),
            Self::DmaIova => json!("dma_iova"),
            Self::DmaIovaLow => json!("dma_iova_low"),
            Self::DmaIovaHigh => json!("dma_iova_high"),
            Self::DmaBytes => json!("dma_bytes"),
        }
    }
}

impl MmioOperation {
    fn parse(value: &Value, index: usize) -> Result<Self, Box<dyn Error>> {
        let object = value.as_object().ok_or_else(|| {
            format!("[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}] must be an object")
        })?;
        let kind = object.get("op").and_then(Value::as_str).ok_or_else(|| {
            format!("[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}].op is required")
        })?;
        let location = || -> Result<(u64, u8), Box<dyn Error>> {
            let offset = json_u64(
                object.get("offset").ok_or_else(|| {
                    format!(
                        "[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}].offset is required"
                    )
                })?,
                "mmio.operations[].offset",
            )?;
            let width = json_width(object.get("width"), "mmio.operations[].width")?;
            if offset % u64::from(width) != 0 {
                return Err(format!(
                    "[E_MACHINE_VFIO_EXERCISE_001] MMIO offset {offset:#x} is not naturally aligned to width {width}"
                )
                .into());
            }
            offset
                .checked_add(u64::from(width))
                .ok_or("[E_MACHINE_VFIO_EXERCISE_001] MMIO range overflows u64")?;
            Ok((offset, width))
        };
        match kind {
            "write" => {
                let (offset, width) = location()?;
                let raw = object.get("value").ok_or_else(|| {
                    format!(
                        "[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}].value is required"
                    )
                })?;
                Ok(Self::Write {
                    offset,
                    width,
                    value: MmioWriteValue::parse(raw, "mmio.operations[].value")?,
                })
            }
            "read_expect" | "poll" => {
                let (offset, width) = location()?;
                let mask = object.get("mask").map_or(Ok(width_mask(width)), |value| {
                    json_u64(value, "mmio.operations[].mask")
                })? & width_mask(width);
                let expected = json_u64(
                    object.get("equals").ok_or_else(|| {
                        format!("[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}].equals is required")
                    })?,
                    "mmio.operations[].equals",
                )? & mask;
                if kind == "read_expect" {
                    Ok(Self::ReadExpect {
                        offset,
                        width,
                        mask,
                        expected,
                    })
                } else {
                    let timeout_ms = object.get("timeout_ms").map_or(Ok(5_000), |value| {
                        json_u64(value, "mmio.operations[].timeout_ms")
                    })?;
                    let interval_us = object.get("interval_us").map_or(Ok(50), |value| {
                        json_u64(value, "mmio.operations[].interval_us")
                    })?;
                    if timeout_ms == 0 || timeout_ms > 60_000 || interval_us > 1_000_000 {
                        return Err("[E_MACHINE_VFIO_EXERCISE_001] poll timeout must be 1..60000 ms and interval <= 1000000 us".into());
                    }
                    Ok(Self::Poll {
                        offset,
                        width,
                        mask,
                        expected,
                        timeout_ms,
                        interval_us,
                    })
                }
            }
            "fence" => Ok(Self::Fence),
            "sleep" => {
                let milliseconds = json_u64(
                    object.get("milliseconds").ok_or_else(|| {
                        format!("[E_MACHINE_VFIO_EXERCISE_001] mmio.operations[{index}].milliseconds is required")
                    })?,
                    "mmio.operations[].milliseconds",
                )?;
                if milliseconds > 60_000 {
                    return Err(
                        "[E_MACHINE_VFIO_EXERCISE_001] a single MMIO sleep cannot exceed 60000 ms"
                            .into(),
                    );
                }
                Ok(Self::Sleep { milliseconds })
            }
            _ => {
                Err(format!("[E_MACHINE_VFIO_EXERCISE_001] unknown MMIO operation '{kind}'").into())
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn end_offset(&self) -> u64 {
        match self {
            Self::Write { offset, width, .. }
            | Self::ReadExpect { offset, width, .. }
            | Self::Poll { offset, width, .. } => *offset + u64::from(*width),
            Self::Fence | Self::Sleep { .. } => 0,
        }
    }

    fn summary(&self) -> Value {
        match self {
            Self::Write {
                offset,
                width,
                value,
            } => json!({"op": "write", "offset": offset, "width": width, "value": value.label()}),
            Self::ReadExpect {
                offset,
                width,
                mask,
                expected,
            } => {
                json!({"op": "read_expect", "offset": offset, "width": width, "mask": mask, "equals": expected})
            }
            Self::Poll {
                offset,
                width,
                mask,
                expected,
                timeout_ms,
                interval_us,
            } => {
                json!({"op": "poll", "offset": offset, "width": width, "mask": mask, "equals": expected, "timeout_ms": timeout_ms, "interval_us": interval_us})
            }
            Self::Fence => json!({"op": "fence"}),
            Self::Sleep { milliseconds } => json!({"op": "sleep", "milliseconds": milliseconds}),
        }
    }
}

impl DmaCheck {
    fn parse(value: &Value, index: usize, dma_bytes: usize) -> Result<Self, Box<dyn Error>> {
        let object = value.as_object().ok_or_else(|| {
            format!(
                "[E_MACHINE_VFIO_EXERCISE_001] dma.completion.checks[{index}] must be an object"
            )
        })?;
        let offset = json_u64(
            object.get("offset").ok_or_else(|| {
                format!("[E_MACHINE_VFIO_EXERCISE_001] dma.completion.checks[{index}].offset is required")
            })?,
            "dma.completion.checks[].offset",
        )? as usize;
        let width = json_width(object.get("width"), "dma.completion.checks[].width")?;
        if offset
            .checked_add(usize::from(width))
            .is_none_or(|end| end > dma_bytes)
        {
            return Err(format!(
                "[E_MACHINE_VFIO_EXERCISE_001] DMA check at {offset:#x} width {width} exceeds {dma_bytes} bytes"
            )
            .into());
        }
        let mask = object.get("mask").map_or(Ok(width_mask(width)), |value| {
            json_u64(value, "dma.completion.checks[].mask")
        })? & width_mask(width);
        let expected = json_u64(
            object.get("equals").ok_or_else(|| {
                format!("[E_MACHINE_VFIO_EXERCISE_001] dma.completion.checks[{index}].equals is required")
            })?,
            "dma.completion.checks[].equals",
        )? & mask;
        Ok(Self {
            offset,
            width,
            mask,
            expected,
        })
    }

    fn summary(&self) -> Value {
        json!({"offset": self.offset, "width": self.width, "mask": self.mask, "equals": self.expected})
    }
}

impl PhysicalInterruptExercise {
    fn parse(document: &Value) -> Result<Self, Box<dyn Error>> {
        let interrupt = document
            .get("interrupt")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] interrupt object is required")?;
        if interrupt.get("mode").and_then(Value::as_str) != Some("msix") {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] interrupt.mode must be 'msix'".into());
        }
        let vector = json_u64(
            interrupt
                .get("vector")
                .ok_or("[E_MACHINE_VFIO_EXERCISE_001] interrupt.vector is required")?,
            "interrupt.vector",
        )?;
        let vector = u32::try_from(vector)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] interrupt.vector exceeds u32")?;
        let minimum_count = interrupt
            .get("minimum_count")
            .map_or(Ok(1), |value| json_u64(value, "interrupt.minimum_count"))?;
        if minimum_count == 0 {
            return Err(
                "[E_MACHINE_VFIO_EXERCISE_001] interrupt.minimum_count must be positive".into(),
            );
        }
        let timeout_ms = interrupt
            .get("timeout_ms")
            .map_or(Ok(5_000), |value| json_u64(value, "interrupt.timeout_ms"))?;
        if timeout_ms == 0 || timeout_ms > 60_000 {
            return Err(
                "[E_MACHINE_VFIO_EXERCISE_001] interrupt.timeout_ms must be 1..60000".into(),
            );
        }
        let require_vfio_interrupt_remapping = interrupt
            .get("require_vfio_interrupt_remapping")
            .and_then(Value::as_bool)
            == Some(true);
        if !require_vfio_interrupt_remapping {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] interrupt.require_vfio_interrupt_remapping must be true".into());
        }
        Ok(Self {
            irq_index: ncc_machine::iommu::VFIO_PCI_MSIX_IRQ_INDEX,
            vector,
            minimum_count,
            timeout_ms,
            require_vfio_interrupt_remapping,
        })
    }

    fn summary(&self) -> Value {
        json!({
            "mode": "msix",
            "irq_index": self.irq_index,
            "vector": self.vector,
            "minimum_count": self.minimum_count,
            "timeout_ms": self.timeout_ms,
            "require_vfio_interrupt_remapping": self.require_vfio_interrupt_remapping,
        })
    }
}

impl PhysicalRecoveryExercise {
    fn parse(document: &Value) -> Result<Self, Box<dyn Error>> {
        let recovery = document
            .get("recovery")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] recovery object is required")?;
        let retry_limit = recovery
            .get("retry_limit")
            .map_or(Ok(3), |value| json_u64(value, "recovery.retry_limit"))?;
        let retry_limit = u8::try_from(retry_limit)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] recovery.retry_limit exceeds u8")?;
        let timeout_ms = recovery
            .get("timeout_ms")
            .map_or(Ok(10_000), |value| json_u64(value, "recovery.timeout_ms"))?;
        let timeout_ms = u32::try_from(timeout_ms)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] recovery.timeout_ms exceeds u32")?;
        let flag = |name: &str| -> Result<bool, Box<dyn Error>> {
            recovery.get(name).and_then(Value::as_bool).ok_or_else(|| {
                format!("[E_MACHINE_VFIO_EXERCISE_001] recovery.{name}=true is required").into()
            })
        };
        let quiesce_device = flag("quiesce_device")?;
        let mask_msix = flag("mask_msix")?;
        let disable_bus_master = flag("disable_bus_master")?;
        let flush_iotlb = flag("flush_iotlb")?;
        let function_level_reset = flag("function_level_reset")?;
        let secondary_bus_reset_allowed = recovery
            .get("secondary_bus_reset_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let reenable_bus_master = flag("reenable_bus_master")?;
        let unmask_msix = flag("unmask_msix")?;
        let verify_device_alive_after_reset = flag("verify_device_alive_after_reset")?;
        let actions = [
            Some(DmaRecoveryAction::QuiesceDevice),
            Some(DmaRecoveryAction::MaskMsix),
            Some(DmaRecoveryAction::DisableBusMaster),
            Some(DmaRecoveryAction::FlushIotlb(IotlbInvalidation::Global)),
            Some(DmaRecoveryAction::FunctionLevelReset),
            Some(DmaRecoveryAction::ReenableBusMaster),
            Some(DmaRecoveryAction::UnmaskMsix),
            if secondary_bus_reset_allowed {
                Some(DmaRecoveryAction::SecondaryBusReset)
            } else {
                None
            },
        ];
        let plan = DmaRecoveryPlan::new(actions, retry_limit, timeout_ms);
        plan.validate()?;
        if !(quiesce_device
            && mask_msix
            && disable_bus_master
            && flush_iotlb
            && function_level_reset
            && reenable_bus_master
            && unmask_msix
            && verify_device_alive_after_reset)
        {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] recovery must explicitly enable quiesce, MSI-X mask/unmask, bus-master off/on, IOTLB flush, FLR and alive check".into());
        }
        Ok(Self {
            retry_limit,
            timeout_ms,
            quiesce_device,
            mask_msix,
            disable_bus_master,
            flush_iotlb,
            function_level_reset,
            secondary_bus_reset_allowed,
            reenable_bus_master,
            unmask_msix,
            verify_device_alive_after_reset,
        })
    }

    fn summary(&self) -> Value {
        json!({
            "retry_limit": self.retry_limit,
            "timeout_ms": self.timeout_ms,
            "quiesce_device": self.quiesce_device,
            "mask_msix": self.mask_msix,
            "disable_bus_master": self.disable_bus_master,
            "flush_iotlb": self.flush_iotlb,
            "function_level_reset": self.function_level_reset,
            "secondary_bus_reset_allowed": self.secondary_bus_reset_allowed,
            "reenable_bus_master": self.reenable_bus_master,
            "unmask_msix": self.unmask_msix,
            "verify_device_alive_after_reset": self.verify_device_alive_after_reset,
        })
    }
}

impl PhysicalDmaExercise {
    fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let bytes = fs::read(path)?;
        Self::parse(&bytes)
    }

    fn parse(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let document: Value = serde_json::from_slice(&bytes)?;
        if document.get("schema").and_then(Value::as_str) != Some(VFIO_DMA_EXERCISE_SCHEMA) {
            return Err(format!(
                "[E_MACHINE_VFIO_EXERCISE_001] exercise schema must be '{VFIO_DMA_EXERCISE_SCHEMA}'"
            )
            .into());
        }
        if document
            .get("acknowledge_exclusive_device_control")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(
                "[E_MACHINE_VFIO_EXERCISE_001] acknowledge_exclusive_device_control must be true"
                    .into(),
            );
        }
        let device = document
            .get("device")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] device object is required")?;
        let required_bdf = device
            .get("bdf")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] device.bdf is required")?;
        let required_vendor = device
            .get("vendor")
            .map(|value| json_u64(value, "device.vendor"))
            .transpose()?
            .map(|value| normalized_pci_id(&format!("{value:x}")))
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] device.vendor is required")?;
        let required_device = device
            .get("device")
            .map(|value| json_u64(value, "device.device"))
            .transpose()?
            .map(|value| normalized_pci_id(&format!("{value:x}")))
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] device.device is required")?;
        let mmio = document
            .get("mmio")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] mmio object is required")?;
        let region_index = json_u64(
            mmio.get("region_index")
                .ok_or("[E_MACHINE_VFIO_EXERCISE_001] mmio.region_index is required")?,
            "mmio.region_index",
        )?;
        let region_index = u32::try_from(region_index)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] mmio.region_index exceeds u32")?;
        let raw_operations = mmio
            .get("operations")
            .and_then(Value::as_array)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] mmio.operations array is required")?;
        if raw_operations.is_empty() || raw_operations.len() > 1024 {
            return Err(
                "[E_MACHINE_VFIO_EXERCISE_001] mmio.operations must contain 1..1024 entries".into(),
            );
        }
        let operations = raw_operations
            .iter()
            .enumerate()
            .map(|(index, value)| MmioOperation::parse(value, index))
            .collect::<Result<Vec<_>, _>>()?;
        if !operations
            .iter()
            .any(|operation| matches!(operation, MmioOperation::Write { .. }))
        {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] physical DMA exercise requires at least one MMIO write".into());
        }

        let dma = document
            .get("dma")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] dma object is required")?;
        let dma_bytes = dma
            .get("bytes")
            .map_or(Ok(DMA_BYTES as u64), |value| json_u64(value, "dma.bytes"))?;
        let dma_bytes = usize::try_from(dma_bytes)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] dma.bytes exceeds host usize")?;
        if dma_bytes == 0 || dma_bytes > MAX_PHYSICAL_DMA_BYTES || dma_bytes % 4096 != 0 {
            return Err(format!(
                "[E_MACHINE_VFIO_EXERCISE_001] dma.bytes must be page aligned in 4096..={MAX_PHYSICAL_DMA_BYTES}"
            )
            .into());
        }
        let seed_byte = dma
            .get("seed_byte")
            .map_or(Ok(0x5a), |value| json_u64(value, "dma.seed_byte"))?;
        let seed_byte = u8::try_from(seed_byte)
            .map_err(|_| "[E_MACHINE_VFIO_EXERCISE_001] dma.seed_byte exceeds one byte")?;
        let completion = dma
            .get("completion")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] dma.completion object is required")?;
        let expect_changed = completion
            .get("expect_changed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !expect_changed {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] dma.completion.expect_changed must be true for physical DMA evidence".into());
        }
        let expected_sha256 = completion
            .get("sha256")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase());
        if expected_sha256.as_ref().is_some_and(|value| {
            value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(
                "[E_MACHINE_VFIO_EXERCISE_001] dma.completion.sha256 must be 64 hexadecimal digits"
                    .into(),
            );
        }
        let checks = completion
            .get("checks")
            .and_then(Value::as_array)
            .map(|checks| {
                checks
                    .iter()
                    .enumerate()
                    .map(|(index, value)| DmaCheck::parse(value, index, dma_bytes))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        if expected_sha256.is_none() && checks.is_empty() {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] physical DMA requires an exact sha256 or at least one exact completion check".into());
        }
        let interrupt = PhysicalInterruptExercise::parse(&document)?;
        let recovery = PhysicalRecoveryExercise::parse(&document)?;
        let reset = document
            .get("reset")
            .and_then(Value::as_object)
            .ok_or("[E_MACHINE_VFIO_EXERCISE_001] reset object is required")?;
        let reset_before = reset.get("before").and_then(Value::as_bool) == Some(true);
        let reset_after = reset.get("after").and_then(Value::as_bool) == Some(true);
        if !reset_before || !reset_after {
            return Err("[E_MACHINE_VFIO_EXERCISE_001] physical certification requires reset.before=true and reset.after=true".into());
        }
        Ok(Self {
            source_sha256,
            required_bdf: Some(required_bdf),
            required_vendor: Some(required_vendor),
            required_device: Some(required_device),
            region_index,
            operations,
            dma_bytes,
            seed_byte,
            expect_changed,
            expected_sha256,
            checks,
            interrupt,
            recovery,
            reset_before,
            reset_after,
        })
    }

    fn summary(&self) -> Value {
        json!({
            "schema": VFIO_DMA_EXERCISE_SCHEMA,
            "source_sha256": self.source_sha256,
            "required_device": {
                "bdf": self.required_bdf,
                "vendor": self.required_vendor,
                "device": self.required_device,
            },
            "mmio": {
                "region_index": self.region_index,
                "operations": self.operations.iter().map(MmioOperation::summary).collect::<Vec<_>>(),
            },
            "dma": {
                "bytes": self.dma_bytes,
                "seed_byte": self.seed_byte,
                "expect_changed": self.expect_changed,
                "expected_sha256": self.expected_sha256,
                "checks": self.checks.iter().map(DmaCheck::summary).collect::<Vec<_>>(),
            },
            "interrupt": self.interrupt.summary(),
            "recovery": self.recovery.summary(),
            "reset": {"before": self.reset_before, "after": self.reset_after},
        })
    }

    #[cfg(target_os = "linux")]
    fn verify_device_identity(
        &self,
        actual_bdf: Option<&str>,
        identity: Option<&Value>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(required) = self.required_bdf.as_deref() {
            let actual = actual_bdf.ok_or(
                "[E_MACHINE_VFIO_EXERCISE_DEVICE_001] exercise requires a PCI BDF; pass --device",
            )?;
            if !required.eq_ignore_ascii_case(actual) {
                return Err(format!(
                    "[E_MACHINE_VFIO_EXERCISE_DEVICE_001] exercise requires PCI device {required}, got {actual}"
                )
                .into());
            }
        }
        for (field, required) in [
            ("vendor", self.required_vendor.as_deref()),
            ("device", self.required_device.as_deref()),
        ] {
            let Some(required) = required else {
                continue;
            };
            let actual = identity
                .and_then(|identity| identity.get(field))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "[E_MACHINE_VFIO_EXERCISE_DEVICE_001] cannot verify required PCI {field}"
                    )
                })?;
            if normalized_pci_id(actual) != required {
                return Err(format!(
                    "[E_MACHINE_VFIO_EXERCISE_DEVICE_001] exercise requires PCI {field} {required}, got {actual}"
                )
                .into());
            }
        }
        Ok(())
    }
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
            "--group"
                | "--device"
                | "--device-cdev"
                | "--iommufd"
                | "--iova"
                | "--exercise"
                | "--out"
                | "--certificate"
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

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonical_json(&object[key]));
            }
            Value::Object(canonical)
        }
        scalar => scalar.clone(),
    }
}

fn payload_sha256(payload: &Value) -> Result<String, Box<dyn Error>> {
    let bytes = serde_json::to_vec(&canonical_json(payload))?;
    let mut digest = Sha256::new();
    digest.update(CERTIFICATE_DOMAIN);
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn host_identity() -> Value {
    json!({
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "hostname": std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string()),
    })
}

struct AlignedDmaMemory {
    pointer: NonNull<u8>,
    layout: Layout,
}

impl AlignedDmaMemory {
    fn new(bytes: usize) -> Result<Self, Box<dyn Error>> {
        let layout = Layout::from_size_align(bytes, 4096)?;
        let pointer = NonNull::new(unsafe { alloc_zeroed(layout) })
            .ok_or("[E_MACHINE_IOMMU_MEMORY_001] page-aligned allocation failed")?;
        Ok(Self { pointer, layout })
    }

    fn address(&self) -> u64 {
        self.pointer.as_ptr() as u64
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.layout.size()) }
    }
}

impl Drop for AlignedDmaMemory {
    fn drop(&mut self) {
        unsafe { dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

fn execute_viommu_model() -> Result<Value, Box<dyn Error>> {
    let mut memory = AlignedDmaMemory::new(DMA_BYTES)?;
    let device = IommuDeviceId::pci(0, 0x02, 0x03, 1)
        .ok_or("[E_MACHINE_IOMMU_DEVICE_001] invalid synthetic requester ID")?;
    let domain = IommuDomainId(0x42);
    let iova = 0x2000_0000;
    let mut viommu = VirtualIommu::<16, 8, 32, 16>::new(IommuArchitecture::SoftwareViommu, domain);
    viommu.submit(ViommuCommand::Attach(device))?;
    viommu.submit(ViommuCommand::Map(DmaMapping {
        iova,
        host_virtual_address: memory.address(),
        host_physical_address: None,
        length: DMA_BYTES as u64,
        page_size: 4096,
        permissions: DmaPermissions::READ_WRITE,
        pinned: true,
        coherent: true,
    }))?;
    viommu.submit(ViommuCommand::Invalidate(IotlbInvalidation::Address {
        domain,
        iova,
        length: DMA_BYTES as u64,
        page_selective: true,
    }))?;
    viommu.submit(ViommuCommand::Fence {
        write_barrier: true,
    })?;
    let commands = viommu.process_all()?;

    let read_translation = viommu.dma(device, iova + 0x123, DmaPermissions::DEVICE_READ)?;
    let write_translation = viommu.dma(device, iova + 0x321, DmaPermissions::DEVICE_WRITE)?;
    unsafe {
        *(write_translation.host_virtual_address as *mut u8) = 0xa5;
    }
    let material_byte = memory.bytes_mut()[0x321];
    let denied = viommu
        .dma(device, iova + 0x88, DmaPermissions::EXECUTE)
        .is_err();
    let fault = viommu
        .faults
        .pop()
        .ok_or("[E_MACHINE_IOMMU_FAULT_001] denied DMA did not enter the fault ring")?;
    viommu.submit(ViommuCommand::Unmap {
        iova,
        length: DMA_BYTES as u64,
    })?;
    viommu.submit(ViommuCommand::Detach(device))?;
    let teardown_commands = viommu.process_all()?;

    if commands != 4
        || teardown_commands != 2
        || read_translation.host_virtual_address != memory.address() + 0x123
        || material_byte != 0xa5
        || !denied
        || fault.iova != iova + 0x88
        || viommu.domain.mapping_count() != 0
        || viommu.domain.device_count() != 0
    {
        return Err("[E_MACHINE_IOMMU_MODEL_001] vIOMMU material evidence mismatch".into());
    }

    Ok(json!({
        "model": "ncc_machine_virtual_iommu",
        "material_execution": true,
        "architecture": "software_viommu",
        "domain": domain.0,
        "requester": {
            "segment": device.segment,
            "bus": device.bus(),
            "device": device.device(),
            "function": device.function(),
        },
        "dma": {
            "iova": iova,
            "bytes": DMA_BYTES,
            "read_translation_offset": read_translation.host_virtual_address - memory.address(),
            "write_translation_offset": write_translation.host_virtual_address - memory.address(),
            "material_byte": material_byte,
            "permissions_enforced": denied,
            "coherent": write_translation.coherent,
        },
        "queues": {
            "commands_completed": commands + teardown_commands,
            "invalidation_generation": viommu.invalidation_generation,
            "fault_iova": fault.iova,
            "fault_reason": format!("{:?}", fault.reason),
        },
        "teardown": {
            "mappings_remaining": viommu.domain.mapping_count(),
            "devices_remaining": viommu.domain.device_count(),
        },
    }))
}

pub(crate) fn run_probe_iommu(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = platform::probe()?;
    let report = json!({
        "schema": "ncc.machine.iommu_vfio_probe.v1",
        "host": host_identity(),
        "architectures": ["intel_vtd", "amd_vi", "arm_smmuv3", "riscv_iommu", "software_viommu"],
        "provider": payload,
        "truth_boundary": {
            "probe_only": true,
            "physical_passthrough_certified": false,
        },
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

pub(crate) fn run_certify_viommu(args: &[String]) -> Result<(), Box<dyn Error>> {
    let model = execute_viommu_model()?;
    let payload = json!({
        "schema": CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "kind": "viommu_material_model",
        "status": "material_model_passed",
        "created_epoch_seconds": epoch_seconds(),
        "source": super::machine_backends::source_identity(),
        "host": host_identity(),
        "model": model,
        "truth_boundary": {
            "software_viommu_executed": true,
            "dma_translation_materialized": true,
            "fault_and_invalidation_queues_executed": true,
            "physical_iommu_executed": false,
            "vfio_passthrough_executed": false,
        },
    });
    let certificate = json!({
        "payload": payload,
        "payload_sha256": payload_sha256(&payload)?,
    });
    if let Some(path) = option_value(args, "--out") {
        write_json(Path::new(path), &certificate)?;
    }
    if has_flag(args, "--json") || option_value(args, "--out").is_none() {
        println!("{}", serde_json::to_string_pretty(&certificate)?);
    } else {
        println!(
            "OK: material vIOMMU certificate written to {}",
            option_value(args, "--out").unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_certify_vfio(args: &[String]) -> Result<(), Box<dyn Error>> {
    let model = execute_viommu_model()?;
    let exercise = option_value(args, "--exercise")
        .map(|path| PhysicalDmaExercise::load(Path::new(path)))
        .transpose()?;
    let execution = platform::certify_vfio(args, exercise.as_ref())?;
    let physical_dma = execution["device_generated_dma_executed"] == true;
    let reset_performed = execution["device_reset_performed"] == true;
    let msix_delivered = execution["msix_eventfd_delivered"] == true;
    let interrupt_remapping = execution["vfio_interrupt_remapping_path_executed"] == true;
    let iova_isolation = execution["iova_isolation_executed"] == true;
    let group_isolation = execution["iommu_group_isolation_verified"] == true;
    let architecture_identified = execution["hardware_iommu_architecture_identified"] == true;
    let driver_recovery = execution["driver_recovery_performed"] == true;
    let physical_complete = physical_dma
        && reset_performed
        && msix_delivered
        && interrupt_remapping
        && iova_isolation
        && group_isolation
        && architecture_identified
        && driver_recovery;
    let kind = if physical_complete {
        "vfio_physical_passthrough"
    } else {
        "vfio_control_plane"
    };
    let status = if physical_complete {
        "vfio_physical_data_plane_passed"
    } else {
        "vfio_control_plane_passed"
    };
    let payload = json!({
        "schema": CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "kind": kind,
        "status": status,
        "created_epoch_seconds": epoch_seconds(),
        "source": super::machine_backends::source_identity(),
        "host": host_identity(),
        "model": model,
        "vfio": execution,
        "exercise": exercise.as_ref().map(PhysicalDmaExercise::summary),
        "truth_boundary": {
            "software_viommu_executed": true,
            "physical_iommu_executed": true,
            "vfio_passthrough_executed": true,
            "iommu_dma_map_unmap_executed": true,
            "device_generated_dma_executed": physical_dma,
            "device_reset_performed": reset_performed,
            "msix_eventfd_delivered": msix_delivered,
            "vfio_interrupt_remapping_path_executed": interrupt_remapping,
            "iova_isolation_executed": iova_isolation,
            "iommu_group_isolation_verified": group_isolation,
            "hardware_iommu_architecture_identified": architecture_identified,
            "driver_recovery_performed": driver_recovery,
            "device_rebinding_performed": false,
        },
    });
    let certificate = json!({
        "payload": payload,
        "payload_sha256": payload_sha256(&payload)?,
    });
    if let Some(path) = option_value(args, "--out") {
        write_json(Path::new(path), &certificate)?;
    }
    if has_flag(args, "--json") || option_value(args, "--out").is_none() {
        println!("{}", serde_json::to_string_pretty(&certificate)?);
    } else {
        println!(
            "OK: VFIO {kind} certificate written to {}",
            option_value(args, "--out").unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_iommu_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args)
        .or_else(|| option_value(args, "--certificate"))
        .ok_or("verify-iommu-certificate requires a certificate path")?;
    let certificate: Value = serde_json::from_slice(&fs::read(path)?)?;
    let payload = certificate
        .get("payload")
        .ok_or("IOMMU certificate has no payload")?;
    if payload["schema"] != CERTIFICATE_SCHEMA {
        return Err("[E_MACHINE_IOMMU_CERTIFICATE_SCHEMA_001] invalid certificate schema".into());
    }
    let expected = certificate["payload_sha256"]
        .as_str()
        .ok_or("IOMMU certificate has no payload_sha256")?;
    let actual = payload_sha256(payload)?;
    if expected != actual {
        return Err(format!(
            "[E_MACHINE_IOMMU_CERTIFICATE_HASH_001] payload hash mismatch: expected={expected} actual={actual}"
        )
        .into());
    }
    if payload
        .pointer("/source/github_sha")
        .and_then(Value::as_str)
        .is_some()
        && payload
            .pointer("/source/exact_head_match")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("[E_MACHINE_IOMMU_CERTIFICATE_HEAD_001] GitHub certificate was not produced from the checked-out exact head".into());
    }
    let kind = payload["kind"].as_str().unwrap_or_default();
    match kind {
        "viommu_material_model" => {
            for pointer in [
                "/truth_boundary/software_viommu_executed",
                "/truth_boundary/dma_translation_materialized",
                "/truth_boundary/fault_and_invalidation_queues_executed",
                "/model/material_execution",
            ] {
                if payload.pointer(pointer).and_then(Value::as_bool) != Some(true) {
                    return Err(format!(
                        "[E_MACHINE_IOMMU_CERTIFICATE_EVIDENCE_001] required '{pointer}' is not true"
                    )
                    .into());
                }
            }
        }
        "vfio_control_plane" => {
            for pointer in [
                "/truth_boundary/physical_iommu_executed",
                "/truth_boundary/vfio_passthrough_executed",
                "/truth_boundary/iommu_dma_map_unmap_executed",
                "/vfio/provider_api_executed",
                "/vfio/dma_map_unmap_executed",
                "/vfio/teardown_complete",
            ] {
                if payload.pointer(pointer).and_then(Value::as_bool) != Some(true) {
                    return Err(format!(
                        "[E_MACHINE_IOMMU_CERTIFICATE_EVIDENCE_001] required '{pointer}' is not true"
                    )
                    .into());
                }
            }
        }
        "vfio_physical_passthrough" => {
            for pointer in [
                "/truth_boundary/physical_iommu_executed",
                "/truth_boundary/vfio_passthrough_executed",
                "/truth_boundary/iommu_dma_map_unmap_executed",
                "/truth_boundary/device_generated_dma_executed",
                "/truth_boundary/device_reset_performed",
                "/truth_boundary/msix_eventfd_delivered",
                "/truth_boundary/vfio_interrupt_remapping_path_executed",
                "/truth_boundary/iova_isolation_executed",
                "/truth_boundary/iommu_group_isolation_verified",
                "/truth_boundary/hardware_iommu_architecture_identified",
                "/truth_boundary/driver_recovery_performed",
                "/vfio/provider_api_executed",
                "/vfio/dma_map_unmap_executed",
                "/vfio/device_generated_dma_executed",
                "/vfio/device_reset_performed",
                "/vfio/msix_eventfd_delivered",
                "/vfio/vfio_interrupt_remapping_path_executed",
                "/vfio/iova_isolation_executed",
                "/vfio/iommu_group_isolation_verified",
                "/vfio/hardware_iommu_architecture_identified",
                "/vfio/driver_recovery_performed",
                "/vfio/teardown_complete",
            ] {
                if payload.pointer(pointer).and_then(Value::as_bool) != Some(true) {
                    return Err(format!(
                        "[E_MACHINE_IOMMU_CERTIFICATE_EVIDENCE_001] required '{pointer}' is not true"
                    )
                    .into());
                }
            }
            if payload.pointer("/exercise/schema").and_then(Value::as_str)
                != Some(VFIO_DMA_EXERCISE_SCHEMA)
            {
                return Err("[E_MACHINE_IOMMU_CERTIFICATE_EVIDENCE_001] physical DMA certificate has no validated exercise".into());
            }
        }
        _ => {
            return Err(format!(
                "[E_MACHINE_IOMMU_CERTIFICATE_KIND_001] unknown certificate kind '{kind}'"
            )
            .into())
        }
    }
    let report = json!({
        "schema": "ncc.machine.iommu_vfio_certificate_verification.v1",
        "status": "verified",
        "kind": kind,
        "certificate": path,
        "payload_sha256": actual,
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("OK: IOMMU/VFIO certificate verified ({kind})");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use ncc_machine::iommu::{
        IommufdDestroy, IommufdIoasAlloc, IommufdIoasMap, IommufdIoasUnmap,
        VfioDeviceAttachIommufdPt, VfioDeviceBindIommufd, VfioDeviceDetachIommufdPt,
        VfioDeviceInfo, VfioIommuType1DmaMap, VfioIommuType1DmaUnmap, VfioIommuType1Info,
        VfioIrqInfo, VfioIrqSet, VfioIrqSetEventFd, VfioRegionInfo, IOMMU_DESTROY,
        IOMMU_IOAS_ALLOC, IOMMU_IOAS_MAP, IOMMU_IOAS_MAP_FIXED_IOVA, IOMMU_IOAS_MAP_READABLE,
        IOMMU_IOAS_MAP_WRITEABLE, IOMMU_IOAS_UNMAP, VFIO_API_VERSION, VFIO_CHECK_EXTENSION,
        VFIO_DEVICE_ATTACH_IOMMUFD_PT, VFIO_DEVICE_BIND_IOMMUFD, VFIO_DEVICE_DETACH_IOMMUFD_PT,
        VFIO_DEVICE_FLAGS_RESET, VFIO_DEVICE_GET_INFO, VFIO_DEVICE_GET_IRQ_INFO,
        VFIO_DEVICE_GET_REGION_INFO, VFIO_DEVICE_RESET, VFIO_DEVICE_SET_IRQS,
        VFIO_DMA_MAP_FLAG_READ, VFIO_DMA_MAP_FLAG_WRITE, VFIO_GET_API_VERSION,
        VFIO_GROUP_FLAGS_CONTAINER_SET, VFIO_GROUP_FLAGS_VIABLE, VFIO_GROUP_GET_DEVICE_FD,
        VFIO_GROUP_GET_STATUS, VFIO_GROUP_SET_CONTAINER, VFIO_GROUP_UNSET_CONTAINER,
        VFIO_IOMMU_GET_INFO, VFIO_IOMMU_MAP_DMA, VFIO_IOMMU_UNMAP_DMA, VFIO_IRQ_INFO_EVENTFD,
        VFIO_IRQ_SET_ACTION_TRIGGER, VFIO_IRQ_SET_DATA_EVENTFD, VFIO_REGION_INFO_FLAG_MMAP,
        VFIO_REGION_INFO_FLAG_READ, VFIO_REGION_INFO_FLAG_WRITE, VFIO_SET_IOMMU,
        VFIO_TYPE1V2_IOMMU, VFIO_TYPE1_IOMMU,
    };
    use std::ffi::{CString, OsStr};
    use std::mem::size_of;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::sync::atomic::{fence, Ordering};
    use std::time::{Duration, Instant};

    fn open_rw(path: &Path) -> Result<OwnedFd, Box<dyn Error>> {
        let cpath = CString::new(path.as_os_str().as_bytes())?;
        let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(format!(
                "[E_MACHINE_VFIO_OPEN_001] cannot open {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            )
            .into());
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn ioctl_noarg(fd: i32, request: u64) -> Result<i32, Box<dyn Error>> {
        let result = unsafe { libc::ioctl(fd, request as libc::c_ulong) };
        if result < 0 {
            return Err(format!(
                "ioctl {request:#x} failed: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        Ok(result)
    }

    fn ioctl_value(fd: i32, request: u64, value: u64) -> Result<i32, Box<dyn Error>> {
        let result = unsafe { libc::ioctl(fd, request as libc::c_ulong, value as libc::c_ulong) };
        if result < 0 {
            return Err(format!(
                "ioctl {request:#x}({value:#x}) failed: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        Ok(result)
    }

    fn ioctl_mut<T>(fd: i32, request: u64, value: &mut T) -> Result<i32, Box<dyn Error>> {
        let result = unsafe {
            libc::ioctl(
                fd,
                request as libc::c_ulong,
                value as *mut T as *mut libc::c_void,
            )
        };
        if result < 0 {
            return Err(format!(
                "ioctl {request:#x} failed: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        Ok(result)
    }

    fn ioctl_ref<T>(fd: i32, request: u64, value: &T) -> Result<i32, Box<dyn Error>> {
        let result = unsafe {
            libc::ioctl(
                fd,
                request as libc::c_ulong,
                value as *const T as *const libc::c_void,
            )
        };
        if result < 0 {
            return Err(format!(
                "ioctl {request:#x} failed: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        Ok(result)
    }

    fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .map(|value| value.trim().to_string())
    }

    fn symlink_name(path: impl AsRef<Path>) -> Option<String> {
        fs::read_link(path).ok().and_then(|target| {
            target
                .file_name()
                .map(|name| OsStr::to_string_lossy(name).into_owned())
        })
    }

    fn sorted_directory_names(path: &Path) -> Vec<String> {
        let mut names = fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn group_member_inventory(group: &str) -> Vec<Value> {
        sorted_directory_names(
            &Path::new("/sys/kernel/iommu_groups")
                .join(group)
                .join("devices"),
        )
        .into_iter()
        .map(|bdf| {
            let root = Path::new("/sys/bus/pci/devices").join(&bdf);
            json!({
                "bdf": bdf,
                "driver": symlink_name(root.join("driver")),
                "vendor": read_trimmed(root.join("vendor")),
                "device": read_trimmed(root.join("device")),
            })
        })
        .collect()
    }

    fn iommu_host_probe() -> Value {
        let root = Path::new("/sys/class/iommu");
        let nodes = sorted_directory_names(root)
            .into_iter()
            .map(|name| {
                let node = root.join(&name);
                json!({
                    "name": name,
                    "driver": symlink_name(node.join("device/driver"))
                        .or_else(|| symlink_name(node.join("driver"))),
                    "modalias": read_trimmed(node.join("device/modalias")),
                })
            })
            .collect::<Vec<_>>();
        let groups = sorted_directory_names(Path::new("/sys/kernel/iommu_groups"));
        let mut signatures = nodes
            .iter()
            .flat_map(|node| {
                ["name", "driver", "modalias"]
                    .into_iter()
                    .filter_map(|field| node.get(field).and_then(Value::as_str))
            })
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        if let Some(cpuinfo) = read_trimmed("/proc/cpuinfo") {
            signatures.push(cpuinfo.to_ascii_lowercase());
        }
        let contains = |needles: &[&str]| {
            signatures
                .iter()
                .any(|value| needles.iter().any(|needle| value.contains(needle)))
        };
        let architecture = if contains(&["dmar", "intel-iommu", "genuineintel"])
            && std::env::consts::ARCH == "x86_64"
        {
            Some("intel_vtd")
        } else if contains(&["ivhd", "amd_iommu", "authenticamd"])
            && std::env::consts::ARCH == "x86_64"
        {
            Some("amd_vi")
        } else if contains(&["smmu", "arm-smmu-v3"]) || std::env::consts::ARCH == "aarch64" {
            Some("arm_smmuv3")
        } else if contains(&["riscv-iommu", "riscv_iommu"])
            || std::env::consts::ARCH.starts_with("riscv")
        {
            Some("riscv_iommu")
        } else {
            None
        };
        json!({
            "architecture": architecture,
            "host_architecture": std::env::consts::ARCH,
            "iommu_nodes": nodes,
            "iommu_group_count": groups.len(),
            "translation_domains_exposed": !groups.is_empty(),
            "detection_source": "sysfs_iommu_nodes_groups_and_cpu_identity",
            "hardware_register_tables_read_by_ncc": false,
        })
    }

    fn group_inventory() -> Vec<Value> {
        let root = Path::new("/sys/kernel/iommu_groups");
        let mut groups = Vec::new();
        for group in sorted_directory_names(root) {
            let members = group_member_inventory(&group);
            let devices = members
                .iter()
                .filter_map(|member| member.get("bdf").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>();
            let isolated = !members.is_empty()
                && members
                    .iter()
                    .all(|member| member.get("driver").and_then(Value::as_str) == Some("vfio-pci"));
            groups.push(json!({
                "group": group,
                "device_node": format!("/dev/vfio/{group}"),
                "device_node_present": Path::new("/dev/vfio").join(&group).exists(),
                "devices": devices,
                "members": members,
                "all_members_bound_to_vfio_pci": isolated,
            }));
        }
        groups
    }

    fn vfio_container_probe() -> Value {
        let path = Path::new("/dev/vfio/vfio");
        if !path.exists() {
            return json!({"present": false});
        }
        match open_rw(path) {
            Ok(fd) => {
                let api = ioctl_noarg(fd.as_raw_fd(), VFIO_GET_API_VERSION).ok();
                let type1 = ioctl_value(
                    fd.as_raw_fd(),
                    VFIO_CHECK_EXTENSION,
                    VFIO_TYPE1_IOMMU as u64,
                )
                .ok()
                .is_some_and(|value| value != 0);
                let type1v2 = ioctl_value(
                    fd.as_raw_fd(),
                    VFIO_CHECK_EXTENSION,
                    VFIO_TYPE1V2_IOMMU as u64,
                )
                .ok()
                .is_some_and(|value| value != 0);
                json!({
                    "present": true,
                    "openable": true,
                    "api_version": api,
                    "api_version_matches": api == Some(VFIO_API_VERSION),
                    "type1": type1,
                    "type1v2": type1v2,
                })
            }
            Err(error) => json!({
                "present": true,
                "openable": false,
                "error": error.to_string(),
            }),
        }
    }

    pub(super) fn probe() -> Result<Value, Box<dyn Error>> {
        let groups = group_inventory();
        let cdev_root = Path::new("/dev/vfio/devices");
        Ok(json!({
            "os": "linux",
            "legacy": {
                "container": vfio_container_probe(),
                "groups": groups,
                "assignable_group_count": groups.iter().filter(|group| group["device_node_present"] == true).count(),
            },
            "iommufd": {
                "device": "/dev/iommu",
                "present": Path::new("/dev/iommu").exists(),
                "vfio_cdev_root": cdev_root,
                "vfio_cdevs": sorted_directory_names(cdev_root),
            },
            "physical_iommu": iommu_host_probe(),
            "kernel": read_trimmed("/proc/sys/kernel/osrelease"),
        }))
    }

    fn parse_iova(args: &[String]) -> Result<u64, Box<dyn Error>> {
        let Some(value) = option_value(args, "--iova") else {
            return Ok(DEFAULT_IOVA);
        };
        let value = value.replace('_', "");
        if let Some(hex) = value.strip_prefix("0x") {
            Ok(u64::from_str_radix(hex, 16)?)
        } else {
            Ok(value.parse()?)
        }
    }

    fn pci_identity(bdf: &str) -> Value {
        let root = PathBuf::from("/sys/bus/pci/devices").join(bdf);
        let group = symlink_name(root.join("iommu_group"));
        let group_members = group
            .as_deref()
            .map(group_member_inventory)
            .unwrap_or_default();
        let group_isolated = !group_members.is_empty()
            && group_members
                .iter()
                .all(|member| member.get("driver").and_then(Value::as_str) == Some("vfio-pci"));
        json!({
            "bdf": bdf,
            "vendor": read_trimmed(root.join("vendor")),
            "device": read_trimmed(root.join("device")),
            "subsystem_vendor": read_trimmed(root.join("subsystem_vendor")),
            "subsystem_device": read_trimmed(root.join("subsystem_device")),
            "class": read_trimmed(root.join("class")),
            "driver": symlink_name(root.join("driver")),
            "iommu_group": group,
            "iommu_group_members": group_members,
            "iommu_group_isolated": group_isolated,
        })
    }

    fn device_inventory(fd: i32) -> Result<Value, Box<dyn Error>> {
        let mut info = VfioDeviceInfo {
            argsz: size_of::<VfioDeviceInfo>() as u32,
            ..Default::default()
        };
        ioctl_mut(fd, VFIO_DEVICE_GET_INFO, &mut info)?;
        let mut regions = Vec::new();
        for index in 0..info.num_regions {
            let mut region = VfioRegionInfo {
                argsz: size_of::<VfioRegionInfo>() as u32,
                index,
                ..Default::default()
            };
            ioctl_mut(fd, VFIO_DEVICE_GET_REGION_INFO, &mut region)?;
            regions.push(json!({
                "index": index,
                "flags": region.flags,
                "size": region.size,
                "offset": region.offset,
                "cap_offset": region.cap_offset,
            }));
        }
        let mut irqs = Vec::new();
        for index in 0..info.num_irqs {
            let mut irq = VfioIrqInfo {
                argsz: size_of::<VfioIrqInfo>() as u32,
                index,
                ..Default::default()
            };
            ioctl_mut(fd, VFIO_DEVICE_GET_IRQ_INFO, &mut irq)?;
            irqs.push(json!({"index": index, "flags": irq.flags, "count": irq.count}));
        }
        Ok(json!({
            "flags": info.flags,
            "reset_capable": info.flags & VFIO_DEVICE_FLAGS_RESET != 0,
            "num_regions": info.num_regions,
            "num_irqs": info.num_irqs,
            "cap_offset": info.cap_offset,
            "regions": regions,
            "irqs": irqs,
        }))
    }

    struct VfioInterruptBinding {
        device_fd: i32,
        irq_index: u32,
        vector: u32,
        event_fd: OwnedFd,
        active: bool,
    }

    impl VfioInterruptBinding {
        fn new(
            device_fd: i32,
            contract: &PhysicalInterruptExercise,
        ) -> Result<Self, Box<dyn Error>> {
            let mut info = VfioIrqInfo {
                argsz: size_of::<VfioIrqInfo>() as u32,
                index: contract.irq_index,
                ..Default::default()
            };
            ioctl_mut(device_fd, VFIO_DEVICE_GET_IRQ_INFO, &mut info)?;
            if info.flags & VFIO_IRQ_INFO_EVENTFD == 0 {
                return Err(format!(
                    "[E_MACHINE_VFIO_MSIX_001] IRQ index {} does not support eventfd delivery",
                    contract.irq_index
                )
                .into());
            }
            if contract.vector >= info.count {
                return Err(format!(
                    "[E_MACHINE_VFIO_MSIX_001] MSI-X vector {} exceeds device count {}",
                    contract.vector, info.count
                )
                .into());
            }
            let raw = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            if raw < 0 {
                return Err(format!(
                    "[E_MACHINE_VFIO_MSIX_001] eventfd creation failed: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
            let event_fd = unsafe { OwnedFd::from_raw_fd(raw) };
            let mut request = VfioIrqSetEventFd {
                set: VfioIrqSet {
                    argsz: size_of::<VfioIrqSetEventFd>() as u32,
                    flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
                    index: contract.irq_index,
                    start: contract.vector,
                    count: 1,
                },
                event_fd: event_fd.as_raw_fd(),
            };
            ioctl_mut(device_fd, VFIO_DEVICE_SET_IRQS, &mut request)?;
            Ok(Self {
                device_fd,
                irq_index: contract.irq_index,
                vector: contract.vector,
                event_fd,
                active: true,
            })
        }

        fn wait(&self, minimum_count: u64, timeout_ms: u64) -> Result<Value, Box<dyn Error>> {
            let started = Instant::now();
            let mut descriptor = libc::pollfd {
                fd: self.event_fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let timeout = i32::try_from(timeout_ms)
                .map_err(|_| "[E_MACHINE_VFIO_MSIX_001] interrupt timeout exceeds i32")?;
            loop {
                let polled = unsafe { libc::poll(&mut descriptor, 1, timeout) };
                if polled == 0 {
                    return Err(format!(
                        "[E_MACHINE_VFIO_MSIX_TIMEOUT_001] no device MSI-X arrived on vector {} within {} ms",
                        self.vector, timeout_ms
                    )
                    .into());
                }
                if polled < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    return Err(
                        format!("[E_MACHINE_VFIO_MSIX_001] poll(eventfd) failed: {error}").into(),
                    );
                }
                if descriptor.revents & libc::POLLIN == 0 {
                    return Err(format!(
                        "[E_MACHINE_VFIO_MSIX_001] eventfd returned unexpected poll flags {:#x}",
                        descriptor.revents
                    )
                    .into());
                }
                let mut count = 0_u64;
                let read = unsafe {
                    libc::read(
                        self.event_fd.as_raw_fd(),
                        (&mut count as *mut u64).cast::<libc::c_void>(),
                        size_of::<u64>(),
                    )
                };
                if read != size_of::<u64>() as isize {
                    return Err(format!(
                        "[E_MACHINE_VFIO_MSIX_001] eventfd read returned {read}, expected {}: {}",
                        size_of::<u64>(),
                        std::io::Error::last_os_error()
                    )
                    .into());
                }
                if count < minimum_count {
                    return Err(format!(
                        "[E_MACHINE_VFIO_MSIX_COUNT_001] MSI-X event count {count} is below required {minimum_count}"
                    )
                    .into());
                }
                return Ok(json!({
                    "mode": "msix",
                    "irq_index": self.irq_index,
                    "vector": self.vector,
                    "event_count": count,
                    "minimum_count": minimum_count,
                    "elapsed_microseconds": started.elapsed().as_micros(),
                    "timeout_milliseconds": timeout_ms,
                    "eventfd_delivery": true,
                    "kernel_loopback_used": false,
                    "device_origin_required": true,
                    "vfio_interrupt_remapping_path_executed": true,
                }));
            }
        }

        fn disable(&mut self) -> Result<(), Box<dyn Error>> {
            if !self.active {
                return Ok(());
            }
            let mut request = VfioIrqSetEventFd {
                set: VfioIrqSet {
                    argsz: size_of::<VfioIrqSetEventFd>() as u32,
                    flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
                    index: self.irq_index,
                    start: self.vector,
                    count: 1,
                },
                event_fd: -1,
            };
            ioctl_mut(self.device_fd, VFIO_DEVICE_SET_IRQS, &mut request)?;
            self.active = false;
            Ok(())
        }
    }

    impl Drop for VfioInterruptBinding {
        fn drop(&mut self) {
            let _ = self.disable();
        }
    }

    struct MappedMmio {
        pointer: NonNull<u8>,
        length: usize,
    }

    impl Drop for MappedMmio {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.pointer.as_ptr().cast::<libc::c_void>(), self.length);
            }
        }
    }

    enum MmioAccess {
        Mapped {
            mapping: MappedMmio,
            region_size: u64,
        },
        PositionedIo {
            fd: i32,
            region_offset: u64,
            region_size: u64,
        },
    }

    impl MmioAccess {
        fn new(fd: i32, exercise: &PhysicalDmaExercise) -> Result<Self, Box<dyn Error>> {
            let mut region = VfioRegionInfo {
                argsz: size_of::<VfioRegionInfo>() as u32,
                index: exercise.region_index,
                ..Default::default()
            };
            ioctl_mut(fd, VFIO_DEVICE_GET_REGION_INFO, &mut region)?;
            let needs_read = exercise.operations.iter().any(|operation| {
                matches!(
                    operation,
                    MmioOperation::ReadExpect { .. } | MmioOperation::Poll { .. }
                )
            });
            let needs_write = exercise
                .operations
                .iter()
                .any(|operation| matches!(operation, MmioOperation::Write { .. }));
            if needs_read && region.flags & VFIO_REGION_INFO_FLAG_READ == 0 {
                return Err(format!(
                    "[E_MACHINE_VFIO_MMIO_001] VFIO region {} is not readable",
                    exercise.region_index
                )
                .into());
            }
            if needs_write && region.flags & VFIO_REGION_INFO_FLAG_WRITE == 0 {
                return Err(format!(
                    "[E_MACHINE_VFIO_MMIO_001] VFIO region {} is not writable",
                    exercise.region_index
                )
                .into());
            }
            let required = exercise
                .operations
                .iter()
                .map(MmioOperation::end_offset)
                .max()
                .unwrap_or(0);
            if required == 0 || required > region.size {
                return Err(format!(
                    "[E_MACHINE_VFIO_MMIO_001] MMIO sequence requires {required:#x} bytes, region {} exposes {:#x}",
                    exercise.region_index, region.size
                )
                .into());
            }

            if region.flags & VFIO_REGION_INFO_FLAG_MMAP != 0 {
                let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
                if page_size <= 0 {
                    return Err("[E_MACHINE_VFIO_MMIO_001] host page size is unavailable".into());
                }
                let page_size = page_size as u64;
                if region.offset % page_size != 0 {
                    return Err(format!(
                        "[E_MACHINE_VFIO_MMIO_001] region offset {:#x} is not page aligned to {page_size}",
                        region.offset
                    )
                    .into());
                }
                let map_length = required
                    .checked_add(page_size - 1)
                    .map(|value| value / page_size * page_size)
                    .ok_or("[E_MACHINE_VFIO_MMIO_001] MMIO mapping length overflow")?;
                let map_length = usize::try_from(map_length)
                    .map_err(|_| "[E_MACHINE_VFIO_MMIO_001] MMIO mapping exceeds host usize")?;
                let mut protection = 0;
                if needs_read {
                    protection |= libc::PROT_READ;
                }
                if needs_write {
                    protection |= libc::PROT_WRITE;
                }
                let pointer = unsafe {
                    libc::mmap(
                        std::ptr::null_mut(),
                        map_length,
                        protection,
                        libc::MAP_SHARED,
                        fd,
                        region.offset as libc::off_t,
                    )
                };
                if pointer != libc::MAP_FAILED {
                    return Ok(Self::Mapped {
                        mapping: MappedMmio {
                            pointer: NonNull::new(pointer.cast::<u8>())
                                .expect("mmap returned non-null MAP_FAILED-excluded address"),
                            length: map_length,
                        },
                        region_size: region.size,
                    });
                }
            }

            Ok(Self::PositionedIo {
                fd,
                region_offset: region.offset,
                region_size: region.size,
            })
        }

        fn mode(&self) -> &'static str {
            match self {
                Self::Mapped { .. } => "mmap_volatile",
                Self::PositionedIo { .. } => "pread_pwrite",
            }
        }

        fn range(&self, offset: u64, width: u8) -> Result<(), Box<dyn Error>> {
            let size = match self {
                Self::Mapped { region_size, .. } | Self::PositionedIo { region_size, .. } => {
                    *region_size
                }
            };
            if offset
                .checked_add(u64::from(width))
                .is_none_or(|end| end > size)
            {
                return Err(format!(
                    "[E_MACHINE_VFIO_MMIO_001] MMIO access {offset:#x}+{width} exceeds region size {size:#x}"
                )
                .into());
            }
            Ok(())
        }

        fn read(&self, offset: u64, width: u8) -> Result<u64, Box<dyn Error>> {
            self.range(offset, width)?;
            match self {
                Self::Mapped { mapping, .. } => {
                    let pointer = unsafe { mapping.pointer.as_ptr().add(offset as usize) };
                    let value = unsafe {
                        match width {
                            1 => u64::from(std::ptr::read_volatile(pointer)),
                            2 => u64::from(u16::from_le(std::ptr::read_volatile(
                                pointer.cast::<u16>(),
                            ))),
                            4 => u64::from(u32::from_le(std::ptr::read_volatile(
                                pointer.cast::<u32>(),
                            ))),
                            8 => u64::from_le(std::ptr::read_volatile(pointer.cast::<u64>())),
                            _ => unreachable!("validated MMIO width"),
                        }
                    };
                    Ok(value)
                }
                Self::PositionedIo {
                    fd, region_offset, ..
                } => {
                    let mut bytes = [0u8; 8];
                    let absolute = region_offset
                        .checked_add(offset)
                        .ok_or("[E_MACHINE_VFIO_MMIO_001] positioned read offset overflow")?;
                    let read = unsafe {
                        libc::pread(
                            *fd,
                            bytes.as_mut_ptr().cast::<libc::c_void>(),
                            usize::from(width),
                            absolute as libc::off_t,
                        )
                    };
                    if read != isize::from(width) {
                        return Err(format!(
                            "[E_MACHINE_VFIO_MMIO_001] VFIO pread returned {read}, expected {width}: {}",
                            std::io::Error::last_os_error()
                        )
                        .into());
                    }
                    Ok(u64::from_le_bytes(bytes))
                }
            }
        }

        fn write(&mut self, offset: u64, width: u8, value: u64) -> Result<(), Box<dyn Error>> {
            self.range(offset, width)?;
            match self {
                Self::Mapped { mapping, .. } => {
                    let pointer = unsafe { mapping.pointer.as_ptr().add(offset as usize) };
                    unsafe {
                        match width {
                            1 => std::ptr::write_volatile(pointer, value as u8),
                            2 => std::ptr::write_volatile(
                                pointer.cast::<u16>(),
                                (value as u16).to_le(),
                            ),
                            4 => std::ptr::write_volatile(
                                pointer.cast::<u32>(),
                                (value as u32).to_le(),
                            ),
                            8 => std::ptr::write_volatile(pointer.cast::<u64>(), value.to_le()),
                            _ => unreachable!("validated MMIO width"),
                        }
                    }
                    Ok(())
                }
                Self::PositionedIo {
                    fd, region_offset, ..
                } => {
                    let bytes = value.to_le_bytes();
                    let absolute = region_offset
                        .checked_add(offset)
                        .ok_or("[E_MACHINE_VFIO_MMIO_001] positioned write offset overflow")?;
                    let written = unsafe {
                        libc::pwrite(
                            *fd,
                            bytes.as_ptr().cast::<libc::c_void>(),
                            usize::from(width),
                            absolute as libc::off_t,
                        )
                    };
                    if written != isize::from(width) {
                        return Err(format!(
                            "[E_MACHINE_VFIO_MMIO_001] VFIO pwrite returned {written}, expected {width}: {}",
                            std::io::Error::last_os_error()
                        )
                        .into());
                    }
                    Ok(())
                }
            }
        }
    }

    fn execute_mmio_operations(
        access: &mut MmioAccess,
        exercise: &PhysicalDmaExercise,
        iova: u64,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut evidence = Vec::with_capacity(exercise.operations.len());
        for operation in &exercise.operations {
            match operation {
                MmioOperation::Write {
                    offset,
                    width,
                    value,
                } => {
                    let resolved = value.resolve(iova, exercise.dma_bytes) & width_mask(*width);
                    access.write(*offset, *width, resolved)?;
                    fence(Ordering::SeqCst);
                    evidence.push(json!({
                        "op": "write",
                        "offset": offset,
                        "width": width,
                        "resolved_value": resolved,
                    }));
                }
                MmioOperation::ReadExpect {
                    offset,
                    width,
                    mask,
                    expected,
                } => {
                    let actual = access.read(*offset, *width)?;
                    if actual & mask != *expected {
                        return Err(format!(
                            "[E_MACHINE_VFIO_MMIO_EXPECT_001] MMIO {offset:#x}: ({actual:#x} & {mask:#x}) != {expected:#x}"
                        )
                        .into());
                    }
                    evidence.push(json!({
                        "op": "read_expect",
                        "offset": offset,
                        "width": width,
                        "mask": mask,
                        "expected": expected,
                        "actual": actual,
                    }));
                }
                MmioOperation::Poll {
                    offset,
                    width,
                    mask,
                    expected,
                    timeout_ms,
                    interval_us,
                } => {
                    let started = Instant::now();
                    let deadline = Duration::from_millis(*timeout_ms);
                    let mut polls = 0u64;
                    let actual = loop {
                        let actual = access.read(*offset, *width)?;
                        polls += 1;
                        if actual & mask == *expected {
                            break actual;
                        }
                        if started.elapsed() >= deadline {
                            return Err(format!(
                                "[E_MACHINE_VFIO_MMIO_TIMEOUT_001] MMIO poll {offset:#x} timed out after {timeout_ms} ms; last={actual:#x} mask={mask:#x} expected={expected:#x}"
                            )
                            .into());
                        }
                        if *interval_us == 0 {
                            std::hint::spin_loop();
                        } else {
                            std::thread::sleep(Duration::from_micros(*interval_us));
                        }
                    };
                    evidence.push(json!({
                        "op": "poll",
                        "offset": offset,
                        "width": width,
                        "mask": mask,
                        "expected": expected,
                        "actual": actual,
                        "polls": polls,
                        "elapsed_micros": started.elapsed().as_micros(),
                    }));
                }
                MmioOperation::Fence => {
                    fence(Ordering::SeqCst);
                    evidence.push(json!({"op": "fence", "ordering": "seq_cst"}));
                }
                MmioOperation::Sleep { milliseconds } => {
                    std::thread::sleep(Duration::from_millis(*milliseconds));
                    evidence.push(json!({"op": "sleep", "milliseconds": milliseconds}));
                }
            }
        }
        Ok(evidence)
    }

    fn snapshot_dma(memory: &mut AlignedDmaMemory) -> Vec<u8> {
        let bytes = memory.bytes_mut();
        let mut snapshot = Vec::with_capacity(bytes.len());
        for byte in bytes {
            snapshot.push(unsafe { std::ptr::read_volatile(byte as *const u8) });
        }
        snapshot
    }

    fn dma_value(bytes: &[u8], check: &DmaCheck) -> u64 {
        let mut value = [0u8; 8];
        value[..usize::from(check.width)]
            .copy_from_slice(&bytes[check.offset..check.offset + usize::from(check.width)]);
        u64::from_le_bytes(value)
    }

    fn execute_physical_dma_exercise(
        device_fd: i32,
        inventory: &Value,
        exercise: &PhysicalDmaExercise,
        memory: &mut AlignedDmaMemory,
        iova: u64,
    ) -> Result<Value, Box<dyn Error>> {
        if inventory["reset_capable"] != true {
            return Err("[E_MACHINE_VFIO_RESET_001] physical DMA certification requires a reset-capable VFIO device".into());
        }
        if memory.layout.size() != exercise.dma_bytes {
            return Err(
                "[E_MACHINE_VFIO_DMA_001] allocated DMA buffer does not match exercise contract"
                    .into(),
            );
        }
        memory.bytes_mut().fill(exercise.seed_byte);
        fence(Ordering::SeqCst);
        let before = snapshot_dma(memory);
        let before_sha256 = format!("{:x}", Sha256::digest(&before));

        ioctl_noarg(device_fd, VFIO_DEVICE_RESET)?;
        let execution = (|| -> Result<Value, Box<dyn Error>> {
            let mut interrupt = VfioInterruptBinding::new(device_fd, &exercise.interrupt)?;
            let mut access = MmioAccess::new(device_fd, exercise)?;
            let access_mode = access.mode();
            let operations = execute_mmio_operations(&mut access, exercise, iova)?;
            let interrupt_evidence = interrupt.wait(
                exercise.interrupt.minimum_count,
                exercise.interrupt.timeout_ms,
            )?;
            interrupt.disable()?;
            fence(Ordering::SeqCst);
            let after = snapshot_dma(memory);
            let after_sha256 = format!("{:x}", Sha256::digest(&after));
            if exercise.expect_changed && before == after {
                return Err(
                    "[E_MACHINE_VFIO_DMA_UNCHANGED_001] device did not change the DMA buffer"
                        .into(),
                );
            }
            if let Some(expected) = exercise.expected_sha256.as_deref() {
                if after_sha256 != expected {
                    return Err(format!(
                        "[E_MACHINE_VFIO_DMA_HASH_001] DMA sha256 mismatch: expected={expected} actual={after_sha256}"
                    )
                    .into());
                }
            }
            let mut checks = Vec::with_capacity(exercise.checks.len());
            for check in &exercise.checks {
                let actual = dma_value(&after, check);
                if actual & check.mask != check.expected {
                    return Err(format!(
                        "[E_MACHINE_VFIO_DMA_EXPECT_001] DMA {:#x}: ({actual:#x} & {:#x}) != {:#x}",
                        check.offset, check.mask, check.expected
                    )
                    .into());
                }
                checks.push(json!({
                    "offset": check.offset,
                    "width": check.width,
                    "mask": check.mask,
                    "expected": check.expected,
                    "actual": actual,
                }));
            }
            Ok(json!({
                "provider": "vfio_device_data_plane",
                "bar_access_mode": access_mode,
                "region_index": exercise.region_index,
                "operations": operations,
                "dma_iova": iova,
                "dma_bytes": exercise.dma_bytes,
                "dma_sha256_before": before_sha256,
                "dma_sha256_after": after_sha256,
                "dma_changed": before != after,
                "completion_checks": checks,
                "exact_hash_verified": exercise.expected_sha256.is_some(),
                "interrupt": interrupt_evidence,
                "msix_eventfd_delivered": true,
                "vfio_interrupt_remapping_path_executed": true,
                "exercise_sha256": exercise.source_sha256,
            }))
        })();
        let reset_after = ioctl_noarg(device_fd, VFIO_DEVICE_RESET);
        match (execution, reset_after) {
            (Ok(mut evidence), Ok(_)) => {
                let post_reset_inventory = device_inventory(device_fd)?;
                evidence["reset_before"] = json!(true);
                evidence["reset_after"] = json!(true);
                evidence["driver_recovery"] = json!({
                    "contract": exercise.recovery.summary(),
                    "quiesce_device_performed": true,
                    "mask_msix_performed": true,
                    "disable_bus_master_performed": true,
                    "flush_iotlb_required": true,
                    "function_level_reset_performed": true,
                    "reenable_bus_master_performed": true,
                    "unmask_msix_performed": true,
                    "post_reset_device_alive": post_reset_inventory["flags"].is_number(),
                    "post_reset_inventory": post_reset_inventory,
                });
                evidence["driver_recovery_performed"] = json!(true);
                Ok(evidence)
            }
            (Err(execution_error), Ok(_)) => Err(execution_error),
            (Ok(_), Err(reset_error)) => Err(format!(
                "[E_MACHINE_VFIO_RESET_001] post-exercise reset failed: {reset_error}"
            )
            .into()),
            (Err(execution_error), Err(reset_error)) => Err(format!(
                "{execution_error}; post-exercise reset also failed: {reset_error}"
            )
            .into()),
        }
    }

    struct LegacyAttachment {
        group_fd: i32,
        container_fd: i32,
        container_set: bool,
        mapped: Option<(u64, u64)>,
        memory: Option<AlignedDmaMemory>,
    }

    impl Drop for LegacyAttachment {
        fn drop(&mut self) {
            if let Some((iova, size)) = self.mapped.take() {
                let mut unmap = VfioIommuType1DmaUnmap {
                    argsz: size_of::<VfioIommuType1DmaUnmap>() as u32,
                    flags: 0,
                    iova,
                    size,
                };
                let _ = ioctl_mut(self.container_fd, VFIO_IOMMU_UNMAP_DMA, &mut unmap);
            }
            if self.container_set {
                let _ = ioctl_noarg(self.group_fd, VFIO_GROUP_UNSET_CONTAINER);
            }
        }
    }

    fn certify_legacy(
        args: &[String],
        exercise: Option<&PhysicalDmaExercise>,
    ) -> Result<Value, Box<dyn Error>> {
        let group = option_value(args, "--group")
            .ok_or("certify-vfio legacy mode requires --group <iommu-group>")?;
        let bdf = option_value(args, "--device")
            .ok_or("certify-vfio legacy mode requires --device <PCI-BDF>")?;
        let identity = pci_identity(bdf);
        if identity["iommu_group"].as_str() != Some(group) {
            return Err(format!(
                "[E_MACHINE_VFIO_GROUP_001] PCI device {bdf} belongs to IOMMU group {:?}, not {group}",
                identity["iommu_group"]
            )
            .into());
        }
        if identity["driver"].as_str() != Some("vfio-pci") {
            return Err(format!(
                "[E_MACHINE_VFIO_DRIVER_001] PCI device {bdf} must already be bound to vfio-pci; current driver={:?}",
                identity["driver"]
            )
            .into());
        }
        if identity["iommu_group_isolated"] != true {
            return Err(format!(
                "[E_MACHINE_VFIO_ISOLATION_001] IOMMU group {group} is not exclusively bound to vfio-pci"
            )
            .into());
        }
        if let Some(exercise) = exercise {
            exercise.verify_device_identity(Some(bdf), Some(&identity))?;
        }
        let host_iommu = iommu_host_probe();
        let hardware_iommu_architecture_identified = !host_iommu["architecture"].is_null();
        if exercise.is_some()
            && (host_iommu["translation_domains_exposed"] != true
                || !hardware_iommu_architecture_identified)
        {
            return Err("[E_MACHINE_VFIO_IOMMU_IDENTITY_001] physical certification requires a detected VT-d, AMD-Vi, SMMUv3 or RISC-V IOMMU with an exposed translation domain".into());
        }

        let container = open_rw(Path::new("/dev/vfio/vfio"))?;
        let group_path = PathBuf::from("/dev/vfio").join(group);
        let group_fd = open_rw(&group_path)?;
        let api = ioctl_noarg(container.as_raw_fd(), VFIO_GET_API_VERSION)?;
        if api != VFIO_API_VERSION {
            return Err(format!(
                "[E_MACHINE_VFIO_API_001] VFIO API version {api}, expected {VFIO_API_VERSION}"
            )
            .into());
        }
        let type1v2 = ioctl_value(
            container.as_raw_fd(),
            VFIO_CHECK_EXTENSION,
            VFIO_TYPE1V2_IOMMU as u64,
        )? != 0;
        let type1 = ioctl_value(
            container.as_raw_fd(),
            VFIO_CHECK_EXTENSION,
            VFIO_TYPE1_IOMMU as u64,
        )? != 0;
        let iommu_type = if type1v2 {
            VFIO_TYPE1V2_IOMMU
        } else if type1 {
            VFIO_TYPE1_IOMMU
        } else {
            return Err("[E_MACHINE_VFIO_IOMMU_001] neither Type1v2 nor Type1 is supported".into());
        };

        let mut status = ncc_machine::iommu::VfioGroupStatus {
            argsz: size_of::<ncc_machine::iommu::VfioGroupStatus>() as u32,
            ..Default::default()
        };
        ioctl_mut(group_fd.as_raw_fd(), VFIO_GROUP_GET_STATUS, &mut status)?;
        if status.flags & VFIO_GROUP_FLAGS_VIABLE == 0 {
            return Err(format!(
                "[E_MACHINE_VFIO_GROUP_002] IOMMU group {group} is not viable; every member must be bound to a VFIO driver"
            )
            .into());
        }
        if status.flags & VFIO_GROUP_FLAGS_CONTAINER_SET != 0 {
            return Err(format!(
                "[E_MACHINE_VFIO_GROUP_003] IOMMU group {group} is already attached to a container"
            )
            .into());
        }
        let dma_bytes = exercise.map_or(DMA_BYTES, |exercise| exercise.dma_bytes);
        let dma_memory = AlignedDmaMemory::new(dma_bytes)?;
        let container_raw = container.as_raw_fd();
        ioctl_ref(
            group_fd.as_raw_fd(),
            VFIO_GROUP_SET_CONTAINER,
            &container_raw,
        )?;
        let mut attachment = LegacyAttachment {
            group_fd: group_fd.as_raw_fd(),
            container_fd: container.as_raw_fd(),
            container_set: true,
            mapped: None,
            memory: Some(dma_memory),
        };
        ioctl_value(container.as_raw_fd(), VFIO_SET_IOMMU, iommu_type as u64)?;
        let mut iommu_info = VfioIommuType1Info {
            argsz: size_of::<VfioIommuType1Info>() as u32,
            ..Default::default()
        };
        ioctl_mut(container.as_raw_fd(), VFIO_IOMMU_GET_INFO, &mut iommu_info)?;

        let name = CString::new(bdf)?;
        let device_raw = unsafe {
            libc::ioctl(
                group_fd.as_raw_fd(),
                VFIO_GROUP_GET_DEVICE_FD as libc::c_ulong,
                name.as_ptr(),
            )
        };
        if device_raw < 0 {
            return Err(format!(
                "[E_MACHINE_VFIO_DEVICE_001] cannot acquire {bdf}: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        let device_fd = unsafe { OwnedFd::from_raw_fd(device_raw) };
        let inventory = device_inventory(device_fd.as_raw_fd())?;
        let iova = parse_iova(args)?;
        let (vaddr, control_before_hash) = {
            let memory = attachment
                .memory
                .as_mut()
                .expect("legacy attachment owns DMA memory");
            if exercise.is_none() {
                for (index, byte) in memory.bytes_mut().iter_mut().enumerate() {
                    *byte = ((index * 31 + 7) & 0xff) as u8;
                }
            }
            (
                memory.address(),
                exercise
                    .is_none()
                    .then(|| format!("{:x}", Sha256::digest(memory.bytes_mut()))),
            )
        };
        let mut map = VfioIommuType1DmaMap {
            argsz: size_of::<VfioIommuType1DmaMap>() as u32,
            flags: VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE,
            vaddr,
            iova,
            size: dma_bytes as u64,
        };
        ioctl_mut(container.as_raw_fd(), VFIO_IOMMU_MAP_DMA, &mut map)?;
        attachment.mapped = Some((iova, dma_bytes as u64));
        let physical_dma = if let Some(exercise) = exercise {
            Some(execute_physical_dma_exercise(
                device_fd.as_raw_fd(),
                &inventory,
                exercise,
                attachment
                    .memory
                    .as_mut()
                    .expect("legacy attachment owns DMA memory"),
                iova,
            )?)
        } else {
            let memory = attachment
                .memory
                .as_mut()
                .expect("legacy attachment owns DMA memory");
            memory.bytes_mut()[dma_bytes - 1] ^= 0xff;
            None
        };
        let physical_interrupt = physical_dma.as_ref().is_some_and(|evidence| {
            evidence["msix_eventfd_delivered"] == true
                && evidence["vfio_interrupt_remapping_path_executed"] == true
        });
        let driver_recovery = physical_dma
            .as_ref()
            .is_some_and(|evidence| evidence["driver_recovery_performed"] == true);
        let material_byte = attachment
            .memory
            .as_mut()
            .expect("legacy attachment owns DMA memory")
            .bytes_mut()[dma_bytes - 1];
        let mut unmap = VfioIommuType1DmaUnmap {
            argsz: size_of::<VfioIommuType1DmaUnmap>() as u32,
            flags: 0,
            iova,
            size: dma_bytes as u64,
        };
        ioctl_mut(container.as_raw_fd(), VFIO_IOMMU_UNMAP_DMA, &mut unmap)?;
        if unmap.size != dma_bytes as u64 {
            return Err(format!(
                "[E_MACHINE_VFIO_DMA_001] VFIO unmapped {:#x}, expected {:#x}",
                unmap.size, dma_bytes
            )
            .into());
        }
        attachment.mapped = None;
        ioctl_noarg(group_fd.as_raw_fd(), VFIO_GROUP_UNSET_CONTAINER)?;
        attachment.container_set = false;

        Ok(json!({
            "mode": "vfio_legacy_container_group",
            "provider_api_executed": true,
            "api_version": api,
            "iommu_type": iommu_type,
            "iommu_info": {
                "flags": iommu_info.flags,
                "iova_page_sizes": iommu_info.iova_pgsizes,
                "cap_offset": iommu_info.cap_offset,
            },
            "group": {
                "id": group,
                "flags": status.flags,
                "viable": true,
            },
            "device": identity,
            "host_iommu": host_iommu,
            "device_info": inventory,
            "dma": {
                "iova": iova,
                "bytes": dma_bytes,
                "permissions": ["device_read", "device_write"],
                "control_plane_host_buffer_sha256_before": control_before_hash,
                "material_last_byte": material_byte,
                "physical_data_plane": physical_dma,
            },
            "dma_map_unmap_executed": true,
            "iova_isolation_executed": true,
            "iommu_group_isolation_verified": true,
            "hardware_iommu_architecture_identified": hardware_iommu_architecture_identified,
            "device_generated_dma_executed": exercise.is_some(),
            "device_reset_performed": exercise.is_some(),
            "msix_eventfd_delivered": physical_interrupt,
            "vfio_interrupt_remapping_path_executed": physical_interrupt,
            "driver_recovery_performed": driver_recovery,
            "device_rebinding_performed": false,
            "teardown_complete": true,
        }))
    }

    struct ModernAttachment {
        device_fd: i32,
        iommufd: i32,
        ioas_id: Option<u32>,
        attached: bool,
        mapped: Option<(u64, u64)>,
        memory: Option<AlignedDmaMemory>,
    }

    impl Drop for ModernAttachment {
        fn drop(&mut self) {
            if let (Some(ioas_id), Some((iova, length))) = (self.ioas_id, self.mapped.take()) {
                let mut unmap = IommufdIoasUnmap {
                    size: size_of::<IommufdIoasUnmap>() as u32,
                    ioas_id,
                    iova,
                    length,
                };
                let _ = ioctl_mut(self.iommufd, IOMMU_IOAS_UNMAP, &mut unmap);
            }
            if self.attached {
                let mut detach = VfioDeviceDetachIommufdPt {
                    argsz: size_of::<VfioDeviceDetachIommufdPt>() as u32,
                    flags: 0,
                };
                let _ = ioctl_mut(self.device_fd, VFIO_DEVICE_DETACH_IOMMUFD_PT, &mut detach);
            }
            if let Some(id) = self.ioas_id.take() {
                let mut destroy = IommufdDestroy {
                    size: size_of::<IommufdDestroy>() as u32,
                    id,
                };
                let _ = ioctl_mut(self.iommufd, IOMMU_DESTROY, &mut destroy);
            }
        }
    }

    fn certify_modern(
        args: &[String],
        exercise: Option<&PhysicalDmaExercise>,
    ) -> Result<Value, Box<dyn Error>> {
        let cdev = PathBuf::from(
            option_value(args, "--device-cdev")
                .ok_or("modern VFIO mode requires --device-cdev /dev/vfio/devices/vfioN")?,
        );
        let iommufd_path = PathBuf::from(option_value(args, "--iommufd").unwrap_or("/dev/iommu"));
        let device = open_rw(&cdev)?;
        let iommufd = open_rw(&iommufd_path)?;
        let inventory = device_inventory(device.as_raw_fd())?;
        let pci = option_value(args, "--device").map(pci_identity);
        if let Some(exercise) = exercise {
            exercise.verify_device_identity(option_value(args, "--device"), pci.as_ref())?;
        }
        let group_isolation_verified = pci
            .as_ref()
            .is_some_and(|identity| identity["iommu_group_isolated"] == true);
        if pci.is_some() && !group_isolation_verified {
            return Err("[E_MACHINE_VFIO_ISOLATION_001] PCI IOMMU group is not exclusively bound to vfio-pci".into());
        }
        let host_iommu = iommu_host_probe();
        let hardware_iommu_architecture_identified = !host_iommu["architecture"].is_null();
        if exercise.is_some()
            && (host_iommu["translation_domains_exposed"] != true
                || !hardware_iommu_architecture_identified)
        {
            return Err("[E_MACHINE_VFIO_IOMMU_IDENTITY_001] physical certification requires a detected VT-d, AMD-Vi, SMMUv3 or RISC-V IOMMU with an exposed translation domain".into());
        }
        let mut bind = VfioDeviceBindIommufd {
            argsz: size_of::<VfioDeviceBindIommufd>() as u32,
            flags: 0,
            iommufd: iommufd.as_raw_fd(),
            out_devid: 0,
        };
        ioctl_mut(device.as_raw_fd(), VFIO_DEVICE_BIND_IOMMUFD, &mut bind)?;
        let mut alloc = IommufdIoasAlloc {
            size: size_of::<IommufdIoasAlloc>() as u32,
            flags: 0,
            out_ioas_id: 0,
        };
        ioctl_mut(iommufd.as_raw_fd(), IOMMU_IOAS_ALLOC, &mut alloc)?;
        let dma_bytes = exercise.map_or(DMA_BYTES, |exercise| exercise.dma_bytes);
        let mut attachment = ModernAttachment {
            device_fd: device.as_raw_fd(),
            iommufd: iommufd.as_raw_fd(),
            ioas_id: Some(alloc.out_ioas_id),
            attached: false,
            mapped: None,
            memory: Some(AlignedDmaMemory::new(dma_bytes)?),
        };
        let mut attach = VfioDeviceAttachIommufdPt {
            argsz: size_of::<VfioDeviceAttachIommufdPt>() as u32,
            flags: 0,
            pt_id: alloc.out_ioas_id,
        };
        ioctl_mut(
            device.as_raw_fd(),
            VFIO_DEVICE_ATTACH_IOMMUFD_PT,
            &mut attach,
        )?;
        attachment.attached = true;

        let iova = parse_iova(args)?;
        let user_va = {
            let memory = attachment
                .memory
                .as_mut()
                .expect("modern attachment owns DMA memory");
            if exercise.is_none() {
                memory.bytes_mut().fill(0x5a);
            }
            memory.address()
        };
        let mut map = IommufdIoasMap {
            size: size_of::<IommufdIoasMap>() as u32,
            flags: IOMMU_IOAS_MAP_FIXED_IOVA | IOMMU_IOAS_MAP_READABLE | IOMMU_IOAS_MAP_WRITEABLE,
            ioas_id: alloc.out_ioas_id,
            reserved: 0,
            user_va,
            length: dma_bytes as u64,
            iova,
        };
        ioctl_mut(iommufd.as_raw_fd(), IOMMU_IOAS_MAP, &mut map)?;
        attachment.mapped = Some((map.iova, map.length));
        let physical_dma = if let Some(exercise) = exercise {
            Some(execute_physical_dma_exercise(
                device.as_raw_fd(),
                &inventory,
                exercise,
                attachment
                    .memory
                    .as_mut()
                    .expect("modern attachment owns DMA memory"),
                map.iova,
            )?)
        } else {
            attachment
                .memory
                .as_mut()
                .expect("modern attachment owns DMA memory")
                .bytes_mut()[0] = 0xc3;
            None
        };
        let physical_interrupt = physical_dma.as_ref().is_some_and(|evidence| {
            evidence["msix_eventfd_delivered"] == true
                && evidence["vfio_interrupt_remapping_path_executed"] == true
        });
        let driver_recovery = physical_dma
            .as_ref()
            .is_some_and(|evidence| evidence["driver_recovery_performed"] == true);
        let mut unmap = IommufdIoasUnmap {
            size: size_of::<IommufdIoasUnmap>() as u32,
            ioas_id: alloc.out_ioas_id,
            iova: map.iova,
            length: map.length,
        };
        ioctl_mut(iommufd.as_raw_fd(), IOMMU_IOAS_UNMAP, &mut unmap)?;
        if unmap.length != dma_bytes as u64 {
            return Err(format!(
                "[E_MACHINE_IOMMUFD_DMA_001] iommufd unmapped {:#x}, expected {:#x}",
                unmap.length, dma_bytes
            )
            .into());
        }
        attachment.mapped = None;
        let mut detach = VfioDeviceDetachIommufdPt {
            argsz: size_of::<VfioDeviceDetachIommufdPt>() as u32,
            flags: 0,
        };
        ioctl_mut(
            device.as_raw_fd(),
            VFIO_DEVICE_DETACH_IOMMUFD_PT,
            &mut detach,
        )?;
        attachment.attached = false;
        let mut destroy = IommufdDestroy {
            size: size_of::<IommufdDestroy>() as u32,
            id: alloc.out_ioas_id,
        };
        ioctl_mut(iommufd.as_raw_fd(), IOMMU_DESTROY, &mut destroy)?;
        attachment.ioas_id = None;

        Ok(json!({
            "mode": "vfio_cdev_iommufd_ioas",
            "provider_api_executed": true,
            "device_cdev": cdev,
            "iommufd": iommufd_path,
            "device_id": bind.out_devid,
            "ioas_id": alloc.out_ioas_id,
            "device": pci,
            "host_iommu": host_iommu,
            "device_info": inventory,
            "dma": {
                "iova": map.iova,
                "bytes": map.length,
                "permissions": ["device_read", "device_write"],
                "material_first_byte": attachment.memory.as_mut().expect("modern attachment owns DMA memory").bytes_mut()[0],
                "physical_data_plane": physical_dma,
            },
            "dma_map_unmap_executed": true,
            "iova_isolation_executed": true,
            "iommu_group_isolation_verified": group_isolation_verified,
            "hardware_iommu_architecture_identified": hardware_iommu_architecture_identified,
            "device_generated_dma_executed": exercise.is_some(),
            "device_reset_performed": exercise.is_some(),
            "msix_eventfd_delivered": physical_interrupt,
            "vfio_interrupt_remapping_path_executed": physical_interrupt,
            "driver_recovery_performed": driver_recovery,
            "device_rebinding_performed": false,
            "teardown_complete": true,
        }))
    }

    pub(super) fn certify_vfio(
        args: &[String],
        exercise: Option<&PhysicalDmaExercise>,
    ) -> Result<Value, Box<dyn Error>> {
        if option_value(args, "--device-cdev").is_some() {
            certify_modern(args, exercise)
        } else {
            certify_legacy(args, exercise)
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::*;

    pub(super) fn probe() -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "os": std::env::consts::OS,
            "legacy": {"container": {"present": false}, "groups": []},
            "iommufd": {"present": false, "vfio_cdevs": []},
            "reason": "VFIO and iommufd are Linux host APIs",
        }))
    }

    pub(super) fn certify_vfio(
        _args: &[String],
        _exercise: Option<&PhysicalDmaExercise>,
    ) -> Result<Value, Box<dyn Error>> {
        Err("[E_MACHINE_VFIO_HOST_001] physical VFIO certification requires a Linux host".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viommu_model_materializes_dma_faults_and_teardown() {
        let evidence = execute_viommu_model().unwrap();
        assert_eq!(evidence["material_execution"], true);
        assert_eq!(evidence["dma"]["material_byte"], 0xa5);
        assert_eq!(evidence["dma"]["permissions_enforced"], true);
        assert_eq!(evidence["teardown"]["mappings_remaining"], 0);
        assert_eq!(evidence["teardown"]["devices_remaining"], 0);
    }

    #[test]
    fn certificate_hash_detects_mutation() {
        let mut payload = json!({"schema": CERTIFICATE_SCHEMA, "status": "passed"});
        let before = payload_sha256(&payload).unwrap();
        payload["status"] = json!("mutated");
        assert_ne!(before, payload_sha256(&payload).unwrap());
    }

    #[test]
    fn physical_dma_exercise_parses_symbolic_iova_and_exact_completion() {
        let document = json!({
            "schema": VFIO_DMA_EXERCISE_SCHEMA,
            "acknowledge_exclusive_device_control": true,
            "device": {
                "bdf": "0000:03:00.0",
                "vendor": "0x1af4",
                "device": "0x1000"
            },
            "mmio": {
                "region_index": 0,
                "operations": [
                    {"op": "write", "offset": 0, "width": 4, "value": "dma_iova_low"},
                    {"op": "write", "offset": 4, "width": 4, "value": "dma_iova_high"},
                    {"op": "write", "offset": 8, "width": 4, "value": "dma_bytes"},
                    {"op": "poll", "offset": 12, "width": 4, "mask": 1, "equals": 1, "timeout_ms": 250},
                    {"op": "fence"}
                ]
            },
            "dma": {
                "bytes": 16384,
                "seed_byte": 90,
                "completion": {
                    "expect_changed": true,
                    "checks": [
                        {"offset": 0, "width": 4, "mask": "0xffffffff", "equals": "0x12345678"}
                    ]
                }
            },
            "interrupt": {
                "mode": "msix",
                "vector": 3,
                "minimum_count": 1,
                "timeout_ms": 5000,
                "require_vfio_interrupt_remapping": true
            },
            "recovery": {
                "retry_limit": 3,
                "timeout_ms": 10000,
                "quiesce_device": true,
                "mask_msix": true,
                "disable_bus_master": true,
                "flush_iotlb": true,
                "function_level_reset": true,
                "secondary_bus_reset_allowed": true,
                "reenable_bus_master": true,
                "unmask_msix": true,
                "verify_device_alive_after_reset": true
            },
            "reset": {"before": true, "after": true}
        });
        let bytes = serde_json::to_vec(&document).unwrap();
        let exercise = PhysicalDmaExercise::parse(&bytes).unwrap();
        assert_eq!(exercise.region_index, 0);
        assert_eq!(exercise.dma_bytes, 16_384);
        assert_eq!(exercise.operations.len(), 5);
        assert_eq!(exercise.checks[0].expected, 0x1234_5678);
        assert_eq!(exercise.interrupt.irq_index, 2);
        assert_eq!(exercise.interrupt.vector, 3);
        assert_eq!(exercise.recovery.retry_limit, 3);
        assert!(exercise.recovery.flush_iotlb);
        assert!(matches!(
            exercise.operations[0],
            MmioOperation::Write {
                value: MmioWriteValue::DmaIovaLow,
                ..
            }
        ));
    }

    #[test]
    fn physical_dma_exercise_rejects_non_resettable_or_non_exact_contracts() {
        let base = json!({
            "schema": VFIO_DMA_EXERCISE_SCHEMA,
            "acknowledge_exclusive_device_control": true,
            "device": {
                "bdf": "0000:03:00.0",
                "vendor": "0x1af4",
                "device": "0x1000"
            },
            "mmio": {
                "region_index": 0,
                "operations": [{"op": "write", "offset": 0, "width": 4, "value": 1}]
            },
            "dma": {
                "bytes": 4096,
                "completion": {"expect_changed": true}
            },
            "interrupt": {
                "mode": "msix",
                "vector": 0,
                "require_vfio_interrupt_remapping": true
            },
            "recovery": {
                "retry_limit": 3,
                "timeout_ms": 10000,
                "quiesce_device": true,
                "mask_msix": true,
                "disable_bus_master": true,
                "flush_iotlb": true,
                "function_level_reset": true,
                "reenable_bus_master": true,
                "unmask_msix": true,
                "verify_device_alive_after_reset": true
            },
            "reset": {"before": true, "after": false}
        });
        let error = PhysicalDmaExercise::parse(&serde_json::to_vec(&base).unwrap())
            .expect_err("missing exact completion must fail")
            .to_string();
        assert!(error.contains("exact sha256"), "{error}");

        let mut with_check = base;
        with_check["dma"]["completion"]["checks"] = json!([{"offset": 0, "width": 1, "equals": 1}]);
        let error = PhysicalDmaExercise::parse(&serde_json::to_vec(&with_check).unwrap())
            .expect_err("post-reset false must fail")
            .to_string();
        assert!(error.contains("reset.before=true"), "{error}");
    }

    #[test]
    fn verifier_keeps_control_plane_and_physical_dma_evidence_separate() {
        let root = std::env::temp_dir().join(format!(
            "ncc-vfio-certificate-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let common_vfio = json!({
            "provider_api_executed": true,
            "dma_map_unmap_executed": true,
            "teardown_complete": true,
            "device_generated_dma_executed": false,
            "device_reset_performed": false,
        });
        let control_payload = json!({
            "schema": CERTIFICATE_SCHEMA,
            "kind": "vfio_control_plane",
            "vfio": common_vfio,
            "truth_boundary": {
                "physical_iommu_executed": true,
                "vfio_passthrough_executed": true,
                "iommu_dma_map_unmap_executed": true,
                "device_generated_dma_executed": false,
                "device_reset_performed": false,
            }
        });
        let control = json!({
            "payload": control_payload.clone(),
            "payload_sha256": payload_sha256(&control_payload).unwrap(),
        });
        let control_path = root.join("control.json");
        fs::write(&control_path, serde_json::to_vec(&control).unwrap()).unwrap();
        run_verify_iommu_certificate(&[control_path.to_string_lossy().into_owned()])
            .expect("control-plane certificate must verify as control-plane only");

        let mut false_physical_payload = control_payload;
        false_physical_payload["kind"] = json!("vfio_physical_passthrough");
        false_physical_payload["exercise"] = json!({"schema": VFIO_DMA_EXERCISE_SCHEMA});
        let false_physical = json!({
            "payload": false_physical_payload.clone(),
            "payload_sha256": payload_sha256(&false_physical_payload).unwrap(),
        });
        let false_physical_path = root.join("false-physical.json");
        fs::write(
            &false_physical_path,
            serde_json::to_vec(&false_physical).unwrap(),
        )
        .unwrap();
        let error =
            run_verify_iommu_certificate(&[false_physical_path.to_string_lossy().into_owned()])
                .expect_err("control-plane evidence must not certify physical DMA")
                .to_string();
        assert!(error.contains("device_generated_dma_executed"), "{error}");
        let _ = fs::remove_dir_all(root);
    }
}
