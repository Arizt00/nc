# ncc Native Memory Ops Lowering

Status: closed for the `native-fast` scalar backend.

This document records the native memory lowering surface added after native register/stack and control-flow lowering. The backend now materializes real x86-64 bytes for pointer and memory operations instead of routing these cases through interpreter/runtime fallback when the expression is scalar and lowerable.

## Supported Operations

- `addrOf(x)` lowers to a stack address with `lea`.
- `ptrRead<T>(p)` lowers to a typed load from `[addr]`.
- `ptrWrite<T>(p, v)` lowers to a typed store to `[addr]`.
- `volatileRead<T>(p)` lowers to a typed volatile-marked load from `[addr]`.
- `volatileWrite<T>(p, v)` lowers to a typed volatile-marked store to `[addr]`.
- `ptrOffset<T>(p, n)` lowers to `addr + n * sizeof(T)`.
- `ptrDiff<T>(b, a)` lowers to `(addrB - addrA) / sizeof(T)`.
- `regionFromPtr<T>(p, n)` preserves a typed region view over an existing pointer.
- `regionPtrAt<T>(region, i)` lowers to `region + i * sizeof(T)`.
- `array<T>` literals allocate contiguous stack storage in native-fast functions.
- `@array[i]` lowers to a typed indexed load.
- `@array[i] >> T : value` lowers to a typed indexed store.
- Pointer method sugar `.read()`, `.write(value)`, `.offset(n)`, `.diff(other)`, `.regionView(n)` lowers to the same native forms.

## Widths And Extensions

The backend chooses load/store width from the element type:

| Type family | Width | Load extension |
| --- | ---: | --- |
| `byte`, `u8`, `bool`, `char` | 1 | zero |
| `sbyte`, `i8` | 1 | sign |
| `word`, `u16` | 2 | zero |
| `sword`, `i16` | 2 | sign |
| `dword`, `u32` | 4 | zero |
| `sdword`, `i32`, `int` | 4 | sign |
| `qword`, `u64`, `addr` | 8 | zero |
| `sqword`, `i64` | 8 | sign |

Native encodings include `movzx`, `movsx`, `movsxd`, `mov r/m`, `mov r`, and `lea`. Stores use byte, word (`0x66` prefix), dword, and qword forms. The backend keeps a `volatile` marker in IR/rendering; on x86-64 the scalar load/store byte sequence is the same, but the operation remains distinct for future optimizer barriers and diagnostics.

## Alignment, Offsets, And Regions

The native-fast backend computes element sizes from the typed pointer/region/array element. Offset and region addressing are scaled by that size. Array literals are lowered into stack storage with an alignment floor matching the element size and native word alignment. Region views preserve element size and length metadata for later addressing and reports.

## Evidence

Positive fixtures:

- `examples/performance/native_fast/memory_ptr_read_write_widths.n`
- `examples/performance/native_fast/memory_method_read_write.n`
- `examples/performance/native_fast/memory_array_index_load_store.n`
- `examples/performance/native_fast/memory_memregion_ptr_at.n`
- `examples/performance/native_fast/memory_ptr_offset_diff.n`
- `examples/performance/native_fast/memory_signed_unsigned_extension.n`
- `examples/performance/native_fast/memory_volatile_ops.n`

Tests:

- `ncc_run_native_fast_executes_memory_ops_fixtures`
- `ncc_optimize_json_reports_native_memory_ops_bytes`
- `encode_backend_materializes_native_memory_width_loads_and_stores`
- `render_backend_marks_native_memory_volatile_boundary`

## Non-Goals

This closure does not change official machine, basal, high-level, primitive, or runtime semantics. It also does not make narrow `ptrRead<byte>` runtime assignment syntax silently unwrap a `Result`; narrow runtime boundary behavior remains governed by the pointer runtime API. The native backend support is present and tested at the encoding layer.
