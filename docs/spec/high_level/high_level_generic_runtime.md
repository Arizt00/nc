# High Level Generic Runtime

Runtime generic families remain explicit imports. Phase 3 verifies that high-level generic calls do not leak `any` through runtime surfaces.

Example:

```n
use alloc.collections { list };

@items >> list<int> : listCreate<int>();
listAdd(@items, 9);
@value >> int : listGet(@items, 0);
```

Closed runtime families:

- List creation and mutation.
- List element access.
- Region view via `listAsRegion`.
- Generic element propagation from container to result.

Runtime mismatches are real diagnostics, for example `listAdd(@items, "bad")` on `list<int>` fails with runtime generic mismatch diagnostics.
