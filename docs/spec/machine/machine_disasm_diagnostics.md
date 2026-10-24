# Disasm Diagnostics

Phase 6 diagnostics:

- `E_MACHINE_DISASM_OBJECT_001`: unsupported or invalid object header.
- `E_MACHINE_DISASM_SECTION_001`: no executable text/code section.
- `E_MACHINE_DISASM_UNKNOWN_BYTES_001`: executable bytes are not recognized by the native decoder.

Strict object disasm fails on diagnostics. `--allow-unknown` may return a partial report, but it is not closure evidence.
