# MCT

MCT is the byte-contract layer used to keep backend output auditable.

Public beta status:

- Scalar and runtime-backed examples should declare whether they lower to MCT.
- Examples that are check-only must say so in their manifest entry.
- Generated public packages must not expose private MCT reports.

## Compact Pointer Surface

`byte*`, `int*` and deeper suffixes are parsed as canonical `ptr<T>` values.
Likewise, `Read*`, `Write*`, `Offset*`, `Diff*` and `Region*` are normalized to
their typed runtime operations before IR construction. Therefore the compact
source surface cannot create a parallel pointer ABI: MCT records the same load,
store, offset, difference and region-view byte operations as it does for
`ptrRead`, `ptrWrite`, `ptrOffset`, `ptrDiff` and `regionFromPtr`.

For a heap or mapped-memory view, `regionFromPtr<T>(p, n)` is live rather than a
copy. A store through `Write*<T>` is read by `regionRead<T>` from the same byte
address, and a `regionWrite<T>` updates the pointee that `Read*<T>` observes.

## Projected `stc` Bytes

`stc` compact initializers are part of the material contract when they are
constant scalar expressions. Unary, binary, reference and conditional
initializers lower to backend data bytes instead of staying as runtime-only
syntax.

Fixed scalar arrays also materialize:

```nC
base >> int = -8;
serie >> array<int, 3> = [1, -2, base + 11];
```

The backend emits `serie` as fixed storage bytes `[1, -2, 3]` using the element
width of `int`. MCT records this as `stc.array.literal`; uninitialized fixed
arrays remain `stc.array.zero_fill`.

Fixed string arrays are material too:

```nC
etiquetas >> array<str, 3> = ["alta", "media", "baja"];
```

Their data section is a three-slot pointer table. Each slot receives an `Abs64`
relocation to a dedicated NUL-terminated UTF-8 byte sequence in the container's
`.rodata` section. The generated object therefore retains both the logical
array layout and the exact text bytes for the linker.

In the LLVM lane, constant `str` arrays become an internal global pointer table
whose entries refer to `private unnamed_addr` UTF-8 globals. They do not pass
through an initializer-stack allocation, so the pointers remain valid for the
entire program lifetime. Repeated literals share one physical byte sequence.
