# Runtime Arrays (v2.1.0)

Status: implemented for high/intermediate runtime surface.

Supported surfaces:
- `array<T>`
- `array<T, N>`
- `T[N]` parser sugar to `array<T, N>`
- `xy<T>`, `xyz<T>`, `matrix<T, ...>`, `tensor<T, ...>` (semantic surface)
- indexed read/write including dynamic index projection.

Key guarantees:
- indexed lvalue projection now resolves for `Array` and `Vector` receivers (`arrayWrite` / `arrayRead`).
- `array<T, N>` declarations without an explicit literal are materialized as `N` default elements
  (`0` for integer-like scalars, `false` for bool, `null` for pointer-like values). This is a
  fixed-size zero-fill contract; dynamic `array<T>` and `vector<T>` declarations still start empty.
- no `String("<expr>")` placeholder in array condition/index surface.
- industrial policy (strict): open-dimensional `matrix<T, ...>` / `tensor<T, ...>` cannot be indexed with `[]`.
  Declare numeric dimensions first (for example `matrix<int, 2, 2>`, `tensor<int, 2, 2, 2>`) before using `[]`.
  Baseical diagnostic: `E_ARRAY_OPEN_DIM_INDEX_001`.

Examples:
- `examples/arrays/array_basic.n`
- `examples/arrays/array_fixed_generic_len.n`
- `examples/arrays/array_c_style_len.n`
- `examples/arrays/array_dynamic_index_write.n`
