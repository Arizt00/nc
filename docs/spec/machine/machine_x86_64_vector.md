# Machine x86_64 Vector

Vector ASM coverage includes SSE, SSE2, SSE3, SSSE3, SSE4, AVX, AVX2, and AVX-512 families.

Current status:

- XMM registers are listed.
- YMM/ZMM/opmask encoding is deferred.
- VEX/EVEX encoding is deferred.
- Feature gates are reported by `ncc machine features x86_64 --json`.

Vector instructions must require explicit CPU features and must not be accepted as successful machine code without encoding evidence.
