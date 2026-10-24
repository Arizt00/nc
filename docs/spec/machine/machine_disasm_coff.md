# COFF Disasm

The COFF disasm lane reads the COFF header, section table, `.text`, symbol table, string table, relocation table, `.pdata`, and `.xdata`.

Phase 6 treats `.pdata` and `.xdata` as unwind annotations and preserves Phase 5 verification. It does not infer roundtrip equivalence from the disassembly.
