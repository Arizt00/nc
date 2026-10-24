# High Level Fase 11 Scope

Status: historical phase scope, closed and superseded by the current closure/coroutine contract.

Fase 11 closes closures, lambdas, and captures over the high-level language already closed through
Fase 10.

Closed:

- lambda grammar and parser support;
- closure AST nodes;
- explicit function types;
- capture modes: value, ref, mut, move;
- local closure calls;
- closure as local value and function argument;
- owned closure return;
- basic iterator closure adapters;
- capture diagnostics;
- effect diagnostics;
- high-level CLI manifests: `closures`, `lambdas`, `captures`.

Not included when Phase 11 originally closed:

- Fase 12 reflection/introspection;
- compile-time evaluation;
- higher-kinded types;
- specialization;
- macros;
- async closures;
- generator closures.

Both closure forms are now implemented as typed native state machines. This list records chronology;
it is not the current capability boundary.
