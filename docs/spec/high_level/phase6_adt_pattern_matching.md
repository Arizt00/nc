# High Level Fase 6: ADT / Pattern Matching

> Historical phase boundary. This file records the original ADT milestone. The
> current matching contract includes structs, enums, unions, ranges, arrays,
> nested patterns and guards, and is defined by the generated exact-head
> specification and the current resolution report.

Status: historical; superseded for current capability status.

Fase 6 adds algebraic data types through `enum:` sections and official ADT pattern matching through `match`. It closes syntax, semantic checks, exhaustiveness, unreachable arm detection, typed payload bindings, basic generic enum constructors, pure guards, examples, tests, CLI manifests, and reports.

Those later capabilities were outside the original Phase 6 boundary. This statement is historical and does not mark them deferred in the current compiler.

Baseical shape:

```n
Root {
    enum:
        Maybe<T> {
            Some(value: T);
            None;
        }

    unwrapInt :: in(value >> Maybe<int>) -> int {
        match (value) {
            Maybe.Some(x) => return @x;
            Maybe.None => return 0;
        }
    }
}
```

Original Phase 6 safety rules:

- At this historical boundary a pattern match scrutinee had to be a known enum type. Current matching also accepts the non-ADT forms listed above.
- All enum variants must be covered unless a wildcard arm is present.
- Wildcard arms make following arms unreachable.
- Duplicate unguarded variant arms are rejected.
- Guarded arms do not count as exhaustive coverage.
- Pattern payload bindings are typed from the enum variant payload.
- `any` is not allowed to leak through a pattern match.

Primary diagnostics: `E_HIGH_ENUM_*`, `E_HIGH_PATTERN_*`, `E_HIGH_MATCH_*`, `E_HIGH_GUARD_*`, and `E_HIGH_ANY_LEAK_001`.
