# Runtime Memory / Region / Heap

Status: closed for the normal/high-level/runtime execution lane.

Official surfaces:

- `nC.runtime.memory`
- `nC.runtime.region`
- `nC.runtime.heap`
- `nC.runtime.ptr`
- `nC.runtime.addr`

`alloc(n)` and `memory.heap.manager.alloc(n)` request exactly `n` raw bytes. Pointee type never scales the request. `free`, double-free, stale aliases, use-after-free, null pointers, provenance, lineage, ownership, misalignment, and overflow do not gate normal compilation. They may be recorded by optional audit/telemetry, while native execution follows C-like undefined behavior.

Raw `ptr<T>` offset, difference, read, write, indexing, and increment use machine-width wrapping address arithmetic and may leave an allocation. They do not acquire a bounds check merely because heap provenance is known. Arrays, vectors, slices, buffers, sites, and `memRegion<T>` remain bounded surfaces and reject contract violations.

Parser/AST preserve typed pointer and heap calls; sema enforces type formation but does not impose raw-pointer state gates; IR lowers width/stride/load/store intent; backend legalization uses the target's address arithmetic and load/store ISA families without hidden pointer checks. Bounded-view contracts remain explicit and separate.
