# Runtime import: `nC.runtime.flush`

## Import

```n
use <io:flush>;
```

## Surface

| Function | Result | Meaning |
|---|---|---|
| `flush()` | `bool` | Synchronize all direct output before continuing. |
| `flushStdout()` | `bool` | Synchronize stdout ordering. |
| `flushStderr()` | `bool` | Synchronize stderr ordering. |
| `autoFlushOnInput(bool)` | `bool` | Set the explicit pre-input policy. |
| `flushBeforeInput()` | `bool` | Apply that policy before a read. |

nC output reaches the OS through direct, unbuffered writes. Flush therefore
materializes as a sequentially-consistent compiler/CPU ordering point; it does
not call `fflush` or create a hidden userspace buffer. The policy occupies one
atomic byte and is updated with `seq_cst` ordering.

## Materialization

The complete family is emitted into the target LLVM/MCT object. Programs that
only use flush do not import console or file-writing APIs and do not link the C
hosted provider.

## Evidence

- `examples/runtime/families/io_flush_basic.n`
- `crates/ncc-ir/src/stdio_llvm.rs`
- `docs/status/RESOLUTION_IMPORT_STD_ERROR_FLUSH_NO_C_2026-08-09.md`
