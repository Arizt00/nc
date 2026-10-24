# AVX-512 Disasm Annotations

AVX-512 disasm entries expose:

- EVEX raw bytes.
- Feature family `avx512`.
- Mask operands when encoded.
- Zeroing and broadcast/rounding/SAE markers when encoded.
- Section, symbol, relocation, debug, and unwind metadata.

Phase 6 does not reopen AVX-512 instruction closure. It only reads emitted AVX-512 object bytes and reports native decode metadata.
