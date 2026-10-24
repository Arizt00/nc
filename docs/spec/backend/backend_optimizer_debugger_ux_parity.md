# Backend Optimizer Debugger UX Parity

Status: closed for every optimized transformation category currently emitted by
the ncc AST and backend peephole optimizers.

This document closes the debugger UX contract for optimized ncc code. The
optimizer is allowed to fold pure expressions, prune unreachable statements and
remove backend-only housekeeping operations, but it must not make those changes
invisible to tools. Every non-zero optimizer counter now produces a structured
`debugger_ux` record in `OptimizationReportIR` and in `ncc optimize --json`.

## Closed Surface

- `OptimizationDebuggerUxTransformIR`.
- `OptimizationReportIR.debugger_ux_schema`.
- `OptimizationReportIR.debugger_ux_status`.
- `OptimizationReportIR.debugger_ux_parity`.
- `OptimizationReportIR.debugger_ux_transformations`.
- `OptimizationReportIR.debugger_ux_diagnostics`.
- CLI JSON under `internal_optimizer.ast.debugger_ux`.
- CLI JSON under `internal_optimizer.backend.debugger_ux`.
- Tests proving AST folds, dead statement pruning and machine peepholes are
  represented in the debugger UX report.

## Transformation Coverage

The current optimizer emits debugger UX records for these transformation
categories:

- `constant_fold_or_pure_simplification`.
- `dead_control_flow_or_unreachable_statement_elimination`.
- `backend_function_peephole_elimination`.
- `backend_machine_peephole_elimination`.

Each row includes:

- pass name;
- transformation name;
- instance count;
- before/after explanation;
- debugger effect;
- source-span policy;
- breakpoint policy;
- variable materialization policy;
- whether the row is user visible.

## Source Span Policy

Optimizer rows do not replace source maps. They explain how optimization affects
stepping, variables and breakpoints. Source locations remain owned by
`ProgramSourceMapIR`, `StatementSourceSpanIR` and the LLVM debug metadata path.
If a statement lacks an exact span, that missing evidence remains visible in the
source-map report instead of being hidden by the optimizer.

## CLI Evidence

```sh
ncc optimize examples/performance/native_fast/optimizer_fold_dead.n --json
```

Expected JSON:

```json
{
  "internal_optimizer": {
    "ast": {
      "debugger_ux": {
        "schema": "ncc.optimizer.debugger_ux.v1",
        "status": "debugger_ux_parity_closed",
        "parity": true
      }
    }
  }
}
```

## Truth Boundary

Closed:

- debugger UX records for every optimized transformation category currently
  performed by the ncc AST optimizer and backend peephole optimizer;
- JSON evidence consumable by IDE/LSP layers;
- explicit breakpoint and variable policies for optimized-out source regions;
- preservation of source-span accountability through the existing source-map
  report.

Not claimed:

- bit-for-bit debugger behavior parity with every Clang/rustc optimization
  mode;
- full optimized variable location reconstruction for arbitrary future language
  constructs;
- a substitute for DWARF source maps or native debugger integration.

Any future optimizer transform that increments a new counter must add a
`debugger_ux` row before it can be considered closed.
