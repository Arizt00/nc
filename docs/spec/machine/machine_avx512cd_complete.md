# AVX512CD Complete

Phase 4D closes the AVX512CD entries in the nC AVX-512 profile:

- `vptestnmd`
- `vpconflictd`

Both entries use the closed EVEX encoder, require the `avx512cd` feature gate, validate zmm and k-register operand forms, and publish byte-pattern evidence through `ncc machine avx512 byte-patterns --json`.

Invalid uses are rejected with `E_MACHINE_AVX512_CD_REQUIRED_001`, `E_MACHINE_AVX512_MASK_001`, `E_MACHINE_AVX512_VECTOR_LENGTH_001`, `E_MACHINE_MEM_WIDTH_001`, or the shared EVEX diagnostics where appropriate.
