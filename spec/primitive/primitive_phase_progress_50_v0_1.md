# nC v0.1 — `[@primitive]` — progreso 50%

## Estado
Cierre parcial histórico de `[@primitive]` alineado con roadmap (F3) y canon operativo vigente.

> Nota: este documento queda superado por `primitive_technical_closure_v0_1.md`, que recoge el cierre total de la superficie activa en esta línea.

## Qué queda cerrado en este 50%
- Superficie primaria activa: `anchor`, `slot`, `pattern`, `mask`, `window`, `signal`, `bridge`, `seal`.
- Parser estricto por contexto `[@primitive]` (sin mezcla con secciones de otros modos).
- `bridge` tipado en AST/IR:
  - `RootPath(Vec<String>)`
  - `MachinePath(Vec<String>)`
- Frontera `bridge` canónica:
  - solo `root.<...>`
  - solo `machine.<container>`
- Validación semántica endurecida:
  - secciones vacías no permitidas
  - símbolos duplicados por familia no permitidos
  - targets `bridge` duplicados no permitidos
  - target `machine.<container>` debe existir
  - target root se valida contra contratos root conocidos
- IR primitive con stream ordenado de operaciones (`PrimitiveOpIR`) para trazabilidad parser→AST→IR.
- Suite de tests ampliada en parser/sema/IR para casos positivos y negativos.

## Qué falta para el 100% de F3
- Gramática expandida de primitive del canon largo (formas materiales con metadata rica).
- Familia completa de nodos AST canónicos avanzados (bus/state/timing/emit/sample/latch/assert/rupture).
- IR P0 completa con metadata material (target, offset, riesgo, timing units, seal-state).
- Reglas semánticas profundas de materialidad hostil y constraints avanzados.
- Lowering/backend material específico para primitive.

## Riesgo controlado en esta etapa
- Evitar solapamiento con `[@machine]`: primitive sigue declarativo y no ejecutable.
- Evitar deriva stringly: continuidad tipada activa en la frontera de `bridge`.

## Referencias
- `docs/spec/primitive/primitive_surface_official_v0_1.md`
- `docs/spec/primitive/primitive_bridge_contracts_v0_1.md`
- `docs/status/STATUS_2026-04_machine_phase.md`
- `_archive/deprecated/context_legacy/roadmap/ROADMAP_MASTER.md` (F3)
