# High Level Generics

Status: Phase 3 closed.

High-level generics are source-level type parameters resolved before later trait/interface/contract dispatch. Phase 3 supports generic functions, generic type applications, runtime generic families, explicit type arguments, and limited inference from value arguments.

Baseical examples:

```n
identity<T> :: in(input >> T) -> T {
    return input;
}

run :: in() -> bool {
    explicit >> int <- identity<int>(42);
    inferred >> int <- identity(7);
    return explicit == inferred;
}
```

Closed surface:

- Type parameters on high-level functions.
- Explicit calls such as `identity<int>(42)`.
- Limited inference from parameters such as `identity(42)`.
- Generic containers: `list<T>`, `vector<T>`, `array<T,N>`, `ptr<T>`, `memRegion<T>`.
- Runtime generic calls such as `listCreate<int>()`, `listAdd`, `listGet`, and `listAsRegion`.

Out of scope for Phase 3:

- Traits/interfaces/contracts dispatch.
- Associated types and specialization.
- Higher-kinded type parameters.
- Complex const generic expressions.
- Pattern matching, closures, async, reflection, and property testing.

Diagnostics are official and must fail closed for duplicate parameters, unknown type params, wrong arity, ambiguous inference, return-only inference, any leaks, and deferred higher-kinded forms.
