# High Level Fase 14 macros overview

Fase 14 closes compile-time macro expansion for the high-level nC surface. The feature is source-level and compile-time only: it parses macro declarations, expands invocations into typed generated items, validates hygiene, keeps source maps, emits expansion reports, and reuses the closed Fase 13 const-eval and comptime-unsafe lanes.

Supported macro families:

- `declarative`: pattern-like generation with typed parameters.
- `template`: structured generation of named items.
- `attribute`: item annotation and wrapper generation.
- `derive`: trait implementation generation from declared type metadata.
- `unsafe`: explicit authority lane for compile-time effects, dynamic eval, reflection mutation, private metadata, construction leaks, side effects, and no-limit expansion.

Macros live in a `macro:` section. A declaration has a kind, name, typed parameters, a body, and one or more `generate` clauses. Invocations use `expand name(args);`. Generated items are validated before the program is accepted.

Example:

```n
phase14MacroDeclarativeSimple {
    macro:
        declarative makeRun(name: ident) {
            generate fn name -> bool = true;
        }
        expand makeRun(generatedRun);

    run :: in() -> bool {
        return true;
    }
}
```

Fase 14 does not start Fase 15 or runtime metaprogramming. The macro engine operates before later lowering and reports `phase15_plus = not_started`.
