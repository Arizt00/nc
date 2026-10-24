# Runtime import: `nC.runtime.std`

## Import

```n
use <io:std>;
```

The import remains explicit. It exposes the process standard streams as raw
native handles; it does not allocate, own or close them.

## Surface

| Function | Result | Meaning |
|---|---|---|
| `stdIn()` | `addr` | Native stdin handle or descriptor. |
| `stdOut()` | `addr` | Native stdout handle or descriptor. |
| `stdErr()` | `addr` | Native stderr handle or descriptor. |
| `isInteractive()` | `bool` | True when stdin or stdout is attached to a terminal. |

Windows returns the values from `GetStdHandle` and probes the console with
`GetConsoleMode`. POSIX targets return descriptors `0`, `1` and `2` and probe
with `isatty`. WASI returns the standard descriptor numbers and reports no TTY
because preview1 has no portable terminal query.

## Materialization

The functions are defined inside the target LLVM/MCT object by
`stdio_llvm.rs`. No `runtime_hosted_native.c` export or hosted runtime object is
used. Machine sees the stable symbols `ncc_rt_std_*`; the final linker retains
only the operating-system imports required by the selected target.

## Evidence

- `examples/runtime/families/io_std_basic.n`
- `crates/ncc-ir/src/stdio_llvm.rs`
- `docs/status/RESOLUTION_IMPORT_STD_ERROR_FLUSH_NO_C_2026-08-09.md`
