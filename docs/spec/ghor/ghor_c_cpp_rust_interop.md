# ghor C/C++/Rust Interconnection

Status: closed in native nC stage1.

`ghor interop` closes the nC package manager bridge between nC packages and
host C, C++ and Rust toolchains. The command does not pretend that nC becomes a
C, C++ or Rust compiler. It generates a concrete C ABI bridge, compiles and
links host artifacts, runs executable smokes, and records hashes for every
artifact that exists.

## Command

```bash
ghor interop --out target/ghor/interop/c-cpp-rust --json
```

Aliases:

```bash
ghor bridge --json
ghor ffi-bridge --json
```

Actions:

- `smoke`: default; scaffold and run all C/C++/Rust bridge checks.
- `verify`: same executable verification path as `smoke`.
- `scaffold`: write bridge files without running host compilers.

## Generated Surface

The generated output contains:

- `ghor_interop_manifest.json`: nC bridge manifest.
- `include/ghor_interop.h`: shared C ABI header.
- `c/ghor_c_add.c`: C symbol implementation.
- `cpp/calls_c.cpp`: C++ executable calling the C ABI symbol.
- `c/calls_rust.c`: C executable calling Rust exported symbols.
- `rust/Cargo.toml`: isolated Rust staticlib package.
- `rust/src/lib.rs`: `extern "C"` Rust exports with `repr(C)` layout.
- `README.md`: local explanation of the generated evidence surface.

## Real Smoke Checks

The smoke performs the following real steps:

1. Compile the C object with `cc`.
2. Link a C++ executable with `c++` against the C object.
3. Run the C++ executable and verify `ghor_c_add(7, 8) == 15`.
4. Build the Rust crate with real `cargo build --release`.
5. Link a C executable with `cc` against the Rust static library.
6. Run the C executable and verify Rust scalar and `repr(C)` record calls.

Every step records:

- program;
- arguments;
- exit code;
- status;
- stdout SHA-256 hash;
- stderr SHA-256 hash;
- stdout/stderr byte counts.

The stage1 authority is `tools/ghor/nc/src/foreign_execution.n`. It passes an
explicit argument vector and working directory to `spawnIn`; Windows lowers
that directory into `CreateProcessA`, while POSIX changes directory only in the
forked child before `execvp`. No command shell participates in these six steps.

If a tool is missing or a compile/link/run step fails, `ghor interop` returns
`E_GHOR_INTEROP_001`. Missing tools are environment blockers, not synthetic
success.

## ABI Contract

The closed bridge uses C ABI only:

- fixed-width integer types from `<stdint.h>`;
- `extern "C"` for C++ and Rust boundaries;
- Rust `#[repr(C)]` records;
- caller-owned stack values;
- no heap ownership transfer in the release smoke bridge;
- no hidden allocator, destructor or exception crossing.

This is the baseline that lets nC packages interoperate with C, C++ and Rust
without inventing compiler evidence. Richer pointer ownership contracts remain
language semantics and FFI-contract work, while this ghor point closes the
package manager orchestration and host tool execution path.

## JSON Evidence

The command emits `ghor.interop.c_cpp_rust.v1` with:

- `status`;
- output directory;
- foreign toolchain matrix;
- nC bridge manifest path;
- artifact records with SHA-256 hashes and sizes;
- smoke results for C, C++ and Rust;
- bridge metadata matching `ghor doctor` and `ghor conformance`.

## Doctor And Conformance

`ghor doctor --json` reports:

```json
{
  "c_cpp_rust_interop": "closed",
  "foreign_interop": {
    "status": "closed",
    "command": "ghor interop --json",
    "abi": "C"
  }
}
```

`ghor conformance --json` includes the same closed interop record.

## Non Goals

This point does not:

- reimplement `clang`, `gcc`, `rustc` or Cargo;
- execute arbitrary C++ exception interop;
- cross language heap ownership by default;
- erase pointer provenance or FFI contracts;
- synthesize success when host tools are missing.

Those boundaries are intentional. ghor orchestrates and verifies the bridge;
the language and compiler layers keep responsibility for semantic FFI safety.
