# Backend Industrial Point 1

This document defines the first industrial backend closure point for `ncc`.

Scope:

- ABI matrix: SysV64, Win64, AArch64 AAPCS64, AArch64 SysV64 where object-level support applies.
- Foreign ABI boundaries: C ABI, C++ ABI through explicit FFI contracts, Rust through `extern C`/`repr(C)`.
- Optimizer pipeline: current release-fast native pipeline, AST constant folding, dead statement pruning, backend peephole and object statistics.
- Register allocation: register allocation v2 with recursive liveness for call-free structured control flow, explicit CFG live-in/live-out, local interference graph coloring, conservative copy coalescing, resident locals, callee-save preservation and spill/reload under pressure.
- Object/link: ELF64, COFF, Mach-O, WASM object/module surfaces and hosted executable link through the host linker where available.
- Debug/unwind: target debug model contracts and closed machine debug/unwind foundation; full high-level source line tables remain future work.
- Atomics/volatile: volatile memory lowering is native; atomic policy is explicit, cannot silently downgrade to non-atomic memory operations, exposes x86_64, Win64 and AArch64 RMW byte matrices for the current backend lanes, and provides a Win64/AArch64 external host smoke over assembler, object bytes, objdump and linker.
- Memory/pointers: `ptrRead`, `ptrWrite`, `ptrOffset`, `ptrDiff`, `ptrFromAddr`, `addrOf`, volatile accesses, atomic RMW rows, barriers/fences, `restrict` alias contracts, UB policy and deep alias evidence are exposed in the industrial manifest.
- Multi-host matrix: Linux x86_64, Linux AArch64, Windows x86_64, Windows AArch64, macOS x86_64 and macOS AArch64 are represented as host-dependent object/execution lanes.

The official machine-readable report is:

```bash
ncc backend-model --industrial --json
```

The report deliberately does not claim full C/C++/Rust backend parity. Full parity would require a global optimizer comparable to LLVM-grade pipelines, full exception/unwind interop, C++ template/proc-macro-equivalent ecosystems, complete debug info for every high-level construct and prolonged multi-host CI.

Closed in this point:

- The backend model exposes an industrial closure boundary with no fake parity.
- ABI rows exist for SysV64, Win64, AArch64 AAPCS64, AArch64 SysV64, C ABI, C++ ABI and Rust ABI boundaries.
- Volatile is closed as native memory lowering.
- Atomics are policy-closed with no silent downgrade; native x86_64, Win64 and AArch64 dword/qword RMW bytes are exposed for exchange, fetch-add and compare-exchange. Win64/AArch64 external smoke is available through `ncc backend-model --industrial --host-smoke --json`. Missing toolchains are reported as `missing_environment`, not success.
- Backend tooling is closed for comparative disasm, seeded codegen fuzz/corpus, differential C/Rust benchmark comparisons and longer benchmark harness execution.
- Multi-host CI is represented by `.github/workflows/ncc-backend-industrial-multi-host.yml`; public release requires rerunning it at exact release head and retaining its artifacts.
- Object/link/debug/optimizer/register allocation status is visible in one CLI report.

Follow-up evidence:

- `docs/spec/backend/backend_register_allocator_atomic_v2.md`.
- `docs/status/STATUS_ncc_backend_ra_atomic_v2.md`.
- `docs/spec/backend/backend_four_point_industrial_closure.md`.
- `docs/status/STATUS_ncc_backend_four_point_industrial_closure.md`.

Required gate:

```bash
cargo test -p ncc-cli backend_model_cli_tests
cargo run -q -p ncc-cli -- backend-model --industrial --host-smoke --json
```
