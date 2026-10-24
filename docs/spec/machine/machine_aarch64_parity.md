# Machine AArch64 Parity

`nC-aarch64-core-v1` closes AAPCS64 userland core parity for [@machine]. The profile includes `x0..x30`, `w0..w30`, `sp`, zero registers, integer ALU operations, branches, calls, returns, load/store, pair load/store, compare/test aliases, and core relocation verification.

Object emission uses ELF64 AArch64. Native disasm reads emitted objects through section and symbol metadata, and roundtrip compares the emitted/disassembled profile back to normalized MachineIR.

Primary evidence:

- `examples/machine/native/aarch64_parity_core.nm`
- `examples/machine/native/aarch64_parity_relocations.nm`
- `examples/machine/native/aarch64_parity_roundtrip.nm`
- `reports/machine_native_total/manifests/phase8_aarch64_instruction_manifest.json`
