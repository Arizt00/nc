# Basal Lowering Evidence Boundary

Phase 4 creates an evidence boundary but does not close machine evidence.

The boundary status is:

```text
pending_phase6_machine_evidence_integration
```

This means:

- machine is available and closed;
- lowering can name target profiles and symbols;
- no final evidence bundle is attached to basal yet;
- no basal seal is produced;
- no `.nbb` writer is active.

Any `.nb` source that claims final machine evidence in Phase 4 fails with `E_BASAL_LOWERING_FAKE_EVIDENCE_001`.
