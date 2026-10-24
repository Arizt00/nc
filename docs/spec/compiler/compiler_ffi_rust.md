# Compiler FFI Rust Contracts

Rust FFI crossing is contract-first and requires `panicBoundary` and `repr/layout` for shared structs.

Industrial diagnostics:
- `E_FFI_RUST_PANIC_BOUNDARY_001`
- `E_FFI_REPR_REQUIRED_001`
- `E_FFI_RESULT_OPTION_BOUNDARY_001`
