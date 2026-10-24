# Machine Conditional Assembly

Conditional assembly covers target, feature, ABI, and profile branches plus compile-time expressions.

Current status: closed for Phase 10.

Required forms:

- `if-target`
- `if-feature`
- `if-abi`
- `if-profile`
- compile-time arithmetic and bitwise expressions
- symbol difference when resolvable

The contract is simple: skipped branches cannot affect emitted MachineIR, active branches verify normally, and reports identify included and excluded branches.

Malformed branches use `E_MACHINE_COND_ASM_001`.
