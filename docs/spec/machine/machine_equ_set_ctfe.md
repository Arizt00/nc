# Machine EQU SET CTFE

`equ` defines immutable constants. `set` defines mutable assembly-time constants.

The compile-time evaluator supports integer literals, hex and binary forms, arithmetic, bitwise operations, shifts, parentheses, and references to prior constants. Divide-by-zero, overflow, and unresolved names are diagnostics:

- `E_MACHINE_CTFE_DIV_ZERO_001`
- `E_MACHINE_CTFE_OVERFLOW_001`
- `E_MACHINE_CTFE_UNRESOLVED_001`

