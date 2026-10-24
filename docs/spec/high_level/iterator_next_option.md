# Iterator `next` Returns `Option`

Fase 8 depends on Fase 7 algebra:

```n
@next >> Option<int> : @iter.next();
```

This keeps exhaustion explicit and forbids silent sentinel values.

Rules:

- `next()` has zero arguments.
- the return type is `Option<T>`.
- `None` requires an `Option<T>` context.
- `Some(T)` payloads must match the iterator item type.
- `any` is never permitted as the iterator item type.

This is a semantic surface rule. It does not open async streams, generators, closures, or effectful scheduling.
