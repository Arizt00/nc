# Backend LLVM/MCT F2: dynamic arrays and slices

Status: closed for the LLVM/MCT local executable lane.

F2 extends the F1 aggregate work with first-class dynamic view descriptors for
array-like storage. The closed representation is explicit:

- `array<T>` lowers to `%nC.dynamic_array_<T> = type { ptr, i64 }`.
- `slice<T>` lowers to `%nC.slice_<T> = type { ptr, i64 }`.
- `span<T>` lowers to `%nC.span_<T> = type { ptr, i64 }`.
- `memRegion<T>` lowers to `%nC.memregion_<T> = type { ptr, i64 }`.
- `dinArray.*<T>` lowers to `%nC.dinarray_<T> = type { ptr, i64, i64 }`.
- `vector<S>` where `S` is a type-contract struct lowers to the same
  `%nC.dinarray_<%struct.S> = type { ptr, i64, i64 }` descriptor.

The first word is the data pointer. The second word is the element length. For
`dinArray`, the third word is capacity. This keeps the runtime shape simple and
auditable while preserving the nC distinction between fixed arrays, dynamic
arrays, slices/spans, struct vectors and memory regions.

Closed behavior:

- Dynamic array literals allocate real LLVM backing storage.
- Descriptor initialization stores data pointer and length.
- `dinArray.binary<T>` literals also store capacity.
- `@array[index]` lowers through descriptor data pointer plus scaled offset.
- `memRegion<T>` created by `regionFromPtr<T>(ptr, len)` stores pointer and len.
- `readAt/get`, `writeAt/set`, and `.len()` lower through the descriptor.
- Constant out-of-bounds dynamic indexes are permitted by the unsafe backend
  policy and emit `W_MCT_LLVM_DYNAMIC_BOUNDS_UNSAFE_001`.
- `mct emit-llvm --json` reports `dynamic_views` with kind, element type,
  field count, size and alignment.
- `vector<S>` for a struct `S` is reported as `kind: "dinarray"` with
  `source_type: "vector<S>"`, three fields, 24-byte size and 8-byte alignment
  on hosted 64-bit targets.
- Materialized `vector<S>` descriptors store `data`, `len` and `capacity`; the
  `data` pointer addresses a backing buffer whose element stride is the
  computed struct size of `S`.
- Indexed `vector<S>` loads/stores and `push` lower through byte addressing:
  `addr = data + index * sizeof(S)`.
- Field access from dynamic aggregate elements is closed for
  `packets[i].field`, `packets.get(i).field` and indexed writes such as
  `packets[i].field >> T : value`; after the element address is computed, LLVM
  emits a typed struct field GEP using the field offset/alignment from the
  struct layout. See `backend_llvm_dynamic_aggregate_arrays.md`.
- Object and executable smoke are covered with clang when available.

F2 scope boundary:

- Grow/realloc/push/pop ownership for `dinArray` is not part of F2. It is
  closed by F7 in `backend_llvm_dinarray_growth_f7.md`.
- Cross-function ABI guarantees for descriptors on every target.
- Borrow-checking or ownership semantics for views.
- Full LLVM optimizer/pass-manager parity.

F2 does not replace ncc-native authority. It makes the LLVM/MCT adapter capable
of lowering the closed dynamic view subset to real LLVM IR, object files and
hosted executables.
