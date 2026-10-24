# nC v0.3 — primitive[@] — superficie oficial mínima de trabajo

## Estado
Documento operativo de implementación dentro del repo.

## Propósito
Fijar la superficie visible mínima y obligatoria de `primitive[@]` como capa de materialidad discontinua, borde físico declarativo e isonivel con hardware programable, sin confundirla con `[@]` ni con `[@machine]`.

## Principios de cierre
1. `primitive[@]` no es `[@]`.
2. `primitive[@]` no es `[@machine]`.
3. `primitive[@]` no ejecuta como dialecto casi-ISA.
4. `primitive[@]` declara materialidad, discontinuidad, borde físico y transición controlada.
5. `primitive[@]` debe poder bajar a AST e IR propias sin perder identidad.

## Superficie mínima inicial
Un contenedor `primitive[@]` puede admitir inicialmente únicamente estas piezas:

- `anchor:`
- `slot:`
- `pattern:`
- `mask:`
- `window:`
- `signal:`
- `bus:`
- `state:`
- `timing:`
- `emit:`
- `sample:`
- `latch:`
- `assert:`
- `rupture:`
- `bridge:`
- `seal:`

## Lectura semántica mínima

### `anchor:`
Punto de anclaje material o soberano.

### `slot:`
Hueco o punto de inserción material/controlado.

### `pattern:`
Patrón declarativo de forma o disposición.

### `mask:`
Máscara de exposición, filtrado o limitación material.

### `window:`
Ventana de apertura o visibilidad material controlada.

### `signal:`
Señal o pulso declarativo de interacción.

### `bus:`
Canal declarativo de interconexión material.

### `state:`
Estado material/simbólico explícito del plano primitive.

### `timing:`
Marca de ritmo o fase temporal declarativa.

### `emit:`
Punto de emisión material declarado.

### `sample:`
Punto de muestreo material declarado.

### `latch:`
Retención declarativa de estado/señal.

### `assert:`
Afirmación material declarativa.

### `rupture:`
Ruptura declarativa de continuidad material.

### `bridge:`
Puente formal entre planos o bordes.

Formas canónicas activas en esta línea:
- `root.<...>` para contratos root.
- `machine.<container>` para puente explícito hacia contenedor machine (forma exacta, sin segmentos extra).

No se permite `bridge` textual no cualificado (por ejemplo `rootToPrimitive;`).
El detalle normativo de frontera está en `primitive_bridge_contracts_v0_1.md`.

Regla estructural activa:
- `bridge:` requiere al menos una sección `anchor:`.
- `seal:` requiere `bridge:`.
- `seal:` debe ser la última sección.
- secciones vacías no están permitidas.
- duplicados por familia no están permitidos.

### `seal:`
Sellado explícito de borde, transición o superficie.

## Qué no se permite
- `stc:`
- `binary:`
- `bind:`
- `raw { ... }`
- `fn:`
- `fa:` / `final:`
- sintaxis `[@machine]`
- `probe:`
- `move fn:`
- `structure:`

## Diferencia formal mínima

### Frente a `[@]`
`[@]` define contratos basales y soberanía semántica.  
`primitive[@]` declara borde material y discontinuidad.

### Frente a `[@machine]`
`[@machine]` ejecuta.  
`primitive[@]` no ejecuta: declara, abre, limita, puentea y sella.

## Cierre mínimo de fase
La fase mínima de superficie de `primitive[@]` se considera aceptable cuando exista:
- documento operativo dentro del repo
- whitelist explícita
- blacklist explícita
- ejemplo mínimo válido
- ejemplo mínimo inválido
