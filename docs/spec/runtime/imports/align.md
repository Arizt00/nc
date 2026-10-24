# nC Runtime Import: nc.runtime.align

## 1. Purpose
`nc.runtime.align` is the registered runtime import surface for the `align` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use core.memory { align };
```

## 3. Aliases
`runtime.align`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `alignLeft` | `alignLeft(str, int) -> str` | `str` | `runtime.text.read, runtime.string.transform` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | UTF-8 byte width, right padding |
| `alignRight` | `alignRight(str, int) -> str` | `str` | `runtime.text.read, runtime.string.transform` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | UTF-8 byte width, left padding |
| `alignCenter` | `alignCenter(str, int) -> str` | `str` | `runtime.text.read, runtime.string.transform` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | odd padding byte remains on the right |

## 6. Type contracts
Function signatures are the active type contract surface for this import. Generic exports keep their declared generic marker in the function name.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive operations are governed by the required capabilities and policies listed below. Out-of-range or invalid state must surface a official runtime diagnostic.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `runtime.text.read`
- `runtime.string.transform`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/align_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/align.json` and `reports/runtime_closure/runtime_reports/align.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
This import does not make runtime calls available without `use`, does not bypass capability/policy checks, and does not cross into `[@machine]` or `primitive[@]`.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.align --json`
- check command: `ncc runtime check examples/runtime/families/align_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/align_basic.n --json`
- evidence hash: `4a5e099ce2215b189b1ed6683adb06d71ea47bfbcbc16c9ff70e652e4025a289`
