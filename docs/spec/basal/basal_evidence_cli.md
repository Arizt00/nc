# Basal Evidence CLI

Phase 6 adds:

```text
ncc basal evidence file.nb --json
ncc basal integrate-evidence file.nb --out file.nbb
ncc basal evidence-report file.nbb --json
```

`integrate-evidence` runs the basal pipeline through lowering, emits a machine smoke artifact from the lowering plan, records machine validation hashes, and writes an NBB0 with integrated evidence.

