# Reflection Visibility

Fase 12 metadata obeys the same visibility boundary as the source program.

Rules:

- `public` symbols may be introspected.
- `private` symbols and fields are not exposed outside their owner.
- `internal` metadata is not exposed outside package scope in this phase.
- public API introspection is filtered to exported public surface.

Visibility violations are source errors, not warnings.
