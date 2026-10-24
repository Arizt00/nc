# Machine COFF Unwind

Windows x86_64 object emission now materializes `.pdata` and `.xdata`.

`.pdata` records begin RVA, end RVA, and the unwind-info RVA. `.xdata` records unwind version, flags, prologue size, unwind-code count, and frame register data for the supported prologue model. The verifier rejects a declared unsupported prologue model with `E_MACHINE_COFF_UNWIND_PROLOGUE_001`.

This closes the COFF unwind lane for Phase 5 and keeps disassembly out of scope.
