# Machine Disasm Overview

Phase 6 adds native object-aware disassembly for artifacts emitted by `[@machine]`.

The disassembler reads object containers directly, locates executable sections, decodes bytes with native project code, and returns structured JSON or text. It does not perform roundtrip normalization and does not close `asm_complete_total`.

Closed lanes:

- ELF64 object-aware disasm.
- COFF object-aware disasm.
- Mach-O object-aware disasm.
- WASM module-aware disasm.
- Section, symbol, relocation, debug-line, and unwind annotations.
- Strict diagnostics for unknown bytes.

Downstream lanes:

- Phase 7 roundtrip is now closed for declared profiles.
- Phase 8 cross-arch parity is now closed for declared profiles.
