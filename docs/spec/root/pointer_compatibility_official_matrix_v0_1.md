# nC v0.1 — Memoria/punteros — matriz canónica de compatibilidad

## Estado
Matriz operativa y ejecutable de compatibilidad de familia puntero en `ncc-sema`.

## Objetivo
Congelar, en forma canónica, qué combinaciones son válidas en:
- proyección/alias `>>`
- cast estricto `!>`

y dejar mapeo directo a pruebas y diagnósticos.

## Convención
En tablas:
- `target <- source` significa `@target >> Ttarget : @source` (o equivalente tipado).
- `target !> source` significa cast estricto `@target !> Ttarget : @source`.

## Matriz canónica `>>` (proyección/alias)
Regla semántica base: `>>` acepta `Exact` y `Upcast`, y rechaza `Downcast`/`Forbidden`.

| Caso | Resultado |
|---|---|
| `ptr <- ptr` | `OK` |
| `ptr <- ptr<T>` | `OK` |
| `ptr<T> <- ptr` | `FAIL` |
| `ptr<T> <- ptr<T>` (mismo `T`) | `OK` |
| `ptr<T> <- ptr<U>` (`T != U`) | `FAIL` |
| `addr <- ptr` | `OK` |
| `addr <- ptr<T>` | `OK` |
| `ptr <- addr` | `FAIL` |
| `ptr<T> <- addr` | `FAIL` |
| `ptr <- skk*` | `OK` |
| `ptr<T> <- skk*` | `OK` |
| `skk* <- ptr` | `FAIL` |
| `skk** <- skk**` | `OK` |
| `region <- region` | `OK` |
| `ptr <- region` | `FAIL` |
| `region <- addr` | `FAIL` |

## Matriz canónica `!>` (cast estricto)
Regla semántica base: `!>` acepta toda compatibilidad no `Forbidden`, y además habilita conversiones explícitas físicas adicionales (`region/skk*/skk** <- addr`).

| Caso | Resultado |
|---|---|
| `ptr !> addr` | `OK` |
| `addr !> ptr` | `OK` |
| `ptr<T> !> addr` | `OK` |
| `addr !> ptr<T>` | `OK` |
| `ptr !> skk*` | `OK` |
| `ptr<T> !> skk*` | `OK` |
| `addr !> skk*` | `OK` |
| `region !> addr` | `OK` |
| `skk* !> addr` | `OK` |
| `skk** !> addr` | `OK` |
| `ptr !> region` | `FAIL` |
| `ptr !> skk**` | `FAIL` |
| `ptr<T> !> region` | `FAIL` |
| `region !> ptr` | `FAIL` |
| `ptr<T> !> ptr<U>` (`T != U`) | `FAIL` |
| `ptr<T> !> ptr` | `OK` |
| `ptr !> ptr<T>` | `OK` |

## Reglas canónicas de estado (`Null`/`Uninit`)
| Regla | Resultado |
|---|---|
| `region <- null` | `FAIL` (`region cannot be null`) |
| `ptr/ptr<T>/addr/skk*/skk** <- null` | `OK` |
| proyección desde `Uninit` hacia `ptr/ptr<T>/addr` | `FAIL` (`use of uninitialized pointer ...`) |
| proyección desde `Uninit` hacia `region/skk*/skk**` | `OK` (alias declarativo, estado permanece `Uninit`) |

## Evidencia ejecutable
Pruebas directas de tabla:
- `root_pointer_compatibility_matrix_is_stable_for_projection_and_strict_cast`

Matriz de fixtures `pointer_phase`:
- `root_pointer_bind_bootstrap_fixtures_pass_in_sema` (positivos p2..p11)
- `root_pointer_and_strict_error_fixtures_fail_in_sema` (negativos p2..p11)
- `root_pointer_error_fixtures_report_official_diagnostics` (diagnóstico canónico por fixture negativo)

## Mapa canónico fixture → diagnóstico
| Fixture | Diagnóstico canónico esperado |
|---|---|
| `pointer_phase_p2_error_addr_to_ptr.n` | `invalid aliasing between Simple("ptr") and Simple("addr")` |
| `pointer_phase_p2_error_ptr_to_skk.n` | `invalid aliasing between Simple("skk*") and Simple("ptr")` |
| `pointer_phase_p3_error_deep.n` | `invalid aliasing between Simple("ptr") and Simple("skk**")` |
| `pointer_phase_p3_error_upcast.n` | `invalid aliasing between Simple("skk*") and Simple("ptr")` |
| `pointer_phase_p4_error_bind.n` | `bind skk** requires source to also be skk**` |
| `pointer_phase_p4_error_to_ptr.n` | `invalid aliasing between Simple("ptr") and Simple("skk**")` |
| `pointer_phase_p5_error_bind.n` | `bind region requires source to also be region` |
| `pointer_phase_p5_error_to_ptr.n` | `invalid aliasing between Simple("ptr") and Simple("region")` |
| `pointer_phase_p6_error_region_null.n` | `region cannot be null` |
| `pointer_phase_p6_error_uninit.n` | `use of uninitialized pointer 'q'` |
| `pointer_phase_p7_error_ptr_to_skk.n` | `explicit pointer cast not allowed: Simple("ptr") !> Simple("skk*")` |
| `pointer_phase_p7_error_region_to_ptr.n` | `explicit pointer cast not allowed: Simple("region") !> Simple("ptr")` |
| `pointer_phase_p8_error_incompatible.n` | `invalid aliasing between Simple("ptr") and Simple("region")` |
| `pointer_phase_p9_alias_incompatible.n` | `invalid aliasing between Simple("ptr") and Simple("region")` |
| `pointer_phase_p9_compat.n` | `invalid aliasing between Simple("ptr") and Simple("addr")` |
| `pointer_phase_p9_full_system.n` | `alias cycle detected at 'p'` |
| `pointer_phase_p9_full_system_fail.n` | `invalid aliasing between Simple("ptr") and Simple("region")` |
| `pointer_phase_p9_invalid_use.n` | `use of uninitialized pointer 'b'` |
| `pointer_phase_p10_ptr_generic_uninit_fail.n` | `use of uninitialized pointer 'q'` |
| `pointer_phase_p11_ptr_generic_alias_conflict_fail.n` | `call 'entry.consume_then_scrub' aliases consumed parameter #1 with later-required parameter #2 via pointer root 'a'` |

