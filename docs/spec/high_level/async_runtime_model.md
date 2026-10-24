# Async runtime model

Status: native state machine closed.

`Future<T>`, `Generator<T>`, `Stream<T>` and `AsyncStream<T>` lower to a typed descriptor containing resume, frame, drop and cancel pointers. The compiler emits the concrete lifecycle functions and turns `await` and `yield` into executable state transitions.

## Storage and scheduling

- Hosted profiles own coroutine frames on the heap.
- Kernel, freestanding, portable and sovereign profiles require caller-owned frames.
- Kernel lowering rejects hidden allocation and scheduler calls.
- Parallel scheduling is explicit; creating or awaiting a coroutine does not silently start a hosted scheduler.

## Lifetime

- Cancellation uses release/acquire-visible state.
- `defer` cleanup runs once on completion, cancellation or drop.
- Scalars, structs, enums and fixed arrays use layout-derived payload storage and byte copies.
- Async closures keep captures in persistent typed frame slots.
- `?` is part of the coroutine state graph: failure stores the complete `Result`, runs cleanup,
  publishes `Complete` and never returns tagged payload bits as a resume status.
- Both the `await` intrinsic and its canonical imported symbol `nC.runtime.async.await` poll the
  native descriptor directly; neither route requires a hosted await shim.

This document supersedes the historical Phase 9 semantic-only boundary.
