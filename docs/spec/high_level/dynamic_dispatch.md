# High Level Dynamic Dispatch

Dynamic dispatch en high-level es explicito y solo opera sobre trait objects `dyn Trait<...>`.

Formas baseicas:

```n
let obj: dyn Render<int> = dynCast<Render<int>>(42);
let text: str = dyn.call(@obj, render);
```

Reglas:

- `dyn Trait<T>` requiere que `Trait` exista y sea trait.
- `dynCast<Trait<T>>(value)` requiere un impl unico para el tipo concreto.
- `dyn.call(@obj, method, args...)` requiere que el metodo exista en el trait object.
- No hay `dyn` implicito.
- No hay fallback `any`.
- Los argumentos deben coincidir con la firma del slot de vtable.
- Los spans del receptor, metodo y llamada se preservan para diagnosticos.

Diagnostics principales:

- `E_HIGH_DYN_TRAIT_UNKNOWN_001`
- `E_HIGH_DYN_NON_TRAIT_001`
- `E_HIGH_DYN_METHOD_MISSING_001`
- `E_HIGH_DYN_METHOD_SIGNATURE_001`
- `E_HIGH_DYN_IMPLICIT_001`
- `E_HIGH_ANY_LEAK_001`
