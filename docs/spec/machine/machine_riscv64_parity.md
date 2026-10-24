# Machine RISC-V Parity

`nC-riscv64-core-v1` closes RV64I plus M profile parity for [@machine]. The profile includes x-registers and ABI aliases, integer ALU operations, loads/stores, branches, `jal`, `jalr`, `ret`, `nop`, system trap forms allowed by the profile, ELF64 object emission, and RISC-V relocation verification.

Native disasm decodes the instructions emitted by the profile and roundtrip validates the resulting normalized MachineIR without relying on external tools.

Primary evidence:

- `examples/machine/native/riscv64_parity_core.nm`
- `examples/machine/native/riscv64_parity_relocations.nm`
- `examples/machine/native/riscv64_parity_roundtrip.nm`
- `reports/machine_native_total/manifests/phase8_riscv64_instruction_manifest.json`
