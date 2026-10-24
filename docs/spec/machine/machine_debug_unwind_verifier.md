# Machine Debug/Unwind Verifier

Phase 5 verifier checks:

- debug file is present when debug or CFI metadata is requested.
- debug line ranges are positive and ordered.
- frame metadata has corresponding CFI.
- CFI does not contradict the declared frame.
- `.eh_frame` FDE range is non-empty.
- COFF prologue model is in the supported lane.
- Mach-O unwind policy is explicit.
- WASM name-section policy is valid.

Diagnostics added or used by Phase 5:

- `E_MACHINE_DEBUG_FILE_001`
- `E_MACHINE_DEBUG_LINE_RANGE_001`
- `E_MACHINE_UNWIND_CFI_001`
- `E_MACHINE_UNWIND_FRAME_001`
- `E_MACHINE_EH_FRAME_FDE_001`
- `E_MACHINE_COFF_UNWIND_PROLOGUE_001`
- `E_MACHINE_MACHO_UNWIND_POLICY_001`
- `E_MACHINE_WASM_DEBUG_NAME_001`
