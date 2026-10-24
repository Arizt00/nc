# Manual raw heap and virtual memory

## Canonical rule

Heap and virtual-memory operations are native prelude names. They need no
`use` declaration. Every call is explicit and direct. The compiler does not
store ownership, mapping provenance, allocation size, lifetime, bounds,
NUMA policy or allocator class.

Null, invalid alignment, stale addresses, overlap, double release and races
remain programmer-visible native behavior. Failure returns `null`, `false`,
`-1` or the platform result. `vmLastError()` exposes `GetLastError()` or
`errno`; it does not create an nC exception.

## Direct surface

```text
alignedAlloc(bytes, alignment) -> p<byte> | null
alignedRealloc(address, oldBytes, newBytes, alignment) -> p<byte> | null
alignedFree(address) -> bool

vmPageSize() -> int
vmAllocationGranularity() -> int
vmReserve(address, bytes, nativeFlags) -> p<byte> | null
vmCommit(address, bytes, nativeProtection) -> bool
vmDecommit(address, bytes) -> bool
vmRelease(address, bytes) -> bool
vmProtect(address, bytes, nativeProtection) -> bool
vmGuard(address, bytes, nativeProtection) -> bool
vmMap(address, bytes, nativeProtection, nativeFlags, handle, offset) -> p<byte> | null
vmUnmap(address, bytes) -> bool
vmRemap(address, oldBytes, newBytes, nativeProtection, nativeFlags, requested) -> p<byte> | null
vmAdvise(address, bytes, nativeAdvice) -> bool
vmSync(address, bytes, nativeFlags) -> bool
vmLock(address, bytes) -> bool
vmUnlock(address, bytes) -> bool
vmExecutable(address, bytes) -> bool
vmNumaAlloc(bytes, node, nativeProtection, nativeFlags) -> p<byte> | null
vmNumaBind(address, bytes, mode, nodeMask, maxNode, nativeFlags) -> bool
vmLastError() -> int

vmSharedCreate(name, bytes, nativeFlags) -> int
vmSharedOpen(name, nativeFlags) -> int
vmSharedMap(handle, address, bytes, nativeAccess, nativeFlags, offset) -> p<byte> | null
vmSharedClose(handle) -> bool
vmSharedUnlink(name) -> bool
```

Types and arity are checked because they are part of the machine ABI. Pointer
validity, range, provenance and lifetime are not checked.

## Reallocation

`alignedRealloc` has one contract on all hosted targets:

- `oldBytes` is explicit;
- success preserves `min(oldBytes, newBytes)` bytes;
- failure leaves the old allocation live;
- `newBytes == 0` releases the old allocation and returns `null`;
- alignment is explicit;
- the compiler adds no size header or registry.

Windows lowers to `_aligned_realloc`. POSIX allocates with `posix_memalign`,
copies and frees only after successful allocation.

`vmRemap` lowers to `mremap` on Linux. Windows and macOS allocate, copy
`min(oldBytes, newBytes)`, release the old mapping only after success and roll
back the new mapping if release fails. Native flags remain caller supplied.

## Native constants

The same numeric value is not translated between operating systems.

| Meaning | Windows | Linux | macOS |
| --- | ---: | ---: | ---: |
| read | `0x02` | `1` | `1` |
| read/write | `0x04` | `3` | `3` |
| read/execute | `0x20` | `5` | `5` |
| read/write/execute | `0x40` | `7` | `7` |
| guard modifier | `0x100` | `PROT_NONE` mapping | `PROT_NONE` mapping |
| shared | `FILE_MAP_*` | `MAP_SHARED=1` | `MAP_SHARED=1` |
| copy on write | `FILE_MAP_COPY=1` | `MAP_PRIVATE=2` | `MAP_PRIVATE=2` |
| anonymous | VirtualAlloc | `MAP_ANONYMOUS=0x20` | `MAP_ANON=0x1000` |
| fixed | requested VirtualAlloc address | `MAP_FIXED=0x10` | `MAP_FIXED=0x10` |
| huge pages | `MEM_LARGE_PAGES=0x20000000` | `MAP_HUGETLB=0x40000` | no generic mmap flag |

Windows `vmGuard` commits the selected range with `PAGE_GUARD`. POSIX guard
pages are normal mappings protected with `PROT_NONE`.

## Hosted implementations

Windows uses `VirtualAlloc`, `VirtualFree`, `VirtualProtect`,
`DiscardVirtualMemory`, `VirtualLock`, `CreateFileMappingA`,
`MapViewOfFileEx`, `UnmapViewOfFile` and `VirtualAllocExNuma`.

Linux uses `mmap`, `mprotect`, `madvise`, `munmap`, `mremap`, `mlock`,
`msync`, `shm_open` and the `mbind` syscall.

macOS uses `mmap`, `mprotect`, `madvise`, `munmap`, `mlock`, `msync` and
POSIX shared-memory objects. Public macOS APIs expose no general NUMA binding;
node zero is the UMA lane and nonzero NUMA requests fail.

`vmExecutable` always performs RW to RX through the OS and flushes the
instruction cache. A programmer can still request RWX explicitly through
`vmProtect`; the controlled helper does not do so.

## Allocator policy

Arena, pool, slab, buddy and per-CPU heaps are source-level policies. The
programmer supplies backing, metadata, alignment, size classes, OOM behavior,
locking and CPU selection. The compiler never chooses one.

The executable reference is
`examples/06_memory/manual_allocator_policies.n`. It uses one explicit
aligned allocation and implements all policy through pointer arithmetic and
metadata writes.
