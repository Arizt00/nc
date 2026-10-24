# Machine WASM32 Parity

`nC-wasm32-core-v1` closes WASM module parity for [@machine]. The profile includes magic/version, type/function/code/export sections, name and `nC.debug` custom sections, local operations, numeric operations, control flow, memory load/store, validation, module-aware disasm, and roundtrip.

WASM does not use native stack unwind metadata like ELF/COFF/Mach-O. Its Phase 8 parity closes module debug naming and custom nC debug metadata instead.

Primary evidence:

- `examples/machine/native/wasm32_parity_core.nm`
- `examples/machine/native/wasm32_parity_memory.nm`
- `examples/machine/native/wasm32_parity_roundtrip.nm`
- `reports/machine_native_total/manifests/phase8_wasm32_instruction_manifest.json`
