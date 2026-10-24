# Machine AVX512F Foundation

Phase 4A closes native EVEX encodings for vaddps, vsubps, vmulps, vdivps, vpaddd, vpsubd, vpxord, vmovdqa32, and vmovdqu32 in zmm foundation forms.

Each closed carrier has verifier coverage, AVX512F feature gating, mask validation, zmm validation, memory validation where applicable, byte-pattern evidence, positive examples, and negative examples.
