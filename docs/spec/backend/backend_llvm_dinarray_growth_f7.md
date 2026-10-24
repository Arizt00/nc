# Backend LLVM/MCT F7: dinArray growth API

Status: closed for the LLVM/MCT local executable lane.

F7 closes the previous dynamic-array growth gap without declaring full LLVM
parity. The closed surface is the `dinArray` method family used by nC source and
lowered by the MCT LLVM adapter:

- `.len()`
- `.capacity()`
- `.push(value)`
- `.pop()`
- `.realloc(capacity)`
- `.reserve(capacity)` as the same lowering contract as `.realloc(capacity)`
- `.clear()`

The descriptor remains the F2 shape:

```llvm
%nC.dinarray_<T> = type { ptr, i64, i64 }
```

`vector<S>` for a type-contract struct `S` is a source-level alias into this
material contract, not a SIMD vector. It lowers as a `dinArray<S>` descriptor
whose element LLVM type is the concrete `%struct.<container>_<S>` layout.

The fields are data pointer, length and capacity. The hosted cross-target ABI
contract for `nC.sysv64`, `nC.win64` and `nC.aapcs64` is:

| Field | LLVM type | Offset | Meaning |
| --- | --- | ---: | --- |
| `data` | `ptr` | 0 | Runtime heap backing pointer |
| `len` | `i64` | 8 | Live element count |
| `capacity` | `i64` | 16 | Allocated element capacity |

The descriptor size is 24 bytes and the descriptor alignment is 8 bytes for the
hosted 64-bit target family. Literal-backed `dinArray` values now allocate
backing storage with spare capacity instead of using the literal length as hard
capacity. The initial capacity is deterministic:

```text
max(8, next_power_of_two(max(1, literal_len)))
```

Lowering contract:

- `.len()` loads descriptor field 1.
- `.capacity()` loads descriptor field 2.
- `.push(value)` checks length against capacity, grows when necessary, stores
  the element at the old length and increments descriptor length.
- `.pop()` returns the last element and decrements descriptor length. Empty pop
  returns the zero value of the element type.
- `dinArray` literals allocate backing storage with hosted runtime `malloc`.
- `vector<S>` literals and materialized values use the same backing-buffer
  policy as `dinArray<S>` once the source type is recognized as a struct vector.
- For `vector<S>`, allocation and pointer arithmetic use the computed
  `sizeof(S)` and `alignof(S)` from the struct layout, not pointer-sized
  fallback storage. An empty literal still materializes a descriptor with
  `len = 0` and initial capacity `8`.
- `push`, `get`/`readAt` and `set`/`writeAt` compute element addresses as
  `data + index * sizeof(S)` for `vector<S>` and use `alignof(S)` for whole
  struct loads/stores.
- `packets[i].field`, `packets.get(i).field` reads and indexed field writes
  such as `packets[i].field >> T : value` are closed in the dynamic aggregate
  arrays phase. They reuse the same element-address stride and then apply a
  typed struct-field GEP. See `backend_llvm_dynamic_aggregate_arrays.md`.
- `.realloc(capacity)` / `.reserve(capacity)` call hosted runtime `realloc`,
  preserve the payload through the runtime heap contract, truncate length when
  the new capacity is smaller, and update data pointer, length and capacity.
- `.clear()` stores zero in the descriptor length and preserves the allocated
  data pointer and capacity for reuse. Runtime built-ins whose receiver adds an
  implicit argument are selected by compatible arity, so a zero-argument user
  function also named `clear` cannot shadow `collection.clear()`.

The emitted LLVM IR declares `malloc`, `realloc` and `free` when a `dinArray`
descriptor is present. `free` is part of the ownership boundary even when a
future destructor/ABI pass decides where to emit final release calls.

Truth boundary:

- This is real LLVM IR lowering, not a fake runtime placeholder.
- The storage model is heap-backed through the hosted runtime allocator.
- Struct-vector growth moves/copies the contiguous element bytes; it does not
  imply an implicit deep clone of nested pointer fields.
- The hosted descriptor ABI is closed for `nC.sysv64`, `nC.win64` and
  `nC.aapcs64`.
- Portable/WASI allocator import ABI remains outside F7.
- Destructor insertion and cross-target finalization policy remain outside F7.
- Full LLVM optimizer/pass-manager parity remains outside F7.
- Arbitrary LLVM language parity remains outside F7.

Primary fixture:

- `examples/backend/mct_llvm_dinarray_growth.n`
- `examples/backend/mct_llvm_vector_struct_access.n` for vector-of-struct field
  read/write access.
- `examples/10_mct/collection_clear_name_collision.n` for `.clear()` descriptor
  mutation and user-function name collision.

Expected smoke:

```sh
ncc check examples/backend/mct_llvm_dinarray_growth.n
ncc mct emit-llvm examples/backend/mct_llvm_dinarray_growth.n --out target/tmp/mct-llvm/dinarray-growth.ll --cc nC.sysv64 --json
ncc mct llvm-exe-smoke examples/backend/mct_llvm_dinarray_growth.n --out-dir target/tmp/mct-llvm/dinarray-growth --json --cc nC.sysv64
```
