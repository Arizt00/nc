# ghor Manifest

`ghor.toml` is the official manifest for ghor projects. `nc.toml` remains accepted as a compatibility manifest so existing projects and tests continue to work.

## Required Sections

```toml
[package]
name = "demo"
version = "0.1.0"
kind = "bin"

[toolchain]
channel = "stable"
manager = "ncup"

[target.default]
triple = "x86_64-unknown-linux-gnu"
abi = "sysv64"

[languages]
primary = "nC"
adapters = []
```

## Dependency Sections

The official dependency sections are:

- `[dependencies]` for nC package dependencies.
- `[basal-dependencies]` or `[basal-deps]` for basal `.nbp.tar.gz`
  packages published by `ncc basal publish`.
- `[rust-dependencies]` for Rust adapter dependencies.
- `[c-dependencies]` for C adapter dependencies.
- `[cpp-dependencies]` for C++ adapter dependencies.
- `[asm-dependencies]` for assembler adapter dependencies.

Remote dependencies must declare a checksum. Local path dependencies must resolve on disk.

Basal dependencies are explicit and never masquerade as Cargo crates. A basal
dependency points at a basal registry index and names the root id or safe
package name:

```toml
[basal-dependencies]
nc_root_ffi_foreign_c = {
  version = "1",
  registry = "./basal-registry",
  package = "nC.root.ffi.foreign_c",
  checksum = "<sha256-of-nbp-archive>"
}
```

`ghor resolve`, `ghor fetch`, `ghor vendor`, and `ghor metadata` surface these
entries with `source = "basal-registry"`. Resolution validates the basal
registry entry, archive checksum, `basal.package.v1` manifest hash, embedded
`ghor.ffi_manifest.v1` validation summary, and NBB0 metadata before accepting
the package. Vendor output writes `.basal-checksum.json`, not
`.cargo-checksum.json`, so basal packages remain distinct from Cargo-compatible
crate packages.

## Native Dependencies

External native libraries use `[native-dependencies]` or
`[native-dependencies.<name>]`. These entries describe C, C++, Rust, Zig, Go
and ASM libraries by language, ABI, headers, include paths, link paths, library
names, exported symbols, link mode, pkg-config package, framework, local
artifact and checksum. They are surfaced in manifest, metadata, graph, link and
lock outputs, but they are not treated as Cargo crates.

Compiler FFI declarations inside `.n` sources are surfaced through
`ghor ffi-manifest --json`. The package manager reuses the compiler
`ffi-link-manifest` collector, so source-level `foreign:` artifacts and
checksums are reported in metadata, link plans and `ghor.lock.meta.json` without
duplicating compiler parsing rules.

When a package is published to a registry, the generated package
`manifest.json` carries that FFI manifest and the registry index carries hashes
for both the package manifest and the FFI manifest. Registry fetch/install/vendor
validate those hashes before accepting package bytes.

## Targets

Target-specific entries use `[target.<triple>]`. `ghor` validates declared triples against the known target matrix.

## Profiles, Features, Capabilities, Artifacts

`[profiles.<name>]`, `[features]`, `[capabilities]`, and `[artifact]` are parsed and surfaced in metadata, reports, lockfiles, build plans, and artifact manifests.

## Diagnostics

- `E_GHOR_MANIFEST_001`: manifest missing.
- `E_GHOR_MANIFEST_MISSING_NAME_001`: `package.name` missing.
- `E_GHOR_MANIFEST_INVALID_VERSION_001`: version missing or malformed.
- `E_GHOR_MANIFEST_INVALID_TARGET_001`: unsupported target triple.
- `E_GHOR_LANGUAGE_ADAPTER_MISSING_001`: declared adapter lacks an adapter section.
- `E_GHOR_CHECKSUM_001`: remote dependency lacks checksum.
