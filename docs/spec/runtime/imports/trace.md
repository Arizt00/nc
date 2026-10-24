# Runtime Import: `audit:trace`

## Import

```nC
use <audit:trace>;
```

## Contract

`trace` records execution events in caller-owned 56-byte records: kind, id,
parent id, tick, payload address, payload length and numeric value. There is no
hidden clock read, allocation, synchronization, persistence or formatting.

```txt
traceStride() -> int
traceWrite(table: addr, index: int, kind: int, id: int, parent: int,
           tick: int, payload: addr, payloadBytes: int, value: int) -> bool
traceKind(table: addr, index: int) -> int
traceId(table: addr, index: int) -> int
traceParent(table: addr, index: int) -> int
traceTick(table: addr, index: int) -> int
tracePayload(table: addr, index: int) -> addr
tracePayloadBytes(table: addr, index: int) -> int
traceValue(table: addr, index: int) -> int
traceCopy(destination: addr, destinationIndex: int,
          source: addr, sourceIndex: int) -> bool
traceClear(table: addr, index: int) -> bool
```

The program chooses identifiers, timestamps and storage. Invalid memory and
indices have raw undefined behavior. The complete API is implemented as direct
LLVM/Machine symbols and has no C provider.

Canonical executable example:
`examples/runtime/families/audit_manual_family_raw.n`.
