# Machine Reserve Directives

Phase 10 supports `resb`, `resw`, `resd`, and `resq`.

Reserve directives lower to BSS-sized MachineIR records. Object formats with section support receive `.bss` symbols; flat profiles can later choose explicit zero fill by policy. Counts are compile-time expressions and cannot be negative.

