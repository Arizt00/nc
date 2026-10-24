# Machine WASM Debug

WASM does not use native stack unwind records in the same way as ELF, COFF, or Mach-O. Phase 5 therefore closes the WASM debug lane with custom sections:

- `name` for function names.
- `nC.debug` for producer, language, source file, and line metadata.

The verifier ensures the module remains valid and rejects invalid name-section policy with `E_MACHINE_WASM_DEBUG_NAME_001`.
