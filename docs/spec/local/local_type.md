# Local Types

`local:` supports zero-cost aliases and structural records scoped to one
container. The compact `>>` separator is official; `:` remains accepted for
source compatibility.

```n
local:
    type Score: int;
    type Pair {
        left >> Score;
        right >> qword;
    }
    current >> Score = 1;
    pair >> Pair;
```

Rules:

- declarations are visible throughout the container regardless of section
  order;
- aliases are resolved before sema, ABI layout, LLVM and Machine backend
  lowering and do not allocate storage;
- compact record fields without a visibility block are public;
- explicit `public:`, `private:`, `internal:` and `strictly:` contracts remain
  available where the type surface admits them;
- recursive aliases and records containing themselves by value are rejected;
- recursive indirection through `p<T>`, `ptr<T>`, `ref<T>`, collections or
  slices is admitted because the record has finite size;
- duplicate names, unknown types and `Any` leaks are rejected;
- local records receive the same concrete size, alignment, offsets, LLVM named
  struct and object type metadata as equivalent `type:` records.

Diagnostics:

- `E_LOCAL_TYPE_DUP_001`
- `E_LOCAL_TYPE_UNKNOWN_001`
- `E_LOCAL_TYPE_RECURSIVE_001`
- `E_LOCAL_TYPE_ANY_LEAK_001`
- `E_LOCAL_TYPE_FIELD_001`
