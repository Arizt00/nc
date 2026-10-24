# nC v0.1 — primitive[@] — superficie oficial activa

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

## Superficie activa actual
Un contenedor `primitive[@]` admite estas familias canónicas:

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

Además de la forma simbólica `keyword:`, la línea activa soporta forma material
`keyword <name>:` para estas familias:

- `anchor <name>: ...`
- `window <name>: ...`
- `signal <name>: ...`
- `bus <name>: ...`

Compatibilidad adicional activa para cláusula inline inicial en esas mismas familias:

- `anchor <name> <clause...>` seguido de cláusulas del bloque
- `window <name> <clause...>` seguido de cláusulas del bloque
- `signal <name> <clause...>` seguido de cláusulas del bloque
- `bus <name> <clause...>` seguido de cláusulas del bloque

Estas formas materiales preservan cláusulas tokenizadas tipadas (`PrimitiveDirectiveItem`) sin
degradar a parsing stringly.

Secciones materiales tipadas adicionales activas:

- `target <arch>` en cabecera de contenedor `[@primitive]`
- `type <name> = <expr>;` como alias material tipado
- `capability <name>:`
- `bitfield <name> <base>:`
- `operation <path>(...):`
- `port <path>:`
- `signature <name>:`
- `invariant <name>:`
- `constraint <name>:`
- `entry <name>:`

Compatibilidad de cabecera corta activa para `primitive`:
- `primitive[@]:`
- `primitive[@] target <arch>:`

Estas formas cortas son equivalentes semánticamente al contenedor `[@primitive]` brace-based y
se validan por el mismo camino tipado de parser/AST/sema/IR.

Bases canónicas activas para `bitfield`:
- `bit`, `bool`
- `byte`, `u8`, `i8`
- `word`, `u16`, `i16`
- `dword`, `u32`, `i32`
- `qword`, `u64`, `i64`
- `oword`, `u128`, `i128`

Validación semántica activa de `bitfield`:
- índices/rangos deben ser numéricos (`decimal`, `0b...` o `0x...`, con `_` opcional);
- rango debe cumplir `start <= end`;
- índices/rangos deben estar dentro del ancho de la base;
- no se permiten solapes de bits entre entradas.

Validación semántica activa de directivas tipadas:
- contrato de clase escalar por directiva (`Symbol`, `Number`, `Text`) además del tipado por subfamilia;
- diagnósticos canónicos cuando una directiva recibe una clase de escalar incompatible.
- en cláusulas materiales, si la cabecera es directiva canónica, se aplica el mismo contrato escalar
  (incluye alias de cabecera: `reg/address/reads/writes/faults`).

Validación semántica activa de cláusulas materiales multi-token:
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

Además, la línea activa soporta directivas materiales tipadas en sección propia
(`PrimitiveDirectiveSection`) para cubrir metadata operativa sin degradación stringly:

- `id:`
- `module:`
- `type:`
- `version:`
- `clock:`
- `edge:`
- `cycle:`
- `domain:`
- `latency:`
- `timeout:`
- `pin:`
- `port:`
- `wire:`
- `reg:` (alias corto de familia `register`)
- `width:`
- `addr:`
- `register:`
- `map:`
- `read:`
- `write:`
- `volatile:`
- `route:`
- `valid:`
- `ready:`
- `ack:`
- `trigger:`
- `guard:`
- `irq:`
- `fault:`
- `error:`
- `retry:`
- `fallback:`
- `atomic:`
- `barrier:`
- `commit:`
- `reset:`
- `trace:`
- `ptr:`
- `load:`
- `store:`
- `mmio:`
- `trap:`
- `fence:`
- `comb:`
- `seq:`
- `assign:`
- `always:`
- `transition:`
- `drive:`
- `release:`
- `mux:`
- `decode:`
- `encode:`
- `pipeline:`
- `hazard:`

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
- en directivas tipadas, duplicados se validan por familia de directiva.
- continuidad modal activa por subfamilia:
  - `timing`, `signal`, `flow` y `fault` requieren bridge `machine.<container>`.
  - `signal` requiere `root.policy.bind`.
  - `flow` y `fault` requieren `root.policy.guard`.
  - `memory` requiere `root.memory.resolve`.
- handshake signal activo:
  - `valid` requiere `ready`.
  - `ready` requiere `valid`.
  - `ack` requiere ambos (`valid` y `ready`).

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
- matriz canónica de diagnósticos (`primitive_diagnostics_official_v0_1.md`)
