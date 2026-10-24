# nC v0.1 — Memoria/punteros — static flow guards (F5 tranche)

## Estado
Tramo F5 cerrado end-to-end en la línea activa para validación semántica estática de flujo de punteros heap.

## Propósito
Alinear sema con el contrato runtime de heap (`memory.heap.manager`) para detectar usos inválidos antes de ejecución.

## Alcance de esta tranche
- llamadas importadas:
  - `memory.heap.manager.alloc`
  - `memory.heap.manager.free`
  - `memory.heap.manager.realloc`
  - `memory.heap.manager.zero`
- estado de puntero en sema:
  - `Uninit`
  - `Null`
  - `Bound`
  - `Invalid`
- propagación de alias local en función para invalidación por grupo.
- propagación interprocedural por firma de función (caller ↔ callee).
- joins CFG de ramas con reglas de merge para estado de puntero y alias.
- semántica material de `memRegion<T>` con lowering/IR tipado y backend físico dedicado en runtime.

## Reglas activas

### 1) Guardas de estado en calls heap manager
Cuando el primer argumento es referencia simple a símbolo tipado en estado de flujo:
- `free/realloc/zero` rechazan `Uninit`.
- `free/realloc/zero` rechazan `Null`.
- `free/realloc/zero` rechazan `Invalid`.

Diagnósticos canónicos:
- `memory.heap.manager.free cannot use uninitialized pointer '<sym>'`
- `memory.heap.manager.zero cannot use invalid pointer '<sym>'`
- análogos para `realloc` y `null`.

### 2) Invalidez por efecto lateral
- `free(sym)` invalida el grupo de alias de `sym`.
- `realloc(sym, n)` invalida el grupo de alias antiguo de `sym`.
- `zero(sym, n)` no invalida por sí misma; solo exige estado válido.

### 3) Alias de flujo local
En sema de función, asignaciones/casts de familia puntero (`addr/ptr/ptr<T>/region/skk*/skk**`) preservan relación de alias local para invalidación conjunta.

Reglas finas activas en esta línea:
- self-bootstrap tipado (`@x >> T : @x`) permitido para evitar ciclos artificiales en bind/proyección.
- ciclo indirecto real sigue fallando con diagnóstico canónico `alias cycle detected ...`.
- proyección desde `Uninit`:
  - `ptr/ptr<T>/addr`: rechazada (`use of uninitialized pointer ...`);
  - `region/skk*/skk**`: permitida como alias declarativo, manteniendo estado `Uninit` hasta materialización.

### 4) Flujo interprocedural (calls locales/importadas resueltas por firma)
- `Expr::Call` resuelve firma concreta (`container/name/params`) y aplica efectos tipados del callee.
- `Expr::QualifiedCall` same-file (`container.fn(...)`) también propaga contrato de puntero del callee.
- si el callee exige puntero válido en un parámetro, el caller valida el argumento real.
- si el callee invalida por side-effect (`free/realloc`), el caller invalida el grupo de alias del argumento real.
- si el callee invalida un parámetro por ownership (`free/realloc`), el argumento real debe ser símbolo puntero simple (`@sym`), no expresión temporal.
- si el contrato del callee consume un parámetro y luego requiere otro, el caller rechaza alias del mismo root entre ambos argumentos.
- esa regla se propaga transitivamente entre funciones (callee de callee) y llega al call-site final.
- si el contrato del callee consume un parámetro y retorna otro, el caller rechaza alias del mismo root entre ambos argumentos.
- esa regla `consume -> return` también se propaga transitivamente entre funciones.
- esta ruta también está cerrada para `ptr<T>` mediante cast explícito canónico `ptr<T> !> addr`
  antes de `memory.heap.manager.{free,zero,realloc}`.
- diagnóstico canónico por contrato de llamada:
  - `call '<container>.<function>' cannot use invalid pointer '<sym>'`
  - análogos para `uninitialized` y `null`.
  - `call '<container>.<function>' requires simple pointer symbol argument for ownership-sensitive parameter #<n>`.
  - `call '<container>.<function>' aliases consumed parameter #<i> with later-required parameter #<j> via pointer root '<sym>'`.
  - `call '<container>.<function>' returns pointer alias of consumed parameter #<i> via parameter #<j> (root '<sym>')`.

