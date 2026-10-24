# High Level Concurrency

Status: closed for Fase 9.

Fase 9 defines semantic concurrency through `spawn`, `join`, and task handles. It validates types, declared effects, and obvious shared mutable races. It does not claim a concrete runtime scheduler.

```n
async compute :: in() -> int {
    return 9;
}

run :: in() -> bool effects(task.spawn, async.await) {
    task >> Task<int> <- spawn compute();
    value >> int <- join task;
    return value == 9;
}
```

Rules:

- `spawn` requires a future-producing expression and `effects(task.spawn)`.
- `join` requires a task handle and `effects(async.await)`.
- shared mutable writes from multiple spawned tasks are diagnosed.
- synchronization primitives are semantic boundaries for later runtime work, not Fase 9 runtime features.

Diagnostics:

- `E_HIGH_SPAWN_NON_ASYNC_001`.
- `E_HIGH_JOIN_NON_TASK_001`.
- `E_HIGH_RACE_SHARED_MUT_001`.
- `E_HIGH_SYNC_REQUIRED_001`.
