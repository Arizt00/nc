# Basal Conformance

Basal conformance is closed at Phase 7. The language source, tables, IR,
lowering, NBB0 binary container, executable surface, machine evidence, final
seal, and release evidence bundle are all covered by explicit reports.

Machine remains the closed backend foundation. Basal conformance requires
`asm_complete_total=true` from `ncc machine asm-complete native-doctor --json`.

| Phase | Area | Required Status |
|---|---|---|
| Phase 1 | parser, AST, spans, diagnostics, recovery | closed |
| Phase 2 | root table, site table, bind table, table diagnostics, table spans | closed |
| Phase 3 | BasalIR, RootIR, SiteIR, BindIR, semantic verifier | closed |
| Phase 4 | lowering, machine ref resolution, lowering reports, pending evidence boundaries | closed |
| Phase 5 | NBB0 header, section table, records, string table, pending evidence, hashes, writer, reader, verifier | closed |
| Phase 5.5 | const, routine, raw, verify, probe alias, report, width types, mem access, terminal authority, ExecutableSurface NBB section | closed |
| Phase 6 | machine evidence integration, doctor/native evidence, object/verify/disasm/roundtrip evidence, evidence hash chain, final seal, verify-seal | closed |
| Phase 7 | release seal, conformance matrix, final evidence bundle, worktree classification | closed |

The final doctor state is:

- `basal_phase7_release_seal_closed = true`
- `basal_closed = true`
- `machine_dependency = available_closed`
- `machine_evidence_integration = closed`
- `seal_final = closed`

Basal release conformance does not reopen high-level, runtime, primitive, or
machine semantics. Dirty worktree entries outside the basal release surface are
classified as release debt rather than hidden or deleted.
