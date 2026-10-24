# Ejemplos MCT y perfil destructivo

Todos los archivos son nC ordinario. La peligrosidad se selecciona con
`--cc nC.destructive`; no existe sintaxis Bear en estas fixtures.

- `destructive_profile_control.n`: cuerpo minimo para comparar perfiles.
- `destructive_backend_profile.n`: asignaciones simples y evidencia LLVM.
- `destructive_statement_order.n`: sentencias que pueden omitirse, repetirse o
  moverse al final del bloque.
- `destructive_manual_pointer.n`: `alloc`, lectura, escritura y `free` manuales;
  la materializacion puede alterar cualquiera de esos pasos.
- `destructive_execute_console.n`: hace visibles `omit`, `repeat`, `split` y
  `once` usando exclusivamente sentencias `println` de nC.
- `destructive_execute_pointer.n`: ejecuta `alloc`, `ptrWrite`, `ptrRead` y
  `free` reales; las operaciones criticas caen en `once` y los marcadores
  conservan la ambiguedad destructiva alrededor de ellas.
- `destructive_missing_symbol.n`: contrato FFI tipado cuyo simbolo ausente se
  enlaza a la direccion `0`. Se debe inspeccionar, no ejecutar a ciegas.

Comprobacion rapida desde WSL:

```bash
target/debug/ncc check examples/10_mct/destructive_statement_order.n
target/debug/ncc check examples/10_mct/destructive_manual_pointer.n
target/debug/ncc check examples/10_mct/destructive_missing_symbol.n
```

Ejecucion nativa desde WSL:

```bash
target/debug/ncc run \
  examples/10_mct/destructive_execute_console.n \
  --native-fast \
  --profile release-fast \
  --cc nC.destructive \
  --target x86_64-unknown-linux-gnu \
  --show-return

target/debug/ncc run \
  examples/10_mct/destructive_execute_pointer.n \
  --native-fast \
  --profile release-fast \
  --cc nC.destructive \
  --target x86_64-unknown-linux-gnu \
  --show-return
```

Ambos comandos compilan por MCT/LLVM, enlazan bytes ELF y lanzan ese
ejecutable. No usan el interprete de nC. El ejemplo de simbolo ausente queda
fuera de esta bateria porque materializa una llamada a la direccion `0`.

Build nativo de cualquier ejemplo:

```bash
target/debug/ncc build \
  examples/10_mct/destructive_statement_order.n \
  target/destructive-order \
  --out-dir target/destructive-order-evidence \
  --profile release-fast \
  --cc nC.destructive \
  --target x86_64-unknown-linux-gnu \
  --no-incremental
```
