# Machine Native Assembler

`[@machine]` has an in-repo native assembler for declared machine profiles.
This is separate from external authority paths (`LLVM/MC`, `NASM`, `GAS`),
which remain compatibility readers rather than the semantic authority.

Current profile status:

- Parses official `.nm` machine contracts.
- Verifies target, mode, registers, labels, flags, stack and forbidden high-level sections.
- Encodes declared x86_64, AArch64, RISC-V and WASM profile forms.
- Emits deterministic bytes, objects, modules and executable artifacts through
  the machine CLI.
- Refuses unsupported instructions or operand forms with diagnostics instead of accepting them silently.

Implemented x86_64 native forms:

- `mov r64, r64`
- `mov r64, imm64`
- `mov [mem64], r64`
- `mov r64, [mem64]`
- `movzx r64, r8`
- `add/sub/and/or/xor/cmp/test r64, r64`
- `add/sub/and/or/xor/cmp r64, imm8/imm32`
- `inc/dec/not/neg r64`
- `push/pop r64`, including `r8` through `r15`
- all `jcc` aliases as verified rel32 slots
- `setcc r8`
- `cmovcc r64, r64`
- `movaps/addps/subps/mulps/divps/xorps xmm, xmm`
- `addsd/subsd/mulsd/divsd xmm, xmm`
- `vmovaps/vaddps/vsubps/vmulps/vdivps/vxorps` for XMM/YMM register forms through VEX3

Implemented memory/addressing phase:

- `[base]`
- `[base + disp8/disp32]`
- `[base + index * scale + disp8/disp32]`
- SIB for `rsp`/`r12` base
- `rbp`/`r13` displacement rule
- scale validation for `1`, `2`, `4`, `8`

Implemented SIMD/vector profile:

- `features: sse` gates packed single-precision SSE forms.
- `features: sse2` gates scalar double SSE2 forms.
- `features: avx` gates VEX register-register forms.
- AVX2, EVEX and AVX-512 declared profile forms are covered by later native
  total phases.
- Missing vector features fail with `E_MACHINE_FEATURE_REQUIRED_001`.

Still not claimed:

- arbitrary x86_64 opcode, prefix and addressing forms outside the declared profile;
- arbitrary external dialect quirks outside the declared ASM import/export profiles;
- undeclared ISA families and vendor-specific tool behavior.

The native assembler is `ASM-complete total` for declared profiles, not for the
arbitrary external assembler universe.
