# High Level Fase 12: Limited Reflection / Safe Introspection

Fase 12 closes a limited, read-only metadata surface for high-level code. It allows code and tools to inspect visible type, symbol, module, package, API, trait, enum, effect, and closure metadata without changing program behavior.

The closed surface is intentionally narrow:

- `typeInfo<T>()`, `typeName(info)`, `typeKind(info)`, and field/variant count helpers.
- `symbolInfo(@fn(name))` for visible functions.
- `moduleInfo`, `packageInfo`, and `apiSurface`.
- `traitInfo`, `interfaceInfo`, `contractInfo`, and `effectInfo`.
- `closureInfo` for signature-level closure metadata.

Fase 12 does not start compile-time evaluation, macros, dynamic evaluation, string-based invocation, metadata mutation, or full runtime reflection. Private/internal data remains protected by visibility gates.
