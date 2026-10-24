# Concat Rules

## Surface
- `str + str` is allowed in high/runtime contexts.
- `str + non-str` is rejected; use explicit `concat()` + `render*()` route.
- `nC.runtime.concat` is the official explicit family:
  - `concat(str, str) -> str`
  - `concat3(str, str, str) -> str`
  - `concatMany(str, ...) -> str`
  - `join(str, any) -> str`

## Context Restrictions
- `[@]`: text `+` is forbidden.
- `[@machine]`: text `+` is forbidden.
- `primitive[@]`: text `+` is forbidden.

## Diagnostics
- `E_TYPE_MODE_FORBIDDEN_001` for mode-forbidden text concat.
- `E_TYPE_MISMATCH_001` for implicit mixed concat (`str + int`, etc).

## Baseical Fixtures
- `/examples/concat/concat_runtime_plus_ok.n`
- `/examples/concat/concat_runtime_function_ok.n`
- `/examples/concat/concat_runtime_concat_many_ok.n`
- `/examples/concat/concat_runtime_join_ok.n`
- `/examples/concat/concat_runtime_render_required_fail.n`
- `/examples/concat/concat_root_mode_forbidden_fail.n`
