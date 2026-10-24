# Compiler FFI C Contracts

`foreign: c symbol "..." { ... }` is contractual. Required: `abi`, `signature`, `ownership`, `sideEffects`, `layout/repr`, `header`, `library`.

Industrial diagnostics:
- `E_FFI_ABI_MISSING_001`
- `E_FFI_SIGNATURE_MISSING_001`
- `E_FFI_FREE_CONTRACT_001`
- `E_FFI_TYPE_UNSUPPORTED_001`
- `E_FFI_STRING_BOUNDARY_001`
