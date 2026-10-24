# High Level Fase 5: Dynamic Dispatch / Trait Objects / Associated Types

Estado: closed.

Fase 5 cierra dispatch dinamico explicito, trait objects, associated types, object safety, modelo semantico de vtable y diagnosticos baseicos. La fase se apoya en Fase 4 normalizada por secciones y preserva dispatch estatico.

No incluye Fase 6+, async traits, effects, pattern matching avanzado, specialization completa ni HKT real.

## Superficie baseica

```n
phase5Example {
    trait:
        Render<T> {
            type Output;
            fn render(value: T) -> Output;
        }

    impl:
        Render<int> for int {
            type Output = str;

            fn render(value: int) -> str {
                public:
                    return "int";
            }
        }

    run :: in(value >> dyn Render<int>) -> str {
        return dyn.call(value, render);
    }
}
```

`dyn` es explicito. No existe conversion implicita desde un bound estatico a trait object. La construccion se hace con `dynCast<Trait<Concrete>>(value)` cuando el tipo concreto tiene un impl unico y verificable.

## Cierre

- Dynamic dispatch: closed.
- Trait objects: closed.
- Associated types: closed.
- Object safety: closed.
- Vtable semantic layout: closed.
- Static dispatch: preserved.
- Specialization: deferred con diagnostico.
- HKT: deferred con diagnostico.
