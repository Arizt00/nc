# nC v0.1 — `[@primitive]` — cierre técnico (línea activa)

## Estado
`[@primitive]` queda cerrado al 100% de la **superficie activa actual del compilador** en parser/AST/sema/IR.

## Secciones soportadas
- `anchor`
- `slot`
- `pattern`
- `mask`
- `window`
- `signal`
- `bus`
- `state`
- `timing`
- `emit`
- `sample`
- `latch`
- `assert`
- `rupture`
- `bridge`
- `seal`
- `target <arch>` en cabecera de contenedor
- `type <name> = <expr>;` (alias material tipado)
- `capability <name>`
- `bitfield <name> <base>`
- `operation <path>(...)`
- `port <path>`
- `signature <name>`
- `invariant <name>`
- `constraint <name>`
- `entry <name>`

Formas materiales adicionales activas para familias canónicas:
- `anchor <name>: ...`
- `window <name>: ...`
- `signal <name>: ...`
- `bus <name>: ...`

## Directivas materiales tipadas soportadas
- `id`, `module`, `type`, `version`
- `clock`, `edge`, `cycle`, `domain`, `latency`, `timeout`
- `pin`, `port`, `wire`, `register` (`reg` como alias en parser), `width`
- `addr`, `map`, `read`, `write`, `volatile`
- `route`, `valid`, `ready`, `ack`
- `trigger`, `guard`
- `irq`, `fault`, `error`, `retry`, `fallback`
- `atomic`, `barrier`, `commit`, `reset`, `trace`
- `ptr`, `load`, `store`, `mmio`, `trap`, `fence`
- `comb`, `seq`, `assign`, `always`, `transition`, `drive`, `release`, `mux`, `decode`, `encode`, `pipeline`, `hazard`

Nombres como `pattern`, `mask`, `window`, `signal`, `bus`, `state`, `timing`, `emit`, `sample`,
`latch`, `assert`, `rupture`, `bridge` y `seal` permanecen como familias canónicas dedicadas de
`primitive[@]` (no como directiva genérica).

## Reglas semánticas activas
- Solo secciones oficiales en contexto `[@primitive]`.
- Secciones primitive fuera de `[@primitive]` fallan en parser.
- Secciones vacías fallan en sema.
- Duplicados por familia fallan en sema.
- Directivas tipadas fuera de `[@primitive]` fallan en parser.
- Secciones de directiva vacías fallan en sema.
- Duplicados de símbolo en una misma familia de directiva fallan en sema.
- `bridge`:
  - solo `root.<...>` o `machine.<container>`
  - target machine debe existir
  - target root debe resolver
- Reglas estructurales:
  - máximo una sección `bridge` por contenedor
  - máximo una sección `seal` por contenedor
  - `bridge` requiere `anchor`
  - `seal` requiere `bridge`
  - `seal` debe ser la última sección
- `bitfield`:
  - base debe ser canónica (`bit/bool`, `byte/u8/i8`, `word/u16/i16`, `dword/u32/i32`, `qword/u64/i64`, `oword/u128/i128`)
  - índices/rangos numéricos obligatorios
  - rangos dentro de base y sin solapes
- referencias materiales cruzadas tipadas:
  - `operation`:
    - `capability <path>` debe resolver a símbolo `capability` existente
    - `reads <path>` / `writes <path>` deben resolver a símbolo `port` existente
  - `port`:
    - `bus <path>` debe resolver a símbolo `bus` existente
  - `signal`:
    - `source <path>` debe resolver a símbolo `port` existente
    - `field <bitfield>.<entry>` debe resolver a entrada existente de `bitfield`
  - `signature`:
    - `window <path>` debe resolver a símbolo `window` existente
  - `entry`:
    - `anchor <path>` debe resolver a símbolo `anchor` existente
    - `requires <path>` (cuando es ruta simple) debe resolver a símbolo `signature` existente

## Pipeline tipado
- AST:
  - `PrimitiveAtSection` ampliado
  - `PrimitiveBridgeTarget`
  - `PrimitiveDirectiveSection { kind, items }` con payload tokenizado tipado (`PrimitiveDirectiveItem/Atom/Punct`)
  - clasificación canónica de cláusula material en AST (`PrimitiveClauseKind`) vía `PrimitiveDirectiveItem::clause_kind()`
  - secciones materiales tipadas con cláusulas para `anchor/window/signal/bus`
- Parser:
  - parseo explícito de familias primitive canónicas
  - parseo de directivas tipadas por familia (`PrimitiveDirectiveKind`)
  - target tipado en `bridge`
  - cabecera corta compatible para primitive:
    - `primitive[@]:`
    - `primitive[@] target <arch>:`
  - dualidad de forma en `anchor/window/signal/bus`:
    - forma simbólica `keyword:`
    - forma material `keyword <name>:` con cláusulas tipadas
    - compatibilidad de cláusula inline inicial (`keyword <name> <clause...>`)
