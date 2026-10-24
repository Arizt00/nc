# nC - B1 Backend Sovereignty Closure Spec

Status: Active Base (B1.1 + B1.2 + B1.3 + B1.4 + B1.5 + B1.6 + B1.7 + B1.8 + B1.9 + B1.10 + B1.11 + B1.12)
Version: v2.0.0-B1

## 1. Scope

This document freezes the backend sovereignty base for nC:

- backend contract as typed data;
- target model with triple, tier and runtime model;
- official data layout + calling convention integration in lowering.

This is the minimum hard closure needed before deeper backend tracks.

## 2. Baseical Backend Contract

The backend contract is represented in `ncc-ir` by `BackendTargetSpecIR`.

Required fields:

- `target_triple`
- `target_family`
- `isa`
- `abi`
- `object_format`
- `relocation_model`
- `code_model`
- `data_layout`
- `endianness`
- `pointer_width`
- `runtime_model`
- `host_services`
- `debug_info_model`
- `linker_model`
- `support_tier`
- `backend_kind`
- `profile_constraints`

This contract is typed and no longer delegated to ad-hoc string logic.

## 3. Baseical Target Model

`BackendLoweringProfileIR` now carries:

- `target`
- `calling_convention`
- `target_spec`

Current official profiles:

- `nC.sysv64`
- `nC.win64`
- `nC.aapcs64`
- `nC.portable`
- `nC.sovereign`

`target_spec` is resolved by `BackendTargetSpecIR::from_target_and_calling_convention(...)`.

## 4. Baseical Data Layout + Calling Convention

`BackendDataLayoutIR` is now explicit and extended:

- `pointer_bytes`
- `word_bytes`
- `stack_align_bytes`
- `integer_align_bytes`
- `float_align_bytes`
- `aggregate_align_bytes`

Lowering no longer hardcodes data layout inline.
`build_function_backend_plan(...)` consumes `backend_profile.target_spec.data_layout`.

This closes B1.3 with a profile-driven backend layout contract.

## 5. Propagation Rule

Target metadata is propagated end-to-end:

- `ProgramIR.target_spec`
- `BackendModuleIR.target_spec`
- `BackendObjectIR.target_spec`

Text emission now includes triple/tier/runtime metadata in backend headers.

## 6. Conformance Locks

Tests covering this closure:

