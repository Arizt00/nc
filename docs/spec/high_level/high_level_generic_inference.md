# High Level Generic Inference

Phase 3 inference is intentionally limited and deterministic.

Inference sources:

- Function arguments.
- Nested generic arguments in `list<T>`, `array<T,N>`, `ptr<T>`, and `memRegion<T>`.
- Runtime generic signatures.

Inference does not use:

- Return type alone.
- Expected projection type alone.
- Trait/interface constraints.
- Higher-kinded type constructors.

Required failures:

- `E_HIGH_GENERIC_INFER_AMBIGUOUS_001` for conflicting argument inference.
- `E_HIGH_GENERIC_INFER_RETURN_ONLY_001` for return-only type parameters.
- `E_HIGH_GENERIC_ANY_LEAK_001` if inference would settle on `any`.