- Sema: validación estructural, de unicidad y de continuidad modal.
- IR:
  - `PrimitiveContainerIR` ampliado por familias.
  - `PrimitiveBridgeTargetIR` tipado.
  - `PrimitiveDirectiveSectionIR` tipado con payload tokenizado (`PrimitiveDirectiveItemIR/AtomIR/PunctIR`) y metadato semántico (`family` + `scalar`).
  - lowering dedicado para `anchor/window/signal/bus` materiales (`<family>Block` + `<family>BlockClause`) en stream `PrimitiveOpIR`.
  - `PrimitiveOpIR` ordenado (`SurfaceOpen` ... `SurfaceClose`) para trazabilidad.

## Tipado semántico interno por subfamilias
Además del tipado sintáctico de directivas, `ncc-sema` clasifica y valida directivas por subfamilias:
- `timing`: `clock/edge/cycle/domain/latency/timeout`
- `memory`: `addr/register/map/read/write/volatile/ptr/load/store/mmio/trap/fence`
- `flow`: `route/atomic/barrier/commit/reset/trace/comb/seq/assign/always/transition/drive/release/mux/decode/encode/pipeline/hazard`
- `signal`: `pin/port/wire/width/valid/ready/ack/trigger/guard/irq`

Este paso ya no valida solo texto unido: cada item de directiva se reduce a `PrimitiveDirectiveSemValue`
con tipo de subfamilia y escalar tipado (`Symbol/Number/Text`), y allí se aplican reglas de contrato.

Además, en secciones materiales con cláusulas (`anchor/window/signal/bus`) y en
`operation/port/signature/invariant/constraint/entry`, si una cláusula empieza con directiva canónica,
se valida tipado de payload escalar por el mismo contrato semántico.

Además, la línea activa valida contrato de clase escalar por directiva individual:
- meta: `id/module/type` (`Symbol|Text`), `version` (`Symbol|Number|Text`)
- timing: `clock/edge/domain` (`Symbol`), `cycle/latency/timeout` (`Symbol|Number`)
- signal: `pin/port/wire/valid/ready/ack/trigger/guard/irq` (`Symbol`), `width` (`Symbol|Number`)
- memory: `addr` (`Symbol|Number`), `register/map/read/write/volatile/ptr/load/store/mmio/trap/fence` (`Symbol`)
- flow: `route/atomic/barrier/commit/reset/trace/comb/seq/assign/always/transition/drive/release/mux/decode/encode/pipeline/hazard` (`Symbol`)
- fault: `fault/error/retry/fallback` (`Symbol`)

Compatibilidad semántica de alias en cabeceras de cláusula:
- `reg` → `register`
- `address` → `addr`
- `reads` → `read`
- `writes` → `write`
- `faults` → `fault`

Escalares numéricos en la línea activa:
- decimal (`123`)
- binario (`0b1010`, con `_` opcional)
- hexadecimal (`0x03F8`, con `_` opcional)

Tipado semántico interno de cláusulas multi-token (línea activa):
- `await <expr> timeout <number> cycle[s] [else fault <name>]`
- `settle <number> cycle[s]`
- `access <symbol>`
- `risk <symbol>`
- `stable after <number> cycle[s]`
- `high means <symbol>`
- `read side_effect <symbol>`
- `capability <path>`
- `requires <expression>`
- `source <path>`
- `field <path>`
- `window <path>`
- `cache <symbol>`
- `owner <path>`
- `immutable after <symbol>`

Estas cláusulas se validan sobre atoms tipados (`PrimitiveDirectiveItem/Atom/Punct`) sin fallback textual.

En `ncc-ir`, cada cláusula tokenizada preserva además metadato tipado de clase
(derivado de la clasificación canónica de AST):
`PrimitiveClauseKindIR` (por ejemplo `AwaitTimeoutCycles`, `CapabilityPath`, `SourcePath`,
`ReadSideEffect`, `ImmutableAfter`, etc.) en `PrimitiveDirectiveItemIR.clause_kind`.

## Continuidad profunda `primitive[@]` ↔ `[@machine]` ↔ `[@]`
Cuando existen directivas por subfamilia, sema exige contratos de continuidad:
- `timing` requiere al menos un bridge `machine.<container>`.
- `signal` requiere al menos un bridge `machine.<container>` y bridge `root.policy.bind`.
- `flow` requiere al menos un bridge `machine.<container>` y bridge `root.policy.guard`.
- `fault` requiere al menos un bridge `machine.<container>` y bridge `root.policy.guard`.
- `memory` requiere bridge `root.memory.resolve`.
- `flow/fault` requieren bridge `root.policy.guard`.
- Handshake signal: `valid` exige `ready` y `ready` exige `valid`.
- Handshake signal extendido: `ack` exige `valid` y `ready`.

