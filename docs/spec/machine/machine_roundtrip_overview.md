# Machine Roundtrip Overview

Phase 7 closes the native roundtrip lane for `[@machine]` sources.

The pipeline is:

1. Parse and verify `.nm`.
2. Build the original MachineIR summary.
3. Emit bytes or an object with the native emitter.
4. Read the emitted artifact with the Phase 6 native object-aware disassembler.
5. Build a normalized disasm MachineIR.
6. Compare instructions, byte spans, symbols, relocations, debug lines and unwind annotations.
7. Emit a stable report with hashes and diagnostics.

Phase 8 cross-arch parity is closed for the declared AArch64, RISC-V and WASM
profiles. `asm_complete_total` is true for declared profiles after the Phase 9
industrial audit and later release seal.
