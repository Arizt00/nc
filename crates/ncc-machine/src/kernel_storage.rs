use crate::kernel_filesystems::{rebuild, PersistentFormat};
use crate::{
    KernelIoError, OpenFlags, Vfs, VfsAclEntry, VfsAclTag, VfsCredentials, VfsSnapshotEntry,
    VirtualBlockDevice, VirtualDeviceError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub enum StorageError {
    Io(KernelIoError),
    Device(VirtualDeviceError),
    InvalidImage(&'static str),
    CorruptImage(String),
    Unsupported(&'static str),
    MountExists(String),
    MountMissing(String),
    MountBusy(String),
    CacheCapacity,
    ReadOnly,
    MountDirty(String),
    JournalCorrupt(String),
    TransactionInterrupted,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),
            Self::Device(error) => error.fmt(f),
            Self::InvalidImage(message) | Self::Unsupported(message) => f.write_str(message),
            Self::CorruptImage(message) => write!(f, "corrupt filesystem: {message}"),
            Self::MountExists(path) => write!(f, "mount point '{path}' is already mounted"),
            Self::MountMissing(path) => write!(f, "mount point '{path}' is not mounted"),
            Self::MountBusy(path) => write!(f, "mount point '{path}' is busy"),
            Self::CacheCapacity => f.write_str("block cache has zero capacity"),
            Self::ReadOnly => f.write_str("filesystem is read-only"),
            Self::MountDirty(path) => write!(f, "mount point '{path}' has unsynchronized changes"),
            Self::JournalCorrupt(message) => write!(f, "corrupt storage journal: {message}"),
            Self::TransactionInterrupted => {
                f.write_str("storage transaction interrupted after journal commit")
            }
        }
    }
}

impl Error for StorageError {}

impl From<KernelIoError> for StorageError {
    fn from(value: KernelIoError) -> Self {
        Self::Io(value)
    }
}

