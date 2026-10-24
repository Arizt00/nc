# Backend Type ABI Layout v1

Status: closed for named type-contract aggregate storage ABI.

This document defines the stable nC type ABI layout emitted by the backend and
MCT. It is the storage/memory representation contract for named `type:`
contracts; it is not a claim of complete foreign aggregate pass/return ABI
classification.

## Source Of Truth

The executable source of truth is `TypeAbiLayoutIR` in `ncc-ir`.

`TypeAbiLayoutIR` is derived from the same layout pass that feeds LLVM named
struct emission. Backend type-contract units and MCT LLVM modules consume that
single result:

- `BackendTypeContractUnitIR.abi_layouts`
- `MctLlvmModuleIR.type_abi_layouts`
- `.meta.type.<container>` object sections
- `; ncc-type-abi ...` comments in emitted LLVM IR

No backend lane should keep a private table of struct field offsets.

## Layout Algorithm

## Complete ABI Storage Matrix

The executable matrix is `backend_type_abi_storage_matrix()` in `ncc-ir`. This
table is the documentation view of the same contract:

| Family | Examples | Storage / LLVM shape | Size on 64-bit lane | Align on 64-bit lane | ABI surface | Status |
| --- | --- | --- | ---: | ---: | --- | --- |
| `void` / `unit` | `void`, `unit` | `void` | 0 | 1 | no value storage | `void_no_storage` |
| Boolean | `bool` | `i1` | 1 | 1 | byte-addressable boolean storage | `scalar_load_store_bytes_real` |
| Character | `char` | `i8` | 1 | 1 | single-byte scalar character | `scalar_load_store_bytes_real` |
| Unsigned integers | `byte`, `word`, `dword`, `qword`, `uint` | `i8` / `i16` / `i32` / `i64` | 1 / 2 / 4 / 8 | 1 / 2 / 4 / 8 | zero-extended integer storage | `scalar_load_store_bytes_real` |
| Signed integers | `sbyte`, `sword`, `sdword`, `sqword`, `int` | `i8` / `i16` / `i32` / `i64` | 1 / 2 / 4 / 8 | 1 / 2 / 4 / 8 | sign-extended integer storage | `scalar_load_store_bytes_real` |
| Floating point | `ft32`, `ft64`, `float`, `double`, `ft` | `float` / `double` | 4 / 8 | 4 / 8 | IEEE storage lane selected by scalar contract | `float_storage_bytes_real` |
| Pointer/reference/text word | `addr`, `ptr<T>`, `ptr mut<T>`, `owner ptr<T>`, `ref<T>`, `ref mut<T>`, `site<T>`, `fn<...>`, `dyn<T>`, `str`, `bytes` | `ptr` | 8 | 8 | target pointer-width word | `pointer_storage_bytes_real` |
| Fixed arrays | `array<dword,4>`, `array<byte,16>` | `[N x T]` | element size * N | element alignment | inline contiguous element storage | `aggregate_storage_bytes_real` |
| Fixed bytes | `bytes<16>` | `[N x i8]` | N | 1 | inline byte buffer | `fixed_bytes_storage_real` |
| Dynamic views | `array<T>`, `slice<T>`, `span<T>`, `memRegion<T>` | `{ ptr, i64 }` | 16 | 8 | data pointer + length descriptor | `dynamic_view_descriptor_bytes_real` |
| Dynamic owning arrays | `dinArray<T>`, `dinArray.binary<T>`, `dinArray.unbinary<T>` | `{ ptr, i64, i64 }` | 24 | 8 | data pointer + length + capacity descriptor | `dynamic_view_descriptor_bytes_real` |
| SIMD vector | `vector<byte>`, `vector<word>`, `vector<dword>`, `vector<qword>`, `vector<ft32>`, `vector<ft64>` | `<lanes x T>` | 16 | 16 | fixed 128-bit vector lane | `vector_storage_bytes_real` |
| Union storage | `dword\|qword`, `byte\|ft64` | `[max_member_size x i8]` | max member size | max member alignment | largest-member byte payload | `union_storage_bytes_real` |
| Type contract struct | `type Packet { ... }` | `%struct.<container>_<type>` | source-order natural layout | max field alignment | `TypeAbiLayoutIR` with field offsets | `stable_type_abi_v1` |
| Type contract enum | `enum Status { ... }` | `{ i32 }` | 4 | 4 | `dword` tag | `stable_type_abi_v1` |
| Type contract tagged union | `tagged union Event { ... }` | `{ i32, [N x i8] }` | tag + max payload rounded to alignment | max(4, payload alignment) | `dword` tag plus largest payload | `stable_type_abi_v1` |
| Opaque fallback | unresolved type string | `i64` | 8 | 8 | explicit fallback, not a stable ABI claim | `opaque_qword_storage_real` |

Native-fast string literals use the pointer/reference/text word row at call boundaries. The word
points at an immutable `{ data_ptr, byte_len }` descriptor in `.rodata`; the descriptor's data pointer
relocates to the NUL-terminated UTF-8 payload, and `byte_len` excludes that final NUL.

The table is considered complete only while the test
`backend_type_abi_storage_matrix_is_complete_and_executable` passes: every
documented lowerable example must lower through the storage class and native
status declared by the matrix, and every named-type family must remain covered.

For `Struct` contracts:

- fields are taken from every `fields` clause in source order;
- visibility does not reorder storage;
- each field offset is aligned up to the field alignment;
- struct size is rounded up to the maximum field alignment;
- field descriptors record nC type, LLVM storage type, index, offset, size and
  alignment.

For `Enum` contracts:

- the ABI layout is a single `dword` tag at offset 0;
- size is 4 bytes and alignment is 4 bytes.

For `TaggedUnion` contracts:

- field 0 is the `dword` tag;
- field 1, when needed, is a `bytes<N>` payload;
- payload size and alignment are computed from the largest/aligned variant
  payload.

## Stability Status

Each layout carries:

- `schema = "nc.type_abi.layout.v1"`
- `status = "stable_type_abi_v1"` when every field lowers to known storage;
- `status = "opaque_fallback_type_abi_v1"` when any field still depends on an
  opaque fallback.
- `fingerprint`: deterministic FNV-1a hash over schema, qualified type name,
  kind, size, alignment and every field descriptor.

The fingerprint is not a security hash. It is a reproducible ABI drift marker
for tests, object metadata and tooling.

## Backend And MCT Emission

`emit-backend` renders lines such as:

```text
; abi typespace.Account kind=struct status=stable_type_abi_v1 size=32 align=8 fields=5 fingerprint=...
; abi_field typespace.Account.pin index=2 ty=qword llvm=i64 offset=8 size=8 align=8
```

`emit-object` stores equivalent data in `.meta.type.<container>` as bytes:

```text
type_abi typespace.Account kind=struct status=stable_type_abi_v1 size=32 align=8 fields=5 fingerprint=...
type_abi_field typespace.Account.owner index=4 ty=int llvm=i32 offset=24 size=4 align=4
```

`mct_emit_llvm_module` exposes `type_abi_layouts` and mirrors the ABI in LLVM IR
comments before each named `%struct` declaration.

## Boundaries

Closed here:

- stable storage layout for named `Struct`, `Enum` and `TaggedUnion` contracts;
- field order, byte offsets, sizes and alignments;
- deterministic ABI fingerprinting;
- backend, object and MCT visibility of the ABI layout.

Not claimed here:

- complete C/C++/Rust aggregate call and return classification;
- packed/repr override semantics beyond the currently represented field layout;
- optimizer-level alias/lifetime proof for all aggregate fields.
