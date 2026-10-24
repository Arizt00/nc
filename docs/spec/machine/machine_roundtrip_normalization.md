# Machine Roundtrip Normalization

Phase 7 uses explicit normalization rules only.

Accepted rules:

- Width-preserving register class aliases.
- Object-format symbol decoration when the target symbol is identical.
- Relocation spelling when the target symbol is identical.
- Numeric section offset formatting.
- Debug line row order when address semantics match.
- VEX/EVEX opcode-family names when raw instruction bytes and feature family match the emitted instruction span.
- WASM source `return` to code-section `end` when the module body is stable.

Not accepted:

- Instruction count changes.
- Byte span changes.
- Symbol target changes.
- Relocation target changes.
- Required debug or unwind metadata disappearing.
