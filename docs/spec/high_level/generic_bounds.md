# High Level Generic Bounds

Los bounds limitan parametros genericos a tipos que implementan una interface o trait.

Inline:

```n
fn render<T: Show<T>>(value: T) -> str {
    public:
        return show(@value);
}
```

Where:

```n
fn render<T>(value: T) -> str where T: Show<T> {
    public:
        return show(@value);
}
```

Reglas:

- El parametro del bound debe estar declarado.
- El surface del bound debe existir.
- La aridad del bound debe coincidir con el surface.
- Al resolver una llamada generica, el tipo concreto debe tener un impl compatible.
- `any` no puede escapar como tipo generico.
- HKT y dispatch dinamico se rechazan con diagnostics de fase.

Diagnostics:

- `E_HIGH_BOUND_UNKNOWN_PARAM_001`
- `E_HIGH_BOUND_UNKNOWN_TRAIT_001`
- `E_HIGH_BOUND_ARITY_001`
- `E_HIGH_BOUND_UNSATISFIED_001`
- `E_HIGH_BOUND_DYNAMIC_DISPATCH_DEFERRED_001`
- `E_HIGH_BOUND_HKT_DEFERRED_001`
