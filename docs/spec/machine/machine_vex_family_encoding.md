# Machine VEX Family Encoding

Phase 1D expands native VEX byte emission by family while keeping the global Phase 1 VEX state honest.

This phase preserves the 104-instruction manifest and the 207 operand forms created in Phase 1A and Phase 1B. It does not reduce the table, does not move work into Phase 2, and does not declare full VEX closure.

Phase 1D closes additional native byte emission for these Phase 1 VEX family segments:

- packed double arithmetic and min/max forms;
- packed double logical forms;
- scalar single and scalar double arithmetic, min/max, and sqrt forms;
- compare forms with explicit `imm8` predicates;
- shuffle forms with explicit `imm8` controls;
- initial conversion forms that are part of the declared Phase 1 VEX domain.

Every closed form records:

- VEX kind and fields;
- opcode map and opcode;
- `pp`, `W`, `L`, and `vvvv`;
- ModRM and memory policy;
- immediate policy where applicable;
- deterministic byte examples;
- verifier status;
- evidence hash.

Forms that remain valid but not encoded stay visible as `deferred` with `E_MACHINE_ENCODING_001` as the emission gate. Unsupported-by-design forms remain explicit and are rejected with `E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001`.

Phase 1D status:

- `phase1d_vex_encoding_expansion`: closed;
- `phase1_vex_complete`: not_closed;
- `phase2_avx2_complete`: not_started;
- `asm_complete_total`: false.
