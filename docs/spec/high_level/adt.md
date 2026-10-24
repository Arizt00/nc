# High Level ADT Model

Status: closed in Fase 6.

High-level ADTs are closed enum families with typed variants. The semantic model preserves the enum identity, variant identity, payload field names, payload field types, and generic parameters.

ADT values are ordinary high-level values. Fase 6 verifies their source-level construction and pattern matching safety. It does not yet define a complete `Result` / `Option` error algebra, effect propagation, or async semantics.

ADT closure criteria:

- Real AST nodes: `EnumDecl`, `EnumVariant`, and `EnumPayloadField`.
- Real semantic inventory for enum and variant lookup.
- Constructor type checking.
- Match safety over ADT variants.
- Diagnostics for duplicate enum/variant, unknown enum/variant, bad arity, bad payload type, and `any` leaks.
