# Runtime Slices / Spans / Views (v2.1.0)

Status: partial.

Implemented today:
- list/array/region helper calls are recognized in runtime/semantic surface where declared.
- list alias API includes `listSlice` and region conversion entry points.

Not fully closed yet:
- full lifetime proof model for slice/span/view invalidation across all movement/realloc paths.
- complete dedicated error matrix for every stale-view subtype.

This is tracked as partial closure while preserving strict diagnostics for unsupported paths.
