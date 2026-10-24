# High-Level Reflection

High-level reflection in Fase 12 is limited to safe metadata queries. Reflection is read-only, visibility-aware, and diagnostic-driven.

Allowed:

- Query a visible type through `typeInfo<T>()`.
- Query a visible function through `symbolInfo(@fn(name))`.
- Query closure signature metadata through `closureInfo(value)`.

Forbidden:

- Dynamic evaluation.
- Calling symbols by string.
- Mutating fields or runtime state through metadata.
- Reading private/internal fields outside their visibility scope.
- Reflecting machine or basal internals through high-level metadata.
