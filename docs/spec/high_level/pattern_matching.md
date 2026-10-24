# High Level Pattern Matching

Status: closed for exhaustive ADT and structural matching.

Baseical pattern matching uses `match (@value) { ... }` with `Enum.Variant` arms.

```n
match (@packet) {
    Packet.Data(code, _) => return @code;
    Packet.Drop => return 0;
}
```

Supported:

- Unit variant patterns.
- Payload variant patterns.
- `_` ignored payload bindings.
- Wildcard catch-all arms.
- Boolean guards, including typed calls.
- Typed pattern binding scopes.
- Literal and inclusive/exclusive range patterns.
- Binding-only catch-all patterns.
- Struct and raw-union fields.
- Fixed arrays, nested arrays and fixed-array rest bindings.
- Nested enum payload patterns containing records or arrays.

The compiler rejects missing variants or scalar intervals, repeated/covered arms, invalid fields or payload arity, non-boolean guards, unguarded raw-union field assumptions and `any` leakage.
