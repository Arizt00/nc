# AVX512VBMI Complete

Phase 4D closes the AVX512VBMI entries in the nC AVX-512 profile:

- `vpermb`
- `vpermi2b`

Both instructions use EVEX map `0F38`, byte permute operand forms, zmm register classes, mask/zero validation, and `avx512vbmi` feature gates.

The byte-pattern manifest includes high-register EVEX encodings so the R/X/B/R'/V' paths remain exercised.
