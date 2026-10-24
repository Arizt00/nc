# nC v0.1 — Linking soberano — matriz canónica de diagnósticos

## Estado
Matriz operativa y ejecutable de diagnósticos canónicos de linking en `ncc-sema`.

## Objetivo
Fijar diagnóstico textual canónico por clase de fallo y su evidencia 1:1 en tests/fixtures.

| Clase | Diagnóstico canónico | Evidencia de test | Fixture/entrada |
|---|---|---|---|
| Import path no reconocido | `unresolved import path '<path>'` | `linking_unresolved_import_path_fails_in_sema` | inline (`use system.net.ghost;`) |
| Referencia no cualificada no resuelta | `function '<name>' not found` | `linking_unqualified_missing_function_fails_in_sema` | `examples/l1a_main_missing_fail.n` |
| Ambigüedad no cualificada same-file/import | `ambiguous reference '<name>': same-file candidates [...], import candidates [...]` | `linking_unqualified_ambiguous_between_same_file_and_import_fails_in_sema` | inline |
| Ambigüedad no cualificada multi-import | `ambiguous reference '<name>': ... import candidates [alt.net.core, system.net.core]` | `linking_unqualified_ambiguous_between_two_imports_fails_in_sema`, `linking_unqualified_wildcard_overlap_multimodule_is_ambiguous_sorted`, `linking_unqualified_nc_runtime_wildcard_mark_is_ambiguous_sorted` | inline |
| Símbolo privado (same-file) | `function '<name>' is private` | `linking_l9_fail_private_fails_in_sema`, `linking_qualified_two_segment_path_private_reports_private`, `linking_expr_unqualified_private_same_file_reports_private` | `examples/linking_l9_fail_private.n`, inline |
| Autoridad inválida no cualificada | `invalid context linking 'from[@ctx]' -> 'to[@ctx]' for reference '<name>'` | `linking_unqualified_context_authority_error_reports_contexts`, `linking_expr_unqualified_context_blocked_same_file_reports_context_error` | inline |
| Autoridad inválida en import no cualificado | `invalid context linking 'from[@ctx]' -> 'import:<module.path>[@ctx]' for reference '<name>'` | `linking_unqualified_import_context_authority_error_reports_import_target`, `linking_expr_unqualified_import_context_blocked_reports_import_context_error`, `linking_unqualified_nc_runtime_wildcard_context_authority_reports_import_target` | inline |
| Contenedor same-file no encontrado | `container '<container>' not found` | `linking_l9_fail_container_fails_in_sema`, `linking_qualified_two_segment_path_requires_existing_container` | `examples/linking_l9_fail_container.n`, inline |
| Función missing en contenedor same-file cualificado | `function '<container>.<fn>' not found` | `linking_l7_same_file_qualified_missing_function_fails_in_sema` | `examples/linking_l7_fail_function_not_found.n` |
| Autoridad inválida cualificada | `invalid context linking qualified call 'from[@ctx]' -> 'to[@ctx]' for target '<target>'` | `linking_qualified_context_authority_error_reports_target`, `linking_expr_qualified_two_segment_context_reports_qualified_context_error` | inline |
| Autoridad inválida en import cualificado | `invalid context linking qualified call 'from[@ctx]' -> 'import:<module.path>[@ctx]' for target '<module.path.fn>'` | `linking_qualified_import_context_authority_error_reports_import_target`, `linking_expr_qualified_import_context_authority_error_reports_import_target` | inline |
| Ruta cualificada sin import visible | `unresolved function path '<module.path.fn>'` | `linking_qualified_import_without_visible_use_fails_unresolved_path`, `linking_expr_qualified_import_without_visible_use_fails_unresolved_path` | inline |
| Submódulo no canónico bajo wildcard | `unresolved function path '<module.path.fn>'` | `linking_qualified_import_unknown_submodule_under_wildcard_fails_unresolved_path`, `linking_expr_qualified_import_unknown_submodule_under_wildcard_fails_unresolved_path` | inline (`use system.net.*;` + `system.net.ghost.connect`) |
| Función faltante en módulo importado | `function '<fn>' not found in import '<module.path>'` | `linking_l9_fail_import_function_fails_in_sema`, `linking_expr_qualified_import_with_wildcard_overlap_reports_official_missing_function` | `examples/linking_l9_fail_import_function.n`, inline |
| Referencia root directa prohibida por contexto | `direct root reference '<root.path>' not allowed in this context` | `linking_l8_direct_root_fail_context_fails_in_sema` | `examples/linking_l8_direct_root_fail_context.n` |
| Símbolo root inexistente | `root symbol '<root.path>' not found` | `linking_l8_root_missing_symbol_fails_in_sema`, `linking_l8_direct_root_missing_symbol_fails_in_sema` | `examples/linking_l8_root_fail_missing.n`, `examples/linking_l8_direct_root_fail_missing.n` |
| Símbolo root físico no enlazado | `root physical symbol '<root.path>' not linked` | `linking_l9_fail_root_physical_fails_in_sema` | `examples/linking_l9_fail_root_physical.n` |
| Símbolo root físico no permitido desde machine | `root physical symbol '<root.path>' not allowed in [@machine] context` | `machine_call_root_physical_fails_in_sema` | `examples/machine/legacy_at/machine_at_call_root_physical_fail.n` |

## Notas de continuidad
- La matriz cubre tanto `@fn(...)` como llamadas cualificadas de expresión.
- La matriz cubre también llamada no cualificada de expresión (`Expr::Call`) en autoridad/visibilidad de same-file e import.
- El orden de `use` no altera la resolución canónica de rutas importadas cualificadas.
- La superposición directo+wildcard sobre el mismo módulo no genera ambigüedad falsa.
- El wildcard alias `use nC.runtime.*;` participa en la misma malla canónica de candidatos/autoridad
  que los módulos base (`runtime.*`, `memory.heap.manager`, `system/alt.net.core`).
- Un wildcard no materializa submódulos no canónicos: `use system.net.*;` no habilita `system.net.ghost.*`.
- La autoridad modal de imports se valida contra contexto de módulo:
  `system.net.core` / `alt.net.core` en `[@network]`, `memory.heap.manager` en `[@runtime]`;
  `[@]` root mantiene autoridad soberana.
- Los símbolos root físicos (`root.memory.map`, `root.binary.resolve`) pasan de `not linked` a
  resolubles cuando existe núcleo físico root (`binary` + `bind` no vacíos) en el programa:
  evidencia en `linking_root_physical_ref_passes_when_root_physical_core_is_present` y
  `primitive_bridge_root_physical_passes_when_root_physical_core_is_present`.
- La frontera machine se mantiene: incluso con núcleo físico root presente, `[@machine]` sigue
  rechazando símbolos root físicos (`machine_call_root_physical_still_fails_even_if_root_physical_core_exists`).
- Cobertura modal no canónica activa:
  - `linking_unqualified_same_unknown_context_passes_in_sema`
  - `linking_unqualified_distinct_unknown_context_fails_in_sema`
  - `linking_qualified_distinct_unknown_context_fails_in_sema`
  - `linking_unqualified_unknown_context_blocked_same_file_does_not_shadow_unique_import`
