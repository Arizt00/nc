use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapProtection(u32);

impl MapProtection {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits & (Self::READ.0 | Self::WRITE.0 | Self::EXECUTE.0))
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn is_readable(self) -> bool {
        self.contains(Self::READ)
    }

    pub const fn is_writable(self) -> bool {
        self.contains(Self::WRITE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapFlags(u32);

impl MapFlags {
    pub const SHARED: Self = Self(1 << 0);
    pub const PRIVATE: Self = Self(1 << 1);
    pub const FIXED: Self = Self(1 << 2);
    pub const ANONYMOUS: Self = Self(1 << 3);
    pub const POPULATE: Self = Self(1 << 4);
    pub const NO_RESERVE: Self = Self(1 << 5);
    pub const STACK: Self = Self(1 << 6);
    pub const LOCKED: Self = Self(1 << 7);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits & 0xff)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAdvice {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSync {
    Async,
    Sync,
    Invalidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MapSource {
    Anonymous,
    File { path: PathBuf, offset: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapBackend {
    Unix,
    #[cfg(windows)]
    WindowsVirtual,
    #[cfg(windows)]
    WindowsFileView,
}

#[derive(Debug)]
pub struct HostMemoryMap {
    ptr: NonNull<u8>,
    len: usize,
    protection: MapProtection,
    flags: MapFlags,
    source: MapSource,
    backend: MapBackend,
}

// The mapping is process memory. Mutation is synchronized by callers when shared.
unsafe impl Send for HostMemoryMap {}
unsafe impl Sync for HostMemoryMap {}

impl HostMemoryMap {
    pub fn anonymous(len: usize, protection: MapProtection, flags: MapFlags) -> io::Result<Self> {
        Self::anonymous_at(None, len, protection, flags)
    }

    pub fn anonymous_at(
        requested_address: Option<usize>,
        len: usize,
        protection: MapProtection,
        flags: MapFlags,
    ) -> io::Result<Self> {
        validate_map_request(len, flags, true)?;
        map_anonymous_platform(requested_address, len, protection, flags)
    }

    pub fn file(
        path: impl AsRef<Path>,
        offset: u64,
        len: usize,
        protection: MapProtection,
        flags: MapFlags,
    ) -> io::Result<Self> {
        Self::file_at(None, path, offset, len, protection, flags)
    }

    pub fn file_at(
        requested_address: Option<usize>,
        path: impl AsRef<Path>,
        offset: u64,
        len: usize,
        protection: MapProtection,
        flags: MapFlags,
    ) -> io::Result<Self> {
        validate_map_request(len, flags, false)?;
        let path = path.as_ref().to_path_buf();
        map_file_platform(requested_address, &path, offset, len, protection, flags)
    }

    pub fn address(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn protection(&self) -> MapProtection {
        self.protection
    }

    pub fn flags(&self) -> MapFlags {
        self.flags
    }

    pub fn is_file_backed(&self) -> bool {
        matches!(self.source, MapSource::File { .. })
    }

    pub fn contains_address(&self, address: usize, width: usize) -> bool {
        let Some(end) = address.checked_add(width) else {
            return false;
        };
        let start = self.address();
        let Some(mapping_end) = start.checked_add(self.len) else {
            return false;
        };
        address >= start && end <= mapping_end
    }

    pub fn read(&self, offset: usize, out: &mut [u8]) -> io::Result<()> {
        let end = offset
            .checked_add(out.len())
            .ok_or_else(|| invalid_input("mapped read offset overflow"))?;
        if end > self.len {
            return Err(invalid_input("mapped read leaves mapping"));
        }
        if !self.protection.is_readable() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "mapping is not readable",
            ));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.ptr.as_ptr().add(offset),
                out.as_mut_ptr(),
                out.len(),
            );
        }
        Ok(())
    }

    pub fn write(&mut self, offset: usize, bytes: &[u8]) -> io::Result<()> {
        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| invalid_input("mapped write offset overflow"))?;
        if end > self.len {
            return Err(invalid_input("mapped write leaves mapping"));
        }
        if !self.protection.is_writable() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "mapping is not writable",
            ));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.ptr.as_ptr().add(offset),
                bytes.len(),
            );
        }
        Ok(())
    }

    pub fn protect(&mut self, protection: MapProtection) -> io::Result<()> {
        protect_platform(self.ptr, self.len, protection)?;
        self.protection = protection;
        Ok(())
    }

    pub fn sync(&self, mode: MapSync) -> io::Result<()> {
        sync_platform(self.ptr, self.len, mode, self.backend)
    }

    pub fn advise(&self, advice: MapAdvice) -> io::Result<()> {
        advise_platform(self.ptr, self.len, advice, self.backend)
    }

    pub fn lock(&self) -> io::Result<()> {
        lock_platform(self.ptr, self.len)
    }

    pub fn unlock(&self) -> io::Result<()> {
        unlock_platform(self.ptr, self.len)
    }

    pub fn remap(
        &mut self,
        new_len: usize,
        may_move: bool,
        fixed_address: Option<usize>,
    ) -> io::Result<()> {
        if new_len == 0 {
            return Err(invalid_input("remap length must be greater than zero"));
        }

        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let mut flags = if may_move { libc::MREMAP_MAYMOVE } else { 0 };
            let result = unsafe {
                if let Some(address) = fixed_address {
                    if !may_move {
                        return Err(invalid_input("fixed remap requires may_move"));
                    }
                    flags |= libc::MREMAP_FIXED;
                    libc::mremap(
                        self.ptr.as_ptr().cast(),
                        self.len,
                        new_len,
                        flags,
                        address as *mut libc::c_void,
                    )
                } else {
                    libc::mremap(self.ptr.as_ptr().cast(), self.len, new_len, flags)
                }
            };
            if result == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            self.ptr = NonNull::new(result.cast())
                .ok_or_else(|| io::Error::other("mremap returned null"))?;
            self.len = new_len;
            return Ok(());
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            if fixed_address.is_some() && !may_move {
                return Err(invalid_input("fixed remap requires may_move"));
            }
            let mut replacement = match &self.source {
                MapSource::Anonymous => {
                    Self::anonymous_at(fixed_address, new_len, self.protection, self.flags)?
                }
                MapSource::File { path, offset } => Self::file_at(
                    fixed_address,
                    path,
                    *offset,
                    new_len,
                    self.protection,
                    self.flags,
                )?,
            };
            let copy_len = self.len.min(new_len);
            if copy_len > 0 && self.protection.is_readable() && replacement.protection.is_writable()
            {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        self.ptr.as_ptr(),
                        replacement.ptr.as_ptr(),
                        copy_len,
                    );
                }
            }
            std::mem::swap(self, &mut replacement);
            Ok(())
        }
    }

    pub fn remap_moved(&mut self, new_len: usize, fixed_address: Option<usize>) -> io::Result<()> {
        if new_len == 0 {
            return Err(invalid_input("remap length must be greater than zero"));
        }
        let mut replacement = match &self.source {
            MapSource::Anonymous => {
                Self::anonymous_at(fixed_address, new_len, self.protection, self.flags)?
            }
            MapSource::File { path, offset } => Self::file_at(
                fixed_address,
                path,
                *offset,
                new_len,
                self.protection,
                self.flags,
            )?,
        };
        let copy_len = self.len.min(new_len);
        if copy_len > 0 && self.protection.is_readable() && self.protection.is_writable() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.ptr.as_ptr(),
                    replacement.ptr.as_ptr(),
                    copy_len,
                );
            }
        }
        if fixed_address.is_none() && replacement.address() == self.address() {
            return Err(io::Error::other(
                "moving remap unexpectedly reused the live source address",
            ));
        }
        std::mem::swap(self, &mut replacement);
        Ok(())
    }
}

