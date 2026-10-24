# AVX512BW Complete

Phase 4C closes the declared AVX512BW profile entries:

- vpaddb.
- vpaddw.
- vpsubb.
- vpsubw.
- vpmovwb.

The verifier checks byte and word ownership through AVX512BW, zmm register forms, k mask use, zeroing, and narrowing width for vpmovwb. The encoder emits EVEX native bytes using the existing Phase 3 EVEX prefix implementation.

AVX512BW closure in Phase 4C does not close AVX-512 total.
