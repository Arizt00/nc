# Machine Flags

Flag producers include `cmp`, `test`, `add`, `sub`, `adc`, `sbb`, `and`, `or`,
`xor`, `imul`, `mul`, `neg`, `inc`, `dec`, `shl`, `shr`, and `sar`.

Conditional jumps require known flag state. A conditional jump without a prior producer fails with `E_MACHINE_FLAGS_UNINITIALIZED_001`.

The report surface records instruction rows and flag effects for the supported x86_64 subset.
