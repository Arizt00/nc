# Basal Seal CLI

Phase 6 adds:

```text
ncc basal seal file.nbb --json
ncc basal verify-seal file.nbb --json
ncc basal seal-report file.nbb --json
```

`seal` reports the final seal already embedded by `integrate-evidence`. `verify-seal` rejects pending evidence, missing evidence records, hash mismatches, and structurally modified NBB0 bytes.

