# Machine VEX Verifier Operand Rules

El verifier VEX usa la matriz `VexOperandForm` como fuente de verdad de Fase 1B.

## Validacion

1. La instruccion se busca en el manifest VEX de 104 entradas.
2. Las entradas `unsupported_by_design` se rechazan con diagnostic dedicado.
3. Se compara el conteo de operandos contra la matriz.
4. Se clasifica cada operando como `xmm`, `ymm`, memoria con anchura, immediate o registro escalar.
5. Se valida que la forma exista.
6. Se valida que `xmm` y `ymm` no se mezclen en una misma forma vectorial.
7. Se valida anchura de memoria.
8. Se valida anchura de immediate.
9. Se valida feature gate.

## Estado

Fase 1B cierra forma y diagnosticabilidad. Encoding exhaustivo, decoder y roundtrip quedan fuera de esta fase y mantienen Fase 1 global en `not_closed`.
