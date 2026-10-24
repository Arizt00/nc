# Compiler FFI Contracts (Unified)

Valid imports:
- `use foreign.c;`
- `use foreign.cpp;`
- `use foreign.rust;`

Invalid dotted import contracts (must fail):
- `use foreign.c.abi...`
- `use foreign.cpp.abi...`
- `use foreign.rust.abi...`

Reason: ABI/layout/ownership/nullability/boundary are contract clauses inside `foreign:` blocks, not modules.

## ABI Contract Matrix

`ncc compiler ffi-contracts <file> --json` is the compiler-side static ABI
contract report. It does not compile or link by itself; it verifies that source
declares the information required by the real FFI/linker lanes.

The JSON includes `symbol_contracts[]`. Each row records:

- `symbol`, `language`, `container`, and `symbol_tag`.
- `abi`, `signature`, `layout`, `ownership`, `nullability`, and `side_effects`.
- `header`, `library`, and `linkage` when the source declares an external
  library contract.
- `ownership_transfer.destructor_or_free_contract` for owned, moved, or
  transferred values.
- C++ specific `exported_as`, `cpp_abi`, `template`, `vtable`, and
  `exception_boundary`.
- Rust specific `panic_boundary`.
- `toolchain_evidence`, naming the runtime command that proves the native lane.

The row `status` is `closed` only when the compiler sees no official
`E_FFI_*` diagnostics for that symbol. If a C++ template or mangled symbol is
declared without `cppAbi`, the row fails and diagnostics include
`E_FFI_ABI_MISMATCH_001`. If C++ ownership is owned, moved, or transferred
without `freeSymbol` or `destructorPolicy`, diagnostics include
`E_FFI_FREE_CONTRACT_001`.

The matrix is intentionally separate from toolchain smoke:

- `ffi-contracts` checks source contracts.
- `ffi-smoke --strict` checks C/C++/Rust/ASM native link/run.
- `ffi-cpp-abi --strict` checks real C++ ABI behavior: mangling, templates,
  vtables, destructors, copy/move, exceptions, static/shared libraries, and
  layout.
