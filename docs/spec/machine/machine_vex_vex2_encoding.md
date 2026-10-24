# Machine VEX2 Encoding

VEX2 uses the `C5` prefix form.

Fields:

- inverted `R`;
- inverted `vvvv`;
- `L`;
- `pp`;
- opcode byte;
- ModRM or immediate only when the instruction form requires it.

Phase 1C closes the zero-operand VEX2 state forms:

- `vzeroupper` -> `c5 f8 77`;
- `vzeroall` -> `c5 fc 77`.

These forms require the `avx` feature gate, accept no operands, and are rejected by the operand matrix if operands are supplied.
