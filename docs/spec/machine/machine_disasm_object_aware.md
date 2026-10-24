# Object-Aware Disasm

Object-aware disasm means the input is a real object file or WASM module, not a naked byte stream.

The Phase 6 report includes:

- Object format and validity.
- Sections and executable section hashes.
- Symbols and function ranges.
- Relocation entries and instruction annotations.
- Debug-line annotations when debug sections exist.
- Unwind annotations when unwind sections exist.
- Unknown-byte diagnostics in strict mode.

`machine disasm-bytes` remains available for raw x86_64 byte buffers, but closure evidence is based on object-aware `machine disasm` and `machine disasm-object`.
