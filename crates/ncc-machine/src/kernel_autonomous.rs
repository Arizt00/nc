use crate::{
    coordinate_complete_kernel, crc32, parse_user_elf, BlockCommand, BlockOpcode, FileSystemKind,
    KernelCoreError, KernelIoError, KernelMachinePlan, KernelSystem, MachineCoordinationError,
    OpenFlags, PciAddress, PciFunction, PciVirtioError, ProcessId, StorageError, StorageSyncReport,
    VirtualBlockDevice, VirtualDeviceError,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

const IMAGE_MAGIC: &[u8; 8] = b"NCKIMG1\0";
const IMAGE_VERSION: u32 = 1;
const IMAGE_HEADER_BYTES: usize = 64;
const COMPONENT_RECORD_BYTES: usize = 40;
const COMPONENT_ALIGNMENT: usize = 4096;
const FLAG_ROOT_WRITABLE: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UnifiedKernelComponentKind {
    Target = 1,
    KernelElf = 2,
    RootFileSystem = 3,
    InitElf = 4,
    MachineManifest = 5,
    CommandLine = 6,
}

impl TryFrom<u32> for UnifiedKernelComponentKind {
    type Error = AutonomousKernelError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Target),
            2 => Ok(Self::KernelElf),
            3 => Ok(Self::RootFileSystem),
            4 => Ok(Self::InitElf),
            5 => Ok(Self::MachineManifest),
            6 => Ok(Self::CommandLine),
            _ => Err(AutonomousKernelError::InvalidImage(
                "unknown unified-kernel component kind",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UnifiedRootFileSystem {
    Fat = 1,
    Ext2 = 2,
}

impl TryFrom<u32> for UnifiedRootFileSystem {
    type Error = AutonomousKernelError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Fat),
            2 => Ok(Self::Ext2),
            _ => Err(AutonomousKernelError::InvalidImage(
                "unknown unified-kernel root filesystem",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedKernelImage {
    pub target: String,
    pub frames: usize,
    pub cpus: usize,
    pub root_file_system: UnifiedRootFileSystem,
    pub root_writable: bool,
    pub kernel_elf: Vec<u8>,
    pub root_image: Vec<u8>,
    pub init_elf: Vec<u8>,
    pub machine_manifest: String,
    pub command_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedKernelImageReport {
    pub total_bytes: usize,
    pub header_bytes: usize,
    pub components: usize,
    pub image_crc32: u32,
    pub component_crc32: BTreeMap<UnifiedKernelComponentKind, u32>,
}

#[derive(Debug)]
pub enum AutonomousKernelError {
    InvalidImage(&'static str),
    ComponentMissing(UnifiedKernelComponentKind),
    ComponentDuplicate(UnifiedKernelComponentKind),
    ComponentChecksum(UnifiedKernelComponentKind),
    TargetManifestMismatch,
    AddressOverflow,
    Core(KernelCoreError),
    Io(KernelIoError),
    Storage(StorageError),
    Coordination(MachineCoordinationError),
}

impl fmt::Display for AutonomousKernelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidImage(message) => formatter.write_str(message),
            Self::ComponentMissing(kind) => write!(formatter, "missing {kind:?} component"),
            Self::ComponentDuplicate(kind) => write!(formatter, "duplicate {kind:?} component"),
            Self::ComponentChecksum(kind) => write!(formatter, "invalid {kind:?} checksum"),
            Self::TargetManifestMismatch => {
                formatter.write_str("machine manifest does not match the image target")
            }
            Self::AddressOverflow => formatter.write_str("unified-kernel image offset overflow"),
            Self::Core(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            Self::Coordination(error) => error.fmt(formatter),
        }
    }
}

impl Error for AutonomousKernelError {}

impl From<KernelCoreError> for AutonomousKernelError {
    fn from(value: KernelCoreError) -> Self {
        Self::Core(value)
    }
}

impl From<KernelIoError> for AutonomousKernelError {
    fn from(value: KernelIoError) -> Self {
        Self::Io(value)
    }
}

impl From<StorageError> for AutonomousKernelError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<MachineCoordinationError> for AutonomousKernelError {
    fn from(value: MachineCoordinationError) -> Self {
        Self::Coordination(value)
    }
}

impl From<VirtualDeviceError> for AutonomousKernelError {
    fn from(value: VirtualDeviceError) -> Self {
        Self::Io(KernelIoError::from(value))
    }
}

impl From<PciVirtioError> for AutonomousKernelError {
    fn from(value: PciVirtioError) -> Self {
        Self::Io(KernelIoError::from(value))
    }
}

fn align_up(value: usize, alignment: usize) -> Result<usize, AutonomousKernelError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(AutonomousKernelError::AddressOverflow)
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, AutonomousKernelError> {
    let source = bytes
        .get(offset..offset + 4)
        .ok_or(AutonomousKernelError::InvalidImage(
            "truncated unified-kernel u32",
        ))?;
    Ok(u32::from_le_bytes(source.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, AutonomousKernelError> {
    let source = bytes
        .get(offset..offset + 8)
        .ok_or(AutonomousKernelError::InvalidImage(
            "truncated unified-kernel u64",
        ))?;
    Ok(u64::from_le_bytes(source.try_into().unwrap()))
}

impl UnifiedKernelImage {
    pub fn encode(&self) -> Result<(Vec<u8>, UnifiedKernelImageReport), AutonomousKernelError> {
        self.validate_components()?;
        let components = [
            (UnifiedKernelComponentKind::Target, self.target.as_bytes()),
            (
                UnifiedKernelComponentKind::KernelElf,
                self.kernel_elf.as_slice(),
            ),
            (
                UnifiedKernelComponentKind::RootFileSystem,
                self.root_image.as_slice(),
            ),
            (
                UnifiedKernelComponentKind::InitElf,
                self.init_elf.as_slice(),
            ),
            (
                UnifiedKernelComponentKind::MachineManifest,
                self.machine_manifest.as_bytes(),
            ),
            (
                UnifiedKernelComponentKind::CommandLine,
                self.command_line.as_bytes(),
            ),
        ];
        let table_end = IMAGE_HEADER_BYTES
            .checked_add(components.len() * COMPONENT_RECORD_BYTES)
            .ok_or(AutonomousKernelError::AddressOverflow)?;
        let header_bytes = align_up(table_end, COMPONENT_ALIGNMENT)?;
        let mut offsets = Vec::with_capacity(components.len());
        let mut cursor = header_bytes;
        for (_, component) in components {
            cursor = align_up(cursor, COMPONENT_ALIGNMENT)?;
            offsets.push(cursor);
            cursor = cursor
                .checked_add(component.len())
                .ok_or(AutonomousKernelError::AddressOverflow)?;
        }
        let mut bytes = vec![0u8; cursor];
        bytes[..8].copy_from_slice(IMAGE_MAGIC);
        write_u32(&mut bytes, 8, IMAGE_VERSION);
        write_u32(&mut bytes, 12, header_bytes as u32);
        write_u32(&mut bytes, 16, components.len() as u32);
        write_u32(&mut bytes, 20, self.root_file_system as u32);
        write_u64(&mut bytes, 24, self.frames as u64);
        write_u32(&mut bytes, 32, self.cpus as u32);
        write_u32(
            &mut bytes,
            36,
            if self.root_writable {
                FLAG_ROOT_WRITABLE
            } else {
                0
            },
        );
        write_u64(&mut bytes, 40, cursor as u64);
        let mut component_crc32 = BTreeMap::new();
        for (index, ((kind, component), offset)) in components.into_iter().zip(offsets).enumerate()
        {
            let record = IMAGE_HEADER_BYTES + index * COMPONENT_RECORD_BYTES;
            let checksum = crc32(component);
            write_u32(&mut bytes, record, kind as u32);
            write_u32(&mut bytes, record + 4, 0);
            write_u64(&mut bytes, record + 8, offset as u64);
            write_u64(&mut bytes, record + 16, component.len() as u64);
            write_u32(&mut bytes, record + 24, checksum);
            write_u32(&mut bytes, record + 28, COMPONENT_ALIGNMENT as u32);
            bytes[offset..offset + component.len()].copy_from_slice(component);
            component_crc32.insert(kind, checksum);
        }
        let header_crc32 = crc32(&bytes[..header_bytes]);
        write_u32(&mut bytes, 48, header_crc32);
        let report = UnifiedKernelImageReport {
            total_bytes: bytes.len(),
            header_bytes,
            components: components.len(),
            image_crc32: crc32(&bytes),
            component_crc32,
        };
        Ok((bytes, report))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, AutonomousKernelError> {
        if bytes.len() < IMAGE_HEADER_BYTES || bytes.get(..8) != Some(IMAGE_MAGIC) {
            return Err(AutonomousKernelError::InvalidImage(
                "invalid unified-kernel magic",
            ));
        }
        if read_u32(bytes, 8)? != IMAGE_VERSION {
            return Err(AutonomousKernelError::InvalidImage(
                "unsupported unified-kernel version",
            ));
        }
        let header_bytes = read_u32(bytes, 12)? as usize;
        let count = read_u32(bytes, 16)? as usize;
        let table_end = IMAGE_HEADER_BYTES
            .checked_add(count * COMPONENT_RECORD_BYTES)
            .ok_or(AutonomousKernelError::AddressOverflow)?;
        if count == 0
            || header_bytes < table_end
            || header_bytes % COMPONENT_ALIGNMENT != 0
            || read_u64(bytes, 40)? as usize != bytes.len()
            || header_bytes > bytes.len()
        {
            return Err(AutonomousKernelError::InvalidImage(
                "invalid unified-kernel geometry",
            ));
        }
        let expected_header_crc = read_u32(bytes, 48)?;
        let mut header = bytes[..header_bytes].to_vec();
        write_u32(&mut header, 48, 0);
        if crc32(&header) != expected_header_crc {
            return Err(AutonomousKernelError::InvalidImage(
                "invalid unified-kernel header checksum",
            ));
        }
        let mut components = BTreeMap::new();
        let mut ranges = Vec::new();
        for index in 0..count {
            let record = IMAGE_HEADER_BYTES + index * COMPONENT_RECORD_BYTES;
            let kind = UnifiedKernelComponentKind::try_from(read_u32(bytes, record)?)?;
            let offset = usize::try_from(read_u64(bytes, record + 8)?)
                .map_err(|_| AutonomousKernelError::AddressOverflow)?;
            let length = usize::try_from(read_u64(bytes, record + 16)?)
                .map_err(|_| AutonomousKernelError::AddressOverflow)?;
            let end = offset
                .checked_add(length)
                .filter(|end| *end <= bytes.len())
                .ok_or(AutonomousKernelError::AddressOverflow)?;
            if offset < header_bytes || offset % COMPONENT_ALIGNMENT != 0 {
                return Err(AutonomousKernelError::InvalidImage(
                    "misaligned unified-kernel component",
                ));
            }
            if ranges
                .iter()
                .any(|(start, previous_end)| offset < *previous_end && end > *start)
            {
                return Err(AutonomousKernelError::InvalidImage(
                    "overlapping unified-kernel components",
                ));
            }
            let component = bytes[offset..end].to_vec();
            if crc32(&component) != read_u32(bytes, record + 24)? {
                return Err(AutonomousKernelError::ComponentChecksum(kind));
            }
            if components.insert(kind, component).is_some() {
                return Err(AutonomousKernelError::ComponentDuplicate(kind));
            }
            ranges.push((offset, end));
        }
        let take = |components: &mut BTreeMap<_, _>, kind| {
            components
                .remove(&kind)
                .ok_or(AutonomousKernelError::ComponentMissing(kind))
        };
        let target = String::from_utf8(take(&mut components, UnifiedKernelComponentKind::Target)?)
            .map_err(|_| AutonomousKernelError::InvalidImage("target is not UTF-8"))?;
        let machine_manifest = String::from_utf8(take(
            &mut components,
            UnifiedKernelComponentKind::MachineManifest,
        )?)
        .map_err(|_| AutonomousKernelError::InvalidImage("machine manifest is not UTF-8"))?;
        let command_line = String::from_utf8(take(
            &mut components,
            UnifiedKernelComponentKind::CommandLine,
        )?)
        .map_err(|_| AutonomousKernelError::InvalidImage("command line is not UTF-8"))?;
        let image = Self {
            target,
            frames: usize::try_from(read_u64(bytes, 24)?)
                .map_err(|_| AutonomousKernelError::AddressOverflow)?,
            cpus: read_u32(bytes, 32)? as usize,
            root_file_system: UnifiedRootFileSystem::try_from(read_u32(bytes, 20)?)?,
            root_writable: read_u32(bytes, 36)? & FLAG_ROOT_WRITABLE != 0,
            kernel_elf: take(&mut components, UnifiedKernelComponentKind::KernelElf)?,
            root_image: take(&mut components, UnifiedKernelComponentKind::RootFileSystem)?,
            init_elf: take(&mut components, UnifiedKernelComponentKind::InitElf)?,
            machine_manifest,
            command_line,
        };
        image.validate_components()?;
        Ok(image)
    }

    fn validate_components(&self) -> Result<(), AutonomousKernelError> {
        if self.frames == 0 || self.cpus == 0 || self.cpus > 128 {
            return Err(AutonomousKernelError::InvalidImage(
                "unified-kernel memory and CPU counts must be non-zero",
            ));
        }
        if !self.kernel_elf.starts_with(b"\x7fELF") || !self.init_elf.starts_with(b"\x7fELF") {
            return Err(AutonomousKernelError::InvalidImage(
                "kernel and init components must be ELF images",
            ));
        }
        if self.root_image.is_empty() || self.target.is_empty() || self.machine_manifest.is_empty()
        {
            return Err(AutonomousKernelError::InvalidImage(
                "unified-kernel components cannot be empty",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousBootReport {
    pub target: String,
    pub root_kind: FileSystemKind,
    pub root_writable: bool,
    pub init_pid: ProcessId,
    pub scheduled: Vec<Option<ProcessId>>,
    pub machine_subsystems: usize,
    pub pcie_devices: usize,
    pub root_files: usize,
    pub network_online: bool,
    pub framebuffer_crc32: u32,
    pub sync: Option<StorageSyncReport>,
}

#[derive(Debug, Clone)]
pub struct AutonomousKernel {
    pub image: UnifiedKernelImage,
    pub plan: KernelMachinePlan,
    pub system: KernelSystem,
    pub init_pid: ProcessId,
    pub scheduled: Vec<Option<ProcessId>>,
    next_block_command: u16,
}

fn expected_elf_machine(target: &str) -> Option<u16> {
    if target.starts_with("x86_64") {
        Some(62)
    } else if target.starts_with("i386") {
        Some(3)
    } else if target.starts_with("aarch64") {
        Some(183)
    } else if target.starts_with("riscv") {
        Some(243)
    } else if target.starts_with("armv7") || target.starts_with("thumb") {
        Some(40)
    } else {
        None
    }
}

impl AutonomousKernel {
    pub fn boot(bytes: &[u8]) -> Result<(Self, AutonomousBootReport), AutonomousKernelError> {
        let image = UnifiedKernelImage::decode(bytes)?;
        let plan = coordinate_complete_kernel(&image.target)?;
        if image.machine_manifest != plan.canonical_manifest() {
            return Err(AutonomousKernelError::TargetManifestMismatch);
        }
        let expected_machine = expected_elf_machine(&image.target).ok_or(
            AutonomousKernelError::InvalidImage("target has no autonomous ELF ABI"),
        )?;
        parse_user_elf(&image.kernel_elf, Some(expected_machine))?;
        let mut system = KernelSystem::new(image.frames, image.cpus)?;
        let mount = match image.root_file_system {
            UnifiedRootFileSystem::Fat => system.vfs.mount_fat(
                "/root",
                "boot-root",
                &image.root_image,
                !image.root_writable,
            )?,
            UnifiedRootFileSystem::Ext2 => system.vfs.mount_ext2_mode(
                "/root",
                "boot-root",
                &image.root_image,
                !image.root_writable,
            )?,
        };
        system.pcie.attach(PciFunction::new(
            PciAddress::new(0, 0, 1, 0)?,
            0x1b36,
            0x0010,
            1,
            0x01,
            0x08,
            0x02,
            0x1b36,
            0x0010,
        ))?;
        let root_device = if image.root_writable {
            system.vfs.persistent_device_image("/root")?.to_vec()
        } else {
            image.root_image.clone()
        };
        system.nvme.attach_namespace(
            1,
            VirtualBlockDevice::from_bytes(root_device, 512, !image.root_writable)?,
        )?;
        system.nvme.create_io_queue(1, 64)?;
        let init_pid =
            system
                .core
                .load_process("init", &image.init_elf, Some(expected_machine), Some(0))?;
        let mut scheduled = Vec::with_capacity(image.cpus);
        for cpu in 0..image.cpus {
            scheduled.push(system.core.schedule_cpu(cpu)?);
        }
        system.display.clear(0xff10_1820)?;
        let mut kernel = Self {
            image,
            plan,
            system,
            init_pid,
            scheduled,
            next_block_command: 1,
        };
        let sync = if kernel.image.root_writable {
            kernel.system.vfs.mkdir_all("/root/var/log", 0o755)?;
            let handle = kernel.system.vfs.open(
                "/root/var/log/boot.log",
                OpenFlags::WRITE.union(OpenFlags::CREATE),
            )?;
            kernel
                .system
                .vfs
                .write(handle, b"nC autonomous kernel booted\n")?;
            kernel.system.vfs.close(handle)?;
            Some(kernel.sync_root()?)
        } else {
            None
        };
        let report = AutonomousBootReport {
            target: kernel.image.target.clone(),
            root_kind: mount.kind,
            root_writable: kernel.image.root_writable,
            init_pid,
            scheduled: kernel.scheduled.clone(),
            machine_subsystems: kernel.plan.ordered_subsystems.len(),
            pcie_devices: kernel.system.pcie.enumerate()?.len(),
            root_files: kernel.system.vfs.snapshot_under("/root")?.len(),
            network_online: kernel.system.network.device.link_up,
            framebuffer_crc32: kernel.system.display.framebuffer.checksum(),
            sync,
        };
        Ok((kernel, report))
    }

    pub fn tick_all(&mut self, elapsed_ns: u64) -> Result<(), AutonomousKernelError> {
        for cpu in 0..self.image.cpus {
            self.scheduled[cpu] = self.system.core.timer_interrupt(cpu, elapsed_ns)?;
        }
        Ok(())
    }

    pub fn sync_root(&mut self) -> Result<StorageSyncReport, AutonomousKernelError> {
        if !self.image.root_writable {
            return Err(AutonomousKernelError::Storage(StorageError::ReadOnly));
        }
        let report = self.system.vfs.sync_mount("/root")?;
        let persisted = self.system.vfs.persistent_device_image("/root")?.to_vec();
        if persisted.len() % 512 != 0 || persisted.len() / 512 > u32::MAX as usize {
            return Err(AutonomousKernelError::InvalidImage(
                "persistent root image does not fit a block command",
            ));
        }
        let command = self.next_block_command;
        self.next_block_command = self.next_block_command.wrapping_add(1).max(1);
        self.system.nvme.submit(
            1,
            1,
            BlockCommand {
                id: command,
                opcode: BlockOpcode::Write,
                lba: 0,
                blocks: (persisted.len() / 512) as u32,
                data: persisted,
            },
        )?;
        self.system
            .nvme
            .completion(1)?
            .ok_or(KernelIoError::InvalidCommand(
                "missing root write completion",
            ))?;
        let command = self.next_block_command;
        self.next_block_command = self.next_block_command.wrapping_add(1).max(1);
        self.system.nvme.submit(
            1,
            1,
            BlockCommand {
                id: command,
                opcode: BlockOpcode::Flush,
                lba: 0,
                blocks: 0,
                data: Vec::new(),
            },
        )?;
        self.system
            .nvme
            .completion(1)?
            .ok_or(KernelIoError::InvalidCommand(
                "missing root flush completion",
            ))?;
        Ok(report)
    }

    pub fn shutdown(mut self) -> Result<Vec<u8>, AutonomousKernelError> {
        if self.image.root_writable {
            self.sync_root()?;
            Ok(self.system.vfs.persistent_device_image("/root")?.to_vec())
        } else {
            Ok(self.image.root_image)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_test_elf64, create_ext2_writable_image};

    fn image() -> UnifiedKernelImage {
        let plan = coordinate_complete_kernel("x86_64-unknown-none").unwrap();
        UnifiedKernelImage {
            target: "x86_64-unknown-none".into(),
            frames: 256,
            cpus: 4,
            root_file_system: UnifiedRootFileSystem::Ext2,
            root_writable: true,
            kernel_elf: build_test_elf64(62, 0x20_0000, &[0xfa, 0xf4]),
            root_image: create_ext2_writable_image(),
            init_elf: build_test_elf64(62, 0x40_0000, &[0x31, 0xc0, 0xc3]),
            machine_manifest: plan.canonical_manifest(),
            command_line: "root=/dev/nvme0n1 rw init=/init".into(),
        }
    }

    #[test]
    fn unified_image_roundtrip_rejects_component_corruption() {
        let source = image();
        let (bytes, report) = source.encode().unwrap();
        assert_eq!(report.components, 6);
        assert_eq!(UnifiedKernelImage::decode(&bytes).unwrap(), source);

        let mut damaged = bytes;
        let root_record = IMAGE_HEADER_BYTES + 2 * COMPONENT_RECORD_BYTES;
        let root_offset = read_u64(&damaged, root_record + 8).unwrap() as usize;
        damaged[root_offset + 1024 + 56] ^= 0x80;
        assert!(matches!(
            UnifiedKernelImage::decode(&damaged),
            Err(AutonomousKernelError::ComponentChecksum(
                UnifiedKernelComponentKind::RootFileSystem
            ))
        ));
    }

    #[test]
    fn one_image_boots_scheduler_vfs_init_devices_network_and_persists_root() {
        let (bytes, _) = image().encode().unwrap();
        let (mut kernel, report) = AutonomousKernel::boot(&bytes).unwrap();
        assert_eq!(report.init_pid, 1);
        assert_eq!(report.scheduled.len(), 4);
        assert!(report.network_online);
        assert_eq!(report.pcie_devices, 1);
        assert!(report.sync.as_ref().unwrap().journal_commits >= 1);
        assert_eq!(
            kernel
                .system
                .vfs
                .file_bytes("/root/var/log/boot.log")
                .unwrap(),
            b"nC autonomous kernel booted\n"
        );
        kernel.tick_all(1_000_000).unwrap();
        let persisted = kernel.shutdown().unwrap();
        let mut remount = crate::MountedVfs::default();
        remount
            .mount_ext2_mode("/root", "persisted", &persisted, false)
            .unwrap();
        assert_eq!(
            remount.vfs.file_bytes("/root/var/log/boot.log").unwrap(),
            b"nC autonomous kernel booted\n"
        );
    }
}
