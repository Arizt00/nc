# Template macros

Template macros describe structured item generation without treating text as source. They produce generated AST items such as functions, types, const records, impl records, annotations, or report entries.

```n
macro:
    template makeBox(name: ident) {
        generate type name;
    }
    expand makeBox(BoxedValue);
```

Template output is validated as generated AST. Invalid generated forms emit `E_HIGH_MACRO_INVALID_GENERATED_AST_001`. Size, recursion, and nesting budgets apply to templates exactly as they do to declarative macros.

Templates may bridge to Fase 13 const eval through explicit `constEval` clauses. Such bridges remain pure, deterministic, bounded, and visibility-aware.
