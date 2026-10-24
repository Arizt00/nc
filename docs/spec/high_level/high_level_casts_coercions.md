# High Level Casts And Coercions

Status: policy closed current surface.

The current high-level coercion policy is intentionally conservative:

- exact primitive projections are accepted;
- compatible numeric widening inside the current numeric family is accepted;
- implicit narrowing is rejected;
- `bool` is not coerced from `int`;
- `addr` is not coerced from an `int` variable;
- `ptr<T>` is not coerced from `addr`;
- strict checks use `!>` and produce diagnostics when the checked shape is incompatible.

Example:

```n
@value >> int : 1;
@value !> int : dev(TypeViolation);
```

Full explicit cast syntax is reserved for a later phase. Fase 2 defines the policy boundary and keeps accidental conversions out of positive examples.