impl Drop for HostMemoryMap {
    fn drop(&mut self) {
        unmap_platform(self.ptr, self.len, self.backend);
    }
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn validate_map_request(len: usize, flags: MapFlags, anonymous: bool) -> io::Result<()> {
    if len == 0 {
        return Err(invalid_input("mapping length must be greater than zero"));
    }
    if flags.contains(MapFlags::SHARED) && flags.contains(MapFlags::PRIVATE) {
        return Err(invalid_input("mapping cannot be both shared and private"));
    }
    if !anonymous && flags.contains(MapFlags::ANONYMOUS) {
        return Err(invalid_input("file mapping cannot use anonymous flag"));
    }
    Ok(())
}

fn open_mapping_file(path: &Path, protection: MapProtection) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    if protection.is_writable() {
        options.write(true);
    }
    options.open(path)
}

#[cfg(unix)]
fn unix_protection(protection: MapProtection) -> libc::c_int {
    let mut result = libc::PROT_NONE;
    if protection.contains(MapProtection::READ) {
        result |= libc::PROT_READ;
    }
    if protection.contains(MapProtection::WRITE) {
        result |= libc::PROT_WRITE;
    }
    if protection.contains(MapProtection::EXECUTE) {
        result |= libc::PROT_EXEC;
    }
    result
}

