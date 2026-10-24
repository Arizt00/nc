# nC v0.1 — `[@]` root — cierre técnico (línea activa)

## Estado
`[@]` root cerrado al 100% en la línea activa del repo para el tramo parser/AST/sema/IR/tests/docs.

## Superficie activa
Secciones root nucleares activas:
- `stc:`
- `binary:`
- `bind:`
- `raw { ... }` y `raw:`
- `probe:`
- `fn:`
- `fa:` / `final:`

Entradas transversales activas en root:
- `to main { ... }`
- `ref main ->> ...;`
- `skk main ->> @fn(...);`

Compatibilidad de suite madura activa:
- fixture canónico integral `examples/root_at/root_at_mature_suite_ok.n` (bootstrap/memoria/alias/perimeter/linking handoff).

## Cierre por capa
- AST: `RootAtSection` tipado, incluyendo `MainEntry`.
- Parser: parseo explícito de `fa`/`final` y entradas `to/ref/skk main`.
- Parser/AST/Sema/IR:
  - `stc` acepta declaración tipada e inicializador tipado por proyección (`name >> type : expr`), incluida forma `strictly:`.
  - proyección/strict-check sin prefijo `@` activa para símbolo simple (`name >> type : expr`, `name !> type : expr`) como compatibilidad canónica.
- Parser/AST/Sema/Runtime:
  - operadores aritméticos/lógicos/bitwise/unarios tipados y con precedencia canónica en la línea activa.
  - tabla oficial: `docs/spec/root/operators_surface_official_v0_1.md`.
- Sema:
  - `binary`, `bind`, `raw` solo válidos en contexto `[@]`.
  - `move fn`, `structure` prohibidos en contexto root.
  - `probe` activo en contexto root (sin degradar camino tipado).
  - `@` referencial endurecido para rutas multi-segmento:
    - `@root.<...>` valida autoridad/contrato root canónico.
    - rutas inspect runtime canónicas (`@heap.events.*`, `@heap.live.allocs`, `@heap.alloc.count`, `@heap.blocks.count`, `@net.*`) validadas tipadamente.
    - rutas `@a.b` no canónicas fallan con diagnóstico explícito.
  - clausura obligatoria: root exige exactamente una sección `fa/final`.
  - referencias directas `root.*` validadas con autoridad modal vigente.
- IR:
  - `RootContainerIR` tipado.
  - `RootMainEntryIR` + `RootMainEntryKindIR` para preservar entradas root en lowering.

## Diagnósticos canónicos root
- `multiple fa/final sections are not allowed in the same container`
- `missing fa/final section in [@] root container`
- `binary section is only allowed in [@] root context`
- `bind section is only allowed in [@] root context`
- `raw section is only allowed in [@] root context`
- `move fn section is not allowed in [@] root context`
- `structure section is not allowed in [@] root context`

## Evidencia de cierre
Validado con:
- `cargo test -p ncc-ast --lib -- --nocapture`
- `cargo test -p ncc-parser --lib -- --nocapture`
- `cargo test -p ncc-sema --lib -- --nocapture`
- `cargo test -p ncc-ir --lib -- --nocapture`
- `cargo test -p ncc-parser root_at_mature_suite_ok_parses -- --nocapture`
- `cargo test -p ncc-sema root_at_mature_suite_ok_passes_in_sema -- --nocapture`
- `cargo test -p ncc-ir lower_root_mature_suite_fixture_into_typed_ir -- --nocapture`
- `cargo run -p ncc-cli -- check examples/root_at/root_at_mature_suite_ok.n`
- `cargo test --workspace -- --nocapture`
