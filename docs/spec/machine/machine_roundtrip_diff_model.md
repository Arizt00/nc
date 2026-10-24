# Machine Roundtrip Diff Model

The diff model separates accepted normalizations from unaccepted differences.

Compared surfaces:

- Instruction index, normalized mnemonic, operand class and raw byte span.
- Symbols recovered from the emitted object.
- Relocations recovered from the emitted object.
- Debug line annotations when debug metadata exists.
- Unwind annotations when unwind metadata exists.
- Object and normalized IR hashes.

Unaccepted diffs produce `E_MACHINE_ROUNDTRIP_*` diagnostics and fail strict roundtrip.
