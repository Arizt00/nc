# nC v0.1 — F10 high-layer contracts (`[@system]` / `[@tooling]` / `[@compiler]` / `[@ai]`)

## Estado
Cierre operativo F10 activo en parser/sema/IR/CLI para la línea vigente.

## Superficie modal permitida
En `[@system]`, `[@tooling]`, `[@compiler]`, `[@ai]`:
- `stc`
- `fn`
- `probe`
- `final`
- `to/ref/skk main`

## Superficie bloqueada
- `binary`, `bind`, `raw`, `move fn`, `structure`
- familias machine/primitive/security (`reg/flag/label/inst/...`, `anchor/bridge/...`, `asset/policy/...`)

## Reglas semánticas activas
- `final` es obligatorio en cada contenedor F10.
- `final` no puede estar vacío.
- Se mantiene autoridad modal estricta en linking/import:
  - `[@]` puede enlazar a cualquier modo;
  - entre modos canónicos no-root se exige igualdad modal.

## Registro callable tipado F10
| Módulo | Contexto canónico | Funciones |
|---|---|---|
| `system.proc.core` | `[@system]` | `spawn(str) -> bool`, `wait() -> bool` |
| `system.io.core` | `[@system]` | `write(str) -> bool` |
| `tooling.build.core` | `[@tooling]` | `plan(str) -> bool`, `apply(str) -> bool` |
| `tooling.lint.core` | `[@tooling]` | `run(str) -> bool` |
| `tooling.fmt.core` | `[@tooling]` | `run(str) -> bool` |
| `tooling.pkg.core` | `[@tooling]` | `pack(str) -> bool`, `publish(str) -> bool` |
| `compiler.parse.ast` | `[@compiler]` | `load(str) -> bool` |
| `compiler.semantic.core` | `[@compiler]` | `check(str) -> bool` |
| `compiler.backend.machine` | `[@compiler]` | `materialize(str) -> bool` |

## Diagnósticos canónicos
- parser (sección prohibida):
  - `section not allowed in [@system] context`
  - `section not allowed in [@tooling] context`
  - `section not allowed in [@compiler] context`
  - `section not allowed in [@ai] context`
- sema (estructura):
  - `missing final section in [@<ctx>] container`
  - `empty final section in [@<ctx>] container`
- sema (autoridad de import):
  - `invalid import declaration '<module.path[.*]>' for context '<container[@ctx]>': no compatible target context among [@ctx, ...]`

## Evidencia de pruebas
- parser: `high_f10_tests::*` en `crates/ncc-parser/src/lib.rs`
- sema: `high_f10_sema_tests::*` en `crates/ncc-sema/src/lib.rs`
- IR: `high_f10_ir_tests::*` + `lower_function_call_links_classify_f10_import_modes_case_insensitively` en `crates/ncc-ir/src/lib.rs`
- runtime CLI: `runtime_f10_import_tests::*` en `crates/ncc-cli/src/main.rs`
- fixtures: `examples/f10_at/*`

## Nota de alcance
Este cierre cubre la superficie callable activa de F10 y su coherencia tipada end-to-end. La expansión profunda de APIs/políticas de `[@ai]` y permisos avanzados queda como evolución versionada posterior.
