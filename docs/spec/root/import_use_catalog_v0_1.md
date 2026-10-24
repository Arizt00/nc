# nC v0.1 — Import/Use Catalog (Active Line)

## Scope
This catalog closes the active `use/import` surface with a typed official registry while preserving strict linking/runtime behavior.

## Baseical Registry
- Source of truth: `crates/ncc-ast/src/import_catalog.rs`
- Baseical known `use/import` paths: **417**
- Maturity guard test: `known_use_import_catalog_has_mature_surface` (`>= 250`)

## Matching Rules
- Path matching is officialized with case-insensitive segment matching.
- Examples treated as equivalent:
  - `system.net.core`
  - `System.net.core`

## Preserved Strictness
- Unknown import paths still fail in sema with:
  - `unresolved import path '<path>'`
- Baseical negative path remains rejected:
  - `system.net.ghost`
- Context-blocked declaration in importing container fails with:
  - `invalid import declaration '<module.path[.*]>' for context '<container[@ctx]>': no compatible target context among [@ctx, ...]`

## Declaration Surface
- Top-level declarations accept both forms with identical semantics:
  - `use module.path;`
  - `import module.path;`
- Wildcards are also equivalent:
  - `use module.path.*;`
  - `import module.path.*;`

## Runtime Callable Import Surface (active)
The callable import/runtime registry remains intentionally strict and typed:
- `system.net.core` / `alt.net.core`: `connect`, `listen`, `accept`, `send`, `recv`
- `memory.heap.manager`: `alloc`, `free`, `realloc`, `zero`
- `runtime.trace`: `mark`
- `runtime.alias`: `mark`
- `system.proc.core`: `spawn`, `wait`
- `system.io.core`: `write`
- `tooling.build.core`: `plan`, `apply`
- `tooling.lint.core`: `run`
- `tooling.fmt.core`: `run`
- `tooling.pkg.core`: `pack`, `publish`
- `compiler.parse.ast`: `load`
- `compiler.semantic.core`: `check`
- `compiler.backend.machine`: `materialize`
- alias de familia compiler: `nC.compiler.parse`, `nC.compiler.semantic`, `nC.compiler.backend`, `nC.compiler.*`

## Coherence Across Layers
- `ncc-ast`: official catalog + path matching helpers
- `ncc-sema`: official unresolved-import validation + typed callable signatures + modal authority checks
- `ncc-ir`: typed import mode classification for call links (`Network/Runtime/System/Tooling/Compiler`)
- `ncc-cli`: aligned import matching/runtime precheck + typed runtime call execution

## Compatibility Notes
- Wildcard behavior and ambiguity semantics are preserved.
- Context authority diagnostics are preserved.
