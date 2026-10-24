# `.nb` Format

`.nb` is readable basal semantic source. It must contain a root container declared with `[@]`.

Required root fields:

| Field | Values |
|---|---|
| `identity` | stable root identity string |
| `version` | semantic version string |
| `authority` | `sealed`, `open`, or `internal` |
| `binary` | `required`, `optional`, or `forbidden` |
| `evidence` | `required` or `optional` |
| `lowering` | `deferred`, `compiler`, `sovereign`, or gated `machine` |

If `authority: sealed`, `seal:` is required. If `binary: required`, `binary:` is required. If `evidence: required`, `evidence:` is required.

Baseical diagnostics include `E_BASAL_ROOT_IDENTITY_001`, `E_BASAL_SEAL_REQUIRED_001`, and `E_BASAL_BINARY_KIND_001`.
