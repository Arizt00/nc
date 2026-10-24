# Machine VEX Operand Matrix

Fase 1B define la matriz de formas de operando para el dominio VEX de Fase 1.

Cada entrada del manifest conserva su identidad de Fase 1A y queda enlazada a una o mas `VexOperandForm`. La matriz declara conteo de operandos, roles, clases de registro, anchuras vectoriales, memoria permitida, immediates, selector VEX2/VEX3, politicas L/W/pp/vvvv, features requeridas, perfiles validos y diagnostics esperados.

Esta fase no cierra encoding completo. La salida debe seguir declarando `phase1_vex_complete = not_closed` y `phase2_avx2_complete = not_started`.

## Reglas

- No existe forma generica de operando.
- `xmm` y `ymm` se validan como clases vectoriales separadas.
- Las formas de memoria declaran anchura: `m32`, `m64`, `m128` o `m256`.
- Los immediates se declaran con anchura explicita, actualmente `imm8`.
- Las formas invalidas tienen diagnostic esperado.
- Las entradas fuera del dominio de Fase 1 siguen en `unsupported_by_design`.

## Comandos

- `ncc machine vex operand-forms --json`
- `ncc machine vex invalid-forms --json`
- `ncc machine vex coverage --json`

La matriz es evidencia de forma, no de bytes finales.
