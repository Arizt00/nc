# BasalIR Verifier

Status: closed for Basal Fase 3.

The semantic verifier checks:

- root count, identity and spans.
- site kind, size, alignment, permission normalization and owner consistency.
- bind duplicates, local site resolution, invalid kinds, unresolved local references and span preservation.
- cross-table consistency between root, site and bind tables.
- machine references are explicit pending references, never fake resolved references.

The verifier emits `BasalIrVerifierReport` through:

- `ncc basal ir <file.nb> --json`
- `ncc basal verify-ir <file.nb> --json`
- `ncc basal report <file.nb> --json`

The verifier does not invoke machine, does not write `.nbb`, and does not create evidence.
