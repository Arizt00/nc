# High Level Interfaces

Una interface declara metodos requeridos que un tipo debe implementar para satisfacer un bound.

```n
interface:
    Show<T> {
        fn show(value: T) -> str;
    }
```

`interface:` es la forma baseica seccionada. La forma flotante `interface Show<T> { ... }` queda como compatibilidad legacy y debe reportarse como no preferida en material nuevo.

Reglas:

- La interface tiene nombre nominal.
- Los parametros genericos son explicitos.
- Cada metodo requerido tiene nombre, parametros y retorno.
- Los impls deben cubrir todos los metodos requeridos.
- La firma del impl debe coincidir con la firma de la interface despues de sustituir parametros genericos.

Ejemplo:

```n
impl:
    Show<int> for int {
        fn show(value: int) -> str {
            public:
                return "int";
        }
    }
```

Diagnostics principales:

- `E_HIGH_INTERFACE_PARSE_001`
- `E_HIGH_INTERFACE_DUP_001`
- `E_HIGH_INTERFACE_METHOD_MISSING_001`
- `E_HIGH_INTERFACE_METHOD_SIGNATURE_001`
