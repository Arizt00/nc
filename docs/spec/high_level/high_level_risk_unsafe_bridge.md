# High Level Risk Unsafe Bridge

Unsafe bridge authority is strict because it crosses a high-level boundary.

Accepted:

```n
risk:
    allow unsafeBridge;
    bridge machine reason "declared machine bridge" authority high.risk.allowUnsafeBridge;
```

Rules:

- `allow unsafeBridge` is required.
- `reason` is required.
- `authority high.risk.allowUnsafeBridge` is required.
- target must be known.
- bridge report must record target, reason, authority, and status.

Diagnostic:

`E_HIGH_RISK_BRIDGE_AUTHORITY_001`

Known bridge targets include runtime, compiler, inspect, ai, system, machine, primitive, basal, root, and kernel. Fase 15 records the bridge authority; it does not merge those domains.
