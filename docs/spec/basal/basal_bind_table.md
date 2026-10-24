# Basal Bind Table

Status: Basal Fase 2 closed.

The bind table is built from `bind:` AST declarations. Each entry records a generated id from the source path, source path, destination kind, destination path, source span, target span, resolution status, diagnostics, and status.

Recognized destination kinds are `machine.proc`, `machine.symbol`, `root.site`, and `root.symbol`. Machine destinations are recorded as `pending_lowering_or_machine_resolution`; Phase 2 does not resolve machine symbols. Local `root.site` destinations are checked against the site table.

The bind table is syntax/table-level only. Linkage, lowering, machine evidence, and BasalIR are not part of this phase.
