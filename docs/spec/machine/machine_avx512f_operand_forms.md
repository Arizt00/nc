# [@machine] AVX512F Operand Forms

Phase 4B accepts only AVX512F zmm-width forms. AVX512VL xmm/ymm forms are deferred to Phase 4C.

Valid Phase 4B form groups:

- `zmm{k}{z}, zmm, zmm`
- `zmm{k}{z}, zmm, zmmword [mem]`
- `zmm{k}{z}, zmm/m512`
- `k1..k7, zmm, zmm/m512, imm8`
- `zmm/m512{k}, zmm`
- `dword [base+zmm*scale]{k}, zmm`

Invalid masks, vector widths, memory tuple forms, subfeatures, and immediates are rejected before emission.
