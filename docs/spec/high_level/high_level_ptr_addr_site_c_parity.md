# High-Level Pointer C/C++ Parity

High-level nC exposes pointer operations with the same practical reach expected from C and C++: raw addresses, typed pointers, pointer-like regions, heap allocation, explicit casts, FFI boundary contracts, pointer arithmetic concepts, and memory ownership transfer. The difference is that nC keeps the contract visible.

## Parity Guarantees

- Raw addresses are represented by `addr`.
- Typed pointers are represented by `ptr<T>` and `ptr mut<T>`.
- Nullable pointers are explicit through union with `null`.
- Heap state tracks live/dead/stale blocks.
- `realloc` invalidates the old generation.
- Aliases propagate invalidation when the owner is freed or reallocated.
- `memRegion<T>` carries a known element type and length.
- FFI pointers require ABI, layout, ownership, side effects, and nullability contracts.

## nC Advantages

nC can express C/C++ level pointer power while retaining diagnostics for lifetime, type, provenance, and bounded abstractions. Raw pointer range behavior remains under programmer control; an optional risk declaration can add audit evidence.

Primary diagnostics include null dereference, out-of-bounds region access, stale site usage, mutable alias conflicts, alignment violations, provenance violations, and FFI contract gaps. Raw `ptr<T>` allocation-range access itself is not diagnosed.

## Explicit Raw Lane

The raw lane is the normal path for intentionally dangerous pointer work. Optional risk metadata does not downgrade diagnostics globally; it marks an evidence boundary so compiler, runtime, and release reports can distinguish declared intent.
