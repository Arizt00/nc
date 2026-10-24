# nC Runtime Import: nC.runtime.render

## 1. Purpose
`nC.runtime.render` materializes runtime values as `str`. It is the only official path for using non-string values inside text concatenation.

## 2. Canonical compact import
```n
use stoud.io { render };
```

## 3. Aliases
No public alias is closed for this family. The module identity is `runtime.render`.

## 4. Mode availability
Allowed in `[@runtime]` and compiler/runtime reports. Forbidden in `[@]`, `primitive[@]`, and `[@machine]` as a convenience import.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `renderInt` | `renderInt(int) -> str` | `str` | `runtime.render.basic` | `imports.official-runtime-required`, `runtime.no-silent-fallback` | `E_RUNTIME_RENDER_IMPORT_001`, `E_RUNTIME_RENDER_TYPE_001` | integer text |
| `renderUInt` | `renderUInt(u64) -> str` | `str` | `runtime.render.basic` | same | same | unsigned text |
| `renderBinary` | `renderBinary(int) -> str` | `str` | `runtime.render.basic` | same | same | binary integer text; hosted runtime may use external libc `%b` after an OS probe; fallback emits only `-`, `0`, `1` bytes |
| `renderBin` | `renderBin(int) -> str` | `str` | `runtime.render.basic` | same | same | compact alias of `renderBinary` |
| `renderFloat` | `renderFloat(ft64[, precision]) -> str` | `str` | `runtime.render.basic` | same | same | minimum arity is one; precision is compatibility input |
| `renderBool` | `renderBool(bool) -> str` | `str` | `runtime.render.basic` | same | same | `true` or `false` |
| `renderChar` | `renderChar(char) -> str` | `str` | `runtime.render.basic` | same | same | single character text |
| `renderType` | `renderType(T) -> str` | `str` | `runtime.render.debug` | same | same | type name |
| `renderDebug` | `renderDebug(T) -> str` | `str` | `runtime.render.debug` | same | same | debug text |
| `renderStable` | `renderStable(T) -> str` | `str` | `runtime.render.stable` | same | same | stable text |

## 6. Type contracts
Every render function returns `str`. A render function must not return `void`, `any`, or a deferred runtime value.

## 7. Ownership rules
Render borrows its input value and returns a fresh string value. It does not move, clone, or mutate the source.

## 8. Nullability rules
Nullable values must be refined or rendered through a family that explicitly accepts them. `renderInt(null)` and similar mismatches fail at sema.

## 9. Bounds rules
Render does not index or allocate bounded memory. Bounds-sensitive values must already be valid before rendering.

## 10. Side effects
Render itself has no output side effect. Printing happens only through `nC.runtime.output`.

## 10.1 Provider contract
`renderBinary` and `renderBin` are part of the external `ncc-runtime-stdlib`
package shipped with the toolchain. In hosted `ncc run`, the runtime selects
the provider per OS:

- `external-libc-percent-b`: uses C `snprintf("%llb")` only after a runtime
  probe verifies that the active OS/libc really returns `101` for `5`.
- `portable-exact`: emits the ASCII bytes itself and is mandatory on systems
  where `%b` is missing or incompatible.

The environment variable `NCC_RENDER_BINARY_PROVIDER` (or
`NC_RENDER_BINARY_PROVIDER`) accepts `auto`, `portable-exact`, and
`libc-percent-b`. The language-level result is identical in all modes:
`0 -> "0"`, `5 -> "101"`, `-5 -> "-101"`.

## 11. Capabilities
- `runtime.render.basic`
- `runtime.render.debug`
- `runtime.render.stable`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`
- `runtime.error.typed-required`
- `runtime.no-secret-output`
- `runtime.redact-secrets`

## 13. Errors
- `E_RUNTIME_USE_001`: render call without `use nC.runtime.render`.
- `E_RUNTIME_RENDER_IMPORT_001`: render import contract missing.
- `E_RUNTIME_RENDER_TYPE_001`: invalid render input type.
- `E_RUNTIME_RENDER_NONE_001`: render attempted to materialize `void`.
- `E_RUNTIME_RENDER_ANY_LEAK_001`: render return escaped as `any`.

## 14. Positive examples
- `examples/runtime/render/render_int_concat_ok.n`
- `examples/runtime/render/render_bool_concat_ok.n`
- `examples/runtime/render/render_chained_concat_ok.n`
- `examples/runtime/render/render_assignment_to_str_ok.n`

## 15. Negative examples
- `examples/runtime/render/render_missing_use_fail.n`
- `examples/runtime/render/render_wrong_type_fail.n`

## 16. Strict mode
Strict mode keeps the same rule: non-string values require explicit render before text concatenation.

## 17. Runtime report shape
Runtime reports list the `render` family import and its `render*` calls. Compiler reports expose `contracts.render_concat.render_calls`.

## 18. Industrial conformance checklist
- `renderInt(@x) -> str`
- `renderBinary(@x) -> str`, with `0 -> "0"`, `5 -> "101"`, `-5 -> "-101"`
- `renderBinary` reports a runtime provider but never leaks a platform-specific prefix
- `println("Valor: " + renderInt(@x))` runs
- chained concatenation with rendered values runs
- missing `use nC.runtime.render` fails
- wrong render input type fails
- output from positive examples contains no `none`, `<expr>`, or `any`

## 19. Known non-goals
Render does not make `str + int` legal. It does not replace `print(@x)`, which remains valid for direct output.

## 20. Evidence
- `cargo test -p ncc-cli --test runtime_render_concat_closure -- --nocapture`
- `ncc run examples/runtime/render/render_int_concat_ok.n`
- `ncc run examples/runtime/render/render_chained_concat_ok.n`
- `ncc run examples/stress_runtime_industrial_pack.n` with input fixture
