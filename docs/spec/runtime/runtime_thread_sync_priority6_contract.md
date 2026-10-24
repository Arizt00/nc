# Runtime Thread/Sync Priority 6 Contract

Status: `closed_real_backing_bytes`

This phase closes the process-local runtime backing for thread and synchronization
families used by normal `.n` programs through `ncc run archivo.n`.

## Imports

Supported compact imports:

- `use auth.system { thread };`
- `use stoud.task { thread };`
- `use stoud.sync { sync };`
- `use stoud.sync { lock };`
- `use stoud.sync { atomic };`
- `use stoud.sync { channel };`

Each import registers real runtime symbols in the import closure table and emits
MCT activation bytes. No `--real` flag is required.

## Nominal Handles

All handles lower as registry slots with stable `qword` layout:

- `ThreadHandle`
- `MutexHandle`
- `RwLockHandle`
- `BarrierHandle`
- `AtomicHandle`
- `ChannelHandle`
- `ThreadLocalHandle`

Errors are returned as `ThreadError` inside `Result<T, ThreadError>`.

## Thread

- `threadSpawn(@fn(...)) -> Result<ThreadHandle, ThreadError>`
- `threadJoin(handle) -> Result<any, ThreadError>`
- `threadJoinTimeout(handle, timeoutMs) -> Result<any, ThreadError>`
- `threadId() -> int`
- `threadSleep(ms) -> void`
- `threadLocalCreate(default) -> Result<ThreadLocalHandle, ThreadError>`
- `threadLocalGet(handle) -> Result<any, ThreadError>`
- `threadLocalSet(handle, value) -> Result<bool, ThreadError>`
- `threadLocalRemove(handle) -> Result<bool, ThreadError>`

`threadJoin` consumes the handle. `threadJoinTimeout` returns `Err` on timeout
and leaves the handle joinable.

## Sync

- `mutexCreate() -> Result<MutexHandle, ThreadError>`
- `lock(handle) -> Result<bool, ThreadError>`
- `tryLock(handle) -> Result<bool, ThreadError>`
- `unlock(handle) -> Result<bool, ThreadError>`
- `barrierCreate(parties) -> Result<BarrierHandle, ThreadError>`
- `barrierWait(handle) -> Result<bool, ThreadError>`
- `rwLockCreate() -> Result<RwLockHandle, ThreadError>`
- `readLock(handle) -> Result<bool, ThreadError>`
- `tryReadLock(handle) -> Result<bool, ThreadError>`
- `writeLock(handle) -> Result<bool, ThreadError>`
- `tryWriteLock(handle) -> Result<bool, ThreadError>`
- `readUnlock(handle) -> Result<bool, ThreadError>`
- `writeUnlock(handle) -> Result<bool, ThreadError>`

`runtime.lock` is a specialized alias for the rwlock subset.

## Atomic

- `atomicCreate(initial) -> Result<AtomicHandle, ThreadError>`
- `atomicLoad(handle) -> Result<int, ThreadError>`
- `atomicStore(handle, value) -> Result<bool, ThreadError>`
- `atomicAdd(handle, delta) -> Result<int, ThreadError>`
- `compareExchange(handle, expected, newValue) -> Result<bool, ThreadError>`
- `atomicFence() -> void`

All operations use `SeqCst` ordering in the current runtime.

## Channel

- `channelCreate(capacity) -> Result<ChannelHandle, ThreadError>`
- `channelSend(handle, value) -> Result<bool, ThreadError>`
- `channelTrySend(handle, value) -> Result<bool, ThreadError>`
- `channelRecv(handle) -> Result<any, ThreadError>`
- `channelTryRecv(handle) -> Result<any, ThreadError>`
- `channelRecvTimeout(handle, timeoutMs) -> Result<any, ThreadError>`
- `channelClose(handle) -> Result<bool, ThreadError>`

The family also keeps stdio channel helpers:

- `stdin()`
- `stdout()`
- `stderr()`
- `writeTo(channel, text)`
- `readFrom(channel)`
- `flushChannel(channel)`

## Evidence

Examples:

- `examples/runtime/runtime_real_thread_join_timeout.n`
- `examples/runtime/runtime_real_sync_atomic.n`
- `examples/runtime/runtime_real_channel_threadlocal.n`
- `examples/runtime/negative/runtime_real_sync_invalid_mutex_handle.n`
- `examples/runtime/negative/runtime_real_channel_timeout.n`

Tests:

- `cargo test -p ncc-cli --test runtime_process_thread_contract -- --nocapture`
- `cargo test -p ncc-ast -- --nocapture`
