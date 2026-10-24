# High Level Risk Partial Match

Partial match authority allows a programmer to declare that a non-exhaustive match shape is intentional in the current high-level surface.

Accepted:

```n
risk:
    allow partialMatch;
    accept partialMatch "knownFiniteSubset";
```

Rejected without authority:

`E_HIGH_RISK_PARTIAL_MATCH_001`

This authority does not weaken existing match exhaustiveness checks elsewhere. It only records an explicit risk item in the Phase15 policy surface.
