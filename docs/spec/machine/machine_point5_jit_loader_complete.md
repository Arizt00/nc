# Machine Point 5: JIT/loader ejecutable

Status: closed for x86_64 host byte execution with explicit loader contract.

## Contrato

`ncc machine run-bytes` ejecuta streams autocontenidos con este modelo:

- host: `x86_64` Windows o Unix;
- ABI: `extern-C fn() -> i64` at offset 0;
- entrada: `.nm` verificado o `.bin` crudo;
- memoria Windows: `VirtualAlloc` RW, copy, `VirtualProtect` RX,
  `FlushInstructionCache`, call, `VirtualFree(MEM_RELEASE)`;
- memoria Unix: `mmap` RW, copy, `mprotect` RX, call, `munmap`;
- W^X: no queda memoria RWX despues de copiar;
- aislamiento: proceso hijo;
- timeout: `--timeout-ms`;
- crash: error de padre, no caida del proceso principal.

El payload JSON contiene:

- `schema: nc.machine.jit.run_bytes.v1`;
- `loader_contract.schema: nc.machine.jit.loader_contract.v1`;
- `memory_backend`;
- `w_x_policy`;
- `icache_policy`;
- `release_policy`;
- `input_contract`;
- `execution_contract`;
- `temp_cleanup`;
- `byte_sha256`.

## Rechazos obligatorios

El runner directo rechaza:

- bytes vacios: `E_MACHINE_JIT_BYTES_001`;
- streams por encima de `--max-bytes`: `E_MACHINE_JIT_BYTES_002`;
- host/ABI no soportado;
- `.nm` no x86_64;
- externs pendientes: `E_MACHINE_JIT_EXTERN_001`;
- relocalizaciones pendientes: `E_MACHINE_JIT_RELOCATION_001`;
- hijo que no vuelve: `E_MACHINE_JIT_TIMEOUT_001`;
- hijo que cae: `E_MACHINE_JIT_CHILD_001`.

## ISA

No se anadio una ISA nueva para este cierre. El loader consume bytes ya emitidos
por el encoder x86_64 existente. Cualquier instruccion nueva que un futuro caso
necesite debe cerrarse primero en machine encoding y luego entrar a esta ruta.

## Fixtures

- `examples/machine/jit_return42_x86_64.nm`
- `examples/machine/jit_extern_rejected_x86_64.nm`
- `examples/machine/jit_reloc_rejected_x86_64.nm`

## Comando de evidencia

```text
cargo test -p ncc-cli --test machine_closure machine_run_bytes -- --nocapture
```
