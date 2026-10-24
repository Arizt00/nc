# Basal `.nb` AST

Phase 1 exposes a JSON AST through:

```text
ncc basal ast <file.nb> --json
ncc basal parse <file.nb> --json
```

Required AST node kinds:

| Node | Status |
|---|---|
| `BasalSourceFile` | closed |
| `BasalRootDecl` | closed |
| `BasalRootSection` | closed |
| `BasalTargetSection` | closed |
| `BasalSiteSection` | closed |
| `BasalSiteDecl` | syntax-only |
| `BasalBindSection` | closed |
| `BasalBindDecl` | syntax-only |
| `BasalSealSection` | closed |
| `BasalSealRequirement` | syntax-only |
| `BasalEvidenceSection` | closed |
| `BasalEvidenceRequirement` | syntax-only |
| `BasalMetaSection` | closed |
| `BasalKeyValue` | closed |
| `BasalLiteral` | closed |
| `BasalPath` | represented in bind/evidence/seal nodes |

The AST is source-level only. BasalIR and RootIR remain `not_started`.
