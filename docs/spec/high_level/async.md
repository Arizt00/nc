# High Level Async

Status: closed for Fase 9.

`async` is function syntax, not a callable value. An async function keeps its declared body return type, but a call to that function is typed as `Future<T>`.

```n
async load :: in() -> int {
    return 7;
}

run :: in() -> bool effects(async.await) {
    future >> Future<int> <- load();
    value >> int <- await future;
    return value == 7;
}
```

Rules:

- `async` is only accepted as a function modifier.
- `async()` as a call is rejected with `E_HIGH_ASYNC_FN_001`.
- async functions preserve normal return checking inside the function body.
- async return mismatches surface as `E_HIGH_ASYNC_RETURN_001`.
- `AsyncStream<T>` is accepted only on async functions or closures and lowers to the typed coroutine descriptor ABI;
- `await` is a poll/state transition and never an eager payload extraction;
- portable, kernel, freestanding and sovereign profiles require caller-owned frames and introduce no implicit scheduler.
