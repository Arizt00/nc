# Compiler FFI Real Link+Run Contract (v2.1.0)

## Scope
This document defines the release FFI lane for native C/C++/Rust/ASM integration and run validation in `ncc`.
ASM is active through `foreign.asm` and remains gated by `[@machine]` for source programs.

Authoritative commands:

- `ncc compiler ffi-smoke --json`
- `ncc compiler ffi-smoke --json --strict`
- `ncc compiler ffi-cpp-abi --json --strict`
- `ncc compiler ffi-external-bindings --json`
- `ncc compiler ffi <file> --json`
- `ncc compiler ffi-symbols <file> --json`
- `ncc compiler ffi-contracts <file> --json`

## What “real” means here
`ffi-smoke` is not static-only. It executes a native build pipeline in a temp workspace:

1. Emit native units (C, C++, Rust).
2. Emit an ASM identity unit (`ffi_asm.S`) for the host ABI.
3. Compile units to objects with host toolchain (`cc`, `c++`, `rustc`).
4. Validate exported symbols (`nm`).
5. Link into one executable.
6. Run executable and assert deterministic contract output, including `nc_ffi_asm_identity(42) == 42`.

If any step fails, compiler emits official `E_FFI_*` diagnostics.

`ffi-cpp-abi` is the deeper C++ ABI lane. It compiles native C++17, links an object file, a static archive, a shared library, and an executable, then executes checks for Itanium-style name mangling/demangling on Linux/macOS/FreeBSD hosts, template instantiation, vtable dispatch, virtual destructor dispatch, copy constructor behavior, move constructor behavior, RAII scope destruction, exception boundary behavior, aligned struct layout, and packed struct layout. The JSON report includes artifact sizes and SHA-256 hashes.

In addition, the repository now contains a user-source end-to-end path with a real nC program:

1. nC source declares `foreign` symbols and calls them.
2. sema resolves those calls from the `foreign` contract block.
3. backend lowers calls into external relocations.
4. linker produces the final executable.
5. executable performs real native calls and returns success.

This path is covered by:

- `examples/ffi/ffi_real_link_run_user_program.n`
- `crates/ncc-cli/tests/ffi_real_link_run_e2e.rs`

And `ncc run` now exposes direct native-link execution for this lane:

- `ncc run <file.n> --native-ffi [--cc <profile>] [--linker <bin>] [--entry <symbol>]`

`ffi-external-bindings` is the broader external-library matrix. It executes C,
Rust, ASM and the C++ ABI lane on hosts with the required tools, and exposes real
optional Zig/Go lanes via C ABI exports. If `zig` or `go` is absent, those rows
report `skipped_missing_tool`; they are not reported as success.

When a program declares `foreign` symbols, `run --native-ffi` now performs autolink without wrapper scripts:

1. Builds nC object from user source.
2. Resolves sidecar sources from FFI contracts (`library` / `crate` / optional `source`).
3. Compiles sidecars (`.c` / `.cpp` / `.rs`) to objects.
4. Links launcher + nC object + sidecars (+ declared libraries when sidecar source is absent).
5. Executes the resulting native binary and validates exit status.

The deeper real linker contract is specified in
`docs/spec/compiler/compiler_ffi_linker_real.md`. The native FFI autolink lane
can consume declared static and shared artifacts through `artifact`,
`libraryPath`, `library`, `linkPath`, `rpath`, and SHA-256 checksum fields.
Shared artifacts receive runtime loader paths, and checksum mismatches fail with
`E_FFI_CHECKSUM_001`.

The linker inputs can be inspected without linking by running
`ncc compiler ffi-link-manifest <file.n> --json`; see
`docs/spec/compiler/compiler_ffi_link_manifest.md`.

## Structured output contract
`ffi-smoke` JSON includes:

- `status`: `ok` or `skipped`
- `toolchain`: resolved paths (`cc`, `cxx`, `rustc`, `nm`)
- `symbols`: required symbols
- `symbol_status`: per-symbol lifecycle state (`contract_validated`, `linkable`, `linked`, `executed`, `skipped_by_toolchain`, `failed`, `failed_code`, `state`)
- `symbol_contracts`: static ABI contract matrix for each source-declared
  foreign symbol, including symbol, ABI, signature, layout, ownership,
  ownership-transfer destructor/free contract, nullability, side effects,
  header/library/linkage metadata, C++ template/vtable/exception fields, and
  Rust panic-boundary fields.
- `interop`: booleans for `c` / `cpp` / `rust` / `asm` lanes
- `contracts`: ABI/layout/ownership/nullability flags
- `workspace`: temp path used for build/run
- `host`: host OS/arch used for the smoke run
- `abi_matrix`: host-linkability matrix for `nC.sysv64`, `nC.win64`, `nC.aapcs64` plus C/C++/Rust/ASM interop flags
- `negative_cases`: real negative executions with official codes:
  - `abi_mismatch_real` -> `E_FFI_ABI_MISMATCH_001`
  - `symbol_missing_real` -> `E_FFI_SYMBOL_MISSING_001`
  - `null_return_nonnull_real` -> `E_FFI_NULLABILITY_001`
  - `panic_real_rust` -> `E_FFI_RUST_PANIC_BOUNDARY_001`
  - `throw_real_cpp` -> `E_FFI_CPP_EXCEPTION_BOUNDARY_001`
- `asm_surface`: explicit active gate for `foreign.asm` (`status=active`, `gate=[@machine]`); missing source bridge reports `E_FFI_ASM_BRIDGE_001`

`--strict` hard-fails if required host tool binaries are missing.

`ffi-cpp-abi` JSON includes:

