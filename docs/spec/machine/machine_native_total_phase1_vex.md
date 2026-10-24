# Machine Native Total Phase 1 VEX

Phase 1 covers VEX x86_64 as a native assembler phase. In this execution, Phase 1 is not closed.

The current native encoder/verifier accepts and emits selected AVX VEX register-register forms for xmm and ymm registers. The phase contract requires the complete VEX domain: VEX2 and VEX3 selection, map_select, W/L/pp/vvvv fields, memory and immediate forms, feature gates, verifier coverage, diagnostics, tests, examples, docs and evidence for every manifest entry.

The native doctor must keep asm_complete_total false while any VEX entry is partial or deferred. AVX2 remains outside this execution and must not be treated as started.

Diagnostics used by this phase include E_MACHINE_VEX_VVVV_001, E_MACHINE_MEM_WIDTH_001, E_MACHINE_REGISTER_WIDTH_001, E_MACHINE_INSTR_OPERAND_001 and E_MACHINE_FEATURE_REQUIRED_001.

Continuation status: Phase 1 remains not_closed. The verifier additionally rejects immediate operands in the current closed VEX forms with E_MACHINE_IMM_WIDTH_001, scalar register-class misuse with E_MACHINE_REGISTER_CLASS_001, and explicitly out-of-domain VEX mnemonics with E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001. These diagnostics are guard rails, not a closure claim.

Decode metadata for VEX is not yet ready for Phase 6 because the full descriptor table for all manifest entries is still missing. The current metadata is therefore reported as not_ready_missing_full_vex_descriptor_table.
