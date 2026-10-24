# Machine stdin, ASM y WASM v1

Estado normativo: activo.

`ncc machine stdin` y su alias `pipe` leen todo el codigo desde la entrada
estandar. La ruta es independiente del shell y funciona con pipes de Bash,
CMD y PowerShell.

Sintaxis de entrada admitida:

- `nm` o `machine`;
- `nasm`, `masm`, `gas-intel`, `gas-att`, `fasm` y `llvm-mc`;
- `wasm` o `wasm-binary` para un modulo binario ya materializado.

Acciones de fuente: `check`, `report`, `emit-ir`, `emit-asm`, `emit-bytes`,
`run-bytes`, `emit-object`, `emit-elf`, `emit-coff`, `emit-macho`, `emit-wasm`,
`build-exe` y `roundtrip`. WASM binario admite `verify-object` y `disasm`.

Ejemplos:

```text
type program.nm | ncc machine stdin check --syntax nm
Get-Content -Raw program.asm | ncc machine stdin emit-coff --syntax nasm --out program.obj
cat program.s | ncc machine stdin build-exe --syntax gas-att --out program
cat module.wasm | ncc machine stdin disasm --syntax wasm --json
```

La traduccion ASM pasa por el perfil dialectal y despues por parser, sema,
encoder y verificador machine. No invoca NASM, MASM, GAS ni LLVM para producir
los bytes. `run-bytes` conserva aislamiento por proceso y transicion W^X
`RW -> RX`.

Para ASM importado, `--profile userland|kernel|privileged|baremetal|firmware|system`
se escribe en el contrato `.nm` antes de sema. Las instrucciones privilegiadas
solo se admiten en el perfil solicitado: el mismo flujo `ins*`/`outs*` compila
con `--profile kernel` y se rechaza con `--profile userland`.

Esta version incorpora `invpcid r64,m128` completo: validacion privilegiada,
feature `invpcid`, codificacion `66 0F 38 82 /r` y desensamblado.

El perfil x86_64 incorpora tambien `rdfsbase`, `rdgsbase`, `wrfsbase` y
`wrgsbase` para operandos r32/r64, feature `fsgsbase`, encoder y decoder. La
ruta kernel acepta ELF binario por stdin mediante `kernel-system load-elf`.

El perfil kernel incorpora transferencias de puerto en bloque `insb`, `insw`,
`insd`, `outsb`, `outsw` y `outsd`, con encoder y decoder. El ejemplo
`examples/machine/kernel_physical_port_io_x86_64.nm` puede enviarse sin archivo
temporal desde PowerShell, CMD o Bash. `kernel-system io-storage` certifica los
contratos de drivers y almacenamiento y `verify-io-storage` verifica el bundle.

La ampliacion de persistencia incorpora `dc zva` en AArch64 y
`cbo.clean/flush/inval/zero` en RV32/RV64. El detalle de formas y bytes esta en
`machine_kernel_storage_persistence_isa_v1.md`.
