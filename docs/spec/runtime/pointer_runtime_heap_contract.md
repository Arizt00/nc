# Runtime Pointer Heap Contract

The native contract treats `p<T>`/`ptr<T>` as a raw address. No `risk` block is required to read, write, offset, diff, cast, free, or realloc. Ownership, lifetime, bounds, generation, and provenance are not part of the value or normal generated code.

Raw arithmetic and access are deliberately not range-gated. Null, missing provenance, stale generation, use-after-free, double-free, misalignment, aliasing, and overflow compile. Their execution has C-like undefined behavior and no deterministic result guarantee. Type formation errors remain diagnostics. `array`, `vector`, slice, buffer, and `memRegion<T>` retain separate bounds contracts.

The runtime may retain allocation events as telemetry, and `ncc compiler pointer-audit` may analyze them statically. This metadata is optional evidence: it does not alter native lowering or inject checks.

## Operations

`memory.heap.manager.alloc(size)` returns an `addr` for a live block. The size must be positive and fit the target address space. `size` is always raw bytes: assigning the result to `ptr<int>`, `ptr<byte>`, `ptr<Packet>` or any other typed pointer does not multiply the allocation by `sizeof(T)`. Typed pointer stride applies later to pointer arithmetic and dereference lowering, for example `p + i` uses `i * sizeof(T)`.

The allocator reserves the requested byte range immediately, before any binder receives the pointer. Reservations are aligned and non-overlapping even for nested or temporary allocation expressions. Address-space exhaustion is failure-atomic and reports `E_HEAP_ADDRESS_OVERFLOW_001`; a failed `realloc` leaves its old generation live.

`memory.heap.manager.free(addr)` lowers to the allocator release operation. A second free compiles and its execution is undefined.

`memory.heap.manager.realloc(addr, size)` may move the block. Using the old address or aliases after a successful move compiles and is undefined.

`memory.heap.manager.zero(addr, size)` writes the requested raw byte range. Invalid addresses or sizes compile and have undefined behavior in native execution.

## Diagnostics

The optional pointer auditor may report:

- `E_PTR_UAF_001`
- `E_PTR_DOUBLE_FREE_001`
- `E_PTR_STALE_001`
- `E_PTR_LEAK_001`
- `E_PTR_BOUNDS_001`
- `E_PTR_ALIAS_001`
- `E_PTR_STALE_SITE_001`
- `E_HEAP_SIZE_RANGE_001`
- `E_HEAP_ADDRESS_OVERFLOW_001`

These are not normal compilation gates. The CLI exposes the runtime metadata through `ncc runtime doctor --json` and the opt-in analysis through `ncc compiler pointer-audit <file.n>`.

## Heap-backed region views

`regionFromPtr<T>(p, elements)` creates a typed `memRegion<T>` view over an existing live heap block. It is not a copy and it is not a new owner. The runtime records:

- `base = addr(p)`;
- `end = base + elements * sizeof(T)`;
- `elem_type = T`;
- backing heap base/root address;
- backing heap generation;
- logical element length.

The view fails with `E_REGION_BOUNDS_001` if `elements == 0`, if the pointer is not heap-backed, or if the requested byte range leaves the owning allocation. It fails with `E_PTR_STALE_SITE_001` if the block has been freed or reallocated before the view is used.

`regionPtrAt<T>(region, index)` materializes an alias pointer using:

```text
addr = region.base + index * sizeof(T)
```

The operation checks index bounds, type equality between `T` and the region element type, and the original heap generation. A pointer recovered from a stale region is stale as well. This makes the roundtrip `ptr<T> -> memRegion<T> -> ptr<T>` material, but still auditable. No `risk` declaration is required for the view itself.
