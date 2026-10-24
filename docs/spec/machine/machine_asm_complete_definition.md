# Machine ASM-Complete Definition

ASM-complete means `[@machine]` covers the functional territory of an industrial assembler while keeping nC's contract rules.

The scope includes ISA instructions, registers, operand forms, addressing modes, labels, sections, data declarations, constants, equates, alignment, symbol visibility, extern/import/export, relocations, object formats, ABI, stack/frame, debug/unwind metadata, macros, conditional assembly, include/incbin, listing/map output, disassembly, roundtrip, linker integration, binary emission, diagnostics, verifier, and retained evidence.

The nC rule is stricter than raw assembly:

- `.nm` is the official source.
- `fn:` and `fa:` are forbidden in positive machine sources.
- A target, ABI, object format, symbols, and verification contract are explicit.
- Unknown instructions fail.
- Operand forms are checked.
- Missing encoding is a diagnostic, not a silent success.
- ASM import/export is tooling, not the official language surface.

Current status: Machine Level 6 is preserved. ASM-complete Level 7, Level 8, and Level 9 are not declared closed by this document.
