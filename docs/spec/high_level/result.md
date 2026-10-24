# High Level Result

Status: closed in Fase 7.

`Result<T, E>` is the official high-level type for explicit success or recoverable error.

Constructors:

```n
Ok(value)
Err(error)
```

Rules:

- `Result<T, E>` requires exactly two type arguments.
- `Ok(value)` infers or checks the success type `T`; the error type `E` is resolved from context.
- `Err(error)` infers or checks the error type `E`; the success type `T` is resolved from context.
- Context flows through assignments, projections, arguments, returns, collections, nested ADTs, helpers, and if-expression branches.
- `Result<any, E>` and `Result<T, any>` are rejected.
- Error transport is explicit and branch-based.
- `isOk`, `isErr`, `unwrap`, `unwrapOr`, and `errorOf` preserve the concrete `T` and `E` payload types in helper and method form.
- Unwrapping `Err` without a fallback, or requesting `errorOf` from `Ok`, is a runtime error.
- `result?` and `try result` yield `T` for `Ok(T)` and return the original `Err(E)` from a compatible `Result<U,E>` function.
- Every active `defer` runs in reverse registration order before propagated and explicit returns.
- Exceptions are not part of the high-level error model.

Example:

```n
Root {
    enum:
        LoadError {
            Missing;
            Invalid;
        }

    load :: in(flag >> bool) -> Result<int, LoadError> {
        if (flag) {
            return Ok(42);
        }
        return Err(LoadError.Missing);
    }
}
```

Primary diagnostics:

- `E_HIGH_RESULT_ARITY_001`
- `E_HIGH_RESULT_OK_TYPE_001`
- `E_HIGH_RESULT_ERR_TYPE_001`
- `E_HIGH_RESULT_FALLBACK_TYPE_001`
- `E_HIGH_RESULT_ERROR_MISMATCH_001`
- `E_HIGH_RESULT_ERROR_ANY_001`
- `E_HIGH_ERROR_ERASURE_001`
