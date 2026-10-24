# Machine VEX3 Encoding

VEX3 uses the `C4` prefix form.

Fields:

- inverted `R`, `X`, and `B`;
- `m-mmmm` opcode map;
- `W`;
- inverted `vvvv`;
- `L`;
- `pp`;
- opcode byte;
- ModRM byte;
- SIB/displacement/immediate when the operand form requires them.

Phase 1C closes VEX3 byte emission for selected register, memory, and immediate forms. Examples include:

- `vmovaps xmm0, xmm1`;
- `vmovaps xmm0, xmmword [rax]`;
- `vmovaps xmmword [rax], xmm1`;
- `vaddps xmm0, xmm1, xmm2`;
- `vaddps xmm0, xmm1, xmmword [rax]`;
- `vcmpps xmm0, xmm1, xmm2, 1`.

The verifier checks operand count, vector class, width, memory width, immediate width, feature gate, and unsupported-by-design status before encoding.
