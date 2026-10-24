# High Level Fase 14 scope

Fase 14 closes macros and compile-time code generation for the high-level language.

In scope:

- `macro:` section parsing and AST.
- macro declarations and invocations.
- declarative, template, attribute, derive, and unsafe macro families.
- typed macro parameters.
- generated AST records.
- macro expansion validation.
- hygiene and capture diagnostics.
- symbol collision diagnostics.
- source maps.
- expansion reports.
- const-eval bridge to the closed Fase 13 evaluator.
- comptime-unsafe bridge to the closed Fase 13 authority model.
- noLimit as an explicit unsafe audited macro authority.
- CLI commands for macro manifest, expansion, report, and hygiene.

Out of scope:

- Fase 15+.
- runtime macro execution.
- new runtime semantics.
- new machine or basal semantics.
- high-level features unrelated to macro expansion.

The phase closes only when positive examples pass, negative examples fail with macro diagnostics, doctor reports Fase 14 closed, and validation keeps Fases 1-13 green.
