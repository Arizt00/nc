# nC Language

## What is this plugin?

nC Language is the official editor integration for the nC programming language. It provides syntax highlighting, semantic token support, snippets, file icons, the nC Sovereign Dark theme family, and integration points for ncc, nclsp and nc-sys-lsp.

The extension provides editor assistance and delegates compiler-backed checks to the installed toolchain.

## Version 1.0.0.21v

`1.0.0.21v` is the canonical plugin baseline. The VS Code marketplace manifest uses the valid SemVer form `1.0.0-21v`, while `ncProductVersion` records the exact product version.

The required surface is comparable to mature C, C++, Rust and Java editor integrations: live compiler diagnostics, semantic tokens, predictive completion, hover documentation, inlay hints, definitions, references, document symbols, safe rename, quick fixes, CodeLens run/debug/check actions, run/check/format actions, toolchain resolution and mode-aware routing for machine and primitive files.

## Visual identity

The plugin ships its own SVG and PNG logo assets for file icons, themes and marketplace packaging.

## What is nC?

nC is a programming language and toolchain designed around explicit computational layers, contracts, modes and evidence. It aims to make boundaries between runtime, system, compiler, inspection, kernel, machine, primitive hardware and basal/root semantics visible and auditable.

## Features

- Syntax highlighting for nC files.
- Semantic tokens for nC concepts.
- Themes:
  - nC Sovereign Dark
  - nC Aqua Runtime
  - nC Crimson Diagnostics
  - nC Systems Spectrum
  - nC Polyglot Systems
- Official nC file icons (`nc-icons`) only for `.n`, `.nm`, `.nbo`, `.nb`, `.nbb` and `.ncp`, with no global file override.
- Snippets for common nC structures.
- Snippets for pointer/nullability, FFI pointer contracts and raw hazard boundaries.
- Mode-aware detection for high-level, root/basal, machine and primitive files.
- Integration points for ncc.
- Runtime family awareness.
- Auto-import quick fixes for common runtime helpers such as `readInt`, `println`, `renderInt`, `ptrFromAddr`, `addrOf` and `spanOf`.
- Completion-time auto-import edits and strong predictions for compact import
  families such as `use stoud.io { output, render, input };` and
  `use stoud.memory { ptr, heap, region };`.
- External `stoud` standard-library catalog for `io`, `memory`,
  `collections`, `math`, `time`, `fs`, `net`, `process` and `thread`, with
  documented parameters, return values, errors and backend/runtime lowering.
- Predictive completions for imported functions (`println`, `renderInt`,
  `alloc`, `free`, `ptrRead`), visible variables, functions, struct fields,
  aggregate element fields, `use <project:...>` modules, entrypoint
  references and useful `ncc` terminal commands.
- Manual low-level editing for `.nm`, `.nb`, `.nbb` and `.ncp`: completion,
  prediction, snippets, word suggestions, inline suggestions, predictive
  inlays and generic fallback are disabled. Machine
  suggestions follow `target.arch` and include declared symbols/procedures,
  valid registers and encodable instructions. Basal and Primitive resolve
  their own sites, bindings, routines, capabilities, anchors and operations.
- Hover signatures for common runtime/list/queue/pointer helpers, with concise
  definitions and import hints.
- Inlay hints for known parameter names and visible expression-type hints.
- Real navigation: Ctrl+Click on local functions, compact variables,
  parameters, imports, `ref main ->> run` and `use <project:...>`.
- CodeLens and editor-title actions for `Run`, `Debug` and `Check` on runnable
  sources. `.nm` exposes `Check` only; Machine execution starts explicitly in a terminal.
- `Format File` and VS Code `Format Document` support for `.n` through
  `nfmt --compact`.
- Official compact syntax assistance: declarations use `name >> type <- value`,
  updates use `x += value`, and counters use `i++` / `i--`.
- Stable type assistance for `int`, `byte`, `char`, `str`, `bool`, `ft32`,
  `ft64`, `array<T,N>`, `list<T>`, `queue<T>`, `vector<T>`, `ptr<T>`,
  `Result<T,E>`, nested structs, struct aggregates and function values.
- Field completion follows nested structs and aggregates such as
  `packets.get(i).header.tag` or `rows[0].address.city`.
- Debug Adapter Protocol integration for breakpoints, stack, scopes, variables
  and stepping when `ncc debug-adapter` is available.
- ncc kit discovery through configured paths, `ncup`, PATH, workspace
  `target/debug`, workspace `.ncc/bin` and WSL.
- Editor-assist import hints through `nc.editorAssistHints`; these are hints,
  not compiler diagnostics.
- Separate TextMate scopes and palette lanes for basal/root, primitive, machine, kernel, pointers, raw hazards and FFI.
- Tagless kernel recognition for `.n`: `name at address`, `*8` through `*512`,
  or a `kernel...` container with `main:` activates the kernel palette without
  requiring `[@kernel]`; comments and strings cannot activate it.
- Pointer-model hovers for `addr`, `ptr<T>`, `ptr mut<T>`, `ref<T>`,
  `site<T>`, `memRegion<T>`, `slice<T>` and `span<T>`.
- Baseical diagnostic hovers for `E_PTR_*` pointer diagnostics.
- Text colors in the systems themes follow the Java/Python/C/C++/Rust-inspired polyglot palette:
  `.n` uses high-level purple, `.nm` uses machine orange/blue, `.nb` uses basal copper/gold,
  `.nbb` uses primitive basal red/gold, `.ncp` uses primitive capability hazard colors and
  `.nbo` uses the retained backend-object black/blue/violet identity.
- Compiler diagnostics integration.
- ncc-style editor diagnostics parse `[line:column]` compiler text output and
  place red ranges on the exact token. Editor-side checks also catch invalid
  string escapes, unknown facade/project imports, unresolved symbols, simple
  type mismatches and broken `@fn(...)` references.
