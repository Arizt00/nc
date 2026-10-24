# Pointer Lowering Evidence

The canonical target/address-space/width contract is
`docs/spec/backend/pointer_multitarget_contract.md`. This file retains the
operation-level and historical evidence behind that contract.

Pointer lowering evidence records how pointer operations are intended to reach backend machine-level behavior without changing [@machine] semantics in this closure. Pointer Model V2 treats `ptr<T>` like a C/C++-grade raw pointer in pointer-capable nC surfaces: `risk` is no longer required for raw pointer reads, writes, address casts, offsets, diffs, frees, reallocs, or heap-backed region views.

## Evidence Items

- `ptrRead<T>` maps to a raw load with width and target endian/alignment requirements; provenance is never mandatory.
- `ptrWrite<T>` maps to a raw store with width and target endian/alignment requirements; ownership and mutability state are never mandatory.
- `ptrOffset<T>` maps to base plus index multiplied by the size of `T`.
- `ptrDiff<T>` reports target-sized `isize` distance in element units without same-allocation, provenance, or alignment gates.
- `addrOf` materializes an address directly and requires no authority.
- `ptrFromAddr<T>` records an explicit bridge from raw address to typed pointer.
- `regionFromPtr<T>` records a typed view over a live heap block: base, end, `sizeof(T)`, length, and heap generation.
- `regionPtrAt<T>` records bounds-checked region addressing and materializes `region.base + index * sizeof(T)`.

This evidence is compiler/backend evidence for normal nC pointer closure. It is not basal lowering and does not alter NBB, machine seal, primitive, or kernel surfaces.

## Physical Formulas

The executable compiler report now records the exact physical intent used by the pointer family:

- `ptrRead<byte>(p)` lowers as an unsigned byte load from `[addr]`, rendered by the native backend as `loadbyte.zero`.
- `ptrWrite<byte>(p, v)` lowers as an 8-bit store to `[addr]`, rendered as `storebyte`.
- `ptrRead<int>(p)` lowers as `load i32 [addr]`.
- `ptrWrite<int>(p, v)` lowers as `store i32 [addr], v`.
- `ptrRead<qword>(p)` lowers as a 64-bit load from `[addr]`, rendered as `loadqword.zero`.
- `ptrWrite<qword>(p, v)` lowers as a 64-bit store to `[addr]`, rendered as `storeqword`.
- `ptrOffset<int>(p, 1)` lowers as `addr + sizeof(int)`.
- Raw offset/read/write operations do not emit allocation, null, lifetime, generation, provenance, or alignment checks. The resulting address may leave the allocation and arithmetic wraps at the target pointer width.
- LLVM raw addressing uses non-`inbounds` GEP and emits no `nsw`/`nuw` promise for pointer offset arithmetic.
- `ptrDiff<T>(pB, pA)` lowers as `(addrB - addrA) / sizeof(T)`.
- `regionFromPtr<int>(p, n)` lowers as `view { base=addr(p), len=n, elem_width=sizeof(int), owner_generation=heap.generation }`.
- `regionPtrAt<int>(r, i)` lowers as `addr = region.base + i * sizeof(int)` with bounds and generation checks.

The hosted fixture `examples/performance/native_fast/memory_ptr_byte_qword_real_bytes.n` is the byte-level smoke for this contract. It runs through `ncc run --native-fast --show-return` and returns 42, and `ncc emit-object --cc nC.sysv64` must show `storebyte`, `loadbyte.zero`, `storeqword`, and `loadqword.zero` in the generated backend text.

The runtime fixtures `examples/pointers/ptr_generic_short_full_runtime_ok.n`, `examples/pointers/ptr_region_view_without_risk_ok.n`, and `examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n` prove the less-verbose surface with `alloc`, `realloc`, `free`, generic pointer calls, `regionFromPtr<T>`, and `regionPtrAt<T>`. The lowering payload keeps the machine base unchanged; it describes normal/high-level nC intent before it reaches the closed machine authority.

## ABI And Layout Boundary

Pointer lowering carries C ABI, real C++ ABI smoke evidence, and ASM bridge metadata. The C lane requires an extern symbol, header contract, calling convention, ownership, nullability, layout, alignment, object/linker, and static or dynamic library contract. The C++ lane is now backed by `ncc compiler ffi-cpp-abi --json --strict`: it compiles native C++17, validates name mangling through `nm`/`c++filt`, executes template/vtable/destructor/exception checks, and links object, static archive, shared library, and executable artifacts. Itanium/MSVC C++ ABI profiles still require explicit symbol, constructor/destructor, ownership, and no-hidden-exception contracts. The ASM bridge records register classes, stack alignment, object format, relocation, symbol visibility, and call-preserved registers.

## Performance Counters

Pointer performance is measurable through the compiler report rather than guessed. Normal raw operations have no pointer-state checks; optional audit findings are computed separately. Reports retain machine intent counts, object metadata, IR/backend output, emitted bytes, and FFI smoke paths.
