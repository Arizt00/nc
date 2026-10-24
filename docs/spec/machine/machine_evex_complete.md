# Machine EVEX Complete Encoding Layer

Phase 3 closes EVEX as an x86_64 encoding format, not as an AVX-512 family-complete instruction claim.

Closed scope:
- EVEX prefix byte `0x62`.
- P0 inverted `R/X/B/R'` plus opcode map.
- P1 `W`, inverted `vvvv`, fixed bit, and `pp`.
- P2 zeroing, `L'L`, broadcast/rounding context, `V'`, and `aaa`.
- `xmm0..31`, `ymm0..31`, `zmm0..31`, and `k0..k7` modeling.
- k-mask, merging, zeroing, broadcast, rounding/SAE, tuple, and compressed displacement validation.
- Carrier instruction evidence for the EVEX alphabet.

Not closed here:
- AVX-512F/DQ/BW/VL/CD complete family coverage.
- EVEX disassembly.
- Roundtrip.
- debug/unwind.

Carrier instructions are marked `closed_as_evex_carrier=true` and `avx512_family_complete=false`.
