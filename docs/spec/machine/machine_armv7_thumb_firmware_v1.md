# Machine ARMv7, Thumb-2 y firmware v1

## Perfiles

| Destino ncc | ISA | ABI | Objeto | Salida |
|---|---|---|---|---|
| `armv7a-unknown-none-eabi` | ARMv7-A + Thumb-2 | AAPCS32 kernel | ELF32 ARM EABI5 | ELF32 `ET_EXEC` |
| `armv7em-none-eabi` | Thumb-2 Cortex-M4 | AAPCS32-M | ELF32 ARM EABI5 | `.bin` + Intel HEX |

Los triples internos de Clang (`armv7a-none-eabi` y
`thumbv7em-none-eabi`) son aliases del mismo contrato de machine.

## Encoder y relocaciones

El encoder tipado cubre registros `r0..r12`, `sp`, `lr`, `pc`, registros de
estado Cortex-M, ALU, loads/stores, control flow, SVC, barreras, atomics
exclusivos, interrupciones y espera.

El objeto y el linker aplican:

- `R_ARM_ABS32` y `R_ARM_REL32`;
- `R_ARM_THM_CALL` y `R_ARM_THM_JUMP24`;
- `R_ARM_MOVW_ABS_NC` y `R_ARM_MOVT_ABS`;
- `R_ARM_PREL31`.

ELF32 ARM usa relocaciones `REL`. El addend queda codificado dentro de
`.text`; una llamada Thumb con addend logico cero conserva `-4`, por el sesgo
arquitectural del PC. El objeto ya no depende del sidecar de ncc para
roundtrip con herramientas externas.

## ABI freestanding

El runtime aporta dentro del mismo objeto:

- `__aeabi_memclr*`, `__aeabi_memset*`, `__aeabi_memcpy*` y
  `__aeabi_memmove*`;
- `__aeabi_uidiv` y `__aeabi_uidivmod` sin division recursiva;
- personalidades EHABI requeridas por objetos ARM;
- `__atomic_*_8` para Cortex-M mediante lock atomico de 32 bits y pares de
  palabras.

No se enlazan libc, compiler-rt ni libatomic.

## Imagen Cortex-M

El finalizador produce una imagen fisica, no un contenedor NCCLINK:

- flash base `0x08000000`;
- SRAM base `0x20000000`, perfil de referencia de 512 KiB;
- tabla de 16 vectores;
- reset handler Thumb;
- copia byte a byte de `.data` desde flash a SRAM;
- limpieza de `.bss`;
- barreras DMB/ISB y llamada AAPCS32 a `kmain`;
- `wfi` estable si `kmain` retorna;
- rechazo si `.data + .bss` invade la pila inicial.

La salida incluye `.bin`, `.hex`, mapa y reporte JSON con hashes y layout.

## Verificacion

```text
ncc build examples/11_machine/kernel_subsystems_compact.n --target armv7a-unknown-none-eabi --profile kernel --json
ncc build examples/11_machine/kernel_subsystems_compact.n --target armv7em-none-eabi --profile kernel --json
ncc machine emit-object examples/machine/thumbv7em_return42.nm --target armv7em-none-eabi --out return42.o
```

La suite compara bytes Thumb contra un fixture ensamblado por Clang y ejecuta
el subconjunto determinista en el executor de bytes sin QEMU. Flashear y
arrancar una placa concreta necesita su transporte y se certifica por
separado.
