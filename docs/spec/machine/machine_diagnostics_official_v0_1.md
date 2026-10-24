# nC v0.1 — [@machine] — diagnósticos canónicos (parser + sema)

## Estado
Documento operativo activo y alineado con la implementación actual de `ncc`.

## Propósito
Fijar los diagnósticos canónicos vigentes para `[@machine]` sin ambigüedad entre parser y sema.

## Regla de autoridad
1. El parser es autoridad de forma en `inst:` y `final:`.
2. Sema es autoridad de validez tipada/flujo sobre AST machine ya válido.
3. No existe fallback `Raw` en el camino tipado de `[@machine]`.

## Diagnósticos canónicos de parser

| Condición | Diagnóstico canónico |
|---|---|
| Sección no permitida dentro de `[@machine]` | `section not allowed in [@machine] context` |
| Sección machine usada fuera de `[@machine]` | `section not allowed outside [@machine] context` |
| Token inválido dentro de `inst:` | `unexpected token in machine inst section: ...` |
| Instrucción vacía en `inst:` | `empty machine instruction in [@machine] inst section` |
| Instrucción inválida en `inst:` | `invalid machine instruction in [@machine] inst section: '...'` |
| Forma inválida de `mem[...]` en `load/store` | `invalid machine instruction in [@machine] inst section: '...'` |
| Operación vacía en `final:` | `empty operation in [@machine] final section` |
| Operación no permitida en `final:` | `final section in [@machine] container only allows 'ret', 'halt' or 'jmp <label>', got '...'` |

## Diagnósticos canónicos de sema

| Condición | Diagnóstico canónico |
|---|---|
| Sección `inst:` vacía | `empty inst section in [@machine] container` |
| Sección `final:` vacía | `empty final section in [@machine] container` |
| Sección `inst:` duplicada | `duplicate inst section in [@machine] container` |
| Sección `final:` duplicada | `duplicate final section in [@machine] container` |
| `final:` con más de una operación | `final section in [@machine] container requires exactly one terminal operation` |
| Registro duplicado | `duplicate reg '...' in [@machine] container` |
| Label duplicada | `duplicate label '...' in [@machine] container` |
| Instrucción inalcanzable tras terminador local | `unreachable instruction '...' after terminator in [@machine] inst block` |
| Operando de registro no declarado (formas binarias) | `... source '...' not declared in reg section of [@machine] container` o `... destination '...' not declared in reg section of [@machine] container` |
| Operando de registro no declarado (formas unitarias) | `... operand '...' not declared in reg section of [@machine] container` |
| Operando no declarado en `lea` | `lea destination '...' not declared in reg section of [@machine] container` |
| Base de memoria no declarada en `lea` | `lea memory base '...' not declared in reg section of [@machine] container` |
| Índice de memoria no declarado en `lea` | `lea memory index '...' not declared in reg section of [@machine] container` |
| Operando no declarado en `bsf` | `bsf source '...' not declared in reg section of [@machine] container` o `bsf destination '...' not declared in reg section of [@machine] container` |
| Operando no declarado en `bsr` | `bsr source '...' not declared in reg section of [@machine] container` o `bsr destination '...' not declared in reg section of [@machine] container` |
| Operando no declarado en `bswap` | `bswap operand '...' not declared in reg section of [@machine] container` |
| Base de memoria no declarada en `load/store` | `load memory base '...' not declared in reg section of [@machine] container` o `store memory base '...' not declared in reg section of [@machine] container` |
| Índice de memoria no declarado en `load/store` | `load memory index '...' not declared in reg section of [@machine] container` o `store memory index '...' not declared in reg section of [@machine] container` |
| Registro no declarado en `load/store` | `load destination '...' not declared in reg section of [@machine] container` o `store source '...' not declared in reg section of [@machine] container` |
| Salto condicional sin estado de comparación activo | `conditional jump '...' requires active compare state from 'cmp' or 'test' in current [@machine] inst block` |
| `call` inline sin ruta de retorno alcanzable | `call target '...' in [@machine] inst block has no reachable 'ret' path` |
| `call` inline con camino no-retornante | `call target '...' in [@machine] inst block has non-returning control path without guaranteed 'ret'` |
| `call` machine a símbolo root físico | `root physical symbol '...' not allowed in [@machine] context` |
| `call` machine a símbolo root inexistente | `root symbol '...' not found` |
| Target de salto/llamada no declarado | `... target '...' not declared in label section of [@machine] container` |
| `jmp` a label no declarada | `jmp target '...' not declared in label section of [@machine] container` |
| Falta sección `inst:` | `missing inst section in [@machine] container` |
| Falta sección `final:` | `missing final section in [@machine] container` |

## Terminadores locales activos (sema)
- `ret`
- `halt`
- `jmp <label>`

## Nota de coherencia de pipeline
`emit-ir` valida semántica antes de bajar a IR para mantener continuidad tipada parser → AST → sema → IR.
