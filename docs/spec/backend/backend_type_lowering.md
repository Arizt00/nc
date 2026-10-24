# Backend Type Lowering

Status: closed for official scalar storage lowering and complete ABI storage
matrix coverage.

This document records how nC official types lower to backend storage. The goal
of this closure is concrete bytes for storage, load/store width, alignment and
LLVM type selection. It does not claim that every arithmetic family has a full
optimizer; native FP instruction selection is tracked by the machine FP profile.

The executable source of truth is the scalar type contract in `ncc-ast`.
Parser/sema/runtime/IR/MCT/LLVM must consume that contract instead of carrying
private scalar tables.

The complete backend ABI storage matrix lives in
`backend_type_abi_storage_matrix()` in `ncc-ir` and is documented in
`backend_type_abi_layout_v1.md`. The scalar table below is the expanded scalar
view; aggregate, descriptor, vector, union, named type and fallback rows are
kept in the ABI matrix so the backend does not drift into parallel private
tables.

## Scalar Types

| nC type family | LLVM storage | bytes | align | native memory lowering |
| --- | --- | ---: | ---: | --- |
| `bool` | `i1` | 1 | 1 | byte zero-extended load/store |
| `char` | `i8` | 1 | 1 | byte zero-extended load/store |
| `byte`, `u8`, `uint8` | `i8` | 1 | 1 | byte zero-extended load/store |
| `sbyte`, `i8`, `int8` | `i8` | 1 | 1 | byte sign-extended load/store |
| `word`, `u16`, `uint16`, `ushort` | `i16` | 2 | 2 | word zero-extended load/store |
| `sword`, `i16`, `int16`, `short` | `i16` | 2 | 2 | word sign-extended load/store |
| `dword`, `u32`, `uint32`, `uint` | `i32` | 4 | 4 | dword zero-extended load/store |
| `sdword`, `i32`, `int32`, `int` | `i32` | 4 | 4 | dword sign-extended load/store |
| `qword`, `u64`, `uint64`, `ulong` | `i64` | 8 | 8 | qword zero-extended load/store |
| `sqword`, `i64`, `int64`, `long` | `i64` | 8 | 8 | qword sign-extended load/store |
| `ft32`, `f32`, `float` | `float` | 4 | 4 | dword storage bytes |
| `ft64`, `f64`, `double`, `ft` | `double` | 8 | 8 | qword storage bytes |
| `void`, `unit` | `void` | 0 | 1 | no storage |

Default surface aliases are fixed: `int` is signed 32-bit, `uint` is unsigned
32-bit and `ft` is `ft64`. Integer literals may project into a narrower integer
scalar only when sema can prove the literal value fits the target width.

## Scalar Validation Pipeline

The same scalar contract is consumed in four places:

- `ncc-ast` owns official names, class, bit width, byte size, alignment and
  LLVM spelling;
- `ncc-sema` uses it to reject implicit narrowing and out-of-range integer
  literal projections;
- `ncc-cli` runtime uses it to validate materialized dynamic values, including
  unsigned ranges above `i64::MAX`;
- `ncc-ir` / MCT / LLVM consume it for storage, load/store width and extension
  policy.

This means `byte`, `char`, `word`, `dword`, `qword`, signed equivalents,
`ft32`, `ft64`, pointer-shaped scalars and text/bytes descriptors have one
declared byte contract from source to backend. `char` lowers as one byte; source
text still lowers through the UTF-8 text-byte path and is not confused with
`char`.

Arithmetic overflow is not silently claimed by this document. Literal/range
validation is closed here; per-operator overflow mode for runtime/native math is
tracked as expression lowering policy.

## Pointer And Reference Types

The pointer families lower to pointer-sized storage:

- `addr`
- `ptr<T>`
- `ptr mut<T>`
- `owner ptr<T>`
- `ref<T>`
- `ref mut<T>`
- `site<T>`
- `fn<...>`
- `dyn<T>`
- `str`, `string`, `bytes`

On the current 64-bit backend lane this is `ptr`, 8 bytes, 8-byte alignment,
and qword native memory lowering. Element operations such as `ptrRead<T>`,
`ptrWrite<T>`, `ptrOffset<T>` and `ptrDiff<T>` use the element type descriptor
instead of a separate fallback table.

Compact C-like pointer forms lower through the same descriptor:

- `*p` is `ptrRead<T>(p)` when `p: ptr<T>`;
- `*p = value` is `ptrWrite<T>(p, value)`;
- `p++` advances the pointer value by `sizeof(T)`;
- `*p++` reads through the old pointer and then advances by `sizeof(T)`;
- `&x` is address materialization for the target storage slot.

