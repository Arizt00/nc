# Import Mode Gates

Allowed import modes:

- normal `.n`
- high-level `.n`
- runtime/compiler/tooling surfaces

Forbidden import modes:

- `[@machine]`: `E_USE_FORBIDDEN_IN_MACHINE_001`
- `primitive[@]`: `E_USE_FORBIDDEN_IN_PRIMITIVE_001`
- basal `.nb`/`.nbb`: `E_USE_FORBIDDEN_IN_BASAL_001`
- future `[@kernel]`: `E_USE_FORBIDDEN_IN_KERNEL_001`

Baseical message:

```text
Este modo no usa imports. Declare capacidades/secciones mediante la gramatica propia del modo.
```
