# High Level Expert Profile Gates

Status: documented_only for High Level Fase 1.

The expert base is intentionally permissive, but dangerous behavior must be explicit. Fase 1 documents gates for future phases and does not implement the underlying capabilities.

## Gate List

| Gate | Status | Future purpose |
| --- | --- | --- |
| `allowRace` | deferred_to_future_phase | Declared data-race acceptance. |
| `allowPartialMatch` | deferred_to_future_phase | Partial match acceptance. |
| `allowNoRollback` | deferred_to_future_phase | Transaction/no-rollback acceptance. |
| `allowUncheckedIndex` | deferred_to_future_phase | Unchecked indexing acceptance. |
| `allowUnboundedStream` | deferred_to_future_phase | Stream without backpressure acceptance. |
| `allowRawAccess` | deferred_to_future_phase | Raw access acceptance. |
| `allowLowLevelBridge` | deferred_to_future_phase | Bridge to lower layers. |
| `allowUnsafeCast` | deferred_to_future_phase | Unsafe cast acceptance. |
| `allowManualLifetime` | deferred_to_future_phase | Manual lifetime acceptance. |
| `allowPatchStacking` | deferred_to_future_phase | Patch-over-patch acceptance. |

## Fase 1 Rule

The gates are `documented_only`. A later phase may add parser/manifest recognition, but Fase 1 must not unlock new dangerous behavior.

Accepted risk is legal only when a future implementation records the explicit gate, source span, diagnostic/evidence entry, and profile that allowed it.
