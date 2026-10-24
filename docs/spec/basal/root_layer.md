# Root Layer

The root layer is entered by `[@]`:

```n
kernelRoot[@] {
    root:
        identity: "nC.root.kernel";
        version: 1;
        authority: sealed;
        binary: required;
        evidence: required;
        lowering: deferred;
}
```

Allowed root sections are `root:`, `stc:`, `type:`, `binary:`, `site:`, `bind:`, `bridge:`, `probe:`, `structure:`, `seal:`, `evidence:`, `fa:`, and `final:`.

Restricted behavior:

| Behavior | Root status |
|---|---|
| direct runtime import | forbidden |
| primitive mutation | forbidden |
| active machine lowering | deferred |
| binary contract reporting | allowed |
| primitive witness reference | allowed when immutable |

The root layer is not runtime, primitive, or machine. It is the authority record for those later derivations.
