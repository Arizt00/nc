# [@machine] AVX512F Complete

Phase 4B closes the AVX512F portion of the nC AVX-512 manifest on top of the closed EVEX encoder.

This remains contractual [@machine], not free-form ASM:

- `.nm` remains official.
- `fn:` and `fa:` remain forbidden in positive machine examples.
- EVEX bytes are emitted natively.
- Unsupported subfeatures produce diagnostics instead of silent acceptance.
- AVX-512 total remains open until DQ/BW/VL/CD/IFMA/VBMI/VNNI/BITALG/VPOPCNTDQ/FP16 and final evidence phases close.

Closed AVX512F groups include zmm arithmetic/logical forms, AVX512F compare-to-mask, compress/expand, ternary permute, scatter VSIB, and existing foundation moves.
