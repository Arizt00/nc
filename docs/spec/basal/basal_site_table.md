# Basal Site Table

Status: Basal Fase 2 closed.

The site table is built from `site:` AST declarations. Each entry records site name, kind, size, alignment, permissions, offset, optional target policy, source span, normalized fields, diagnostics, and status.

Allowed kinds are `memory_anchor`, `code_anchor`, `data_anchor`, `policy_anchor`, `evidence_anchor`, and `custom`. Permissions normalize to `read`, `write`, `execute`, `seal`, `evidence`, and `root`. Phase 2 rejects duplicate site names, invalid kinds, zero sizes, non-power-of-two alignment, unknown permissions, and `write | execute`.

The site table is structural only. Address assignment and object layout remain later phases.
