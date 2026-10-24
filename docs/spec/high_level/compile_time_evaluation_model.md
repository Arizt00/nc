# High Level Compile-time Evaluation Model

High Level Fase 13 defines two separate compile-time lanes.

Pure const eval is deterministic, bounded, side-effect free, cacheable and visibility aware. It evaluates literals, arithmetic, boolean logic, fixed aggregate access, public-safe metadata tokens, exhaustive enum matches and `const fn` calls. It never touches runtime, filesystem, process, system, FFI, machine, heap mutation, reflection mutation, dynamic eval, private metadata, `Any`, construction leaks or `noLimit`.

`comptime unsafe` is the explicit effect lane. It executes only effects declared by `allow`, records an audit entry for each effect, marks reproducibility/cache impact, and requires `comptime unsafe noLimit` for unbounded loop or recursion authority. The two lanes do not share authority: const eval cannot borrow comptime permissions.

Diagnostics are official and phase-scoped. Const eval uses `E_HIGH_CONST_EVAL_*`; comptime unsafe uses `E_HIGH_COMPTIME_*`.
