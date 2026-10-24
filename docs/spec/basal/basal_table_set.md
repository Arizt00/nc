# Basal Table Set

Status: Basal Fase 2 closed.

`ncc basal tables <file.nb> --json` emits a `BasalTableSet` containing:

- `root_table`
- `site_table`
- `bind_table`
- top-level diagnostics
- `phase: phase2_tables`
- `lowering: not_started`
- `basal_ir: not_started`
- `nbb_writer: not_started`
- `evidence_integration: not_started`

`basal report --json` and `basal read --json` include the same table set for `.nb` files.
