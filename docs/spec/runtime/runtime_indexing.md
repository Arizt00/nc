# Runtime Indexing (v2.1.0)

Status: implemented for read/write in compiler+runtime pipeline.

Supported:
- static read: `@arr[0]`
- dynamic read: `@arr[@i]`
- static write: `@arr[0] >> int : 10;`
- dynamic write: `@arr[@i] >> int : @v;`
- multidim syntax accepted (`@grid[x, y]`) and compiler-side diagnostics available.

Compiler diagnostics surface includes:
- `E_TYPE_ARRAY_BOUNDS_001`
- `E_ARRAY_DIMENSION_001`
- `E_TYPE_MISMATCH_001`
