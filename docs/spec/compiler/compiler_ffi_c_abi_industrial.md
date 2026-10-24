# Compiler FFI C ABI Industrial Contract

This document defines the C ABI lane for nC foreign integration. It does not
change nC semantics; it records the closed contract that lets nC call, link,
inspect, and package C ABI artifacts with the same discipline expected from a
native C toolchain boundary.

## Baseical Source Surface

The source import remains:

```n
use foreign.c;
```

The import itself only selects the foreign language lane. Authority lives in the
`foreign:` symbol contract, not in a dotted import path. A C symbol contract must
declare:

- `abi: C`
- `header`
- `library`
- `linkage`
- `layout` or `repr`
- `ownership`
- `sideEffects`
- `signature`
- `nullability` when the signature contains pointer-like values
- `freeSymbol` or destructor policy when ownership is owned or moved

The release fixture is:

```text
examples/ffi/c_abi_industrial_static_shared.n
```

The official missing-header negative fixture is:

```text
examples/ffi/c_abi_missing_header_fail.n
```

It must fail with `E_FFI_IMPORT_CONTRACT_001`.

## Real Toolchain Evidence

The integration test `ffi_c_abi_header_static_shared_checksum_smoke` creates a
temporary C ABI package and validates it through the host toolchain:

1. Writes `nc_c_abi_industrial.h`.
2. Writes `nc_c_abi_industrial.c`.
3. Compiles a PIC object file with `cc`.
4. Creates `libnc_c_abi_industrial.a` with `ar`.
5. Creates `libnc_c_abi_industrial.so` with `cc -shared`.
6. Uses `nm -g` to verify exported symbols.
7. Links and executes a static-driver binary.
8. Links and executes a shared-library driver binary.
9. Emits a temp JSON manifest with SHA-256 hashes for header, object, archive,
   shared library, and both driver executables.

This is not a fake bridge and not JSON pretending to be C. The test executes
native C code, native static linking, native shared linking, native symbol
inspection, and native binary execution.

## Symbols Covered

The C ABI smoke covers:

- `nc_c_abi_add`
- `nc_c_abi_packet_init`
- `nc_c_abi_packet_value`
- `nc_c_abi_alloc_bytes`
- `nc_c_abi_free_bytes`

Together they cover scalar calls, struct layout by pointer, const pointer reads,
owned allocation return, and who-frees ownership release.

## Boundary Rules

- nC does not infer C layout silently.
- nC does not accept owned C memory without a release contract.
- nC does not accept pointer-like C signatures without nullability.
- nC does not pass `str`, `text`, `Result`, `Option`, list, or runtime/unbinary
  objects over the C ABI directly.
- C symbols requiring filesystem, process, or network side effects must declare
  capability and policy.

## Validation Commands

```bash
cargo test -p ncc-cli --test ffi_real_link_run_e2e -- --nocapture
cargo run -p ncc-cli -- check examples/ffi/c_abi_industrial_static_shared.n
cargo run -p ncc-cli -- check examples/ffi/c_abi_missing_header_fail.n || true
cargo run -p ncc-cli -- compiler ffi examples/ffi/c_abi_industrial_static_shared.n --json
cargo run -p ncc-cli -- compiler ffi-contracts examples/ffi/c_abi_industrial_static_shared.n --json
```

## Status

`closed_real_c_abi_toolchain_smoke` when the host provides `cc`, `ar`, and `nm`.
When those tools are missing, the integration test skips rather than inventing
evidence.