- `abi_ir_tests::backend_target_spec_sysv64_is_tier1_hosted_with_elf64`
- `abi_ir_tests::backend_target_spec_win64_is_tier1_hosted_with_coff`
- `abi_ir_tests::backend_target_spec_aapcs64_is_tier1_hosted_with_macho`
- `abi_ir_tests::backend_plan_consumes_profile_data_layout_instead_of_hardcoded_layout`
- `abi_ir_tests::backend_legalization_matrix_for_hosted_contains_official_runtime_and_primitive_diagnostics`
- `abi_ir_tests::backend_legalization_matrix_for_freestanding_contains_runtime_missing_guard`
- `abi_ir_tests::backend_legalization_matrix_for_portable_contains_capability_safe_pointer_console_and_vector_rules`
- `abi_ir_tests::backend_legalization_matrix_for_sovereign_contains_native_boundary_rules_and_vector_reject`
- `abi_ir_tests::backend_legalization_matrix_shape_validation_rejects_duplicate_issue`
- `abi_ir_tests::backend_legalization_matrix_shape_validation_rejects_missing_required_issue`
- `abi_ir_tests::backend_linking_contract_hosted_lane_supports_full_artifact_family`
- `abi_ir_tests::backend_linking_contract_freestanding_rejects_build_exe_artifact`
- `backend_emit_ir_tests::emit_object_elf64_hosted_lane_allows_unresolved_imported_symbols`
- `backend_emit_ir_tests::emit_object_elf64_freestanding_lane_rejects_unresolved_symbols_at_backend_stage`
- `backend_emit_cli_tests::build_executable_rejects_unresolved_symbols_for_freestanding_lane_before_linker_spawn`
- `abi_ir_tests::backend_link_execution_contract_hosted_uses_external_host_linker`
- `abi_ir_tests::backend_link_execution_contract_freestanding_is_internal`
- `abi_ir_tests::backend_lane_acceptance_gate_rejects_non_sovereign_context_for_sovereign_lane`
- `abi_ir_tests::backend_lane_acceptance_gate_accepts_root_machine_primitive_for_sovereign_lane`
- `abi_ir_tests::backend_lane_contract_matrix_covers_all_four_lanes`
- `backend_emit_cli_tests::build_executable_rejects_non_external_link_execution_contract`
- `backend_emit_cli_tests::emit_backend_rejects_non_sovereign_container_under_sovereign_lane_gate`
- `abi_ir_tests::backend_target_spec_portable_is_tier1_portable_with_wasm_module`
- `abi_ir_tests::backend_linking_contract_portable_lane_supports_wasm_artifacts`
- `abi_ir_tests::backend_link_execution_contract_portable_is_internal_module_driver`
- `backend_emit_ir_tests::emit_module_wasm32_portable_lane_materializes_valid_module_header_and_start_export`
- `backend_emit_cli_tests::parse_backend_profile_accepts_portable`
- `backend_emit_cli_tests::backend_artifact_profile_rejects_emit_elf_for_portable_lane`
- `backend_emit_cli_tests::emit_wasm_with_portable_profile_materializes_wasm_module`
- `abi_ir_tests::backend_target_spec_sovereign_is_tier3_sovereign_with_freestanding_image`
- `abi_ir_tests::backend_linking_contract_sovereign_lane_supports_native_image_artifacts`
- `backend_emit_ir_tests::emit_image_sovereign_materializes_native_image_header_and_counts`
- `backend_emit_ir_tests::emit_image_sovereign_rejects_unresolved_symbols_at_backend_stage`
- `backend_emit_ir_tests::link_image_sovereign_materializes_linked_header_and_entry_address`
- `backend_emit_ir_tests::link_image_sovereign_rejects_missing_entry_symbol_override`
- `backend_emit_cli_tests::parse_backend_profile_accepts_sovereign`
- `backend_emit_cli_tests::backend_artifact_profile_rejects_emit_elf_for_sovereign_lane`
- `backend_emit_cli_tests::emit_sovereign_with_sovereign_profile_materializes_native_image`
- `backend_emit_cli_tests::parse_sovereign_command_options_defaults_to_sovereign_profile_without_cc`
- `backend_emit_cli_tests::parse_sovereign_command_options_accepts_entry_for_build_mode`
- `backend_emit_cli_tests::parse_sovereign_command_options_rejects_duplicate_entry`
- `backend_emit_cli_tests::parse_sovereign_command_options_rejects_unknown_option`
- `backend_emit_cli_tests::build_sovereign_with_sovereign_profile_materializes_linked_image`
- `backend_emit_cli_tests::build_sovereign_accepts_explicit_entry_symbol_override`
- `backend_emit_cli_tests::build_sovereign_rejects_unknown_entry_symbol`

## 7. Legalization Matrix (B1.4)

Lane-specific legalization is now typed in `ncc-ir`:

- `BackendLaneIR`
- `BackendLegalizationIssueIR`
- `BackendLegalizationActionIR`
- `BackendLegalizationRuleIR`
- `BackendLegalizationMatrixIR`

Reference document:

- `docs/spec/backend/B1_legalization_matrix.md`

## 8. Linking Contract (B1.5)

Linking model is now lane-typed and connected to CLI artifact validation.

Reference document:

- `docs/spec/backend/B1_linking_model.md`

## 9. Symbol Resolution (B1.6)

Unresolved symbol policy is lane-bound and enforced in backend ELF emission.

Reference document:

