# ghor C And C++ Integration

C and C++ adapters are declared through `[language_adapter.c]` and `[language_adapter.cpp]`.

`ghor` records dependency sections for C and C++, include/link intent through manifest data, and host compiler availability for `cc`, `clang`, `gcc`, `c++`, `clang++`, and `g++`.

Absent compilers are environment state, not successful integration evidence.

## Executable Bridge

`ghor interop --json` closes the executable C/C++ side of the bridge:

- it writes a shared C ABI header;
- it compiles `ghor_c_add.c` with `cc`;
- it links `calls_c.cpp` with `c++` against the C object;
- it runs the resulting executable;
- it records stdout/stderr hashes and artifact SHA-256 values.

The bridge is intentionally C ABI based. C++ participates through `extern "C"`
symbols, so the smoke validates linkable interop without pretending that ghor
owns C++ exception, template or runtime semantics.

See `docs/spec/ghor/ghor_c_cpp_rust_interop.md`.
