# High Level Exit Code Policy

Status: closed for High Level Fase 1.

This policy applies to high-level source commands and to phase sweep scripts. It does not add Fase 2+ language features.

## Baseical Exit Codes

| Code | Meaning | Required shape |
| --- | --- | --- |
| 0 | Real success | No error/fatal/internal diagnostic unless the command is explicitly report-only. |
| 1 | Source/user error | Parser, semantic, capability, unresolved symbol, or declared contract error. |
| 2 | CLI usage error | Wrong command, wrong options, missing required arguments. |
| 3 | Internal compiler error | Compiler bug, invariant break, panic boundary, or impossible internal state. |
| 4 | Toolchain/dependency error | Missing linker/tool/dependency required by a command. |
| 5 | Doctor/conformance error | Doctor, release gate, or conformance check failed. |
| 6 | Environment error | Host permissions, filesystem, unsupported host, or non-deterministic environment issue. |
| 7 | Evidence/seal/fake-success error | Evidence, seal, or fake-success mismatch. |

## Hard Rule

If a command emits a diagnostic with severity `error`, `fatal`, or `internal`, its exit code cannot be 0 unless the command is explicitly report-only and emits machine-readable metadata:

```json
{
  "status": "diagnostic_report",
  "exit_policy": "report_only",
  "report_only": true,
  "has_errors": true
}
```

`ncc check` is not report-only. A source error under `ncc check` must exit nonzero.

## Fase 1 Scope

Fase 1 closes the policy and adds the high-level doctor gate. It classifies previously confusing exit-zero negative fixtures instead of expanding the language. Commands that already behave as report-only must keep explicit metadata when they retain exit code 0.

Diagnostics introduced for the policy:

- `E_HIGHLEVEL_EXIT_POLICY_001`
- `E_HIGHLEVEL_DIAGNOSTIC_EXIT_MISMATCH_001`
