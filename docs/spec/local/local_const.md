# local const

`local const` declares immutable state owned by a container.

Syntax:

```n
local:
    const Base >> int <- 40;
    const Step >> int <- 2;
```

Rules:

- a local const must have an explicit type;
- it must have an initializer;
- the initializer is checked by the compile-time expression validator;
- assigning to a local const is an error;
- local consts may be referenced by functions and other local initializers.

Baseical diagnostic:

- `E_LOCAL_CONST_EVAL_001`
- `E_LOCAL_CONST_MUTATION_001`

Example failure:

```n
local:
    const Limit >> int <- 4;

run :: in() -> bool {
    Limit = 5;
    return true;
}
```
