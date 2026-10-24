# Compiler ABI Contract Matrix

The compiler ABI contract matrix is the static source-level contract for FFI
symbols before the real linker/toolchain lanes run.

Command:

```bash
ncc compiler ffi-contracts examples/ffi/cpp_abi_full_contract.n --json
```

The command emits `symbol_contracts[]`. A contract row is closed only when the
symbol has all required declarations for its language and no official
`E_FFI_*` diagnostics.

## Common Fields

Every FFI symbol must declare:

- `abi`
- `signature`
- `layout` or `repr`
- `ownership`
- `sideEffects`

Pointer-like signatures (`ptr<T>` or `memRegion<T>`) must also declare
`nullability`.

## Ownership Transfer

Owned, moved, and transferred ownership all require a destruction contract:

- `freeSymbol`
- `destructorPolicy`

Missing destruction contracts report `E_FFI_FREE_CONTRACT_001`. This keeps C and
C++ power available while ensuring that ownership crossing the ABI boundary is
not silently under-specified.

## C++ Contracts

C++ symbols require:

- `exceptionBoundary`
- `layout` or `repr`
- `cppAbi` when the symbol is mangled, namespaced, or templated

The matrix detects template-like symbols and records whether `cppAbi` closes the
contract. It also exposes vtable-like symbols when the source name or explicit
fields indicate virtual dispatch. Real execution evidence is owned by:

```bash
ncc compiler ffi-cpp-abi --json --strict
```

That command validates native C++17 behavior for mangling, demangling, template
instantiation, vtables, virtual destructors, copy/move constructors, RAII,
exception boundaries, aligned/packed layout, object files, static archives,
shared libraries, and executable drivers.

## Rust Contracts

Rust FFI symbols require:

- `panicBoundary`
- `repr` or `layout`

Rust must cross through explicit ABI-safe representations. Native evidence is
provided by `ncc compiler ffi-smoke --json --strict`.

## Boundary

The matrix does not claim that an external library exists. It proves the nC
source declares the ABI contract completely enough for later package/linker
stages to consume it.
