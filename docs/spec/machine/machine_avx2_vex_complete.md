# Machine AVX2 VEX Complete

Fase 2 closes the AVX2 VEX forms that were left outside the VEX base and FMA/F16C scopes.

Closed forms:

- `vpermps` register/register/register.
- `vpermps` register/register/memory.
- `vgatherdps` xmm VSIB gather.
- `vgatherdps` ymm VSIB gather.
- `vpmulld` register/register/register.
- `vpmulld` register/register/memory.

Each closed form has:

- AVX2 feature gating.
- operand and vector-width verification.
- native VEX byte emission.
- ModRM and SIB validation where memory is used.
- byte-pattern evidence and regression tests.

The VEX surface now has 207 of 207 operand forms encoded and 207 of 207 forms verified. `phase1_vex_complete` may therefore report `closed`.

`asm_complete_total` remains `false`. EVEX, AVX-512, debug/unwind, object-aware disassembly, roundtrip, and cross-arch native parity remain separate roadmap phases.
