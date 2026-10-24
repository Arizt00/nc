# Machine Declared Profiles

Phase 9 seals the declared profiles below:

- x86_64 native: nC machine x86_64 profile with legacy/core, VEX, AVX2, EVEX, and AVX-512.
- AArch64: `nC-aarch64-core-v1`.
- RISC-V: `nC-riscv64-core-v1`.
- WASM: `nC-wasm32-core-v1`.

Cross-arch parity means each target is complete within its declared contract. It does not mean non-x86 architectures copy x86 instruction families.
