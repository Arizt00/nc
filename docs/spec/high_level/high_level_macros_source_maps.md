# Macro source maps

Every accepted macro expansion records source mapping from declaration to invocation to generated item. The mapping is used by diagnostics, reports, and future lowering stages.

Source map fields:

- macro declaration line and column.
- invocation line and column.
- generated item line and column.
- container name.
- generated item kind.
- generated name template.

If a macro suppresses required mapping, the compiler emits `E_HIGH_MACRO_SOURCE_MAP_001`. This keeps generated code auditable and prevents errors from being detached from user source.
