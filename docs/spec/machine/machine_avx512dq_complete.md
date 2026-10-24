# AVX512DQ Complete

Phase 4C closes the declared AVX512DQ profile entries:

- vpmullq.
- vfpclassps.
- vpmovdb.

Each entry is encoded through the EVEX layer, gated by AVX512DQ, and validated with operand, mask, width, immediate, and memory checks. vpmullq covers qword vector arithmetic, vfpclassps covers k destination plus imm8, and vpmovdb covers dword to byte narrowing into xmm or m128 destinations.

AVX512DQ closure in Phase 4C does not close AVX-512 total.
