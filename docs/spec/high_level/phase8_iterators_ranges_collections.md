# High Level Fase 8 - Iterators / Ranges / Collection Protocols

Status: historical Fase 8 boundary. Current iterator/generator authority is `iterators_generators_async.md`.

Fase 8 originally closed the finite source-level surface without opening Fase 9. Later phases have since closed typed closures, adapters, generators, streams and executable coroutine state machines.

Closed:

- `Iterator<T>` as the official pull protocol.
- `Iterable<T>` and `IntoIterator<T>` as iteration-entry protocols.
- `Collection<T>` for `len`, `isEmpty`, `get`, and `iter`.
- `for item in value { ... }` over list, array/vector, and `memRegion<T>` surfaces.
- finite integer ranges using `start..end`, `start..=end`, and optional non-zero integer `step`.
- type-level `next() -> Option<T>`.
- diagnostics for wrong protocol arity, any leaks, bad ranges and mutation during `for-in`.

Historical exclusions, now superseded where noted by the current specification:

- closures/lambdas and typed map/filter/fold adapters: closed.
- generators/streams/async state machines: closed.
- infinite or unbounded streams.
- complete specialization or higher-kinded iterator families.

Fase 8 does not change Machine or Basal semantics. Machine remains the closed dependency, and Basal remains closed over Machine.
