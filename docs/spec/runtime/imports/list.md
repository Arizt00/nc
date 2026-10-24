# nC Runtime Import: nC.runtime.list

## 1. Purpose
`nC.runtime.list` is the official runtime surface for `list<T>`. It provides explicit constructors, mutation, lookup, conversion, and copy/move helpers for typed runtime lists.

## 2. Canonical compact import
```n
use stoud.collections { list };
```

## 3. Aliases
Allowed aliases: `use nC.list.*;`, `use nC.runtime.list.*;`, and `use runtime.list;`.

## 4. Mode availability
Allowed in `[@runtime]` and runtime-aware compiler analysis. Denied in forbidden modes under strict runtime import rules. A call without an explicit import emits `E_RUNTIME_USE_001`.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `listCreate<T>` | `listCreate<T>()` | `list<T>` | `list.create` | `imports.official-runtime-required` | `E_RUNTIME_GENERIC_INFERENCE_001`, `E_RUNTIME_ANY_LEAK_001` | Explicit `T` creates a typed list. |
| `listCreateWithCapacity<T>` | `listCreateWithCapacity<T>(int)` | `list<T>` | `list.create` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Capacity is checked as `int`. |
| `listFromArray<T>` | `listFromArray<T>(array<T>)` | `list<T>` | `list.create` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Preserves element type. |
| `listLen` | `listLen(any)` | `int` | `list.read` | `diagnostics.stable` | runtime bounds diagnostics | Legacy untyped receiver helper. |
| `listCapacity` | `listCapacity(any)` | `int` | `list.read` | `diagnostics.stable` | runtime diagnostics | Legacy capacity helper. |
| `listIsEmpty` | `listIsEmpty(any)` | `bool` | `list.read` | `diagnostics.stable` | runtime diagnostics | Legacy emptiness helper. |
| `listAdd<T>` | `listAdd(list<T>, T)` | `bool` | `list.push` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Value must match element type. |
| `listPush<T>` | `listPush(list<T>, T)` | `bool` | `list.push` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Alias-compatible push. |
| `listPop<T>` | `listPop(list<T>)` | `Option<T>` | `list.pop` | `runtime.no-silent-fallback` | runtime bounds diagnostics | Empty list is represented by `Option<T>`. |
| `listGet<T>` | `listGet(list<T>, int)` | `T` | `list.read` | `runtime.no-silent-fallback` | type diagnostics | Getter returns `T`; out-of-bounds returns the typed default value. |
| `listSet<T>` | `listSet(list<T>, int, T)` | `bool` | `list.write` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Stored value must be `T`; out-of-bounds returns `false`. |
| `listInsert<T>` | `listInsert(list<T>, int, T)` | `bool` | `list.write` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Insert preserves `T`; invalid index returns `false`. |
| `listRemoveAt<T>` | `listRemoveAt(list<T>, int)` | `T` | `list.write` | `runtime.no-silent-fallback` | type diagnostics | Removed value has type `T`; out-of-bounds returns the typed default value. |
| `listClear` | `listClear(any)` | `bool` | `list.write` | `diagnostics.stable` | runtime diagnostics | Legacy clear helper. |
| `listContains<T>` | `listContains(list<T>, T)` | `bool` | `list.read` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Search value must be `T`. |
| `listIndexOf<T>` | `listIndexOf(list<T>, T)` | `int` | `list.read` | `runtime.no-silent-fallback` | `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Search value must be `T`. |
| `listSlice<T>` | `listSlice(list<T>, int, int)` | `list<T>` | `list.read` | `runtime.no-silent-fallback` | type diagnostics | Slice preserves `T`; invalid ranges clamp or return an empty list. |
| `listAsArray<T>` | `listAsArray(list<T>)` | `array<T>` | `list.read` | `runtime.no-silent-fallback` | conversion diagnostics | Converts without losing `T`. |
| `listAsRegion<T>` | `listAsRegion(list<T>)` | `memRegion<T>` | `list.read` | `memory.bounds.required` | conversion diagnostics | Region conversion preserves `T`. |
| `listClone<T>` | `listClone(list<T>)` | `list<T>` | `list.read` | `runtime.no-silent-fallback` | clone diagnostics | Clone preserves `T`. |
| `listCopy<T>` | `listCopy(list<T>)` | `list<T>` | `list.read` | `runtime.no-silent-fallback` | copy diagnostics | Copy preserves `T`. |
| `listMove<T>` | `listMove(list<T>, int, int)` | `bool` | `list.write` | `runtime.no-silent-fallback` | bounds diagnostics | Move within one list. |
| `listSwap<T>` | `listSwap(list<T>, int, int)` | `bool` | `list.write` | `runtime.no-silent-fallback` | bounds diagnostics | Swap within one list. |

## 6. Type contracts
The active generic contract is `T`. Constructors return `list<T>`, getters return `T`, mutators require values of `T`, and conversions preserve `T`. A generic runtime export that can resolve `T` must never expose `any` to a typed projection.

## 7. Ownership rules
`listCreate<T>` creates a new owned runtime list handle. `listAdd`, `listSet`, `listInsert`, `listMove`, and `listSwap` mutate that handle. `listClone` and `listCopy` produce another `list<T>` according to the clone/copy contract.

## 8. Nullability rules
`list<T>` is non-null at the type level. Absence from `listPop<T>` is represented as `Option<T>`, not as a nullable or `any` value.

## 9. Bounds rules
Index operations require `int` indices. Dynamic collection bounds are permissive: reads outside the current length return the declared element default (`0`, `false`, `""`, null pointer, or diagnostic when untyped), mutators return `false`, and slices clamp instead of aborting. No bounds operation may silently return `any`.

## 10. Side effects
The family mutates runtime list storage only. It does not perform filesystem, network, process, primitive, or machine side effects.

## 11. Capabilities
`list.create`, `list.push`, `list.pop`, `list.read`, `list.write`.

## 12. Policies
`imports.official-runtime-required`, `runtime.no-silent-fallback`, `diagnostics.stable`, `runtime.trace.required`.

## 13. Errors
`E_RUNTIME_USE_001`, `E_RUNTIME_GENERIC_INFERENCE_001`, `E_RUNTIME_GENERIC_RETURN_001`, `E_RUNTIME_GENERIC_ARG_MISMATCH_001`, `E_RUNTIME_ANY_LEAK_001`, `E_RUNTIME_EXPORT_SIGNATURE_001`, plus official projection/type mismatch diagnostics.

## 14. Positive examples
`examples/stress/stress_runtime_list_pack.n` checks and runs:
```text
list original: 15
list modified: 115
sum: 130
```

## 15. Negative examples
Covered by `crates/ncc-cli/tests/runtime_generics_closure.rs`: missing import, constructor mismatch, getter assignment mismatch, setter value mismatch, and any-leak guard.

## 16. Strict mode
Strict mode keeps the same type contracts and additionally requires explicit imports and stable diagnostics. Runtime calls are not made available magically.

## 17. Runtime report shape
`ncc runtime exports nC.runtime.list --json` emits typed export rows with `generic_params`, `arg_types`, and `return`.

## 18. Industrial conformance checklist
- `listCreate<int>() -> list<int>`
- `listGet(list<int>, int) -> int`
- `listSet(list<int>, int, int) -> bool`
- `lista.push(10)`, `lista.get(0)`, and `lista.len()` are accepted compact method forms.
- out-of-bounds reads on typed lists return the typed default value.
- missing import fails with `E_RUNTIME_USE_001`
- type mismatches fail before runtime execution
- positive stress examples do not leak an untyped `any` return

## 19. Known non-goals
This family does not replace arrays, does not grant calls without `use`, does not use `primitive[@]`, and does not cross into `[@machine]`.

## 20. Evidence
- `cargo test -p ncc-cli --test runtime_generics_closure -- --nocapture`: 20 passed.
- `target/debug/ncc run examples/stress/stress_runtime_list_pack.n`: passed with typed list output.
- `target/debug/ncc runtime exports nC.runtime.list --json`: exports typed generic signatures.
