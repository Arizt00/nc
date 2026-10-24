# AVX512IFMA Complete

Phase 4D closes the AVX512IFMA entries in the nC AVX-512 profile:

- `vpmadd52luq`
- `vpmadd52huq`

The forms are EVEX zmm masked integer FMA forms with register and memory source coverage. They require `avx512ifma`, keep AVX512F/EVEX dependencies intact, and report subfeature coverage through `ncc machine avx512 avx512ifma --json`.

Missing feature and invalid operand forms are rejected before encoding.
