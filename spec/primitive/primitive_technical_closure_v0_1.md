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

## Reglas semánticas activas
- Solo secciones oficiales en contexto `[@primitive]`.
- Secciones primitive fuera de `[@primitive]` fallan en parser.
- Secciones vacías fallan en sema.
- Duplicados por familia fallan en sema.
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

## Pipeline tipado
- AST: `PrimitiveAtSection` ampliado + `PrimitiveBridgeTarget`.
- Parser: parseo explícito de todas las secciones primitive y target tipado en `bridge`.
- Sema: validación estructural, de unicidad y de continuidad modal.
- IR:
  - `PrimitiveContainerIR` ampliado por familias.
  - `PrimitiveBridgeTargetIR` tipado.
  - `PrimitiveOpIR` ordenado (`SurfaceOpen` ... `SurfaceClose`) para trazabilidad.

## Cobertura de pruebas
- parser: casos mínimos, extendidos y negativos canónicos.
- sema: reglas estructurales, duplicados, bridge root/machine, errores canónicos.
- IR: lowering de superficie extendida y stream de operaciones.

## Nota de alcance
Este cierre 100% aplica a la línea activa del repo (superficie brace-based vigente). Las formas canónicas extendidas del canon largo (gramática radical alternativa y metadata material profunda de backend) se tratan como fase posterior de evolución, no como deuda de esta línea cerrada.
