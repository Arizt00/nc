# nC Runtime Import: nC.runtime.region

## 1. Purpose
`nC.runtime.region` is the official runtime surface for `memRegion<T>`. It provides typed region construction, length, read, write, lifecycle, and movement contracts.

## 2. Canonical compact import
```n
use stoud.memory { region };
```

## 3. Aliases
Allowed alias: `use runtime.region;`. No hidden import may make region calls available without `use`.

## 4. Mode availability
Allowed in `[@runtime]` and runtime-aware compiler analysis. Denied in forbidden modes under strict runtime import rules. A call without an explicit import emits `E_RUNTIME_USE_001`.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `regionCreate<T>` | `regionCreate<T>(int)` | `memRegion<T>` | `runtime.region.create` | `memory.bounds.required` | `E_RUNTIME_GENERIC_INFERENCE_001`, `E_RUNTIME_ANY_LEAK_001` | Explicit `T` creates a typed region. |
| `regionLen<T>` | `regionLen(memRegion<T>)` | `int` | `runtime.region.read` | `memory.bounds.required` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Length is always `int`. |
| `regionRead<T>` | `regionRead<T>(memRegion<T>, int)` | `T` | `runtime.region.read` | `memory.bounds.required` | `E_TYPE_MISMATCH_001`, bounds diagnostics | Read returns the element type, never `any`. |
| `regionWrite<T>` | `regionWrite<T>(memRegion<T>, int, T)` | `bool` | `runtime.region.write` | `memory.bounds.required` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Written value must be `T`. |
| `regionReadonly<T>` | `regionReadonly(memRegion<T>)` | `memRegion<T>` | `runtime.region.read` | `memory.alias-tracking.required` | lifecycle diagnostics | Preserves region element type. |
| `regionInvalidate<T>` | `regionInvalidate(memRegion<T>)` | `bool` | `runtime.region.invalidate` | `runtime.no-silent-fallback` | lifecycle diagnostics | Invalidates the handle. |
| `regionCopy<T>` | `regionCopy(memRegion<T>, memRegion<T>, int)` | `bool` | `runtime.region.write` | `memory.alias-tracking.required` | bounds diagnostics | Source and destination share `T`. |
| `regionMove<T>` | `regionMove(memRegion<T>, int, memRegion<T>, int, int)` | `bool` | `runtime.region.write` | `memory.alias-tracking.required` | bounds diagnostics | Move preserves `T`. |
| `regionZero<T>` | `regionZero(memRegion<T>)` | `bool` | `runtime.region.write` | `memory.bounds.required` | lifecycle diagnostics | Zeroes the typed region. |
| `regionFill<T>` | `regionFill(memRegion<T>, T)` | `bool` | `runtime.region.write` | `memory.bounds.required` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Fill value must be `T`. |

## 6. Type contracts
The active generic contract is `T`. `regionCreate<T>` returns `memRegion<T>`, `regionRead<T>` returns `T`, and `regionWrite<T>` requires a value of `T`. The compiler resolves explicit generic arguments first and then verifies receiver/value types.

## 7. Ownership rules
`regionCreate<T>` creates an owned region handle. Reads borrow the region, writes mutate it, and `regionInvalidate` closes its lifecycle. Read/write after invalidation is a runtime lifecycle error.

## 8. Nullability rules
`memRegion<T>` is not nullable. Pointer conversion and nullable pointer refinement belong to `nC.runtime.ptr`, not this import.

## 9. Bounds rules
Region length and index values are `int`. Reads, writes, copy, move, zero, and fill are bounds-sensitive and must not silently fall back.

## 10. Side effects
The family mutates runtime memory-region storage only. It does not perform filesystem, network, process, primitive, or machine side effects.

## 11. Capabilities
`runtime.region.create`, `runtime.region.read`, `runtime.region.write`, `runtime.region.invalidate`, `runtime.region.bounds`.

## 12. Policies
`imports.official-runtime-required`, `memory.alias-tracking.required`, `memory.bounds.required`, `memory.null-refinement.required`, `memory.ptr-origin.required`, `runtime.no-silent-fallback`.

## 13. Errors
`E_RUNTIME_USE_001`, `E_RUNTIME_GENERIC_INFERENCE_001`, `E_RUNTIME_GENERIC_RETURN_001`, `E_RUNTIME_GENERIC_ARG_MISMATCH_001`, `E_RUNTIME_ANY_LEAK_001`, `E_RUNTIME_EXPORT_SIGNATURE_001`, plus official projection/type mismatch and bounds diagnostics.

## 14. Positive examples
`examples/stress/stress_runtime_region_pack.n` checks and runs:
```text
region[2]: 30
region[3]: 40
```

## 15. Negative examples
Covered by `crates/ncc-cli/tests/runtime_generics_closure.rs`: missing import, constructor mismatch, read assignment mismatch, write value mismatch, and any-leak guard.

## 16. Strict mode
Strict mode keeps the same type contracts and additionally requires explicit imports and stable diagnostics. Runtime calls are not made available magically.

## 17. Runtime report shape
`ncc runtime exports nC.runtime.region --json` emits typed export rows with `generic_params`, `arg_types`, and `return`.

## 18. Industrial conformance checklist
- `regionCreate<int>(4) -> memRegion<int>`
- `regionRead<int>(memRegion<int>, int) -> int`
- `regionWrite<int>(memRegion<int>, int, int) -> bool`
- missing import fails with `E_RUNTIME_USE_001`
- type mismatches fail before runtime execution
- positive stress examples do not leak an untyped `any` return

## 19. Known non-goals
This family does not expose raw hardware, does not use `primitive[@]`, does not cross into `[@machine]`, and does not allow region calls without import.

## 20. Evidence
- `cargo test -p ncc-cli --test runtime_generics_closure -- --nocapture`: 20 passed.
- `target/debug/ncc run examples/stress/stress_runtime_region_pack.n`: passed with typed region output.
- `target/debug/ncc runtime exports nC.runtime.region --json`: exports typed generic signatures.
