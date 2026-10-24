# Machine Repeat And Times

Phase 10 supports bounded `times` and `repeat` blocks.

The count must be a compile-time integer and must stay under the expansion policy limit. Huge or malformed expansion reports `E_MACHINE_REPEAT_LIMIT_001` or `E_MACHINE_REPEAT_001`.

Expanded instructions are normal MachineIR instructions and participate in encoding, disasm, and roundtrip.

