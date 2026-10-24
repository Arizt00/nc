# Compiler Indexed LValue (v2.1.0)

Implemented:
- parser recognizes indexed lvalue projection targets.
- lowering route maps indexed projection to internal `arrayWrite` call-form.
- sema now accepts `Array` and `Vector` receivers for array method/index operations.

Validated by:
- `examples/arrays/array_dynamic_index_write.n`
- integration test matrix `official_arrays_lists_indexing_matrix`.