impl From<VirtualDeviceError> for StorageError {
    fn from(value: VirtualDeviceError) -> Self {
        Self::Device(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemKind {
    InitramfsNewc,
    Fat12,
    Fat16,
    Fat32,
    Ext2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountInfo {
    pub path: String,
    pub source: String,
    pub kind: FileSystemKind,
    pub read_only: bool,
    pub files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageSyncFault {
    None,
    AfterJournalCommit,
    AfterTargetBlocks(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSyncReport {
    pub mount: String,
    pub transaction: u64,
    pub changed_blocks: usize,
    pub journal_commits: usize,
    pub journal_replays: usize,
    pub data_blocks: usize,
    pub metadata_blocks: usize,
    pub indirect_blocks: usize,
    pub acl_blocks: usize,
    pub long_names: usize,
    pub bytes_written: u64,
    pub flushes: u64,
}

const JOURNAL_MAGIC: &[u8; 8] = b"NCCJNL2\0";
const JOURNAL_VERSION: u32 = 2;
const JOURNAL_PREPARED: u32 = 1;
const JOURNAL_BLOCKS: usize = 65;

#[derive(Debug, Clone)]
struct PersistentMount {
    format: PersistentFormat,
    device: VirtualBlockDevice,
    logical_bytes: usize,
    block_size: usize,
    transaction: u64,
    baseline: Vec<VfsSnapshotEntry>,
    journal_replays: usize,
}

impl PersistentMount {
    fn with_image(
        format: PersistentFormat,
        image: &[u8],
        logical_bytes: usize,
        block_size: usize,
    ) -> Result<Self, StorageError> {
        if logical_bytes == 0
            || logical_bytes > image.len()
            || logical_bytes % block_size != 0
            || block_size < 512
            || !block_size.is_power_of_two()
        {
            return Err(StorageError::InvalidImage(
                "invalid persistent filesystem geometry",
            ));
        }
        let journal_bytes = JOURNAL_BLOCKS
            .checked_mul(block_size)
            .ok_or(StorageError::InvalidImage("journal size overflow"))?;
        let capacity =
            logical_bytes
                .checked_add(journal_bytes)
                .ok_or(StorageError::InvalidImage(
                    "persistent volume size overflow",
                ))?;
        let mut bytes = vec![0u8; capacity];
        let copied = image.len().min(capacity);
        bytes[..copied].copy_from_slice(&image[..copied]);
        let device = VirtualBlockDevice::from_bytes(bytes, 512, false)?;
        let mut mount = Self {
            format,
            device,
            logical_bytes,
            block_size,
            transaction: 0,
            baseline: Vec::new(),
            journal_replays: 0,
        };
        mount.recover()?;
        Ok(mount)
    }

    fn journal_offset(&self) -> usize {
        self.logical_bytes
    }

    fn journal_header(&self) -> Result<(u32, u64, Vec<u64>), StorageError> {
        let start = self.journal_offset();
        let bytes = self.device.bytes();
        let header = bytes
            .get(start..start + self.block_size)
            .ok_or_else(|| StorageError::JournalCorrupt("header is out of bounds".into()))?;
        if header[..8] != *JOURNAL_MAGIC {
            if header.iter().all(|byte| *byte == 0) {
                return Ok((0, 0, Vec::new()));
            }
            return Err(StorageError::JournalCorrupt("invalid magic".into()));
        }
        if le32(header, 8, "journal version")? != JOURNAL_VERSION {
            return Err(StorageError::JournalCorrupt("unsupported version".into()));
        }
        let state = le32(header, 12, "journal state")?;
        let transaction = u64::from_le_bytes(
            checked_slice(header, 16, 8, "journal transaction")?
                .try_into()
                .unwrap(),
        );
        let count = le32(header, 24, "journal record count")? as usize;
        let encoded_block_size = le32(header, 28, "journal block size")? as usize;
        let maximum = (self.block_size - 32) / 8;
        if count > maximum || count >= JOURNAL_BLOCKS || encoded_block_size != self.block_size {
            return Err(StorageError::JournalCorrupt(
                "invalid record geometry".into(),
            ));
        }
        let mut blocks = Vec::with_capacity(count);
        for index in 0..count {
            blocks.push(u64::from_le_bytes(
                checked_slice(header, 32 + index * 8, 8, "journal target block")?
                    .try_into()
                    .unwrap(),
            ));
        }
        Ok((state, transaction, blocks))
    }

    fn clear_journal(&mut self) -> Result<(), StorageError> {
        let header = vec![0u8; self.block_size];
        self.device.write(self.journal_offset() as u64, &header)?;
        self.device.flush();
        Ok(())
    }

    fn recover(&mut self) -> Result<usize, StorageError> {
        let (state, transaction, blocks) = self.journal_header()?;
        self.transaction = self.transaction.max(transaction);
        if state == 0 {
            return Ok(0);
        }
        if state != JOURNAL_PREPARED {
            return Err(StorageError::JournalCorrupt(format!(
                "unknown state {state}"
            )));
        }
        for (index, block) in blocks.iter().enumerate() {
            let target = block
                .checked_mul(self.block_size as u64)
                .filter(|offset| *offset + self.block_size as u64 <= self.logical_bytes as u64)
                .ok_or_else(|| {
                    StorageError::JournalCorrupt("target block is out of bounds".into())
                })?;
            let payload_offset = self.journal_offset() + (index + 1) * self.block_size;
            let payload =
                self.device.bytes()[payload_offset..payload_offset + self.block_size].to_vec();
            self.device.write(target, &payload)?;
        }
        self.device.flush();
        self.clear_journal()?;
        self.journal_replays = self.journal_replays.saturating_add(1);
        Ok(blocks.len())
    }

    fn write_chunk(
        &mut self,
        rebuilt: &[u8],
        blocks: &[u64],
        fault: StorageSyncFault,
        written: &mut usize,
    ) -> Result<(), StorageError> {
        self.transaction = self.transaction.wrapping_add(1).max(1);
        let mut header = vec![0u8; self.block_size];
        header[..8].copy_from_slice(JOURNAL_MAGIC);
        header[8..12].copy_from_slice(&JOURNAL_VERSION.to_le_bytes());
        header[12..16].copy_from_slice(&JOURNAL_PREPARED.to_le_bytes());
        header[16..24].copy_from_slice(&self.transaction.to_le_bytes());
        header[24..28].copy_from_slice(&(blocks.len() as u32).to_le_bytes());
        header[28..32].copy_from_slice(&(self.block_size as u32).to_le_bytes());
        for (index, block) in blocks.iter().enumerate() {
            header[32 + index * 8..40 + index * 8].copy_from_slice(&block.to_le_bytes());
            let source = *block as usize * self.block_size;
            let payload_offset = self.journal_offset() + (index + 1) * self.block_size;
            self.device.write(
                payload_offset as u64,
                &rebuilt[source..source + self.block_size],
            )?;
        }
        self.device.write(self.journal_offset() as u64, &header)?;
        self.device.flush();
        if fault == StorageSyncFault::AfterJournalCommit {
            return Err(StorageError::TransactionInterrupted);
        }
        for block in blocks {
            let offset = *block as usize * self.block_size;
            self.device
                .write(offset as u64, &rebuilt[offset..offset + self.block_size])?;
            *written += 1;
            if matches!(fault, StorageSyncFault::AfterTargetBlocks(limit) if *written == limit) {
                return Err(StorageError::TransactionInterrupted);
            }
        }
        self.device.flush();
        self.clear_journal()?;
        Ok(())
    }

    fn synchronize(
        &mut self,
        snapshot: &[VfsSnapshotEntry],
        fault: StorageSyncFault,
    ) -> Result<StorageSyncReport, StorageError> {
        self.recover()?;
        let rebuilt = rebuild(
            self.format,
            &self.device.bytes()[..self.logical_bytes],
            snapshot,
        )
        .map_err(StorageError::CorruptImage)?;
        if rebuilt.logical_bytes != self.logical_bytes || rebuilt.block_size % 512 != 0 {
            return Err(StorageError::CorruptImage(
                "rebuilt filesystem changed the declared geometry".into(),
            ));
        }
        let changed = (0..self.logical_bytes / self.block_size)
            .filter(|block| {
                let offset = block * self.block_size;
                self.device.bytes()[offset..offset + self.block_size]
                    != rebuilt.bytes[offset..offset + self.block_size]
            })
            .map(|block| block as u64)
            .collect::<Vec<_>>();
        let max_records = ((self.block_size - 32) / 8).min(JOURNAL_BLOCKS - 1);
        if max_records == 0 {
            return Err(StorageError::JournalCorrupt(
                "journal header has no record capacity".into(),
            ));
        }
        let bytes_before = self.device.written_bytes;
        let flushes_before = self.device.flush_count;
        let mut written = 0usize;
        let mut commits = 0usize;
        for chunk in changed.chunks(max_records) {
            self.write_chunk(&rebuilt.bytes, chunk, fault, &mut written)?;
            commits += 1;
        }
        self.baseline = snapshot.to_vec();
        Ok(StorageSyncReport {
            mount: String::new(),
            transaction: self.transaction,
            changed_blocks: changed.len(),
            journal_commits: commits,
            journal_replays: self.journal_replays,
            data_blocks: rebuilt.data_blocks,
            metadata_blocks: rebuilt.metadata_blocks,
            indirect_blocks: rebuilt.indirect_blocks,
            acl_blocks: rebuilt.acl_blocks,
            long_names: rebuilt.long_names,
            bytes_written: self.device.written_bytes.saturating_sub(bytes_before),
            flushes: self.device.flush_count.saturating_sub(flushes_before),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportedKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportedEntry {
    path: String,
    kind: ImportedKind,
    mode: u16,
    uid: u32,
    gid: u32,
    data: Vec<u8>,
    acl: Vec<VfsAclEntry>,
}

fn checked_slice<'a>(
    bytes: &'a [u8],
    offset: usize,
    length: usize,
    what: &str,
) -> Result<&'a [u8], StorageError> {
    bytes
        .get(offset..offset.saturating_add(length))
        .ok_or_else(|| StorageError::CorruptImage(format!("{what} is out of bounds")))
}

fn le16(bytes: &[u8], offset: usize, what: &str) -> Result<u16, StorageError> {
    Ok(u16::from_le_bytes(
        checked_slice(bytes, offset, 2, what)?.try_into().unwrap(),
    ))
}

fn le32(bytes: &[u8], offset: usize, what: &str) -> Result<u32, StorageError> {
    Ok(u32::from_le_bytes(
        checked_slice(bytes, offset, 4, what)?.try_into().unwrap(),
    ))
}

fn archive_path(path: &str) -> Result<String, StorageError> {
    let path = path.trim_start_matches('/');
    if path.is_empty()
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || path.contains('\0')
    {
        return Err(StorageError::CorruptImage(format!(
            "unsafe archive path '{path}'"
        )));
    }
    Ok(path.to_string())
}

fn mount_join(mount: &str, relative: &str) -> String {
    if mount == "/" {
        format!("/{relative}")
    } else {
        format!("{mount}/{relative}")
    }
}

fn align_up(value: usize, alignment: usize) -> Result<usize, StorageError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| StorageError::CorruptImage("alignment overflow".into()))
}

fn hex_field(bytes: &[u8], offset: usize, what: &str) -> Result<u32, StorageError> {
    let raw = checked_slice(bytes, offset, 8, what)?;
    let text = std::str::from_utf8(raw)
        .map_err(|_| StorageError::CorruptImage(format!("{what} is not ASCII")))?;
    u32::from_str_radix(text, 16)
        .map_err(|_| StorageError::CorruptImage(format!("{what} is not hexadecimal")))
}

fn parse_newc(bytes: &[u8]) -> Result<Vec<ImportedEntry>, StorageError> {
    let mut offset = 0usize;
    let mut entries = Vec::new();
    let mut saw_trailer = false;
    while offset < bytes.len() {
        let header = checked_slice(bytes, offset, 110, "newc header")?;
        if &header[..6] != b"070701" && &header[..6] != b"070702" {
            return Err(StorageError::InvalidImage("invalid cpio newc magic"));
        }
        let mode = hex_field(header, 14, "newc mode")?;
        let uid = hex_field(header, 22, "newc uid")?;
        let gid = hex_field(header, 30, "newc gid")?;
        let size = hex_field(header, 54, "newc size")? as usize;
        let name_size = hex_field(header, 94, "newc name size")? as usize;
        if name_size == 0 {
            return Err(StorageError::CorruptImage(
                "newc entry has an empty encoded name".into(),
            ));
        }
        let name_start = offset + 110;
        let name_raw = checked_slice(bytes, name_start, name_size, "newc name")?;
        if name_raw.last() != Some(&0) {
            return Err(StorageError::CorruptImage(
                "newc name is not NUL terminated".into(),
            ));
        }
        let name = std::str::from_utf8(&name_raw[..name_size - 1])
            .map_err(|_| StorageError::CorruptImage("newc name is not UTF-8".into()))?;
        let data_start = align_up(name_start + name_size, 4)?;
        let data = checked_slice(bytes, data_start, size, "newc payload")?;
        offset = align_up(data_start + size, 4)?;
        if name == "TRAILER!!!" {
            saw_trailer = true;
            break;
        }
        let path = archive_path(name)?;
        let kind = match mode & 0o170000 {
            0o040000 => ImportedKind::Directory,
            0o100000 => ImportedKind::File,
            0o120000 => ImportedKind::Symlink,
            _ => continue,
        };
        entries.push(ImportedEntry {
            path,
            kind,
            mode: (mode & 0o7777) as u16,
            uid,
            gid,
            data: data.to_vec(),
            acl: Vec::new(),
        });
    }
    if !saw_trailer {
        return Err(StorageError::CorruptImage(
            "newc archive has no TRAILER!!! entry".into(),
        ));
    }
    Ok(entries)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FatKind {
    Fat12,
    Fat16,
    Fat32,
}

struct FatVolume<'a> {
    bytes: &'a [u8],
    kind: FatKind,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    fat_offset: usize,
    fat_bytes: usize,
    data_offset: usize,
    root_offset: usize,
    root_bytes: usize,
    root_cluster: u32,
    cluster_count: u32,
    total_sectors: usize,
}

impl<'a> FatVolume<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, StorageError> {
        if bytes.len() < 512 || bytes.get(510..512) != Some(&[0x55, 0xaa]) {
            return Err(StorageError::InvalidImage("invalid FAT boot sector"));
        }
        let bytes_per_sector = le16(bytes, 11, "FAT sector size")? as usize;
        let sectors_per_cluster = bytes[13] as usize;
        let reserved = le16(bytes, 14, "FAT reserved sectors")? as usize;
        let fats = bytes[16] as usize;
        let root_entries = le16(bytes, 17, "FAT root entries")? as usize;
        let total16 = le16(bytes, 19, "FAT total sectors")? as usize;
        let total32 = le32(bytes, 32, "FAT total sectors32")? as usize;
        let fat16 = le16(bytes, 22, "FAT size16")? as usize;
        let fat32 = le32(bytes, 36, "FAT size32")? as usize;
        if !bytes_per_sector.is_power_of_two()
            || !(512..=4096).contains(&bytes_per_sector)
            || !sectors_per_cluster.is_power_of_two()
            || sectors_per_cluster == 0
            || reserved == 0
            || fats == 0
        {
            return Err(StorageError::CorruptImage("invalid FAT geometry".into()));
        }
        let total_sectors = if total16 != 0 { total16 } else { total32 };
        let fat_sectors = if fat16 != 0 { fat16 } else { fat32 };
        let root_sectors = (root_entries * 32).div_ceil(bytes_per_sector);
        let data_sectors = total_sectors
            .checked_sub(reserved + fats * fat_sectors + root_sectors)
            .ok_or_else(|| StorageError::CorruptImage("FAT geometry underflow".into()))?;
        let cluster_count = (data_sectors / sectors_per_cluster) as u32;
        let kind = if cluster_count < 4085 {
            FatKind::Fat12
        } else if cluster_count < 65525 {
            FatKind::Fat16
        } else {
            FatKind::Fat32
        };
        let fat_offset = reserved * bytes_per_sector;
        let fat_bytes = fat_sectors * bytes_per_sector;
        let root_offset = (reserved + fats * fat_sectors) * bytes_per_sector;
        let root_bytes = root_sectors * bytes_per_sector;
        let data_offset = (reserved + fats * fat_sectors + root_sectors) * bytes_per_sector;
        let root_cluster = if kind == FatKind::Fat32 {
            le32(bytes, 44, "FAT32 root cluster")?
        } else {
            0
        };
        let volume_bytes = total_sectors
            .checked_mul(bytes_per_sector)
            .ok_or_else(|| StorageError::CorruptImage("FAT volume size overflow".into()))?;
        if volume_bytes > bytes.len()
            || fat_offset + fat_bytes > bytes.len()
            || root_offset + root_bytes > bytes.len()
            || data_offset > bytes.len()
        {
            return Err(StorageError::CorruptImage(
                "FAT regions exceed the image".into(),
            ));
        }
        Ok(Self {
            bytes,
            kind,
            bytes_per_sector,
            sectors_per_cluster,
            fat_offset,
            fat_bytes,
            data_offset,
            root_offset,
            root_bytes,
            root_cluster,
            cluster_count,
            total_sectors,
        })
    }

    fn cluster_size(&self) -> usize {
        self.bytes_per_sector * self.sectors_per_cluster
    }

    fn cluster_bytes(&self, cluster: u32) -> Result<&'a [u8], StorageError> {
        if cluster < 2 || cluster >= self.cluster_count.saturating_add(2) {
            return Err(StorageError::CorruptImage(format!(
                "FAT cluster {cluster} is out of range"
            )));
        }
        let offset = self
            .data_offset
            .checked_add((cluster as usize - 2) * self.cluster_size())
            .ok_or_else(|| StorageError::CorruptImage("FAT cluster offset overflow".into()))?;
        checked_slice(self.bytes, offset, self.cluster_size(), "FAT cluster")
    }

    fn next_cluster(&self, cluster: u32) -> Result<Option<u32>, StorageError> {
        let fat = checked_slice(self.bytes, self.fat_offset, self.fat_bytes, "FAT")?;
        let value = match self.kind {
            FatKind::Fat12 => {
                let offset = cluster as usize + cluster as usize / 2;
                let pair = le16(fat, offset, "FAT12 entry")?;
                (if cluster & 1 == 0 {
                    pair & 0x0fff
                } else {
                    pair >> 4
                }) as u32
            }
            FatKind::Fat16 => le16(fat, cluster as usize * 2, "FAT16 entry")? as u32,
            FatKind::Fat32 => le32(fat, cluster as usize * 4, "FAT32 entry")? & 0x0fff_ffff,
        };
        let end = match self.kind {
            FatKind::Fat12 => 0x0ff8,
            FatKind::Fat16 => 0xfff8,
            FatKind::Fat32 => 0x0fff_fff8,
        };
        if value >= end {
            Ok(None)
        } else if value < 2 {
            Err(StorageError::CorruptImage(format!(
                "FAT chain contains reserved cluster {value}"
            )))
        } else {
            Ok(Some(value))
        }
    }

    fn chain(&self, first: u32) -> Result<Vec<u32>, StorageError> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = first;
        loop {
            if !seen.insert(current) {
                return Err(StorageError::CorruptImage(
                    "FAT cluster chain contains a loop".into(),
                ));
            }
            chain.push(current);
            if chain.len() > self.cluster_count as usize {
                return Err(StorageError::CorruptImage(
                    "FAT cluster chain exceeds the volume".into(),
                ));
            }
            match self.next_cluster(current)? {
                Some(next) => current = next,
                None => return Ok(chain),
            }
        }
    }

    fn read_chain(&self, first: u32, size: Option<usize>) -> Result<Vec<u8>, StorageError> {
        if first < 2 {
            return Ok(Vec::new());
        }
        let mut output = Vec::new();
        for cluster in self.chain(first)? {
            output.extend_from_slice(self.cluster_bytes(cluster)?);
        }
        if let Some(size) = size {
            if size > output.len() {
                return Err(StorageError::CorruptImage(
                    "FAT file size exceeds its cluster chain".into(),
                ));
            }
            output.truncate(size);
        }
        Ok(output)
    }

    fn short_name(entry: &[u8]) -> Option<String> {
        let base = std::str::from_utf8(&entry[..8]).ok()?.trim_end_matches(' ');
        let ext = std::str::from_utf8(&entry[8..11])
            .ok()?
            .trim_end_matches(' ');
        if base.is_empty() {
            return None;
        }
        Some(if ext.is_empty() {
            base.to_ascii_lowercase()
        } else {
            format!("{}.{}", base.to_ascii_lowercase(), ext.to_ascii_lowercase())
        })
    }

    fn lfn_piece(entry: &[u8]) -> Option<(u8, bool, u8, Vec<u16>)> {
        if entry.len() != 32 || entry[11] != 0x0f || entry[0] & 0x1f == 0 {
            return None;
        }
        let mut units = Vec::with_capacity(13);
        for range in [1..11, 14..26, 28..32] {
            for pair in entry[range].chunks_exact(2) {
                let unit = u16::from_le_bytes([pair[0], pair[1]]);
                if unit == 0 {
                    return Some((entry[0] & 0x1f, entry[0] & 0x40 != 0, entry[13], units));
                }
                if unit != 0xffff {
                    units.push(unit);
                }
            }
        }
        Some((entry[0] & 0x1f, entry[0] & 0x40 != 0, entry[13], units))
    }

    fn short_checksum(entry: &[u8]) -> u8 {
        entry[..11]
            .iter()
            .fold(0u8, |sum, byte| sum.rotate_right(1).wrapping_add(*byte))
    }

    fn walk_directory(
        &self,
        bytes: &[u8],
        prefix: &str,
        depth: usize,
        visited: &mut BTreeSet<u32>,
        output: &mut Vec<ImportedEntry>,
    ) -> Result<(), StorageError> {
        if depth > 64 {
            return Err(StorageError::CorruptImage(
                "FAT directory nesting exceeds 64".into(),
            ));
        }
        let mut lfn = BTreeMap::<u8, Vec<u16>>::new();
        let mut lfn_checksum = None;
        let mut lfn_last = None;
        for entry in bytes.chunks_exact(32) {
            if entry[0] == 0 {
                break;
            }
            if entry[0] == 0xe5 {
                lfn.clear();
                lfn_checksum = None;
                lfn_last = None;
                continue;
            }
            if let Some((ordinal, last, checksum, units)) = Self::lfn_piece(entry) {
                if lfn_checksum.is_some_and(|value| value != checksum) {
                    lfn.clear();
                }
                lfn_checksum = Some(checksum);
                if last {
                    lfn_last = Some(ordinal);
                }
                lfn.insert(ordinal, units);
                continue;
            }
            if entry[11] & 0x08 != 0 {
                lfn.clear();
                lfn_checksum = None;
                lfn_last = None;
                continue;
            }
            let name = if lfn_checksum == Some(Self::short_checksum(entry))
                && lfn_last.is_some_and(|last| {
                    last as usize == lfn.len() && (1..=last).all(|index| lfn.contains_key(&index))
                }) {
                let units = (1..=lfn_last.unwrap())
                    .flat_map(|index| lfn.remove(&index).unwrap_or_default())
                    .collect::<Vec<_>>();
                String::from_utf16(&units)
                    .map_err(|_| StorageError::CorruptImage("FAT LFN is not valid UTF-16".into()))?
            } else {
                Self::short_name(entry).unwrap_or_default()
            };
            lfn.clear();
            lfn_checksum = None;
            lfn_last = None;
            if name.is_empty() {
                continue;
            }
            if matches!(name.as_str(), "." | "..") {
                continue;
            }
            let high = u16::from_le_bytes([entry[20], entry[21]]) as u32;
            let low = u16::from_le_bytes([entry[26], entry[27]]) as u32;
            let cluster = (high << 16) | low;
            let size = u32::from_le_bytes(entry[28..32].try_into().unwrap()) as usize;
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry[11] & 0x10 != 0 {
                output.push(ImportedEntry {
                    path: path.clone(),
                    kind: ImportedKind::Directory,
                    mode: 0o755,
                    uid: 0,
                    gid: 0,
                    data: Vec::new(),
                    acl: Vec::new(),
                });
                if cluster >= 2 && visited.insert(cluster) {
                    let directory = self.read_chain(cluster, None)?;
                    self.walk_directory(&directory, &path, depth + 1, visited, output)?;
                }
            } else {
                output.push(ImportedEntry {
                    path,
                    kind: ImportedKind::File,
                    mode: if entry[11] & 0x01 != 0 { 0o444 } else { 0o644 },
                    uid: 0,
                    gid: 0,
                    data: self.read_chain(cluster, Some(size))?,
                    acl: Vec::new(),
                });
            }
        }
        Ok(())
    }

    fn entries(&self) -> Result<Vec<ImportedEntry>, StorageError> {
        let root = if self.kind == FatKind::Fat32 {
            self.read_chain(self.root_cluster, None)?
        } else {
            checked_slice(self.bytes, self.root_offset, self.root_bytes, "FAT root")?.to_vec()
        };
        let mut output = Vec::new();
        let mut visited = BTreeSet::new();
        if self.root_cluster >= 2 {
            visited.insert(self.root_cluster);
        }
        self.walk_directory(&root, "", 0, &mut visited, &mut output)?;
        Ok(output)
    }
}

struct Ext2Volume<'a> {
    bytes: &'a [u8],
    block_size: usize,
    inode_size: usize,
    inodes_per_group: u32,
    group_table_offset: usize,
}

#[derive(Debug)]
struct Ext2Inode {
    mode: u16,
    uid: u32,
    gid: u32,
    size: usize,
    sectors: u32,
    blocks: [u32; 15],
    inline: [u8; 60],
    file_acl: u32,
}

impl<'a> Ext2Volume<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, StorageError> {
        let superblock = checked_slice(bytes, 1024, 1024, "ext2 superblock")?;
        if le16(superblock, 56, "ext2 magic")? != 0xef53 {
            return Err(StorageError::InvalidImage("invalid ext2 superblock"));
        }
        let log_block = le32(superblock, 24, "ext2 block size")?;
        if log_block > 6 {
            return Err(StorageError::Unsupported("unsupported ext2 block size"));
        }
        let block_size = 1024usize << log_block;
        let inodes_per_group = le32(superblock, 40, "ext2 inodes/group")?;
        let blocks_per_group = le32(superblock, 32, "ext2 blocks/group")?;
        let revision = le32(superblock, 76, "ext2 revision")?;
        let inode_size = if revision == 0 {
            128
        } else {
            le16(superblock, 88, "ext2 inode size")? as usize
        };
        if inodes_per_group == 0
            || blocks_per_group == 0
            || inode_size < 128
            || inode_size > block_size
        {
            return Err(StorageError::CorruptImage(
                "invalid ext2 group or inode geometry".into(),
            ));
        }
        let group_table_offset = if block_size == 1024 { 2048 } else { block_size };
        checked_slice(bytes, group_table_offset, 32, "ext2 group table")?;
        Ok(Self {
            bytes,
            block_size,
            inode_size,
            inodes_per_group,
            group_table_offset,
        })
    }