#[cfg(unix)]
fn unix_flags(flags: MapFlags, anonymous: bool) -> libc::c_int {
    let mut result = if flags.contains(MapFlags::SHARED) {
        libc::MAP_SHARED
    } else {
        libc::MAP_PRIVATE
    };
    if anonymous {
        result |= libc::MAP_ANON;
    }
    if flags.contains(MapFlags::FIXED) {
        result |= libc::MAP_FIXED;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if flags.contains(MapFlags::POPULATE) {
            result |= libc::MAP_POPULATE;
        }
        if flags.contains(MapFlags::NO_RESERVE) {
            result |= libc::MAP_NORESERVE;
        }
        if flags.contains(MapFlags::STACK) {
            result |= libc::MAP_STACK;
        }
        if flags.contains(MapFlags::LOCKED) {
            result |= libc::MAP_LOCKED;
        }
    }
    result
}

#[cfg(unix)]
fn map_anonymous_platform(
    requested_address: Option<usize>,
    len: usize,
    protection: MapProtection,
    flags: MapFlags,
) -> io::Result<HostMemoryMap> {
    let requested = requested_address.unwrap_or(0) as *mut libc::c_void;
    let result = unsafe {
        libc::mmap(
            requested,
            len,
            unix_protection(protection),
            unix_flags(flags, true),
            -1,
            0,
        )
    };
    if result == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    let ptr = NonNull::new(result.cast()).ok_or_else(|| io::Error::other("mmap returned null"))?;
    Ok(HostMemoryMap {
        ptr,
        len,
        protection,
        flags: flags.union(MapFlags::ANONYMOUS),
        source: MapSource::Anonymous,
        backend: MapBackend::Unix,
    })
}

#[cfg(unix)]
fn map_file_platform(
    requested_address: Option<usize>,
    path: &Path,
    offset: u64,
    len: usize,
    protection: MapProtection,
    flags: MapFlags,
) -> io::Result<HostMemoryMap> {
    use std::os::fd::AsRawFd;

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 || offset % page_size as u64 != 0 {
        return Err(invalid_input("file mapping offset must be page aligned"));
    }
    let file = open_mapping_file(path, protection)?;
    let required_len = offset
        .checked_add(len as u64)
        .ok_or_else(|| invalid_input("file mapping length overflow"))?;
    if protection.is_writable() && file.metadata()?.len() < required_len {
        file.set_len(required_len)?;
    }
    let requested = requested_address.unwrap_or(0) as *mut libc::c_void;
    let result = unsafe {
        libc::mmap(
            requested,
            len,
            unix_protection(protection),
            unix_flags(flags, false),
            file.as_raw_fd(),
            offset as libc::off_t,
        )
    };
    if result == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    let ptr = NonNull::new(result.cast()).ok_or_else(|| io::Error::other("mmap returned null"))?;
    Ok(HostMemoryMap {
        ptr,
        len,
        protection,
        flags,
        source: MapSource::File {
            path: path.to_path_buf(),
            offset,
        },
        backend: MapBackend::Unix,
    })
}

