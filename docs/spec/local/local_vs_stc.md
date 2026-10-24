# local: vs stc:

`stc:` is the single official high-level container-state section. It is not a
legacy alias and a container cannot declare it twice. Its declarations reach
AST, sema, IR and backend storage.

`local:` is a separate explicit section for scoped declarations, aliases and
metadata that should not enlarge the stable state contract:

- `name >> Type <- value;` declares mutable local storage.
- `const Name >> Type <- value;` declares an immutable local value.
- `type Name: Type;` declares a local alias.
- `type Name { field >> Type; }` declares a local record surface.
- `metadata key: value;` stores inspectable metadata.

Canonical separation:

```n
stc:
    total >> int <- 0;

local:
    scratch >> int <- 0;
```

`state:` is not an alternative high-level spelling. It is reserved for
primitive material and basal `.nb` metadata.
