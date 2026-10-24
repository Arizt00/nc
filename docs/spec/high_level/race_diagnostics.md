# Race Diagnostics

Status: closed for Fase 9 semantic detection.

Fase 9 detects obvious shared mutable writes across spawned tasks. A function marked with `effects(state.write)` is considered a writer. If multiple spawned calls write the same first context argument, the checker emits a race diagnostic.

```n
@a >> Task<bool> : spawn writeShared(@shared);
@b >> Task<bool> : spawn writeShared(@shared);
```

Diagnostic:

- `E_HIGH_RACE_SHARED_MUT_001`.

Alias-sensitive deep borrow analysis is not claimed as complete runtime data-race proof. Fase 9 provides deterministic source diagnostics for the closed high-level surface.
