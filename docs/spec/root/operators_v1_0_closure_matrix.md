# nC v1.0.0 — Matriz de cierre de operadores (inicio)

Fecha de apertura: 2026-04-30

Leyenda:

- `SI`: implementado end-to-end en línea activa (`lexer/parser/AST/sema/runtime` y/o lowering cuando aplica).
- `PARCIAL`: implementado en parte, pendiente endurecimiento o cobertura faltante.
- `NO`: no implementado en alcance general.

## Operadores y símbolos operacionales

| Operador/Símbolo | Estado | Nota de alcance actual |
|---|---|---|
| `+` | SI | aritmético binario y unario |
| `-` | SI | aritmético binario y unario |
| `*` | SI | aritmético binario |
| `/` | SI | aritmético binario (incluye control de div-by-zero runtime) |
| `%` | SI | módulo entero |
| `==` | SI | numérico/bool/pointer/string |
| `!=` | SI | numérico/bool/pointer/string |
| `<` | SI | comparación numérica |
| `>` | SI | comparación numérica |
| `<=` | SI | comparación numérica |
| `>=` | SI | comparación numérica |
| `&&` | SI | lógico con short-circuit |
| `||` | SI | lógico con short-circuit |
| `!` | SI | unario lógico |
| `&` | SI | bitwise |
| `|` | SI | bitwise y unión semántica por contexto |
| `^` | SI | bitwise |
| `~` | SI | unario bitwise |
| `<<` | SI | shift-left |
| `>>` (shift) | SI | shift-right en expresión |
| `>>>` | SI | shift-right lógico en expresión |
| `>>` (proyección nC) | SI | sentencia tipada de materialización |
| `!>` | SI | strict-check tipado |
| `@` | SI | referencia contextual (incluye rutas tipadas activas) |
| `->` | SI | retorno/dirección estructural en firma y machine call |
| `->>` | SI | `ref main`/`skkMain` tipados con `@fn(...)` y ruta estructural directa (`container.fn` / `module.path.fn`) con validación y ejecución coherente |
| `.` | SI | ruta cualificada y acceso por campo tipado |
| `:` | SI | cláusula/tipo/sección |
| `;` | SI | cierre de sentencia |
| `,` | SI | separador |
| `()` | SI | agrupación/llamadas |
| `[]` | SI | contexto/modo y indexación |
| `{}` | SI | bloque/contenedor |
| `<`/`>` (genéricos) | SI | resolución contextual con comparativos |

## Criterio de cierre v1.0.0 para este bloque

Este bloque se considera cerrado (`SI`) cuando:

1. la tabla completa esté en `SI` o justificada explícitamente como `N/A` por diseño de lenguaje;
2. exista suite canónica dedicada de operadores en parser/sema/runtime/lowering;
3. no haya ambigüedad semántica abierta en operadores polimorfos (`>>`, `|`, `<`, `>`).
