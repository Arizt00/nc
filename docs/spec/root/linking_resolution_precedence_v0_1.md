# nC v0.1 — Linking soberano — precedencia y autoridad de resolución

## Estado
Documento operativo de precedencia para el resolvedor semántico actual de `ncc`.

## Propósito
Fijar orden de resolución y reglas de autoridad de contexto para evitar deriva entre código, tests y base.

## Alcance
Las mismas reglas de precedencia/autoridad aplican a:
- referencia semántica `@fn(...)` (main/linking).
- llamada no cualificada de expresión (`name(...)`).
- llamada cualificada de expresión (`container.fn(...)` y `module.path.fn(...)`).

## Orden de resolución activo (referencia `@fn(...)`)
Para referencia no vacía:

1. Si la ruta empieza en `root.*`, se resuelve como referencia root directa y aplica regla de contexto root/machine.
2. Si la ruta es no cualificada (`@fn(name)`):
   - prioridad 1: función local en el contenedor actual.
   - prioridad 2: candidatos same-file (otros contenedores del mismo archivo) e imports visibles.
   - los imports wildcard (`use module.path.*;`) aportan candidatos por prefijo sobre módulos conocidos.
   - los candidatos import solo cuentan si su contexto modal de módulo es compatible con el contexto actual.
   - para ambigüedad no cualificada solo cuentan candidatos same-file accesibles (no `private` y con contexto compatible).
   - un candidato same-file bloqueado por visibilidad o contexto no debe eclipsar un único candidato importado válido.
   - un único import visible pero bloqueado por autoridad modal reporta error canónico de contexto.
   - si no hay candidatos: error `function 'name' not found`.
   - si hay más de un candidato entre same-file e imports: error de ambigüedad.
   - si hay un único candidato same-file: aplica visibilidad y autoridad de contexto.
   - si hay un único candidato importado: resolución válida.
3. Si la ruta tiene forma `container.fn` y `container` existe en same-file:
   - resolución same-file cualificada.
   - aplica visibilidad y autoridad de contexto.
   - prioridad: `container.fn` (2 segmentos) pertenece al namespace de contenedor same-file, no al namespace de import.
   - si el contenedor no existe, el diagnóstico canónico es `container 'container' not found`.
4. Si la ruta es cualificada importada (`module.path.fn`):
   - debe existir `use` compatible (directo o wildcard).
   - wildcard no materializa submódulos no canónicos (por ejemplo, `use system.net.*;` no habilita `system.net.ghost.*`).
   - la función debe estar exportada por el módulo conocido.
   - debe pasar autoridad modal contra el contexto del módulo importado.
   - la resolución es independiente del orden de declaraciones `use`; se evalúa visibilidad del módulo de forma canónica.
   - wildcard superpuesto sobre el mismo módulo (`use module.path.core;` + `use module.path.*;`) no crea ambigüedad falsa.
5. Si no encaja en ninguna de las rutas anteriores: error `unresolved function path '...'`.

## Autoridad de contexto activa
- enlaces siempre permitidos cuando alguno de los lados es `[@]` root.
- enlaces entre contextos no canónicos (por ejemplo `[@custom]`, `[@edge]`) solo son válidos si comparten exactamente el mismo label.
- un contexto no canónico no tiene autoridad implícita para enlazar contra contextos nombrados (`[@system]`, `[@runtime]`, etc.).
- para el resto de contextos, el enlace exige igualdad modal (`[@system]`→`[@system]`, etc.).

## Reglas root directas activas
- `[@]` puede resolver símbolos root implícitos.
- `[@]` puede resolver símbolos root físicos cuando existe núcleo físico root en el programa
  (al menos un contenedor `[@]` con secciones `binary` y `bind` no vacías).
- `[@machine]` solo puede usar root implícito en su frontera autorizada.
- otros contextos no pueden usar referencia directa `root.*`.

## Diagnósticos canónicos de precedencia/autoridad
- ambigüedad no cualificada:
  - `ambiguous reference 'name': same-file candidates [...], import candidates [...]`
- autoridad de contexto no cualificada:
  - `invalid context linking 'from[@ctx]' -> 'to[@ctx]' for reference 'name'`
- autoridad de contexto en llamada cualificada same-file:
  - `invalid context linking qualified call 'from[@ctx]' -> 'to[@ctx]' for target 'container.fn'`
- autoridad de contexto en import no cualificado:
  - `invalid context linking 'from[@ctx]' -> 'import:module.path[@ctx]' for reference 'name'`
- autoridad de contexto en import cualificado:
  - `invalid context linking qualified call 'from[@ctx]' -> 'import:module.path[@ctx]' for target 'module.path.fn'`

## Notas de continuidad
- este documento describe el comportamiento vigente y probado; no adelanta semántica futura.
- la matriz de tests en `ncc-sema` es la evidencia ejecutable de estas reglas.
- la matriz canónica de diagnósticos está publicada en `docs/spec/root/linking_diagnostics_official_matrix_v0_1.md`.
- la matriz modal de autoridad está publicada en `docs/spec/root/linking_context_authority_matrix_v0_1.md`.
- cobertura adicional activa: resolución no cualificada desde wildcard import (`use ...*`) y ambigüedad same-file vs wildcard import.
- prefijos wildcard activos en esta línea: `system.net.*`, `alt.net.*`, `memory.heap.*`, `nC.runtime.*` (alias canónico hacia módulos runtime/network/memory conocidos).
- cobertura adicional activa para wildcard alias `nC.runtime.*`:
  - resolución no cualificada única (`alloc`) sobre `memory.heap.manager`;
  - ambigüedad no cualificada canónica (`mark`) con candidatos `[runtime.alias, runtime.trace]`;
  - autoridad modal en import no cualificado (`[@system]` -> `import:memory.heap.manager[@runtime]`) con diagnóstico canónico.
- cobertura adicional activa: un same-file `private` o contextualmente bloqueado no eclipsa un único import no cualificado válido.
- cobertura adicional activa: prioridad canónica de `container.fn` sobre namespace import y diagnóstico de contenedor ausente en ruta cualificada de 2 segmentos.
- cobertura adicional activa: la misma prioridad/diagnóstico de `container.fn` está testeada también en llamadas cualificadas de expresión (`system.connect()`).
- cobertura adicional activa: la llamada no cualificada de expresión (`name(...)`) respeta la misma malla de visibilidad/autoridad que `@fn(name)`:
  - `private` same-file => diagnóstico canónico de privado;
  - same-file bloqueado por contexto => diagnóstico canónico de contexto;
  - import no cualificado bloqueado por contexto => diagnóstico canónico `import:<module.path>[@ctx]`.
- cobertura adicional activa: wildcard superpuesto en import cualificado (directo + wildcard del mismo módulo) mantiene resolución estable.
- cobertura adicional activa: ambigüedad no cualificada multi-módulo con wildcard superpuesto mantiene lista canónica ordenada de candidatos import.
- cobertura adicional activa: autoridad modal de imports con diagnóstico canónico en:
  `linking_unqualified_import_context_authority_error_reports_import_target`,
  `linking_qualified_import_context_authority_error_reports_import_target`,
  `linking_expr_qualified_import_context_authority_error_reports_import_target`.
- cobertura adicional activa en `Expr::Call`:
  `linking_expr_unqualified_private_same_file_reports_private`,
  `linking_expr_unqualified_context_blocked_same_file_reports_context_error`,
  `linking_expr_unqualified_import_context_blocked_reports_import_context_error`,
  `linking_expr_unqualified_import_network_context_passes_in_sema`.
