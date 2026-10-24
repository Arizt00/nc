# Machine VEX FMA/F16C

Phase 2B closes the VEX extension forms that were classified after Phase 1F as FMA and F16C rather than AVX2.

Closed forms:

- `vfmadd132ps` register/register/register.
- `vfmadd132ps` register/register/memory.
- `vcvtph2ps` register/register.
- `vcvtph2ps` register/memory.

`vfmadd132ps` requires the `fma` feature and emits native VEX `0F38` map bytes with `66` prefix semantics, `W0`, width-selected `L`, ModRM, and inverted source register encoding.

`vcvtph2ps` requires the `f16c` feature and emits native VEX `0F38` map bytes with `66` prefix semantics, `W0`, reserved `vvvv`, width-selected `L`, and validated half-precision source width.

Phase 2B does not start AVX2. `vpermps`, `vgatherdps`, and `vpmulld` remain delegated to Phase 2 and continue to fail with the explicit VEX unsupported-by-design diagnostic until that phase is approved.

VEX total therefore remains not closed: base VEX and Phase 2B are closed, but AVX2 is still gated.
