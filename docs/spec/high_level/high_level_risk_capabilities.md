# High Level Risk Capabilities

Capabilities are explicit authority tokens. They are not permissions to skip validation; they are evidence that the programmer accepted a named risk and wants it visible in reports.

Closed capability families:

- `race` for data race authority.
- `aliasMutation` for alias mutation authority.
- `streamNoBackpressure` and `channelNoBackpressure`.
- `noRollback` and `transactionPartialCommit`.
- `partialMatch`.
- `uncheckedUnwrap` and `uncheckedCast`.
- `effectErasure`.
- `privateMetadataAccess`, `anyLeak`, `anyConstruction`, `constructionLeak`.
- `rawPtr`, `rawAddr`, `rawSite`.
- `unsafeBridge`.
- `comptimeUnsafe` and `unsafeMacro`.
- `noLimit`, `unboundedLoop`, `unboundedRecursion`.
- `lowLevelPatch` and `fragileDesign`.

Capabilities may be written with high-level authority prefixes:

```n
risk:
    allow high.risk.allowUnsafeBridge;
    bridge machine reason "declared machine bridge" authority high.risk.allowUnsafeBridge;
```

The semantic checker normalizes spelling and records the original span. Unknown capability names fail with `E_HIGH_RISK_CAPABILITY_REQUIRED_001`.
