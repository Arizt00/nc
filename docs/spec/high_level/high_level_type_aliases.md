# High Level Type Aliases

Status: policy closed current surface.

Fase 2 does not introduce nominal high-level type alias declarations. It closes the current policy:

- pointer and region flow aliases are tracked semantically;
- primitive type alias sections remain primitive-scope and are not imported into high-level;
- future nominal alias declarations must not erase diagnostics or widen into `any`;
- aliases must preserve source spans and diagnostics when introduced in a later phase.

Current supported example:

```n
@root >> ptr<byte> | null : null;
@mirror >> ptr<byte> | null : @root;
```

That is a flow alias, not a new named type.
