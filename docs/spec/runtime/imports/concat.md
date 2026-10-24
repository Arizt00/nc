# nC Runtime Import: nC.runtime.concat

## 1. Purpose
`nC.runtime.concat` provides explicit string joining functions. The `+` operator also performs text concatenation only when both operands are already `str`.

## 2. Canonical compact import
```n
use stoud.io { concat };
```

## 3. Aliases
No public alias is closed for this family. The module identity is `runtime.concat`.

## 4. Mode availability
Allowed in `[@runtime]` and reports. Forbidden in `[@]`, `primitive[@]`, and `[@machine]` as a convenience import.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `concat` | `concat(str, str) -> str` | `str` | `runtime.format.basic` | `imports.official-runtime-required`, `runtime.no-silent-fallback` | `E_RUNTIME_CONCAT_IMPORT_001`, `E_RUNTIME_CONCAT_TYPE_001` | explicit two-string concat |
| `concat3` | `concat3(str, str, str) -> str` | `str` | `runtime.format.basic` | same | same | explicit three-string concat |
| `concatMany` | `concatMany(str, ...) -> str` | `str` | `runtime.format.basic` | same | same | variadic string concat |
| `join` | `join(str, any) -> str` | `str` | `runtime.format.basic` | same | same | separator plus renderable value/vector |

## 6. Type contracts
`str + str -> str`. `str + int`, `str + bool`, `str + ptr<T>`, and other mixed forms fail unless the non-string side is explicitly rendered first.

## 7. Ownership rules
Concat borrows input strings and returns a fresh string value.

## 8. Nullability rules
Null is not a string. Nullable values must be refined or rendered by an explicit nullable-aware surface before concatenation.

## 9. Bounds rules
Concat does not expose indexing. Array or region bounds are validated before values reach concat.

## 10. Side effects
Concat has no output side effect. `print` and `println` perform output.

## 11. Capabilities
- `runtime.format.basic`
- `runtime.render.basic`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.error.typed-required`

## 13. Errors
- `E_RUNTIME_USE_001`: direct concat call without `use nC.runtime.concat`.
- `E_RUNTIME_CONCAT_IMPORT_001`: concat import contract missing.
- `E_RUNTIME_CONCAT_TYPE_001`: `+` or direct concat received a non-string value without render.
- `E_RUNTIME_CONCAT_NONE_001`: concat attempted to join `void`.
- `E_RUNTIME_CONCAT_ANY_LEAK_001`: concat return escaped as `any`.

## 14. Positive examples
- `examples/runtime/concat/concat_str_str_ok.n`
- `examples/runtime/concat/concat_chained_ok.n`
- `examples/runtime/concat/concat_runtime_output_ok.n`

## 15. Negative examples
- `examples/runtime/concat/concat_str_int_fail.n`

## 16. Strict mode
Strict mode keeps the same rule: `+` accepts only `str + str` for text concatenation.

## 17. Runtime report shape
Runtime reports list the `concat` family import and direct concat calls. Compiler reports expose `contracts.render_concat.concat_expressions`.

## 18. Industrial conformance checklist
- `concat("a", "b") -> str`
- chained direct concat runs
- `print("Sensor " + renderInt(@i) + ": ")` runs
- `println("Valor: " + @x)` fails for `x:int`
- positive runtime output contains no `none`, `<expr>`, or `any`

## 19. Known non-goals
Concat does not perform magic rendering. Users must call `renderInt`, `renderBool`, or another explicit render function.

## 20. Evidence
- `cargo test -p ncc-cli --test runtime_render_concat_closure -- --nocapture`
- `ncc run examples/runtime/concat/concat_str_str_ok.n`
- `ncc run examples/runtime/concat/concat_chained_ok.n`
- `ncc check examples/runtime/concat/concat_str_int_fail.n`
