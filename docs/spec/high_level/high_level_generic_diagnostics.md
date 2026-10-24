# High Level Generic Diagnostics

Baseical diagnostics:

- `E_HIGH_GENERIC_DUP_PARAM_001`
- `E_HIGH_GENERIC_UNKNOWN_PARAM_001`
- `E_HIGH_GENERIC_ARITY_001`
- `E_HIGH_GENERIC_UNKNOWN_BASE_001`
- `E_HIGH_GENERIC_TYPE_MISMATCH_001`
- `E_HIGH_GENERIC_INFER_AMBIGUOUS_001`
- `E_HIGH_GENERIC_INFER_RETURN_ONLY_001`
- `E_HIGH_GENERIC_ANY_LEAK_001`
- `E_HIGH_GENERIC_HKT_DEFERRED_001`

Policy:

- Source errors exit non-zero.
- Generic ambiguity never silently becomes `any`.
- Unknown generic bases are rejected.
- Higher-kinded forms are explicitly deferred, not accepted as placeholders.
- Runtime generic mismatches retain runtime diagnostics when the failing surface is runtime-owned.
