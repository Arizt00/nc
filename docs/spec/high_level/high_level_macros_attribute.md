# Attribute macros

Attribute macros attach compile-time annotation behavior to a high-level item. Fase 14 models the attribute target through an `item` parameter and generates an annotation or reportable item.

```n
macro:
    attribute trace(target: item) {
        generate annotation traced;
    }
    expand trace(run);
```

Attribute macros preserve the expansion site and generated-item source map. They cannot mutate the target item through reflection unless declared as `unsafe` with `allow comptime.reflection.mutate;`.

Errors are reported against both the declaration span and invocation span where possible. Missing or invalid source mapping emits `E_HIGH_MACRO_SOURCE_MAP_001`.
