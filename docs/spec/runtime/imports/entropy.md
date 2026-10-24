# Entropia del sistema

## Superficie canonica

La familia `nC.runtime.entropy` forma parte del prelude hosted del backend. No
requiere `use` en un archivo `.n`.

```n
buffer >> p<byte> <- stackAlloc(32, 16);
ok >> bool <- entropyFill(buffer, 32);
```

| Funcion | Firma | Propiedad |
|---|---|---|
| `entropyFill` | `(p<byte>, u64) -> bool` | Escribe exactamente en memoria del llamador |
| `entropyAvailable` | `() -> bool` | Comprueba el proveedor con una lectura real |
| `entropyBytes` | `(u64) -> Result<memRegion<byte>, EntropyError>` | Compatibilidad: asigna una region explicita |
| `entropyRelease` | `(Result<memRegion<byte>, EntropyError>) -> bool` | Libera la region de compatibilidad |

## Semantica raw

`entropyFill` no asigna, no amplia, no valida procedencia y no libera memoria.
El llamador aporta la direccion y la capacidad. Un puntero nulo, colgante,
desalineado o demasiado corto conserva el comportamiento raw del programa.

`entropyBytes` se mantiene para codigo anterior. Su resultado material posee
memoria y debe entregarse a `entropyRelease` exactamente cuando el programador
lo decida.

## Materializacion

| Target | Proveedor |
|---|---|
| Windows x86_64/AArch64 | `SystemFunction036` de `ADVAPI32` |
| Linux/Android, 32 y 64 bits | `getrandom` |
| Darwin x86_64/AArch64 | `getentropy`, dividido en bloques de 256 bytes |
| `wasm32-wasi` | `__wasi_random_get` |

LLVM/MCT define `ncc_rt_entropy_fill`, `ncc_rt_entropy_available`,
`ncc_rt_entropy_bytes` y `ncc_rt_entropy_release`. La familia no tiene una
implementacion paralela en `runtime_hosted_native.c`.

## Evidencia

- Ejemplo: `examples/runtime/families/entropy_basic.n`.
- Windows: parser, sema, IR, LLVM, objeto COFF, enlace PE y ejecucion nativa.
- Linux y Darwin: LLVM y objeto target-native generados desde Windows; el
  enlace y la ejecucion requieren el sysroot y el host correspondientes.
