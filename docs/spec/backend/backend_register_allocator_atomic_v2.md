# Backend Register Allocation and Atomic RMW V2

This document closes the second backend hardening slice after the industrial backend
manifest: structured-control-flow scalar register allocation with explicit CFG
liveness, local interference graph coloring and conservative copy coalescing, plus
native RMW matrices for x86_64, Win64 and AArch64.

## Register Allocation V2

The native scalar lowering path now performs recursive liveness over
structured control flow, materializes a scalar CFG with successor edges, solves
fixed-point `live_in` / `live_out` sets, builds a scalar-local interference graph
and colors that graph over the current general-purpose resident register pool.
The allocator keeps scalar locals resident across:

- `if` / `else`.
- `while`.
- `do while`.
- classic `for`.
- range `for`.
- `loop`.
- `repeat until`.
- counted repeat loops.
- labeled break/continue loops.

The allocator no longer leaves interprocedural call awareness, vector/SIMD
allocation planning, profile-guided allocation weights or optimizer work beyond
the scalar native-fast lane outside this closure. It still does not claim formal
optimizer equivalence with GCC/LLVM/rustc on arbitrary programs, and
target-specific SIMD instruction selection is closed for the basic x86_64
SSE/AVX and AArch64 NEON operation set, with wider vector operation coverage
left as a separate code-generation expansion. The closed contract is:

- local scalar liveness is collected recursively;
- scalar CFG nodes include explicit successors for branch and loop edges;
- arbitrary successor graphs supplied by the IR liveness layer, including
  irreducible-like multi-entry cycles, are solved by the same fixed-point engine;
- live-in/live-out sets are solved to a fixed point;
- call-containing CFG nodes are recorded explicitly;
- scalar locals that are live across a call are excluded from caller-clobbered
  register residence rather than silently corrupted;
- call-local temporaries can still be register-resident;
- scalar local interference is represented as a real graph;
- graph coloring selects resident registers before spilling;
- copy assignments can reuse the source register when the graph proves the live
  ranges do not interfere;
- single-definition scalar candidates receive internal SSA value records;
- transitive copy chains can be globally coalesced when the interference graph
  proves the group is non-interfering;
- spill-cost scoring is weighted by use count, loop depth, live interval length
  interference degree and optional profile-guidance weight;
- single-definition scalar literals and booleans can be rematerialized at their
  use-site instead of taking a stack spill slot;
- pure non-literal unary/binary expressions can be rematerialized when their
  dependencies are also safe rematerialization candidates;
- vector locals receive a target register allocation plan (`xmm` on x86_64,
  `v` on AArch64) with explicit spills when the vector register window is full;
- basic vector operations have target-specific instruction selection rows for
  x86_64 SSE, x86_64 AVX and AArch64 NEON, including bytes, assembly and disasm
  evidence needles;
- AST algebraic simplification and backend peephole passes run as optimizer
  work outside the scalar native-fast emission path;
- address-taken locals are not promoted;
- scalar locals with supported fixed-width types can be register-resident;
- structured control-flow locals can avoid stack reload/store traffic when they
  do not cross unsafe call boundaries;
- explicit spill/reload still exists when pressure exceeds the register window.

## Atomic RMW Matrix

The backend exposes concrete RMW matrices for:

- `atomicExchange<T>`;
- `atomicFetchAdd<T>`;
- `atomicCompareExchange<T>`.

Supported initial widths are `dword` and `qword`. Orderings are represented as
`relaxed`, `acquire`, `release`, `acq_rel`, and `seq_cst`.

Targets:

- `x86_64`: native x86_64 bytes for the SysV lane.
- `x86_64-pc-windows-msvc`: the same x86_64 ISA byte contract under the Win64
  target profile.
- `aarch64`: exclusive load/store loop bytes for the initial AArch64 lane.

The x86_64/Win64 encoding uses the hardware ordering behavior of locked
instructions and exposes `seq_cst` as an explicit fence boundary in the manifest.
AArch64 uses an exclusive load/store loop and appends a `dmb ish` boundary for
`seq_cst` rows.

Baseical byte lanes:

