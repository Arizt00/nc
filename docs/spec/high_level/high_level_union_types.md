# High Level Union Types

Status: closed current surface.

Union syntax:

```n
value: int | str;
@value >> int | str : 1;
@value >> int | str : "ok";
```

Rules:

- a value may project into a union if it is compatible with at least one member;
- a union source may project into a target only when every non-null member is compatible;
- operations on unions without narrowing are rejected;
- pattern matching and exhaustive destructuring are reserved for later phases.

Invalid examples:

```n
@value >> int | str : true;
@out >> int : @value + 1;
```
