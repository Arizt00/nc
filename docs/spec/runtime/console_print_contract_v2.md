# nC Runtime Console Contract (v2)

## Objetivo
Congelar el contrato semantico de la familia `print` para salida estable y mantenible.

## Surface oficial
- `print(x)` -> stdout, sin salto
- `print.sp()` -> stdout, imprime un espacio
- `print.nl()` -> stdout, imprime salto de linea
- `println(x)` -> stdout, con salto
- `println()` -> stdout, salto de linea vacio
- `print.err(x)` -> stderr, con salto
- `print.wm(x)` -> stderr, con salto
- `print.trc(x)` -> stdout, con salto, render estable de trazabilidad
- `print.dbg(x)` -> stdout, con salto, render debug
- `print.sta(x)` -> stdout, con salto, render estable de estado
- `print.inspect(x)` -> stdout, con salto, snapshot inspect tipado
- `print.mem(x)` -> stdout, con salto, snapshot memoria
- `print.st(x)` -> stdout, con salto, snapshot state
- `print.state(x)` -> stdout, con salto, alias compatible de `print.st`

## Base de resolucion
Builtins admitidos:
- `print`
- `print.sp`
- `print.nl`
- `println`
- `print.err`
- `print.wm`
- `print.trc`
- `print.dbg`
- `print.sta`
- `print.inspect`
- `print.mem`
- `print.st`
- `print.state`

Diagnostico baseico para modo desconocido:
- `invalid console builtin path`

## Contrato de render (F6)
Modos oficiales de render runtime:
- `human`: salida legible para usuario.
- `debug`: salida enriquecida por tipo (`str("...")`, `bool(...)`, `ptr(...)`, etc.).
- `inspect`: salida orientada a introspeccion (`type:value` y estructuras tipadas).
- `stable`: salida determinista para snapshots/regresion.

Politica por familia `print`:
- `print`, `println`, `print.sp`, `print.nl`, `print.err`, `print.wm` -> `human`.
- `print.dbg` -> `debug`.
- `print.inspect` -> `inspect` (mas renderer inspect de runtime).
- `print.trc`, `print.sta`, `print.st` -> `stable` (en `print.st` prima el snapshot de estado).
- `print.state` comparte contrato de `print.st`.
- `print.mem` -> renderer memoria dedicado.

## Arquitectura
- Resolver builtin, especificacion de canal y emision en `ncc-runtime`.
- `ncc-cli` solo mapea AST `PrintKind` y delega.

## Compatibilidad
La salida de color ANSI puede evolucionar, pero:
- el canal (`stdout`/`stderr`) y el contrato de salto no se rompe en minor.
- control de color runtime:
  - `NC_NO_COLOR=1|true|yes|on` desactiva ANSI.

## Output rico en `print`/`println`
La salida rica vive como valores `str` construidos por `nC.runtime.output`.
`color`, `colorBold`, `bgColor`, `style`, `emoji`, `progressBar`,
`spinnerFrame`, `table2` y `table3` no son nuevas familias `print.*`; son
constructores puros de texto que `print` y `println` emiten por el canal
canónico. Esto permite colores, emojis, barras de progreso, frames de
animacion y tablas sin mezclarlo con `print.mem`, `print.raw`, `print.inspect`
ni otros modos de inspeccion.

Suite final F7 (golden sobre binario real):
- ver `docs/spec/runtime/console_runtime_suite_f7_v2_0_0.md`.
- Matriz oficial builtin/canal/render/newline:
  - `docs/spec/runtime/console_builtin_matrix_v2_0_0.md`.
