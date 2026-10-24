# Machine DWARF Debug

ELF64 debug emission writes a compact DWARF lane: `.debug_line`, `.debug_info`, `.debug_abbrev`, and `.debug_str`.

The line table records the machine source file and a deterministic address/line program for the emitted text. The info unit links to the abbreviation table, string table, and line table and records a compilation unit plus a subprogram range. The verifier requires a source file, an ordered positive line range, and section references that can be discovered by `machine debug-info`.

This lane is intentionally debug metadata only. It does not decode instruction streams and does not close object-aware disassembly.
