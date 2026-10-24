# ncc Internal Optimizer

Status: closed for the release-fast internal optimization surface.

This document defines the conservative optimizer used by `ncc optimize` and the `perf native-fast` nC build path. The optimizer is intentionally internal: it does not introduce new language syntax and it does not change parser, semantic, pointer, basal, machine, or runtime rules.

## Pipeline

The pipeline is:

1. Parse source into the nC AST.
2. Run semantic and contract gates.
3. Run the AST optimizer.
4. Lower optimized AST to `ProgramIR`.
5. Emit `BackendModuleIR`.
6. Run backend peephole optimization.
7. Emit `BackendObjectIR`.
8. Emit/link the native-fast artifact when requested.

## AST Optimizer

The AST optimizer folds pure literal expressions and removes unreachable statements in a block after terminal statements.

Implemented folds:

- integer arithmetic: `+`, `-`, `*`, `/`, `%` when overflow/division-by-zero can be avoided;
- integer bit operations: `&`, `|`, `^`, `<<`, `>>`, `>>>`;
- integer comparisons: `<`, `<=`, `>`, `>=`, `==`, `!=`;
- boolean operations: `&&`, `||`, `==`, `!=`, `!`;
- string equality/inequality;
- `if`/`unless`/`while false` pruning when the condition is statically boolean;
- expression `if-else` selection when the condition is statically boolean.

The optimizer does not fold calls, pointer operations, memory operations, FFI boundaries, raw regions, or any expression requiring side-effect knowledge.

## Backend Peephole

The backend peephole pass removes:

- zero-sized stack reservation/release operations;
- duplicate adjacent function labels;
- machine `nop` operations;
- duplicate adjacent machine labels;
- duplicate adjacent machine final operations;
- backend ops after terminal backend returns/finals.

This pass is structural and does not invent new backend instructions.

## JSON Report

`ncc optimize <file.n> --json` includes:

- `internal_optimizer.schema = ncc.performance.internal_optimizer.v1`;
- AST before/after statement counts;
- constant fold count;
- removed statement count;
- backend before/after op counts;
- backend and machine op removal counts;
- pass list with before/after/removed/changed fields.

The report is evidence for actual optimization and does not claim C/Rust optimizer parity by itself.

## Boundaries

The optimizer is conservative by design. It does not:

- assume alias safety;
- erase raw pointer hazards;
- fold memory reads/writes;
- evaluate runtime or FFI calls;
- change machine official semantics;
- change basal lowering, NBB, evidence, or seal semantics.
