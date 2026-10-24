# Machine ASM Complete Total

`asm_complete_total` is true after Phase 9 because every native ASM phase is closed and the industrial audit has sealed the evidence.

Closed scope:

- x86_64 native ASM including VEX, AVX2, EVEX, and AVX-512.
- x86_64 declared absolute coverage for prefixes, rare addressing forms,
  register classes, VEX/EVEX carrier rules, AVX2, AVX-512 subfeatures and
  profiled catalog gates for privileged/vendor-private families.
- debug/unwind.
- object-aware disassembly.
- roundtrip.
- cross-arch parity for declared AArch64, RISC-V, and WASM profiles.
- Phase 9 audit/evidence bundle and later Phase 10-14 macro/directive,
  dialect import/export, linker, external object compatibility and corpus
  hardening evidence.

Declared cross-arch profiles:

- `nC-aarch64-core-v1`.
- `nC-riscv64-core-v1`.
- `nC-wasm32-core-v1`.

The operational source of truth is:

- `ncc machine asm-complete absolute --json`
- `ncc machine asm-complete doctor --json`

The remaining non-claim is not internal machine debt. It is the arbitrary
external universe: vendor-private opcodes, every historical assembler quirk, and
tool behavior outside the declared nC machine profiles or explicit dialect
profiles.
