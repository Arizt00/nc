# Runtime Memory/Buffer/Heap/Region

> The canonical raw contract is
> [Manual raw heap and virtual memory](MANUAL_RAW_HEAP_VIRTUAL_MEMORY.md).
> This document describes optional tracked buffer and `memRegion` views.

## Cobertura
- buffer create/read/write/slice
- heap alloc/free/check/stats
- memRegion bounds/owner/readonly
- ptr/addr/site contracts

## Reglas
- bounds apply to `buffer` and `memRegion`, not to `p<T>`;
- owner tracking exists only in explicitly tracked runtime objects;
- stale/alias diagnostics are optional audit output and never change raw code;
- raw pointers, heap calls and virtual-memory calls have no hidden tracking.
