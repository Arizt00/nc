# Compiler FFI Link Manifest

`ncc compiler ffi-link-manifest <file.n> --json` emits the static packaging
manifest for `foreign:` symbols. It is the bridge between the source-level ABI
contract matrix and the real native linker lane.

The command does not link or execute. It reports the exact inputs a later linker
or package manager must consume:

- foreign symbol and container;
- language, ABI, signature, ownership, side-effect and layout contract status;
- declared `header`;
- declared `library` or path-like library;
- declared `artifact`, `artifactPath`, `libraryPath`, or `linkArtifact`;
- declared `linkPath` / `libraryDir` / search path;
- declared `rpath` / runtime library path;
- expected and actual SHA-256 when an artifact exists;
- linker arguments such as direct artifact paths, `-L`, and `-l`;
- runtime paths needed by shared libraries;
- official diagnostics for missing artifacts or checksum mismatch.

## Status Values

- `closed_manifest`: a concrete artifact exists and its checksum contract, if
  present, matches.
- `deferred_to_sidecar_or_system_library`: the source declares a library name
  but no concrete artifact path. This is valid for sidecar resolution and
  system-library lanes.
- `missing_artifact`: a concrete artifact path was declared but does not exist.
- `missing_link_path`: a declared search directory does not exist.
- `checksum_invalid`: the checksum contract is not valid SHA-256 hex.
- `checksum_mismatch`: the artifact exists but bytes do not match the declared
  checksum.
- `failed_contract`: source-level ABI diagnostics exist for the symbol.

## Diagnostics

- `E_FFI_LINK_001`: missing artifact or missing link path.
- `E_FFI_CHECKSUM_001`: invalid or mismatched artifact checksum.
- Any source-level `E_FFI_*` diagnostic already emitted by
  `ffi-contracts`.

## Relationship To Other Commands

- `ffi-contracts` proves the ABI/source contract.
- `ffi-link-manifest` packages linker inputs.
- `run --native-ffi` consumes the declared inputs and links/executes them.
- `ffi-cpp-abi --strict` proves C++ ABI behavior with real toolchains.

This command is intentionally machine-readable so `ghor`, Basal evidence, and
release packaging can consume the same manifest without scraping prose.
