# High Level Collection Protocols

`Collection<T>` is the official protocol for finite, indexable collection surfaces.

Closed Fase 8 operations:

```n
@items.len()
@items.isEmpty()
@items.get(0)
@items.iter()
```

Rules:

- `Collection` requires exactly one type argument.
- `Collection<any>` is rejected.
- `len()` and `isEmpty()` take no arguments.
- `get(index)` requires an integer-like index.
- `iter()` returns `Iterator<T>`.
- mutation during `for-in` is rejected when the iterated symbol is structurally changed in the loop body.

The existing list, array/vector, and `memRegion<T>` surfaces participate in the Fase 8 iteration model without changing their runtime ownership or memory semantics.
