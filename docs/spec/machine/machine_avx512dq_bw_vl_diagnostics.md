# AVX512DQ/BW/VL Diagnostics

Phase 4C maps the following diagnostics:

- E_MACHINE_AVX512_DQ_REQUIRED_001 for missing AVX512DQ.
- E_MACHINE_AVX512_BW_REQUIRED_001 for missing AVX512BW.
- E_MACHINE_AVX512_VL_REQUIRED_001 for missing AVX512VL on xmm or ymm EVEX forms.
- E_MACHINE_AVX512_SUBFEATURE_001 for subfeatures deferred beyond Phase 4C.
- E_MACHINE_AVX512_VECTOR_LENGTH_001 for invalid Phase 4C AVX-512 vector widths.
- E_MACHINE_AVX512_MASK_001 for invalid k register use.
- E_MACHINE_MEM_WIDTH_001 for invalid memory width.
- E_MACHINE_IMM_WIDTH_001 for invalid imm8.

EVEX field, ModRM, SIB, and compressed displacement checks remain owned by the Phase 3 EVEX layer and are reused by Phase 4C.
