# nC Runtime Import: nc.runtime.ptr

## 1. Purpose
`nc.runtime.ptr` is the compatibility import surface for the raw pointer family. The canonical operations are native-prelude identifiers, so ordinary `.n` code does not need this import.

## 2. Canonical compact import
```n
use stoud.memory { ptr };
```

## 3. Aliases
`runtime.ptr`

## 4. Mode availability
Allowed as a compatibility alias in hosted/runtime code. `p<T>`, `pRead`, `pWrite`, `pOffset`, `pDiff`, `ptrRead`, `ptrWrite`, `ptrOffset`, `ptrDiff`, `ptrFromAddr`, and `ptrAddr` lower through the native backend without an import gate.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `ptrIsNull` | `ptrIsNull(any) -> bool` | `bool` | `runtime.ptr.create, runtime.ptr.read, runtime.ptr.write, runtime.ptr.refine, runtime.ptr.origin` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `none` | official export |
| `ptrAddr` | `ptrAddr(any) -> addr` | `addr` | `runtime.ptr.create, runtime.ptr.read, runtime.ptr.write, runtime.ptr.refine, runtime.ptr.origin` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `none` | official export |
| `ptrCast<T>` | `ptrCast<T>(any) -> result` | `result` | `runtime.ptr.create, runtime.ptr.read, runtime.ptr.write, runtime.ptr.refine, runtime.ptr.origin` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `none` | official export |
| `ptrFromAddr<T>` | `ptrFromAddr<T>(addr) -> ptr<T>` | `ptr<T>` | native prelude | none | type-shape errors | direct raw-address bridge |
| `ptrOffset<T>` | `ptrOffset<T>(ptr<T>, int) -> ptr<T>` | `ptr<T>` | native prelude | none | type-shape errors | wrapping typed stride; range unchecked |
| `ptrDiff<T>` | `ptrDiff<T>(ptr<T>, ptr<T>) -> int` | `int` | native prelude | none | type-shape errors | raw element-unit difference; no same-origin gate |
| `ptrRead<T>` | `ptrRead<T>(ptr<T>) -> T` | `T` | native prelude | none | type-shape errors | raw range-unchecked load |
| `ptrWrite<T>` | `ptrWrite<T>(ptr<T>, T) -> bool` | `bool` | native prelude | none | type-shape errors | raw range-unchecked store |

## 6. Type contracts
Function signatures are the active type contract surface for this import. Generic exports keep their declared generic marker in the function name.

## 7. Ownership rules
Raw pointers carry no ownership, lifetime, generation, provenance, or automatic cleanup contract.

## 8. Nullability rules
Plain `p<T>`/`ptr<T>` may contain null. `Option` or nullable unions are optional API-modeling tools, not dereference gates.

## 9. Bounds rules
Raw operations are never bounds-sensitive. `ptrOffset`, `ptrRead`, and `ptrWrite` may leave an allocation. Convert to `memRegion<T>` when a checked span is required.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `runtime.ptr.create`
- `runtime.ptr.read`
- `runtime.ptr.write`
- `runtime.ptr.refine`
- `runtime.ptr.origin`

## 12. Policies
No policy enables raw pointer access. Legacy capability/policy rows may remain in generated catalogs for compatibility and auditing, but are not normal compilation gates.

## 13. Errors
- none registered for this family

## 14. Positive examples
- `examples/runtime/families/ptr_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/ptr.json` and `reports/runtime_closure/runtime_reports/ptr.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
This compatibility import does not add safety. Null, misalignment, aliasing, UAF, double free, out-of-range access, dangling values, and overflow compile and have C-like undefined behavior. `memRegion<T>` and other bounded types retain their own contracts.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.ptr --json`
- check command: `ncc runtime check examples/runtime/families/ptr_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/ptr_basic.n --json`
- evidence hash: `45aa26cde1bfb0ca323a973b86850c7d7244f70dea1f888adb6d268e76be8c3d`
