# AVX512BITALG Complete

Phase 4D closes the AVX512BITALG entries in the nC AVX-512 profile:

- `vpshufbitqmb`
- `vpopcntb`

`vpshufbitqmb` closes the k-destination bit shuffle form. `vpopcntb` closes the masked zmm population-count form. Both require `avx512bitalg`, carry byte-pattern hashes, and participate in the AVX-512 family coverage report.
