# nC Runtime Import: nc.runtime.random

## 1. Purpose
`nc.runtime.random` is the registered runtime surface for the `random` family.
Its operations are part of the hosted backend prelude and therefore do not
require a source import. An explicit import remains accepted for compatibility.

## 2. Optional compact import
```n
use stoud.io { random };
```

## 3. Aliases
`runtime.random`

## 4. Mode availability
Available without import in hosted root and runtime containers. Machine,
primitive, kernel and freestanding contexts do not receive hosted PRNG state.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `randomInt` | `randomInt() -> int` | `int` | `random.int, random.float, random.range` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `randomRange` | `randomRange(int, int) -> int` | `int` | `random.int, random.float, random.range` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `randomFloat` | `randomFloat() -> ft64` | `ft64` | `random.int, random.float, random.range` | `runtime.no-silent-fallback` | `none` | 53-bit fraction in `[0, 1)` |
| `randomBool` | `randomBool() -> bool` | `bool` | `random.int, random.float, random.range` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `seed` | `seed(int) -> bool` | `bool` | `random.int, random.float, random.range` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | resets deterministic PRNG state |

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
- `random.int`
- `random.float`
- `random.range`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/random_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/random.json` and `reports/runtime_closure/runtime_reports/random.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
The prelude resolves names and signatures only. Calls remain explicit and do
not cross into `[@machine]`, primitive or kernel contexts.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.random --json`
- check command: `ncc runtime check examples/runtime/families/random_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/random_basic.n --json`
- evidence hash: `1244024f6a2c3758022dda916041de1793dce03fbbfdfd18e870090fa8f9c401`
