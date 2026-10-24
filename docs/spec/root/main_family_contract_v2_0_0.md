# nC Familia `main` — Contrato Cerrado (v2.0.0)

## Alcance
Este documento congela la familia `main` como contrato operativo formal en parser, sema, runtime y CLI.

## Variantes activas oficiales
- `to main { ... }`
- `to main@<profile> { ... }`
- `ref main ->> ...;`
- `ref main ->> ruta.directa`
- `skkMain ->> @fn(...)`
- `skkMain ->> ruta.directa`

Compatibilidad activa:
- `skk main ->> ...` (alias parseado y normalizado a `SkkFn` en AST)

## Restricciones estructurales (`to main` y `to main@<profile>`)
Las entradas inline quedan cerradas con límites duros:
- máximo de statements top-level: `16`
- máximo de statements lógicos totales: `20`
- nesting máximo: `2`
- máximo de loops: `1`
- loops anidados: `prohibidos`
- máximo de llamadas directas: `6`
- lógica pesada: `prohibida` (diagnóstico canónico)

Estas restricciones aplican igual a:
- `to main { ... }`
- `to main@<profile> { ... }`

## Restricciones de `ref main`
- `ref main` no admite cuerpo inline.
- el target debe existir, ser resoluble y callable.
- firma de target: `0` parámetros.
- contrato de retorno: `bool` / `int` / `void` (`none`).

## Restricciones de `skk main`
- `skk main` solo está permitido en contexto soberano `[@]`.
- duplicado de `skk main` en la misma unidad efectiva: error canónico.
- conflicto entre `skk main` y otra entrada efectiva default (`to main` / `ref main`): error canónico.

## Restricciones de perfil
- nombre de perfil válido: segmentos `lowercase` separados por `.`
- cada segmento debe iniciar en letra `[a-z]` y solo permite `[a-z0-9_]`
- duplicado de perfil: error canónico

## Prioridad y selección
Orden de selección en `run`:
1. si se pidió `--main-profile`, se selecciona solo entrada perfilada que coincida;
2. si no hay perfil pedido, se usa entrada default (`to main` / `ref main` / `skkMain`);
3. si no hay default y existe una única perfilada, se ejecuta;
4. si hay múltiples perfiladas sin selección explícita, error canónico.

## Contrato de retorno de entrada
Retornos válidos de entrypoint:
- `bool`
- `int` (y enteros compatibles)
- `void`/`none`

Retornos no válidos:
- `str`, `ft*`, colecciones, records, otros tipos no contratos de proceso.

Aplicación del contrato:
- sema: `ref main`/`skkMain` valida firma target (`0 params`) y retorno permitido;
- runtime: valida contrato de retorno para toda entrada efectiva (`to main`, `to main@profile`, `ref main`, `skkMain`).

## Diagnósticos canónicos
- `missing main entry in program`
- `multiple main entries found; program entry is ambiguous`
- `duplicate main profile '<profile>'`
- `invalid main profile name '<profile>'`
- `multiple profiled main entries found; select a profile explicitly`
- `requested profile '<profile>' has no valid main entry`
- `ref main cannot declare an inline body`
- `skk main is only allowed in sovereign entry contexts`
- `duplicate skk main entry`
- `skk main conflicts with another effective program entry`
- `to main exceeds maximum logical statement budget (max: 20, found: N)`
- `to main contains too many statements; move logic into fn:`
- `to main is too deeply nested; entrypoints must remain shallow`
- `to main cannot contain more than one loop`
- `to main cannot contain nested loops`
- `to main contains too many direct calls`
- `to main contains heavy logic; move behavior into fn:`
- `entry target has invalid signature for ref main: expected 0 params, got N in '<path>'`
- `entry target has invalid signature for skkMain: expected 0 params, got N in '<path>'`
- `entry target has invalid return contract for ref main: expected bool/int/void, got <type> in '<path>'`
- `entry target has invalid return contract for skkMain: expected bool/int/void, got <type> in '<path>'`
- `entry target has invalid return contract for to main: expected bool/int/void, got <value/type>`

## Evidencia de validación (suite)
- unitarios sema y runtime en `crates/ncc-sema/src/lib.rs` y `crates/ncc-cli/src/lib.rs`.
- golden CLI:
  - `crates/ncc-cli/tests/main_family_golden.rs`
  - fixtures: `examples/main_family_v2_0_0/*.n`

Comando:
```bash
cargo test -p ncc-cli --test main_family_golden -- --nocapture
```
