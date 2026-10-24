# Compiler Runtime Generic Inference

## Scope
This document defines how compiler sema resolves generic runtime functions for the runtime list and region families.

## Problem Closed
The stress pack exposed a type resolver gap: generic runtime exports were cataloged or resolved as untyped `any`. Typed projections such as `@eventos >> list<int> : listCreate<int>();` therefore failed before the generic return contract could be applied.

## Inference Contract
The compiler now keeps explicit call type arguments in the parsed callee name and resolves dynamic runtime signatures through generic binding.

Resolution order:

1. Parse explicit call type arguments: `listCreate<int>()` stores `int` as the explicit `T`.
2. Bind `T` from arguments when possible: `listGet(@eventos, 3)` binds `T` from `@eventos: list<int>`.
3. Substitute `T` into return and parameter templates.
4. Reject unresolved or contradictory generics with official diagnostics.

## Closed Cases
| Source | Resolved Type |
|---|---|
| `listCreate<int>()` | `list<int>` |
| `listGet(@eventos, 3)` where `@eventos: list<int>` | `int` |
| `regionCreate<int>(4)` | `memRegion<int>` |
| `regionRead<int>(@region, 0)` where `@region: memRegion<int>` | `int` |

## Diagnostics
| Diagnostic | Meaning |
|---|---|
| `E_RUNTIME_USE_001` | Runtime call requires explicit import. |
| `E_RUNTIME_GENERIC_INFERENCE_001` | Generic type parameter cannot be inferred. |
| `E_RUNTIME_GENERIC_RETURN_001` | Generic return remains unresolved. |
| `E_RUNTIME_GENERIC_ARG_MISMATCH_001` | Explicit or inferred generic contradicts an argument. |
| `E_RUNTIME_ANY_LEAK_001` | Runtime generic tried to leak `any` into a typed projection. |
| `E_RUNTIME_EXPORT_SIGNATURE_001` | Runtime export lacks a typed signature. |

## Negative Behavior
`listSet(list<int>, int, bool)` and `regionWrite<int>(memRegion<int>, int, bool)` fail in sema with `E_RUNTIME_GENERIC_ARG_MISMATCH_001`. Missing imports fail with `E_RUNTIME_USE_001`.

## Non-Goals
The compiler does not use `primitive[@]`, does not touch `[@machine]`, and does not make runtime calls available by fallback. Bare unresolved generics remain invalid for typed projections.

## Evidence
- Parser preserves generic call arguments in callee names.
- Sema dynamic import resolution substitutes generic parameters before projection checking.
- Runtime catalog JSON exposes `generic_params`, typed `arg_types`, and typed `return`.
- `crates/ncc-cli/tests/runtime_generics_closure.rs` validates positive and negative cases.
