# ELF64 Disasm

The ELF64 disasm lane reads the ELF header, section table, `.text`, `.symtab`, `.strtab`, relocation sections, `.debug_*`, and `.eh_frame` when present.

Instruction records include section offsets, file offsets, raw bytes, mnemonic, operands, feature family, symbol owner, relocation annotation, debug line, and unwind relation.

Unknown bytes in executable sections produce `E_MACHINE_DISASM_UNKNOWN_BYTES_001` unless `--allow-unknown` is explicit.
