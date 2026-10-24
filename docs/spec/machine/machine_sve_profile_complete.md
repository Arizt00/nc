# Machine SVE Profile Complete

The AArch64 SVE profile is closed for the nC-declared native machine surface.
This means accepted `.nm` forms in the profile emit deterministic instruction
words without delegating to an external assembler.

Closed SVE base families:

- predicate setup: `ptrue`, `pfalse`;
- predicate loop formation: `whilelo` for b/h/s/d and w/x counters;
- predicate logic: `not`, `and`, `orr`, `eor` over predicate masks;
- unit-stride predicated memory: `ld1b`, `ld1h`, `ld1w`, `ld1d`, `st1b`,
  `st1h`, `st1w`, `st1d`;
- vector counts: `cntb`, `cnth`, `cntw`, `cntd`, `incb`, `inch`, `incw`,
  `incd`;
- predicated FP arithmetic: `fadd`, `fsub`, `fmul`, `fdiv`;
- predicated FP multiply-accumulate: `fmla`;
- integer vector arithmetic: `add`, `sub`;
- logical vector operations: `and`, `orr`, `eor`;
- comparisons and selection: `cmpeq`, `cmpne`, `cmplt`, `cmpgt`, `cmple`,
  `cmpge`, `sel`;
- FP reduction: `faddv`.

Closed admitted SVE2 row:

- integer vector multiply: `mul z*.b/h/s/d`.

Evidence:

- `cargo test -p ncc-cli machine_encoder_tests::aarch64_sve_encoder_bytes --lib -- --nocapture`;
- `cargo test -p ncc-cli machine_encoder_tests --lib -- --nocapture`;
- `cargo check -p ncc-cli`.

Boundary:

This closes the admitted nC SVE/SVE2 machine profile. It does not claim SME,
SVE crypto, gather/scatter addressing variants, first-faulting memory forms, or
every future SVE extension until those forms are admitted with parser, verifier,
encoder and test rows.
