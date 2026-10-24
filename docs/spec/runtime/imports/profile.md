# Runtime Import: `audit:profile`

## Import

```nC
use <audit:profile>;
```

## Contract

`profile` exposes caller-owned 48-byte descriptors: mode, flags, CPU-feature
mask, optimization level, target address and target byte count. The compiler
does not discover, allocate or normalize a profile on behalf of the program.

```txt
profileStride() -> int
profileWrite(table: addr, index: int, mode: int, flags: int,
             cpuFeatures: int, optimization: int,
             target: addr, targetBytes: int) -> bool
profileMode(table: addr, index: int) -> int
profileFlags(table: addr, index: int) -> int
profileCpuFeatures(table: addr, index: int) -> int
profileOptimization(table: addr, index: int) -> int
profileTarget(table: addr, index: int) -> addr
profileTargetBytes(table: addr, index: int) -> int
profileHas(table: addr, index: int, requiredFlags: int) -> bool
profileRequire(table: addr, index: int, requiredFlags: int) -> bool
profileCopy(destination: addr, destinationIndex: int,
            source: addr, sourceIndex: int) -> bool
profileClear(table: addr, index: int) -> bool
```

`profileRequire` traps when the requested bits are absent. Invalid memory and
indices remain raw undefined behavior. The implementation is direct
LLVM/Machine code and does not use C or a hosted profile object.

Canonical executable example:
`examples/runtime/families/audit_manual_family_raw.n`.
