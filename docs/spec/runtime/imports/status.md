# Runtime Import: `audit:status`

## Import

```nC
use <audit:status>;
```

## Contract

`status` stores operational state in caller-owned 40-byte records: kind, code,
message address, message length and detail. It never allocates or renders a
managed status value.

```txt
statusStride() -> int
statusWrite(table: addr, index: int, kind: int, code: int,
            message: addr, messageBytes: int, detail: int) -> bool
statusKind(table: addr, index: int) -> int
statusCode(table: addr, index: int) -> int
statusMessage(table: addr, index: int) -> addr
statusMessageBytes(table: addr, index: int) -> int
statusDetail(table: addr, index: int) -> int
statusCopy(destination: addr, destinationIndex: int,
           source: addr, sourceIndex: int) -> bool
statusClear(table: addr, index: int) -> bool
```

The message is a borrowed address/length pair. Lifetime, termination and
encoding are the programmer's responsibility. Invalid memory and indices have
raw undefined behavior. All operations materialize in LLVM/Machine without C.

Canonical executable example:
`examples/runtime/families/audit_manual_family_raw.n`.
