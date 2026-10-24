# nC Ghidra Extension

nC Ghidra Extension is the low-level Ghidra-side integration for nC artifacts.

## Scope

- Consumes ncc-generated low-level artifacts/reports where available.
- Helps inspect machine/primitive-facing outputs in a Ghidra workflow.
- Uses official artifact routes instead of inventing private semantics.

## Visual identity

- Baseical logo source is mirrored from `tools/assets/logo` into:
  - `tools/ghidra/nc-ghidra-extension/data/nc-logo.png`
  - `tools/ghidra/nc-ghidra-extension/data/nc-logo.svg`
- Color mappings come from `data/nc_colors.json`, aligned with:
  - nC Sovereign Dark
  - nC Dust Rose
  - nC Aqua Runtime
  - nC Crimson Diagnostics
  - nC Systems Spectrum

## Important boundaries

- ncc remains the semantic source of truth.
- The extension does not invent disassembly.
- The extension does not claim machine completeness when ncc does not expose it.
- QEMU evidence remains emulated evidence, not real hardware certification.

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

SPDX-License-Identifier: MIT OR Apache-2.0
