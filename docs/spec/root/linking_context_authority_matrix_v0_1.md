# nC v0.1 — Linking soberano — matriz de autoridad de contexto

## Estado
Matriz operativa de autoridad modal para resolución same-file e importada
(`@fn(...)`, `container.fn(...)`, `module.path.fn(...)`).

## Regla base activa
- si alguno de los lados es `[@]`, el enlace modal es válido.
- entre contextos canónicos no-root, el enlace exige igualdad exacta de contexto.
- entre contextos no canónicos (labels libres), el enlace solo es válido si el label coincide exactamente.
- un contexto no canónico no puede enlazar con contexto canónico no-root.

## Matriz resumida
| Desde | Hacia | Permitido |
|---|---|---|
| `[@]` | cualquiera | sí |
| cualquiera | `[@]` | sí |
| `[@machine]` | `[@machine]` | sí |
| `[@system]` | `[@runtime]` | no |
| `[@customA]` | `[@customA]` | sí |
| `[@customA]` | `[@customB]` | no |
| `[@custom]` | `[@system]` | no |
| `[@system]` | `[@custom]` | no |
| `[@system]` | `import:system.net.core[@network]` | no |
| `[@network]` | `import:system.net.core[@network]` | sí |
| `[@runtime]` | `import:memory.heap.manager[@runtime]` | sí |

## Diagnóstico canónico de fallo modal
- declaración `use/import` (cuando el contenedor consume superficie importable):
  - `invalid import declaration '<module.path[.*]>' for context '<container[@ctx]>': no compatible target context among [@ctx, ...]`
- no cualificada:
  - `invalid context linking 'from[@ctx]' -> 'to[@ctx]' for reference '<name>'`
- cualificada:
  - `invalid context linking qualified call 'from[@ctx]' -> 'to[@ctx]' for target '<container.fn>'`
- import no cualificado:
  - `invalid context linking 'from[@ctx]' -> 'import:<module.path>[@ctx]' for reference '<name>'`
- import cualificado:
  - `invalid context linking qualified call 'from[@ctx]' -> 'import:<module.path>[@ctx]' for target '<module.path.fn>'`

## Evidencia de pruebas
- canónico↔canónico bloqueado:
  - `linking_unqualified_context_authority_error_reports_contexts`
  - `linking_qualified_context_authority_error_reports_target`
- barrido matricial canónico completo:
  - `linking_unqualified_same_official_context_matrix_passes_in_sema`
  - `linking_unqualified_cross_official_context_matrix_fails_in_sema`
  - `linking_qualified_cross_official_context_matrix_fails_in_sema`
- no canónico↔no canónico:
  - `linking_unqualified_same_unknown_context_passes_in_sema`
  - `linking_unqualified_distinct_unknown_context_fails_in_sema`
  - `linking_qualified_distinct_unknown_context_fails_in_sema`
- no canónico bloqueado no eclipsa import único:
  - `linking_unqualified_unknown_context_blocked_same_file_does_not_shadow_unique_import`
- autoridad modal en imports:
  - `linking_unqualified_import_context_authority_error_reports_import_target`
  - `linking_unqualified_nc_runtime_wildcard_context_authority_reports_import_target`
  - `linking_qualified_import_context_authority_error_reports_import_target`
  - `linking_expr_qualified_import_context_authority_error_reports_import_target`

## Notas
- autoridad en declaración `use/import`:
  - se valida por contenedor únicamente cuando ese contenedor contiene referencias/llamadas que pueden consumir imports (evita bloquear imports globales no usados por contenedores auxiliares del mismo archivo).
  - el diagnóstico de declaración ocurre antes de validaciones de linking de llamada cuando aplica.
- en imports, contexto de módulo activo:
  `system.net.core` y `alt.net.core` en `[@network]`, `memory.heap.manager` en `[@runtime]`.
- wildcard alias activo `use nC.runtime.*;` participa en la misma malla modal:
  `linking_unqualified_nc_runtime_wildcard_unique_passes_in_sema`,
  `linking_unqualified_nc_runtime_wildcard_mark_is_ambiguous_sorted`,
  `linking_unqualified_nc_runtime_wildcard_context_authority_reports_import_target`.
- las reglas root directas (`root.*`) se rigen además por su frontera específica (`[@]` / `[@machine]` / otros).