    fn block(&self, block: u32) -> Result<&'a [u8], StorageError> {
        let offset = block as usize * self.block_size;
        checked_slice(self.bytes, offset, self.block_size, "ext2 block")
    }

    fn inode(&self, number: u32) -> Result<Ext2Inode, StorageError> {
        if number == 0 {
            return Err(StorageError::CorruptImage("ext2 inode zero".into()));
        }
        let index = number - 1;
        let group = index / self.inodes_per_group;
        let local = index % self.inodes_per_group;
        let descriptor = checked_slice(
            self.bytes,
            self.group_table_offset + group as usize * 32,
            32,
            "ext2 group descriptor",
        )?;
        let inode_table = le32(descriptor, 8, "ext2 inode table")?;
        let offset = inode_table as usize * self.block_size + local as usize * self.inode_size;
        let raw = checked_slice(self.bytes, offset, self.inode_size, "ext2 inode")?;
        let mode = le16(raw, 0, "ext2 inode mode")?;
        let uid = le16(raw, 2, "ext2 inode uid")? as u32;
        let size = le32(raw, 4, "ext2 inode size")? as usize;
        let gid = le16(raw, 24, "ext2 inode gid")? as u32;
        let mut blocks = [0u32; 15];
        let mut inline = [0u8; 60];
        inline.copy_from_slice(&raw[40..100]);
        for (index, slot) in blocks.iter_mut().enumerate() {
            *slot = le32(raw, 40 + index * 4, "ext2 inode block")?;
        }
        Ok(Ext2Inode {
            mode,
            uid,
            gid,
            size,
            sectors: le32(raw, 28, "ext2 inode sector count")?,
            blocks,
            inline,
            file_acl: le32(raw, 104, "ext2 inode ACL block")?,
        })
    }

    fn collect_indirect(
        &self,
        pointer: u32,
        depth: u8,
        needed: usize,
        output: &mut Vec<Option<u32>>,
    ) -> Result<(), StorageError> {
        if output.len() >= needed {
            return Ok(());
        }
        let pointers_per_block = self.block_size / 4;
        if pointer == 0 {
            let capacity = pointers_per_block.saturating_pow(depth as u32);
            output.resize(needed.min(output.len().saturating_add(capacity)), None);
            return Ok(());
        }
        let block = self.block(pointer)?;
        for chunk in block.chunks_exact(4) {
            if output.len() >= needed {
                break;
            }
            let child = u32::from_le_bytes(chunk.try_into().unwrap());
            if depth == 1 {
                output.push((child != 0).then_some(child));
            } else {
                self.collect_indirect(child, depth - 1, needed, output)?;
            }
        }
        Ok(())
    }

    fn inode_data(&self, inode: &Ext2Inode) -> Result<Vec<u8>, StorageError> {
        if inode.mode & 0xf000 == 0xa000 && inode.size <= inode.inline.len() && inode.sectors == 0 {
            return Ok(inode.inline[..inode.size].to_vec());
        }
        let needed = inode.size.div_ceil(self.block_size);
        let mut block_numbers = inode.blocks[..12]
            .iter()
            .copied()
            .take(needed)
            .map(|block| (block != 0).then_some(block))
            .collect::<Vec<_>>();
        for (pointer, depth) in inode.blocks[12..15].iter().copied().zip(1..=3) {
            self.collect_indirect(pointer, depth, needed, &mut block_numbers)?;
        }
        if block_numbers.len() < needed {
            return Err(StorageError::CorruptImage(
                "ext2 inode pointer tree is shorter than i_size".into(),
            ));
        }
        let mut output = Vec::with_capacity(inode.size);
        for block in block_numbers.into_iter().take(needed) {
            if let Some(block) = block {
                output.extend_from_slice(self.block(block)?);
            } else {
                output.resize(output.len() + self.block_size, 0);
            }
        }
        output.truncate(inode.size);
        Ok(output)
    }

    fn inode_acl(&self, inode: &Ext2Inode) -> Result<Vec<VfsAclEntry>, StorageError> {
        if inode.file_acl == 0 {
            return Ok(Vec::new());
        }
        let block = self.block(inode.file_acl)?;
        if le32(block, 0, "ext2 xattr magic")? != 0xea02_0000 {
            return Err(StorageError::CorruptImage(
                "invalid ext2 xattr magic".into(),
            ));
        }
        let mut offset = 32usize;
        while offset + 16 <= block.len() && le32(block, offset, "ext2 xattr entry")? != 0 {
            let name_len = block[offset] as usize;
            let name_index = block[offset + 1];
            let value_offset = le16(block, offset + 2, "ext2 xattr value offset")? as usize;
            let value_block = le32(block, offset + 4, "ext2 xattr value block")?;
            let value_size = le32(block, offset + 8, "ext2 xattr value size")? as usize;
            if value_block == 0 && name_index == 2 && name_len == 0 {
                let value = checked_slice(block, value_offset, value_size, "ext2 POSIX ACL")?;
                if le32(value, 0, "ext2 ACL version")? != 2 {
                    return Err(StorageError::CorruptImage(
                        "unsupported ext2 ACL version".into(),
                    ));
                }
                let mut cursor = 4usize;
                let mut acl = Vec::new();
                while cursor + 4 <= value.len() {
                    let tag = le16(value, cursor, "ext2 ACL tag")?;
                    let permissions = le16(value, cursor + 2, "ext2 ACL permissions")? as u8;
                    let (tag, id, width) = match tag {
                        0x01 => (VfsAclTag::User, None, 4),
                        0x02 => (
                            VfsAclTag::User,
                            Some(le32(value, cursor + 4, "ext2 ACL user")?),
                            8,
                        ),
                        0x04 => (VfsAclTag::Group, None, 4),
                        0x08 => (
                            VfsAclTag::Group,
                            Some(le32(value, cursor + 4, "ext2 ACL group")?),
                            8,
                        ),
                        0x10 => (VfsAclTag::Mask, None, 4),
                        0x20 => (VfsAclTag::Other, None, 4),
                        _ => return Err(StorageError::CorruptImage("invalid ext2 ACL tag".into())),
                    };
                    if permissions > 0o7 || cursor + width > value.len() {
                        return Err(StorageError::CorruptImage("invalid ext2 ACL entry".into()));
                    }
                    acl.push(VfsAclEntry {
                        tag,
                        id,
                        permissions,
                    });
                    cursor += width;
                }
                return Ok(acl);
            }
            offset = align_up(offset + 16 + name_len, 4)?;
        }
        Ok(Vec::new())
    }

    fn walk(
        &self,
        inode_number: u32,
        prefix: &str,
        depth: usize,
        visited: &mut BTreeSet<u32>,
        output: &mut Vec<ImportedEntry>,
    ) -> Result<(), StorageError> {
        if depth > 64 || !visited.insert(inode_number) {
            return Err(StorageError::CorruptImage(
                "ext2 directory loop or excessive depth".into(),
            ));
        }
        let inode = self.inode(inode_number)?;
        if inode.mode & 0xf000 != 0x4000 {
            return Err(StorageError::CorruptImage(
                "ext2 directory entry references a non-directory".into(),
            ));
        }
        let data = self.inode_data(&inode)?;
        let mut offset = 0usize;
        while offset < data.len() {
            let inode_number = le32(&data, offset, "ext2 directory inode")?;
            let record = le16(&data, offset + 4, "ext2 directory record")? as usize;
            let name_len = data
                .get(offset + 6)
                .copied()
                .ok_or_else(|| StorageError::CorruptImage("ext2 directory name".into()))?
                as usize;
            if record < 8
                || record % 4 != 0
                || offset + record > data.len()
                || name_len > record - 8
            {
                return Err(StorageError::CorruptImage(
                    "invalid ext2 directory record".into(),
                ));
            }
            if inode_number != 0 {
                let name = std::str::from_utf8(&data[offset + 8..offset + 8 + name_len])
                    .map_err(|_| StorageError::CorruptImage("ext2 name is not UTF-8".into()))?;
                if !matches!(name, "." | "..") {
                    let name = archive_path(name)?;
                    let path = if prefix.is_empty() {
                        name
                    } else {
                        format!("{prefix}/{name}")
                    };
                    let child = self.inode(inode_number)?;
                    match child.mode & 0xf000 {
                        0x4000 => {
                            output.push(ImportedEntry {
                                path: path.clone(),
                                kind: ImportedKind::Directory,
                                mode: child.mode & 0o7777,
                                uid: child.uid,
                                gid: child.gid,
                                data: Vec::new(),
                                acl: self.inode_acl(&child)?,
                            });
                            self.walk(inode_number, &path, depth + 1, visited, output)?;
                        }
                        0x8000 => output.push(ImportedEntry {
                            path,
                            kind: ImportedKind::File,
                            mode: child.mode & 0o7777,
                            uid: child.uid,
                            gid: child.gid,
                            data: self.inode_data(&child)?,
                            acl: self.inode_acl(&child)?,
                        }),
                        0xa000 => output.push(ImportedEntry {
                            path,
                            kind: ImportedKind::Symlink,
                            mode: child.mode & 0o7777,
                            uid: child.uid,
                            gid: child.gid,
                            data: self.inode_data(&child)?,
                            acl: self.inode_acl(&child)?,
                        }),
                        _ => {}
                    }
                }
            }
            offset += record;
        }
        visited.remove(&inode_number);
        Ok(())
    }

    fn entries(&self) -> Result<Vec<ImportedEntry>, StorageError> {
        let mut output = Vec::new();
        self.walk(2, "", 0, &mut BTreeSet::new(), &mut output)?;
        Ok(output)
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    bytes: Vec<u8>,
    dirty: bool,
    last_use: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlockCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub writebacks: u64,
}

