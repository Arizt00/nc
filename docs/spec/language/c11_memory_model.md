# C11-style memory model

Status: canonical for the admitted scalar atomic profile.

The language exposes atomic storage with `atomic<T>` and ordered operations
from `core::sync`. Atomicity, volatile access and compiler barriers are three
different contracts and are never treated as synonyms.

## Storage and two execution surfaces

`atomic<T>` is direct typed storage. Calls whose first argument is an
`atomic<T>` lower to IR and target instructions without a hosted runtime
handle. The payload type controls storage width and argument checking.

`AtomicHandle` is the hosted service surface. It uses the same order rules but
returns `Result<..., ThreadError>` where registry or wait-service operations
can fail. Compatibility operations without an order are permanently
`seq_cst`.

## Memory orders

The canonical order names are:

```text
relaxed acquire release acq_rel seq_cst
```

Order legality is checked before IR generation:

| Operation | Legal orders |
| --- | --- |
| load and wait | `relaxed`, `acquire`, `seq_cst` |
| store | `relaxed`, `release`, `seq_cst` |
| exchange and fetch RMW | all five orders |
| thread/compiler fence | all five orders |
| compare-exchange success | all five orders |
| compare-exchange failure | `relaxed`, `acquire`, `seq_cst` |

A compare-exchange failure order cannot be `release` or `acq_rel` and cannot
be stronger than its success order. Unknown names and invalid pairs are hard
semantic errors.

## Operations

The ordered surface contains:

- `atomicLoadOrder` and `atomicStoreOrder`;
- `atomicExchange`;
- `atomicFetchAdd`, `atomicFetchSub`, `atomicFetchAnd`, `atomicFetchOr` and
  `atomicFetchXor`;
- `atomicCompareExchangeStrong` and `atomicCompareExchangeWeak`;
- `atomicThreadFence` and `atomicCompilerFence`;
- `atomicWait`, `atomicNotifyOne` and `atomicNotifyAll`;
- `volatileLoad` and `volatileStore`.

Fetch and compare-exchange operations return the value observed before the
operation. Direct typed stores, fences, notifications and volatile stores
return `void`. The hosted-handle surface preserves its documented `Result`
and count return types.

`atomicWait(cell, expected, order)` observes the cell while it equals
`expected` and returns the first different value. Hosted execution parks with
a condition variable. Native lowering uses an architecture wait loop and the
target's wait/yield hint. Notifications wake one or all hosted waiters and map
to the target notification instruction where one exists.

## Volatile and barriers

Volatile load/store guarantee an observable memory access. They do not create
inter-thread synchronization. A compiler fence constrains compiler motion but
does not emit a hardware memory fence. A thread fence emits the ordering
required by the selected target.

## Type checking

Direct calls derive their value and return type from `atomic<T>`. Store, RMW,
compare-exchange and volatile-store operands must match `T`. Side-effect-only
operations are typed `void`; using them as a value is rejected by the normal
expression type rules.

The backend accepts atomic payloads that map to legal first-class target
storage. Unsupported aggregate or alignment combinations must fail during
target legalization; they are never silently changed into non-atomic loads or
stores.

## Example

```text
use core.sync { atomic };

counter >> atomic<int> = 0;
atomicStoreOrder(counter, 1, "release");
old >> int = atomicFetchAdd(counter, 2, "acq_rel");
now >> int = atomicLoadOrder(counter, "acquire");
```

## Evidence

- Canonical orders and legality: `crates/ncc-ast/src/memory_model.rs`.
- Semantic integration: `crates/ncc-sema/src/lib.rs` and
  `crates/ncc-sema/tests/c11_memory_model.rs`.
- Hosted runtime: `crates/ncc-runtime/src/atomic.rs`.
- Direct backend lowering: `crates/ncc-ir/src/lib.rs` and
  `crates/ncc-ir/src/c11_memory.rs`.
- High-level fixture: `examples/06_memory/c11_atomic_native.n`.
- Hosted fixture: `examples/06_memory/c11_atomic_orders.n`.

Normative regression commands:

```text
cargo test -p ncc-sema --test c11_memory_model -- --nocapture
cargo test -p ncc-cli --test c11_atomic_runtime_e2e -- --nocapture
cargo test -p ncc-cli --test core_alloc_c11_native_e2e -- --nocapture
```
