# nC Runtime Import: nc.runtime.memory

## 1. Purpose
`nc.runtime.memory` is the registered runtime import surface for the `memory` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.memory { map };
```

## 3. Aliases
`runtime.memory`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `sizeOf` | `sizeOf(any) -> any` | `any` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `alignOf` | `alignOf(any) -> any` | `any` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `memoryCopy` | `memoryCopy(any, any, any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `memoryMove` | `memoryMove(any, any, any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `memoryZero` | `memoryZero(any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `memoryCompare` | `memoryCompare(any, any, any) -> int` | `int` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arrayCopy` | `arrayCopy(any, any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arrayMove` | `arrayMove(any, int, any, int, int) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arraySwap` | `arraySwap(any, int, int) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arrayFill` | `arrayFill(any, any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arrayZero` | `arrayZero(any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |
| `arrayReverse` | `arrayReverse(any) -> bool` | `bool` | `runtime.memory.sizeof, runtime.memory.alignof, runtime.memory.copy, runtime.memory.zero, runtime.memory.compare` | `imports.official-runtime-required, memory.alias-tracking.required, memory.bounds.required, memory.null-refinement.required, memory.ptr-origin.required, runtime.no-silent-fallback` | `E_RUNTIME_MEMORY_OVERFLOW_001, E_RUNTIME_MEMORY_COPY_ERROR_001, E_RUNTIME_MEMORY_ZERO_ERROR_001, E_RUNTIME_MEMORY_COMPARE_ERROR_001, E_RUNTIME_HEAP_ERROR_001, E_RUNTIME_HEAP_EXHAUSTED_001, E_RUNTIME_HEAP_DOUBLE_FREE_001, E_RUNTIME_HEAP_USE_AFTER_FREE_001, E_RUNTIME_STACK_ERROR_001, E_RUNTIME_STACK_OVERFLOW_001, E_RUNTIME_STACK_ESCAPE_001, E_RUNTIME_REGION_ERROR_001, E_RUNTIME_REGION_CREATE_ERROR_001, E_RUNTIME_REGION_INVALID_001, E_RUNTIME_REGION_OUT_OF_BOUNDS_001, E_RUNTIME_REGION_READONLY_001, E_RUNTIME_REGION_WRITEONLY_001, E_RUNTIME_REGION_FREED_001, E_RUNTIME_REGION_ALIAS_STALE_001, E_RUNTIME_BUFFER_ERROR_001, E_RUNTIME_BUFFER_OVERFLOW_001, E_RUNTIME_BUFFER_UNDERFLOW_001, E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001, E_RUNTIME_BUFFER_INVALID_001, E_RUNTIME_PTR_ERROR_001, E_RUNTIME_PTR_NULL_001, E_RUNTIME_PTR_ORIGIN_MISSING_001, E_RUNTIME_PTR_DANGLING_001, E_RUNTIME_ADDR_ERROR_001, E_RUNTIME_ADDR_UNMATERIALIZED_001, E_RUNTIME_ADDR_REDACTED_001, E_RUNTIME_ADDR_AUTHORITY_MISSING_001, E_RUNTIME_ENDIAN_ERROR_001, E_RUNTIME_LAYOUT_ERROR_001` | official export |

## 6. Type contracts
Function signatures are the active type contract surface for this import. Generic exports keep their declared generic marker in the function name.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive operations are governed by the required capabilities and policies listed below. Out-of-range or invalid state must surface a official runtime diagnostic.

This rule applies to bounded views and size-bearing copy/zero operations. Raw `ptr<T>` arithmetic/read/write is intentionally allocation-range unchecked and remains subject to null, stale-generation, provenance, type, and alignment contracts.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `runtime.memory.sizeof`
- `runtime.memory.alignof`
- `runtime.memory.copy`
- `runtime.memory.zero`
- `runtime.memory.compare`

## 12. Policies
- `imports.official-runtime-required`
- `memory.alias-tracking.required`
- `memory.bounds.required`
- `memory.null-refinement.required`
- `memory.ptr-origin.required`
- `runtime.no-silent-fallback`

## 13. Errors
- `E_RUNTIME_MEMORY_OVERFLOW_001` (RuntimeMemoryOverflow)
- `E_RUNTIME_MEMORY_COPY_ERROR_001` (RuntimeMemoryCopyError)
- `E_RUNTIME_MEMORY_ZERO_ERROR_001` (RuntimeMemoryZeroError)
- `E_RUNTIME_MEMORY_COMPARE_ERROR_001` (RuntimeMemoryCompareError)
- `E_RUNTIME_HEAP_ERROR_001` (RuntimeHeapError)
- `E_RUNTIME_HEAP_EXHAUSTED_001` (RuntimeHeapExhausted)
- `E_RUNTIME_HEAP_DOUBLE_FREE_001` (RuntimeHeapDoubleFree)
- `E_RUNTIME_HEAP_USE_AFTER_FREE_001` (RuntimeHeapUseAfterFree)
- `E_RUNTIME_STACK_ERROR_001` (RuntimeStackError)
- `E_RUNTIME_STACK_OVERFLOW_001` (RuntimeStackOverflow)
- `E_RUNTIME_STACK_ESCAPE_001` (RuntimeStackEscape)
- `E_RUNTIME_REGION_ERROR_001` (RuntimeRegionError)
- `E_RUNTIME_REGION_CREATE_ERROR_001` (RuntimeRegionCreateError)
- `E_RUNTIME_REGION_INVALID_001` (RuntimeRegionInvalid)
- `E_RUNTIME_REGION_OUT_OF_BOUNDS_001` (RuntimeRegionOutOfBounds)
- `E_RUNTIME_REGION_READONLY_001` (RuntimeRegionReadonly)
- `E_RUNTIME_REGION_WRITEONLY_001` (RuntimeRegionWriteonly)
- `E_RUNTIME_REGION_FREED_001` (RuntimeRegionFreed)
- `E_RUNTIME_REGION_ALIAS_STALE_001` (RuntimeRegionAliasStale)
- `E_RUNTIME_BUFFER_ERROR_001` (RuntimeBufferError)
- `E_RUNTIME_BUFFER_OVERFLOW_001` (RuntimeBufferOverflow)
- `E_RUNTIME_BUFFER_UNDERFLOW_001` (RuntimeBufferUnderflow)
- `E_RUNTIME_BUFFER_OUT_OF_BOUNDS_001` (RuntimeBufferOutOfBounds)
- `E_RUNTIME_BUFFER_INVALID_001` (RuntimeBufferInvalid)
- `E_RUNTIME_PTR_ERROR_001` (RuntimePtrError)
- `E_RUNTIME_PTR_NULL_001` (RuntimePtrNull)
- `E_RUNTIME_PTR_ORIGIN_MISSING_001` (RuntimePtrOriginMissing)
- `E_RUNTIME_PTR_DANGLING_001` (RuntimePtrDangling)
- `E_RUNTIME_ADDR_ERROR_001` (RuntimeAddrError)
- `E_RUNTIME_ADDR_UNMATERIALIZED_001` (RuntimeAddrUnmaterialized)
- `E_RUNTIME_ADDR_REDACTED_001` (RuntimeAddrRedacted)
- `E_RUNTIME_ADDR_AUTHORITY_MISSING_001` (RuntimeAddrAuthorityMissing)
- `E_RUNTIME_ENDIAN_ERROR_001` (RuntimeEndianError)
- `E_RUNTIME_LAYOUT_ERROR_001` (RuntimeLayoutError)

## 14. Positive examples
- `examples/runtime/families/memory_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/memory.json` and `reports/runtime_closure/runtime_reports/memory.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
This import does not make runtime calls available without `use`, does not bypass capability/policy checks, and does not cross into `[@machine]` or `primitive[@]`.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.memory --json`
- check command: `ncc runtime check examples/runtime/families/memory_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/memory_basic.n --json`
- evidence hash: `1b9c86532effc8a3edda3391aaa156a8761457ed41b88b0ce4416e02c3882293`
