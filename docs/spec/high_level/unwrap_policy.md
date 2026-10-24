# High Level Unwrap Policy

Status: closed.

Typed extraction is available in method and helper form:

```n
value = optionValue.unwrap();
value = unwrapOption(optionValue);
value = optionValue.unwrapOr(0);
value = resultValue.unwrap();
value = unwrapOr(resultValue, 0);
error = resultValue.errorOf();
```

`unwrap` preserves `T`; `errorOf` preserves `E`; fallback arguments must unify with `T`. A non-fallback unwrap of `None`/`Err`, or `errorOf` on `Ok`, produces an explicit runtime variant error. `match` remains the branch-safe form:

```n
match (maybeValue()) {
    Some(value) => return @value;
    None => return 0;
}
```

The legacy unqualified spelling `unwrap(optionValue)` is not the Option helper. Use `unwrapOption(optionValue)`, an Option method, or `match`. That legacy ambiguity retains:

- `E_HIGH_UNWRAP_DEFERRED_001`
