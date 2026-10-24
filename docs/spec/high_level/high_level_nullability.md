# High Level Nullability

Status: closed.

Nullability is explicit and uses union syntax:

```n
maybe: ptr<byte> | null;
@maybe >> ptr<byte> | null : null;
```

Rules:

- `null` is rejected for non-nullable scalar targets.
- `null` may project into pointer-like unions that include `null`.
- `void` returns cannot silently return `null`; the existing `discardNull(nullable)` contract is required.
- nullable values cannot be used as normal indexed values without a refinement surface.

Invalid examples:

```n
@value >> int : null;
@value >> byte : @maybe[0]; // maybe: ptr<byte> | null
```

Refinement syntax beyond strict checks remains outside Fase 2.
