# High Level Fase 13 Scope

Fase 13 closes the compile-time evaluation model for high-level source.

Closed:

- `const:` pure declarations;
- `const fn:` pure compile-time functions;
- `comptime:` pure compile-time declarations;
- `comptime unsafe:` effect blocks with explicit `allow`;
- `comptime unsafe noLimit:` audited unbounded authority;
- official diagnostics;
- CLI manifests and file reports.

Out of scope:

- macro expansion;
- syntax generation;
- runtime execution model changes;
- primitive/machine/basal semantic changes;
- high-level feature phases after the compile-time evaluation model.