- `docs/spec/backend/B1_symbol_resolution_policy.md`

## 10. Link Execution Contract + Sovereign Acceptance Gate (B1.7)

B1.7 closes two hard gaps:

- lane-typed linker execution contract (`BackendLinkExecutionContractIR`);
- backend lane acceptance gate for container contexts (`validate_program_for_backend_lane(...)`).

The execution contract freezes whether a lane may invoke external linkers:

- hosted lane: external host linker contract;
- portable lane: internal portable wasm module contract;
- freestanding lane: internal linker contract;
- sovereign lane: internal sovereign linker contract.

The acceptance gate enforces context/lane compatibility and includes the sovereign rule:

- sovereign lane accepts only `[@]`, `[@machine]`, `primitive[@]`;
- non-sovereign contexts are rejected with official diagnostics.

Reference documents:

- `docs/spec/backend/B1_linking_model.md`
- `docs/spec/backend/B1_lane_acceptance_gate.md`

## 11. Closure Note

This document closes B1.1/B1.2/B1.3/B1.4/B1.5/B1.6/B1.7/B1.8/B1.9/B1.10/B1.11/B1.12.
Sovereign native image emission and sovereign internal link execution are closed.

## 12. Portable Lane Native Closure (B1.8)

B1.8 closes portable lane emission as first-class backend output:

- official portable profile: `nC.portable`;
- lane contracts include `emit-wasm` and `build-wasm`;
- typed wasm module emission: `emit_module_wasm32(...)`;
- CLI commands: `emit-wasm` and `build-wasm`;
- portable lane rejects `emit-elf` by contract diagnostics.

Reference documents:

- `docs/spec/backend/B1_linking_model.md`
- `docs/spec/backend/B1_portable_lane_closure.md`

## 13. Sovereign Lane Native Closure (B1.9)

B1.9 closes sovereign lane native image emission as first-class backend output:

- official sovereign profile: `nC.sovereign`;
- lane contracts include `emit-sovereign` and `build-sovereign`;
- typed sovereign image emission: `emit_image_sovereign(...)`;
- CLI commands: `emit-sovereign` and `build-sovereign`;
- sovereign lane rejects `emit-elf` by typed contract diagnostics.

Reference documents:

- `docs/spec/backend/B1_linking_model.md`
- `docs/spec/backend/B1_sovereign_lane_closure.md`

## 14. Sovereign Internal Linker Closure (B1.10)

B1.10 closes sovereign internal link execution as first-class backend behavior:

- typed internal linker entrypoint: `link_image_sovereign(...)`;
- deterministic linked image contract (`NCSOVLNK`);
- official relocation patching (`CallNear32`, `JumpNear32`, `Abs64`);
- explicit entry override support (`--entry`) and deterministic default entry policy.

Reference documents:

- `docs/spec/backend/B1_linking_model.md`
- `docs/spec/backend/B1_sovereign_internal_linker_closure.md`

## 15. Legalization Matrix Deepening (B1.11)

B1.11 deepens legalization closure from baseline lane mapping to official matrix invariants:

- lane matrix rule lookup by issue;
- official shape validation for every lane;
- required issue coverage (pointer/region, console render, structured control flow);
- duplicate-rule rejection and missing-rule diagnostics.

Reference documents:

- `docs/spec/backend/B1_legalization_matrix.md`

## 16. Cross-Target Object Serializer Deepening (B1.12)

B1.12 closes cross-target deep object serialization in the active backend matrix:

- native `COFF` emission in `ncc-ir` (`emit_object_coff(...)`);
- native `Mach-O` emission in `ncc-ir` (`emit_object_macho64(...)`);
- CLI `emit-coff` / `emit-macho` now use native serializer output as the first-class path.

This removes dependence on conversion-only flows for the official object emit commands and keeps serializer behavior lane-aware and typed end-to-end.

Reference document:

- `docs/spec/backend/B1_cross_target_object_serializers.md`
