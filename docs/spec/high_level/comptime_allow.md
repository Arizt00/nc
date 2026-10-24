# Comptime Allow

`allow` is the authority statement inside a `comptime unsafe` block. It is lexical, local to the block and auditable.

Supported capabilities:

- `filesystem.read`, `filesystem.write`, `filesystem.list`, `filesystem.create`, `filesystem.delete`, `filesystem.metadata`
- `process.spawn`, `process.status`
- `system.env`, `system.user`, `system.path`
- `heap.mutate`
- `reflection.mutate`
- `dynamic.eval`
- `private.metadata`
- `construction.leak`
- `sideEffects`
- `unboundedLoops`
- `unboundedRecursion`

Wildcard families such as `filesystem.*`, `process.*` and `system.*` expand to the matching family. Unknown effects or effects without authority fail with `E_HIGH_COMPTIME_UNDECLARED_EFFECT_001` or the more specific capability diagnostic.
