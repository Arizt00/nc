# Runtime import: `memory:span`

```n
use <memory:span>;
```

A span descriptor is 32 bytes:
`{ segments: addr, count: u64, bytes: u64, flags: u64 }`. Each caller-owned
segment is 16 bytes: `{ data: addr, bytes: u64 }`.

| Operation | Contract |
|---|---|
| `spanStride()` / `spanSegmentStride()` | `() -> int` |
| `spanSegmentAt(table, index, data, bytes)` | `(addr, int, addr, int) -> bool` |
| `spanAt(out, segments, count, bytes, flags)` | `(addr, addr, int, int, int) -> bool` |
| `spanSegments/Count/Len/Flags(value)` | `(addr) -> addr/int` |
| `spanRead8(value, offset)` | `(addr, int) -> int` |
| `spanContains8(value, scalar)` | `(addr, int) -> bool` |
| `spanIsEmpty(value)` | `(addr) -> bool` |
| `spanClear(value)` | `(addr) -> bool` |

The backend traverses exactly the table supplied by the program. It does not
allocate, join, retain, validate or release segments. Invalid tables and
out-of-range reads are raw behavior. LLVM/Machine materialize `__ncc_span_*`
without a C provider. See `examples/runtime/families/memory_span_basic.n`.
