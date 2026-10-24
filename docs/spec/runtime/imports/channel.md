# nC Runtime Import: nc.runtime.channel

## 1. Purpose
`nc.runtime.channel` is the registered runtime import surface for the `channel` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.sync { channel };
```

## 3. Aliases
`runtime.channel`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `stdin` | `stdin() -> any` | `any` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `stdout` | `stdout() -> any` | `any` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `stderr` | `stderr() -> any` | `any` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `writeTo` | `writeTo(any, str) -> result` | `result` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `readFrom` | `readFrom(any) -> result` | `result` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |
| `flushChannel` | `flushChannel(any) -> bool` | `bool` | `runtime.output.write, runtime.output.flush` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | official export |

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
- `runtime.output.write`
- `runtime.output.flush`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/channel_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/channel.json` and `reports/runtime_closure/runtime_reports/channel.md`.

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
- exports command: `ncc runtime exports nc.runtime.channel --json`
- check command: `ncc runtime check examples/runtime/families/channel_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/channel_basic.n --json`
- evidence hash: `08a0c13c7b1c587495a8c5c54c3dd57e6526b1025dc3882c8ba2aa3fd3f72a61`
