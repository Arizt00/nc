# High Level Fase 9 Diagnostics

> Historical diagnostic inventory. `E_HIGH_OBSOLETE_CALLABLE_SPELLING_001` applies
> only to obsolete callable spellings such as `closure(...)`, `lambda(...)`
> and `asyncClosure(...)`; typed closure literals are implemented and
> executable. The generated language manifest is the current authority.

Status: closed.

Baseical diagnostics:

- `E_HIGH_ASYNC_FN_001`: `async` used as a callable or invalid async syntax.
- `E_HIGH_AWAIT_PARSE_001`: invalid await arity.
- `E_HIGH_AWAIT_NON_FUTURE_001`: await operand is not `Future<T>`.
- `E_HIGH_AWAIT_CONTEXT_001`: missing `effects(async.await)`.
- `E_HIGH_ASYNC_RETURN_001`: async function body return mismatch.
- `E_HIGH_SPAWN_NON_ASYNC_001`: spawn operand is not future-producing.
- `E_HIGH_JOIN_NON_TASK_001`: join operand is not `Task<T>`.
- `E_HIGH_EFFECT_UNKNOWN_001`: unknown declared effect.
- `E_HIGH_EFFECT_MISSING_001`: caller missing required effect.
- `E_HIGH_EFFECT_NOT_ALLOWED_001`: invalid effect combination.
- `E_HIGH_EFFECT_PURE_VIOLATION_001`: pure function requires an effect.
- `E_HIGH_EFFECT_ERASURE_001`: effect erasure attempt.
- `E_HIGH_CHANNEL_UNBOUNDED_001`: channel lacks bounded capacity.
- `E_HIGH_CHANNEL_CAPACITY_001`: invalid channel capacity.
- `E_HIGH_CHANNEL_ITEM_ANY_001`: `channel<any,N>` leak.
- `E_HIGH_CHANNEL_BACKPRESSURE_001`: plain send without policy.
- `E_HIGH_CHANNEL_SEND_001`: invalid send shape or type.
- `E_HIGH_CHANNEL_RECV_001`: invalid receive shape or type.
- `E_HIGH_RACE_SHARED_MUT_001`: shared mutable write race.
- `E_HIGH_ASYNC_STREAM_CONTEXT_001`: `AsyncStream<T>` requires an async callable.
- `E_HIGH_YIELD_CONTEXT_001`: `yield` used outside a generator or stream.
- `E_HIGH_YIELD_TYPE_001`: yielded value does not match the declared item type.
- `E_HIGH_OBSOLETE_CALLABLE_SPELLING_001`: obsolete `closure(...)`, `lambda(...)` or
  `asyncClosure(...)` callable spelling.
- `E_HIGH_ANY_LEAK_001`: `any` leak through async/channel surface.
