# nC Runtime Render And Concat

## Contract
Runtime text output has three separate surfaces:

- `render`: value to `str`.
- `concat`: `str` plus `str` to `str`.
- `output`: `print` and `println`.

The contract is explicit. `println("Valor: " + renderInt(@x));` is valid. `println("Valor: " + @x);` is invalid when `@x` is `int`.

## Runtime Rules
| Expression | Result |
|---|---|
| `renderInt(int)` | `str` |
| `renderBool(bool)` | `str` |
| `renderFloat(ft64[, precision])` | `str` |
| `renderChar(char)` | `str` |
| `"a" + "b"` | `str` |
| `"a" + renderInt(1)` | `str` |
| `renderInt(1) + "a"` | `str` |
| `"a" + 1` | `E_RUNTIME_CONCAT_TYPE_001` |
| `renderInt("x")` | `E_RUNTIME_RENDER_TYPE_001` |

## Evaluator Requirements
Render calls must produce `RuntimeValue::String`. Concat expressions must preserve intermediate strings in chained binary expressions. `RuntimeValue::Void` is not a valid render or concat result because human rendering of void is `none`.

## Imports
`render*` requires:

```n
use stoud.io { render };
```

Direct `concat(...)` calls require:

```n
use stoud.io { concat };
```

The `+` operator does not require importing `nC.runtime.concat`, but it only accepts operands that are already `str`.

## Evidence
- `examples/runtime/render/render_int_concat_ok.n`
- `examples/runtime/render/render_chained_concat_ok.n`
- `examples/runtime/concat/concat_str_str_ok.n`
- `examples/runtime/concat/concat_str_int_fail.n`
- `examples/stress_runtime_industrial_pack.n`
- `crates/ncc-cli/tests/runtime_render_concat_closure.rs`

## Release State
`nC.runtime.render` and `nC.runtime.concat` are closed for local release-industrial behavior when the full validation suite is green. This document does not claim external hardware or CI evidence.
