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
- `probe:`
- `fn:`
- `fa:`
- `final:`

Operadores de entrada transversales ya activos (parse/sema):
- `to main { ... }`
- `ref main ->> ...;`
- `skk main ->> @fn(...);`

## Principios de cierre
1. `[@]` no es `primitive[@]`.
2. `[@]` no es `[@machine]`.
3. `[@]` es la raíz semántica permanente y soberana.

## Semántica mínima por sección

### `stc:`
Zona de símbolos basales, policy, contratos estáticos y declaraciones raíz.
Formas activas en la línea:
- `name: type;`
- `name >> type : expr;`
- `strictly name: type;`
- `strictly name >> type : expr;`
- `strictly: name: type;` (compatibilidad canónica)

### `binary:`
Zona de contratos de bits, layouts, mapas y relaciones binarias basales.

### `bind:`
Zona de anclaje semántico, físico y soberano.

### `raw { ... }`
Cámara de contacto directo controlado del root.

### `probe:`
Zona de verificación/handshake root previa a clausura; permite flujo de control y llamadas con linking soberano.

### `fn:`
Funciones nucleares del modo raíz.

### `fa:` / `final:`
Clausura terminal del contenedor raíz.

Regla canónica vigente:
- un contenedor `[@]` debe declarar exactamente una clausura `fa:`/`final:`.

## Linking soberano (referencia operativa)
Precedencia y autoridad de resolución activas documentadas en:

- [linking_resolution_precedence_v0_1.md](./linking_resolution_precedence_v0_1.md)
