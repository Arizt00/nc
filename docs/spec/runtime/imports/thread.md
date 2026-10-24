# nC Runtime Import: nc.runtime.thread

## 1. Purpose
`nc.runtime.thread` is the registered runtime import surface for the `thread` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.thread { thread };
```

## 3. Aliases
`runtime.thread`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `threadSpawn` | `threadSpawn(@fn(...)) -> Result<ThreadHandle, ThreadError>` | `Result<ThreadHandle, ThreadError>` | `thread.spawn` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_001` | crea un hilo real del host y devuelve un handle nominal opaco |
| `threadJoin` | `threadJoin(any) -> Result<any, ThreadError>` | `Result<any, ThreadError>` | `thread.join` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_001` | consume el handle y devuelve el valor real retornado por la función del hilo; sema permite proyectarlo a `Result<T, ThreadError>` cuando el destino concreta `T` |
| `threadJoinTimeout` | `threadJoinTimeout(any, int) -> Result<any, ThreadError>` | `Result<any, ThreadError>` | `thread.join` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_001` | devuelve `Err` si el hilo no termina antes del timeout y conserva el handle para un `threadJoin` posterior |
| `threadId` | `threadId() -> int` | `int` | `thread.inspect` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `none` | devuelve un id numérico estable por hilo dentro del proceso `ncc run` |
| `threadSleep` | `threadSleep(int) -> void` | `void` | `thread.sleep` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_001` | bloquea el hilo actual usando `std::thread::sleep` |
| `threadLocalCreate` | `threadLocalCreate(any) -> Result<ThreadLocalHandle, ThreadError>` | `Result<ThreadLocalHandle, ThreadError>` | `thread.local` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_LOCAL_001` | crea una clave thread-local con valor por defecto |
| `threadLocalGet` | `threadLocalGet(any) -> Result<any, ThreadError>` | `Result<any, ThreadError>` | `thread.local` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_LOCAL_001` | lee el valor local del hilo actual |
| `threadLocalSet` | `threadLocalSet(any, any) -> Result<bool, ThreadError>` | `Result<bool, ThreadError>` | `thread.local` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_LOCAL_001` | escribe el valor local del hilo actual |
| `threadLocalRemove` | `threadLocalRemove(any) -> Result<bool, ThreadError>` | `Result<bool, ThreadError>` | `thread.local` | `diagnostics.stable, imports.official-runtime-required, runtime.no-silent-fallback, runtime.trace.required` | `E_RUNTIME_THREAD_LOCAL_001` | elimina el valor local del hilo actual |

## 6. Type contracts
Function signatures are the active type contract surface for this import. `ThreadHandle` and `ThreadError` are runtime nominal values, represented internally as tagged runtime records instead of plain `int`/`str`.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive operations are governed by the required capabilities and policies listed below. Out-of-range or invalid state must surface a official runtime diagnostic.

## 10. Side effects
`threadSpawn` ejecuta la función nC en un `ExecState` aislado dentro de un hilo real
del host. Los efectos externos que esa función invoque, por ejemplo FS o TCP, siguen
pasando por sus imports/runtime respectivos. El estado `stc/local` no se comparte
implícitamente entre hilos.

## 11. Capabilities
- `thread.spawn`
- `thread.join`
- `thread.inspect`
- `thread.sleep`

## 12. Policies
- `diagnostics.stable`
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.trace.required`

## 13. Errors
- `E_RUNTIME_THREAD_001`: spawn con argumento que no es `@fn(...)`, handle desconocido,
  handle ya consumido, panic del hilo o `threadSleep` con milisegundos negativos.

El runtime mantiene un registro interno de hilos vivos. `threadSpawn` inserta el
`JoinHandle` real en esa tabla y `threadJoin` elimina la entrada al consumirlo.
Cualquier segundo `threadJoin` sobre el mismo handle produce `Result.err(ThreadError)`.

## 14. Positive examples
- `examples/runtime/families/thread_basic.n`
- `examples/runtime/runtime_real_thread_spawn_join.n`
- `examples/runtime/runtime_real_thread_sleep_id.n`
- `examples/runtime/runtime_real_thread_join_timeout.n`
- `examples/runtime/runtime_real_channel_threadlocal.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

Runtime negative examples:

- `examples/runtime/negative/runtime_real_thread_invalid_spawn_result.n`
- `examples/runtime/negative/runtime_real_thread_double_join_result.n`

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/thread.json` and `reports/runtime_closure/runtime_reports/thread.md`.

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
- exports command: `ncc runtime exports nc.runtime.thread --json`
- check command: `ncc runtime check examples/runtime/families/thread_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/thread_basic.n --json`
- evidence hash: `3b20e7eeedb2ebf5a48db96aaf364ea5c3980240130e7daa11027d21ce746823`

## 21. Native materialization

The native `threadSpawn/threadJoin` path is defined by LLVM/MCT as
`ncc_rt_thread_spawn` and `ncc_rt_thread_join`; it is not supplied by the
hosted C provider. The raw record contains the OS thread handle, function
pointer and returned scalar payload. Join waits through `CreateThread`/pthread,
reads the payload, closes the handle and releases the record explicitly.
