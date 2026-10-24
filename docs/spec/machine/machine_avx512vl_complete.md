# AVX512VL Complete

Phase 4C closes the declared AVX512VL forms for x86_64 EVEX carriers in the nC profile.

Closed VL forms include xmm and ymm EVEX encodings for AVX512F, AVX512DQ, and AVX512BW carrier instructions where the phase manifest declares them. The verifier requires AVX512VL for 128-bit and 256-bit EVEX vector forms and rejects mismatched vector widths.

Narrowing moves such as vpmovdb and vpmovwb keep their DQ/BW feature ownership because their xmm/ymm destination width is part of the narrowing operation, not a standalone VL form.
