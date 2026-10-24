# API Surface Introspection

Fase 12 reuses the public API surface hardened in Fase 10.

`apiSurface()` returns an `ApiSurfaceInfo` value. `apiExportCount()` provides a stable scalar query for tests and reports.

The API surface is visibility-filtered. Private symbols and private types are not exposed as public metadata.
