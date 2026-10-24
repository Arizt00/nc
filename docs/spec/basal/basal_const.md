# Basal Const Surface

Basal Fase 5.5 defines `const:` as the official root constant surface for `[@]` files.

`const:` is exclusive to basal. It is not a compatibility spelling for historical `stc:` syntax. A constant is immutable, explicitly typed by width, materializable as deterministic bytes, hashable, visible in BasalIR, visible in the lowering plan, and serializable into NBB0 executable surface records.

Baseical form:

```nb
const:
    machineWidth >> dword : 32;
    label >> str : "root";
```

Allowed width types are `byte`, `word`, `dword`, `qword`, `sbyte`, `sword`, `sdword`, `sqword`, `addr`, `bool`, `char`, `str`, and `bytes<N>`. Integer aliases such as `u32` map to the corresponding official width, but new examples use the official names.

Diagnostics include `E_BASAL_CONST_001`, `E_BASAL_CONST_TYPE_001`, `E_BASAL_CONST_WIDTH_001`, `E_BASAL_CONST_LITERAL_001`, `E_BASAL_CONST_RANGE_001`, `E_BASAL_CONST_MUTATION_001`, `E_BASAL_CONST_ENDIAN_001`, `E_BASAL_CONST_HASH_001`, and `E_BASAL_CONST_SCOPE_001`.
