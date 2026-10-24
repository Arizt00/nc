# nC raw API v1

`nC.raw.api.v1` is the canonical manual memory surface. Compact spellings are
normalized by the parser; AST, sema, IR, LLVM and machine retain only canonical
operations. None of these operations adds ownership, bounds, lifetime or
provenance.

## Pointers

| Compact | Canonical | Meaning |
|---|---|---|
| `p<T>` | `ptr<T>` | Raw address typed as `T` |
| `rd<T>(p)` | `ptrRead<T>(p)` | Load from `p` |
| `wr<T>(p, v)` | `ptrWrite<T>(p, v)` | Store `v` at `p` |
| `off<T>(p, n)` | `ptrOffset<T>(p, n)` | Address `p + n * sizeOf<T>()` |
| `dist(a, b)` | `ptrDiff(a, b)` | Raw pointer difference |
| `adr(p)` | `ptrAddr(p)` | Pointer to integer address |
| `from<T>(a)` | `ptrFromAddr<T>(a)` | Integer address to pointer |
| `cast<T>(p)` | `ptrCast<T>(p)` | Raw pointer cast |
| `nil(p)` | `ptrIsNull(p)` | Compare with null |

Dereferencing null, misaligned or stale addresses and accessing outside an
object remain undefined behavior. Optional auditors do not change generated
code.

## Material operations

`alloc`, `calloc`, `realloc` and `free` expose byte counts and addresses
directly; allocation failure is not repaired. `cat(dst, cap, a, b)` writes a
concatenation into caller storage. `cp`, `mv`, `fill` and `cmp` map to
`memcpy`, `memmove`, `memset` and `memcmp`. Dynamic
collections bind caller storage with `listAt<T>`, `queueAt<T>`, `vectorAt<T>`
or `dinAt<T>`. `push/add` never grows that storage.

Closures use `closureAt(storage, closure)`. Calls returning callable material
use `callAt(storage, call)`. Async calls use `asyncAt(storage, call)`. The
compiler does not allocate descriptor, environment or coroutine frames.

## Virtual and device memory

`map`, `protect`, `unmap` and `exec` normalize to the VM backend. `mrdN` and
`mwrN` cover 8, 16, 32, 64, 128, 256 and 512-bit MMIO. DMA aliases expose pin,
unpin, scatter/gather, bounce, cache maintenance and fences. VM, MMIO and DMA
arguments are never repaired or validated implicitly.
