# Runtime FS Priority 3 Contract

Status: superseded_by_material_backend_contract.

La superficie sigue siendo compatible, pero su autoridad de implementacion es
ahora `docs/spec/runtime/imports/fs.md`: las operaciones bajan a LLVM/MCT y ABI
directa del sistema sin compilar ni enlazar `runtime_hosted_native.c`.

This document closes the nC runtime filesystem surface that is exposed through:

```n
use stoud.io { fsread, fsappend, fsync, absolutePath, copyFile, createDir, deleteFile, lockFile, metadata, movePath, officialPath, permissions, readBytes, readBytesInto, removeDir, renamePath, setReadonly, unlockFile, writeBytes };
```

## Real Operations

Todas las funciones usan llamadas reales del sistema durante `ncc run
archivo.n`; no requieren `--real` ni un proveedor escrito en C.

| Function | Return | Real effect |
| --- | --- | --- |
| `fsread(path)` | `Result<str,str>` | opens, reads UTF-8 text and closes |
| `fswrite(path, text)` | `Result<bool,str>` | creates/replaces text file, writes, flushes and closes |
| `fsappend(path, text)` | `Result<bool,str>` | opens in append mode, writes, flushes and closes |
| `fsync(path)` | `Result<bool,str>` | opens file descriptor and calls `sync_all` |
| `exists(path)` | `bool` | checks host path existence |
| `deleteFile(path)` | `Result<bool,str>` | removes a real file |
| `copyFile(src, dst)` | `Result<int,str>` | copies real bytes and returns byte count |
| `createDir(path)` | `Result<bool,str>` | creates directory tree |
| `removeDir(path)` | `Result<bool,str>` | removes directory tree; missing path is cleanup-success |
| `listDir(path)` | `Result<list<str>,FsError>` | returns owned entry names in stable sorted order |
| `metadata(path)` | `Result<str,str>` | returns stable text metadata: type, length, readonly, modified time |
| `permissions(path)` | `Result<str,str>` | returns readonly state |
| `setReadonly(path, flag)` | `Result<bool,str>` | mutates host readonly permission flag |
| `renamePath(src, dst)` / `movePath(src, dst)` | `Result<bool,str>` | renames/moves a real path |
| `absolutePath(path)` | `Result<str,str>` | resolves against current working directory; path need not exist |
| `officialPath(path)` | `Result<str,str>` | officializes an existing real path |
| `lockFile(path)` | `Result<FileLockHandle,str>` | creates an exclusive `.nclock` sidecar and registry handle |
| `unlockFile(handle)` | `Result<bool,str>` | consumes `FileLockHandle` and removes sidecar lock |
| `readBytes(path)` | `Result<array<byte>,str>` | reads raw bytes into runtime byte vector |
| `readBytesInto(path, destination)` | `Result<int,FsError>` | reads into caller-owned physical storage and returns byte count |
| `writeBytes(path, bytes)` | `Result<int,str>` | writes raw byte payload exactly and returns byte count |

## ABI And MCT

Every function in this surface has a row in `ncc runtime abi --family fs --json`.
The import closure table registers the compact imports as `closed_real_backing_bytes`,
and MCT receives the corresponding runtime symbol names, for example
`nC.runtime.fs.readBytesInto`, `nC.runtime.fs.writeBytes` and
`nC.runtime.fs.lockFile`.

`FileLockHandle` is a stable nominal runtime handle:

```text
__runtimeFamily  = runtime.fs.fileLockHandle
__runtimeVariant = handle
value            = int registry_slot
native lowering  = qword slot, size 8, align 8
```

Ownership is linear: `lockFile` creates the handle and `unlockFile` consumes it.

## Error Contract

Fallible functions use a typed `FsError` payload and `E_RUNTIME_FS_001`. This keeps
OS errors recoverable in `.n` code through `isOk`, `isErr`, `match` and `unwrap`.
`listDir` owns both the returned list and every entry string. While its collection
ABI is hosted-runtime-only, plain `ncc run archivo.n` automatically selects that
real runtime route instead of attempting an incomplete native link.

## Validation Fixtures

Positive examples:

- `examples/07_runtime/file_manager_profiler.n`
- `examples/runtime/runtime_real_fs_directories_paths_metadata.n`
- `examples/runtime/runtime_real_fs_binary_lock_move.n`
- `examples/runtime/runtime_real_fs_append_fsync.n`

Negative/recoverable example:

- `examples/runtime/negative/runtime_real_fs_double_lock_result.n`

Tests:

- `cargo test -p ncc-cli runtime_symbol_abi -- --nocapture`
- `cargo test -p ncc-cli --test runtime_process_thread_contract -- --nocapture`
- `cargo test -p ncc-cli --test import_surface_stoud -- --nocapture`
- `cargo test -p ncc-cli --test backend_mct_runtime_symbols -- --nocapture`
