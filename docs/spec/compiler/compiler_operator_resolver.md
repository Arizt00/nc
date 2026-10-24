# Compiler Operator Resolver

## Cobertura
Aritméticos, comparación, lógicos, bitwise, casts, indexación, llamada, proyección y concat.

## Reglas
- precedence/associativity explícitas,
- `if/while` no aceptan placeholders,
- `str + str` permitido en `[@runtime]` y contextos altos equivalentes,
- `str + non-str` falla con ruta explícita requerida (`concat` + `render*`),
- `str + str` prohibido en `[@]`, `[@machine]`, `primitive[@]`,
- concat sin regla de render falla.
