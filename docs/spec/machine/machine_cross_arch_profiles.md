# Machine Cross-Arch Profiles

Phase 8 uses formal profiles rather than open-ended ISA claims.

| Target | Profile | Scope |
| --- | --- | --- |
| AArch64 | `nC-aarch64-core-v1` | AAPCS64 core integer/object/disasm/roundtrip profile |
| RISC-V | `nC-riscv64-core-v1` | RV64I plus M object/disasm/roundtrip profile |
| WASM32 | `nC-wasm32-core-v1` | Core module, validation, debug custom sections, disasm, roundtrip |

The profiles close the machine lifecycle for their declared domains. Future profile expansion must add new manifests, examples, diagnostics, and evidence before changing these contracts.
