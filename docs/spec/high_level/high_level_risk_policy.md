# High Level Risk Policy

Risk policy is a high-level declaration surface for explicitly authorized risk. It is intentionally declarative. A risk item cannot bypass type checking, lowering, macro hygiene, const-eval purity, or runtime/compiler diagnostics.

Baseical grammar:

```n
risk:
    allow <capability>;
    accept <riskKind> "optional detail";
    bridge <target> reason "why" authority high.risk.allowUnsafeBridge;
    report <name>;
```

Rules:

- `allow` declares authority.
- `accept` consumes authority for one risk kind.
- `bridge` requires both `allow unsafeBridge` and an authority clause.
- `report` names an audit report entry.
- aliases `policy:` and `authority:` parse to the same AST node.
- missing authority is a source error.
- unsupported or impossible safety claims remain source errors.

Example:

```n
phase15RiskPolicyBasic {
    risk:
        allow race;
        accept dataRace "sharedCounter";
        report riskPolicyBasic;

    run :: in() -> bool {
        return true;
    }
}
```

Risk policy is high-level only. Machine, primitive, system, kernel, and root contexts do not gain this surface.
