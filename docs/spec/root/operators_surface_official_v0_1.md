# nC v0.1 — [@] — superficie canónica de operadores (línea activa)

## Estado
Tipado de operadores cerrado de forma coherente en `lexer -> AST -> parser -> sema -> runtime`.

## Operadores soportados

### Aritméticos binarios
- `+`
- `-`
- `*`
- `/`
- `%`

### Comparativos
- `==`
- `!=`
- `<`
- `>`
- `<=`
- `>=`

### Lógicos
- `&&`
- `||`

### Bitwise
- `&`
- `|`
- `^`
- `~` (unario)
- `<<`
- `>>` (shift-right en contexto de expresión)
- `>>>` (shift-right lógico en contexto de expresión)

### Unarios
- `+x`
- `-x`
- `!x`
- `~x`

## Operadores semánticos de nC (no degradar a binario aritmético)
- `>>` proyección tipada / materialización en sentencias (`@dest >> Tipo : expr;`)
- compatibilidad activa: también se acepta `dest >> Tipo : expr;` para símbolo simple
- `!>` chequeo estricto tipado (`@valor !> Tipo : expr;`)
- compatibilidad activa: también se acepta `valor !> Tipo : expr;` para símbolo simple
- `@` referencia contextual de símbolo/ruta viva
- rutas `@` multi-segmento no canónicas fallan (salvo `@root.<...>` válido y paths inspect canónicos)
- `->` y `->>` para direccionamiento estructural
- `|` como unión en contexto de tipos (`ptr<byte> | null`) y desvíos (`dev(a | b)`)

## Precedencia activa en parser (de menor a mayor)
1. `||`
2. `&&`
3. `|`
4. `^`
5. `&`
6. `==`, `!=`
7. `<`, `>`, `<=`, `>=`
8. `<<`, `>>` (shift)
9. `+`, `-`
10. `*`, `/`, `%`
11. unarios (`+`, `-`, `!`, `~`)

## Reglas de tipado semántico activas
- `+ - * /`: operandos numéricos; resultado `ft` si hay flotante, en otro caso `int`.
- `%`: operandos integer-like; resultado `int`.
- `& | ^`: operandos integer-like; resultado `int`.
- `<< >>`: operandos integer-like; resultado `int`.
- `&& ||`: operandos bool-like; resultado `bool`.
- `== !=`: operandos compatibles; resultado `bool`.
- `< > <= >=`: operandos numéricos; resultado `bool`.
- unario `+ -`: operando numérico.
- unario `!`: operando bool-like; resultado `bool`.
- unario `~`: operando integer-like; resultado `int`.

## Cobertura de validación
- parser:
  - `root_at_extended_operator_surface_parses`
  - `root_at_operator_precedence_for_shift_and_bitwise_is_stable`
  - `root_at_nested_generic_triple_gt_closure_and_shift_right_are_disambiguated`
- sema:
  - `root_operator_surface_extended_passes_in_sema`
  - `root_bitwise_with_non_integer_operand_fails_in_sema`
  - `root_nested_generic_triple_gt_closure_and_shift_right_passes_in_sema`
- lexer:
  - `lex_operator_surface_tokens`

## Transición a v1.0.0
- matriz de cierre mayor: `docs/spec/root/operators_v1_0_closure_matrix.md`
