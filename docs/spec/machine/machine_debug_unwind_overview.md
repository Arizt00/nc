# Machine Debug/Unwind Overview

Phase 5 adds real object metadata for debug and unwind while preserving the machine surface syntax. Positive examples use explicit `debug:` and `cfi:` sections together with `frame:` and `stack:` metadata.

Closed lanes:

- ELF64 DWARF: `.debug_line`, `.debug_info`, `.debug_abbrev`, `.debug_str`.
- ELF64 unwind: `.eh_frame` with CIE/FDE payloads tied to emitted text ranges.
- COFF/Windows unwind: `.pdata` and `.xdata`.
- Mach-O unwind: `__compact_unwind` plus DWARF debug sections.
- WASM debug: `name` and `nC.debug` custom sections.

The CLI exposes `machine debug-info`, `machine unwind-info`, `machine debug-report`, `machine unwind-report`, and enhanced `machine verify-object` output. Phase 5 itself did not implement object-aware disassembly, roundtrip, or cross-architecture parity; those lanes are closed by later Phase 6, Phase 7, and Phase 8 evidence.
