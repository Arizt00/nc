# Basal Fase 2 Scope

Status: closed.

Basal Fase 2 closes root, site, and bind tables over the `.nb` AST closed in Basal Fase 1.

In scope:

- root table construction and diagnostics.
- site table construction, permission normalization, and diagnostics.
- bind table construction, syntax-level local target checks, and pending machine resolution markers.
- `basal tables <file.nb> --json`.
- table output in `basal report --json` and `basal read --json`.

Out of scope:

- BasalIR / RootIR.
- basal to `[@machine]` lowering.
- `.nbb` writer.
- machine evidence integration.
- basal release readiness.
