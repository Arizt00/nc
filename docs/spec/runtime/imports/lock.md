# nC Runtime Import: nc.runtime.lock

## 1. Purpose
`nc.runtime.lock` is the registered runtime import surface for the `lock` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.sync { lock };
```

## 3. Aliases
`runtime.lock`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `rwLockCreate` | `rwLockCreate() -> any` | `any` | `lock.mutex, lock.rw` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `readLock` | `readLock(any) -> bool` | `bool` | `lock.mutex, lock.rw` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `writeLock` | `writeLock(any) -> bool` | `bool` | `lock.mutex, lock.rw` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |
| `unlock` | `unlock(any) -> bool` | `bool` | `lock.mutex, lock.rw` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | official export |

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
- `lock.mutex`
- `lock.rw`

## 12. Policies
- `diagnostics.stable`
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.trace.required`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/lock_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/lock.json` and `reports/runtime_closure/runtime_reports/lock.md`.

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
- exports command: `ncc runtime exports nc.runtime.lock --json`
- check command: `ncc runtime check examples/runtime/families/lock_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/lock_basic.n --json`
- evidence hash: `27ba8ceee5dcea9c50a03b98d3f56694ea5288be70726c614f73978a70d55836`
