# local site and memory notes

`local:` is prepared for site-aware local memory but this phase does not redesign pointer semantics.

Supported now:

```n
local:
    maybePtr: ptr<int> | null = null;
    region: memRegion<int>;
```

Future pointer closure will define the full model for:

- `addr`;
- `ptr<T>`;
- `ptr mut<T>`;
- `ref<T>`;
- `ref mut<T>`;
- `site<T>`;
- `memRegion<T>`;
- provenance;
- aliasing;
- bounds;
- alignment;
- raw hazards.

This phase only makes `local:` able to hold those declared types where existing nC type rules already support them.
