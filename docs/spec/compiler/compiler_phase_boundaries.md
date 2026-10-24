# Compiler Phase Boundaries

## Reglas de frontera
- Salida de lexer: tokens válidos.
- Salida de parser: AST estructurada.
- Salida de sema: constraints y diagnósticos.
- Salida nIR: sin placeholders prohibidos.
- Entrada backend: nIR válida únicamente.
