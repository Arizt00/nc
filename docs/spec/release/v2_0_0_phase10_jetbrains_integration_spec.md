# v2.0.0 Phase10 — JetBrains Integration Spec

## Scope

- Plugin root: `tools/jetbrains/nc-language-plugin`
- Semantic authority: `ncc`
- Lane routing:
  - high-level -> `nclsp`
  - low-level -> `nc-sys-lsp`

## Mandatory metadata

- Product plugin version: `2.0.0.2`
- License files:
  - `LICENSE`
  - `LICENSE-MIT`
  - `LICENSE-APACHE`
- SPDX string official: `MIT OR Apache-2.0`

## Mandatory implementation

- File type support: `.n`, `.nb`, `.nbb`, `.nm`, `.ncp`
- Syntax highlighter operational (non-empty lexer)
- Color keys aligned to `nC Sovereign Dark`
- Professional IDE parity tier: `c_cpp_rust_java_grade`
- Reference-language surface: C, C++, Rust and Java-grade editing expectations
- Actions:
  - resolve toolchain
  - route LSP backend
  - run active file
  - debug active file

## Professional IDE parity surface

The plugin must expose the editor surface expected from mature C/C++/Rust/Java integrations: live compiler diagnostics, semantic tokens, completion, auto-import assistance, hover documentation, go-to-definition, references, document symbols, safe rename, code actions, toolchain resolution, run/debug actions and lane-aware routing for `.nb`, `.nm` and `.ncp`.

This is editor parity, not semantic ownership. Parser, sema, IR, lowering and diagnostics remain owned by `ncc`.

## Guardrails

- Plugin does not perform independent semantic validation
- Diagnostics are delegated to `ncc` via LSP tooling
- Runtime is not permitted in low-level forbidden contexts by plugin policy docs

## Validation

- Gradle build when wrapper/environment is available
- Router contract checks with official JSON
