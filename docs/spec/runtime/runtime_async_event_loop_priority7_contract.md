# Runtime async/event loop priority 7

Status: `closed_real_backing_bytes`.

This priority closes the first real async/event-loop layer for `ncc run archivo.n`
without requiring `--real`. The surface is intentionally small, typed and backed by
runtime registry handles so future machine/native lowering has one ABI contract.

## Imports

```n
use stoud.net { tcp };
use stoud.task { async, cancel, scheduler, task };
use stoud.time { timer };
```

These imports are registered in the runtime closure table as
`closed_real_backing_bytes`. The backend/MCT manifest exposes activation bytes and
runtime symbols for each callable operation.

## Handles and errors

- `FutureHandle`: owned handle for a pending or completed async future.
- `TaskHandle`: owned handle for scheduled runtime tasks.
- `TimerHandle`: stopwatch handle backed by the process monotonic clock.
- `CancelToken`: standalone cooperative cancellation handle.
- `EventHandle`: named runtime event handle.
- `AsyncError`: typed error payload used by async, task, scheduler, timer, cancel
  and event operations.

All handles lower as qword registry slots in the runtime symbol ABI. `Result<T,E>`
keeps the same tagged layout used by process/thread/fs/network runtime APIs.

## Timer and await

`timerStart()` creates a real monotonic stopwatch. `timerStop(timer)` returns
elapsed milliseconds. `timeout(ms)` creates a non-blocking future that resolves
with `true` after the delay:

```n
future: Result<FutureHandle, AsyncError> = timeout(5);
waited: Result<bool, AsyncError> = await(future);
```

`futureReady(future)` polls without blocking. `await(future)` consumes the future
handle and returns its payload.

## Tasks and scheduler

`taskSpawn(@fn(worker))`, `schedule(@fn(worker))` and
`scheduleAfter(@fn(worker), ms)` run a referenced nC function on a runtime worker.
`scheduleEvery(@fn(worker), ms)` repeats until cancellation is requested.

The scheduler currently uses host runtime workers as the real event-loop backing.
It is not a parser-only feature and does not silently fall back to a fake
interpreter path.

## Cancellation

Cancellation is cooperative. `futureCancel(handle)`, `taskCancel(handle)` and
`cancel(token)` set an atomic cancellation flag. Pending timers and scheduled
delays observe the flag and finish as `Result.err(AsyncError)`.

This deliberately does not force-kill arbitrary running nC code. Hard preemption
belongs to a later machine/kernel layer.

## TCP async

TCP async calls create futures around real socket operations:

- `tcpConnectAsync(host, port) -> Result<FutureHandle, AsyncError>`
- `tcpAcceptAsync(listener) -> Result<FutureHandle, AsyncError>`
- `tcpSendAsync(stream, text) -> Result<FutureHandle, AsyncError>`
- `tcpRecvAsync(stream, maxBytes) -> Result<FutureHandle, AsyncError>`

`await` returns the inner network `Result`, for example:

```n
future: Result<FutureHandle, AsyncError> = tcpConnectAsync("127.0.0.1", 9000);
connected: Result<Result<TcpHandle, TcpError>, AsyncError> = await(future);
```

Send and receive clone the underlying stream before blocking I/O, so an async read
does not hold the global TCP registry lock while another future writes.

## Validation

The closure is covered by:

- `examples/runtime/runtime_real_async_timer_await.n`
- `examples/runtime/runtime_real_task_scheduler_cancel.n`
- `examples/runtime/runtime_real_tcp_async_loopback.n`
- `examples/runtime/negative/runtime_real_async_invalid_await_result.n`
- `crates/ncc-cli/tests/runtime_async_event_loop_contract.rs`
