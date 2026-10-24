# Basal `.nb` Grammar

Basal Phase 1 closes the source grammar for `.nb` files at syntax level. The official root form remains the existing nC form:

```text
RootName[@] {
    root:
        identity: "nC.root.example";
        version: 1;
        authority: sealed;
        binary: optional;
        evidence: optional;
        lowering: deferred;
}
```

The parser also accepts the Phase 1 header form `[@] root RootName {` for migration experiments, but examples keep the existing `RootName[@]` base.

Closed syntax:

| Surface | Status |
|---|---|
| line comments `//` | closed |
| block comments `/* ... */` | closed |
| root declaration | closed |
| section labels | closed |
| key/value entries | closed |
| string, integer, boolean, path literals | closed |
| permission sets with `|` | closed |
| `site:` declarations | syntax-only |
| `bind:` declarations | syntax-only |
| `seal:` requirements | syntax-only |
| `evidence:` requirements | syntax-only |
| `target:` and `meta:` entries | syntax-only |

Phase 1 does not resolve bind targets, validate evidence content, perform lowering, or write `.nbb`.
