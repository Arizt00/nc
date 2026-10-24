# Machine Cross-Arch Parity Overview

Phase 8 closes native parity for the non-x86 targets that [@machine] exposes today. Parity is profile based: each target closes the register model, ABI, verifier, encoder, object or module emission, verifier-object path, native disasm, roundtrip, examples, and negative diagnostics for its declared nC profile.

This does not mean AArch64, RISC-V, and WASM copy the x86_64 instruction universe. It means each target has a formal contract and the same lifecycle evidence as x86_64: source check, MachineIR, bytes or module, object awareness, disasm, roundtrip, and doctor status.

Closed profiles:

- `nC-aarch64-core-v1`
- `nC-riscv64-core-v1`
- `nC-wasm32-core-v1`

With Phase 8 closed after Phases 1-7, `asm_complete_total` may be true.
