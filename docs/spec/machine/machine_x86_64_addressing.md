# Machine x86_64 Addressing

ASM parity requires register, immediate, direct memory, base, base+disp, base+index, base+index*scale, base+index*scale+disp, RIP-relative, absolute moffs, segment override, stack-relative, frame-relative, symbol-relative, and section-relative addressing.

Current status:

- register operands: verified for the admitted x86_64 profile.
- immediate operands: verified for the admitted scalar/profile forms.
- branch/call symbol references: relocation smoke verified.
- direct memory operands: closed for `[disp32]`, `[symbol]` and
  `[symbol +/- disp32]`; symbolic direct memory emits base-less SIB bytes and
  `Abs32` relocation records.
- symbol-relative operands: closed for `[symbol]` and
  `[symbol +/- addend]` through base-less SIB disp32 bytes plus `Abs32`
  relocation records. They are accepted for `mov` load/store and `lea`, and
  remain separate from RIP-relative `[rel symbol]` forms.
- section-relative operands: closed through explicit `section(.name)` terms.
  `[section(.data) +/- addend]` emits base-less SIB plus `Abs32`; `[rel
  section(.text) +/- addend]` emits RIP-relative bytes plus `PcRel32`.
- base memory operands: closed for all admitted GPR64 bases `rax..r15`,
  including no displacement, signed disp8, signed disp32, `rsp/r12` SIB
  encoding, and `rbp/r13` forced zero disp8 encoding.
- base+disp operands: closed at the encoding boundary. Signed displacements
  in `[-128, 127]` use `disp8`; other signed `i32` displacements use
  `disp32`; values outside `i32` are rejected before byte emission.
- base+index operands: closed for first-register-as-base and second-register-
  as-index forms with implicit scale 1. All admitted GPR64 bases are accepted;
  all GPR64 indexes except `rsp/r12` are accepted because SIB index code `100`
  means no index.
- base+index*scale operands: closed for explicit scale `1`, `2`, `4` and `8`.
  The scale is emitted in the SIB scale field, high registers set REX.X/REX.B,
  and duplicate indexes or non power-of-two scales are rejected.
- base+index*scale+disp operands: closed for the full SIB form with signed
  accumulated displacement. `disp8`/`disp32` selection follows the same
  boundary as base+disp, and high-register REX plus segment prefixes are
  emitted before the opcode.
- long-mode 32-bit address-size override operands: closed for the admitted
  `0x67` profile. The encoder accepts implicit 32-bit address registers
  `eax..edi`, explicit `addr32`/`a32` memory markers, segment-qualified
  addr32 memory, symbolic/direct addr32 memory with `Abs32` relocations,
  VSIB addr32 memory and `moffs* addr32[...]`. Extended `r8d..r15d`, mixed
  32/64 address registers and RIP/`rel` spellings under `addr32` are rejected.
- stack-relative operands: closed for `[rsp]`, `[rsp +/- disp8]`,
  `[rsp +/- disp32]` and `[rsp + index*scale +/- disp]`. `rsp` is always a
  SIB base, never an index, and stack-relative memory remains distinct from
  frame-relative `rbp` encoding.
- frame-relative operands: closed for `[rbp]`, `[rbp +/- disp8]`,
  `[rbp +/- disp32]` and `[rbp + index*scale +/- disp]`. The no-displacement
  spelling emits the required zero `disp8`; `rbp` remains the semantic frame
  pointer while `r13` only shares the same displacement-required encoding class.
- RIP-relative operands: closed for numeric `[rip +/- disp32]`, symbolic
  `[rel symbol +/- addend]`, `[rip + symbol +/- addend]` and
  `[symbol + rip +/- addend]`. Symbolic forms emit `PcRel32`; direct
  `[symbol]` forms remain `Abs32`.
- memory operands: closed for base, base+disp, base+index*scale,
  base+index*scale+disp, stack/frame-relative, base-less SIB absolute disp32,
  long-mode addr32 override, RIP-relative disp32 and `[rel symbol]` forms.
- segment overrides: closed for accepted memory operands through
  `cs/ds/es/fs/gs/ss:` prefixes. They are emitted before REX/VEX/opcode bytes
  and after group-1 prefixes such as `lock` when present.
- absolute moffs: closed through explicit `moffs8/16/32/64 [addr]` operands.
  These are accumulator-only `A0/A1/A2/A3` forms with an absolute `u64`
  address field in x86_64 mode; they are not parsed as ModRM/SIB memory.
- far control-transfer operands: closed for explicit far immediates
  `ptr16:16 selector:offset` and `ptr16:32 selector:offset`, far memory
  operands such as `fword ptr [base]`/`tword ptr [base]`, generic
  `jmp/call far`, far returns and interrupt returns.

Invalid operands must produce diagnostics. Ambiguous memory width cannot be accepted silently.
