# C++ ABI Real Toolchain Boundary

This document closes the normal nC/high-level C++ ABI boundary as a real host-toolchain smoke, not as a static declaration. It does not change [@machine], basal, primitive, parser semantics, pointer semantics, or NBB0. It adds a compiler evidence lane:

```text
ncc compiler ffi-cpp-abi --json --strict
```

The command creates a temporary C++17 workspace, compiles real C++ source, links object/static/shared artifacts, executes a driver, and inspects symbols with `nm` plus `c++filt`.

## Covered ABI Features

- C++ name mangling: real mangled symbols are emitted and demangled.
- Templates: explicit instantiations are emitted and called.
- Vtables: virtual dispatch through a base pointer is executed.
- Destructors: deleting through a base pointer invokes the derived destructor.
- Copy constructor: a native C++ object is copied through its real copy constructor
  and the event counter is checked by the executable.
- Move constructor: a native C++ object is moved through its real move
  constructor, with moved-from state checked by the executable.
- RAII scope destruction: local C++ object construction/destruction is checked
  at block exit.
- Exceptions: C++ exceptions are caught inside the C++ boundary; an uncaught throw is rejected as a boundary failure.
- Layout: `alignas(16)` and packed layout are checked at runtime.
- Linkage: object file, static archive, shared library, and executable are produced by the host toolchain.
- Evidence: emitted artifacts include size and SHA-256 in the JSON report.

## Boundary Policy

nC does not pretend that arbitrary C++ is safe. C++ crossings still require explicit symbol, ABI, layout, ownership, nullability, lifetime, destructor/copy/move policy when ownership crosses the boundary, and exception-boundary contracts. The real smoke proves that the compiler surface can interact with native C++ ABI artifacts; it does not import C++ exception semantics into safe nC.

For Linux/macOS/FreeBSD hosts, the validated profile is Itanium C++ ABI when `_ZN`/`_ZTV` symbols and expected demangled names are present. MSVC C++ ABI remains a host-specific profile and must be validated on a Windows/MSVC host before being claimed for that host.

## Baseical Diagnostics

- `E_FFI_CPP_ABI_001`: the C++ ABI toolchain smoke failed to compile, link, inspect, or validate.
- `E_FFI_CPP_EXCEPTION_BOUNDARY_001`: an uncaught C++ exception crossed the boundary silently.
- `E_FFI_DEFERRED_001`: non-strict execution skipped because a required host tool is missing.

## Evidence Commands

```bash
ncc compiler ffi-cpp-abi --json --strict
ncc check examples/ffi/cpp_abi_full_contract.n
```

The command is also surfaced by `ncc compiler doctor --json` under `cpp_abi_surface`.
