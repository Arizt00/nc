# nC Console Builtin Matrix (v2.0.0)

## Objetivo
Congelar de forma auditable el mapeo:
- builtin de superficie;
- canal (`stdout`/`stderr`);
- modo de render (`Human`/`Debug`/`Inspect`/`Stable`);
- terminación de línea.

## Pipeline canónico único
Toda salida de consola válida sigue este camino:
1. parser (`print*` -> `Stmt::Print` + `PrintKind`)
2. sema (contratos de contexto/tipos/paths inspect)
3. runtime evaluation (valor)
4. selección de render (por `PrintKind`)
5. dispatch de consola (`emit_print` -> `ConsoleBuiltinSpec`)
6. emisión final a `stdout` o `stderr`

No hay rutas paralelas autorizadas para la familia `print`.

## Matriz oficial
| Builtin de superficie | Canal | Render mode | Newline | Notas |
|---|---|---|---|---|
| `print(x)` | `stdout` | `Human` | no | salida humana directa |
| `println(x)` | `stdout` | `Human` | sí | salida humana por línea |
| `println()` | `stdout` | `Human` | sí | línea vacía |
| `print.sp()` | `stdout` | `Human` | no | imprime espacio |
| `print.nl()` | `stdout` | `Human` | sí | salto explícito |
| `print.err(x)` | `stderr` | `Human` | sí | error operativo |
| `print.wm(x)` | `stderr` | `Human` | sí | warning operativo |
| `print.trc(x)` | `stdout` | `Stable` | sí | traza reproducible |
| `print.dbg(x)` | `stdout` | `Debug` | sí | depuración estructurada |
| `print.sta(x)` | `stdout` | `Stable` | sí | estado determinista |
| `print.inspect(x)` | `stdout` | `Inspect` | sí | inspección tipada runtime |
| `print.mem(x)` | `stdout` | `Inspect` | sí | snapshot de memoria |
| `print.st(x)` | `stdout` | `Inspect` (snapshot) | sí | estado global runtime |
| `print.state(x)` | `stdout` | `Inspect` (snapshot) | sí | alias compatible de `print.st` |

## Política humano vs estructurado
- Humanos: `print`, `println`, `print.err`, `print.wm`.
- Estructurados: `print.trc`, `print.dbg`, `print.sta`, `print.inspect`, `print.mem`, `print.st`/`print.state`.

## Compatibilidad
- `print.state` es alias canónico de compatibilidad para `print.st`.
- El contrato de canal/render/newline no debe romper en minor.
