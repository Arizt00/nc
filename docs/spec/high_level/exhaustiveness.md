# High Level Match Exhaustiveness

Status: closed in Fase 6.

Exhaustiveness is checked over the closed set of variants in the scrutinee enum.

Rules:

- Every variant must have an unguarded arm, or the match must contain `_`.
- Guarded variant arms are checked but do not count as exhaustive coverage.
- Duplicate unguarded arms are unreachable and rejected.
- Arms after `_` are unreachable and rejected.
- Empty pattern matches are rejected.

Diagnostics:

- `E_HIGH_MATCH_EMPTY_001`
- `E_HIGH_MATCH_NON_EXHAUSTIVE_001`
- `E_HIGH_MATCH_UNREACHABLE_ARM_001`
- `E_HIGH_MATCH_ARM_TYPE_001`
