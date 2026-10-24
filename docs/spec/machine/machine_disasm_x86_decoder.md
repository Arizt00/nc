# X86 Decoder

The Phase 6 x86_64 decoder is native project code. It decodes the instruction bytes emitted by official positive `[@machine]` examples across:

- Legacy/core machine instructions.
- SSE forms emitted by existing examples.
- VEX and AVX2 examples.
- EVEX carrier examples.
- AVX-512 examples from Phase 4.

The decoder models legacy prefixes, REX, ModRM, SIB, displacement, immediates, VEX2, VEX3, and EVEX prefix fields enough to classify emitted instructions and preserve byte boundaries. Unknown executable bytes are diagnostics, not silent acceptance.
