# nC Runtime Import: nc.runtime.task

## 1. Purpose
`nc.runtime.task` is the registered runtime import surface for the `task` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.task { task };
```

## 3. Aliases
`runtime.task`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `taskSpawn` | `taskSpawn(any) -> result` | `result` | `task.spawn, task.join, task.cancel` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `taskAwait` | `taskAwait(any) -> result` | `result` | `task.spawn, task.join, task.cancel` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `taskCancel` | `taskCancel(any) -> result` | `result` | `task.spawn, task.join, task.cancel` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `taskStatus` | `taskStatus(any) -> any` | `any` | `task.spawn, task.join, task.cancel` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |

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
- `task.spawn`
- `task.join`
- `task.cancel`

## 12. Policies
- `diagnostics.stable`
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.trace.required`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/task_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/task.json` and `reports/runtime_closure/runtime_reports/task.md`.

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
- exports command: `ncc runtime exports nc.runtime.task --json`
- check command: `ncc runtime check examples/runtime/families/task_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/task_basic.n --json`
- evidence hash: `925adb2abc199c0f3249524275c6fd290a3974d2c22eab085ed69c67b1f3b00f`
