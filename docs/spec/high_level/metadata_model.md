# High-Level Metadata Model

Fase 12 metadata is represented by limited metadata carrier types:

- `TypeInfo<T>`
- `SymbolInfo`
- `ModuleInfo`
- `PackageInfo`
- `ApiSurfaceInfo`
- `TraitInfo`
- `InterfaceInfo`
- `ContractInfo`
- `EffectInfo`
- `ClosureInfo`
- `MetadataReport`

Metadata values are ordinary read-only values for type checking. They do not grant authority to mutate, invoke, or construct code dynamically.
