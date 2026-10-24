# Machine x86_64 Encoding Complete

ASM-complete encoding requires legacy prefixes, REX, one-byte opcode map, `0F`, `0F 38`, `0F 3A`, ModRM, SIB, displacements, immediates, relative offsets, relocation slots, VEX, EVEX, opcode extensions, moffs, and the 15-byte instruction limit.

Current status:

- REX: closed for the admitted GPR, memory and vector forms.
- ModRM: closed for admitted scalar, SIMD, atomic and system-profile forms.
- SIB: closed for base, index, scale, disp8/disp32, base-less absolute disp32,
  `rsp/r12` SIB-required bases and `rbp/r13` displacement-required bases.
- address-size override: closed for admitted long-mode `0x67` forms, including
  32-bit address registers, explicit `addr32`/`a32`, segments, direct symbolic
  `Abs32`, VSIB memory and addr32 moffs.
- symbol-relative: closed for `[symbol]` and `[symbol +/- addend]` as
  base-less SIB disp32 bytes with `Abs32` relocation records.
- stack-relative: closed for `[rsp]`, signed disp8/disp32 stack slots,
  indexed stack slots and segment-qualified stack memory.
- frame-relative: closed for `[rbp]`, forced zero `disp8`, signed
  disp8/disp32 frame slots, indexed frame slots and segment-qualified frame
  memory.
- VEX: closed for the admitted VEX profile.
- EVEX: closed for the admitted EVEX/AVX-512 profile.
- segment overrides: closed for admitted memory operands.
- moffs: closed through explicit `moffs8/16/32/64` operands, accumulator-only
  `A0/A1/A2/A3` opcodes, segment prefixes and unsigned 64-bit absolute
  addresses.
- branch/call, `[symbol]` and `[rel symbol]` memory relocations: verified
  smoke forms.

The command `ncc machine encoding-coverage x86_64 --json` is the normative report for this surface.
