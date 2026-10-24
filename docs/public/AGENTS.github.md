# AGENTS.md - Operating guide for the nC project

## Repository

This repository contains the experimental nC compiler and its technical context.
nC is a sovereign, multi-level, semantically continuous language. Treat major
changes as contracts across levels, rather than isolated syntax features.
Preserve continuity between:

- `[@]`;
- `[@machine]`;
- `primitive[@]`;
- linking;
- memory, pointers and regions;
- runtime;
- future modal-system layers.

## Central rule

Parser acceptance alone does not close a phase. Parser, AST, semantic analysis,
IR, tests and documentation must agree.

## Current priority

Consolidate `[@machine]` as a stable, typed and coherent layer prepared for
material continuity. Preserve instruction typing, terminator typing, control
flow, the parser/sema/IR relationship and established diagnostics.

## Scope

Compiler infrastructure, the base root, base primitives, machine surface/sema/IR,
important linking paths and language documentation are advanced. Full backend
closure, deep runtime coverage, complete memory/pointer/region/aliasing
materialization and higher modal layers remain open beyond the profiles whose
evidence is explicitly recorded.

## Change rules

Make coherent typed changes. Reduce debt between parser, semantics and IR.
Add positive and negative fixtures where the contract requires them. Preserve
clear diagnostics, documentation aligned with implementation and commands
reproducible from WSL.

Do not add syntax without semantic justification, duplicate semantics between
machine and primitive layers, replace established typed paths with unstructured
strings, change canonical meaning without documentation, or break existing tests
to claim progress.

## Completion criteria

A change is not complete merely because it compiles. As applicable, verify
parser, AST, semantics, IR, tests and fixtures, the CLI, and documentation of any
surface or semantic decision.

Use `ncc` directly for supported language checks. With the retained Stage0 built,
run from the repository root on Linux or WSL:

```bash
./target/debug/ncc check examples/11_machine/checked_add_overflow_machine.n
./target/debug/ncc emit-ir examples/11_machine/checked_add_overflow_machine.n
```

On Windows, use `.\target\debug\ncc.exe`. The native kit uses its own
`compiler sema` and `compiler emit-ir` routes; do not assume CLI equivalence.

Rust implementation changes still require the Rust workspace checks after seed
configuration described in `docs/install.md`:

```bash
cargo fmt --all -- --check
cargo test --locked --workspace --no-fail-fast -- --nocapture
```

Keep public documentation and contribution-facing text in English.
