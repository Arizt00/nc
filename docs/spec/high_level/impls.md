# High Level Impl Blocks

Un impl conecta una interface o trait con un tipo concreto.

```n
impl:
    Show<int> for int {
        fn show(value: int) -> str {
            public:
                return "int";
        }
    }
```

`impl:` es la forma baseica seccionada. La normalizacion no cambia la resolucion de bounds ni el dispatch estatico: solo agrupa implementaciones en una superficie estable.

Reglas:

- El target del impl debe ser una interface o trait conocido.
- La aridad generica debe coincidir.
- Un par `(surface, type)` no puede repetirse.
- Los metodos requeridos deben existir.
- Las firmas se comparan despues de sustituir parametros del surface.

El compilador no acepta impls ambiguos ni metodos con firmas incompatibles.

Diagnostics:

- `E_HIGH_IMPL_PARSE_001`
- `E_HIGH_IMPL_UNKNOWN_TRAIT_001`
- `E_HIGH_IMPL_ARITY_001`
- `E_HIGH_IMPL_DUP_001`
