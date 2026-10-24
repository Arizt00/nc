# TypeInfo

`TypeInfo<T>` describes a visible, concrete high-level type. It supports primitives, declared type contracts, enums, visible trait/interface/contract surfaces, and concrete generic forms such as `Option<int>` or `Result<int, str>`.

Examples:

```n
@info >> TypeInfo<User> : typeInfo<User>();
return typeName(@info) == "User";
```

`any` is rejected. Unresolved type parameters are rejected until Fase 13 or later defines compile-time evaluation rules.
