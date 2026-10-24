# High Level Generic Types

Phase 3 closes generic type applications already present in the high-level type system and validates them through function resolution.

Closed families:

- `list<T>`
- `vector<T>` and `T[]`
- `array<T,N>`
- `ptr<T>`
- `memRegion<T>`
- User type contracts with declared generic arity.

Nested type applications are supported where the underlying family supports them:

```n
@nested >> list<list<int>> : listCreate<list<int>>();
```

Unknown generic bases fail with `E_HIGH_GENERIC_UNKNOWN_BASE_001`. A generic parameter used as a type constructor is deferred and rejected with `E_HIGH_GENERIC_HKT_DEFERRED_001`.
