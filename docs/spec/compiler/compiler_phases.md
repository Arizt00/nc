# Compiler Phases

This document freezes the release-industrial phase contract exposed by `ncc compiler doctor`.

## Baseical Phase Order

| # | Phase | Contract |
|---|---|---|
| 1 | `source` | Source is loaded from the requested path without fallback paths. |
| 2 | `lexer` | Tokens preserve line and column spans and reject invalid lexical input. |
| 3 | `parser` | Containers, sections, declarations, statements and expressions are parsed into AST. |
| 4 | `ast` | Program structure keeps modes, imports, type contracts, FFI contracts and spans. |
| 5 | `sema` | Semantic analysis validates symbols, modes, imports, visibility and contracts. |
| 6 | `type_resolver` | Types are resolved against the official catalog and nominal `type{}` contracts. |
| 7 | `operator_resolver` | Operators are typed, precedence checked and concat/render rules enforced. |
| 8 | `condition_resolver` | Conditions are checked for bool/null/placeholder violations. |
| 9 | `import_resolver` | Runtime, FFI and container imports are resolved with mode restrictions. |
| 10 | `flow_resolver` | Main, return, dev/error and control-flow contracts are checked. |
| 11 | `nir` | nIR preserves typed containers, calls, contracts, imports and diagnostics metadata. |
| 12 | `nir_verifier` | Verifier rejects lost contracts, unresolved symbols and invalid lowered structure. |
| 13 | `lowering` | Lowering maps AST/nIR to backend-ready structures without inventing semantics. |
| 14 | `backend` | Backend selection respects target/profile availability and artifact contracts. |
| 15 | `diagnostics` | Diagnostics are stable, cataloged and surfaced by compiler reports. |

## Release Gates

- `ncc compiler doctor --json` must report all phases above.
- `ncc compiler pipeline <file> --json` must include phase status for source, lexer, parser, sema, nIR, lowering and backend-facing checks.
- `cargo test -p ncc-cli compiler -- --nocapture` must keep compiler phase tests green.
- `cargo check --workspace --all-targets`, `cargo test --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` must pass before this phase set is considered releasable.

## Non-Goals

This phase contract does not use `[@machine]` or `primitive[@]` to repair compiler/runtime semantics. Machine and primitive are validated only as declared modes when their existing tests exercise cross-mode restrictions.
