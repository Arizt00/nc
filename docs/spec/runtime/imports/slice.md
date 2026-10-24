# Runtime import: `memory:slice`

```n
use <memory:slice>;
```

`slice` is a 24-byte caller-owned raw view:
`{ data: addr, bytes: u64, flags: u64 }`.

| Operation | Contract |
|---|---|
| `sliceStride()` | `() -> int` |
| `sliceAt(out, data, bytes, flags)` | `(addr, addr, int, int) -> bool` |
| `sliceData/Len/Flags(value)` | `(addr) -> addr/int` |
| `sliceRead8(value, offset)` | `(addr, int) -> int` |
| `sliceWrite8(value, offset, scalar)` | `(addr, int, int) -> bool` |
| `sliceView(out, source, offset, bytes)` | `(addr, addr, int, int) -> bool` |
| `sliceIsEmpty(value)` | `(addr) -> bool` |
| `sliceClear(value)` | `(addr) -> bool` |

No operation owns or checks the backing range. The caller supplies storage,
keeps it live and decides every offset. `sliceClear` clears only the descriptor.
LLVM/Machine materialize `__ncc_slice_*` without C, allocation or hidden
lifetime tracking. See `examples/runtime/families/memory_slice_basic.n`.
