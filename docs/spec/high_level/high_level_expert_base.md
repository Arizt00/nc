# High Level Expert Base

Status: historical Phase 0 audit baseline. It defines policy, not current implementation status.

## Base

The advanced/expert high-level compiler is not a moral or architectural babysitter. It exposes the power of the language when the corresponding profile and capability declarations make that power explicit.

Normative rules:

- Dangerous and declared is permitted.
- Impossible and undeclared is an error.
- Structural false success is prohibited.
- Explicit contracts are validated.
- Impossible lowering fails.
- Risk acceptance can generate evidence and audit records.
- The expert programmer owns the consequences of accepted risk.
- The compiler does not reject an architecture merely because it is unusual, partial, or sharp.
- The compiler does reject contradictions in syntax, symbols, types, capabilities, contracts, lowering, or reports.

## Expert Capabilities To Phase In

These names are reserved for future phases and are not implemented by Phase 0:

- `allowRace`
- `allowPartialMatch`
- `allowNoRollback`
- `allowUncheckedIndex`
- `allowUnboundedStream`
- `allowRawAccess`
- `allowLowLevelBridge`
- `allowUnsafeCast`
- `allowManualLifetime`
- `allowPatchStacking`

## Audit Baseline

Phase 0 found a working baseline and recorded the debt visible at that date. Current status for generics, traits, pattern matching and async/streams is defined by the dedicated current specifications and resolution reports; this snapshot must not be used to reopen those completed points.

Phase 0 does not convert that debt into implementation. It freezes the map.