### 6) Guarda de retorno para punteros
- `return @sym` sobre función con retorno de familia puntero (`addr/ptr/ptr<T>/region/skk*/skk**`) valida estado de flujo:
  - rechaza `Uninit`
  - rechaza `Invalid`
  - permite `Null` para tipos que admiten null en la línea activa.
- diagnóstico canónico:
  - `return cannot use uninitialized pointer '<sym>'`
  - `return cannot use invalid pointer '<sym>'`

### 5) CFG global de punteros (joins de ramas)
- cada `if` se analiza por ramas con estado independiente.
- si ambas ramas caen por flujo, se hace merge:
  - estado de puntero: divergencia entre ramas => `Invalid` (conservador/safe-first).
  - alias: divergencia de raíz de alias entre ramas => símbolo marcado `Invalid`.
- si solo una rama cae por flujo (la otra termina con `return`), se preserva la rama que cae.
- si ambas ramas terminan, el bloque actual se marca terminado para no propagar side-effects de código inalcanzable.

## Coherencia end-to-end
- AST ahora expone clasificación canónica tipada de heap manager (`HeapManagerOp`):
  - `Alloc`, `Free`, `Realloc`, `Zero`.
- parser mantiene representación cualificada tipada (`Expr::QualifiedCall`) sin fallback stringly.
- sema aplica guardas/invalidación usando `HeapManagerOp` compartido (sin lógica duplicada de path por strings).
- sema usa índices por hilo para firmas/efectos de función, evitando drift/race entre análisis paralelos.
- sema valida firma tipada de métodos `memRegion<T>`:
  - `bind(int) -> bool`
  - `readAt(int) -> T`
  - `writeAt(int, T) -> void`
  - `len() -> int`
  - `zero() -> void`
  - `snapshot() -> memRegion<T>`
- IR baja `heap_effects` tipados por función (`HeapManagerEffectIR`) para `free/realloc/zero` en orden de evaluación.
- IR baja también contrato tipado de alias por función (`pointer_contract`) con:
  - `require_valid_params`
  - `invalidated_params`
  - `invalidates_then_requires`
  - `returned_params`
  - `invalidates_then_returns`
- IR baja efectos tipados de `memRegion<T>` por función (`mem_region_effects`) sin fallback stringly:
  - `Bind`, `ReadAt`, `WriteAt`, `Len`, `Zero`, `Snapshot`
  - `region_symbol`, `elem_type` y argumentos tipados (`ContextSymbol/NumberLiteral/StringLiteral/BoolLiteral`).
- runtime mantiene validaciones dinámicas de bloque stale/dead.
- ejecución de llamadas en runtime comparte `heap/events` entre caller↔callee (frame local de vars aislado),
  para que efectos de `free/realloc/zero` sean observables interproceduralmente.
- runtime propaga metadata de binder/actor de heap para toda la familia puntero material (`addr/ptr/ptr<T>/region/skk*/skk**`)
  en proyección y cast estricto (`>>` / `!>`), evitando pérdida de trazabilidad en alias/hechos de alloc.
- runtime materializa backend físico de `memRegion<T>` (`MemRegionStorage`) con:
  - handle estable (`MemRegionHandle`) por símbolo;
  - `bind` con `bound_addr` y reset de estado material;
  - celdas tipadas por elemento + `logical_len`;
  - guardas runtime para `readAt/writeAt/zero` sobre región no bound;
  - `snapshot` como clon material aislado de la región.
  - `writeAt` aplica coerción tipada previa y conserva nullable de familia puntero:
    - `memRegion<addr>` acepta `null` coherente con sema.

