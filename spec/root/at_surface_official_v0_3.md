# nC v0.3 — [@] — superficie oficial de trabajo

## Estado
Documento operativo de implementación dentro del repo.

Estado actual de F0: parser, sema, AST root explícito, nIR-root mínimo y ejemplos base validados en WSL.

## Propósito
Fijar la superficie visible mínima y obligatoria de [@] como raíz semántica permanente de nC para implementación en parser, sema, AST y nIR-root.

## Secciones oficiales de [@]
Un contenedor [@] puede admitir únicamente estas secciones nucleares:

- `stc:`
- `binary:`
- `bind:`
- `raw { ... }`
- `fn:`
- `fa:`
- `final:`

## Principios de cierre
1. `[@]` no es `primitive[@]`.
2. `[@]` no es `[@machine]`.
3. `[@]` es la raíz semántica permanente y soberana.

## Semántica mínima por sección

### `stc:`
Zona de símbolos basales, policy, contratos estáticos y declaraciones raíz.

### `binary:`
Zona de contratos de bits, layouts, mapas y relaciones binarias basales.

### `bind:`
Zona de anclaje semántico, físico y soberano.

### `raw { ... }`
Cámara de contacto directo controlado del root.

### `fn:`
Funciones nucleares del modo raíz.

### `fa:` / `final:`
Clausura terminal del contenedor raíz.
