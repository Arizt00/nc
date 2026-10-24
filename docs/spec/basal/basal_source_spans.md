# Basal Source Spans

Every Phase 1 AST node and diagnostic carries a span:

```json
{
  "file": "examples/basal/root_minimal.nb",
  "start": 0,
  "end": 16,
  "line": 1,
  "column": 1
}
```

Spans are byte offsets plus human line/column coordinates. They are used by diagnostics, parser recovery, tooling, later BasalIR lowering, and future seal/evidence hashing.

Closed span tests cover root declarations, site declarations, bind declarations, seal requirements, evidence requirements, and diagnostic locations.
