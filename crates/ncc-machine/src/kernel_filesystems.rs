use crate::{VfsAclEntry, VfsAclTag, VfsNodeKind, VfsSnapshotEntry};
use std::collections::{BTreeMap, BTreeSet};

const FAT_DIRECTORY_ENTRY_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistentFormat {
    Fat,
    Ext2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RebuiltFileSystem {
    pub bytes: Vec<u8>,
    pub block_size: usize,
    pub logical_bytes: usize,
    pub data_blocks: usize,
    pub metadata_blocks: usize,
    pub indirect_blocks: usize,
    pub acl_blocks: usize,
    pub long_names: usize,
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .map(|raw| u16::from_le_bytes(raw.try_into().unwrap()))
        .ok_or_else(|| format!("filesystem field at {offset:#x} is out of bounds"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .map(|raw| u32::from_le_bytes(raw.try_into().unwrap()))
        .ok_or_else(|| format!("filesystem field at {offset:#x} is out of bounds"))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), String> {
    let output = bytes
        .get_mut(offset..offset + 2)
        .ok_or_else(|| format!("filesystem field at {offset:#x} is out of bounds"))?;
    output.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), String> {
    let output = bytes
        .get_mut(offset..offset + 4)
        .ok_or_else(|| format!("filesystem field at {offset:#x} is out of bounds"))?;
    output.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn align_up(value: usize, alignment: usize) -> Result<usize, String> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| "filesystem alignment overflow".to_string())
}

fn parent_and_name(path: &str) -> Result<(&str, &str), String> {
    if path.is_empty() || path.starts_with('/') || path.ends_with('/') || path.contains('\0') {
        return Err(format!("invalid filesystem path '{path}'"));
    }
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    if name.is_empty() || matches!(name, "." | "..") || name.as_bytes().len() > 255 {
        return Err(format!("invalid filesystem component '{name}'"));
    }
    Ok((parent, name))
}

fn validate_tree(entries: &[VfsSnapshotEntry]) -> Result<(), String> {
    let mut known = BTreeSet::new();
    known.insert(String::new());
    for entry in entries {
        if !known.insert(entry.path.clone()) {
            return Err(format!("duplicate filesystem path '{}'", entry.path));
        }
    }
    for entry in entries {
        let (parent, _) = parent_and_name(&entry.path)?;
        if !parent.is_empty()
            && !entries.iter().any(|candidate| {
                candidate.path == parent && candidate.metadata.kind == VfsNodeKind::Directory
            })
        {
            return Err(format!("filesystem parent '{parent}' is absent"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FatKind {
    Fat12,
    Fat16,
    Fat32,
}

#[derive(Debug, Clone)]
struct FatLayout {
    kind: FatKind,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    reserved_sectors: usize,
    fat_count: usize,
    fat_sectors: usize,
    root_entries: usize,
    total_sectors: usize,
    fat_offset: usize,
    root_offset: usize,
    root_bytes: usize,
    data_offset: usize,
    cluster_count: u32,
}

impl FatLayout {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 512 || bytes.get(510..512) != Some(&[0x55, 0xaa]) {
            return Err("invalid FAT boot sector".into());
        }
        let bytes_per_sector = read_u16(bytes, 11)? as usize;
        let sectors_per_cluster = bytes[13] as usize;
        let reserved_sectors = read_u16(bytes, 14)? as usize;
        let fat_count = bytes[16] as usize;
        let root_entries = read_u16(bytes, 17)? as usize;
        let total16 = read_u16(bytes, 19)? as usize;
        let total32 = read_u32(bytes, 32)? as usize;
        let fat16 = read_u16(bytes, 22)? as usize;
        let fat32 = read_u32(bytes, 36)? as usize;
        if !(512..=4096).contains(&bytes_per_sector)
            || !bytes_per_sector.is_power_of_two()
            || sectors_per_cluster == 0
            || !sectors_per_cluster.is_power_of_two()
            || reserved_sectors == 0
            || fat_count == 0
        {
            return Err("invalid FAT geometry".into());
        }
        let total_sectors = if total16 != 0 { total16 } else { total32 };
        let fat_sectors = if fat16 != 0 { fat16 } else { fat32 };
        let root_sectors = (root_entries * FAT_DIRECTORY_ENTRY_BYTES).div_ceil(bytes_per_sector);
        let data_sectors = total_sectors
            .checked_sub(reserved_sectors + fat_count * fat_sectors + root_sectors)
            .ok_or_else(|| "FAT geometry underflow".to_string())?;
        let cluster_count = (data_sectors / sectors_per_cluster) as u32;
        let kind = if cluster_count < 4085 {
            FatKind::Fat12
        } else if cluster_count < 65525 {
            FatKind::Fat16
        } else {
            FatKind::Fat32
        };
        let logical_bytes = total_sectors
            .checked_mul(bytes_per_sector)
            .ok_or_else(|| "FAT size overflow".to_string())?;
        if logical_bytes > bytes.len() || fat_sectors == 0 {
            return Err("FAT image is truncated".into());
        }
        let fat_offset = reserved_sectors * bytes_per_sector;
        let root_offset = (reserved_sectors + fat_count * fat_sectors) * bytes_per_sector;
        let root_bytes = root_sectors * bytes_per_sector;
        let data_offset = root_offset + root_bytes;
        Ok(Self {
            kind,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            fat_count,
            fat_sectors,
            root_entries,
            total_sectors,
            fat_offset,
            root_offset,
            root_bytes,
            data_offset,
            cluster_count,
        })
    }

    fn cluster_size(&self) -> usize {
        self.bytes_per_sector * self.sectors_per_cluster
    }

    fn logical_bytes(&self) -> usize {
        self.total_sectors * self.bytes_per_sector
    }

    fn cluster_offset(&self, cluster: u32) -> Result<usize, String> {
        if cluster < 2 || cluster >= self.cluster_count + 2 {
            return Err(format!("FAT cluster {cluster} is out of range"));
        }
        self.data_offset
            .checked_add((cluster as usize - 2) * self.cluster_size())
            .ok_or_else(|| "FAT cluster offset overflow".to_string())
    }
}

#[derive(Debug, Clone)]
struct FatName {
    short: [u8; 11],
    long: Option<String>,
}

fn valid_short_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"$%'-_@~`!(){}^#&".contains(&byte)
}

fn short_component(component: &str, width: usize) -> Vec<u8> {
    component
        .bytes()
        .filter(|byte| valid_short_char(byte.to_ascii_uppercase()))
        .map(|byte| byte.to_ascii_uppercase())
        .take(width)
        .collect()
}

fn allocate_short_name(name: &str, used: &mut BTreeSet<[u8; 11]>) -> Result<FatName, String> {
    let (base, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    let base_short = short_component(base, 8);
    let extension_short = short_component(extension, 3);
    let exact = !base.is_empty()
        && base.len() <= 8
        && extension.len() <= 3
        && base.bytes().all(valid_short_char)
        && extension.bytes().all(valid_short_char)
        && name.is_ascii()
        && name == name.to_ascii_uppercase();
    if exact {
        let mut short = [b' '; 11];
        short[..base_short.len()].copy_from_slice(&base_short);
        short[8..8 + extension_short.len()].copy_from_slice(&extension_short);
        if used.insert(short) {
            return Ok(FatName { short, long: None });
        }
    }
    let stem = short_component(base, 6);
    if stem.is_empty() {
        return Err(format!(
            "FAT name '{name}' has no representable short alias"
        ));
    }
    for ordinal in 1..=999_999u32 {
        let tail = format!("~{ordinal}");
        let prefix_len = 8usize.saturating_sub(tail.len()).min(stem.len());
        let mut short = [b' '; 11];
        short[..prefix_len].copy_from_slice(&stem[..prefix_len]);
        short[prefix_len..prefix_len + tail.len()].copy_from_slice(tail.as_bytes());
        short[8..8 + extension_short.len()].copy_from_slice(&extension_short);
        if used.insert(short) {
            return Ok(FatName {
                short,
                long: Some(name.to_string()),
            });
        }
    }
    Err(format!("FAT directory exhausted aliases for '{name}'"))
}

fn fat_checksum(short: &[u8; 11]) -> u8 {
    short
        .iter()
        .fold(0u8, |sum, byte| sum.rotate_right(1).wrapping_add(*byte))
}

fn lfn_slots(name: &str) -> usize {
    name.encode_utf16().count().div_ceil(13)
}

fn write_lfn_units(entry: &mut [u8], units: &[u16]) {
    let positions = [1usize, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
    for (index, position) in positions.into_iter().enumerate() {
        let value = units.get(index).copied().unwrap_or(0xffff);
        entry[position..position + 2].copy_from_slice(&value.to_le_bytes());
    }
}

fn encode_lfn(name: &str, short: &[u8; 11]) -> Vec<[u8; 32]> {
    let mut units = name.encode_utf16().collect::<Vec<_>>();
    units.push(0);
    while units.len() % 13 != 0 {
        units.push(0xffff);
    }
    let chunks = units.chunks(13).collect::<Vec<_>>();
    let checksum = fat_checksum(short);
    let mut output = Vec::with_capacity(chunks.len());
    for index in (0..chunks.len()).rev() {
        let mut entry = [0u8; 32];
        entry[0] = (index as u8 + 1) | if index + 1 == chunks.len() { 0x40 } else { 0 };
        entry[11] = 0x0f;
        entry[12] = 0;
        entry[13] = checksum;
        write_lfn_units(&mut entry, chunks[index]);
        output.push(entry);
    }
    output
}

#[derive(Debug, Clone)]
struct FatNode<'a> {
    entry: &'a VfsSnapshotEntry,
    name: FatName,
    first_cluster: u32,
    clusters: Vec<u32>,
}

fn set_fat_entry(fat: &mut [u8], kind: FatKind, cluster: u32, value: u32) -> Result<(), String> {
    match kind {
        FatKind::Fat12 => {
            let offset = cluster as usize + cluster as usize / 2;
            let bytes = fat
                .get_mut(offset..offset + 2)
                .ok_or_else(|| "FAT12 table overflow".to_string())?;
            let mut pair = u16::from_le_bytes(bytes.try_into().unwrap());
            if cluster & 1 == 0 {
                pair = (pair & 0xf000) | value as u16 & 0x0fff;
            } else {
                pair = (pair & 0x000f) | ((value as u16 & 0x0fff) << 4);
            }
            bytes.copy_from_slice(&pair.to_le_bytes());
        }
        FatKind::Fat16 => write_u16(fat, cluster as usize * 2, value as u16)?,
        FatKind::Fat32 => {
            let offset = cluster as usize * 4;
            let previous = read_u32(fat, offset)? & 0xf000_0000;
            write_u32(fat, offset, previous | value & 0x0fff_ffff)?;
        }
    }
    Ok(())
}

fn fat_end(kind: FatKind) -> u32 {
    match kind {
        FatKind::Fat12 => 0x0fff,
        FatKind::Fat16 => 0xffff,
        FatKind::Fat32 => 0x0fff_ffff,
    }
}

fn write_short_entry(
    output: &mut Vec<[u8; 32]>,
    name: &FatName,
    node: &VfsSnapshotEntry,
    first_cluster: u32,
) {
    if let Some(long) = &name.long {
        output.extend(encode_lfn(long, &name.short));
    }
    let mut entry = [0u8; 32];
    entry[..11].copy_from_slice(&name.short);
    entry[11] = match node.metadata.kind {
        VfsNodeKind::Directory => 0x10,
        _ => 0x20 | u8::from(node.metadata.mode & 0o222 == 0),
    };
    entry[20..22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
    entry[26..28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
    entry[28..32].copy_from_slice(&(node.data.len() as u32).to_le_bytes());
    output.push(entry);
}

fn directory_dot(name: &[u8; 11], cluster: u32) -> [u8; 32] {
    let mut entry = [0u8; 32];
    entry[..11].copy_from_slice(name);
    entry[11] = 0x10;
    entry[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    entry[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
    entry
}

pub(crate) fn rebuild_fat(
    original: &[u8],
    entries: &[VfsSnapshotEntry],
) -> Result<RebuiltFileSystem, String> {
    validate_tree(entries)?;
    let layout = FatLayout::parse(original)?;
    let mut output = original[..layout.logical_bytes()].to_vec();
    output[layout.fat_offset..].fill(0);

    let mut children = BTreeMap::<String, Vec<String>>::new();
    children.entry(String::new()).or_default();
    for entry in entries {
        let (parent, _) = parent_and_name(&entry.path)?;
        children
            .entry(parent.to_string())
            .or_default()
            .push(entry.path.clone());
        if entry.metadata.kind == VfsNodeKind::Directory {
            children.entry(entry.path.clone()).or_default();
        }
    }
    for values in children.values_mut() {
        values.sort();
    }

    let mut names = BTreeMap::<String, FatName>::new();
    let mut long_names = 0usize;
    for child_paths in children.values() {
        let mut used = BTreeSet::new();
        for path in child_paths {
            let (_, name) = parent_and_name(path)?;
            let encoded = allocate_short_name(name, &mut used)?;
            long_names += usize::from(encoded.long.is_some());
            names.insert(path.clone(), encoded);
        }
    }

    let cluster_size = layout.cluster_size();
    let mut next_cluster = 2u32;
    let mut allocated = BTreeSet::new();
    let mut allocate = |count: usize| -> Result<Vec<u32>, String> {
        let mut result = Vec::with_capacity(count);
        while result.len() < count {
            while allocated.contains(&next_cluster) {
                next_cluster += 1;
            }
            if next_cluster >= layout.cluster_count + 2 {
                return Err("FAT volume has no free cluster for the synchronized tree".into());
            }
            allocated.insert(next_cluster);
            result.push(next_cluster);
            next_cluster += 1;
        }
        Ok(result)
    };

    let mut root_clusters = Vec::new();
    if layout.kind == FatKind::Fat32 {
        let root_entries = children.get("").map_or(0, |items| {
            items
                .iter()
                .map(|path| 1 + names[path].long.as_deref().map_or(0, lfn_slots))
                .sum::<usize>()
        });
        root_clusters = allocate((root_entries * 32 + 32).div_ceil(cluster_size).max(1))?;
        write_u32(&mut output, 44, root_clusters[0])?;
    }

    let mut nodes = BTreeMap::<String, FatNode<'_>>::new();
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|entry| {
        (
            entry.metadata.kind != VfsNodeKind::Directory,
            entry.path.matches('/').count(),
            entry.path.clone(),
        )
    });
    for entry in ordered {
        let count = match entry.metadata.kind {
            VfsNodeKind::Directory => {
                let slots = 2 + children[&entry.path]
                    .iter()
                    .map(|path| 1 + names[path].long.as_deref().map_or(0, lfn_slots))
                    .sum::<usize>();
                (slots * 32 + 32).div_ceil(cluster_size).max(1)
            }
            _ => entry.data.len().div_ceil(cluster_size),
        };
        let clusters = allocate(count)?;
        nodes.insert(
            entry.path.clone(),
            FatNode {
                entry,
                name: names[&entry.path].clone(),
                first_cluster: clusters.first().copied().unwrap_or(0),
                clusters,
            },
        );
    }

    let mut fat = vec![0u8; layout.fat_sectors * layout.bytes_per_sector];
    match layout.kind {
        FatKind::Fat12 => fat[..3].copy_from_slice(&[0xf8, 0xff, 0xff]),
        FatKind::Fat16 => {
            write_u16(&mut fat, 0, 0xfff8)?;
            write_u16(&mut fat, 2, 0xffff)?;
        }
        FatKind::Fat32 => {
            write_u32(&mut fat, 0, 0x0fff_fff8)?;
            write_u32(&mut fat, 4, 0x0fff_ffff)?;
        }
    }
    for chain in std::iter::once(&root_clusters).chain(nodes.values().map(|node| &node.clusters)) {
        for (index, cluster) in chain.iter().enumerate() {
            let next = chain
                .get(index + 1)
                .copied()
                .unwrap_or(fat_end(layout.kind));
            set_fat_entry(&mut fat, layout.kind, *cluster, next)?;
        }
    }
    for copy in 0..layout.fat_count {
        let start = layout.fat_offset + copy * fat.len();
        output[start..start + fat.len()].copy_from_slice(&fat);
    }

    let build_directory =
        |path: &str, self_cluster: u32, parent_cluster: u32| -> Result<Vec<u8>, String> {
            let mut raw = Vec::<[u8; 32]>::new();
            if !path.is_empty() {
                raw.push(directory_dot(b".          ", self_cluster));
                raw.push(directory_dot(b"..         ", parent_cluster));
            }
            for child in children.get(path).into_iter().flatten() {
                let node = &nodes[child];
                write_short_entry(&mut raw, &node.name, node.entry, node.first_cluster);
            }
            let mut bytes = raw.into_iter().flatten().collect::<Vec<_>>();
            bytes.resize(align_up(bytes.len() + 32, cluster_size)?, 0);
            Ok(bytes)
        };

    let root_data = build_directory("", root_clusters.first().copied().unwrap_or(0), 0)?;
    if layout.kind == FatKind::Fat32 {
        for (cluster, chunk) in root_clusters.iter().zip(root_data.chunks(cluster_size)) {
            let offset = layout.cluster_offset(*cluster)?;
            output[offset..offset + chunk.len()].copy_from_slice(chunk);
        }
    } else {
        if root_data.len() > layout.root_bytes || root_data.len() / 32 > layout.root_entries {
            return Err("FAT fixed root directory is full".into());
        }
        output[layout.root_offset..layout.root_offset + root_data.len()]
            .copy_from_slice(&root_data);
    }

    let mut data_blocks = 0usize;
    for node in nodes.values() {
        let bytes = if node.entry.metadata.kind == VfsNodeKind::Directory {
            let (parent, _) = parent_and_name(&node.entry.path)?;
            let parent_cluster = if parent.is_empty() {
                root_clusters.first().copied().unwrap_or(0)
            } else {
                nodes[parent].first_cluster
            };
            build_directory(&node.entry.path, node.first_cluster, parent_cluster)?
        } else {
            node.entry.data.clone()
        };
        for (cluster, chunk) in node.clusters.iter().zip(bytes.chunks(cluster_size)) {
            let offset = layout.cluster_offset(*cluster)?;
            output[offset..offset + chunk.len()].copy_from_slice(chunk);
        }
        if node.entry.metadata.kind != VfsNodeKind::Directory {
            data_blocks += node.clusters.len();
        }
    }

    if layout.kind == FatKind::Fat32 && layout.reserved_sectors > 1 {
        let fs_info = layout.bytes_per_sector;
        if output.get(fs_info..fs_info + 4) == Some(&[0x52, 0x52, 0x61, 0x41]) {
            let free = layout.cluster_count.saturating_sub(allocated.len() as u32);
            write_u32(&mut output, fs_info + 488, free)?;
            write_u32(&mut output, fs_info + 492, next_cluster)?;
        }
    }

    Ok(RebuiltFileSystem {
        logical_bytes: output.len(),
        bytes: output,
        block_size: layout.bytes_per_sector,
        data_blocks,
        metadata_blocks: layout.fat_count * layout.fat_sectors
            + layout.root_bytes.div_ceil(layout.bytes_per_sector),
        indirect_blocks: 0,
        acl_blocks: 0,
        long_names,
    })
}

#[derive(Debug, Clone)]
struct Ext2Layout {
    block_size: usize,
    logical_bytes: usize,
    blocks_count: u32,
    inodes_count: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    inode_size: usize,
    first_data_block: u32,
    group_table_offset: usize,
    groups: usize,
}

impl Ext2Layout {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if read_u16(bytes, 1024 + 56)? != 0xef53 {
            return Err("invalid ext2 superblock".into());
        }
        let log_block_size = read_u32(bytes, 1024 + 24)?;
        if log_block_size > 6 {
            return Err("unsupported ext2 block size".into());
        }
        let block_size = 1024usize << log_block_size;
        let blocks_count = read_u32(bytes, 1024 + 4)?;
        let inodes_count = read_u32(bytes, 1024)?;
        let blocks_per_group = read_u32(bytes, 1024 + 32)?;
        let inodes_per_group = read_u32(bytes, 1024 + 40)?;
        let first_data_block = read_u32(bytes, 1024 + 20)?;
        let revision = read_u32(bytes, 1024 + 76)?;
        let inode_size = if revision == 0 {
            128
        } else {
            read_u16(bytes, 1024 + 88)? as usize
        };
        if blocks_count == 0
            || inodes_count < 11
            || blocks_per_group == 0
            || inodes_per_group == 0
            || inode_size < 128
            || inode_size > block_size
        {
            return Err("invalid ext2 geometry".into());
        }
        let logical_bytes = blocks_count as usize * block_size;
        if logical_bytes > bytes.len() {
            return Err("ext2 image is truncated".into());
        }
        let groups = (blocks_count - first_data_block).div_ceil(blocks_per_group) as usize;
        Ok(Self {
            block_size,
            logical_bytes,
            blocks_count,
            inodes_count,
            blocks_per_group,
            inodes_per_group,
            inode_size,
            first_data_block,
            group_table_offset: if block_size == 1024 { 2048 } else { block_size },
            groups,
        })
    }

    fn descriptor_offset(&self, group: usize) -> usize {
        self.group_table_offset + group * 32
    }

    fn group_first_block(&self, group: usize) -> u32 {
        self.first_data_block + group as u32 * self.blocks_per_group
    }
}

fn ext2_acl_value(acl: &[VfsAclEntry]) -> Result<Vec<u8>, String> {
    let mut value = 2u32.to_le_bytes().to_vec();
    for entry in acl {
        if entry.permissions > 0o7 {
            return Err("ext2 ACL permission is outside rwx".into());
        }
        let tag = match (entry.tag, entry.id) {
            (VfsAclTag::User, None) => 0x01,
            (VfsAclTag::User, Some(_)) => 0x02,
            (VfsAclTag::Group, None) => 0x04,
            (VfsAclTag::Group, Some(_)) => 0x08,
            (VfsAclTag::Mask, None) => 0x10,
            (VfsAclTag::Other, None) => 0x20,
            _ => return Err("invalid qualified ext2 ACL tag".into()),
        };
        value.extend_from_slice(&(tag as u16).to_le_bytes());
        value.extend_from_slice(&(entry.permissions as u16).to_le_bytes());
        if let Some(id) = entry.id {
            value.extend_from_slice(&id.to_le_bytes());
        }
    }
    Ok(value)
}

fn ext2_acl_block(block_size: usize, acl: &[VfsAclEntry]) -> Result<Vec<u8>, String> {
    let value = ext2_acl_value(acl)?;
    if value.len() + 52 > block_size {
        return Err("ext2 ACL exceeds one xattr block".into());
    }
    let mut block = vec![0u8; block_size];
    write_u32(&mut block, 0, 0xea02_0000)?;
    write_u32(&mut block, 4, 1)?;
    write_u32(&mut block, 8, 1)?;
    let value_offset = block_size - value.len();
    block[32] = 0;
    block[33] = 2;
    write_u16(&mut block, 34, value_offset as u16)?;
    write_u32(&mut block, 36, 0)?;
    write_u32(&mut block, 40, value.len() as u32)?;
    block[value_offset..].copy_from_slice(&value);
    Ok(block)
}

#[derive(Debug, Clone)]
struct Ext2Node<'a> {
    entry: Option<&'a VfsSnapshotEntry>,
    inode: u32,
    parent_inode: u32,
    data: Vec<u8>,
    data_blocks: Vec<u32>,
    pointer_blocks: Vec<u32>,
    pointers: [u32; 15],
    acl_block: u32,
}

#[derive(Debug)]
struct Ext2Allocator {
    free_blocks: BTreeSet<u32>,
}

impl Ext2Allocator {
    fn allocate(&mut self) -> Result<u32, String> {
        let block = self
            .free_blocks
            .pop_first()
            .ok_or_else(|| "ext2 volume has no free blocks".to_string())?;
        Ok(block)
    }
}

fn encode_pointer_tree(
    block_size: usize,
    data_blocks: &[u32],
    allocator: &mut Ext2Allocator,
    block_payloads: &mut BTreeMap<u32, Vec<u8>>,
) -> Result<([u32; 15], Vec<u32>), String> {
    let mut pointers = [0u32; 15];
    let direct = data_blocks.len().min(12);
    pointers[..direct].copy_from_slice(&data_blocks[..direct]);
    let mut cursor = direct;
    let fanout = block_size / 4;
    let mut metadata = Vec::new();

    fn build_level(
        depth: u8,
        fanout: usize,
        block_size: usize,
        data: &[u32],
        cursor: &mut usize,
        allocator: &mut Ext2Allocator,
        payloads: &mut BTreeMap<u32, Vec<u8>>,
        metadata: &mut Vec<u32>,
    ) -> Result<u32, String> {
        if *cursor >= data.len() {
            return Ok(0);
        }
        let block = allocator.allocate()?;
        metadata.push(block);
        let mut payload = vec![0u8; block_size];
        for index in 0..fanout {
            if *cursor >= data.len() {
                break;
            }
            let pointer = if depth == 1 {
                let value = data[*cursor];
                *cursor += 1;
                value
            } else {
                build_level(
                    depth - 1,
                    fanout,
                    block_size,
                    data,
                    cursor,
                    allocator,
                    payloads,
                    metadata,
                )?
            };
            write_u32(&mut payload, index * 4, pointer)?;
        }
        payloads.insert(block, payload);
        Ok(block)
    }

    for (slot, depth) in [(12, 1), (13, 2), (14, 3)] {
        if cursor >= data_blocks.len() {
            break;
        }
        pointers[slot] = build_level(
            depth,
            fanout,
            block_size,
            data_blocks,
            &mut cursor,
            allocator,
            block_payloads,
            &mut metadata,
        )?;
    }
    if cursor != data_blocks.len() {
        return Err("ext2 file exceeds triple-indirect capacity".into());
    }
    Ok((pointers, metadata))
}

fn ext2_dir_entry(inode: u32, name: &str, file_type: u8) -> Result<Vec<u8>, String> {
    if name.as_bytes().len() > 255 {
        return Err("ext2 directory name exceeds 255 bytes".into());
    }
    let length = align_up(8 + name.len(), 4)?;
    let mut output = vec![0u8; length];
    write_u32(&mut output, 0, inode)?;
    write_u16(&mut output, 4, length as u16)?;
    output[6] = name.len() as u8;
    output[7] = file_type;
    output[8..8 + name.len()].copy_from_slice(name.as_bytes());
    Ok(output)
}

fn ext2_directory_data(
    path: &str,
    inode: u32,
    parent: u32,
    children: &BTreeMap<String, Vec<String>>,
    nodes: &BTreeMap<String, Ext2Node<'_>>,
    block_size: usize,
) -> Result<Vec<u8>, String> {
    let mut records = vec![
        ext2_dir_entry(inode, ".", 2)?,
        ext2_dir_entry(parent, "..", 2)?,
    ];
    for child in children.get(path).into_iter().flatten() {
        let node = &nodes[child];
        let entry = node.entry.unwrap();
        let (_, name) = parent_and_name(child)?;
        let kind = match entry.metadata.kind {
            VfsNodeKind::Directory => 2,
            VfsNodeKind::Symlink => 7,
            _ => 1,
        };
        records.push(ext2_dir_entry(node.inode, name, kind)?);
    }
    let mut output = Vec::new();
    let mut last_start = None;
    for mut record in records {
        let used = output.len() % block_size;
        if used != 0 && used + record.len() > block_size {
            let previous =
                last_start.ok_or_else(|| "ext2 directory has no previous entry".to_string())?;
            let old = read_u16(&output, previous + 4)? as usize;
            write_u16(&mut output, previous + 4, (old + block_size - used) as u16)?;
            output.resize(output.len() + block_size - used, 0);
        }
        last_start = Some(output.len());
        output.append(&mut record);
    }
    if let Some(previous) = last_start {
        let used = output.len() % block_size;
        if used != 0 {
            let old = read_u16(&output, previous + 4)? as usize;
            write_u16(&mut output, previous + 4, (old + block_size - used) as u16)?;
            output.resize(output.len() + block_size - used, 0);
        }
    }
    Ok(output)
}

pub(crate) fn rebuild_ext2(
    original: &[u8],
    entries: &[VfsSnapshotEntry],
) -> Result<RebuiltFileSystem, String> {
    validate_tree(entries)?;
    let layout = Ext2Layout::parse(original)?;
    let mut output = original[..layout.logical_bytes].to_vec();
    let inode_table_blocks =
        (layout.inodes_per_group as usize * layout.inode_size).div_ceil(layout.block_size);
    let mut reserved = BTreeSet::new();
    for group in 0..layout.groups {
        let descriptor = layout.descriptor_offset(group);
        let block_bitmap = read_u32(&output, descriptor)?;
        let inode_bitmap = read_u32(&output, descriptor + 4)?;
        let inode_table = read_u32(&output, descriptor + 8)?;
        reserved.insert(block_bitmap);
        reserved.insert(inode_bitmap);
        reserved.extend(inode_table..inode_table + inode_table_blocks as u32);
        if group == 0 {
            reserved.extend(0..=inode_table);
        }
    }
    let mut allocator = Ext2Allocator {
        free_blocks: (layout.first_data_block..layout.blocks_count)
            .filter(|block| !reserved.contains(block))
            .collect(),
    };
    let mut children = BTreeMap::<String, Vec<String>>::new();
    children.entry(String::new()).or_default();
    for entry in entries {
        let (parent, _) = parent_and_name(&entry.path)?;
        children
            .entry(parent.into())
            .or_default()
            .push(entry.path.clone());
        if entry.metadata.kind == VfsNodeKind::Directory {
            children.entry(entry.path.clone()).or_default();
        }
    }
    for children in children.values_mut() {
        children.sort();
    }
    let needed_inodes = entries.len() + 11;
    if needed_inodes > layout.inodes_count as usize {
        return Err("ext2 volume has no free inode for the synchronized tree".into());
    }

    let mut nodes = BTreeMap::<String, Ext2Node<'_>>::new();
    nodes.insert(
        String::new(),
        Ext2Node {
            entry: None,
            inode: 2,
            parent_inode: 2,
            data: Vec::new(),
            data_blocks: Vec::new(),
            pointer_blocks: Vec::new(),
            pointers: [0; 15],
            acl_block: 0,
        },
    );
    for (index, entry) in entries.iter().enumerate() {
        let (parent, _) = parent_and_name(&entry.path)?;
        let parent_inode = nodes.get(parent).map_or(2, |node| node.inode);
        nodes.insert(
            entry.path.clone(),
            Ext2Node {
                entry: Some(entry),
                inode: 11 + index as u32,
                parent_inode,
                data: match entry.metadata.kind {
                    VfsNodeKind::Symlink => entry
                        .symlink_target
                        .as_deref()
                        .unwrap_or_default()
                        .as_bytes()
                        .to_vec(),
                    _ => entry.data.clone(),
                },
                data_blocks: Vec::new(),
                pointer_blocks: Vec::new(),
                pointers: [0; 15],
                acl_block: 0,
            },
        );
    }
    let directory_paths = nodes
        .iter()
        .filter_map(|(path, node)| {
            (node.entry.is_none()
                || node
                    .entry
                    .is_some_and(|entry| entry.metadata.kind == VfsNodeKind::Directory))
            .then_some(path.clone())
        })
        .collect::<Vec<_>>();
    for path in directory_paths {
        let inode = nodes[&path].inode;
        let parent = nodes[&path].parent_inode;
        let data = ext2_directory_data(&path, inode, parent, &children, &nodes, layout.block_size)?;
        nodes.get_mut(&path).unwrap().data = data;
    }

    let mut payloads = BTreeMap::<u32, Vec<u8>>::new();
    let mut indirect_blocks = 0usize;
    let mut acl_blocks = 0usize;
    for node in nodes.values_mut() {
        let inline_symlink = node
            .entry
            .is_some_and(|entry| entry.metadata.kind == VfsNodeKind::Symlink)
            && node.data.len() <= 60;
        if !inline_symlink {
            for chunk in node.data.chunks(layout.block_size) {
                let block = allocator.allocate()?;
                let mut payload = vec![0u8; layout.block_size];
                payload[..chunk.len()].copy_from_slice(chunk);
                payloads.insert(block, payload);
                node.data_blocks.push(block);
            }
            let (pointers, metadata) = encode_pointer_tree(
                layout.block_size,
                &node.data_blocks,
                &mut allocator,
                &mut payloads,
            )?;
            node.pointers = pointers;
            indirect_blocks += metadata.len();
            node.pointer_blocks = metadata;
        }
        if let Some(entry) = node.entry.filter(|entry| !entry.acl.is_empty()) {
            let block = allocator.allocate()?;
            payloads.insert(block, ext2_acl_block(layout.block_size, &entry.acl)?);
            node.acl_block = block;
            acl_blocks += 1;
        }
    }

    for (block, payload) in &payloads {
        let offset = *block as usize * layout.block_size;
        output[offset..offset + layout.block_size].copy_from_slice(payload);
    }

    let inode_tables = (0..layout.groups)
        .map(|group| read_u32(&output, layout.descriptor_offset(group) + 8))
        .collect::<Result<Vec<_>, _>>()?;
    let write_inode = |output: &mut [u8], node: &Ext2Node<'_>| -> Result<(), String> {
        let index = node.inode - 1;
        let group = index / layout.inodes_per_group;
        let local = index % layout.inodes_per_group;
        let offset = inode_tables[group as usize] as usize * layout.block_size
            + local as usize * layout.inode_size;
        let raw = output
            .get_mut(offset..offset + layout.inode_size)
            .ok_or_else(|| "ext2 inode table is truncated".to_string())?;
        raw.fill(0);
        let (kind, mode, uid, gid) = if let Some(entry) = node.entry {
            let kind = match entry.metadata.kind {
                VfsNodeKind::Directory => 0x4000,
                VfsNodeKind::Symlink => 0xa000,
                _ => 0x8000,
            };
            (
                kind,
                entry.metadata.mode,
                entry.metadata.uid,
                entry.metadata.gid,
            )
        } else {
            (0x4000, 0o755, 0, 0)
        };
        write_u16(raw, 0, kind | mode)?;
        write_u16(raw, 2, uid as u16)?;
        write_u32(raw, 4, node.data.len() as u32)?;
        write_u16(raw, 24, gid as u16)?;
        write_u16(
            raw,
            26,
            if kind == 0x4000 {
                2 + children
                    .get(if node.inode == 2 {
                        ""
                    } else {
                        node.entry.unwrap().path.as_str()
                    })
                    .map_or(0, |items| {
                        items
                            .iter()
                            .filter(|path| {
                                nodes[*path].entry.is_some_and(|entry| {
                                    entry.metadata.kind == VfsNodeKind::Directory
                                })
                            })
                            .count()
                    }) as u16
            } else {
                1
            },
        )?;
        let sectors =
            (node.data_blocks.len() + node.pointer_blocks.len() + usize::from(node.acl_block != 0))
                * layout.block_size
                / 512;
        write_u32(raw, 28, sectors as u32)?;
        if kind == 0xa000 && node.data.len() <= 60 {
            raw[40..40 + node.data.len()].copy_from_slice(&node.data);
        } else {
            for (index, pointer) in node.pointers.iter().enumerate() {
                write_u32(raw, 40 + index * 4, *pointer)?;
            }
        }
        write_u32(raw, 104, node.acl_block)?;
        if layout.inode_size >= 120 {
            write_u16(raw, 116, (uid >> 16) as u16)?;
            write_u16(raw, 118, (gid >> 16) as u16)?;
        }
        Ok(())
    };
    for node in nodes.values() {
        write_inode(&mut output, node)?;
    }

    let used_inodes = nodes
        .values()
        .map(|node| node.inode)
        .collect::<BTreeSet<_>>();
    let used_blocks = reserved
        .iter()
        .copied()
        .chain(payloads.keys().copied())
        .collect::<BTreeSet<_>>();
    let mut total_free_blocks = 0u32;
    let mut total_free_inodes = 0u32;
    for group in 0..layout.groups {
        let descriptor = layout.descriptor_offset(group);
        let block_bitmap = read_u32(&output, descriptor)? as usize * layout.block_size;
        let inode_bitmap = read_u32(&output, descriptor + 4)? as usize * layout.block_size;
        output[block_bitmap..block_bitmap + layout.block_size].fill(0);
        output[inode_bitmap..inode_bitmap + layout.block_size].fill(0);
        let first = layout.group_first_block(group);
        let block_count = layout
            .blocks_per_group
            .min(layout.blocks_count.saturating_sub(first));
        let mut used_in_group = 0u32;
        for block in used_blocks.range(first..first + block_count) {
            let local = (*block - first) as usize;
            output[block_bitmap + local / 8] |= 1 << (local % 8);
            used_in_group += 1;
        }
        let inode_first = group as u32 * layout.inodes_per_group + 1;
        let inode_count = layout
            .inodes_per_group
            .min(layout.inodes_count.saturating_sub(inode_first - 1));
        let mut group_inodes = BTreeSet::new();
        for inode in used_inodes.range(inode_first..inode_first + inode_count) {
            let local = (*inode - inode_first) as usize;
            output[inode_bitmap + local / 8] |= 1 << (local % 8);
            group_inodes.insert(*inode);
        }
        if group == 0 {
            for inode in 1..=10.min(inode_count) {
                let local = (inode - 1) as usize;
                output[inode_bitmap + local / 8] |= 1 << (local % 8);
                group_inodes.insert(inode);
            }
        }
        let used_inode_count = group_inodes.len() as u32;
        let free_blocks = block_count - used_in_group;
        let free_inodes = inode_count - used_inode_count;
        write_u16(&mut output, descriptor + 12, free_blocks as u16)?;
        write_u16(&mut output, descriptor + 14, free_inodes as u16)?;
        write_u16(
            &mut output,
            descriptor + 16,
            nodes
                .values()
                .filter(|node| {
                    node.inode >= inode_first
                        && node.inode < inode_first + inode_count
                        && (node.entry.is_none()
                            || node
                                .entry
                                .is_some_and(|entry| entry.metadata.kind == VfsNodeKind::Directory))
                })
                .count() as u16,
        )?;
        total_free_blocks += free_blocks;
        total_free_inodes += free_inodes;
    }
    write_u32(&mut output, 1024 + 12, total_free_blocks)?;
    write_u32(&mut output, 1024 + 16, total_free_inodes)?;
    write_u16(&mut output, 1024 + 58, 1)?;

    Ok(RebuiltFileSystem {
        logical_bytes: output.len(),
        bytes: output,
        block_size: layout.block_size,
        data_blocks: nodes.values().map(|node| node.data_blocks.len()).sum(),
        metadata_blocks: reserved.len(),
        indirect_blocks,
        acl_blocks,
        long_names: entries
            .iter()
            .filter(|entry| parent_and_name(&entry.path).is_ok_and(|(_, name)| name.len() > 12))
            .count(),
    })
}

pub(crate) fn rebuild(
    format: PersistentFormat,
    original: &[u8],
    entries: &[VfsSnapshotEntry],
) -> Result<RebuiltFileSystem, String> {
    match format {
        PersistentFormat::Fat => rebuild_fat(original, entries),
        PersistentFormat::Ext2 => rebuild_ext2(original, entries),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(path: &str, kind: VfsNodeKind, bytes: Vec<u8>) -> VfsSnapshotEntry {
        VfsSnapshotEntry {
            path: path.into(),
            metadata: crate::VfsMetadata {
                kind,
                mode: if kind == VfsNodeKind::Directory {
                    0o755
                } else {
                    0o644
                },
                uid: 0,
                gid: 0,
                size: bytes.len(),
                generation: 0,
            },
            data: bytes,
            symlink_target: None,
            acl: Vec::new(),
        }
    }

    fn fat_image(kind: FatKind) -> Vec<u8> {
        let (reserved, root_entries, clusters, fat_sectors, total) = match kind {
            FatKind::Fat12 => (1usize, 32usize, 256usize, 1usize, 260usize),
            FatKind::Fat16 => (1, 512, 4096, 17, 4146),
            FatKind::Fat32 => (32, 0, 65_525, 512, 66_069),
        };
        let mut bytes = vec![0u8; total * 512];
        bytes[0] = 0xeb;
        bytes[11..13].copy_from_slice(&512u16.to_le_bytes());
        bytes[13] = 1;
        bytes[14..16].copy_from_slice(&(reserved as u16).to_le_bytes());
        bytes[16] = 1;
        bytes[17..19].copy_from_slice(&(root_entries as u16).to_le_bytes());
        bytes[21] = 0xf8;
        if total <= u16::MAX as usize {
            bytes[19..21].copy_from_slice(&(total as u16).to_le_bytes());
        } else {
            bytes[32..36].copy_from_slice(&(total as u32).to_le_bytes());
        }
        if kind == FatKind::Fat32 {
            bytes[36..40].copy_from_slice(&(fat_sectors as u32).to_le_bytes());
            bytes[44..48].copy_from_slice(&2u32.to_le_bytes());
            bytes[512..516].copy_from_slice(&0x4161_5252u32.to_le_bytes());
        } else {
            bytes[22..24].copy_from_slice(&(fat_sectors as u16).to_le_bytes());
        }
        bytes[510..512].copy_from_slice(&[0x55, 0xaa]);
        assert!(
            clusters
                >= match kind {
                    FatKind::Fat12 => 1,
                    FatKind::Fat16 => 4085,
                    FatKind::Fat32 => 65525,
                }
        );
        bytes
    }

    #[test]
    fn fat12_fat16_and_fat32_emit_real_chains_and_lfn_entries() {
        let entries = vec![
            snapshot("kernel", VfsNodeKind::Directory, Vec::new()),
            snapshot(
                "kernel/a-very-long-manual-kernel-file-name.bin",
                VfsNodeKind::Regular,
                vec![0x5a; 2048],
            ),
        ];
        for kind in [FatKind::Fat12, FatKind::Fat16, FatKind::Fat32] {
            let image = fat_image(kind);
            let rebuilt = rebuild_fat(&image, &entries).unwrap();
            let parsed = FatLayout::parse(&rebuilt.bytes).unwrap();
            assert_eq!(parsed.kind, kind);
            assert_eq!(rebuilt.long_names, 2);
            assert_eq!(rebuilt.data_blocks, 4);
            assert!(rebuilt.bytes.windows(2).any(|bytes| bytes != [0, 0]));
        }
    }

    #[test]
    fn ext2_pointer_builder_crosses_single_double_and_triple_indirection() {
        let fanout = 256usize;
        let data = (1..=(12 + fanout + fanout * fanout + 1) as u32).collect::<Vec<_>>();
        let mut allocator = Ext2Allocator {
            free_blocks: (1_000_000..1_001_000).collect(),
        };
        let mut payloads = BTreeMap::new();
        let (pointers, metadata) =
            encode_pointer_tree(1024, &data, &mut allocator, &mut payloads).unwrap();
        assert_ne!(pointers[12], 0);
        assert_ne!(pointers[13], 0);
        assert_ne!(pointers[14], 0);
        assert_eq!(metadata.len(), 261);
        assert_eq!(payloads.len(), metadata.len());
    }
}
