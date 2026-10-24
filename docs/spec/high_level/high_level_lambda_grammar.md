# High Level Fase 11 Lambda Grammar

Status: closed for Fase 11.

Lambda expressions are first-class high-level expressions with explicit parameter and return
types. A lambda does not infer width, arity, or return type silently.

Baseical forms:

```n
|x: int| -> int {
    return @x + 1;
}

|| -> int {
    return 42;
}
```

Function types use the official shape:

```n
fn(int) -> int
fn(int, bool) -> Result<int, Error>
fn() -> bool
```

Grammar summary:

```text
lambda_expr       :<- "move"? lambda_params "->" type effects? captures? block
lambda_params     :<- "||" | "|" lambda_param ("," lambda_param)* "|"
lambda_param      :<- ident ":" type
effects           :<- "effects" "(" ident_path ("." ident_path)* ")"
captures          :<- "captures" "(" capture_item ("," capture_item)* ")"
capture_item      :<- ("ref" | "mut" | "move")? "@" ident
```

Parser diagnostics include `E_HIGH_CLOSURE_PARAM_001` for a missing explicit parameter type.
`async |...|` and `async move |...|` are accepted and lower to the same typed coroutine descriptor
ABI as async functions. Obsolete callable spellings such as `lambda(...)` remain rejected; they are
not aliases for closure literals.
