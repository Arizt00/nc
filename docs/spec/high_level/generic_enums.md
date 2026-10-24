# High Level Generic Enums

Status: closed_basic in Fase 6.

Generic enum declarations support simple type parameters and payload substitution.

```n
enum:
    Pair<T> {
        Both(left: T, right: T);
        Empty;
    }
```

Constructor inference binds generic parameters from payload values when possible:

```n
@pair >> Pair<int> : Pair.Both(4, 6);
```

Deferred:

- Higher-kinded enum parameters.
- Generic associated types.
- Enum type parameter bounds.
- Full `Result` / `Option` error algebra.

Invalid bounds produce `E_HIGH_ENUM_PARSE_001`.
