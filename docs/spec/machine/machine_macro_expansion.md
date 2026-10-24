# Machine Macro Expansion

Expansion is a structured MachineIR step. The source macro invocation records name and arguments, the expansion stores emitted instructions and label maps, and the final instruction stream is encoded normally.

Recursive self-expansion is rejected by `E_MACHINE_MACRO_RECURSION_001`. Expansion count and generated instruction lists are exposed through `ncc machine macro-report` and `ncc machine expand`.

Roundtrip compares the expanded semantics. Reconstructing original macro spelling is outside Phase 10 unless future metadata chooses to preserve it.