The raw pointer lane intentionally gives C/C++-grade power: null pointers,
stale pointers and mismatched pointee types are representable hazards at the
surface/backend boundary. Safe diagnostics and runtime checks may still report
them on checked runtime paths, but the backend contract does not reclassify
those hazards as impossible.

## Aggregate And View Types

Fixed arrays lower to LLVM array storage:

```text
array<dword,4> -> [4 x i32], size 16, align 4
bytes<16>      -> [16 x i8], size 16, align 1
```

Fixed array initializers are normalized before backend emission:

- `[0]` is a scalar broadcast initializer for the full declared fixed shape;
- `fill[v,...,v]` is represented as the same broadcast form and materialized
  as per-element stores unless the value is the zero initializer;
- `array<T, D1, D2, ...>` lowers to one flat row-major storage block with
  `D1 * D2 * ...` elements;
- a flat full literal such as `array<int,2,2> = [1, 2, 3, 4]` and a shaped
  literal such as `[[1, 2], [3, 4]]` both lower to the same `[4 x i32]`
  storage;
- fixed `array<Struct,N>` lowers to `[N x %struct.X]`; `[0]` emits a typed
  aggregate zero initializer and field reads/writes use the same struct layout
  metadata as direct `@value.field` access.

Dynamic views are real descriptors, not plain pointers:

- `array<T>`
- `slice<T>`
- `span<T>`
- `memRegion<T>`
- `dinArray<T>`
- `vector<S>` when `S` is a type-contract struct

They lower to `%nC...` descriptor types with pointer and length/capacity fields.
`dinArray<T>` uses three pointer-sized fields; `slice<T>`, `span<T>` and
`memRegion<T>` use two pointer-sized fields. A struct vector lowers as the
`dinArray<S>` shape, so it is 24 bytes and 8-byte aligned on the hosted 64-bit
backend lane. Reads such as `packets[i].tag` and `packets.get(i).tag`, plus
indexed writes such as `packets[i].tag >> T : value`, lower by first computing
`data + i * sizeof(S)` and then selecting the typed struct field using the
computed field index/alignment.

Union-like type strings lower to byte storage sized to the largest member and
aligned to the strictest member. This gives them stable storage bytes while
semantic union behavior remains governed by the front-end type rules.

Named `type:` contracts now also emit a stable type ABI layout descriptor. The
descriptor is `TypeAbiLayoutIR` and records schema, stability status, kind,
qualified name, size, alignment, field count, every field offset/size/alignment
and a deterministic fingerprint. Backend type-contract units, object
`.meta.type.*` sections and MCT LLVM modules consume this same descriptor, so
field layout is not recalculated independently by each backend surface.

The v1 algorithm is source-order C-like natural alignment: align each field
offset to that field alignment, then round the aggregate size to the largest
field alignment. Enums lower to a `dword` tag. Tagged unions lower to a `dword`
tag plus an aligned byte payload sized from the largest variant payload.

## Vector Types

`vector<T>` lowers to a 16-byte vector storage lane where the element type is a
scalar first-class storage type:

- `vector<byte>` -> `<16 x i8>`
- `vector<word>` -> `<8 x i16>`
- `vector<dword>` -> `<4 x i32>`
- `vector<qword>` -> `<2 x i64>`
- `vector<ft32>` -> `<4 x float>`
- `vector<ft64>` -> `<2 x double>`

SIMD instruction selection is handled by the vector lowering path. This type
lowering closure guarantees storage size and alignment for scalar vector
values. `vector<S>` for a type-contract struct `S` is excluded from this SIMD
path and lowers through the dynamic-view descriptor path above.

## Native Bytes

The x86_64 native encoder emits distinct byte sequences for:

- byte zero/sign loads: `movzx` / `movsx`
- word zero/sign loads: `movzx` / `movsx`
- dword zero/sign loads: `mov` / `movsxd`
- qword loads: `mov`
- byte/word/dword/qword stores: width-specific `mov`

These are tested directly in `crates/ncc-ir/src/lib.rs`.

## Boundaries

Closed here:

- official type-to-storage descriptor;
- complete ABI storage matrix for scalar, pointer, fixed aggregate, descriptor,
  vector, union, named type-contract and fallback families;
- LLVM type, size and alignment for scalar, pointer, aggregate, view and vector
  storage;
- stable named type ABI layout descriptors with field offsets, size, alignment
  and fingerprints;
- scalar memory type reuse for pointer element operations;
- native load/store byte coverage for byte/word/dword/qword.
- runtime scalar value validation for signed/unsigned ranges, including
  `uint64`/`qword` values above `i64::MAX`.

Not claimed here:

- optimizer-wide FP arithmetic equivalence beyond the machine FP profile;
- complete arbitrary aggregate ABI passing rules;
- optimizer parity with LLVM/GCC/rustc.
