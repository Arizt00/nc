# High Level Risk No Rollback

Rollback risk covers single-phase operations and partial commit behavior.

Accepted:

```n
risk:
    allow noRollback;
    accept noRollback "singlePhaseCommit";
```

Partial commit authority is separate:

```n
risk:
    allow transactionPartialCommit;
    accept transactionPartialCommit "auditTrail";
```

Diagnostics:

- `E_HIGH_RISK_NO_ROLLBACK_001`
- `E_HIGH_RISK_PARTIAL_COMMIT_001`

Fase 15 records the policy. It does not introduce transaction machinery.
