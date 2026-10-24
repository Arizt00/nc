# Await

Status: closed for Fase 9.

`await` unwraps a `Future<T>` to `T`.

```n
@future >> Future<int> : load();
@value >> int : await @future;
```

Rules:

- `await` requires exactly one expression.
- the expression must type as `Future<T>`.
- a function using `await` must declare `effects(async.await)`.
- awaiting a non-future is rejected.

Diagnostics:

- `E_HIGH_AWAIT_PARSE_001`.
- `E_HIGH_AWAIT_NON_FUTURE_001`.
- `E_HIGH_AWAIT_CONTEXT_001`.
