# Machine kernel storage persistence ISA v1

Estado normativo: activo.

Las rutas de persistencia y DMA pueden escribir instrucciones directamente en
`.nm` o enviarlas por stdin desde PowerShell, CMD, Bash o WSL.

## x86/x86_64

`clflush`, `clflushopt`, `clwb`, `sfence`, `mfence` y `lfence` materializan
bytes propios del encoder, incluidas formas de memoria ModRM/SIB.

## AArch64

`dc cvau`, `dc cvac`, `dc civac`, `dc ivac`, `dc zva`, `ic ivau`, `dsb`, `dmb`
e `isb` materializan las palabras A64 correspondientes. El registro de
direccion siempre es explicito.

## RISC-V RV32/RV64

Zicbom/Zicboz incorpora `cbo.clean`, `cbo.flush`, `cbo.inval` y `cbo.zero` con
operando `(reg)` o `0(reg)`. Desplazamientos distintos de cero se rechazan
porque no forman parte de la codificacion CBO.

Bytes de referencia:

```text
cbo.clean (a0) -> 0f 20 15 00
cbo.flush (a1) -> 0f a0 25 00
cbo.inval (a2) -> 0f 20 06 00
cbo.zero  (a3) -> 0f a0 46 00
dc zva, x4      -> 24 74 0b d5
```

El coordinador declara `cache-maintenance` como feature requerida por storage,
block cache, FAT, ext2, NVMe y AHCI.
