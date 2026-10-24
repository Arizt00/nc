# v2.0.0 Phase10 — IDE Fullstack Spec

## Components

- VS Code: active
- JetBrains: active
- Ghidra: active-experimental
- `nclsp`: active
- `nc-sys-lsp`: active

## Base law

1. `ncc` is source of truth.
2. IDEs/extensions do not invent semantics.
3. Raw/official low-level diagnostics are preserved.
4. QEMU evidence is always labeled emulated.

## Shared assets

- Palette source: `tools/theme/nc_colors.json`
- Semantic tokens source: `tools/theme/nc_semantic_tokens.json`
- Theme: `nC Sovereign Dark`

## Licensing

- Base SPDX: `MIT OR Apache-2.0`
- Applied across plugin/tooling packages with required license files

## Readiness gates

- Smoke tests pass (VS Code)
- Routing contracts documented (VS Code/JetBrains/Ghidra)
- No semantic contradiction vs `ncc`
