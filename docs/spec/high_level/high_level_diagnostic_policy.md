# High Level Diagnostic Policy

Status: closed for High Level Fase 1.

Diagnostics are part of the compiler contract. A diagnostic must not be used to hide success, and a generic diagnostic should not be used where a specific one exists.

## Severities

| Severity | Meaning | Exit behavior |
| --- | --- | --- |
| info | Context or explanation | Exit 0 allowed. |
| warning | Suspicious but accepted default behavior | Exit 0 by default; may fail under deny-warnings policy. |
| error | User/source error | Nonzero for non-report-only commands. |
| fatal | Pipeline cannot continue | Nonzero. |
| internal | Compiler bug or impossible state | Exit 3 target. |
| deferred | Feature intentionally deferred to a named future phase | Nonzero when used as source failure; report-only commands must mark `has_errors`. |
| accepted_risk | Risk accepted under explicit expert profile policy | Exit 0 allowed only when the profile/gate is declared and recorded. |

## Rules

- `ncc check` treats parser, semantic, unresolved symbol, and contract failures as source errors.
- Report-only commands may exit 0 while reporting errors only when they expose `report_only=true` and `has_errors=true`.
- Deferred diagnostics must include a future phase or reason.
- Accepted-risk diagnostics require an explicit expert profile or future gate; Fase 1 only documents those gates.
- Silent error success is forbidden.

## Fase 1 Boundaries

Fase 1 does not implement advanced expert capabilities. It documents severity rules, adds the high-level doctor, and classifies examples so later phases can act without changing the meaning of existing code.