#[derive(Debug, Clone)]
pub struct BlockCache {
    device: VirtualBlockDevice,
    block_size: usize,
    capacity: usize,
    entries: BTreeMap<u64, CacheEntry>,
    clock: u64,
    pub stats: BlockCacheStats,
}

impl BlockCache {
    pub fn new(
        device: VirtualBlockDevice,
        block_size: usize,
        capacity: usize,
    ) -> Result<Self, StorageError> {
        if capacity == 0 {
            return Err(StorageError::CacheCapacity);
        }
        if block_size == 0
            || !block_size.is_power_of_two()
            || block_size % device.sector_size as usize != 0
            || device.capacity() % block_size != 0
        {
            return Err(StorageError::InvalidImage(
                "block cache geometry is not sector aligned",
            ));
        }
        Ok(Self {
            device,
            block_size,
            capacity,
            entries: BTreeMap::new(),
            clock: 0,
            stats: BlockCacheStats::default(),
        })
    }

    fn offset(&self, block: u64) -> Result<u64, StorageError> {
        let offset = block
            .checked_mul(self.block_size as u64)
            .ok_or(StorageError::InvalidImage("block offset overflow"))?;
        if offset + self.block_size as u64 > self.device.capacity() as u64 {
            return Err(StorageError::InvalidImage("block is outside the device"));
        }
        Ok(offset)
    }

