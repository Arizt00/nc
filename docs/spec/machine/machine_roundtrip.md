# Machine Roundtrip

Roundtrip means `.nm -> MachineIR -> bytes -> disasm -> comparable MachineIR`.

Current status:

- object-aware roundtrip is closed for ELF64, COFF, Mach-O and WASM artifacts.
- multi-ISA roundtrip is closed for the declared x86_64, AArch64, RISC-V and
  WASM profiles.
- strict reports include hashes, normalized MachineIR, diff diagnostics, symbols,
  relocations, debug lines and unwind annotations.

Roundtrip closure is profile-scoped. It does not claim arbitrary external ISA or
assembler dialect roundtrip outside the declared machine profiles.
