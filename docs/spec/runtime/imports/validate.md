# Runtime Import: `audit:validate`

## Import

```nC
use <audit:validate>;
```

## Contract

`validate` contains explicit predicates over raw addresses, lengths and integer
values. Predicates return `bool`; they do not create diagnostics, repair data,
throw exceptions or install safety checks around later accesses.

```txt
validateNotNull(value: addr) -> bool
validateRange(value: int, minimum: int, maximum: int) -> bool
validateLength(length: int, minimum: int, maximum: int) -> bool
validateEqual(left: addr, leftBytes: int, right: addr, rightBytes: int) -> bool
validateAscii(data: addr, bytes: int) -> bool
validateUtf8(data: addr, bytes: int) -> bool
validateAligned(value: addr, alignment: int) -> bool
validatePowerOfTwo(value: int) -> bool
validateZero(data: addr, bytes: int) -> bool
validateMask(value: int, required: int, forbidden: int) -> bool
```

`validateUtf8` checks complete one-to-four-byte UTF-8 sequences, including
overlong, surrogate and Unicode-limit restrictions. Passing inaccessible raw
memory is undefined behavior. All predicates lower directly to LLVM/Machine;
there is no C provider.

Canonical executable example:
`examples/runtime/families/audit_manual_family_raw.n`.