    fn evict_one(&mut self) -> Result<(), StorageError> {
        let victim = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_use)
            .map(|(block, _)| *block)
            .ok_or(StorageError::CacheCapacity)?;
        let entry = self.entries.remove(&victim).unwrap();
        if entry.dirty {
            let offset = self.offset(victim)?;
            self.device.write(offset, &entry.bytes)?;
            self.stats.writebacks = self.stats.writebacks.wrapping_add(1);
        }
        self.stats.evictions = self.stats.evictions.wrapping_add(1);
        Ok(())
    }

    fn ensure(&mut self, block: u64) -> Result<(), StorageError> {
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&block) {
            entry.last_use = self.clock;
            self.stats.hits = self.stats.hits.wrapping_add(1);
            return Ok(());
        }
        self.stats.misses = self.stats.misses.wrapping_add(1);
        if self.entries.len() == self.capacity {
            self.evict_one()?;
        }
        let mut bytes = vec![0; self.block_size];
        self.device.read(self.offset(block)?, &mut bytes)?;
        self.entries.insert(
            block,
            CacheEntry {
                bytes,
                dirty: false,
                last_use: self.clock,
            },
        );
        Ok(())
    }

    pub fn read_block(&mut self, block: u64) -> Result<Vec<u8>, StorageError> {
        self.ensure(block)?;
        Ok(self.entries[&block].bytes.clone())
    }

    pub fn write_block(&mut self, block: u64, bytes: &[u8]) -> Result<(), StorageError> {
        if self.device.read_only {
            return Err(StorageError::ReadOnly);
        }
        if bytes.len() != self.block_size {
            return Err(StorageError::InvalidImage(
                "cached write must contain exactly one block",
            ));
        }
        self.ensure(block)?;
        let entry = self.entries.get_mut(&block).unwrap();
        entry.bytes.copy_from_slice(bytes);
        entry.dirty = true;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), StorageError> {
        let dirty = self
            .entries
            .iter()
            .filter_map(|(block, entry)| entry.dirty.then_some(*block))
            .collect::<Vec<_>>();
        for block in dirty {
            let offset = self.offset(block)?;
            let entry = self.entries.get_mut(&block).unwrap();
            self.device.write(offset, &entry.bytes)?;
            entry.dirty = false;
            self.stats.writebacks = self.stats.writebacks.wrapping_add(1);
        }
        self.device.flush();
        Ok(())
    }

    pub fn invalidate(&mut self) -> Result<(), StorageError> {
        self.flush()?;
        self.entries.clear();
        Ok(())
    }

    pub fn device(&self) -> &VirtualBlockDevice {
        &self.device
    }
}

#[derive(Debug, Clone)]
pub struct MountedVfs {
    pub vfs: Vfs,
    mounts: BTreeMap<String, MountInfo>,
    persistent: BTreeMap<String, PersistentMount>,
}

impl Default for MountedVfs {
    fn default() -> Self {
        Self {
            vfs: Vfs::default(),
            mounts: BTreeMap::new(),
            persistent: BTreeMap::new(),
        }
    }
}

impl Deref for MountedVfs {
    type Target = Vfs;

    fn deref(&self) -> &Self::Target {
        &self.vfs
    }
}

impl DerefMut for MountedVfs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vfs
    }
}

impl MountedVfs {
    fn apply_mount(
        &mut self,
        mount: &str,
        source: &str,
        kind: FileSystemKind,
        read_only: bool,
        mut entries: Vec<ImportedEntry>,
    ) -> Result<MountInfo, StorageError> {
        let mount = if mount == "/" {
            "/".to_string()
        } else {
            let trimmed = mount.trim_end_matches('/');
            if !trimmed.starts_with('/') || trimmed.contains("..") || trimmed.contains('\0') {
                return Err(StorageError::InvalidImage("invalid mount point"));
            }
            trimmed.to_string()
        };
        if self.mounts.contains_key(&mount) {
            return Err(StorageError::MountExists(mount));
        }
        self.vfs.mkdir_all(&mount, 0o755)?;
        if !self.vfs.list(&mount)?.is_empty() {
            return Err(StorageError::MountBusy(mount));
        }
        entries.sort_by_key(|entry| {
            (
                entry.path.matches('/').count(),
                entry.kind != ImportedKind::Directory,
                entry.path.clone(),
            )
        });
        for entry in &entries {
            let path = mount_join(&mount, &entry.path);
            match entry.kind {
                ImportedKind::Directory => {
                    self.vfs.mkdir_all(&path, entry.mode)?;
                    self.vfs.chown(
                        &path,
                        VfsCredentials {
                            uid: entry.uid,
                            gid: entry.gid,
                        },
                    )?;
                    self.vfs.chmod(&path, entry.mode)?;
                }
                ImportedKind::File => self.vfs.import_file(
                    &path,
                    if read_only {
                        entry.mode & !0o222
                    } else {
                        entry.mode
                    },
                    VfsCredentials {
                        uid: entry.uid,
                        gid: entry.gid,
                    },
                    &entry.data,
                )?,
                ImportedKind::Symlink => {
                    let target = std::str::from_utf8(&entry.data).map_err(|_| {
                        StorageError::CorruptImage("symlink target is not UTF-8".into())
                    })?;
                    let target = if target.starts_with('/') {
                        target.to_string()
                    } else {
                        mount_join(&mount, &archive_path(target)?)
                    };
                    self.vfs.symlink(&target, &path)?;
                }
            }
            if !entry.acl.is_empty() {
                self.vfs.set_acl(&path, entry.acl.clone())?;
            }
        }
        let info = MountInfo {
            path: mount.clone(),
            source: source.to_string(),
            kind,
            read_only,
            files: entries
                .iter()
                .filter(|entry| entry.kind == ImportedKind::File)
                .count(),
        };
        self.mounts.insert(mount, info.clone());
        Ok(info)
    }

    pub fn mount_initramfs(
        &mut self,
        mount: &str,
        source: &str,
        image: &[u8],
    ) -> Result<MountInfo, StorageError> {
        self.apply_mount(
            mount,
            source,
            FileSystemKind::InitramfsNewc,
            true,
            parse_newc(image)?,
        )
    }

    pub fn mount_fat(
        &mut self,
        mount: &str,
        source: &str,
        image: &[u8],
        read_only: bool,
    ) -> Result<MountInfo, StorageError> {
        let probe = FatVolume::parse(image)?;
        let logical_bytes = probe.total_sectors * probe.bytes_per_sector;
        let mut persistent = (!read_only)
            .then(|| {
                PersistentMount::with_image(
                    PersistentFormat::Fat,
                    image,
                    logical_bytes,
                    probe.bytes_per_sector,
                )
            })
            .transpose()?;
        let bytes = persistent
            .as_ref()
            .map_or(&image[..logical_bytes], |mount| {
                &mount.device.bytes()[..logical_bytes]
            });
        let volume = FatVolume::parse(bytes)?;
        let kind = match volume.kind {
            FatKind::Fat12 => FileSystemKind::Fat12,
            FatKind::Fat16 => FileSystemKind::Fat16,
            FatKind::Fat32 => FileSystemKind::Fat32,
        };
        let info = self.apply_mount(mount, source, kind, read_only, volume.entries()?)?;
        if let Some(mut persistent) = persistent.take() {
            persistent.baseline = self.vfs.snapshot_under(&info.path)?;
            self.persistent.insert(info.path.clone(), persistent);
        }
        Ok(info)
    }

    pub fn mount_ext2(
        &mut self,
        mount: &str,
        source: &str,
        image: &[u8],
    ) -> Result<MountInfo, StorageError> {
        self.mount_ext2_mode(mount, source, image, true)
    }

    pub fn mount_ext2_mode(
        &mut self,
        mount: &str,
        source: &str,
        image: &[u8],
        read_only: bool,
    ) -> Result<MountInfo, StorageError> {
        let probe = Ext2Volume::parse(image)?;
        let blocks_count = le32(image, 1028, "ext2 block count")? as usize;
        let logical_bytes = blocks_count
            .checked_mul(probe.block_size)
            .ok_or(StorageError::InvalidImage("ext2 logical size overflow"))?;
        let mut persistent = (!read_only)
            .then(|| {
                PersistentMount::with_image(
                    PersistentFormat::Ext2,
                    image,
                    logical_bytes,
                    probe.block_size,
                )
            })
            .transpose()?;
        let bytes = persistent
            .as_ref()
            .map_or(&image[..logical_bytes], |mount| {
                &mount.device.bytes()[..logical_bytes]
            });
        let volume = Ext2Volume::parse(bytes)?;
        let info = self.apply_mount(
            mount,
            source,
            FileSystemKind::Ext2,
            read_only,
            volume.entries()?,
        )?;
        if let Some(mut persistent) = persistent.take() {
            persistent.baseline = self.vfs.snapshot_under(&info.path)?;
            self.persistent.insert(info.path.clone(), persistent);
        }
        Ok(info)
    }

