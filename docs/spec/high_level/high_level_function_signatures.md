# High Level Function Signatures

Status: closed.

Function signature checks cover:

- arity;
- parameter type compatibility;
- return type compatibility;
- same-file function pointer surface;
- FFI signature parsing where already implemented.

Example:

```n
add(x: int, y: int) -> int {
    public:
        return @x + @y;
}

@total >> int : add(1, 2);
```

Invalid example:

```n
@total >> int : add(1, "bad");
```

Complete generics, traits, interface dispatch, closures, and higher-order inference are not started in Fase 2.
