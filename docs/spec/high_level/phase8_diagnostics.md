# Iterator, generator and stream diagnostics

Status: current registry. The filename is retained for historical links.

- `E_HIGH_ITERATOR_PROTOCOL_001`
- `E_HIGH_ITERATOR_PROTOCOL_ARITY_001`
- `E_HIGH_ITERATOR_ITEM_001`
- `E_HIGH_ITERATOR_NEXT_001`
- `E_HIGH_ITERATOR_ADAPTER_CLOSURE_001`
- `E_HIGH_ITERATOR_ADAPTER_TYPE_001`
- `E_HIGH_YIELD_CONTEXT_001`
- `E_HIGH_YIELD_TYPE_001`
- `E_HIGH_COLLECTION_PROTOCOL_001`
- `E_HIGH_COLLECTION_LEN_001`
- `E_HIGH_COLLECTION_EMPTY_001`
- `E_HIGH_COLLECTION_GET_001`
- `E_HIGH_FOR_IN_ITERABLE_001`
- `E_HIGH_FOR_IN_MUTATION_001`
- `E_HIGH_RANGE_001`
- `E_HIGH_RANGE_BOUND_001`
- `E_HIGH_RANGE_STEP_001`
- `E_HIGH_RANGE_STEP_ZERO_001`
- `E_HIGH_STREAM_UNBOUNDED_DEFERRED_001`
- `E_HIGH_ANY_LEAK_001`

The unbounded-stream diagnostic enforces explicit finite or externally driven policy; it does not
mean that typed streams or generators are deferred. Invalid source exits with failure.
