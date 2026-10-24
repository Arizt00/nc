# Machine VEX Compare, Shuffle, And Conversion

VEX compare and shuffle instructions require an explicit immediate operand. Phase 1D verifies and encodes the closed forms with native bytes.

Compare rules:

- `vcmpps`, `vcmppd`, `vcmpss`, and `vcmpsd` require four operands.
- The fourth operand is `imm8`.
- Missing or out-of-range immediates produce `E_MACHINE_IMM_WIDTH_001`.
- Register/register/register and register/register/memory forms remain distinct in the operand matrix.

Shuffle rules:

- `vshufps` and `vshufpd` require four operands.
- The fourth operand is `imm8`.
- Missing immediates produce `E_MACHINE_IMM_WIDTH_001`.
- Memory operands are width-checked before encoding.

Conversion rules:

- Phase 1D closes byte emission for the conversion forms that are currently inside the Phase 1 VEX domain.
- Remaining conversion forms stay deferred with exact form-level debt.
- Conversion operands still pass through the same register class, vector width, memory width, and feature gate verifier used by the rest of VEX.

This document does not authorize AVX2, EVEX, AVX-512, disassembly closure, or roundtrip closure. Those phases remain separate approval gates.
