# High Level Option

Status: closed in Fase 7.

`Option<T>` is the official high-level type for explicit absence.

Constructors:

```n
Some(value)
None
```

Rules:

- `Option<T>` requires exactly one type argument.
- `Some(value)` infers `T` from its payload and unifies it with any expected `Option<T>`.
- `None` creates an inference hole that is resolved bidirectionally from assignments, projections, function parameters and returns, collection elements, nested ADTs, and if-expression branches.
- `Option<any>` is rejected so absence cannot erase type information.
- Inference holes may cross helper calls and function boundaries, but may not escape unresolved.
- `Option<T>` can be consumed through `match`, `isSome`/`isNone`, `unwrap`/`unwrapOption`, or `unwrapOr`/`unwrapOptionOr`.
- Unwrapping `None` without a fallback is a runtime error; the type returned by an unwrap is always `T`.

Example:

```n
Root {
    maybe :: in(flag >> bool) -> Option<int> {
        if (flag) {
            return Some(1);
        }
        return None;
    }

    read :: in(flag >> bool) -> int {
        match (maybe(flag)) {
            Some(value) => return @value;
            None => return 0;
        }
    }
}
```

Primary diagnostics:

- `E_HIGH_OPTION_ARITY_001`
- `E_HIGH_OPTION_NONE_CONTEXT_001`
- `E_HIGH_OPTION_SOME_TYPE_001`
- `E_HIGH_OPTION_FALLBACK_TYPE_001`
- `E_HIGH_OPTION_UNKNOWN_001` (only for a genuinely non-Option or unresolved escaping value)
- `E_HIGH_ANY_LEAK_001`
