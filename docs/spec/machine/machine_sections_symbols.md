# Sections And Symbols

Machine source declares `section:`, `symbol:`, `extern:`, `export:`, and `entry:` contracts.

Object emission materializes `.text` with global entry symbol. Phase 12 link units preserve `.rodata`, `.data`, `.bss`, unwind and debug sections when present. Final ELF64 executable output emits section headers and permissioned loader segments for those families instead of collapsing everything into one writable/executable range.

Diagnostics include `E_MACHINE_SECTION_001`, `E_MACHINE_SYMBOL_001`, `E_MACHINE_EXTERN_001`, `E_MACHINE_EXPORT_001`, and `E_MACHINE_ENTRY_001`.
