# ghor Compiler FFI Manifest Integration

`ghor ffi-manifest --json` connects package orchestration with the compiler FFI
link manifest. It does not parse `foreign:` contracts independently. Instead,
it delegates source-level extraction to `ncc compiler ffi-link-manifest` through
the shared compiler collector and then packages the result beside ghor native
dependency metadata.

The command reports:

- package identity and manifest path;
- scanned nC source files;
- compiler FFI entries declared in `foreign:` sections;
- local native artifacts and SHA-256 checksum status;
- compiler link args and runtime paths;
- `[native-dependencies]` from `ghor.toml`;
- manifest diagnostics from the compiler and ghor.

Default source discovery scans direct root `.n` files and `src/**/*.n`. Passing
`--source <file.n>` inspects one explicit source. Passing `--all-sources` also
scans `examples/**/*.n`; this is intentionally opt-in so packages can keep
negative or teaching examples without breaking normal metadata operations.

The JSON schema is `ghor.ffi_manifest.v1`. A successful source-level FFI package
view has `status = "closed"`. Missing artifacts, checksum mismatches, source
parse failures, and native-dependency manifest errors produce
`E_GHOR_FFI_MANIFEST_001` in the ghor report while preserving the underlying
compiler diagnostic codes, such as `E_FFI_CHECKSUM_001`.

`ghor metadata --json` includes the same report under `ffi_link_manifest`.
`ghor link --json` includes it under `compiler_ffi_link_manifest`.
`ghor package` writes it into package `manifest.json`, and `ghor lock` stores it
in `ghor.lock.meta.json` while keeping the primary `ghor.lock` file in
Cargo.lock-style TOML form.

Registry publishing records the package manifest hash and the FFI manifest hash
in the ghor registry entry and in the Cargo-compatible JSON-lines entry:

- `package_manifest_hash` / `ghor_package_manifest_hash`;
- `ffi_link_manifest_hash` / `ghor_ffi_link_manifest_hash`.

Registry consumers validate these fields. `ghor fetch`, `ghor install`, and
`ghor vendor` download or read `packages/<name>/<version>/manifest.json`, verify
its SHA-256 when the index declares it, validate that its `ffi_link_manifest`
has `status = "closed"`, and compare the FFI manifest hash when present.
Corrupt package manifests, missing FFI manifests, failed compiler FFI reports,
or FFI hash mismatches fail with `E_GHOR_FFI_MANIFEST_001` or
`E_GHOR_REGISTRY_MANIFEST_001`. ghor does not silently install or vendor a
registry package whose source-level FFI artifacts are invalid.

This closes the package-manager side of compiler FFI packaging: ghor can now
surface, lock, publish, fetch, install, vendor and link-plan native artifacts
declared inside nC sources without duplicating compiler semantics.
