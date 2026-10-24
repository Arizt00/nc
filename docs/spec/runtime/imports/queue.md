# nC Runtime Import: nc.runtime.queue

## 1. Purpose
`nC.runtime.queue` is the registered runtime import surface for `queue<T>`. It exposes typed FIFO construction, push/pop, lookup, length, search, and compact method forms under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.collections { queue };
```

## 3. Aliases
`runtime.queue`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `queueCreate<T>` | `queueCreate<T>() -> queue<T>` | `queue<T>` | `queue.create` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | typed constructor |
| `queuePush<T>` | `queuePush(queue<T>, T) -> bool` | `bool` | `queue.push` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | type diagnostics | Appends to the tail. |
| `queuePop<T>` | `queuePop(queue<T>) -> option` | `option` | `queue.pop` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | none | Removes from the head; empty queue is `None`. |
| `queueLen<T>` | `queueLen(queue<T>) -> int` | `int` | `queue.read` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | none | Reads descriptor length. |
| `queueGet<T>` | `queueGet(queue<T>, int) -> T` | `T` | `queue.read` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | type diagnostics | Out-of-bounds returns the typed default value. |
| `queueSet<T>` | `queueSet(queue<T>, int, T) -> bool` | `bool` | `queue.write` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | type diagnostics | Out-of-bounds returns `false`. |
| `queueIsEmpty<T>` | `queueIsEmpty(queue<T>) -> bool` | `bool` | `queue.read` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | none | Checks emptiness. |
| `queueClear<T>` | `queueClear(queue<T>) -> bool` | `bool` | `queue.write` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | none | Clears storage. |
| `queueContains<T>` | `queueContains(queue<T>, T) -> bool` | `bool` | `queue.read` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | type diagnostics | Search helper. |
| `queueIndexOf<T>` | `queueIndexOf(queue<T>, T) -> int` | `int` | `queue.read` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | type diagnostics | Returns `-1` when absent. |

## 6. Type contracts
Function signatures are the active type contract surface for this import. `T` is preserved by constructors, getters, mutators, and method calls. Compact forms `cola.push(x)`, `cola.pop()`, `cola.get(i)`, and `cola.len()` are canonical.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive dynamic collection operations are permissive. Reads outside the current length return the declared element default, writes return `false`, and `queuePop` over an empty queue returns `None` in function form or the typed default in method form.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `queue.create`
- `queue.push`
- `queue.pop`
- `queue.read`
- `queue.write`

## 12. Policies
- `diagnostics.stable`
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.trace.required`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/queue_basic.n`
- `examples/runtime/collections/collections_comfy_oob_ok.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/queue.json` and `reports/runtime_closure/runtime_reports/queue.md`.

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
- exports command: `ncc runtime exports nc.runtime.queue --json`
- check command: `ncc runtime check examples/runtime/families/queue_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/queue_basic.n --json`
- evidence hash: `3ea2ad8a826b991fb4915da3a7b09d32fab2dbfdc103aed55072a97775d6a4f4`
