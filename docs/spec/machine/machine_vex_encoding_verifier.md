# Machine VEX Encoding And Verifier

Historical note: this file records the Phase 1C VEX checkpoint. Later native
total phases closed the declared VEX/AVX2/EVEX/AVX-512 profile and the final
machine source of truth is `asm_complete_total=true` for declared profiles.

Phase 1C turns the Phase 1B operand/form matrix into explicit encoding and verifier status per VEX form.

At this historical point it did not close full VEX, AVX2, EVEX, disassembly,
roundtrip, or ASM-complete total. It recorded which modeled forms emitted native
bytes then and which valid forms remained deferred with a diagnostic path.

Rules:

- Every Phase 1 VEX operand form has an explicit `encoding_status`.
- Closed forms emit deterministic native bytes through the nC encoder.
- Deferred forms are valid and verifier-checked, but fail emission with `E_MACHINE_ENCODING_001` if used as machine output before their encoder lands.
- Unsupported-by-design forms are rejected with `E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001`.
- No VEX form is replaced by exported textual ASM.
- `phase1_vex_complete` remained `not_closed` at this historical checkpoint.

Phase 1C currently closes selected VEX2/VEX3 forms for the AVX base subset already supported by the native encoder, including `vmovaps`, `vmovups`, `vaddps`, `vsubps`, `vmulps`, `vdivps`, `vxorps`, `vcmpps`, `vzeroall`, and `vzeroupper`.
