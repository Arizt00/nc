# Basal Phase 4 Scope

Basal Phase 4 closes lowering from BasalIR/RootIR to `[@machine]`.

Closed:

- lowering planner;
- root lowering;
- site lowering;
- bind lowering;
- machine target reference planning;
- machine symbol reference planning;
- local site resolution;
- explicit pending machine emit states;
- lowering diagnostics;
- lowering JSON CLI/report output.

Not started:

- `.nbb` writer;
- final machine evidence integration;
- basal seal generation;
- high-level/runtime/primitive integration.

Phase 4 is complete only when unresolved local references and unresolved machine references fail unless explicitly marked pending.
