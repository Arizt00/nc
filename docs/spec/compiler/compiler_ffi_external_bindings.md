# Compiler FFI External Bindings

This document defines the external-library binding matrix for normal nC and
high-level nC. It does not add new language semantics and it does not introduce
`foreign.zig` or `foreign.go`. Zig and Go cross nC through explicit C ABI
exports, the same way many production toolchains expose stable interop.

Authoritative command:

```bash
ncc compiler ffi-external-bindings --json
```

## Required Lanes

The required lanes must execute when `cc`, `rustc`, and `nm` exist on the host:

- C: compiles a C object exporting `nc_ext_c_add`.
- Rust: compiles an `extern "C"` object with a `repr(C)` struct and
  `nc_ext_rust_pair_sum`.
- ASM: compiles a host C ABI assembly bridge exporting `nc_ffi_asm_identity`.
- C++: delegates to `ncc compiler ffi-cpp-abi --json --strict`, which validates
  templates, vtables, destructors, copy/move constructors, RAII, exceptions,
  object files, static libraries, shared libraries, executable linking, and
  symbol demangling.

## Optional Lanes

Zig and Go are real optional lanes:

- Zig: if `zig` exists, the command builds a Zig object with an exported C ABI
  symbol and links/runs a C driver.
- Go: if `go` exists, the command builds a `c-archive` with an exported C ABI
  symbol and links/runs a C driver.

If those toolchains do not exist, the lanes report:

```text
skipped_missing_tool
```

This is not success and not fake evidence. It is an explicit host-environment
boundary.

On the current WSL release host, Zig `0.16.0` and Go `go1.26.4` are provisioned
locally under `~/.local/toolchains` and exposed through `~/.local/bin`. With
`PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"`, both optional lanes execute
and the command reports `optional_status=closed`.

## Policy

- External libraries cross through explicit C ABI or C++ ABI boundaries.
- Rust uses `extern "C"` and `repr(C)` for stable layout.
- Zig uses exported C ABI symbols.
- Go uses `c-archive`/cgo style boundaries.
- ASM uses the existing `foreign.asm` bridge rule and remains source-gated by
  `[@machine]` when declared in nC source.
- Missing optional toolchains are not claimed closed.

## Validation

```bash
PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" ncc compiler ffi-external-bindings --json
cargo test -p ncc-cli --test ffi_real_link_run_e2e -- --nocapture
```

The JSON report includes lane status, toolchain discovery, artifact records, and
`no_fake_evidence=true`.
