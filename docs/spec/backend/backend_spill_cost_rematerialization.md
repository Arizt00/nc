# Backend Spill-Cost Modeling / Rematerialization

Status: closed for register allocation V3 spill-cost, rematerialization, call awareness and profile weights.

This document records the native backend closure for spill-cost modeling and
rematerialization in the current native backend path.

## Scope

The closed scope is the scalar/native register-allocation V3 lane. It
does not change nC language semantics, parser syntax, public IR contracts, ABI
contracts, object formats or linker behavior.

The liveness and cost engine is not limited to a single syntactic structured
shape. Structured statements feed it today, but the solver itself accepts
arbitrary successor graphs supplied by the IR liveness layer, including
irreducible-like multi-entry cycles. Those graph facts feed the same
interference, global copy coalescing and spill-cost model.

Call-containing CFG nodes are explicit. A local that is live-in and live-out
across such a node is recorded as call-crossing and is not assigned to the
current caller-clobbered resident register pool. Call-local temporaries remain
eligible, so functions with calls no longer disable the whole allocator.

## SSA Value Records

The allocator now builds internal SSA value records for every single-definition
scalar candidate. Each record includes:

- official symbol;
- SSA version (`0` for the closed single-definition lane);
- defining expression;
- dependency symbols;
- purity flag;
- rematerialization eligibility.

Multi-definition variables remain represented by CFG liveness and interference;
they are not falsely promoted into this SSA lane without a phi-compatible model.

## Global Copy Coalescing

Copy preferences are no longer only local ordering hints. The allocator forms
transitive copy groups and validates each candidate group against the
interference graph. A group is coalesced only when no pair in the group
interferes. When a group receives a resident register, compatible peers are
assigned to the same register, reducing redundant moves across the whole scalar
function.

## Spill-Cost Model

Each candidate scalar local receives a cost record:

- weighted use count;
- loop-weighted use count;
- profile-guidance weight;
- live interval length;
- interference degree;
- rematerializable flag;
- final spill score.

Uses inside structured loops are weighted more heavily than straight-line uses.
The allocator sorts pressure candidates by score before assigning caller-saved
resident registers. Hot loop locals therefore remain preferred over cold scalar
literals when the function has more live locals than available registers.
Profile-guided allocation can deliberately promote a profiled symbol above the
static loop score when the caller supplies a profile weight.

## Rematerialization

The V2 allocator can rematerialize a scalar local when all of these are true:

- it has exactly one definition;
- its address is not taken;
- its definition is a numeric literal, boolean literal, parameter-derived symbol
  or pure unary/binary expression over rematerializable scalar values;
- its type is one of the scalar register-resident fixed-width types.

Division and modulo are deliberately excluded from expression rematerialization
because moving them can change fault timing. Calls, memory operations, address
taking, pointer arithmetic and array operations are also excluded.

When register pressure prevents assigning a register, eligible locals are
recorded as rematerialized rather than spilled. The scalar emitter then skips the
original stack store and re-emits the pure expression at each use-site.

## Not Claimed

- Rematerialization of division, modulo, calls, memory reads, pointer
  expressions or address-taken values.
- Formal optimizer equivalence with GCC/LLVM/rustc on arbitrary programs.
- Target-specific SIMD instruction selection beyond the basic closed x86_64
  SSE/AVX and AArch64 NEON operation set.

## Vector/SIMD Register Plan

Vector locals are classified separately from scalar locals. The allocator builds
a vector plan with `xmm` registers for x86_64 and `v` registers for AArch64, and
records explicit vector spills when the target register window is full. Basic
SSE/AVX/NEON target instruction selection is now attached to that plan through a
byte-level SIMD matrix and `ncc backend-model --industrial --simd-smoke --json`.
This is still bounded to the closed basic operation set, not every possible
future vector expression.

## Evidence

Focused tests:

```sh
cargo test -p ncc-ir scalar_register_spill_cost_model_prioritizes_loop_hot_locals
cargo test -p ncc-ir emit_backend_register_allocation_v2_rematerializes_literal_spills
cargo test -p ncc-ir scalar_register_liveness_handles_irreducible_cfg_and_costs
cargo test -p ncc-ir scalar_register_ssa_analysis_tracks_complex_rematerializable_values
cargo test -p ncc-ir scalar_register_global_coalescing_unifies_transitive_copy_chain
cargo test -p ncc-ir emit_backend_register_allocation_v2_rematerializes_complex_expression_spills
cargo test -p ncc-ir scalar_register_allocation_v2_is_call_aware_interprocedural
cargo test -p ncc-ir vector_simd_register_allocator_assigns_vector_locals
cargo test -p ncc-ir simd_instruction_selection_matrix_has_real_target_bytes
cargo test -p ncc-cli backend_industrial_manifest_covers_abi_optimizer_ra_object_debug_atomic_multi_host
cargo test -p ncc-cli backend_simd_host_smoke_plan_covers_sse_avx_and_neon_rows
```

Manifest status:

```text
cfg_ssa_interprocedural_vector_pgo_optimizer_v3_closed
```
