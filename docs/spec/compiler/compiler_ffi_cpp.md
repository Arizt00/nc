# Compiler FFI C++ Contracts

Preferred boundary: `exportedAs: externC`.

Required contracts include `exceptionBoundary` and `layout/repr`. Mangled exports require `cppAbi`.

Industrial diagnostics:
- `E_FFI_CPP_EXCEPTION_BOUNDARY_001`
- `E_FFI_ABI_MISMATCH_001`
- `E_FFI_LAYOUT_REQUIRED_001`
