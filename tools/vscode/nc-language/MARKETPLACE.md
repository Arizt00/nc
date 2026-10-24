# Marketplace publication guide — nC Language

## Plugin name

nC Language

## Short description

Official language support for nC: syntax highlighting, semantic tokens, snippets, hover, inlay hints, Run/Debug actions, nC Sovereign Dark theme, file icons, and integration with ncc, nclsp and nc-sys-lsp.

## Long description

nC Language provides editor support for the nC programming language. nC is a contract-driven language and toolchain focused on explicit computational layers, including runtime, compiler, system, inspection, kernel, machine, primitive hardware and basal/root semantics.

This extension brings nC source files into Visual Studio Code with syntax highlighting, semantic tokens, snippets, official file icons, real nC logo assets, the official palette pack, and integration points for ncc-based workflows.

Release-grade editor surface includes:

- Systems Spectrum highlighting for basal/root, primitive, machine and kernel lanes.
- Product version `1.0.0.21v` with VS Code SemVer package version `1.0.0-21v`.
- Professional IDE parity tier comparable to mature C/C++/Rust/Java plugins.
- Polyglot Systems text colors based on Java amber/steel, Python blue/yellow, C standard blue,
  C++ core/light blue and Rust copper/hazard.
- Separate grammar scopes and isolated icon variants for `.n`, `.nm`, `.nb`, `.nbb` and `.ncp`.
- Pointer/ref/site/memRegion semantic tokens.
- Raw hazard and FFI contract snippets.
- Runtime auto-import quick fixes for common helpers.
- Completion-time auto-import edits for common runtime and pointer helpers.
- Strong predictive completion for compact import families, imported functions,
  heap helpers, struct fields, aggregate element fields, project modules,
  entrypoint references and `ncc` commands.
- External `stoud` standard-library catalog for `io`, `memory`, `collections`,
  `math`, `time`, `fs`, `net`, `process` and `thread`, with signatures,
  parameters, return values, errors and backend-lowering notes.
- Hover signatures and parameter/type inlay hints for common nC calls.
- Ctrl+Click navigation for functions, compact variables, parameters, imports,
  `ref main ->> run` and `use <project:...>` child modules.
- Find References across project `.n` files with comments/strings ignored, and
  safe Rename for locally defined code symbols.
- Project graph command backed by `bridge.json`, `.ncc/code/*.json` and sources,
  with mother/child detection, visible cycles and duplicate-container warnings.
- `use <project:...>` completion, diagnostics and Ctrl+Click are backed by
  the same project graph so child/mother imports resolve consistently.
- File icons are extension-scoped only, so C, C++, Rust, Markdown, JSON and other
  plugins keep their own icon/theme ownership.
- Editor-title, context-menu and CodeLens Run/Debug/Check actions for `.n`,
  `.nb`, `.nbb`, `.nm` and `.ncp`.
- `Format File` and VS Code Format Document for `.n`, backed by
  `nfmt --compact`.
- Official compact syntax snippets and diagnostics for `name >> type = value`,
  `x += value` and `i++`, while legacy verbose source remains compatible.
- Stable type assistance for scalar types, `array<T,N>`, `list<T>`,
  `queue<T>`, `vector<T>`, `ptr<T>`, `Result<T,E>`, nested structs,
  struct aggregates and function values.
- Compact formatter support for ordered imports, compact declarations,
  section spacing and comment-safe rewrites.
- Native `nc` debug type with breakpoints, stack, variables and stepping through
  `ncc debug-adapter`, plus fallback terminal debugging.
- Auto Run/Debug host routing for PowerShell, CMD, WSL, Kali, Ubuntu and Linux;
  low-level `.nm`, `.nb`, `.nbb` and `.ncp` execution opens outside the IDE.
- ncc kit discovery and workspace kit settings for environments where VS Code cannot find `ncup`.
- Editor-assist import hints that remain non-authoritative and can be disabled.
- Pointer-model hovers and official `E_PTR_*` diagnostic explanations.
- Compiler-owned diagnostics routed through ncc/nclsp/nc-sys-lsp.
- ncc-style diagnostic text parsing with exact red ranges, plus editor-side
  checks for invalid escapes, unknown imports, unresolved symbols, type
  mismatches and broken `@fn(...)` references.

The extension does not replace the ncc compiler. Semantic validation remains owned by ncc. The extension is designed to present and route nC tooling, not to invent language rules.

## Visual identity assets

- Baseical source: `site/public/assets/nc-logo.svg`
- Bundled SVG: `assets/nc-logo.svg`
- VS Code icon: `assets/nc-logo-128.png`
- Marketplace variants: `assets/marketplace/nc-logo-128.png`, `assets/marketplace/nc-logo-256.png`
- File icon theme id: `nc-icons`

## Official palettes

- nC Sovereign Dark
- nC Dust Rose
- nC Aqua Runtime
- nC Crimson Diagnostics
- nC Systems Spectrum
- nC Polyglot Systems

## License

MIT OR Apache-2.0

## Pre-publication checklist

- npm run test
- npm run package:vsix
- repository present
- license present
- icon present
- README present
- CHANGELOG present
- no tokens
- no node_modules
- no target/debug
- no automatic publish

## Publish commands

Documented only; do not execute in this phase:

```bash
npx @vscode/vsce login <publisher>
npx @vscode/vsce publish
npx ovsx publish dist/nc-language-1.0.0-21v.vsix
```

Do not run publish commands unless explicitly authorized by the project owner.
