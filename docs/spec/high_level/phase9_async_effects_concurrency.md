# High Level Fase 9 - Async / Effects / Concurrency

Status: closed.

Fase 9 closes the high-level async/effects/concurrency surface on top of the Fase 8 iterator/range/collection baseline. It introduces semantic checking for async functions, `Future<T>`, `Task<T>`, `await`, `spawn`, `join`, bounded channels, declared effects, and shared mutable race diagnostics.

Fase 9 does not implement Fase 10+ modules/packages/public API hardening. Closures/lambdas, higher-kinded types, async streams, yield generators, and runtime-level parallel scheduling remain outside this closure.

Baseical example:

```n
phase9AsyncBasic {
    async compute() -> int {
        return 42;
    }

    run() -> bool
    effects(async.await)
    {
        public:
            future >> Future<int> <- compute();
            value >> int <- await future;
            return value == 42;
    }
}
```

Closed components:

- async function declarations.
- async call return projection to `Future<T>`.
- `await Future<T> -> T`.
- `spawn Future<T> -> Task<T>`.
- `join Task<T> -> T`.
- declared effects and effect propagation checks.
- bounded `channel<T,N>` with explicit backpressure send operations.
- race diagnostics for shared mutable writes across spawned tasks.
- semantic async runtime model documentation.

Deferred components:

- async streams.
- yield-based generators.
- closures/lambdas.
- runtime scheduler implementation.
- task cancellation and timeout execution semantics.
- Fase 10 modules/packages/public API surface hardening.