#[cfg(unix)]
fn protect_platform(ptr: NonNull<u8>, len: usize, protection: MapProtection) -> io::Result<()> {
    let rc = unsafe { libc::mprotect(ptr.as_ptr().cast(), len, unix_protection(protection)) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn sync_platform(
    ptr: NonNull<u8>,
    len: usize,
    mode: MapSync,
    _backend: MapBackend,
) -> io::Result<()> {
    let flags = match mode {
        MapSync::Async => libc::MS_ASYNC,
        MapSync::Sync => libc::MS_SYNC,
        MapSync::Invalidate => libc::MS_SYNC | libc::MS_INVALIDATE,
    };
    let rc = unsafe { libc::msync(ptr.as_ptr().cast(), len, flags) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn advise_platform(
    ptr: NonNull<u8>,
    len: usize,
    advice: MapAdvice,
    _backend: MapBackend,
) -> io::Result<()> {
    let advice = match advice {
        MapAdvice::Normal => libc::MADV_NORMAL,
        MapAdvice::Random => libc::MADV_RANDOM,
        MapAdvice::Sequential => libc::MADV_SEQUENTIAL,
        MapAdvice::WillNeed => libc::MADV_WILLNEED,
        MapAdvice::DontNeed => libc::MADV_DONTNEED,
    };
    let rc = unsafe { libc::madvise(ptr.as_ptr().cast(), len, advice) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn lock_platform(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
    let rc = unsafe { libc::mlock(ptr.as_ptr().cast(), len) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn unlock_platform(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
    let rc = unsafe { libc::munlock(ptr.as_ptr().cast(), len) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn unmap_platform(ptr: NonNull<u8>, len: usize, _backend: MapBackend) {
    unsafe {
        libc::munmap(ptr.as_ptr().cast(), len);
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    type Handle = *mut c_void;
    const INVALID_HANDLE_VALUE: Handle = -1_isize as Handle;
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const MEM_RESET: u32 = 0x80000;
    const PAGE_NOACCESS: u32 = 0x01;
    const PAGE_READONLY: u32 = 0x02;
    const PAGE_READWRITE: u32 = 0x04;
    const PAGE_EXECUTE: u32 = 0x10;
    const PAGE_EXECUTE_READ: u32 = 0x20;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const FILE_MAP_WRITE: u32 = 0x2;
    const FILE_MAP_READ: u32 = 0x4;
    const FILE_MAP_EXECUTE: u32 = 0x20;

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualAlloc(
            address: *mut c_void,
            size: usize,
            allocation: u32,
            protect: u32,
        ) -> *mut c_void;
        fn VirtualFree(address: *mut c_void, size: usize, free_type: u32) -> i32;
        fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
        fn VirtualLock(address: *mut c_void, size: usize) -> i32;
        fn VirtualUnlock(address: *mut c_void, size: usize) -> i32;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *mut c_void,
            creation: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
        fn CreateFileMappingW(
            file: Handle,
            security: *mut c_void,
            protect: u32,
            size_high: u32,
            size_low: u32,
            name: *const u16,
        ) -> Handle;
        fn MapViewOfFileEx(
            mapping: Handle,
            access: u32,
            offset_high: u32,
            offset_low: u32,
            bytes: usize,
            base: *mut c_void,
        ) -> *mut c_void;
        fn UnmapViewOfFile(address: *const c_void) -> i32;
        fn FlushViewOfFile(address: *const c_void, bytes: usize) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    fn page_protection(protection: MapProtection) -> u32 {
        let read = protection.contains(MapProtection::READ);
        let write = protection.contains(MapProtection::WRITE);
        let execute = protection.contains(MapProtection::EXECUTE);
        match (read, write, execute) {
            (false, false, false) => PAGE_NOACCESS,
            (false, false, true) => PAGE_EXECUTE,
            (true, false, false) => PAGE_READONLY,
            (true, false, true) => PAGE_EXECUTE_READ,
            (_, true, false) => PAGE_READWRITE,
            (_, true, true) => PAGE_EXECUTE_READWRITE,
        }
    }

    pub(super) fn map_anonymous(
        requested_address: Option<usize>,
        len: usize,
        protection: MapProtection,
        flags: MapFlags,
    ) -> io::Result<HostMemoryMap> {
        let result = unsafe {
            VirtualAlloc(
                requested_address.unwrap_or(0) as *mut c_void,
                len,
                MEM_RESERVE | MEM_COMMIT,
                page_protection(protection),
            )
        };
        let ptr = NonNull::new(result.cast()).ok_or_else(io::Error::last_os_error)?;
        Ok(HostMemoryMap {
            ptr,
            len,
            protection,
            flags: flags.union(MapFlags::ANONYMOUS),
            source: MapSource::Anonymous,
            backend: MapBackend::WindowsVirtual,
        })
    }

    pub(super) fn map_file(
        requested_address: Option<usize>,
        path: &Path,
        offset: u64,
        len: usize,
        protection: MapProtection,
        flags: MapFlags,
    ) -> io::Result<HostMemoryMap> {
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let access = GENERIC_READ
            | if protection.is_writable() {
                GENERIC_WRITE
            } else {
                0
            };
        let file = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if file == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let total = offset
            .checked_add(len as u64)
            .ok_or_else(|| invalid_input("file mapping length overflow"))?;
        let mapping = unsafe {
            CreateFileMappingW(
                file,
                std::ptr::null_mut(),
                page_protection(protection),
                (total >> 32) as u32,
                total as u32,
                std::ptr::null(),
            )
        };
        unsafe {
            CloseHandle(file);
        }
        if mapping.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut view_access = FILE_MAP_READ;
        if protection.is_writable() {
            view_access |= FILE_MAP_WRITE;
        }
        if protection.contains(MapProtection::EXECUTE) {
            view_access |= FILE_MAP_EXECUTE;
        }
        let result = unsafe {
            MapViewOfFileEx(
                mapping,
                view_access,
                (offset >> 32) as u32,
                offset as u32,
                len,
                requested_address.unwrap_or(0) as *mut c_void,
            )
        };
        unsafe {
            CloseHandle(mapping);
        }
        let ptr = NonNull::new(result.cast()).ok_or_else(io::Error::last_os_error)?;
        Ok(HostMemoryMap {
            ptr,
            len,
            protection,
            flags,
            source: MapSource::File {
                path: path.to_path_buf(),
                offset,
            },
            backend: MapBackend::WindowsFileView,
        })
    }

    pub(super) fn protect(
        ptr: NonNull<u8>,
        len: usize,
        protection: MapProtection,
    ) -> io::Result<()> {
        let mut old = 0_u32;
        let ok = unsafe {
            VirtualProtect(
                ptr.as_ptr().cast(),
                len,
                page_protection(protection),
                &mut old,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn sync(ptr: NonNull<u8>, len: usize, backend: MapBackend) -> io::Result<()> {
        if backend == MapBackend::WindowsFileView {
            let ok = unsafe { FlushViewOfFile(ptr.as_ptr().cast(), len) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub(super) fn advise(
        ptr: NonNull<u8>,
        len: usize,
        advice: MapAdvice,
        backend: MapBackend,
    ) -> io::Result<()> {
        if advice == MapAdvice::DontNeed && backend == MapBackend::WindowsVirtual {
            let result =
                unsafe { VirtualAlloc(ptr.as_ptr().cast(), len, MEM_RESET, PAGE_READWRITE) };
            if result.is_null() {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub(super) fn lock(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
        if unsafe { VirtualLock(ptr.as_ptr().cast(), len) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn unlock(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
        if unsafe { VirtualUnlock(ptr.as_ptr().cast(), len) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn unmap(ptr: NonNull<u8>, backend: MapBackend) {
        unsafe {
            match backend {
                MapBackend::WindowsVirtual => {
                    VirtualFree(ptr.as_ptr().cast(), 0, MEM_RELEASE);
                }
                MapBackend::WindowsFileView => {
                    UnmapViewOfFile(ptr.as_ptr().cast());
                }
                MapBackend::Unix => {}
            }
        }
    }
}

#[cfg(windows)]
fn map_anonymous_platform(
    requested_address: Option<usize>,
    len: usize,
    protection: MapProtection,
    flags: MapFlags,
) -> io::Result<HostMemoryMap> {
    windows::map_anonymous(requested_address, len, protection, flags)
}

#[cfg(windows)]
fn map_file_platform(
    requested_address: Option<usize>,
    path: &Path,
    offset: u64,
    len: usize,
    protection: MapProtection,
    flags: MapFlags,
) -> io::Result<HostMemoryMap> {
    windows::map_file(requested_address, path, offset, len, protection, flags)
}

#[cfg(windows)]
fn protect_platform(ptr: NonNull<u8>, len: usize, protection: MapProtection) -> io::Result<()> {
    windows::protect(ptr, len, protection)
}

#[cfg(windows)]
fn sync_platform(
    ptr: NonNull<u8>,
    len: usize,
    _mode: MapSync,
    backend: MapBackend,
) -> io::Result<()> {
    windows::sync(ptr, len, backend)
}

#[cfg(windows)]
fn advise_platform(
    ptr: NonNull<u8>,
    len: usize,
    advice: MapAdvice,
    backend: MapBackend,
) -> io::Result<()> {
    windows::advise(ptr, len, advice, backend)
}

#[cfg(windows)]
fn lock_platform(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
    windows::lock(ptr, len)
}

#[cfg(windows)]
fn unlock_platform(ptr: NonNull<u8>, len: usize) -> io::Result<()> {
    windows::unlock(ptr, len)
}

#[cfg(windows)]
fn unmap_platform(ptr: NonNull<u8>, _len: usize, backend: MapBackend) {
    windows::unmap(ptr, backend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

    fn rw() -> MapProtection {
        MapProtection::READ.union(MapProtection::WRITE)
    }

    #[test]
    fn anonymous_mapping_reads_writes_and_remaps_without_losing_prefix() {
        let mut mapping =
            HostMemoryMap::anonymous(4096, rw(), MapFlags::PRIVATE).expect("anonymous mapping");
        mapping.write(0, &[1, 2, 3, 4]).expect("mapped write");
        let mut bytes = [0_u8; 4];
        mapping.read(0, &mut bytes).expect("mapped read");
        assert_eq!(bytes, [1, 2, 3, 4]);

        mapping.remap(8192, true, None).expect("mapped remap");
        let mut bytes = [0_u8; 4];
        mapping.read(0, &mut bytes).expect("read remapped prefix");
        assert_eq!(bytes, [1, 2, 3, 4]);
        assert_eq!(mapping.len(), 8192);
    }

    #[test]
    fn protection_transitions_are_observable() {
        let mut mapping =
            HostMemoryMap::anonymous(4096, rw(), MapFlags::PRIVATE).expect("anonymous mapping");
        mapping.write(0, &[9]).expect("initial write");
        mapping
            .protect(MapProtection::READ)
            .expect("protect read-only");
        assert!(mapping.write(0, &[8]).is_err());
        mapping.protect(rw()).expect("restore read-write");
        mapping.write(0, &[8]).expect("write after restore");
    }

    #[test]
    fn shared_file_mapping_syncs_to_file() {
        let path = std::env::temp_dir().join(format!(
            "ncc-runtime-map-{}-{}.bin",
            std::process::id(),
            TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = File::create(&path).expect("create map file");
        file.set_len(4096).expect("size map file");
        file.seek(SeekFrom::Start(0)).expect("seek map file");
        file.write_all(&[0_u8; 4]).expect("seed map file");
        drop(file);

        {
            let mut mapping = HostMemoryMap::file(&path, 0, 4096, rw(), MapFlags::SHARED)
                .expect("shared file mapping");
            mapping.write(0, &[4, 3, 2, 1]).expect("mapped file write");
            mapping.sync(MapSync::Sync).expect("mapped file sync");
        }

        let mut file = File::open(&path).expect("open synced map file");
        let mut bytes = [0_u8; 4];
        file.read_exact(&mut bytes).expect("read synced bytes");
        assert_eq!(bytes, [4, 3, 2, 1]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_mapping_contracts_are_rejected() {
        assert!(HostMemoryMap::anonymous(0, rw(), MapFlags::PRIVATE).is_err());
        assert!(
            HostMemoryMap::anonymous(4096, rw(), MapFlags::PRIVATE.union(MapFlags::SHARED),)
                .is_err()
        );
    }
}