- Primitive tooling integration.
- LSP routing through nclsp and nc-sys-lsp where available.
- Clickable diagnostics for compiler/LSP routes: ranges preserve line/column
  when `ncc`, `nide`, `nclsp` or `nc-sys-lsp` expose them, diagnostic codes are
  attached to the Problems entry, and files inside a `bridge.json` project carry
  a related location back to that bridge.

## Supported file extensions

- `.n`
- `.nb`
- `.nbb`
- `.nm`
- `.nbo` (read-only backend object IR retained in `.ncc/lowering`)
- `.ncp`

## Supported nC surfaces

- high-level nC
- `[@runtime]`
- `[@compiler]`
- `[@system]`
- `[@tooling]`
- `[@inspect]`
- `[@security]`
- `[@network]`
- `[@kernel]`
- `[@machine]`
- `primitive[@]`
- `[@]` / basal/root surface

`[@kernel]` remains accepted for compatibility. New kernel source uses a
tagless container; the compiler and plugin infer its lane from the compact
kernel surface.

## Official palette pack

- nC Sovereign Dark
- nC Dust Rose
- nC Aqua Runtime
- nC Crimson Diagnostics
- nC Systems Spectrum
- nC Polyglot Systems

Baseical palette files:

- `tools/theme/nc_colors.json`
- `tools/theme/nc_semantic_tokens.json`
- `tools/theme/nc_visual_identity.json`

## ncc integration

If ncc is not installed or not available in PATH, syntax highlighting and themes still work, but compiler-backed diagnostics and command flows may be unavailable.

`nC: Run Current File`, `nC: Debug Current File` and `nC: Check Current File`
resolve the real `ncc` through the active ncc kit. Auto discovery verifies each
candidate before using it: the compiler must expose `stoud::memory::map` and the
closed native debug-adapter protocol. Discovery checks `nc.nccPath`, `ncup`, the
installed nC suite, PATH, workspace builds and WSL, and rejects stale workspace
copies. In a folder containing `bridge.json`, those commands delegate to
`ncc project run <root>` and `ncc project check <root>` so multi-file projects
behave like one project instead of isolated files.

Run/Debug automatically chooses the host route. `nc.runHost` can force
`powershell`, `cmd`, `wsl`, `kali`, `ubuntu` or `linux`; `auto` chooses the
best available native or WSL route. `.n` runs in the integrated terminal so
interactive input such as `readInt` and `readStr` works normally. `.nm` remains
fully editable, checked, highlighted and navigable, but has no IDE Run, Debug or
breakpoint lane. Start it explicitly with `ncc machine ...` in a terminal.

`nc.profile` controls editor assistance only. `nc.runProfile` controls compiler
execution and defaults to `debug`. `nc.target` defaults to `auto`, so a normal
`.n` file runs for the host without an unsupported or foreign `--target` option.
Explicit targets remain available for project and low-level build lanes.

Use `nC: Create ncc Kit` or `nC: Select ncc Kit` when VS Code cannot find
`ncup` from its own environment.

`nC: Debug Current File` starts the `nc` debug adapter and supports breakpoints,
stack frames, scopes, variables and stepping for runnable sources. Machine
inspection remains available from a terminal through `ncc machine debug-bridge`.

`nC: Format File` saves the current `.n` buffer and runs
`nfmt <file> --compact --in-place`. The normal written form of `.n` is compact:
declarations prefer `contador >> int <- 0;` and `activo >> bool <- true;`,
updates prefer `x += 1;`, and increments prefer `i++;`. Verbose projections
remain compatible input, but snippets, formatting and editor assistance present
compact source as the primary style. Verbose projections are normalized to compact declarations/assignments, compact
imports are grouped and ordered, `stc:`, `local:`, `const:`, `Name :: type`
and `name :: fn` spacing is normalized, and comments are preserved. The compact form still
passes through parser, sema and backend like any other `.n` source.

Ctrl+Click resolves local functions, variables, parameters and containers in
the active file. `ref main ->> run` jumps to `run`. Runtime imports open a
portable language definition, while `use <project:biblioteca>` jumps to the
matching child file. Find References searches project `.n` files and skips
strings/comments. Rename is limited to code symbols with a local definition and
does not rewrite imports, comments, strings or member fields by accident.

`nC: Show Project Graph` reads `bridge.json`, `.ncc/code/*.json` and project
sources, then opens a virtual graph with mothers, children, edges, cycles and
duplicate containers. Cycles are shown for inspection but do not create editor
warnings. Duplicate container names are warned because project order decides the
winner: `bridge.json` main first, then `link_order`, then lexicographic path.

`use <project:usuarios, biblioteca>` uses that same graph for completion,
diagnostics and Ctrl+Click. Unknown modules are reported, duplicate containers
explain which file wins, and project cycles remain visible without turning the
editor into false-error noise.

## Run tasks

Workspace tasks can be generated by the toolchain:

```bash
ncc ide init --ide vscode
```

The generated `.vscode/tasks.json` exposes `nC: Run Active File`, `nC: Debug Active File`, `nC: Check Active File` and `nC: Format Active File`. Format delegates to `ncc nfmt <file> --compact --in-place`.

## What this plugin does not do

- It does not replace ncc.
- It does not compile nC by itself.
- It does not invent semantic validation.
- It does not turn import hints into compiler truth.
- It does not make raw pointer operations safe.
- It does not certify hardware.
- It does not treat QEMU as real hardware.
- It does not bypass nC contracts.

## Installation

```bash
code --install-extension dist/nc-language-1.0.0-21v.vsix --force
```

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

SPDX-License-Identifier: MIT OR Apache-2.0