## Evidencia ejecutable
- `memory_free_invalidates_alias_group_fails_in_sema`
- `memory_realloc_invalidates_old_aliases_but_new_symbol_stays_valid`
- `memory_zero_on_freed_symbol_fails_in_sema`
- `memory_free_on_uninitialized_symbol_fails_in_sema`
- `memory_interprocedural_free_invalidates_caller_alias_group_fails_in_sema`
- `memory_interprocedural_requires_valid_pointer_on_call_fails_in_sema`
- `memory_interprocedural_qualified_call_invalidates_caller_alias_group_fails_in_sema`
- `memory_interprocedural_qualified_call_requires_valid_pointer_fails_in_sema`
- `memory_interprocedural_call_consuming_param_requires_simple_symbol_argument`
- `memory_interprocedural_qualified_call_consuming_param_requires_simple_symbol_argument`
- `memory_interprocedural_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_qualified_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_ptr_generic_alias_conflict_consumed_then_required_param_fails_in_sema`
- `memory_interprocedural_transitive_alias_conflict_bubbles_to_caller_fails_in_sema`
- `memory_interprocedural_transitive_qualified_alias_conflict_bubbles_to_caller_fails_in_sema`
- `memory_interprocedural_return_alias_of_consumed_param_fails_in_sema`
- `memory_interprocedural_qualified_return_alias_of_consumed_param_fails_in_sema`
- `memory_interprocedural_ptr_generic_return_alias_of_consumed_param_fails_in_sema`
- `memory_return_invalid_pointer_direct_fails_in_sema`
- `memory_return_null_pointer_direct_allowed_in_sema`
- `memory_pointer_contracts_capture_transitive_consume_require`
- `memory_pointer_contracts_capture_transitive_consume_return`
- `memory_pointer_contracts_capture_transitive_consume_require_ptr_generic`
- `memory_pointer_contracts_capture_transitive_consume_return_ptr_generic`
- `memory_cfg_join_marks_pointer_invalid_after_branch_side_effect_fails_in_sema`
- `memory_cfg_join_ignores_terminated_branch_and_keeps_fallthrough_state_ok`
- `root_list_family_imports_pass_in_sema`
- `root_pointer_bind_bootstrap_fixtures_pass_in_sema` (matriz positiva `pointer_phase` extendida: p2/p3/p4/p5/p6/p7/p8/p9/p10/p11)
- `root_pointer_and_strict_error_fixtures_fail_in_sema` (matriz negativa `pointer_phase` extendida, incluyendo `p9_full_system_fail` ya alineado a fallo semántico y no de parser)
- `root_pointer_error_fixtures_report_official_diagnostics` (bloquea diagnóstico esperado por fixture negativo en `pointer_phase` p2..p11)
- `lower_root_function_heap_effects_into_typed_ir_stream`
- `lower_heap_effects_preserve_argument_first_evaluation_order`
- `lower_function_pointer_contract_consume_require_into_typed_ir`
- `lower_function_pointer_contract_consume_return_into_typed_ir`
- `lower_function_pointer_contract_ptr_generic_consume_require_into_typed_ir`
- `lower_function_pointer_contract_ptr_generic_consume_return_into_typed_ir`
- `lower_mem_region_effects_into_typed_ir_stream`
- `lower_mem_region_len_snapshot_effects_preserve_order_and_types`
- `runtime_interprocedural_free_propagates_to_caller_alias_use`
- `runtime_interprocedural_alloc_return_keeps_heap_live_for_caller`
- `runtime_mem_region_bind_write_read_roundtrip`
- `runtime_mem_region_zero_materializes_typed_cells`
- `runtime_mem_region_unbound_read_fails`
- `runtime_mem_region_snapshot_isolated_from_later_writes`
- `runtime_mem_region_rebind_resets_region_material_state`
- `runtime_mem_region_zero_preserves_text_element_type`
- `runtime_mem_region_write_type_mismatch_fails`
- `runtime_mem_region_addr_accepts_null_element`
- `runtime_ptr_generic_null_projection_roundtrip`
- `runtime_ptr_generic_alloc_projection_updates_heap_actor_and_alias_metadata`
- `runtime_ptr_generic_interprocedural_free_propagates_through_addr_cast_alias`
- `root_l2k_s7_memory_runtime_fixtures_pass_in_sema`
- `root_l2k_s7_memory_runtime_error_fixtures_report_official_diagnostics`
- `runtime_l2k_s7_check_ok_fixtures_run_successfully`
- `runtime_l2k_s7_check_fail_fixtures_stay_blocked_by_sema`

Comando de validación:
1. `cargo test -p ncc-sema --lib -- --nocapture`
2. `cargo test -p ncc-ir --lib -- --nocapture`
3. `cargo test -p ncc-cli -- --nocapture`
4. `cargo test --workspace -- --nocapture`

## Cierre adicional en esta línea
- `memRegion<T>` queda integrado end-to-end en parser→AST→sema→IR→runtime con backend físico dedicado.
- matriz canónica de compatibilidad puntero/casts publicada en:
  - `docs/spec/root/pointer_compatibility_official_matrix_v0_1.md`
- matriz canónica operativa de `memory.heap.manager` publicada en:
  - `docs/spec/root/memory_heap_manager_official_matrix_v0_1.md`
