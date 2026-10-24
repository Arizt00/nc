# Filesystem backend surface

## Authority

`nC.runtime.fs` is a hosted backend prelude. Source code calls its functions
directly; no `use` declaration is required. `print` and `println` remain part of
`io:output` and still require their explicit import.

The implementation is emitted as LLVM/MCT bodies and direct operating-system
ABI calls. It does not compile or link `runtime_hosted_native.c`.

## Manual raw API

| Function | Contract | Result |
|---|---|---|
| `fsWriteFrom` | `(path, data, length, append)` | bytes written or negative OS-stage error |
| `fsReadInto` | `(path, out, capacity)` | bytes read or negative OS-stage error |
| `fsSize` | `(path)` | byte length or negative error |
| `fsCopy` | `(source, destination)` | copied byte count or negative stage error |
| `listDirInto` | `(path, out, capacity)` | required/written newline-delimited bytes |
| `absolutePathInto` | `(path, out, capacity)` | absolute spelling length or negative error |
| `officialPathInto` | `(path, out, capacity)` | OS-canonical spelling length or negative error |
| `fsMetadataInto` | `(path, out, capacity)` | stable metadata text length or negative error |
| `fsPermissionsInto` | `(path, out, capacity)` | stable permission text length or negative error |
| `fsSetReadonly` | `(path, readonly)` | mutates the OS permission bits/attribute |
| `fsLock` | `(path, exclusive, wait)` | raw OS lock handle or null address |
| `fsUnlock` | `(handle)` | unlocks and closes the raw handle |
| `fsRelease` | `(address)` | releases an owned compatibility allocation |
| `fsTextRelease` | `(taggedText)` | decodes and releases a successful `Result<str, FsError>` |
| `fsBytesRelease` | `(taggedBytes)` | releases `readBytes` data and its array descriptor |
| `fsListRelease` | `(taggedList)` | releases a `listDir` result and its strings |
| `fsLinesRelease` | `(arrayAddress)` | releases every `fread_lines` item |

Buffers, capacities and lifetimes belong to the caller. The backend performs no
bounds, ownership, lifetime or provenance checks beyond what the operating
system itself requires.

## Material compatibility API

This wider catalog describes the Stage0 adapter surface. It is not a claim
that every entry is present in a self-built native runtime publication.
The pinned `bootstrap/runtime-native-units-v79.json` and publication templates
identify the actual native bodies and exports.

The provider also materializes `readFile`, `writeFile`, `appendFile`, `fsread`,
`fswrite`, `fsappend`, `fsync`, `exists`, `fexists`, `deleteFile`,
`delete_file`, `renamePath`, `movePath`, `rename_file`, `createDir`,
`removeDir`, `listDir`, `currentDir`, `changeDir`, `fread_lines`,
`absolutePath`, `officialPath`, `lockFile`, `unlockFile`, `copyFile`,
`readBytes`, `metadata`, `permissions`, `setReadonly`, `fswriteResult`,
`readBytesInto` and `writeBytes`.

Owned string results may be released as a tagged value with `fsTextRelease` or
as an already extracted address with `fsRelease`. Lists and line arrays use
their matching explicit release operation. Compatibility wrappers do not
introduce a garbage collector or hidden lifetime tracking.

## Target ABI

### Native nC publication V79

`absolutePathInto` is implemented by `runtime/nc/fs_path_provider.n` and the
selected `fs_path_host_{windows,linux}.n`. Its ABI is `(str,p<byte>,u64)->int`.
It does not require the destination to exist. Windows uses strict UTF-8/UTF-16
conversion and `GetFullPathNameW`; Linux prefixes a relative spelling with
`getcwd`, retaining dot components and repeated separators in that spelling.
Only `officialPathInto` resolves existing paths and symbolic links.

The result excludes the terminating NUL; capacity includes it. Null/empty
input is `-1`, null/insufficient output is `-2`, OS failure is `-3`, and
allocation/representation failure is `-4`. Windows rejects malformed UTF-8;
Linux preserves non-NUL pathname bytes. Failed calls leave output unchanged;
input and output may overlap. This is a buffer contract, not pointer ownership
or provenance enforcement. The caller still supplies valid backing storage.

Temporary storage is allocated and released explicitly in the provider source.
The verifier grows its output buffer instead of imposing a fixed 32 KiB limit.
Relative paths observe process cwd; applications must synchronize cwd changes
or resolve once and keep the resulting absolute spelling. Native execution
certification is scoped to Windows/Ubuntu x86_64, not every historical target.

### Historical Stage0 LLVM adapters

- Windows x86_64/AArch64: direct Win32 file and directory calls.
- Windows path lowering uses `GetFullPathNameA` and a real file/directory
  handle with `GetFinalPathNameByHandleA`.
- Windows locking uses `LockFileEx`/`UnlockFileEx`; POSIX locking uses `flock`.
- Windows metadata and readonly mutation use `GetFileAttributesExA` and
  `SetFileAttributesA`; POSIX targets use their target-native `stat`/`stat64`
  layout and `chmod`.
- Metadata text is `type=...;length=...;readonly=...;modified=...` and
  permission text is `readonly=...;mode=...`.
- Linux/Android x86_64/AArch64: direct POSIX calls.
- Linux i386/i686/ARMv7/RV32: ILP32 adapters for `read`, `write`, `getcwd` and
  64-bit file offsets.
- macOS x86_64/AArch64: direct POSIX/Darwin calls.
- WASI: rejected until a WASI filesystem capability provider is selected.

## Evidence

- `examples/runtime/families/filesystem_raw_manual.n`
- `examples/runtime/families/filesystem_paths_raw_manual.n`
- `examples/runtime/families/filesystem_lock_raw_manual.n`
- `examples/runtime/families/filesystem_text_compat_release.n`
- `examples/runtime/families/filesystem_copy_bytes_release.n`
- `examples/runtime/families/filesystem_metadata_permissions_manual.n`
- `crates/ncc-ir/src/filesystem_llvm.rs`
- `docs/status/RESOLUTION_IMPORT_FILESYSTEM_NO_C_2026-08-09.md`

The Windows fixture executes as a PE64 program. Linux x86_64, Linux i386 and
Darwin produce target-native objects on the Windows development host; final
foreign-host execution requires the corresponding sysroot/runner.
