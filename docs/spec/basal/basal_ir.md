# BasalIR

Status: closed for Basal Fase 3.

BasalIR is the semantic internal representation built from the Phase 1 `.nb` AST and the Phase 2 root/site/bind tables.

`BasalIrModule` contains:

- `BasalRootIr` entries.
- `BasalSiteIr` entries.
- `BasalBindIr` entries.
- modeled seal requirements.
- verifier diagnostics.
- source span mappings back to `.nb`.

BasalIR is contract-only in Phase 3. It does not lower to `[@machine]`, does not write `.nbb`, and does not integrate machine evidence.

Phase 5.5 extension: BasalIR now also carries `BasalConstIr`, `BasalRoutineIr`, `BasalRawBlockIr`, `BasalVerifyIr`, `BasalReportIr`, `BasalMemAccessIr`, and `BasalWidthType` records. These records model executable surface intent and keep final evidence pending.
