# Compiler FFI Real Linker

This document closes the compiler FFI linker lane for real external artifacts.
The lane is intentionally narrow: it consumes already declared `foreign:`
contracts and links real host artifacts during `ncc run --native-ffi`. It does
not invent symbols, synthesize replacement libraries, or bypass the contract
matrix.

## Contract Fields

The linker recognizes these contract fields on a `foreign:` symbol:

- `artifact`, `artifactPath`, `libraryPath`, `linkArtifact`: authoritative
  artifact path. The file must exist.
- `library`, `lib`, `libs`: library name or path. A path-like value is linked
  directly; a plain value becomes `-l<name>`.
- `linkPath`, `linkDir`, `libraryDir`, `librarySearchPath`, `searchPath`:
  directory added as `-L<dir>`.
- `rpath`, `runtimePath`, `runtimeLibraryPath`, `runtimeDir`: runtime search
  directory for shared artifacts.
- `linkage`: `static`, `dynamic`, or `shared`.
- `checksum`, `cksum`, `sha256`, `artifactChecksum`, `artifactSha256`:
  optional SHA-256 contract. Values may be raw hex or `sha256:<hex>`.

## Static And Shared Artifacts

Static artifacts (`.a`, `.lib`, or `linkage: static`) are linked directly when a
path is declared. Shared artifacts (`.so`, `.dylib`, `.dll`, or
`linkage: dynamic/shared`) are linked directly and their parent directory is
added to the runtime loader path. On Linux the host linker also receives
`-Wl,-rpath,<dir>` and the process receives `LD_LIBRARY_PATH` for the same
directories.

The nC backend currently emits non-PIE host objects for this lane, so Linux
native FFI autolink passes `-no-pie` to the host linker. This is part of the
native executable contract, not a semantic change to FFI.

## Diagnostics

- `E_FFI_LINK_001`: missing artifact, missing link directory, sidecar compile
  failure, or host linker failure.
- `E_FFI_CHECKSUM_001`: invalid checksum contract or artifact checksum mismatch.
- `E_FFI_CALL_001`: linked executable could not start or returned failure.

## Evidence

The integration test
`ffi_native_autolink_consumes_declared_static_and_shared_artifacts` builds:

- a real static archive with `ar`;
- a real shared object with `cc -shared`;
- SHA-256 hashes for both artifacts;
- a temporary nC program that declares both artifacts in `foreign:`;
- a successful `ncc run --native-ffi` execution;
- a negative checksum fixture that fails with `E_FFI_CHECKSUM_001`.

The older sidecar and wrapper linker lanes remain covered by
`ffi_real_link_run_user_program_end_to_end` and
`ffi_real_link_run_native_autolink_without_wrapper`.
