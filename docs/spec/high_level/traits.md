# High Level Traits

Un trait declara comportamiento nominal reutilizable. En Fase 4 los traits admiten metodos requeridos y default methods dentro del subconjunto cerrado.

```n
trait:
    Ready<T> {
        fn truthy(value: T) -> bool;
        fn ready(value: T) -> bool {
            public:
                return true;
        }
    }
```

`trait:` es la forma baseica seccionada. Las declaraciones flotantes se aceptan solo para compatibilidad de Fase 4 y no son el base para ejemplos nuevos.

Los metodos requeridos deben existir en el impl. Los default methods quedan disponibles para dispatch estatico si el impl no los redefine.

Diagnostics principales:

- `E_HIGH_TRAIT_PARSE_001`
- `E_HIGH_TRAIT_DUP_001`
- `E_HIGH_TRAIT_METHOD_MISSING_001`
- `E_HIGH_TRAIT_METHOD_SIGNATURE_001`
- `E_HIGH_TRAIT_AMBIGUOUS_IMPL_001`

Trait objects y dispatch dinamico quedan fuera de Fase 4.
