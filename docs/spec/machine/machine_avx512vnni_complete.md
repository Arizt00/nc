# AVX512VNNI Complete

Phase 4D closes the AVX512VNNI entries in the nC AVX-512 profile:

- `vpdpbusd`
- `vpdpwssd`

The dot-product forms are closed for zmm masked register and memory forms, require `avx512vnni`, and reuse the Phase 3 EVEX prefix and ModRM/SIB machinery.

The verifier rejects missing VNNI, invalid vector length, invalid mask use, and unsupported operand classes before byte emission.
