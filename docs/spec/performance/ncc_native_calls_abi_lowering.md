# nCC Native Calls ABI Lowering

Status: closed for the current x86_64 `nC.sysv64` native-fast scalar backend.

This document defines the native call lowering surface used by the release-fast
path after native control-flow lowering. The goal is to make calls occur at the
same source position as the AST expression or statement that caused them, while
still using the existing nC ABI planning data.

## Covered Surface

- Local same-container calls are lowered to native `call rel32` with a
  `CallNear32` relocation.
- Scalar return values are read from `rax`.
- Register arguments are materialized into the SysV64 argument registers:
  `rdi`, `rsi`, `rdx`, `rcx`, `r8`, and `r9`.
- Stack arguments beyond the register set are materialized into the outgoing
  argument area at `[rsp + offset]`.
- Nested calls in arguments are evaluated first, stored into frame temporaries,
  then copied into final ABI locations.
- Calls in `if`, `while`, `for`, `loop`, `repeat`, and multiple-return blocks
  are emitted in lexical control-flow position.
- Calls used as expression statements are preserved and are not removed by
  scalar constant folding.
- Functions with stack parameters use a frame pointer so incoming stack
  arguments remain stable across local spills.

## Frame Layout

The scalar native builder separates the outgoing argument area from local spill
slots. Local variables and temporaries are placed above the outgoing call area,
so stack arguments do not overwrite scalar locals. For functions that need
`rbp`, incoming stack arguments are addressed from `rbp + 16`, `rbp + 24`, and
so on.

For scalar plans, the payload area is aligned after callee-saved registers are
pushed. This keeps `rsp` aligned at call boundaries for the native-fast SysV64
path.

## Non-Goals

This phase does not claim full native register allocation, whole-program
inlining, tail-call optimization, varargs, aggregate ABI lowering, exception
ABI, or cross-mode bridge execution inside scalar calls. Those remain later
backend work. The existing backend call-site model still documents bridge and
foreign-boundary metadata.

## Fixtures

- `examples/performance/native_fast/call_branch_return.n`
- `examples/performance/native_fast/call_nested_args.n`
- `examples/performance/native_fast/call_stack_args.n`
- `examples/performance/native_fast/call_loop_accumulate.n`
- `examples/performance/native_fast/call_statement_before_return.n`

Each fixture returns `42` through `ncc run --native-fast --show-return`.
