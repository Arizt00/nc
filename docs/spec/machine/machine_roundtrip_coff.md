# Machine Roundtrip COFF

COFF roundtrip emits and reads `.text`, the COFF symbol table, relocation table and unwind-related sections when present.

The Phase 7 COFF lane verifies that object format, symbols and instruction byte spans survive the emit-disasm-normalize-compare path.