- `xchg [rdi], rax` for qword exchange: `488707`.
- `lock xadd [rdi], rax` for qword fetch-add: `f0480fc107`.
- `lock cmpxchg [rdi], rsi` for qword compare-exchange: `f0480fb137`.
- AArch64 relaxed qword compare-exchange exclusive loop:
  `037c5fc87f0001eb61000054027c05c885ffff35e00303aa`.
- AArch64 qword compare-exchange `seq_cst` uses `ldaxr/stlxr` plus DMB:
  `03fc5fc87f0001eb6100005402fc05c885ffff35e00303aabf3b03d5`.

The Win64 and AArch64 lanes now have an external host-smoke path:

```sh
ncc backend-model --industrial --host-smoke --json
```

That smoke generates one assembly fixture per Win64/AArch64 matrix row, assembles
it with the external target assembler, checks that the expected byte sequence is
present in the object, disassembles the object with target `objdump`, and links a
target artifact with the external linker. On the WSL host used for this closure
the smoke passed all 60 Win64/AArch64 rows. This is still not a claim of formal
memory-order proof, byte/word coverage, or prolonged CI on every physical host.

## SIMD Instruction Selection

The backend also exposes a basic target SIMD selection matrix:

- x86_64 SSE: `movaps`, `addps`, `mulps`, `pand`, `por`, `pxor`.
- x86_64 AVX: `vmovaps`, `vaddps`, `vmulps`, `vandps`, `vorps`, `vxorps`,
  `vzeroupper`.
- AArch64 NEON: `ldr/str q0`, `fadd/fmul v0.4s`, integer `add v0.4s`, and
  `and/orr/eor v0.16b`.

The external object/disasm smoke is:

```sh
ncc backend-model --industrial --simd-smoke --json
```

On the closure host this smoke assembled, disassembled and linked all 39 SIMD
rows.

## CLI Evidence

The official machine-readable surface is:

```sh
ncc backend-model --industrial --json
```

The report includes:

- `register_allocation.status = cfg_ssa_interprocedural_vector_pgo_optimizer_v3_closed`.
- `simd_instruction_selection.status = vector_expression_lowering_sse_avx_neon_shuffle_mask_gather_scatter_closed`.
- `atomics_volatile.atomic.status = native_x86_64_win64_aarch64_rmw_matrix_closed_host_smoke_available_memory_model_ci_pending`.
- `atomics_volatile.atomic.rmw_matrix[]` with target, op, width, ordering,
  mnemonic, bytes, lock prefix and fence boundary metadata.
- `atomics_volatile.atomic.host_smoke.command = ncc backend-model --industrial --host-smoke --json`.

## Gates

Required focused tests:

```sh
cargo test -p ncc-ir emit_backend_register_allocation_v2_keeps_control_flow_locals_resident
cargo test -p ncc-ir scalar_register_cfg_liveness_v2_tracks_loop_back_edges_and_live_sets
cargo test -p ncc-ir scalar_register_liveness_handles_irreducible_cfg_and_costs
cargo test -p ncc-ir emit_backend_register_allocation_v2_coalesces_non_interfering_local_copy
cargo test -p ncc-ir scalar_register_spill_cost_model_prioritizes_loop_hot_locals
cargo test -p ncc-ir emit_backend_register_allocation_v2_rematerializes_literal_spills
cargo test -p ncc-ir scalar_register_ssa_analysis_tracks_complex_rematerializable_values
cargo test -p ncc-ir scalar_register_global_coalescing_unifies_transitive_copy_chain
cargo test -p ncc-ir emit_backend_register_allocation_v2_rematerializes_complex_expression_spills
cargo test -p ncc-ir backend_atomic_rmw_x86_64_matrix_materializes_lock_prefixed_bytes
cargo test -p ncc-ir backend_atomic_rmw_cross_target_matrices_materialize_win64_and_aarch64_bytes
cargo test -p ncc-cli backend_industrial_manifest_covers_abi_optimizer_ra_object_debug_atomic_multi_host
cargo test -p ncc-cli backend_atomic_host_smoke_plan_covers_win64_and_aarch64_rows
cargo run -q -p ncc-cli -- backend-model --industrial --host-smoke --json
```
