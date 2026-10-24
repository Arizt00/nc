use crate::{
    DirtyRect, KernelCore, KernelCoreError, MountedVfs, PciAddress, PciBus, PciFunction,
    PciVirtioError, PixelFormat, VirtualBlockDevice, VirtualDeviceError, VirtualFramebuffer,
    VirtualNetworkDevice,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum KernelIoError {
    InvalidPath,
    AlreadyExists(String),
    NotFound(String),
    NotDirectory(String),
    IsDirectory(String),
    PermissionDenied(String),
    InvalidHandle(u32),
    InvalidOffset,
    SymlinkLoop,
    InvalidController(&'static str),
    InvalidQueue(u16),
    InvalidCommand(&'static str),
    DuplicateCommand(u16),
    InvalidPacket(&'static str),
    NeighborUnresolved(Ipv4Address),
    SocketMissing(u16),
    Usb(&'static str),
    Core(KernelCoreError),
    Device(VirtualDeviceError),
    Pci(PciVirtioError),
}

impl fmt::Display for KernelIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath => f.write_str("invalid VFS path"),
            Self::AlreadyExists(path) => write!(f, "'{path}' already exists"),
            Self::NotFound(path) => write!(f, "'{path}' was not found"),
            Self::NotDirectory(path) => write!(f, "'{path}' is not a directory"),
            Self::IsDirectory(path) => write!(f, "'{path}' is a directory"),
            Self::PermissionDenied(path) => write!(f, "permission denied for '{path}'"),
            Self::InvalidHandle(handle) => write!(f, "invalid VFS handle {handle}"),
            Self::InvalidOffset => f.write_str("invalid file or device offset"),
            Self::SymlinkLoop => f.write_str("too many symbolic-link resolutions"),
            Self::InvalidController(message)
            | Self::InvalidCommand(message)
            | Self::InvalidPacket(message)
            | Self::Usb(message) => f.write_str(message),
            Self::InvalidQueue(queue) => write!(f, "invalid queue {queue}"),
            Self::DuplicateCommand(id) => write!(f, "command id {id} is already pending"),
            Self::NeighborUnresolved(address) => write!(f, "ARP neighbor {address} is unresolved"),
            Self::SocketMissing(port) => write!(f, "UDP port {port} is not bound"),
            Self::Core(error) => error.fmt(f),
            Self::Device(error) => error.fmt(f),
            Self::Pci(error) => error.fmt(f),
        }
    }
}

impl Error for KernelIoError {}

impl From<KernelCoreError> for KernelIoError {
    fn from(value: KernelCoreError) -> Self {
        Self::Core(value)
    }
}

impl From<VirtualDeviceError> for KernelIoError {
    fn from(value: VirtualDeviceError) -> Self {
        Self::Device(value)
    }
}

impl From<PciVirtioError> for KernelIoError {
    fn from(value: PciVirtioError) -> Self {
        Self::Pci(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsNodeKind {
    Directory,
    Regular,
    Device,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsMetadata {
    pub kind: VfsNodeKind,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: usize,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfsCredentials {
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VfsAclTag {
    User,
    Group,
    Mask,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfsAclEntry {
    pub tag: VfsAclTag,
    pub id: Option<u32>,
    pub permissions: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsSnapshotEntry {
    pub path: String,
    pub metadata: VfsMetadata,
    pub data: Vec<u8>,
    pub symlink_target: Option<String>,
    pub acl: Vec<VfsAclEntry>,
}

impl VfsCredentials {
    pub const ROOT: Self = Self { uid: 0, gid: 0 };
}

#[derive(Debug, Clone)]
struct VfsNode {
    kind: VfsNodeKind,
    mode: u16,
    uid: u32,
    gid: u32,
    data: Vec<u8>,
    target: Option<String>,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags(u8);

impl OpenFlags {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const CREATE: Self = Self(1 << 2);
    pub const TRUNCATE: Self = Self(1 << 3);
    pub const APPEND: Self = Self(1 << 4);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

#[derive(Debug, Clone)]
struct VfsHandle {
    path: String,
    offset: usize,
    flags: OpenFlags,
}

#[derive(Debug, Clone)]
pub struct Vfs {
    nodes: BTreeMap<String, VfsNode>,
    handles: BTreeMap<u32, VfsHandle>,
    acls: BTreeMap<String, Vec<VfsAclEntry>>,
    next_handle: u32,
}

impl Default for Vfs {
    fn default() -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "/".into(),
            VfsNode {
                kind: VfsNodeKind::Directory,
                mode: 0o755,
                uid: 0,
                gid: 0,
                data: Vec::new(),
                target: None,
                generation: 0,
            },
        );
        Self {
            nodes,
            handles: BTreeMap::new(),
            acls: BTreeMap::new(),
            next_handle: 3,
        }
    }
}

fn normalize_path(path: &str) -> Result<String, KernelIoError> {
    if !path.starts_with('/') || path.contains('\0') {
        return Err(KernelIoError::InvalidPath);
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(KernelIoError::InvalidPath);
                }
            }
            value => parts.push(value),
        }
    }
    Ok(if parts.is_empty() {
        "/".into()
    } else {
        format!("/{}", parts.join("/"))
    })
}

fn parent_path(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
        .unwrap_or("/")
}

impl Vfs {
    fn permits(
        &self,
        path: &str,
        node: &VfsNode,
        credentials: VfsCredentials,
        requested: u16,
    ) -> bool {
        if credentials.uid == 0 {
            return true;
        }
        if let Some(acl) = self.acls.get(path) {
            let mask = acl
                .iter()
                .find(|entry| entry.tag == VfsAclTag::Mask && entry.id.is_none())
                .map_or(0o7, |entry| entry.permissions & 0o7);
            if credentials.uid == node.uid {
                let permissions = acl
                    .iter()
                    .find(|entry| entry.tag == VfsAclTag::User && entry.id.is_none())
                    .map_or(((node.mode >> 6) & 0o7) as u8, |entry| entry.permissions);
                return permissions as u16 & requested == requested;
            }
            if let Some(entry) = acl
                .iter()
                .find(|entry| entry.tag == VfsAclTag::User && entry.id == Some(credentials.uid))
            {
                return (entry.permissions & mask) as u16 & requested == requested;
            }
            let mut group_permissions = 0u8;
            if credentials.gid == node.gid {
                group_permissions |= acl
                    .iter()
                    .find(|entry| entry.tag == VfsAclTag::Group && entry.id.is_none())
                    .map_or(((node.mode >> 3) & 0o7) as u8, |entry| entry.permissions);
            }
            for entry in acl
                .iter()
                .filter(|entry| entry.tag == VfsAclTag::Group && entry.id == Some(credentials.gid))
            {
                group_permissions |= entry.permissions;
            }
            if group_permissions != 0 {
                return (group_permissions & mask) as u16 & requested == requested;
            }
            let permissions = acl
                .iter()
                .find(|entry| entry.tag == VfsAclTag::Other && entry.id.is_none())
                .map_or((node.mode & 0o7) as u8, |entry| entry.permissions);
            return permissions as u16 & requested == requested;
        }
        let shift = if credentials.uid == node.uid {
            6
        } else if credentials.gid == node.gid {
            3
        } else {
            0
        };
        ((node.mode >> shift) & requested) == requested
    }

    fn ensure_parent(&self, path: &str) -> Result<(), KernelIoError> {
        let parent = parent_path(path);
        match self.nodes.get(parent) {
            Some(node) if node.kind == VfsNodeKind::Directory => Ok(()),
            Some(_) => Err(KernelIoError::NotDirectory(parent.into())),
            None => Err(KernelIoError::NotFound(parent.into())),
        }
    }

    fn resolve(&self, path: &str) -> Result<String, KernelIoError> {
        let mut current = normalize_path(path)?;
        for _ in 0..16 {
            let Some(node) = self.nodes.get(&current) else {
                return Ok(current);
            };
            if node.kind != VfsNodeKind::Symlink {
                return Ok(current);
            }
            current = normalize_path(node.target.as_deref().ok_or(KernelIoError::InvalidPath)?)?;
        }
        Err(KernelIoError::SymlinkLoop)
    }

    pub fn mkdir_all(&mut self, path: &str, mode: u16) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        let mut current = String::new();
        for part in path
            .trim_start_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
        {
            current.push('/');
            current.push_str(part);
            if let Some(node) = self.nodes.get(&current) {
                if node.kind != VfsNodeKind::Directory {
                    return Err(KernelIoError::NotDirectory(current));
                }
                continue;
            }
            self.nodes.insert(
                current.clone(),
                VfsNode {
                    kind: VfsNodeKind::Directory,
                    mode,
                    uid: 0,
                    gid: 0,
                    data: Vec::new(),
                    target: None,
                    generation: 0,
                },
            );
        }
        Ok(())
    }

    pub fn create_file(&mut self, path: &str, mode: u16) -> Result<(), KernelIoError> {
        self.create_file_owned(path, mode, VfsCredentials::ROOT)
    }

    pub fn create_file_owned(
        &mut self,
        path: &str,
        mode: u16,
        owner: VfsCredentials,
    ) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        self.ensure_parent(&path)?;
        if self.nodes.contains_key(&path) {
            return Err(KernelIoError::AlreadyExists(path));
        }
        self.nodes.insert(
            path,
            VfsNode {
                kind: VfsNodeKind::Regular,
                mode,
                uid: owner.uid,
                gid: owner.gid,
                data: Vec::new(),
                target: None,
                generation: 0,
            },
        );
        Ok(())
    }

    pub fn create_device(&mut self, path: &str, mode: u16) -> Result<(), KernelIoError> {
        self.create_file(path, mode)?;
        self.nodes.get_mut(&normalize_path(path)?).unwrap().kind = VfsNodeKind::Device;
        Ok(())
    }

    pub fn symlink(&mut self, target: &str, path: &str) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        let target = normalize_path(target)?;
        self.ensure_parent(&path)?;
        if self.nodes.contains_key(&path) {
            return Err(KernelIoError::AlreadyExists(path));
        }
        self.nodes.insert(
            path,
            VfsNode {
                kind: VfsNodeKind::Symlink,
                mode: 0o777,
                uid: 0,
                gid: 0,
                data: Vec::new(),
                target: Some(target),
                generation: 0,
            },
        );
        Ok(())
    }

    pub fn open(&mut self, path: &str, flags: OpenFlags) -> Result<u32, KernelIoError> {
        self.open_as(path, flags, VfsCredentials::ROOT)
    }

    pub fn open_as(
        &mut self,
        path: &str,
        flags: OpenFlags,
        credentials: VfsCredentials,
    ) -> Result<u32, KernelIoError> {
        let unresolved = normalize_path(path)?;
        if !self.nodes.contains_key(&unresolved) && flags.contains(OpenFlags::CREATE) {
            self.create_file_owned(&unresolved, 0o644, credentials)?;
        }
        let path = self.resolve(&unresolved)?;
        let node = self
            .nodes
            .get(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        if node.kind == VfsNodeKind::Directory {
            return Err(KernelIoError::IsDirectory(path));
        }
        if flags.contains(OpenFlags::READ) && !self.permits(&path, node, credentials, 0o4) {
            return Err(KernelIoError::PermissionDenied(path));
        }
        if flags.contains(OpenFlags::WRITE) && !self.permits(&path, node, credentials, 0o2) {
            return Err(KernelIoError::PermissionDenied(path));
        }
        let node = self.nodes.get_mut(&path).unwrap();
        if flags.contains(OpenFlags::TRUNCATE) && flags.contains(OpenFlags::WRITE) {
            node.data.clear();
            node.generation = node.generation.wrapping_add(1);
        }
        let offset = if flags.contains(OpenFlags::APPEND) {
            node.data.len()
        } else {
            0
        };
        let handle = self.next_handle;
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or(KernelIoError::InvalidHandle(u32::MAX))?;
        self.handles.insert(
            handle,
            VfsHandle {
                path,
                offset,
                flags,
            },
        );
        Ok(handle)
    }

    pub fn read(&mut self, handle: u32, output: &mut [u8]) -> Result<usize, KernelIoError> {
        let state = self
            .handles
            .get_mut(&handle)
            .ok_or(KernelIoError::InvalidHandle(handle))?;
        if !state.flags.contains(OpenFlags::READ) {
            return Err(KernelIoError::PermissionDenied(state.path.clone()));
        }
        let node = self
            .nodes
            .get(&state.path)
            .ok_or_else(|| KernelIoError::NotFound(state.path.clone()))?;
        let amount = output
            .len()
            .min(node.data.len().saturating_sub(state.offset));
        output[..amount].copy_from_slice(&node.data[state.offset..state.offset + amount]);
        state.offset += amount;
        Ok(amount)
    }

    pub fn write(&mut self, handle: u32, input: &[u8]) -> Result<usize, KernelIoError> {
        let state = self
            .handles
            .get_mut(&handle)
            .ok_or(KernelIoError::InvalidHandle(handle))?;
        if !state.flags.contains(OpenFlags::WRITE) {
            return Err(KernelIoError::PermissionDenied(state.path.clone()));
        }
        let node = self
            .nodes
            .get_mut(&state.path)
            .ok_or_else(|| KernelIoError::NotFound(state.path.clone()))?;
        if state.flags.contains(OpenFlags::APPEND) {
            state.offset = node.data.len();
        }
        let end = state
            .offset
            .checked_add(input.len())
            .ok_or(KernelIoError::InvalidOffset)?;
        if end > node.data.len() {
            node.data.resize(end, 0);
        }
        node.data[state.offset..end].copy_from_slice(input);
        state.offset = end;
        node.generation = node.generation.wrapping_add(1);
        Ok(input.len())
    }

    pub fn seek(&mut self, handle: u32, offset: usize) -> Result<(), KernelIoError> {
        let state = self
            .handles
            .get_mut(&handle)
            .ok_or(KernelIoError::InvalidHandle(handle))?;
        state.offset = offset;
        Ok(())
    }

    pub fn close(&mut self, handle: u32) -> Result<(), KernelIoError> {
        self.handles
            .remove(&handle)
            .map(|_| ())
            .ok_or(KernelIoError::InvalidHandle(handle))
    }

    pub fn stat(&self, path: &str) -> Result<VfsMetadata, KernelIoError> {
        let path = self.resolve(path)?;
        let node = self
            .nodes
            .get(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        Ok(VfsMetadata {
            kind: node.kind,
            mode: node.mode,
            uid: node.uid,
            gid: node.gid,
            size: node.data.len(),
            generation: node.generation,
        })
    }

    pub fn list(&self, path: &str) -> Result<Vec<String>, KernelIoError> {
        let path = self.resolve(path)?;
        if self
            .nodes
            .get(&path)
            .is_none_or(|node| node.kind != VfsNodeKind::Directory)
        {
            return Err(KernelIoError::NotDirectory(path));
        }
        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{path}/")
        };
        let mut children = BTreeSet::new();
        for candidate in self
            .nodes
            .keys()
            .filter(|candidate| candidate.starts_with(&prefix))
        {
            let rest = &candidate[prefix.len()..];
            if let Some(first) = rest.split('/').next().filter(|value| !value.is_empty()) {
                children.insert(first.to_string());
            }
        }
        Ok(children.into_iter().collect())
    }

    pub fn rename(&mut self, source: &str, destination: &str) -> Result<(), KernelIoError> {
        let source = normalize_path(source)?;
        let destination = normalize_path(destination)?;
        self.ensure_parent(&destination)?;
        if !self.nodes.contains_key(&source) {
            return Err(KernelIoError::NotFound(source));
        }
        if self.nodes.contains_key(&destination) {
            return Err(KernelIoError::AlreadyExists(destination));
        }
        let keys = self
            .nodes
            .keys()
            .filter(|path| **path == source || path.starts_with(&format!("{source}/")))
            .cloned()
            .collect::<Vec<_>>();
        for old in keys {
            let node = self.nodes.remove(&old).unwrap();
            let new = format!("{destination}{}", &old[source.len()..]);
            if let Some(acl) = self.acls.remove(&old) {
                self.acls.insert(new.clone(), acl);
            }
            self.nodes.insert(new, node);
        }
        for handle in self.handles.values_mut() {
            if handle.path == source || handle.path.starts_with(&format!("{source}/")) {
                handle.path = format!("{destination}{}", &handle.path[source.len()..]);
            }
        }
        Ok(())
    }

    pub fn unlink(&mut self, path: &str) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        let node = self
            .nodes
            .get(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        if node.kind == VfsNodeKind::Directory
            && self
                .nodes
                .keys()
                .any(|candidate| candidate.starts_with(&format!("{path}/")))
        {
            return Err(KernelIoError::PermissionDenied(
                "directory is not empty".into(),
            ));
        }
        self.nodes.remove(&path);
        self.acls.remove(&path);
        Ok(())
    }

    pub fn file_bytes(&self, path: &str) -> Result<&[u8], KernelIoError> {
        let path = self.resolve(path)?;
        let node = self
            .nodes
            .get(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        if node.kind == VfsNodeKind::Directory {
            return Err(KernelIoError::IsDirectory(path));
        }
        Ok(&node.data)
    }

    pub fn import_file(
        &mut self,
        path: &str,
        mode: u16,
        owner: VfsCredentials,
        data: &[u8],
    ) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        if let Some(parent) =
            path.rsplit_once('/')
                .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
        {
            self.mkdir_all(parent, 0o755)?;
        }
        if self.nodes.contains_key(&path) {
            return Err(KernelIoError::AlreadyExists(path));
        }
        self.nodes.insert(
            path,
            VfsNode {
                kind: VfsNodeKind::Regular,
                mode,
                uid: owner.uid,
                gid: owner.gid,
                data: data.to_vec(),
                target: None,
                generation: 0,
            },
        );
        Ok(())
    }

    pub fn chmod(&mut self, path: &str, mode: u16) -> Result<(), KernelIoError> {
        let path = self.resolve(path)?;
        let node = self
            .nodes
            .get_mut(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        node.mode = mode & 0o7777;
        node.generation = node.generation.wrapping_add(1);
        Ok(())
    }

    pub fn chown(&mut self, path: &str, owner: VfsCredentials) -> Result<(), KernelIoError> {
        let path = self.resolve(path)?;
        let node = self
            .nodes
            .get_mut(&path)
            .ok_or_else(|| KernelIoError::NotFound(path.clone()))?;
        node.uid = owner.uid;
        node.gid = owner.gid;
        node.generation = node.generation.wrapping_add(1);
        Ok(())
    }

    pub fn set_acl(&mut self, path: &str, mut acl: Vec<VfsAclEntry>) -> Result<(), KernelIoError> {
        let path = self.resolve(path)?;
        if !self.nodes.contains_key(&path) {
            return Err(KernelIoError::NotFound(path));
        }
        for entry in &acl {
            if entry.permissions > 0o7
                || matches!(entry.tag, VfsAclTag::Mask | VfsAclTag::Other) && entry.id.is_some()
            {
                return Err(KernelIoError::PermissionDenied(
                    "invalid POSIX ACL entry".into(),
                ));
            }
        }
        acl.sort_by_key(|entry| (entry.tag, entry.id));
        acl.dedup_by_key(|entry| (entry.tag, entry.id));
        self.acls.insert(path.clone(), acl);
        if let Some(node) = self.nodes.get_mut(&path) {
            node.generation = node.generation.wrapping_add(1);
        }
        Ok(())
    }

    pub fn acl(&self, path: &str) -> Result<&[VfsAclEntry], KernelIoError> {
        let path = self.resolve(path)?;
        if !self.nodes.contains_key(&path) {
            return Err(KernelIoError::NotFound(path));
        }
        Ok(self.acls.get(&path).map_or(&[], Vec::as_slice))
    }

    pub fn snapshot_under(&self, root: &str) -> Result<Vec<VfsSnapshotEntry>, KernelIoError> {
        let root = normalize_path(root)?;
        let prefix = if root == "/" {
            "/".to_string()
        } else {
            format!("{root}/")
        };
        if self
            .nodes
            .get(&root)
            .is_none_or(|node| node.kind != VfsNodeKind::Directory)
        {
            return Err(KernelIoError::NotDirectory(root));
        }
        let mut snapshot = Vec::new();
        for (path, node) in self
            .nodes
            .iter()
            .filter(|(path, _)| path.starts_with(&prefix))
        {
            let relative = path[prefix.len()..].to_string();
            if relative.is_empty() {
                continue;
            }
            snapshot.push(VfsSnapshotEntry {
                path: relative,
                metadata: VfsMetadata {
                    kind: node.kind,
                    mode: node.mode,
                    uid: node.uid,
                    gid: node.gid,
                    size: node.data.len(),
                    generation: node.generation,
                },
                data: node.data.clone(),
                symlink_target: node.target.clone(),
                acl: self.acls.get(path).cloned().unwrap_or_default(),
            });
        }
        Ok(snapshot)
    }

    pub fn has_open_handles_under(&self, path: &str) -> Result<bool, KernelIoError> {
        let path = normalize_path(path)?;
        let prefix = format!("{path}/");
        Ok(self
            .handles
            .values()
            .any(|handle| handle.path == path || handle.path.starts_with(&prefix)))
    }

    pub fn remove_tree(&mut self, path: &str) -> Result<(), KernelIoError> {
        let path = normalize_path(path)?;
        if path == "/" {
            return Err(KernelIoError::PermissionDenied(
                "cannot remove the VFS root".into(),
            ));
        }
        if self.has_open_handles_under(&path)? {
            return Err(KernelIoError::PermissionDenied(
                "mount contains open handles".into(),
            ));
        }
        let prefix = format!("{path}/");
        let keys = self
            .nodes
            .keys()
            .filter(|candidate| **candidate == path || candidate.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return Err(KernelIoError::NotFound(path));
        }
        for key in keys {
            self.nodes.remove(&key);
            self.acls.remove(&key);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOpcode {
    Read,
    Write,
    Flush,
    Discard,
    Identify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCommand {
    pub id: u16,
    pub opcode: BlockOpcode,
    pub lba: u64,
    pub blocks: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCompletion {
    pub id: u16,
    pub success: bool,
    pub data: Vec<u8>,
}

fn execute_block(
    device: &mut VirtualBlockDevice,
    command: &BlockCommand,
) -> Result<Vec<u8>, KernelIoError> {
    let bytes = usize::try_from(command.blocks)
        .ok()
        .and_then(|blocks| blocks.checked_mul(device.sector_size as usize))
        .ok_or(KernelIoError::InvalidCommand(
            "block transfer length overflow",
        ))?;
    match command.opcode {
        BlockOpcode::Read => {
            if command.blocks == 0 {
                return Err(KernelIoError::InvalidCommand("zero-length block read"));
            }
            let mut output = vec![0; bytes];
            device.read_sectors(command.lba, &mut output)?;
            Ok(output)
        }
        BlockOpcode::Write => {
            if command.blocks == 0 || command.data.len() != bytes {
                return Err(KernelIoError::InvalidCommand(
                    "block write payload size mismatch",
                ));
            }
            device.write_sectors(command.lba, &command.data)?;
            Ok(Vec::new())
        }
        BlockOpcode::Flush => {
            device.flush();
            Ok(Vec::new())
        }
        BlockOpcode::Discard => {
            device.discard(command.lba * device.sector_size as u64, bytes)?;
            Ok(Vec::new())
        }
        BlockOpcode::Identify => {
            let mut data = Vec::new();
            data.extend_from_slice(&(device.capacity() as u64).to_le_bytes());
            data.extend_from_slice(&device.sector_size.to_le_bytes());
            data.push(u8::from(device.read_only));
            Ok(data)
        }
    }
}

#[derive(Debug, Clone)]
pub struct NvmeController {
    namespaces: BTreeMap<u32, VirtualBlockDevice>,
    queues: BTreeMap<u16, VecDeque<BlockCompletion>>,
    pending: BTreeSet<(u16, u16)>,
    pub doorbell_writes: u64,
    pub interrupts: u64,
}

impl NvmeController {
    pub fn new() -> Self {
        let mut queues = BTreeMap::new();
        queues.insert(0, VecDeque::new());
        Self {
            namespaces: BTreeMap::new(),
            queues,
            pending: BTreeSet::new(),
            doorbell_writes: 0,
            interrupts: 0,
        }
    }

    pub fn attach_namespace(
        &mut self,
        id: u32,
        device: VirtualBlockDevice,
    ) -> Result<(), KernelIoError> {
        if id == 0 || self.namespaces.insert(id, device).is_some() {
            return Err(KernelIoError::InvalidController(
                "invalid or duplicate NVMe namespace",
            ));
        }
        Ok(())
    }

    pub fn create_io_queue(&mut self, id: u16, depth: usize) -> Result<(), KernelIoError> {
        if id == 0 || depth < 2 || depth > 65536 || self.queues.contains_key(&id) {
            return Err(KernelIoError::InvalidQueue(id));
        }
        self.queues.insert(id, VecDeque::with_capacity(depth));
        Ok(())
    }

    pub fn submit(
        &mut self,
        queue: u16,
        namespace: u32,
        command: BlockCommand,
    ) -> Result<(), KernelIoError> {
        if !self.queues.contains_key(&queue) {
            return Err(KernelIoError::InvalidQueue(queue));
        }
        if !self.pending.insert((queue, command.id)) {
            return Err(KernelIoError::DuplicateCommand(command.id));
        }
        let device = self
            .namespaces
            .get_mut(&namespace)
            .ok_or(KernelIoError::InvalidController("NVMe namespace is absent"))?;
        let data = execute_block(device, &command)?;
        self.queues
            .get_mut(&queue)
            .unwrap()
            .push_back(BlockCompletion {
                id: command.id,
                success: true,
                data,
            });
        self.doorbell_writes = self.doorbell_writes.wrapping_add(1);
        self.interrupts = self.interrupts.wrapping_add(1);
        Ok(())
    }

    pub fn completion(&mut self, queue: u16) -> Result<Option<BlockCompletion>, KernelIoError> {
        let completion = self
            .queues
            .get_mut(&queue)
            .ok_or(KernelIoError::InvalidQueue(queue))?
            .pop_front();
        if let Some(completion) = &completion {
            self.pending.remove(&(queue, completion.id));
        }
        Ok(completion)
    }

    pub fn namespace_bytes(&self, namespace: u32) -> Result<&[u8], KernelIoError> {
        self.namespaces
            .get(&namespace)
            .map(VirtualBlockDevice::bytes)
            .ok_or(KernelIoError::InvalidController("NVMe namespace is absent"))
    }
}

impl Default for NvmeController {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct AhciPort {
    device: VirtualBlockDevice,
    active_slots: u32,
    completions: VecDeque<BlockCompletion>,
}

#[derive(Debug, Clone)]
pub struct AhciController {
    ports: Vec<Option<AhciPort>>,
    pub interrupts: u64,
}

impl AhciController {
    pub fn new(port_count: usize) -> Result<Self, KernelIoError> {
        if port_count == 0 || port_count > 32 {
            return Err(KernelIoError::InvalidController(
                "AHCI port count must be 1..32",
            ));
        }
        Ok(Self {
            ports: vec![None; port_count],
            interrupts: 0,
        })
    }

    pub fn attach(&mut self, port: usize, device: VirtualBlockDevice) -> Result<(), KernelIoError> {
        let slot = self
            .ports
            .get_mut(port)
            .ok_or(KernelIoError::InvalidController("AHCI port is absent"))?;
        if slot.is_some() {
            return Err(KernelIoError::InvalidController(
                "AHCI port is already occupied",
            ));
        }
        *slot = Some(AhciPort {
            device,
            active_slots: 0,
            completions: VecDeque::new(),
        });
        Ok(())
    }

    pub fn issue(
        &mut self,
        port: usize,
        slot: u8,
        command: BlockCommand,
    ) -> Result<(), KernelIoError> {
        if slot >= 32 {
            return Err(KernelIoError::InvalidCommand(
                "AHCI command slot must be below 32",
            ));
        }
        let port = self
            .ports
            .get_mut(port)
            .and_then(Option::as_mut)
            .ok_or(KernelIoError::InvalidController("AHCI port has no device"))?;
        let mask = 1u32 << slot;
        if port.active_slots & mask != 0 {
            return Err(KernelIoError::DuplicateCommand(command.id));
        }
        port.active_slots |= mask;
        let data = execute_block(&mut port.device, &command)?;
        port.active_slots &= !mask;
        port.completions.push_back(BlockCompletion {
            id: command.id,
            success: true,
            data,
        });
        self.interrupts = self.interrupts.wrapping_add(1);
        Ok(())
    }

    pub fn completion(&mut self, port: usize) -> Result<Option<BlockCompletion>, KernelIoError> {
        self.ports
            .get_mut(port)
            .and_then(Option::as_mut)
            .map(|port| port.completions.pop_front())
            .ok_or(KernelIoError::InvalidController("AHCI port has no device"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcieDeviceInfo {
    pub address: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub programming_interface: u8,
}

#[derive(Debug, Clone, Default)]
pub struct PcieHost {
    pub bus: PciBus,
}

impl PcieHost {
    pub fn attach(&mut self, function: PciFunction) -> Result<(), KernelIoError> {
        self.bus.attach(function)?;
        Ok(())
    }

    pub fn enumerate(&mut self) -> Result<Vec<PcieDeviceInfo>, KernelIoError> {
        let addresses = self.bus.functions.keys().copied().collect::<Vec<_>>();
        let mut devices = Vec::new();
        for address in addresses {
            let vendor_id = self.bus.config_read(address, 0, 2)? as u16;
            if vendor_id == 0xffff {
                continue;
            }
            devices.push(PcieDeviceInfo {
                address,
                vendor_id,
                device_id: self.bus.config_read(address, 2, 2)? as u16,
                programming_interface: self.bus.config_read(address, 9, 1)? as u8,
                subclass: self.bus.config_read(address, 10, 1)? as u8,
                class: self.bus.config_read(address, 11, 1)? as u8,
            });
        }
        Ok(devices)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address(pub [u8; 4]);

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagram {
    pub source: Ipv4Address,
    pub source_port: u16,
    pub destination: Ipv4Address,
    pub destination_port: u16,
    pub payload: Vec<u8>,
}

fn internet_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for chunk in bytes.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], 0])
        };
        sum += word as u32;
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
    }
    !(sum as u16)
}

#[derive(Debug, Clone)]
pub struct NetworkStack {
    pub device: VirtualNetworkDevice,
    pub address: Ipv4Address,
    pub gateway: Option<Ipv4Address>,
    arp: BTreeMap<Ipv4Address, [u8; 6]>,
    sockets: BTreeMap<u16, VecDeque<UdpDatagram>>,
    next_ip_id: u16,
}

impl NetworkStack {
    pub fn new(mac: [u8; 6], address: Ipv4Address) -> Result<Self, KernelIoError> {
        Ok(Self {
            device: VirtualNetworkDevice::new(mac, 1500, 256)?,
            address,
            gateway: None,
            arp: BTreeMap::new(),
            sockets: BTreeMap::new(),
            next_ip_id: 1,
        })
    }

    pub fn add_neighbor(&mut self, address: Ipv4Address, mac: [u8; 6]) {
        self.arp.insert(address, mac);
    }

    pub fn bind_udp(&mut self, port: u16) -> Result<(), KernelIoError> {
        if port == 0 || self.sockets.contains_key(&port) {
            return Err(KernelIoError::InvalidPacket(
                "UDP port is zero or already bound",
            ));
        }
        self.sockets.insert(port, VecDeque::new());
        Ok(())
    }

    pub fn send_udp(
        &mut self,
        source_port: u16,
        destination: Ipv4Address,
        destination_port: u16,
        payload: &[u8],
    ) -> Result<(), KernelIoError> {
        if !self.sockets.contains_key(&source_port) {
            return Err(KernelIoError::SocketMissing(source_port));
        }
        let destination_mac = *self
            .arp
            .get(&destination)
            .ok_or(KernelIoError::NeighborUnresolved(destination))?;
        let udp_len = 8usize
            .checked_add(payload.len())
            .ok_or(KernelIoError::InvalidPacket("UDP length overflow"))?;
        let ip_len = 20usize
            .checked_add(udp_len)
            .ok_or(KernelIoError::InvalidPacket("IPv4 length overflow"))?;
        if ip_len > 1500 {
            return Err(KernelIoError::InvalidPacket(
                "IPv4 fragmentation is required",
            ));
        }
        let mut frame = vec![0u8; 14 + ip_len];
        frame[..6].copy_from_slice(&destination_mac);
        frame[6..12].copy_from_slice(&self.device.mac);
        frame[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        let ip = &mut frame[14..34];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
        ip[4..6].copy_from_slice(&self.next_ip_id.to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&self.address.0);
        ip[16..20].copy_from_slice(&destination.0);
        let checksum = internet_checksum(ip);
        ip[10..12].copy_from_slice(&checksum.to_be_bytes());
        let udp = &mut frame[34..42];
        udp[..2].copy_from_slice(&source_port.to_be_bytes());
        udp[2..4].copy_from_slice(&destination_port.to_be_bytes());
        udp[4..6].copy_from_slice(&(udp_len as u16).to_be_bytes());
        frame[42..].copy_from_slice(payload);
        self.device.transmit(&frame)?;
        self.next_ip_id = self.next_ip_id.wrapping_add(1);
        Ok(())
    }

    pub fn inject_frame(&mut self, frame: &[u8]) -> Result<(), KernelIoError> {
        self.device.inject_received(frame)?;
        while let Some(frame) = self.device.receive() {
            self.process_frame(&frame)?;
        }
        Ok(())
    }

    fn process_frame(&mut self, frame: &[u8]) -> Result<(), KernelIoError> {
        if frame.len() < 42 || u16::from_be_bytes([frame[12], frame[13]]) != 0x0800 {
            return Err(KernelIoError::InvalidPacket(
                "unsupported or truncated Ethernet frame",
            ));
        }
        let ihl = (frame[14] & 0x0f) as usize * 4;
        if ihl < 20 || frame.len() < 14 + ihl + 8 || frame[23] != 17 {
            return Err(KernelIoError::InvalidPacket("invalid IPv4/UDP header"));
        }
        if internet_checksum(&frame[14..14 + ihl]) != 0 {
            return Err(KernelIoError::InvalidPacket("invalid IPv4 checksum"));
        }
        let total = u16::from_be_bytes([frame[16], frame[17]]) as usize;
        if total < ihl + 8 || 14 + total > frame.len() {
            return Err(KernelIoError::InvalidPacket("invalid IPv4 total length"));
        }
        let udp = 14 + ihl;
        let destination_port = u16::from_be_bytes([frame[udp + 2], frame[udp + 3]]);
        let queue = self
            .sockets
            .get_mut(&destination_port)
            .ok_or(KernelIoError::SocketMissing(destination_port))?;
        let udp_len = u16::from_be_bytes([frame[udp + 4], frame[udp + 5]]) as usize;
        if udp_len < 8 || udp + udp_len > 14 + total {
            return Err(KernelIoError::InvalidPacket("invalid UDP length"));
        }
        queue.push_back(UdpDatagram {
            source: Ipv4Address(frame[26..30].try_into().unwrap()),
            source_port: u16::from_be_bytes([frame[udp], frame[udp + 1]]),
            destination: Ipv4Address(frame[30..34].try_into().unwrap()),
            destination_port,
            payload: frame[udp + 8..udp + udp_len].to_vec(),
        });
        Ok(())
    }

    pub fn receive_udp(&mut self, port: u16) -> Result<Option<UdpDatagram>, KernelIoError> {
        Ok(self
            .sockets
            .get_mut(&port)
            .ok_or(KernelIoError::SocketMissing(port))?
            .pop_front())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    Low,
    Full,
    High,
    Super,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDeviceDescriptor {
    pub vendor: u16,
    pub product: u16,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub max_packet_size: u16,
    pub configurations: u8,
}

#[derive(Debug, Clone)]
struct UsbDevice {
    speed: UsbSpeed,
    descriptor: UsbDeviceDescriptor,
    address: u8,
    configuration: u8,
}

#[derive(Debug, Clone)]
pub struct UsbHostController {
    ports: Vec<Option<UsbDevice>>,
    next_address: u8,
    pub transfers: u64,
}

impl UsbHostController {
    pub fn new(ports: usize) -> Result<Self, KernelIoError> {
        if ports == 0 || ports > 255 {
            return Err(KernelIoError::Usb("USB host port count must be 1..255"));
        }
        Ok(Self {
            ports: vec![None; ports],
            next_address: 1,
            transfers: 0,
        })
    }

    pub fn connect(
        &mut self,
        port: usize,
        speed: UsbSpeed,
        descriptor: UsbDeviceDescriptor,
    ) -> Result<(), KernelIoError> {
        if descriptor.max_packet_size == 0 || descriptor.configurations == 0 {
            return Err(KernelIoError::Usb("invalid USB device descriptor"));
        }
        let slot = self
            .ports
            .get_mut(port)
            .ok_or(KernelIoError::Usb("USB port is absent"))?;
        if slot.is_some() {
            return Err(KernelIoError::Usb("USB port is occupied"));
        }
        *slot = Some(UsbDevice {
            speed,
            descriptor,
            address: 0,
            configuration: 0,
        });
        Ok(())
    }

    pub fn enumerate(&mut self, port: usize) -> Result<(u8, UsbDeviceDescriptor), KernelIoError> {
        let device = self
            .ports
            .get_mut(port)
            .and_then(Option::as_mut)
            .ok_or(KernelIoError::Usb("USB device is absent"))?;
        if self.next_address == 0 || self.next_address > 127 {
            return Err(KernelIoError::Usb("USB address space is exhausted"));
        }
        device.address = self.next_address;
        device.configuration = 1;
        self.next_address += 1;
        self.transfers = self.transfers.wrapping_add(3);
        Ok((device.address, device.descriptor.clone()))
    }

    pub fn speed(&self, port: usize) -> Result<UsbSpeed, KernelIoError> {
        self.ports
            .get(port)
            .and_then(Option::as_ref)
            .map(|device| device.speed)
            .ok_or(KernelIoError::Usb("USB device is absent"))
    }

    pub fn disconnect(&mut self, port: usize) -> Result<(), KernelIoError> {
        self.ports
            .get_mut(port)
            .ok_or(KernelIoError::Usb("USB port is absent"))?
            .take()
            .map(|_| ())
            .ok_or(KernelIoError::Usb("USB device is absent"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidKeyTransition {
    pub usage: u8,
    pub pressed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HidKeyboard {
    previous: [u8; 8],
}

impl HidKeyboard {
    pub fn consume_boot_report(
        &mut self,
        report: [u8; 8],
    ) -> Result<Vec<HidKeyTransition>, KernelIoError> {
        if report[2..].contains(&1) {
            return Err(KernelIoError::Usb("HID keyboard reports rollover"));
        }
        let mut transitions = Vec::new();
        for bit in 0..8 {
            let usage = 0xe0 + bit;
            let was = self.previous[0] & (1 << bit) != 0;
            let now = report[0] & (1 << bit) != 0;
            if was != now {
                transitions.push(HidKeyTransition {
                    usage,
                    pressed: now,
                });
            }
        }
        for usage in self.previous[2..]
            .iter()
            .copied()
            .filter(|usage| *usage != 0)
        {
            if !report[2..].contains(&usage) {
                transitions.push(HidKeyTransition {
                    usage,
                    pressed: false,
                });
            }
        }
        for usage in report[2..].iter().copied().filter(|usage| *usage != 0) {
            if !self.previous[2..].contains(&usage) {
                transitions.push(HidKeyTransition {
                    usage,
                    pressed: true,
                });
            }
        }
        self.previous = report;
        Ok(transitions)
    }
}

#[derive(Debug, Clone)]
pub struct KernelDisplay {
    pub framebuffer: VirtualFramebuffer,
}

impl KernelDisplay {
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Result<Self, KernelIoError> {
        Ok(Self {
            framebuffer: VirtualFramebuffer::new(width, height, format)?,
        })
    }

    pub fn clear(&mut self, rgba: u32) -> Result<(), KernelIoError> {
        self.framebuffer.fill_rect(
            DirtyRect {
                x: 0,
                y: 0,
                width: self.framebuffer.width,
                height: self.framebuffer.height,
            },
            rgba,
        )?;
        Ok(())
    }

    pub fn rect(&mut self, rect: DirtyRect, rgba: u32) -> Result<(), KernelIoError> {
        self.framebuffer.fill_rect(rect, rgba)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KernelSystem {
    pub core: KernelCore,
    pub vfs: MountedVfs,
    pub pcie: PcieHost,
    pub nvme: NvmeController,
    pub ahci: AhciController,
    pub network: NetworkStack,
    pub usb: UsbHostController,
    pub keyboard: HidKeyboard,
    pub display: KernelDisplay,
}

impl KernelSystem {
    pub fn new(frames: usize, cpus: usize) -> Result<Self, KernelIoError> {
        let mut vfs = MountedVfs::default();
        vfs.mkdir_all("/dev", 0o755)?;
        vfs.mkdir_all("/proc", 0o555)?;
        vfs.mkdir_all("/tmp", 0o777)?;
        Ok(Self {
            core: KernelCore::new(frames, cpus)?,
            vfs,
            pcie: PcieHost::default(),
            nvme: NvmeController::new(),
            ahci: AhciController::new(4)?,
            network: NetworkStack::new([0x02, 0, 0, 0, 0, 1], Ipv4Address([10, 0, 2, 15]))?,
            usb: UsbHostController::new(8)?,
            keyboard: HidKeyboard::default(),
            display: KernelDisplay::new(320, 200, PixelFormat::Bgra8888)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfs_covers_paths_links_handles_sparse_write_rename_and_unlink() {
        let mut vfs = Vfs::default();
        vfs.mkdir_all("/etc/nc", 0o755).unwrap();
        let handle = vfs
            .open(
                "/etc/nc/config",
                OpenFlags::READ
                    .union(OpenFlags::WRITE)
                    .union(OpenFlags::CREATE),
            )
            .unwrap();
        vfs.write(handle, b"kernel=raw").unwrap();
        vfs.seek(handle, 0).unwrap();
        let mut output = [0; 16];
        let read = vfs.read(handle, &mut output).unwrap();
        assert_eq!(&output[..read], b"kernel=raw");
        vfs.symlink("/etc/nc/config", "/etc/current").unwrap();
        assert_eq!(vfs.file_bytes("/etc/current").unwrap(), b"kernel=raw");
        vfs.rename("/etc/nc", "/etc/ncc").unwrap();
        assert_eq!(vfs.list("/etc/ncc").unwrap(), vec!["config"]);
        vfs.close(handle).unwrap();
        vfs.unlink("/etc/ncc/config").unwrap();
        vfs.unlink("/etc/ncc").unwrap();
    }

    #[test]
    fn nvme_and_ahci_move_exact_sector_bytes_and_complete_commands() {
        let mut nvme = NvmeController::new();
        nvme.attach_namespace(1, VirtualBlockDevice::new(4096, 512, false).unwrap())
            .unwrap();
        nvme.create_io_queue(1, 32).unwrap();
        let payload = vec![0x5a; 512];
        nvme.submit(
            1,
            1,
            BlockCommand {
                id: 7,
                opcode: BlockOpcode::Write,
                lba: 2,
                blocks: 1,
                data: payload.clone(),
            },
        )
        .unwrap();
        assert!(nvme.completion(1).unwrap().unwrap().success);
        nvme.submit(
            1,
            1,
            BlockCommand {
                id: 8,
                opcode: BlockOpcode::Read,
                lba: 2,
                blocks: 1,
                data: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(nvme.completion(1).unwrap().unwrap().data, payload);

        let mut ahci = AhciController::new(2).unwrap();
        ahci.attach(0, VirtualBlockDevice::new(4096, 512, false).unwrap())
            .unwrap();
        ahci.issue(
            0,
            3,
            BlockCommand {
                id: 9,
                opcode: BlockOpcode::Write,
                lba: 1,
                blocks: 1,
                data: vec![0xa5; 512],
            },
        )
        .unwrap();
        assert_eq!(ahci.completion(0).unwrap().unwrap().id, 9);
        assert_eq!(ahci.interrupts, 1);
    }

    #[test]
    fn pcie_enumeration_reads_real_configuration_space() {
        let mut host = PcieHost::default();
        host.attach(PciFunction::new(
            PciAddress::new(0, 0, 1, 0).unwrap(),
            0x1b36,
            0x0010,
            1,
            0x01,
            0x08,
            0x02,
            0x1b36,
            0x0010,
        ))
        .unwrap();
        let devices = host.enumerate().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].class, 0x01);
        assert_eq!(devices[0].subclass, 0x08);
        assert_eq!(devices[0].programming_interface, 0x02);
    }

    #[test]
    fn ethernet_ipv4_udp_roundtrip_validates_checksum_and_socket() {
        let a_ip = Ipv4Address([10, 0, 2, 15]);
        let b_ip = Ipv4Address([10, 0, 2, 16]);
        let a_mac = [0x02, 0, 0, 0, 0, 1];
        let b_mac = [0x02, 0, 0, 0, 0, 2];
        let mut a = NetworkStack::new(a_mac, a_ip).unwrap();
        let mut b = NetworkStack::new(b_mac, b_ip).unwrap();
        a.bind_udp(4000).unwrap();
        b.bind_udp(5000).unwrap();
        a.add_neighbor(b_ip, b_mac);
        a.send_udp(4000, b_ip, 5000, b"nC kernel UDP").unwrap();
        let frame = a.device.take_transmitted().unwrap();
        b.inject_frame(&frame).unwrap();
        let datagram = b.receive_udp(5000).unwrap().unwrap();
        assert_eq!(datagram.source, a_ip);
        assert_eq!(datagram.payload, b"nC kernel UDP");
    }

    #[test]
    fn usb_enumeration_hid_transitions_and_framebuffer_are_material() {
        let mut system = KernelSystem::new(64, 2).unwrap();
        system
            .usb
            .connect(
                0,
                UsbSpeed::Full,
                UsbDeviceDescriptor {
                    vendor: 0x1234,
                    product: 0x5678,
                    class: 3,
                    subclass: 1,
                    protocol: 1,
                    max_packet_size: 64,
                    configurations: 1,
                },
            )
            .unwrap();
        let (address, descriptor) = system.usb.enumerate(0).unwrap();
        assert_eq!(address, 1);
        assert_eq!(descriptor.protocol, 1);
        let pressed = system
            .keyboard
            .consume_boot_report([0, 0, 4, 0, 0, 0, 0, 0])
            .unwrap();
        let released = system.keyboard.consume_boot_report([0; 8]).unwrap();
        assert_eq!(
            pressed,
            vec![HidKeyTransition {
                usage: 4,
                pressed: true
            }]
        );
        assert_eq!(
            released,
            vec![HidKeyTransition {
                usage: 4,
                pressed: false
            }]
        );
        let before = system.display.framebuffer.checksum();
        system.display.clear(0xff10_2030).unwrap();
        system
            .display
            .rect(
                DirtyRect {
                    x: 10,
                    y: 10,
                    width: 20,
                    height: 8,
                },
                0xffff_0000,
            )
            .unwrap();
        assert_ne!(system.display.framebuffer.checksum(), before);
        assert_eq!(system.display.framebuffer.take_dirty().len(), 2);
    }
}
