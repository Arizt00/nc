# Basal Foreign Bindings

Basal foreign bindings model external C, C++, Rust, Zig, Go, and ASM symbols as
explicit root contracts. They are not imports. A Basal root still cannot use
`use foreign.*`; machine, primitive, basal, and kernel modes keep their own
section grammar.

Baseical form:

```n
foreign:
    c_add:
        language: c;
        symbol: nc_basal_c_add;
        abi: c;
        layout: repr_c;
        ownership: caller;
        header: "nc_basal_c_add.h";
        artifact: "libnc_basal_c_add.a";
        link: static;
```

Required fields are `language`, `symbol`, `abi`, `layout`, and `ownership`.
Optional fields are `header`, `artifact`, `link`, and `bridge`. Supported
languages are `c`, `cpp`, `rust`, `zig`, `go`, and `asm`.

`basal ir --json` emits each contract as `BasalForeignBindingIr`. `basal lower
--json` emits `BasalExternalBindingLowering`, a machine symbol with status
`resolved_to_external_abi_symbol`, and a relocation intent
`external_abi_symbol_reference`.

`basal ffi-manifest <file.nb> --json` emits a package-facing FFI manifest for
the same bindings. The embedded `ffi_link_manifest` uses
`schema = "ghor.ffi_manifest.v1"` with `producer = "basal"` and
`ghor_compatible = true`, so package tooling can validate Basal foreign
contracts with the same manifest/hash boundary used by ghor registry packages.
The command also emits:

- `ffi_link_manifest_hash`;
- `package_manifest_hash`;
- `compiler_ffi_entry_count`;
- one entry per `foreign:` binding;
- artifact metadata for declared external artifacts;
- diagnostics if a binding is not closed.

NBB0 compatibility is preserved by recording foreign binding metadata in the
ExecutableSurface section and by carrying external symbol/relocation strings in
the existing lowering record. When a root declares foreign bindings, the
ExecutableSurface section also includes a `foreign_ffi_manifest` record with
the package FFI manifest hash and entry count. No new binary magic, fake
evidence, or final machine evidence claim is introduced by this surface.

During `basal integrate-evidence`, each declared foreign binding is compiled
into a small real external artifact for its language lane, linked with a C ABI
driver, executed, hashed, and embedded in the NBB0 machine evidence record as
`external_artifacts`. The current evidence lanes are:

- C: object file compiled with `cc`, linked and executed through a C driver.
- C++: object file compiled with `c++`; C ABI symbols and selected mangled C++
  symbols are link-smoked without inventing a fake resolver.
- C++ ABI full surface: Basal fixtures exercise Itanium C++ ABI contracts for
  template instantiation, vtable dispatch, destructor/RAII execution, exception
  boundary handling, and static/dynamic library declarations. Each binding is
  compiled, linked through the driver, executed, hashed, and recorded as an
  external artifact. The deeper C++ toolchain matrix remains owned by
  `ncc compiler ffi-cpp-abi --json --strict`; Basal consumes that closed
  contract as foreign evidence instead of cloning compiler semantics.
- Rust: `staticlib` compiled with `rustc`, linked through the C ABI driver.
- Zig: object file compiled with `zig build-obj`, linked through the C ABI
  driver.
- Go: `c-archive` compiled with `go build`, linked through the C ABI driver.
- ASM: object file assembled with `cc`, linked through the C ABI driver.

The external artifact hash and executable hash participate in the deterministic
machine evidence hash. If a compile, link, or run step fails, Basal emits
`E_BASAL_EVIDENCE_EXTERNAL_LINK_001` and does not claim integrated evidence.

Diagnostics:

- `E_BASAL_FOREIGN_BINDING_001`
- `E_BASAL_FOREIGN_LANGUAGE_001`
- `E_BASAL_FOREIGN_SYMBOL_001`
- `E_BASAL_FOREIGN_ABI_001`
- `E_BASAL_FOREIGN_LAYOUT_001`
- `E_BASAL_FOREIGN_OWNERSHIP_001`
- `E_BASAL_FOREIGN_FIELD_001`
- `E_BASAL_FOREIGN_DUPLICATE_001`
- `E_BASAL_EVIDENCE_EXTERNAL_LINK_001`

Examples live under `examples/basal/ffi/`.

## Basal Package Registry Bridge

Foreign roots can now be packaged as basal NBB packages without crossing
through imports or changing the NBB0 binary contract:

```sh
ncc basal package examples/basal/ffi/foreign_c_contract.nb --out target/basal/package --json
ncc basal publish examples/basal/ffi/foreign_c_contract.nb --registry target/basal/registry --json
ncc basal install nC.root.ffi.foreign_c --registry target/basal/registry --root target/basal/install --json
ncc basal vendor target/basal/vendor --registry target/basal/registry --package nC.root.ffi.foreign_c --json
```

`basal package` executes the closed basal pipeline: parse, tables, BasalIR,
semantic verifier, lowering verifier, and NBB0 writer. If any step fails, no
package is emitted. The package archive is a real tar.gz payload with:

- the source `.nb`;
- an NBB0 `.nbb` artifact;
- `manifest.json` using `schema = "basal.package.v1"`;
- `ffi_link_manifest.json` using `schema = "ghor.ffi_manifest.v1"`;
- SHA-256 hashes for source, NBB, package manifest, and FFI manifest.

`basal publish` writes a local registry index, package entry, archive and
manifest. `basal install` and `basal vendor` validate the registry archive
checksum, package manifest hash, embedded FFI manifest hash, extracted source
hash, and extracted NBB hash before reporting success.

Remote registry transports remain owned by `ghor`. Basal accepts a local
registry path for this bridge and rejects `http://`, `https://`, `sparse+`, and
`git+` registry strings with `E_BASAL_PACKAGE_REGISTRY_001` instead of
pretending to implement ghor transport internally.
