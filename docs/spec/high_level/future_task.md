# Future<T> And Task<T>

Status: closed. The Phase 9 heading is historical.

`Future<T>` is the semantic result of calling an async function. `Task<T>` is the semantic handle produced by spawning a future.

```n
@future >> Future<int> : compute();
@task >> Task<int> : spawn @future;
@value >> int : join @task;
```

Rules:

- `Future<T>` requires exactly one non-`any` type argument.
- `Task<T>` requires exactly one non-`any` type argument.
- `spawn` accepts only `Future<T>`.
- `join` accepts only `Task<T>`.
- `Future<T>` uses the native coroutine descriptor and executable resume state machine.
- scheduling remains explicit; awaiting does not introduce an invisible parallel executor.
- kernel/freestanding profiles use caller-owned frames and reject hidden scheduler or heap work.

Diagnostics:

- `E_HIGH_ASYNC_FN_001`.
- `E_HIGH_SPAWN_NON_ASYNC_001`.
- `E_HIGH_JOIN_NON_TASK_001`.
- `E_HIGH_ANY_LEAK_001`.
