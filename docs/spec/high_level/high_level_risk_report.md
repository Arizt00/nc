# High Level Risk Report

`ncc high-level risk-report <file.n> --json` parses and semantically validates a source file before emitting a report. It never reports success for a file that fails sema.

Report contents:

- source file.
- item count.
- accepted risks.
- declared allowances.
- bridge targets.
- declared report names.
- authority policy summary.
- unification preparation status.

Example command:

```bash
ncc high-level risk-report examples/high_level/phase15/risk_policy_basic.n --json
```

Representative JSON fields:

```json
{
  "phase": "phase15_risk_authority_expert_policy_surface",
  "status": "passed",
  "risk_report": {
    "status": "closed",
    "policy": {
      "authority": "integrated_high_level_policy_surface"
    }
  }
}
```

`authority-report` uses the same validated source and focuses on the authority section of the report.
