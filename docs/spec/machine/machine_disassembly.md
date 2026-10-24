# Machine Disassembly

Disassembly turns object or raw bytes into audited machine instructions.

Current status:

- `ncc machine disasm` reads ELF64, COFF, Mach-O and WASM artifacts with
  section, symbol, relocation, debug-line and unwind annotations.
- `ncc machine disasm-bytes` decodes raw x86_64 byte buffers for the supported
  smoke/debug lane.
- Unknown bytes produce diagnostics.

Object-aware disassembly is closed for the declared machine profiles. Arbitrary
external ISA and dialect decoding outside those profiles is not claimed.
