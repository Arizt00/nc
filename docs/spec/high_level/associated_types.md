# High Level Associated Types

Associated types se declaran dentro de `trait:` y se enlazan dentro de `impl:`.

```n
trait:
    Container<T> {
        type Item;
        fn get(value: T) -> Item;
    }

impl:
    Container<BoxInt> for BoxInt {
        type Item = int;

        fn get(value: BoxInt) -> int {
            public:
                return 1;
        }
    }
```

Reglas:

- Cada associated type debe declararse una sola vez por trait.
- Cada impl debe proporcionar todos los bindings requeridos.
- No se aceptan bindings desconocidos.
- El retorno de metodos puede proyectar `Item` y resolverse contra el impl concreto.
- Las proyecciones ambiguas fallan; no hay inferencia silenciosa.

Diagnostics:

- `E_HIGH_ASSOC_TYPE_DUP_001`
- `E_HIGH_ASSOC_TYPE_MISSING_001`
- `E_HIGH_ASSOC_TYPE_UNKNOWN_001`
- `E_HIGH_ASSOC_TYPE_AMBIGUOUS_001`
- `E_HIGH_ASSOC_TYPE_BINDING_001`