## Diagnósticos canónicos (primitive)
| Caso | Diagnóstico |
| --- | --- |
| sección primitive fuera de `[@primitive]` | `section not allowed outside [@primitive] context` |
| sección vacía | `empty <section> section is not allowed in [@primitive] container` |
| símbolo duplicado en familia | `duplicate <section> symbol '<name>' in [@primitive] container` |
| item de directiva con múltiples tokens | `invalid <kind> directive item '...': expected a single scalar token` |
| item de directiva con puntuación pura | `invalid <kind> directive item '...': scalar token cannot be punctuation` |
| cláusula de directiva sin payload escalar | `invalid <kind> directive item '...': expected scalar payload after directive head in [@primitive] container` |
| cláusula `await` mal formada | `invalid await clause '...': expected 'await <expr> timeout <number> cycle[s] [else fault <name>]' in [@primitive] container` |
| cláusula `settle` mal formada | `invalid settle clause '...': expected 'settle <number> cycle[s]' in [@primitive] container` |
| cláusula `access` mal formada | `invalid access clause '...': expected 'access <symbol>' in [@primitive] container` |
| cláusula `risk` mal formada | `invalid risk clause '...': expected 'risk <symbol>' in [@primitive] container` |
| cláusula `stable` mal formada | `invalid stable clause '...': expected 'stable after <number> cycle[s]' in [@primitive] container` |
| cláusula `high` mal formada | `invalid high clause '...': expected 'high means <symbol>' in [@primitive] container` |
| cláusula `read side_effect` mal formada | `invalid read side_effect clause '...': expected 'read side_effect <symbol>' in [@primitive] container` |
| cláusula `capability` mal formada | `invalid capability clause '...': expected 'capability <path>' in [@primitive] container` |
| cláusula `requires` mal formada | `invalid requires clause '...': expected 'requires <expression>' in [@primitive] container` |
| cláusula `source` mal formada | `invalid source clause '...': expected 'source <path>' in [@primitive] container` |
| cláusula `field` mal formada | `invalid field clause '...': expected 'field <path>' in [@primitive] container` |
| cláusula `window` mal formada | `invalid window clause '...': expected 'window <path>' in [@primitive] container` |
| cláusula `cache` mal formada | `invalid cache clause '...': expected 'cache <symbol>' in [@primitive] container` |
| cláusula `owner` mal formada | `invalid owner clause '...': expected 'owner <path>' in [@primitive] container` |
| cláusula `immutable` mal formada | `invalid immutable clause '...': expected 'immutable after <symbol>' in [@primitive] container` |
| referencia `operation -> capability` no resuelta | `operation '<op>' in [@primitive] references unknown capability '<path>' in clause '...'` |
| referencia `operation -> port` (`reads/writes`) no resuelta | `operation '<op>' in [@primitive] references unknown port '<path>' in clause '...'` |
| referencia `port -> bus` no resuelta | `port '<port>' in [@primitive] references unknown bus '<path>' in clause '...'` |
| referencia `signal -> source(port)` no resuelta | `signal '<signal>' in [@primitive] references unknown port '<path>' in clause '...'` |
| referencia `signal -> field(bitfield)` no resuelta | `signal '<signal>' in [@primitive] references unknown bitfield '<path>' in clause '...'` |
| referencia `signal -> field(bitfield.entry)` no resuelta | `signal '<signal>' in [@primitive] references unknown bitfield field '<bitfield>.<entry>' in clause '...'` |
| referencia `signature -> window` no resuelta | `signature '<sig>' in [@primitive] references unknown window '<path>' in clause '...'` |
| referencia `entry -> anchor` no resuelta | `entry '<entry>' in [@primitive] references unknown anchor '<path>' in clause '...'` |
| referencia `entry -> signature` no resuelta | `entry '<entry>' in [@primitive] references unknown signature '<path>' in clause '...'` |
| target bridge no canónico | `bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>', got '...'` |
| target machine bridge no encontrado | `machine bridge target 'machine.<name>' not found` |
| `seal` no último | `seal section must be the last section in [@primitive] container` |

Matriz completa de diagnósticos en:
- `docs/spec/primitive/primitive_diagnostics_official_v0_1.md`

## Cobertura de pruebas
- parser: casos mínimos, extendidos y negativos canónicos.
- sema: reglas estructurales, duplicados, bridge root/machine, errores canónicos.
- IR: lowering de superficie extendida y stream de operaciones.

## Nota de alcance
Este cierre 100% aplica a la línea activa del repo (superficie brace-based vigente) e incluye compatibilidad corta de cabecera `primitive[@]:`/`primitive[@] target <arch>:` por el mismo pipeline tipado.
Las formas canónicas extendidas más profundas del base largo (gramática radical amplia y metadata material de backend físico) se tratan como fase posterior de evolución, no como deuda de esta línea cerrada.
