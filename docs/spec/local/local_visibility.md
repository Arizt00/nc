# local visibility

Local declarations may carry visibility where the surrounding container allows it:

```n
local:
    public exposed: int = 1;
    private scratch: int = 2;
    internal stage: int = 3;
```

Rules:

- visibility is metadata on the local declaration;
- public local types cannot expose private markers;
- visibility does not make `local:` valid in machine, primitive, basal or kernel.

Baseical diagnostic:

- `E_LOCAL_PUBLIC_PRIVATE_TYPE_001`
