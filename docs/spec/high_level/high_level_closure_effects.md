# High Level Fase 11 Closure Effects

Status: closed for Fase 11.

Closure effects are explicit annotations on callable values. Fase 11 uses them for capture safety
and pure-call boundaries; it does not introduce a new runtime effect engine.

Example:

```n
@writeState >> fn(int) -> int : |value: int| -> int
effects(state.write)
captures(mut @state)
{
    @state >> int : @value;
    return @state;
};
```

Rules:

- mutating a captured symbol requires `effects(state.write)`;
- closures passed to pure contexts must not declare effects;
- effect annotations are preserved in the AST and semantic validation;
- async closures preserve their explicit effects and lower captures into persistent frame slots;
- generator and async-stream suspension never erases the declared callable or payload type.

Diagnostics:

- `E_HIGH_CLOSURE_EFFECT_MISSING_001`
- `E_HIGH_CLOSURE_EFFECT_MISMATCH_001`
- `E_HIGH_CLOSURE_EFFECT_TYPE_001`
