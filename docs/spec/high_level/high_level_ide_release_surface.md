# High-Level IDE Release Surface

Status: release-industrial.

This document defines the nC normal/high-level IDE surface after the official
pointer model closure.

Semantic authority remains in `ncc`. Editor plugins assist, route, highlight,
complete and suggest imports; they do not decide compiler semantics.

## Surfaces

- VS Code extension: `tools/vscode/nc-language`.
- JetBrains plugin: `tools/jetbrains/nc-language-plugin`.
- Optional low-level LSP adapter: `tools/nc-sys-lsp`.

## Color Lanes

The release-grade palette separates the language layers:

- Java amber/steel: compiler/control text and runtime support lanes.
- C standard blue: scalar types, FFI and import hints.
- C++ core/light blue: machine/native symbols, sections, functions and structured members.
- Rust copper: basal/root ownership, strings and pointer/ref/site/memRegion text.
- Rust hazard red: primitive/kernel/raw/diagnostic danger lanes.

The official color source is:

- `tools/theme/nc_colors.json`.
- `tools/theme/nc_semantic_tokens.json`.
- `tools/vscode/nc-language/themes/nc-systems-spectrum-color-theme.json`.
- `tools/vscode/nc-language/themes/nc-polyglot-systems-color-theme.json`.
- `tools/jetbrains/nc-language-plugin/resources/nc_colors.json`.

## Completion And Import Suggestions

The VS Code plugin provides completion entries and quick fixes for common
runtime imports. Examples:

- `readInt` -> `use stoud.io { input };`
- `println` -> `use stoud.io { output };`
- `renderInt` -> `use stoud.io { render };`
- `regionFromPtr` -> `use core.memory { region };`
- `regionCreate` -> `use core.memory { region };`
- `listAsRegion` -> `use core.memory { region };`
- `ptrFromAddr` -> `use core.memory { ptr };`
- `addrOf` -> `use core.memory { addr };`
- `spanOf` -> `use core.memory { span };`
- `sliceOf` -> `use core.memory { slice };`

For VS Code `2.2.0`, selecting a known helper completion inserts the missing
`use` line automatically when possible. The same import remains available as a
quick fix and as a non-authoritative hint controlled by `nc.editorAssistHints`.
The compiler still validates imports. The plugin only proposes source edits.

## Pointer Hovers

The VS Code plugin explains the closed pointer model for:

- `addr`
- `ptr<T>`
- `ptr mut<T>`
- `ref<T>`
- `ref mut<T>`
- `site<T>`
- `memRegion<T>`
- `slice<T>`
- `span<T>`

Diagnostic hovers explain official `E_PTR_*` codes. These hovers are
documentation surfaces; they do not replace `ncc` diagnostics.

## Pointer And FFI Snippets

The release surface includes snippets for:

- nullable pointer declarations.
- mutable references.
- FFI pointer contracts with ABI/layout/ownership/lifetime/nullability.
- raw pointer hazard boundaries.

## Diagnostics

Diagnostics come from `ncc`, `nclsp`, `nide` or `nc-sys-lsp` routes. The editor
may show local quick fixes, but it does not invent compiler errors.

## Release Gate

Build `ncc-tools-v43` using the [V43 native tool instructions](../../NATIVE_TOOLS_V43_COMMANDS.md).
The regenerable gate, from the repository root with that bin directory on `PATH`, is:

```bash
ncc-tools-v43 editor-industrial
```

The gate checks:

- VS Code package JSON and smoke tests.
- semantic token contract.
- Systems Spectrum theme.
- Polyglot Systems theme with Java/C/C++/Rust-inspired text colors.
- pointer/FFI snippets.
- auto-import quick-fix implementation.
- completion-time auto-import implementation.
- editor-assist import hint implementation.
- pointer hovers and `E_PTR_*` diagnostic hovers.
- JetBrains palette/capability contract.
- official theme token mappings.
