# Machine Interop From `.n`

Status: implemented as bridge manifest and machine gate.

This phase closes the typed contract between generated `.n` functions and
`[@machine]` consumers. The source of truth is emitted by:

```bash
ncc compiler machine-interop examples/09_ffi/machine_interop_bridge.n --target x86_64-unknown-linux-gnu --json
ncc machine interop examples/09_ffi/machine_interop_bridge.n --target x86_64-unknown-linux-gnu --json --strict
```

The compiler manifest schema is `nc.compiler.machine_interop.v1`. Its ABI
parity subsection is `nc.compiler.highlevel_machine_abi_parity.v1`. The machine
gate schema is `nc.machine.interop_gate.v1`.

## Closed Surface

- Names: each `.n` function receives a logical name and a native symbol alias,
  for example `machineInteropBridge.exportedAdd =
  nc_machineInteropBridge_exportedAdd`.
- ABI: target triples select concrete ABI rows: SysV64, Win64, AAPCS64 and
  RISC-V64. Register contracts include argument registers, return register,
  bool return register, shadow space and callee-saved registers. Each exported
  high-level function now also carries a `call_abi` row with visible argument
  locations, hidden `sret` storage when needed, stack argument bytes and shadow
  space.
- Structs: `type:` contracts are lowered to natural target layout with size,
  alignment and field offsets. `Packet` in the fixture is 32 bytes with fields
  at offsets `0`, `8`, `16` and `24`.
- Pointers and strings: `ptr<T>`, `addr` and `str` lower as pointer-width
  machine values. `str` is a pointer to an immutable `{ data_ptr, byte_len }`
  descriptor; string literals get payload labels, descriptor labels, UTF-8
  bytes, NUL-terminated bytes and descriptor relocation metadata.
- Foreign imports: `foreign:` contracts are retained as machine extern rows,
  their `fn(...) -> ...` signatures are classified through the same ABI
  algorithm, and they are also embedded with the existing FFI link manifest.
- Runtime symbols: imported runtime calls are selected from
  `nc.runtime.symbol_abi.v1`; machine does not infer runtime ABI shapes.

## Directionality

The manifest emits two rows for callable `.n` functions:

- `n_to_machine_call`: generated `.n` code may call a declared machine or
  foreign symbol using the target ABI.
- `machine_to_n_call`: machine code may call generated `.n` functions through
  the emitted native symbol and ABI row.

Both directions carry params, return type, native symbol, target ABI and
lowering status. This keeps names, structs, pointers, strings and runtime
symbols intact across the boundary.

Aggregate return parity is explicit. Small aggregates use the target register
window only when the active ABI permits it. Larger aggregates use hidden
structure-return storage:

- SysV64: hidden pointer in `rdi`, visible integer arguments start after it.
- Win64: hidden pointer in `rcx`, with the usual 32-byte shadow space.
- AAPCS64: hidden pointer in `x8`.
- RISC-V64: hidden pointer in `a0`.

## Evidence

The fixture is `examples/09_ffi/machine_interop_bridge.n`.

The regression tests live in `crates/ncc-cli/tests/machine_closure.rs`:

- `machine_interop_compiler_manifest_from_n_ok`
- `machine_interop_gate_from_n_ok`
- `machine_interop_win64_abi_contract_from_n_ok`

These tests cover parser/sema acceptance, manifest JSON, struct layout, runtime
`print`/`println`, native string bytes, external symbol import and Win64 ABI
register selection.
