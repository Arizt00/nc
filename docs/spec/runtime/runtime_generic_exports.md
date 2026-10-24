# Runtime Generic Exports

## Scope
This document records the industrial contract for runtime generic exports corrected for `list<T>` and `memRegion<T>`.

## Rule
A runtime generic export must return its declared contract when `T` is explicit or inferable. It must not expose `any` to typed projections.

## Baseical list signatures
| Export | Args | Return |
|---|---|---|
| `listCreate<T>` | `()` | `list<T>` |
| `listCreateWithCapacity<T>` | `(int)` | `list<T>` |
| `listFromArray<T>` | `(array<T>)` | `list<T>` |
| `listAdd<T>` | `(list<T>, T)` | `bool` |
| `listPush<T>` | `(list<T>, T)` | `bool` |
| `listPop<T>` | `(list<T>)` | `Option<T>` |
| `listGet<T>` | `(list<T>, int)` | `T` |
| `listSet<T>` | `(list<T>, int, T)` | `bool` |
| `listInsert<T>` | `(list<T>, int, T)` | `bool` |
| `listRemoveAt<T>` | `(list<T>, int)` | `T` |
| `listContains<T>` | `(list<T>, T)` | `bool` |
| `listIndexOf<T>` | `(list<T>, T)` | `int` |
| `listSlice<T>` | `(list<T>, int, int)` | `list<T>` |
| `listAsArray<T>` | `(list<T>)` | `array<T>` |
| `listAsRegion<T>` | `(list<T>)` | `memRegion<T>` |
| `listClone<T>` | `(list<T>)` | `list<T>` |
| `listCopy<T>` | `(list<T>)` | `list<T>` |
| `listMove<T>` | `(list<T>, int, int)` | `bool` |
| `listSwap<T>` | `(list<T>, int, int)` | `bool` |

## Baseical region signatures
| Export | Args | Return |
|---|---|---|
| `regionCreate<T>` | `(int)` | `memRegion<T>` |
| `regionLen<T>` | `(memRegion<T>)` | `int` |
| `regionRead<T>` | `(memRegion<T>, int)` | `T` |
| `regionWrite<T>` | `(memRegion<T>, int, T)` | `bool` |
| `regionReadonly<T>` | `(memRegion<T>)` | `memRegion<T>` |
| `regionInvalidate<T>` | `(memRegion<T>)` | `bool` |
| `regionCopy<T>` | `(memRegion<T>, memRegion<T>, int)` | `bool` |
| `regionMove<T>` | `(memRegion<T>, int, memRegion<T>, int, int)` | `bool` |
| `regionZero<T>` | `(memRegion<T>)` | `bool` |
| `regionFill<T>` | `(memRegion<T>, T)` | `bool` |

## Resolution
The compiler resolves runtime generic calls in this order:

1. explicit call type arguments, for example `listCreate<int>()`;
2. generic bindings inferred from arguments, for example `listGet(@xs, 0)` where `@xs: list<int>`;
3. typed return substitution, for example `T -> int`.

If a generic parameter cannot be inferred, the compiler emits `E_RUNTIME_GENERIC_INFERENCE_001` or `E_RUNTIME_GENERIC_RETURN_001`. If a bound generic contradicts an argument, it emits `E_RUNTIME_GENERIC_ARG_MISMATCH_001`.

## Any-Leak Guard
`E_RUNTIME_ANY_LEAK_001` is reserved for a runtime generic that would otherwise expose `any` into a typed projection. The corrected positive stress examples check and run without this error.

## Evidence
- `target/debug/ncc runtime exports nC.runtime.list --json` shows `listCreate<T> -> list<T>` and `listGet -> T`.
- `target/debug/ncc runtime exports nC.runtime.region --json` shows `regionCreate<T> -> memRegion<T>` and `regionRead<T> -> T`.
- `cargo test -p ncc-cli --test runtime_generics_closure -- --nocapture` passes 20 tests.
