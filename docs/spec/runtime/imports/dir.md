# Directory backend surface

## Import

Directory convenience operations are selected explicitly:

```n
use <fs:dir>;
```

This keeps `currentDir` and `changeDir` out of ordinary identifier resolution,
where broad names would otherwise be ambiguous. The lower-level filesystem
prelude remains available without an import.

## Operations

| Function | Contract |
|---|---|
| `currentDirInto(out, capacity)` | writes the current path into caller storage |
| `listDir(path)` | returns an owned tagged list of owned strings |
| `createDir(path)` | creates one directory |
| `removeDir(path)` | removes one empty directory |
| `currentDir()` | returns an owned tagged string |
| `changeDir(path)` | changes process current directory |

`fsRelease` releases the `currentDir` payload and `fsListRelease` releases the
entire `listDir` result. No sandbox, traversal filter, automatic cleanup or
implicit policy is inserted by the compiler.

## Lowering

All operations become typed runtime call edges, LLVM/MCT function bodies and
direct Win32 or POSIX ABI calls. No filesystem symbol is supplied by the hosted
C provider.

## Evidence

- `examples/runtime/families/dir_basic.n`
- `examples/runtime/families/filesystem_raw_manual.n`
- `crates/ncc-ir/src/filesystem_llvm.rs`
- `docs/status/RESOLUTION_IMPORT_FILESYSTEM_NO_C_2026-08-09.md`
