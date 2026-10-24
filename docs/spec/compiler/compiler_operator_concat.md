# Compiler Operator Concat

## Rule
The compiler resolves `+` as text concat only for `str + str`. It resolves numeric `+` separately. Mixed text and non-text values are rejected with `E_RUNTIME_CONCAT_TYPE_001`.

## Accepted
```n
@line >> str : "A=" + renderInt(@a) + " B=" + renderInt(@b);
```

This succeeds because every `+` node sees `str + str` after `renderInt` returns `str`.

## Rejected
```n
@line >> str : "A=" + @a;
```

This fails because `@a:int` has not been rendered.

## Import Discipline
The operator itself is not a direct `concat(...)` call and does not require `use nC.runtime.concat`. Render calls used inside the expression still require `use nC.runtime.render`.

## Compiler Report
`ncc compiler report <file> --json` includes:

```json
{
  "contracts": {
    "render_concat": {
      "render_calls": {},
      "concat_function_calls": {},
      "concat_expressions": 0,
      "concat_invalid": 0,
      "render_none_risk": 0,
      "concat_none_risk": 0,
      "any_leak_risk": 0
    }
  }
}
```

On a successful report, invalid concat and value-leak risks are zero because sema has already accepted the program and lowering has passed verification.

## Diagnostics
- `E_RUNTIME_CONCAT_TYPE_001`: explicit render required for `str + non-str`.
- `E_RUNTIME_RENDER_TYPE_001`: invalid input to `render*`.
- `E_RUNTIME_USE_001`: render or direct concat call missing its import.
