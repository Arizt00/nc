# AVX512VPOPCNTDQ Complete

Phase 4D closes the AVX512VPOPCNTDQ entries in the nC AVX-512 profile:

- `vpopcntd`
- `vpopcntq`

The dword and qword population-count forms are feature-gated by `avx512vpopcntdq`, validated as masked zmm EVEX forms, and emitted through native EVEX bytes with evidence hashes.

Invalid width and missing subfeature cases remain explicit diagnostics.
