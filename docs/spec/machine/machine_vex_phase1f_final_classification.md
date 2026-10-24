# Machine VEX Phase 1F Final Classification

Phase 1F resolves the remaining extension-owned VEX forms after Phase 1E closed all valid base-domain deferred forms.

## Result

Phase 1F does not start AVX2, FMA, F16C, EVEX, AVX-512, disassembly, or roundtrip work. It classifies the remaining ten forms and introduces a precise base-versus-total status:

- `phase1_vex_base_complete`: closed.
- `phase1_vex_complete`: not_closed.
- `phase2_avx2_complete`: not_started.
- `phase2b_vex_fma_f16c`: not_started.
- `asm_complete_total`: false.

## Classified Forms

AVX2 delegated to Phase 2:

- `vpermps`
- `vgatherdps`
- `vpmulld`

FMA/F16C delegated to Phase 2B:

- `vfmadd132ps`
- `vcvtph2ps`

Each instruction has two operand forms in the Phase 1 manifest, for ten delegated forms total.

## Closure Rule

VEX base is complete because every valid Phase 1 base form has native byte emission, verifier coverage, examples, tests, and evidence. VEX total remains open because total VEX includes extension domains that require separate phase approval.

Use before the owning phase is enabled must report `E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001`.
