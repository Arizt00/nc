# Machine FP Profile Complete

The floating-point machine profile is closed for the native nC machine surface.
Accepted forms in this profile emit deterministic bytes directly from `ncc`
without requiring an external assembler.

Closed x86_64 FP forms:

- scalar/packed SSE load and store: `movss`, `movsd`, `movaps`, `movups`,
  `movapd`, `movupd`;
- scalar FP arithmetic: `addss`, `subss`, `mulss`, `divss`, `sqrtss`,
  `addsd`, `subsd`, `mulsd`, `divsd`, `sqrtsd`;
- packed FP arithmetic/logical rows already admitted by the SIMD profile;
- integer/FP conversions: `cvtsi2ss`, `cvtsi2sd`, `cvtss2si`, `cvtsd2si`
  over r32/r64 and dword/qword memory forms;
- scalar FP comparisons: `comiss`, `ucomiss`, `comisd`, `ucomisd` over xmm and
  typed memory forms.
- x87 transcendental forms admitted by nC machine: `fsin`, `fcos`, `fsincos`
  and the `flog`/`flogn`, `flog2`, `flog10` pseudoforms lowered to explicit
  `fld*` + `fxch st1` + `fyl2x` byte sequences over `ST0`.

Closed AArch64 scalar FP forms:

- moves and unary ops: `fmov`, `fabs`, `fneg`, `fsqrt`;
- arithmetic: `fadd`, `fsub`, `fmul`, `fdiv`, `fmax`, `fmin`;
- width conversion: `fcvt` between `s` and `d`;
- compare: `fcmp` register/register and `#0.0`;
- conditional select: `fcsel`;
- signed integer/FP conversions: `scvtf`, `fcvtzs` across w/x and s/d forms.

Closed vector FP rows:

- AArch64 NEON admitted `fadd/fmul v*.4s`;
- AArch64 SVE admitted predicated `fadd/fsub/fmul/fdiv`, `fmla` and `faddv`;
- x86_64 AVX/EVEX FP rows admitted by the VEX/EVEX/AVX-512 profile.

Evidence:

- `cargo test -p ncc-cli machine_encoder_tests --lib -- --nocapture`;
- `cargo check -p ncc-cli`.

Boundary:

This closes the nC native FP profile used by the machine backend, including the
declared sin/cos/log surface. It does not claim the whole historical x87 stack
ISA, arbitrary libm entrypoints, FP exception-mode modeling, strict IEEE
environment metadata, or optimizer-wide math equivalence with LLVM/GCC/rustc.
