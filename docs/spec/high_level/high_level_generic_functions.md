# High Level Generic Functions

Generic functions use explicit type parameters after the function name:

```n
choose<T>(left: T, right: T) -> T {
    public:
        return @right;
}
```

Resolution rules:

- A function may declare one or more type parameters.
- Duplicate type parameters are rejected.
- Type parameters may appear in parameter and return types.
- Explicit call arity must match the declaration.
- Inference only uses argument positions. A parameter that appears only in the return type requires an explicit type argument.
- Inference conflicts reject the call instead of choosing `any`.

Examples:

```n
@a >> int : choose<int>(1, 2);
@b >> int : choose(3, 4);
```

Rejected:

```n
choose<int, str>(1);
choose(1, "bad");
make<T>() -> T;
```
