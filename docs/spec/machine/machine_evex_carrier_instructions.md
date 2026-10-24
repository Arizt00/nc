# Machine EVEX Carrier Instructions

Phase 3 carrier instructions exist to close the EVEX encoding alphabet.

Closed carriers:
- `vmovdqa32`
- `vmovdqu32`
- `vpaddd`
- `vpsubd`
- `vpxord`
- `vaddps`
- `vsubps`

Each carrier has native bytes, verifier coverage, positive examples, negative examples, docs, and report evidence.

This does not close AVX-512 as a complete ISA family.
