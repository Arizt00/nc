# Basal Evidence

`evidence:` records the proof obligations for a root contract. Evidence can require a hash, report, and seal. It may reference compiler reports or primitive witness data, but it cannot claim physical hardware verification by itself.

When `root.evidence` is `required`, the `evidence:` section is mandatory.

Evidence report shape:

```json
{
  "claim": "contract_only",
  "status": "basal_lowering_deferred_machine",
  "hash": "sha256",
  "no_machine_lowering": true
}
```

Baseical diagnostic: `E_BASAL_EVIDENCE_MISSING_001`.
