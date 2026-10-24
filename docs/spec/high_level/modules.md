# High Level Modules

A module is the logical namespace for high-level declarations.

Baseical section form:

```n
geometryModule {
    module:
        path: math.geometry;
        package: geometry_core;
}
```

Top-level single-module form is also accepted:

```n
module math.geometry;
```

Rules:

- A module path is lowercase, stable and dot-separated.
- Duplicate module paths are rejected.
- The module graph is built from section-level `use:` declarations.
- Cycles are rejected with `E_HIGH_MODULE_CYCLE_001`.
- A module may be associated with a local package.

Diagnostics: `E_HIGH_MODULE_DUP_001`, `E_HIGH_MODULE_PATH_001`, `E_HIGH_MODULE_CYCLE_001`, `E_HIGH_MODULE_UNKNOWN_001`, `E_HIGH_MODULE_SYMBOL_DUP_001`.
