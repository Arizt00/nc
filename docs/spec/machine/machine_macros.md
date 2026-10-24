# Machine Macros

ASM-complete macro support requires parameterized macros, variadic macros, local label hygiene, conditional expansion, repetition, compile-time expressions, expansion reports, recursion limits, and diagnostics.

Current status: closed for Phase 10 declared macro/directive profiles.

Required diagnostics:

- `E_MACHINE_MACRO_001`
- `E_MACHINE_MACRO_ARG_001`
- `E_MACHINE_MACRO_RECURSION_001`
- `E_MACHINE_MACRO_LABEL_COLLISION_001`
- `E_MACHINE_MACRO_EXPANSION_001`

Macros must expand into normal MachineIR and pass the same verifier as hand-written instructions.
