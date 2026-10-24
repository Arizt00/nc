# High Level Effects

Status: closed for Fase 9.

Effects are explicit function annotations used by the semantic checker. They describe required authority and concurrency behavior without importing runtime semantics.

```n
load :: in() -> Result<int, str> effects(io.read) {
    return Ok(1);
}

run :: in() -> bool effects(io.read) {
    result >> Result<int, str> <- load();
    return isOk(result);
}
```

Allowed Fase 9 effects:

- `pure`.
- `io.read`, `io.write`.
- `fs.read`, `fs.write`.
- `net.read`, `net.write`.
- `async.await`.
- `task.spawn`.
- `channel.send`, `channel.recv`.
- `state.read`, `state.write`.
- `unsafe.raw`.

Rules:

- callers must declare the effects required by called functions and intrinsic operations.
- `pure` cannot be combined with additional effects.
- pure functions cannot call effectful functions.
- unknown effects are rejected.
- effect erasure is rejected.

Diagnostics:

- `E_HIGH_EFFECT_UNKNOWN_001`.
- `E_HIGH_EFFECT_MISSING_001`.
- `E_HIGH_EFFECT_NOT_ALLOWED_001`.
- `E_HIGH_EFFECT_PURE_VIOLATION_001`.
- `E_HIGH_EFFECT_ERASURE_001`.
