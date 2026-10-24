# High Level Generic Lowering

Phase 3 closes the high-level lowering strategy for generics without starting Phase 4 dispatch.

Strategy:

- Generic functions resolve to concrete call signatures at semantic analysis time.
- Explicit type arguments bind the signature directly.
- Inferred type arguments are derived from argument types.
- The resolved call signature is the monomorphization boundary for later compiler stages.
- Runtime generic calls lower through the existing runtime generic signature catalog.

Not implemented in Phase 3:

- Trait/interface/contract based dispatch.
- Associated type lowering.
- Specialization.
- Higher-kinded lowering.

This is not a fake lowering marker: calls must resolve to concrete semantic signatures or fail.
