# Explicit Binary Layout

Status: implemented in parser, AST, sema, IR, interpreter and native backend.

Binary layout is declared in a type contract:

```n
Packet {
    public:
        flags: dword;
        mode: dword;
        code: byte;
        tail: array<byte, 0>;
    layout:
        repr(C);
        packed;
        align(8);
        endian(little);
        bitfield(mode, flags, 1, 3);
        flexible(tail);
}
```

## Layout items

- `repr(C)` uses the target C ABI field order and target pointer width.
- `repr(native)` selects the native nC representation.
- `packed` reduces field alignment to one byte.
- `align(N)` requires power-of-two aggregate alignment.
- `size(N)` pads the aggregate and must contain its natural layout.
- `endian(native|little|big)` converts integer field loads and stores.
- `bitfield(field, storage, offset, width)` aliases bits of integer storage.
- `flexible(field)` marks the final `array<T>` or `array<T,0>` member.

`Name union { ... }` is a real union: every member starts at offset zero; size
and alignment are the maxima required by its members. A union cannot contain a
flexible member.

Bitfield members share the declared storage field. Width must be non-zero,
must fit the storage integer, and cannot overlap another bitfield assigned to
the same storage. Writes use read-mask-merge-store; reads shift and mask.

A flexible member contributes zero bytes to `sizeOf<T>()`, retains element
alignment and is emitted as `[0 x T]` in LLVM. It must be final and cannot be
used to hide a descriptor or a fixed non-zero array.

## Compile-time queries

```n
packetSize >> int = sizeOf<Packet>();
packetAlign >> int = alignOf<Packet>();
codeOffset >> int = offsetOf<Packet>(code);
staticAssert(packetSize == 8, "Packet ABI drift");
```

Queries are constant-folded by sema and lowering. `staticAssert` requires a
compile-time boolean and rejects false with `E_STATIC_ASSERT_001`.

## Materialization

- Nested structs and fixed arrays use recursively computed offsets and stride.
- Arrays embedded in fields support indexed read/write, arrays of structs and
  multidimensional linearization.
- Explicit byte-storage aggregates use byte-offset GEPs in LLVM.
- Endian conversion uses native byte-swap operations where target order differs.
- The interpreter mirrors the same backing bytes, aliasing and field offsets.
- ABI metadata exports size, alignment, offsets, representation, endian,
  bitfield and flexible-member information.

Primary diagnostics use the `E_LAYOUT_*` family and `E_STATIC_ASSERT_001`.

## ABI certification

`abi_layout_multitarget` compiles an equivalent C ABI probe with Clang for
x86_64, AArch64 and RISC-V, with GCC and its cross compilers on Linux, and with
MSVC on Windows. It verifies aggregate assertions at compile time and checks
the emitted ELF or COFF architecture directly from each object header.

The same test asks nC to lower `explicitBinaryLayout.n` into real cross-target
objects and verifies their object format and architecture. The host smoke also
links and executes the nC ELF through the operating-system loader. This keeps
the FFI contract tied to material object bytes, not only to IR metadata.
