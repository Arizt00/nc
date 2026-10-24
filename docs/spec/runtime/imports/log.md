# Runtime Import: `audit:log`

## Import

```nC
use <audit:log>;
```

## Contract

`log` is a manual binary-record API. The caller owns and sizes every table and
message buffer. A record is 48 bytes: level, category, message address, message
length, numeric value and flags. No allocation, formatting, string ownership,
bounds check, persistence or recovery is implicit.

```txt
logStride() -> int
logWrite(table: addr, index: int, level: int, category: int,
         message: addr, messageBytes: int, value: int, flags: int) -> bool
logLevel(table: addr, index: int) -> int
logCategory(table: addr, index: int) -> int
logMessage(table: addr, index: int) -> addr
logMessageBytes(table: addr, index: int) -> int
logValue(table: addr, index: int) -> int
logFlags(table: addr, index: int) -> int
logCopy(destination: addr, destinationIndex: int,
        source: addr, sourceIndex: int) -> bool
logClear(table: addr, index: int) -> bool
```

Null, invalid, overlapping, undersized or out-of-range memory is accepted by
the language and has raw undefined behavior. `logCopy` copies exactly 48 bytes.
The API materializes as internal LLVM/Machine symbols; it has no C provider.

Canonical executable example:
`examples/runtime/families/audit_manual_family_raw.n`.
