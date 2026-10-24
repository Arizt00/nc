# High Level Risk Data Races

Data race authority requires `allow race`.

Accepted:

```n
risk:
    allow race;
    accept dataRace "sharedCounter";
```

Rejected:

```n
risk:
    accept dataRace "sharedCounter";
```

Diagnostic:

`E_HIGH_RISK_RACE_REQUIRES_ALLOW_001`

The authority does not prove absence of a race. It records that the risk is intentionally accepted and must appear in risk reports.
