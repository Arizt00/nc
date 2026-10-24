# Basal Root Table

Status: Basal Fase 2 closed.

The root table is built from the Phase 1 `.nb` AST. It records the root declaration name, root id, positive integer version, profile, target reference, declared sections, metadata, source span, diagnostics, and entry status.

The table validates one root declaration per file, one `root:` section, a root id via `id` or `identity`, a positive integer `version`, and an allowed profile. Missing or malformed fields produce official `E_BASAL_ROOT_*` diagnostics.

This table does not create BasalIR and does not lower anything to `[@machine]`.
