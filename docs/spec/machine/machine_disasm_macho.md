# Mach-O Disasm

The Mach-O disasm lane reads the Mach-O header, load commands, `__text`, symbol table, string table, relocations, compact unwind, and DWARF fallback sections when present.

Mach-O unwind data is reported as an instruction annotation when the object carries compact unwind or DWARF debug fallback evidence.
