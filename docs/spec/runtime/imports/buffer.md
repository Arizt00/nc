# Runtime import: `memory:buffer`

```n
use <memory:buffer>;
```

`buffer` is a manual raw descriptor over caller-owned bytes. Its 32-byte layout
is `{ data: addr, len: u64, capacity: u64, flags: u64 }`. The backend does not
allocate, release, retain, grow, validate or repair the descriptor.

| Operation | Contract |
|---|---|
| `bufferStride()` | `() -> int` |
| `bufferAt(out, data, len, cap, flags)` | `(addr, addr, int, int, int) -> bool` |
| `bufferData/Len/Capacity/Flags(value)` | `(addr) -> addr/int` |
| `bufferSetLen(value, len)` | `(addr, int) -> bool` |
| `bufferRead8/16/32(value, byteOffset)` | `(addr, int) -> int` |
| `bufferRead64(value, byteOffset)` | `(addr, int) -> u64` |
| `bufferWrite8/16/32(value, byteOffset, scalar)` | `(addr, int, int) -> bool` |
| `bufferWrite64(value, byteOffset, scalar)` | `(addr, int, u64) -> bool` |
| `bufferFill(value, offset, bytes, scalar)` | `(addr, int, int, int) -> bool` |
| `bufferCopy(dst, dstOff, src, srcOff, bytes)` | `(addr, int, addr, int, int) -> bool` |
| `bufferView(out, source, offset, bytes)` | `(addr, addr, int, int) -> bool` |
| `bufferClear(value)` | `(addr) -> bool` |

Offsets are bytes. Out-of-range, null, stale, overlapping and misaligned access
remain raw behavior. `bufferCopy` is overlap-safe; `bufferClear` only zeroes the
descriptor and never frees backing storage.

LLVM materializes `__ncc_buffer_*` bodies directly. There is no
`ncc_rt_buffer_*`, CRT or C provider. See
`examples/runtime/families/memory_buffer_manual_raw.n`.
