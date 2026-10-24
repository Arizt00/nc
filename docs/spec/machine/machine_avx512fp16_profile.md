# AVX512FP16 Profile

The nC Phase 4D AVX-512 profile includes the initial FP16 entries:

- `vaddph`
- `vmulph`

Both are closed in Phase 4D with EVEX map 5 encoding, zmm masked operand forms, `avx512fp16` feature gates, positive examples, negative missing-feature checks, and byte-pattern hashes.

Because FP16 is included in the current profile, it is not left as `unsupported_by_design`.
