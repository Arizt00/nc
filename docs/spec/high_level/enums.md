# High Level Enums

Status: closed in Fase 6.

`enum:` defines a closed ADT family inside high-level roots. Variants may be unit variants or payload variants. Basic generic enum constructors are supported.

```n
enum:
    Shape {
        Circle(radius: int);
        Rect(width: int, height: int);
    }
```

Rules:

- Enum names are unique within the program.
- Variant names are unique within an enum.
- Payload arity and payload types are checked at constructor calls.
- Generic payload parameters may be inferred from constructor arguments.
- Bounds on enum type parameters remain deferred and emit `E_HIGH_ENUM_PARSE_001`.

Constructors use qualified call form:

```n
@shape >> Shape : Shape.Rect(4, 5);
@maybe >> Maybe<int> : Maybe.Some(9);
```
