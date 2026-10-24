# Basal Executable Surface

Basal Fase 6 consumes the executable surface hash as a seal input. It does not reopen the Phase 5.5 official forms for const, routine, raw, verify, probe alias, report, width types, or mem access.

Basal Fase 5.5 officializes executable source forms before final evidence and seal phases.

Baseical sections:

- `const:`
- `binary:`
- `site:`
- `bind:`
- `routine:`
- `raw { ... }`
- `verify:`
- `report:`
- `seal:` reserved for the later seal phase.

Compatibility:

- `probe:` is accepted as a transitional alias and reported as such.
- `fn:` is rejected in basal executable source and must migrate to `routine:`.
- `final:` is not the official executable reporting surface; `report:` is.

Phase 5.5 updates parser, AST, BasalIR, lowering, report output, and NBB0 records. Machine evidence integration and final seal remain not started.
