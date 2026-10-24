# Backend Four-Point Industrial Closure

This document closes the current backend hardening slice for four requested
areas:

1. C/C++-grade memory and pointer behavior.
2. x86_64 plus ARM64 backend architecture and OS matrix.
3. Backend tooling for comparative disassembly, codegen fuzzing, differential
   testing and longer benchmarks.
4. Prolonged multi-host CI with real object and executable evidence.

The machine-readable authority is:

```bash
ncc backend-model --industrial --json
```

## Memory And Pointers

The closed ncc backend surface treats raw pointer operations as C/C++-grade
backend operations when the program is in a pointer-capable surface:

- `ptrRead<T>` lowers as one typed load.
- `ptrWrite<T>` lowers as one typed store.
- `ptrOffset<T>` lowers as `addr + index * sizeof(T)`.
- `ptrDiff<T>` lowers as `(addr_left - addr_right) / sizeof(T)`.
- `volatileRead<T>` and `volatileWrite<T>` lower as observable volatile
  accesses and may not be silently replaced with ordinary loads/stores.
- `atomicExchange<T>`, `atomicFetchAdd<T>` and
  `atomicCompareExchange<T>` select rows from the x86_64, Win64 or AArch64
  atomic RMW byte matrix.
- `barrier` and fence semantics are explicit target/backend boundaries.
- `restrict` is represented as an aliasing contract. It is not a safety proof;
  it is an optimization promise equivalent in spirit to C/C++ low-level alias
  contracts.

The UB policy is intentionally C/C++-powerful: ncc diagnoses null dereference,
out-of-bounds, stale pointer, double free, use-after-free, bad alignment and
alias contract violations when they are statically or dynamically provable.
When the raw backend cannot prove the issue, it records the evidence boundary
instead of pretending Rust-like safety.

## Architecture And OS Matrix

The industrial backend matrix is no longer x86_64-only. The declared rows are:

| OS | Architecture | Profile | Object | Evidence |
| --- | --- | --- | --- | --- |
| Linux | x86_64 | `nC.sysv64` | ELF64 | native executable smoke |
| Linux | AArch64 | `nC.aapcs64-linux` | ELF64 | cross object/disasm/link, native runner when available |
| Windows | x86_64 | `nC.win64` | COFF | host linker smoke |
| Windows | AArch64 | `nC.win64-arm64-contract` | COFF | retained CI row, native runner/toolchain required |
| macOS | x86_64 | `nC.sysv64-macho-contract` | Mach-O | host linker smoke when available |
| macOS | AArch64 | `nC.aapcs64` | Mach-O | Apple Silicon linker smoke |

AArch64 coverage is visible through machine AArch64 object/disasm evidence,
backend AArch64 atomic exclusive-loop rows, NEON SIMD rows and MCT target
attributes.

## Backend Tooling

The backend tooling closure includes:

- Comparative disassembly through ncc machine/object-aware disasm plus
  external objdump in backend host-smoke commands.
- Seeded codegen fuzz/corpus coverage through machine fuzz smokes, native-fast
  control/call/memory fixtures and atomic/SIMD row matrices.
- Differential testing against C and Rust through `ncc perf native-fast`.
- Longer benchmark runs through `ncc perf native-fast --iterations 25 --json`
  in CI.

The benchmark harness records parser, semantic analysis, IR lowering, backend
emit, object emit, link, native text bytes, object bytes, executable size,
runtime and overhead ratios against C/Rust.

## Multi-Host CI

The CI authority for this closure is:

```text
.github/workflows/ncc-backend-industrial-multi-host.yml
```

It runs the backend lane on:

- Linux x86_64.
- Linux AArch64.
- Windows x86_64.
- Windows AArch64.
- macOS x86_64.
- macOS AArch64.

Each row retains:

- backend industrial manifest;
- atomic host-smoke report;
- SIMD host-smoke report;
- performance/differential benchmark report;
- native object/executable smoke log;
- machine native doctor report.

Missing external assemblers, objdump tools or linkers are reported as
`missing_environment`; they are not counted as passing evidence. Public release
closure still requires a remote CI rerun and retained artifacts for the exact
release head.
