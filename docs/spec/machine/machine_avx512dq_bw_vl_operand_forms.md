# AVX512DQ/BW/VL Operand Forms

Phase 4C operand validation covers:

- zmm{k}{z}, zmm, zmm and zmm{k}{z}, zmm, m512 forms for DQ/BW arithmetic.
- k, zmm or m512, imm8 forms for vfpclassps.
- xmm or m128 destination plus zmm source for vpmovdb.
- ymm or m256 destination plus zmm source for vpmovwb.
- xmm and ymm EVEX VL forms gated by AVX512VL.

Invalid forms produce diagnostics for missing subfeature gates, invalid masks, invalid vector length, invalid memory width, invalid immediate width, and deferred subfeature ownership.
