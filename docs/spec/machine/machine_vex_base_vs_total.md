# Machine VEX Base Versus Total

The native VEX plan separates base-domain closure from total-extension closure.

## VEX Base

VEX base covers the AVX-domain forms intentionally closed in Phase 1A through Phase 1F:

- manifest and table backbone;
- operand and invalid-form matrix;
- verifier coverage;
- native byte-pattern emission;
- deferred-form reduction to zero;
- final classification of extension-owned forms.

When these gates are satisfied, `phase1_vex_base_complete` is `closed`.

## VEX Total

VEX total includes base VEX plus extension domains such as AVX2, FMA, and F16C. Those domains require explicit phase approval and cannot be absorbed into Phase 1 without changing the roadmap.

Therefore `phase1_vex_complete` remains `not_closed` until extension-owned forms are implemented in their owning phases or the roadmap formally narrows the meaning of total.

## Phase Ownership

- Phase 2 owns AVX2 VEX forms.
- Phase 2B owns FMA and F16C VEX forms.
- Phase 3 and later own EVEX and AVX-512 work.