    pub fn mounts(&self) -> Vec<MountInfo> {
        self.mounts.values().cloned().collect()
    }

    pub fn unmount(&mut self, mount: &str) -> Result<MountInfo, StorageError> {
        let info = self
            .mounts
            .get(mount)
            .cloned()
            .ok_or_else(|| StorageError::MountMissing(mount.to_string()))?;
        if self.vfs.has_open_handles_under(mount)? {
            return Err(StorageError::MountBusy(mount.to_string()));
        }
        if let Some(persistent) = self.persistent.get(mount) {
            let current = self.vfs.snapshot_under(mount)?;
            if current != persistent.baseline {
                return Err(StorageError::MountDirty(mount.to_string()));
            }
        }
        self.vfs.remove_tree(mount)?;
        self.vfs.mkdir_all(mount, 0o755)?;
        self.mounts.remove(mount);
        self.persistent.remove(mount);
        Ok(info)
    }

    pub fn sync_mount(&mut self, mount: &str) -> Result<StorageSyncReport, StorageError> {
        self.sync_mount_with_fault(mount, StorageSyncFault::None)
    }

    pub fn sync_mount_with_fault(
        &mut self,
        mount: &str,
        fault: StorageSyncFault,
    ) -> Result<StorageSyncReport, StorageError> {
        let snapshot = self.vfs.snapshot_under(mount)?;
        let persistent = self.persistent.get_mut(mount).ok_or_else(|| {
            if self.mounts.contains_key(mount) {
                StorageError::ReadOnly
            } else {
                StorageError::MountMissing(mount.to_string())
            }
        })?;
        let mut report = persistent.synchronize(&snapshot, fault)?;
        report.mount = mount.to_string();
        Ok(report)
    }

    pub fn recover_mount(&mut self, mount: &str) -> Result<usize, StorageError> {
        self.persistent
            .get_mut(mount)
            .ok_or_else(|| StorageError::MountMissing(mount.to_string()))?
            .recover()
    }

    pub fn filesystem_image(&self, mount: &str) -> Result<&[u8], StorageError> {
        let persistent = self
            .persistent
            .get(mount)
            .ok_or_else(|| StorageError::MountMissing(mount.to_string()))?;
        Ok(&persistent.device.bytes()[..persistent.logical_bytes])
    }

    pub fn persistent_device_image(&self, mount: &str) -> Result<&[u8], StorageError> {
        Ok(self
            .persistent
            .get(mount)
            .ok_or_else(|| StorageError::MountMissing(mount.to_string()))?
            .device
            .bytes())
    }

    pub fn unmount_discard(&mut self, mount: &str) -> Result<MountInfo, StorageError> {
        if self.vfs.has_open_handles_under(mount)? {
            return Err(StorageError::MountBusy(mount.to_string()));
        }
        let info = self
            .mounts
            .remove(mount)
            .ok_or_else(|| StorageError::MountMissing(mount.to_string()))?;
        self.persistent.remove(mount);
        self.vfs.remove_tree(mount)?;
        self.vfs.mkdir_all(mount, 0o755)?;
        Ok(info)
    }

