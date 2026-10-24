# Machine Conformance Matrix

The conformance matrix records closed behavior per declared [@machine] profile. It is profile-based, not universe-based.

Closed rows:

- x86_64 native ASM: parser/check, verifier, encoder, object emission, debug/unwind, disasm, and roundtrip.
- AArch64 `nC-aarch64-core-v1`: encoding, ELF object, relocations, disasm, and roundtrip.
- RISC-V `nC-riscv64-core-v1`: encoding, ELF object, relocations, disasm, and roundtrip.
- WASM `nC-wasm32-core-v1`: module validation, names/debug sections, disasm, and roundtrip.

The generated matrix lives at `reports/machine_native_total/phase9/phase9_conformance_matrix.json`.
