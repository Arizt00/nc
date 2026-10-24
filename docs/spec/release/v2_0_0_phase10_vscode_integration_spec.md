# v2.0.0 Phase10 — VS Code Integration Spec

## Scope

- Plugin root: `tools/vscode/nc-language`
- Semantic authority: `ncc`
- LSP lanes:
  - high-level -> `nide`/`nclsp`
  - low-level (`[@]`, `[@machine]`, `primitive[@]`) -> `nc-sys-lsp`

## Mandatory metadata

- `package.json`:
  - `"version": "2.0.0-2"` because VS Code packaging requires SemVer
  - `"ncProductVersion": "2.0.0.2"`
  - `"license": "MIT OR Apache-2.0"`
  - `repository` object present
- License files present in plugin root:
  - `LICENSE`
  - `LICENSE-MIT`
  - `LICENSE-APACHE`

## Mandatory surface

- Language extensions: `.n`, `.nb`, `.nbb`, `.nm`, `.ncp`
- Commands:
  - check/run/debug/format current file
  - create/select ncc kit
  - compiler diagnostics/pipeline/ast/ir
  - runtime families/exports/capabilities/policies
  - primitive doctor/check/witness/map/lower/qemu
  - select toolchain, open base docs
- Editor action visibility:
  - `Run`, `Debug` and `Check` CodeLens on nC files
  - editor-title actions for `Run`, `Debug` and `Check`
  - editor-context actions for `Run`, `Debug`, `Check` and `Format`
- Predictive IDE assistance:
  - hover signatures for known runtime/list/queue/pointer helpers
  - parameter/type inlay hints
  - local-symbol and command prediction completions
- Theme: `nC Sovereign Dark`
- Professional IDE parity tier: `c_cpp_rust_java_grade`
- Reference-language surface: C, C++, Rust and Java-grade editing expectations

## Professional IDE parity surface

The plugin must expose the editor surface expected from mature C/C++/Rust/Java integrations: live compiler diagnostics, semantic tokens, completion, predictive completions, auto-import assistance, hover documentation, inlay hints, go-to-definition, references, document symbols, safe rename, code actions, ncc kit resolution, toolchain resolution, CodeLens run/debug/check actions, run/check/format actions and lane-aware routing for `.nb`, `.nm` and `.ncp`.

Run/debug resolution must not fail just because VS Code does not inherit `ncup`
from the shell PATH. The extension must try configured `nc.nccPath`, `ncup`,
PATH `ncc`, workspace `target/debug`, workspace `.ncc/bin` and WSL where
available.

This is editor parity, not semantic ownership. Parser, sema, IR, lowering and diagnostics remain owned by `ncc`.

## Non-goals

- No semantic invention in extension
- No fake diagnostics replacing `ncc`
- No machine doctor command if CLI is not officially exposed
- QEMU never certified as physical hardware

## Validation

- `node src/smoke_test.js`
- packaging route to `dist/*.vsix`