    pub fn open_as(
        &mut self,
        path: &str,
        flags: OpenFlags,
        credentials: VfsCredentials,
    ) -> Result<u32, StorageError> {
        Ok(self.vfs.open_as(path, flags, credentials)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCertificationReport {
    pub filesystems: Vec<FileSystemKind>,
    pub files_materialized: usize,
    pub permission_rejections: u64,
    pub busy_unmount_rejections: u64,
    pub cache: BlockCacheStats,
    pub persisted_bytes: usize,
    pub writable_filesystems: Vec<FileSystemKind>,
    pub journal_commits: usize,
    pub journal_replays: usize,
    pub indirect_blocks: usize,
    pub acl_blocks: usize,
    pub long_names: usize,
}

fn certification_push_newc(output: &mut Vec<u8>, name: &str, mode: u32, data: &[u8]) {
    output.extend_from_slice(b"070701");
    for field in [
        1u32,
        mode,
        1000,
        100,
        1,
        0,
        data.len() as u32,
        0,
        0,
        0,
        0,
        name.len() as u32 + 1,
        0,
    ] {
        output.extend_from_slice(format!("{field:08x}").as_bytes());
    }
    output.extend_from_slice(name.as_bytes());
    output.push(0);
    while output.len() % 4 != 0 {
        output.push(0);
    }
    output.extend_from_slice(data);
    while output.len() % 4 != 0 {
        output.push(0);
    }
}

fn certification_newc() -> Vec<u8> {
    let mut bytes = Vec::new();
    certification_push_newc(&mut bytes, "etc", 0o040755, &[]);
    certification_push_newc(&mut bytes, "etc/issue", 0o100640, b"nC initramfs\n");
    certification_push_newc(&mut bytes, "TRAILER!!!", 0, &[]);
    bytes
}

pub fn create_fat12_writable_image() -> Vec<u8> {
    let mut bytes = vec![0u8; 512 * 8];
    bytes[0] = 0xeb;
    bytes[11..13].copy_from_slice(&512u16.to_le_bytes());
    bytes[13] = 1;
    bytes[14..16].copy_from_slice(&1u16.to_le_bytes());
    bytes[16] = 1;
    bytes[17..19].copy_from_slice(&16u16.to_le_bytes());
    bytes[19..21].copy_from_slice(&8u16.to_le_bytes());
    bytes[21] = 0xf8;
    bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
    bytes[510..512].copy_from_slice(&[0x55, 0xaa]);
    bytes[512..515].copy_from_slice(&[0xf8, 0xff, 0xff]);
    bytes[515..517].copy_from_slice(&[0xff, 0x0f]);
    let root = 1024;
    bytes[root..root + 11].copy_from_slice(b"KERNEL  N  ");
    bytes[root + 26..root + 28].copy_from_slice(&2u16.to_le_bytes());
    bytes[root + 28..root + 32].copy_from_slice(&6u32.to_le_bytes());
    bytes[1536..1542].copy_from_slice(b"nC FAT");
    bytes
}

pub fn create_ext2_writable_image() -> Vec<u8> {
    let blocks = 1024u32;
    let inodes = 128u32;
    let mut bytes = vec![0u8; blocks as usize * 1024];
    let sb = 1024;
    bytes[sb..sb + 4].copy_from_slice(&inodes.to_le_bytes());
    bytes[sb + 4..sb + 8].copy_from_slice(&blocks.to_le_bytes());
    bytes[sb + 20..sb + 24].copy_from_slice(&1u32.to_le_bytes());
    bytes[sb + 32..sb + 36].copy_from_slice(&blocks.to_le_bytes());
    bytes[sb + 40..sb + 44].copy_from_slice(&inodes.to_le_bytes());
    bytes[sb + 56..sb + 58].copy_from_slice(&0xef53u16.to_le_bytes());
    bytes[sb + 58..sb + 60].copy_from_slice(&1u16.to_le_bytes());
    bytes[sb + 76..sb + 80].copy_from_slice(&1u32.to_le_bytes());
    bytes[sb + 84..sb + 88].copy_from_slice(&11u32.to_le_bytes());
    bytes[sb + 88..sb + 90].copy_from_slice(&128u16.to_le_bytes());
    bytes[sb + 96..sb + 100].copy_from_slice(&2u32.to_le_bytes());
    bytes[2048..2052].copy_from_slice(&3u32.to_le_bytes());
    bytes[2052..2056].copy_from_slice(&4u32.to_le_bytes());
    bytes[2056..2060].copy_from_slice(&5u32.to_le_bytes());
    let root_inode = 5 * 1024 + 128;
    bytes[root_inode..root_inode + 2].copy_from_slice(&0x41edu16.to_le_bytes());
    bytes[root_inode + 4..root_inode + 8].copy_from_slice(&1024u32.to_le_bytes());
    bytes[root_inode + 26..root_inode + 28].copy_from_slice(&2u16.to_le_bytes());
    bytes[root_inode + 28..root_inode + 32].copy_from_slice(&2u32.to_le_bytes());
    bytes[root_inode + 40..root_inode + 44].copy_from_slice(&21u32.to_le_bytes());
    let file_inode = 5 * 1024 + 2 * 128;
    bytes[file_inode..file_inode + 2].copy_from_slice(&0x81a4u16.to_le_bytes());
    bytes[file_inode + 4..file_inode + 8].copy_from_slice(&7u32.to_le_bytes());
    bytes[file_inode + 26..file_inode + 28].copy_from_slice(&1u16.to_le_bytes());
    bytes[file_inode + 28..file_inode + 32].copy_from_slice(&2u32.to_le_bytes());
    bytes[file_inode + 40..file_inode + 44].copy_from_slice(&22u32.to_le_bytes());
    let mut offset = 21 * 1024;
    for (inode, record, name) in [
        (2u32, 12u16, b".".as_slice()),
        (2, 12, b".."),
        (3, 1000, b"hello"),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&inode.to_le_bytes());
        bytes[offset + 4..offset + 6].copy_from_slice(&record.to_le_bytes());
        bytes[offset + 6] = name.len() as u8;
        bytes[offset + 7] = if inode == 3 { 1 } else { 2 };
        bytes[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
        offset += record as usize;
    }
    bytes[22 * 1024..22 * 1024 + 7].copy_from_slice(b"nC ext2");
    bytes
}

pub fn certify_storage_contracts() -> Result<StorageCertificationReport, StorageError> {
    let mut mounted = MountedVfs::default();
    let init = mounted.mount_initramfs("/init", "boot:initrd", &certification_newc())?;
    let mut permission_rejections = 0;
    if matches!(
        mounted.open_as(
            "/init/etc/issue",
            OpenFlags::WRITE,
            VfsCredentials {
                uid: 2000,
                gid: 2000
            }
        ),
        Err(StorageError::Io(KernelIoError::PermissionDenied(_)))
    ) {
        permission_rejections += 1;
    }
    let handle = mounted.open_as(
        "/init/etc/issue",
        OpenFlags::READ,
        VfsCredentials {
            uid: 1000,
            gid: 100,
        },
    )?;
    let mut busy_unmount_rejections = 0;
    if matches!(mounted.unmount("/init"), Err(StorageError::MountBusy(_))) {
        busy_unmount_rejections += 1;
    }
    mounted.vfs.close(handle)?;
    mounted.unmount("/init")?;
    let fat = mounted.mount_fat("/fat", "disk0p1", &create_fat12_writable_image(), true)?;
    mounted.unmount("/fat")?;
    let ext = mounted.mount_ext2("/ext", "disk0p2", &create_ext2_writable_image())?;
    if mounted.vfs.file_bytes("/ext/hello")? != b"nC ext2" {
        return Err(StorageError::CorruptImage(
            "ext2 certification payload mismatch".into(),
        ));
    }
    mounted.unmount("/ext")?;

    mounted.mount_fat("/fatw", "disk0p1", &create_fat12_writable_image(), false)?;
    let fat_path = "/fatw/manual-persistent-long-name.txt";
    let fat_handle = mounted.open(fat_path, OpenFlags::WRITE.union(OpenFlags::CREATE))?;
    mounted.write(fat_handle, b"fat journal")?;
    mounted.close(fat_handle)?;
    let fat_sync = mounted.sync_mount("/fatw")?;
    let fat_handle = mounted.open(fat_path, OpenFlags::WRITE.union(OpenFlags::TRUNCATE))?;
    mounted.write(fat_handle, b"fat recovered")?;
    mounted.close(fat_handle)?;
    if !matches!(
        mounted.sync_mount_with_fault("/fatw", StorageSyncFault::AfterJournalCommit),
        Err(StorageError::TransactionInterrupted)
    ) {
        return Err(StorageError::JournalCorrupt(
            "fault injection did not interrupt FAT transaction".into(),
        ));
    }
    let replayed = mounted.recover_mount("/fatw")?;
    let fat_after_recovery = mounted.sync_mount("/fatw")?;

    mounted.mount_ext2_mode("/extw", "disk0p2", &create_ext2_writable_image(), false)?;
    let ext_path = "/extw/manual-kernel-double-indirect-payload.bin";
    let ext_handle = mounted.open(ext_path, OpenFlags::WRITE.union(OpenFlags::CREATE))?;
    let ext_payload = (0..300_000usize)
        .map(|index| (index.wrapping_mul(17) & 0xff) as u8)
        .collect::<Vec<_>>();
    mounted.write(ext_handle, &ext_payload)?;
    mounted.close(ext_handle)?;
    mounted.set_acl(
        ext_path,
        vec![
            VfsAclEntry {
                tag: VfsAclTag::User,
                id: Some(1000),
                permissions: 0o4,
            },
            VfsAclEntry {
                tag: VfsAclTag::Mask,
                id: None,
                permissions: 0o4,
            },
        ],
    )?;
    let ext_sync = mounted.sync_mount("/extw")?;

    let device = VirtualBlockDevice::new(4096, 512, false)?;
    let mut cache = BlockCache::new(device, 512, 2)?;
    cache.write_block(0, &[0x11; 512])?;
    cache.write_block(1, &[0x22; 512])?;
    let _ = cache.read_block(0)?;
    cache.write_block(2, &[0x33; 512])?;
    cache.flush()?;
    Ok(StorageCertificationReport {
        filesystems: vec![init.kind, fat.kind, ext.kind],
        files_materialized: init.files + fat.files + ext.files,
        permission_rejections,
        busy_unmount_rejections,
        cache: cache.stats,
        persisted_bytes: cache.device().written_bytes as usize
            + fat_sync.bytes_written as usize
            + fat_after_recovery.bytes_written as usize
            + ext_sync.bytes_written as usize,
        writable_filesystems: vec![fat.kind, ext.kind],
        journal_commits: fat_sync.journal_commits
            + fat_after_recovery.journal_commits
            + ext_sync.journal_commits,
        journal_replays: usize::from(replayed > 0),
        indirect_blocks: ext_sync.indirect_blocks,
        acl_blocks: ext_sync.acl_blocks,
        long_names: fat_sync.long_names + ext_sync.long_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_newc_entry(output: &mut Vec<u8>, name: &str, mode: u32, data: &[u8]) {
        let fields = [
            1u32,
            mode,
            1000,
            100,
            1,
            0,
            data.len() as u32,
            0,
            0,
            0,
            0,
            name.len() as u32 + 1,
            0,
        ];
        output.extend_from_slice(b"070701");
        for field in fields {
            output.extend_from_slice(format!("{field:08x}").as_bytes());
        }
        output.extend_from_slice(name.as_bytes());
        output.push(0);
        while output.len() % 4 != 0 {
            output.push(0);
        }
        output.extend_from_slice(data);
        while output.len() % 4 != 0 {
            output.push(0);
        }
    }

    fn newc_fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_newc_entry(&mut bytes, "etc", 0o040755, &[]);
        push_newc_entry(&mut bytes, "etc/issue", 0o100640, b"nC initramfs\n");
        push_newc_entry(&mut bytes, "TRAILER!!!", 0, &[]);
        bytes
    }

    fn fat12_fixture() -> Vec<u8> {
        let mut bytes = vec![0u8; 512 * 8];
        bytes[0] = 0xeb;
        bytes[11..13].copy_from_slice(&512u16.to_le_bytes());
        bytes[13] = 1;
        bytes[14..16].copy_from_slice(&1u16.to_le_bytes());
        bytes[16] = 1;
        bytes[17..19].copy_from_slice(&16u16.to_le_bytes());
        bytes[19..21].copy_from_slice(&8u16.to_le_bytes());
        bytes[21] = 0xf8;
        bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes[510..512].copy_from_slice(&[0x55, 0xaa]);
        bytes[512..515].copy_from_slice(&[0xf8, 0xff, 0xff]);
        bytes[515..517].copy_from_slice(&[0xff, 0x0f]);
        let root = 1024;
        bytes[root..root + 11].copy_from_slice(b"KERNEL  N  ");
        bytes[root + 26..root + 28].copy_from_slice(&2u16.to_le_bytes());
        bytes[root + 28..root + 32].copy_from_slice(&6u32.to_le_bytes());
        bytes[1536..1542].copy_from_slice(b"nC FAT");
        bytes
    }

    fn ext2_fixture() -> Vec<u8> {
        let mut bytes = vec![0u8; 16 * 1024];
        let sb = 1024;
        bytes[sb..sb + 4].copy_from_slice(&32u32.to_le_bytes());
        bytes[sb + 4..sb + 8].copy_from_slice(&16u32.to_le_bytes());
        bytes[sb + 24..sb + 28].copy_from_slice(&0u32.to_le_bytes());
        bytes[sb + 32..sb + 36].copy_from_slice(&16u32.to_le_bytes());
        bytes[sb + 40..sb + 44].copy_from_slice(&32u32.to_le_bytes());
        bytes[sb + 56..sb + 58].copy_from_slice(&0xef53u16.to_le_bytes());
        bytes[sb + 76..sb + 80].copy_from_slice(&1u32.to_le_bytes());
        bytes[sb + 88..sb + 90].copy_from_slice(&128u16.to_le_bytes());
        let gd = 2048;
        bytes[gd + 8..gd + 12].copy_from_slice(&5u32.to_le_bytes());
        let root_inode = 5 * 1024 + 128;
        bytes[root_inode..root_inode + 2].copy_from_slice(&0x41edu16.to_le_bytes());
        bytes[root_inode + 4..root_inode + 8].copy_from_slice(&1024u32.to_le_bytes());
        bytes[root_inode + 40..root_inode + 44].copy_from_slice(&7u32.to_le_bytes());
        let file_inode = 5 * 1024 + 2 * 128;
        bytes[file_inode..file_inode + 2].copy_from_slice(&0x81a4u16.to_le_bytes());
        bytes[file_inode + 4..file_inode + 8].copy_from_slice(&7u32.to_le_bytes());
        bytes[file_inode + 40..file_inode + 44].copy_from_slice(&8u32.to_le_bytes());
        let dir = 7 * 1024;
        let records = [
            (2u32, 12u16, b".".as_slice()),
            (2, 12, b".."),
            (3, 1000, b"hello"),
        ];
        let mut offset = dir;
        for (inode, record, name) in records {
            bytes[offset..offset + 4].copy_from_slice(&inode.to_le_bytes());
            bytes[offset + 4..offset + 6].copy_from_slice(&record.to_le_bytes());
            bytes[offset + 6] = name.len() as u8;
            bytes[offset + 7] = if inode == 3 { 1 } else { 2 };
            bytes[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
            offset += record as usize;
        }
        bytes[8 * 1024..8 * 1024 + 7].copy_from_slice(b"nC ext2");
        bytes
    }

    fn writable_ext2_fixture() -> Vec<u8> {
        let blocks = 1024u32;
        let inodes = 128u32;
        let mut bytes = vec![0u8; blocks as usize * 1024];
        let sb = 1024;
        bytes[sb..sb + 4].copy_from_slice(&inodes.to_le_bytes());
        bytes[sb + 4..sb + 8].copy_from_slice(&blocks.to_le_bytes());
        bytes[sb + 20..sb + 24].copy_from_slice(&1u32.to_le_bytes());
        bytes[sb + 24..sb + 28].copy_from_slice(&0u32.to_le_bytes());
        bytes[sb + 32..sb + 36].copy_from_slice(&blocks.to_le_bytes());
        bytes[sb + 40..sb + 44].copy_from_slice(&inodes.to_le_bytes());
        bytes[sb + 56..sb + 58].copy_from_slice(&0xef53u16.to_le_bytes());
        bytes[sb + 58..sb + 60].copy_from_slice(&1u16.to_le_bytes());
        bytes[sb + 76..sb + 80].copy_from_slice(&1u32.to_le_bytes());
        bytes[sb + 84..sb + 88].copy_from_slice(&11u32.to_le_bytes());
        bytes[sb + 88..sb + 90].copy_from_slice(&128u16.to_le_bytes());
        bytes[sb + 96..sb + 100].copy_from_slice(&2u32.to_le_bytes());
        let gd = 2048;
        bytes[gd..gd + 4].copy_from_slice(&3u32.to_le_bytes());
        bytes[gd + 4..gd + 8].copy_from_slice(&4u32.to_le_bytes());
        bytes[gd + 8..gd + 12].copy_from_slice(&5u32.to_le_bytes());
        let root_inode = 5 * 1024 + 128;
        bytes[root_inode..root_inode + 2].copy_from_slice(&0x41edu16.to_le_bytes());
        bytes[root_inode + 4..root_inode + 8].copy_from_slice(&1024u32.to_le_bytes());
        bytes[root_inode + 26..root_inode + 28].copy_from_slice(&2u16.to_le_bytes());
        bytes[root_inode + 28..root_inode + 32].copy_from_slice(&2u32.to_le_bytes());
        bytes[root_inode + 40..root_inode + 44].copy_from_slice(&21u32.to_le_bytes());
        let file_inode = 5 * 1024 + 2 * 128;
        bytes[file_inode..file_inode + 2].copy_from_slice(&0x81a4u16.to_le_bytes());
        bytes[file_inode + 4..file_inode + 8].copy_from_slice(&7u32.to_le_bytes());
        bytes[file_inode + 26..file_inode + 28].copy_from_slice(&1u16.to_le_bytes());
        bytes[file_inode + 28..file_inode + 32].copy_from_slice(&2u32.to_le_bytes());
        bytes[file_inode + 40..file_inode + 44].copy_from_slice(&22u32.to_le_bytes());
        let dir = 21 * 1024;
        for (inode, record, kind, name) in [
            (2u32, 12u16, 2u8, b".".as_slice()),
            (2, 12, 2, b".."),
            (3, 1000, 1, b"hello"),
        ] {
            let offset = if name == b"." {
                dir
            } else if name == b".." {
                dir + 12
            } else {
                dir + 24
            };
            bytes[offset..offset + 4].copy_from_slice(&inode.to_le_bytes());
            bytes[offset + 4..offset + 6].copy_from_slice(&record.to_le_bytes());
            bytes[offset + 6] = name.len() as u8;
            bytes[offset + 7] = kind;
            bytes[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
        }
        bytes[22 * 1024..22 * 1024 + 7].copy_from_slice(b"nC ext2");
        bytes
    }

    #[test]
    fn initramfs_mount_permissions_busy_unmount_and_paths_are_material() {
        let mut mounted = MountedVfs::default();
        let info = mounted
            .mount_initramfs("/init", "boot:initrd", &newc_fixture())
            .unwrap();
        assert_eq!(info.files, 1);
        assert_eq!(
            mounted.vfs.file_bytes("/init/etc/issue").unwrap(),
            b"nC initramfs\n"
        );
        let denied = mounted.open_as(
            "/init/etc/issue",
            OpenFlags::WRITE,
            VfsCredentials {
                uid: 2000,
                gid: 2000,
            },
        );
        assert!(matches!(
            denied,
            Err(StorageError::Io(KernelIoError::PermissionDenied(_)))
        ));
        let handle = mounted
            .open_as(
                "/init/etc/issue",
                OpenFlags::READ,
                VfsCredentials {
                    uid: 1000,
                    gid: 100,
                },
            )
            .unwrap();
        assert!(matches!(
            mounted.unmount("/init"),
            Err(StorageError::MountBusy(_))
        ));
        mounted.vfs.close(handle).unwrap();
        mounted.unmount("/init").unwrap();
        assert!(mounted.vfs.list("/init").unwrap().is_empty());
    }

    #[test]
    fn fat12_and_ext2_initial_profiles_mount_real_directory_and_file_bytes() {
        let mut mounted = MountedVfs::default();
        assert_eq!(
            mounted
                .mount_fat("/fat", "disk0p1", &fat12_fixture(), true)
                .unwrap()
                .kind,
            FileSystemKind::Fat12
        );
        assert_eq!(mounted.vfs.file_bytes("/fat/kernel.n").unwrap(), b"nC FAT");
        mounted.unmount("/fat").unwrap();
        mounted
            .mount_ext2("/ext", "disk0p2", &ext2_fixture())
            .unwrap();
        assert_eq!(mounted.vfs.file_bytes("/ext/hello").unwrap(), b"nC ext2");
    }

    #[test]
    fn writable_fat_persists_lfn_and_recovers_a_committed_redo_journal() {
        let mut mounted = MountedVfs::default();
        mounted
            .mount_fat("/fat", "disk0p1", &fat12_fixture(), false)
            .unwrap();
        let path = "/fat/manual-long-file-name.txt";
        let handle = mounted
            .open(
                path,
                OpenFlags::READ
                    .union(OpenFlags::WRITE)
                    .union(OpenFlags::CREATE),
            )
            .unwrap();
        mounted.write(handle, b"persistent FAT").unwrap();
        mounted.close(handle).unwrap();
        let report = mounted.sync_mount("/fat").unwrap();
        assert!(report.changed_blocks >= 3);
        assert!(report.long_names >= 1);
        let image = mounted.persistent_device_image("/fat").unwrap().to_vec();

        let mut remounted = MountedVfs::default();
        remounted
            .mount_fat("/fat", "disk0p1", &image, false)
            .unwrap();
        assert_eq!(remounted.file_bytes(path).unwrap(), b"persistent FAT");
        let handle = remounted
            .open(path, OpenFlags::WRITE.union(OpenFlags::TRUNCATE))
            .unwrap();
        remounted.write(handle, b"journal replay").unwrap();
        remounted.close(handle).unwrap();
        assert!(matches!(
            remounted.sync_mount_with_fault("/fat", StorageSyncFault::AfterJournalCommit),
            Err(StorageError::TransactionInterrupted)
        ));
        let interrupted = remounted.persistent_device_image("/fat").unwrap().to_vec();
        let mut recovered = MountedVfs::default();
        recovered
            .mount_fat("/fat", "disk0p1", &interrupted, false)
            .unwrap();
        assert_eq!(recovered.file_bytes(path).unwrap(), b"journal replay");
    }

    #[test]
    fn writable_ext2_persists_double_indirect_data_acl_and_long_names() {
        let mut mounted = MountedVfs::default();
        mounted
            .mount_ext2_mode("/ext", "disk0p2", &writable_ext2_fixture(), false)
            .unwrap();
        let path = "/ext/large-manual-kernel-storage-payload.bin";
        let handle = mounted
            .open(
                path,
                OpenFlags::READ
                    .union(OpenFlags::WRITE)
                    .union(OpenFlags::CREATE),
            )
            .unwrap();
        let payload = (0..300_000usize)
            .map(|index| (index.wrapping_mul(31) & 0xff) as u8)
            .collect::<Vec<_>>();
        mounted.write(handle, &payload).unwrap();
        mounted.close(handle).unwrap();
        mounted
            .set_acl(
                path,
                vec![
                    VfsAclEntry {
                        tag: VfsAclTag::User,
                        id: Some(2000),
                        permissions: 0o4,
                    },
                    VfsAclEntry {
                        tag: VfsAclTag::Mask,
                        id: None,
                        permissions: 0o4,
                    },
                ],
            )
            .unwrap();
        let report = mounted.sync_mount("/ext").unwrap();
        assert!(report.indirect_blocks >= 3);
        assert_eq!(report.acl_blocks, 1);
        let image = mounted.filesystem_image("/ext").unwrap().to_vec();

        let mut remounted = MountedVfs::default();
        remounted
            .mount_ext2_mode("/ext", "disk0p2", &image, false)
            .unwrap();
        assert_eq!(remounted.file_bytes(path).unwrap(), payload);
        assert_eq!(remounted.acl(path).unwrap().len(), 2);
        let handle = remounted
            .open_as(path, OpenFlags::READ, VfsCredentials { uid: 2000, gid: 9 })
            .unwrap();
        remounted.close(handle).unwrap();
        assert!(matches!(
            remounted.open_as(path, OpenFlags::WRITE, VfsCredentials { uid: 2000, gid: 9 }),
            Err(StorageError::Io(KernelIoError::PermissionDenied(_)))
        ));
    }

    #[test]
    fn writeback_cache_evicts_flushes_and_rejects_out_of_range_blocks() {
        let device = VirtualBlockDevice::new(4096, 512, false).unwrap();
        let mut cache = BlockCache::new(device, 512, 2).unwrap();
        cache.write_block(0, &[0x11; 512]).unwrap();
        cache.write_block(1, &[0x22; 512]).unwrap();
        assert_eq!(cache.read_block(0).unwrap()[0], 0x11);
        cache.write_block(2, &[0x33; 512]).unwrap();
        assert_eq!(cache.stats.evictions, 1);
        cache.flush().unwrap();
        assert_eq!(&cache.device().bytes()[0..4], &[0x11; 4]);
        assert_eq!(&cache.device().bytes()[1024..1028], &[0x33; 4]);
        assert!(cache.read_block(8).is_err());
    }

    #[test]
    fn complete_storage_certification_exercises_every_declared_filesystem_and_failure() {
        let report = certify_storage_contracts().unwrap();
        assert_eq!(
            report.filesystems,
            [
                FileSystemKind::InitramfsNewc,
                FileSystemKind::Fat12,
                FileSystemKind::Ext2
            ]
        );
        assert_eq!(report.files_materialized, 3);
        assert_eq!(report.permission_rejections, 1);
        assert_eq!(report.busy_unmount_rejections, 1);
        assert_eq!(report.cache.evictions, 1);
        assert!(report.cache.writebacks >= 3);
        assert!(report.persisted_bytes >= 3 * 512);
    }
}
