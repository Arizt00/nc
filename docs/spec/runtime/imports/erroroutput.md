# Runtime import: `nC.runtime.errorOutput`

## Import

```n
use <io:errorOutput>;
```

The import is explicit and independent from `print`/`println`.

## Surface

| Function | Result | Output |
|---|---|---|
| `eprint(value)` | `void` | Raw value, no newline. |
| `eprintln(value)` | `void` | Value plus newline. |
| `warn(value)` | `void` | `[warn] `, value and newline. |
| `error(value)` | `void` | `[error] `, value and newline. |
| `flushErr()` | `bool` | Synchronize stderr ordering. |

`str`, integer, `bool` and `ft64` values are typed before lowering. Text is
written directly to stderr. Numeric conversion uses the nC provider in
`runtime/nc/text_format.n` and releases its explicit allocation after writing.
Machine transports floating values through `ncc_rt_error_f64_bits` as raw
IEEE-754 bits, avoiding a false integer-register implementation of an FP ABI.

## Target ABI

- Windows: `GetStdHandle(STD_ERROR_HANDLE)` plus `WriteFile`.
- Linux/Android/Darwin: descriptor `2` plus target-width `write`.
- WASI: descriptor `2` plus `fd_write` and a caller-visible result.

No implementation or formatting path remains in
`runtime_hosted_native.c`.

## Evidence

- `examples/runtime/families/error_output_basic.n`
- `examples/runtime/families/error_output_numeric_native.n`
- `crates/ncc-ir/src/stdio_llvm.rs`
- `docs/status/RESOLUTION_IMPORT_STD_ERROR_FLUSH_NO_C_2026-08-09.md`
