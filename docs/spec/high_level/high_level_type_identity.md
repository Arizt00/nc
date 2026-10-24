# High Level Type Identity

Status: closed.

Type identity is nominal for primitive spellings and structural for current compound forms.

Rules:

- `bool` is not `int`.
- `str` is not numeric.
- `null` is its own value type and only projects into nullable or pointer-like targets.
- `addr` is address-shaped and distinct from `int`; numeric literals may initialize an address explicitly, but an `int` variable is not an address.
- `p<T>`/`ptr<T>` is pointer-shaped and distinct from `addr`; an explicit projection/cast from `addr` is accepted without authority and carries raw C-like semantics.
- `array<T,N>` and `T[]` lower through the array/vector family and require homogeneous element types.

Example:

```n
@value >> int : 42;
@label >> str : "phase2";
@accepted >> bool : true;
```

Invalid example:

```n
@accepted >> bool : 1;
```

The invalid projection fails because numeric truthiness is not a high-level boolean conversion.
