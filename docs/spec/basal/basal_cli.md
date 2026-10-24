# Basal CLI

Basal commands:

| Command | Purpose |
|---|---|
| `ncc basal doctor` | report basal surface status |
| `ncc basal doctor --json` | machine-readable status |
| `ncc basal check <file.nb>` | verify basal semantic source |
| `ncc basal check <file.nbb>` | verify basal binary source |
| `ncc basal report <file> --json` | JSON report |
| `ncc basal report <file> --md` | Markdown report |
| `ncc basal parse <file.nb> --json` | Phase 1 parse report |
| `ncc basal ast <file.nb> --json` | Phase 1 AST with source spans |
| `ncc basal tables <file.nb> --json` | Phase 2 root/site/bind table set |
| `ncc basal ir <file.nb> --json` | Phase 3 BasalIR/RootIR/SiteIR/BindIR report |
| `ncc basal verify-ir <file.nb> --json` | Phase 3 semantic verifier report |
| `ncc basal lower <file.nb> --json` | Phase 4 BasalIR to machine lowering plan |
| `ncc basal lowering-plan <file.nb> --json` | Phase 4 lowering plan alias |
| `ncc basal verify-lowering <file.nb> --json` | Phase 4 lowering verifier report |
| `ncc basal surface <file.nb> --json` | Phase 5.5 executable surface report |
| `ncc basal verify-surface <file.nb> --json` | Phase 5.5 executable surface verifier |
| `ncc basal report-plan <file.nb> --json` | Phase 5.5 structured report plan |
| `ncc basal read <file.nb> --json` | AST plus table set plus BasalIR plus lowering plan |
| `ncc basal read <file.nbb> --json` | decode `.nbb` structure |
| `ncc basal emit-nbb <file.nb> --out <file.nbb>` | write structural NBB0 with pending evidence |
| `ncc basal read-nbb <file.nbb> --json` | read NBB0 records |
| `ncc basal inspect-nbb <file.nbb> --json` | inspect NBB0 records |
| `ncc basal verify-nbb <file.nbb> --json` | verify NBB0 structure |
| `ncc basal nbb-info <file.nbb> --json` | summarize NBB0 structure |
| `ncc basal evidence <file.nb> --json` | build real machine evidence report |
| `ncc basal integrate-evidence <file.nb> --out <file.nbb>` | write NBB0 with integrated machine evidence |
| `ncc basal seal <file.nbb> --json` | report final seal state |
| `ncc basal verify-seal <file.nbb> --json` | verify final seal |
| `ncc basal evidence-report <file.nbb> --json` | extract integrated evidence report |
| `ncc basal seal-report <file.nbb> --json` | extract final seal report |
| `ncc basal verify <file.nbb> --json` | verify `.nbb` contract |
| `ncc basal dump <file.nbb> --json` | dump verified binary metadata |
| `ncc basal witness <file> --json` | evidence projection |
| `ncc basal map <file> --json` | contract map projection |

Superseded history: older text tied seal deferral to machine being closed.
Machine is now available, `.nbb` writing is closed, machine evidence
integration is closed, and final seal verification is closed.

Phase 4 update: `basal report <file.nb> --json` and `basal read <file.nb>` include the BasalIR module, semantic verifier report, and lowering plan. These commands do not write `.nbb`, seal basal, or integrate final machine evidence.

Phase 5.5 update: CLI remains the authority for executable surface validation. El IDE asiste. El terminal sentencia.

Phase 7 update: `basal doctor --json` reports
`basal_phase7_release_seal_closed=true` and `basal_closed=true`.
The final CLI matrix is recorded in `docs/spec/basal/basal_cli_final.md` and
`reports/basal/phase7/basal_final_cli_matrix.md`.
