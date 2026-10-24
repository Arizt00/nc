# Iterators, generators and async

Status: closed for the declared high-level and backend profiles.

## Surface

- `Iterator<T>` is the finite pull protocol and returns `Option<T>` from `next()`.
- Fixed arrays, dynamic arrays, vectors, lists and regions lower `for-in` to an explicit index loop.
- `Generator<T>`, `Stream<T>` and `AsyncStream<T>` preserve `T` without `any` erasure.
- `yield value;` and `yield(value);` are the same typed suspension operation.
- `async fn` and async closures return a typed `Future<T>` descriptor.
- `resume()`/`poll()`, `payload()`/`current()`, `drop()`/`close()` and supported `cancel()` calls are descriptor operations, not runtime reflection.

Sema rejects wrong protocol arity, wrong yielded item types, orphan `yield`, non-async `AsyncStream<T>`, invalid method arity and cancellation of synchronous generators/streams.

## Executable state machine

Every coroutine is split before backend emission into real machine functions:

1. `new`: initializes the descriptor and frame.
2. `resume`: dispatches the stored state and executes until pending, yield, completion or cancellation.
3. `drop`: runs active cleanup exactly once and releases hosted storage.
4. `cancel`: publishes cancellation and makes the next resume return `Cancelled`.

The descriptor is four pointer-width words in this order: `resume`, `frame`, `drop`, `cancel`. Status values are `0 Pending`, `1 Yielded`, `2 Complete`, and `3 Cancelled`.

`await` polls the child descriptor, returns `Pending` without advancing when the child is pending, and copies the typed payload only after completion. `yield` stores the typed payload, advances the state and returns `Yielded`. Aggregate payloads use resolved size, alignment and byte copies; they are never collapsed to one machine word.

Error propagation is also a state transition. For `Result<T, E>`, `?` evaluates its operand once,
branches on the tag, extracts `T` on success, and on failure stores the original `Result<T, E>` in
the frame payload before cleanup and `Complete`. A raw tagged value can therefore never escape from
`resume` as though it were one of the four status values.

## Lifetime and cleanup

Variables live across suspension have deterministic frame slots. Active `defer` scopes are represented by frame flags. Cleanup is LIFO, is not run on `Pending` or `Yielded`, and runs once on normal completion, cancellation/drop or explicit close. Repeated drop cannot repeat cleanup.

Hosted profiles own frame and descriptor storage through explicit `malloc`/`free` calls. Portable, freestanding, kernel and sovereign profiles are caller-owned. Their constructor receives storage and cannot introduce heap allocation or scheduler calls. Polling is explicit; a scheduler exists only when source code asks for one.

## ABI and targets

The layout is derived from each target data layout. The retained matrix covers SysV64, Win64, x86_64 Darwin, AAPCS64, Linux/Windows AArch64, wasm32 portable and sovereign profiles. Coroutine lowering uses typed load, store, compare, branch, direct/indirect call and admitted atomic acquire/release operations, so no vendor-private instruction is a semantic requirement.

## Evidence

- `cargo test -p ncc-sema generator_stream_semantics_tests -- --nocapture`
- `cargo test -p ncc-sema async_closure_semantics_tests -- --nocapture`
- `cargo test -p ncc-sema monomorphize::tests -- --nocapture`
- `cargo test -p ncc-ir coroutine::tests -- --nocapture`

The IR tests retain Clang link/run smoke for hosted futures, generators, async closures, aggregate payloads, descriptor methods, cancellation, cleanup and `Result` propagation through canonical imported `await`. They also retain real ELF64 object bytes for generated lifecycle functions and require complete direct-backend coverage for every function in the combined `await + Result + ? + defer` fixture.
