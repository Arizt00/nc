# WASM Disasm

The WASM disasm lane reads the module header, section stream, code section, export/name metadata, and custom debug sections.

The Phase 6 WASM decoder covers the machine-emitted WASM instruction subset:

- `i32.const`
- `i32.add`
- `local.get`
- `return`
- `end`

WASM has debug name annotations but no native stack unwind section.