- `cpp_abi.status = closed_real_toolchain_smoke`
- `name_mangling.status = closed_real`
- `templates.status = closed_real`
- `vtables.status = closed_real`
- `destructors.status = closed_real`
- `copy_move.status = closed_real`
- `exceptions.status = closed_real`
- `layout.status = closed_real`
- `libraries.static/shared/object_files`
- `artifacts[]` with path, size, and SHA-256

## Baseical diagnostics
- `E_FFI_IMPORT_CONTRACT_001`
- `E_FFI_IMPORT_UNKNOWN_001`
- `E_FFI_ABI_MISSING_001`
- `E_FFI_ABI_MISMATCH_001`
- `E_FFI_LAYOUT_REQUIRED_001`
- `E_FFI_REPR_REQUIRED_001`
- `E_FFI_SYMBOL_MISSING_001`
- `E_FFI_LINK_FAILED_001`
- `E_FFI_LINK_001`
- `E_FFI_CHECKSUM_001`
- `E_FFI_CALL_001`
- `E_FFI_OWNERSHIP_001`
- `E_FFI_NULLABILITY_001`
- `E_FFI_CPP_EXCEPTION_BOUNDARY_001`
- `E_FFI_CPP_ABI_001`
- `E_FFI_RUST_PANIC_BOUNDARY_001`
- `E_FFI_SIDE_EFFECT_001`
- `E_FFI_CAPABILITY_001`
- `E_FFI_POLICY_001`
- `E_FFI_TYPE_UNSUPPORTED_001`
- `E_FFI_STRING_BOUNDARY_001`
- `E_FFI_RESULT_OPTION_BOUNDARY_001`
- `E_FFI_PRIMITIVE_CROSSING_001`
- `E_FFI_ASM_BRIDGE_001`

## Safety boundaries
- compiler remains semantic authority,
- runtime does not invent foreign symbols,
- `use foreign.c|cpp|rust|asm;` is official and dotted import contracts are rejected,
- `foreign.asm` requires a `[@machine]` bridge; missing bridge reports `E_FFI_ASM_BRIDGE_001`,
- primitive crossing remains explicitly rejected.

## Release examples
FFI contract examples live at:

- `examples/ffi/c_ffi_validate_amount.n`
- `examples/ffi/c_ffi_buffer_with_bounds.n`
- `examples/ffi/c_ffi_errno_capture.n`
- `examples/ffi/c_abi_industrial_static_shared.n`
- `examples/ffi/c_abi_missing_header_fail.n`
- `examples/ffi/c_ffi_owned_requires_free_fail.n`
- `examples/ffi/cpp_ffi_extern_c_score.n`
- `examples/ffi/cpp_ffi_exception_boundary_ok.n`
- `examples/ffi/cpp_abi_full_contract.n`
- `examples/ffi/cpp_ffi_exception_boundary_missing_fail.n`
- `examples/ffi/cpp_ffi_mangled_requires_cppabi_fail.n`
- `examples/ffi/rust_ffi_validate_transaction.n`
- `examples/ffi/rust_ffi_repr_c_ok.n`
- `examples/ffi/rust_ffi_panic_boundary_ok.n`
- `examples/ffi/rust_ffi_panic_boundary_missing_fail.n`
- `examples/ffi/ffi_bad_path_import_fail.n`
- `examples/ffi/ffi_list_direct_crossing_fail.n`
- `examples/ffi/ffi_str_direct_crossing_fail.n`
- `examples/ffi/ffi_unknown_side_effect_fail.n`
- `examples/ffi/ffi_real_link_run_user_program.n`
- `examples/compiler/compiler_ffi_asm_machine_ok.nm`
- `examples/ffi/ffi_c_add.c`
- `examples/ffi/ffi_cpp_mul.cpp`
- `examples/ffi/ffi_rust_sub.rs`

## Evidence commands
```bash
cargo run -p ncc-cli -- compiler ffi-smoke --json
cargo run -p ncc-cli -- compiler ffi-smoke --json --strict
cargo run -p ncc-cli -- compiler ffi-cpp-abi --json --strict
cargo run -p ncc-cli -- compiler ffi-external-bindings --json
cargo run -p ncc-cli -- compiler ffi examples/ffi/c_ffi_validate_amount.n --json
cargo run -p ncc-cli -- compiler ffi examples/ffi/c_abi_industrial_static_shared.n --json
cargo run -p ncc-cli -- compiler ffi-symbols examples/ffi/cpp_ffi_extern_c_score.n --json
cargo run -p ncc-cli -- compiler ffi-contracts examples/ffi/rust_ffi_validate_transaction.n --json
cargo run -p ncc-cli -- compiler diagnostics --json
cargo run -p ncc-cli -- check examples/ffi/c_ffi_validate_amount.n
cargo run -p ncc-cli -- check examples/ffi/ffi_bad_path_import_fail.n || true
cargo run -p ncc-cli -- run examples/ffi/ffi_real_link_run_user_program.n --native-ffi --entry ffiRealLinkRunUserProgram.run
```

## CI multi-host matrix
Real CI coverage for FFI strict smoke is wired through:

- `.github/workflows/ffi-smoke-multi-host.yml`

Matrix rows:
- Linux x86_64 (`ubuntu-24.04`)
- Linux aarch64 (`ubuntu-24.04-arm`)
- macOS x86_64 (`macos-15-intel`)
- macOS aarch64 (`macos-14`)
- Windows x86_64 (`windows-latest`)
- Windows aarch64 (`windows-11-arm`)

Each row runs:
- toolchain probe,
- `cargo build -p ncc-cli --bin ncc`,
- `cargo run -p ncc-cli -- compiler ffi-smoke --json --strict`,
- JSON evidence upload as artifact.

The workflow is release-blocking: a non-zero strict smoke exit code fails the row, and therefore the matrix.
