# ptr/region roundtrip

`regionFromPtr<T>` and `regionPtrAt<T>` close the runtime path for turning a heap pointer into a typed region view and recovering a typed pointer from that view.

```n
use stoud.memory { ptr, region, heap };
use stoud.collections { result };

@p !> ptr<int> : memory.heap.manager.alloc(32);
ptrWrite<int>(@p, 7);
@region >> memRegion<int> : regionFromPtr<int>(@p, 4);
@q >> ptr<int> : regionPtrAt<int>(@region, 0);
```

The lowering intent is physical:

```text
regionFromPtr<int>(p,n) = view { base=addr(p), len=n, elem_width=sizeof(int), owner_generation=heap.generation }
regionPtrAt<int>(r,i)  = region.base + i * sizeof(int)
```

`ptrRead<T>` returns `Result<T, PtrError>` at the runtime API boundary. Existing direct assignments remain compatible: assigning `ptrRead<int>(p)` into `int` unwraps `Ok(int)` and fails on `Err(PtrError)`.

Baseical diagnostics:

- `E_REGION_BOUNDS_001` for zero length, out-of-range region views, and bad indexes.
- `E_PTR_STALE_SITE_001` for freed/reallocated backing heap blocks.
- `E_PTR_TYPE_002` for asking `regionPtrAt<U>` from `memRegion<T>` when `U != T`.
- `heapRegionView` no longer requires `risk` or `allow` under Pointer Model V2. A declared risk block is still recorded as evidence, but the raw pointer operation is C/C++-grade and backend-lowerable by default.

Executable fixtures:

- `examples/pointers/ptr_runtime_region_roundtrip_e2e_ok.n`
- `examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n`
- `examples/pointers/ptr_region_view_without_risk_ok.n`
- `examples/pointers/negative/ptr_region_oob_fail.n`
- `examples/pointers/negative/ptr_region_stale_after_free_fail.n`
- `examples/pointers/negative/ptr_region_type_mismatch_fail.n`
