# `local:` Surface

`local:` is the compact high-level surface for declarations owned by one
container. `stc:` remains the explicit stable container-state section; it is
not merely a spelling alias.

`local:` is for:

- mutable container-local state;
- immutable local constants;
- local type aliases;
- local record declarations;
- metadata;
- future site-aware local memory declarations.

Example:

```n
localBasicState {
    local:
        total >> int <- 0;
        const Limit >> int = 4;
        type Score: int;
        score >> Score <- 1;
        metadata profile: "local-basic";

    run :: in() -> bool {
        total = @Limit + score;
        return true;
    }
}
```

Mutable values lower to container storage and their initializers join the
container initialization path. Constants are typed and materialized before
function bodies where needed. Aliases are resolved with no runtime storage;
records lower to ABI layouts and object type metadata; metadata remains an
inspectable compiler descriptor and can be removed by release zero-cost rules.

The normal `.n` build retains a source-specific `.nm` Machine entry bridge and
material identity, plus the full high-level body in optimized LLVM and the
native program object. The bridge is not an instruction-for-instruction text
dump of every high-level operation.
