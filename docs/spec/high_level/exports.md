# High Level Exports

Exports define the public module surface.

```n
export:
    fn area;
    enum ShapeKind;
    trait Drawable;
    use math.geometry.area;
```

Rules:

- Unknown exports fail.
- Private symbols cannot be exported.
- Duplicate exports fail.
- Re-exports must target visible exportable symbols.
- Generic, trait, enum, async and Result/Option signatures remain visible in the API surface.

Diagnostics: `E_HIGH_EXPORT_UNKNOWN_001`, `E_HIGH_EXPORT_PRIVATE_001`, `E_HIGH_EXPORT_DUP_001`, `E_HIGH_EXPORT_REEXPORT_001`, `E_HIGH_EXPORT_EFFECTS_001`, `E_HIGH_EXPORT_API_UNSTABLE_001`.
