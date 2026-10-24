# Basal NBB0 Evidence Section

The NBB0 evidence section keeps the existing section kind code `6` for compatibility. Pending NBBs store `EVPN`; integrated Phase 6 NBBs store `EVID` followed by machine evidence and a `SEAL` record.

Allowed states:

- `pending_machine_evidence_integration`
- `integrated_machine_evidence`
- `invalid_machine_evidence`
- `missing_machine_evidence`
- `seal_verified`
- `seal_invalid`

`verify-nbb` accepts pending evidence for structural checks. `verify-seal` requires integrated evidence and a valid seal record.

