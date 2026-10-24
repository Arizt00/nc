# Forbidden Reflection Operations

Fase 12 forbids operations that would turn metadata into authority.

Forbidden calls include:

- `eval`
- `reflectInvoke`
- `setField`
- `typeFromString`
- private/internal field reflection helpers
- machine/basal reflection through high-level commands
- closure captured-value reads

These operations remain deferred or forbidden because Fase 12 closes introspection, not dynamic execution.
