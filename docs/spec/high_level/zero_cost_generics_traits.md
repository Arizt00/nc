# Zero-cost generics and traits

Status: implemented for the compiler's declared backend target profiles.

## Contract

- Generic functions are templates only. They cannot reach regular IR.
- Every material call produces one deterministic concrete symbol per tuple of type and const arguments.
- Type parameters, associated type projections and const parameters are substituted before IR lowering.
- Trait-bound calls select a concrete `impl` method and use a direct symbol call.
- Dynamic dispatch is never introduced as a fallback. An explicit dynamic-dispatch surface must remain explicit.
- Kernel lowering rejects hidden dynamic dispatch, allocation, scheduler work and async execution machinery.
- Generic imports from the typed runtime catalog are external intrinsics, not source templates. They
  preserve their resolved type arguments for backend specialization and never synthesize a missing
  local function body.

## Pipeline

1. Parser retains generic parameters, bounds and const generic declarations.
2. Sema infers material arguments, validates bounds, associated types and const ranges.
3. `materialize_zero_cost_program` clones concrete functions, rewrites calls and removes templates.
4. IR receives concrete types and direct call targets only.
5. Normal ABI classification, optimization and instruction selection process each concrete function.

## Target scope

Monomorphization and static dispatch are ISA-independent. The same materialized IR is consumed by every admitted target profile. Intel, AMD, Arm and RISC-V vendor instructions are selected later from declared target features. An undocumented vendor-private extension is not claimed as supported without a public encoding, ABI impact and executable validation corpus.

## Evidence

- Inferred generic materialization.
- Bounded static trait dispatch.
- Associated type substitution.
- Const generic substitution and range diagnostics.
- Kernel hidden-work rejection.
- Runtime generic/import boundary for lists, nested lists and regions.
- Linked qualified `const fn` contracts remain statically verified before materialization, while
  prevalidated per-unit IR emission does not rerun whole-program sema without dependency bodies.

Tests: `cargo test -p ncc-sema monomorphize::tests` and `cargo test -p ncc-ir zero_cost_materialization_ir_tests`.
