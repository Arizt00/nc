# nC v0.1 — Memoria/heap.manager — matriz canónica operativa

## Estado
Matriz operativa y ejecutable de reglas semánticas para:
- `memory.heap.manager.alloc`
- `memory.heap.manager.free`
- `memory.heap.manager.realloc`
- `memory.heap.manager.zero`

en `ncc-sema` (F5 línea activa).

## Objetivo
Congelar reglas, diagnósticos y evidencia de pruebas para evitar deriva semántica en guardas de estado, efectos de alias e interproceduralidad.

## 1) Guardas por operación y estado de puntero
Convención de estado: `Uninit | Null | Bound | Invalid`.

| Operación | Uninit | Null | Bound | Invalid |
|---|---|---|---|---|
| `alloc(size)` | `OK` (no guarda de puntero) | `OK` | `OK` | `OK` |
| `free(ptr)` | `FAIL` | `FAIL` | `OK` | `FAIL` |
| `realloc(ptr, size)` | `FAIL` | `FAIL` | `OK` | `FAIL` |
| `zero(ptr, size)` | `FAIL` | `FAIL` | `OK` | `FAIL` |

Diagnóstico canónico local:
- `memory.heap.manager.free cannot use uninitialized pointer '<sym>'`
- `memory.heap.manager.free cannot use null pointer '<sym>'`
- `memory.heap.manager.free cannot use invalid pointer '<sym>'`
- `memory.heap.manager.realloc cannot use uninitialized pointer '<sym>'`
- `memory.heap.manager.realloc cannot use null pointer '<sym>'`
- `memory.heap.manager.realloc cannot use invalid pointer '<sym>'`
- `memory.heap.manager.zero cannot use uninitialized pointer '<sym>'`
- `memory.heap.manager.zero cannot use null pointer '<sym>'`
- `memory.heap.manager.zero cannot use invalid pointer '<sym>'`

## 2) Efectos laterales canónicos sobre alias
| Operación | Requiere puntero válido | Invalida grupo de alias |
|---|---|---|
| `alloc` | `No` | `No` |
| `free` | `Sí` | `Sí` |
| `realloc` | `Sí` | `Sí` (grupo viejo) |
| `zero` | `Sí` | `No` |

## 3) Contrato interprocedural de llamadas
Si una llamada resuelta entra a función con contrato de puntero, el caller valida estado del argumento real.

Diagnóstico canónico en call-site:
- `call '<container>.<function>' cannot use uninitialized pointer '<sym>'`
- `call '<container>.<function>' cannot use null pointer '<sym>'`
- `call '<container>.<function>' cannot use invalid pointer '<sym>'`

Ownership-sensitivity en call-site:
- `call '<container>.<function>' requires simple pointer symbol argument for ownership-sensitive parameter #<n>`
- `call '<container>.<function>' aliases consumed parameter #<i> with later-required parameter #<j> via pointer root '<sym>'`
- `call '<container>.<function>' returns pointer alias of consumed parameter #<i> via parameter #<j> (root '<sym>')`

## 4) Evidencia ejecutable (tests)
Guardas locales:
- `memory_free_on_uninitialized_symbol_fails_in_sema`
- `memory_free_on_null_symbol_fails_in_sema`
- `memory_zero_on_null_symbol_fails_in_sema`
- `memory_zero_on_freed_symbol_fails_in_sema`
- `memory_realloc_on_uninitialized_symbol_fails_in_sema`
- `memory_realloc_on_null_symbol_fails_in_sema`
- `memory_realloc_on_invalid_symbol_fails_in_sema`

Efectos de alias:
- `memory_free_invalidates_alias_group_fails_in_sema`
- `memory_realloc_invalidates_old_aliases_but_new_symbol_stays_valid`

Interprocedural (estado):
- `memory_interprocedural_requires_valid_pointer_on_call_fails_in_sema`
- `memory_interprocedural_requires_non_null_pointer_on_call_fails_in_sema`
- `memory_interprocedural_qualified_call_requires_valid_pointer_fails_in_sema`
- `memory_interprocedural_qualified_call_requires_non_null_pointer_fails_in_sema`
- `memory_interprocedural_free_invalidates_caller_alias_group_fails_in_sema`
- `memory_interprocedural_qualified_call_invalidates_caller_alias_group_fails_in_sema`

Ownership interprocedural:
- `memory_interprocedural_call_consuming_param_requires_simple_symbol_argument`
- `memory_interprocedural_qualified_call_consuming_param_requires_simple_symbol_argument`
- `memory_interprocedural_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_qualified_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_transitive_alias_conflict_bubbles_to_caller_fails_in_sema`
- `memory_interprocedural_transitive_qualified_alias_conflict_bubbles_to_caller_fails_in_sema`
- `memory_interprocedural_return_alias_of_consumed_param_fails_in_sema`
- `memory_interprocedural_qualified_return_alias_of_consumed_param_fails_in_sema`
- `memory_interprocedural_ptr_generic_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_ptr_generic_return_alias_of_consumed_param_fails_in_sema`

## 5) Notas de continuidad
- Esta matriz describe semántica activa de `ncc-sema`; no adelanta comportamiento de fases futuras.
- Coherencia de flags base en AST (`HeapManagerOp`) está fijada en:
  - `heap_manager_op_pointer_guard_flags_match_base`.
- La matriz de punteros/casts complementaria está publicada en:
  - `docs/spec/root/pointer_compatibility_official_matrix_v0_1.md`.

