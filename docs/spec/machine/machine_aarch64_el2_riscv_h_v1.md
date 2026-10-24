# Virtualización AArch64 EL2 y RISC-V H v1

## AArch64 EL2

`ncc-machine::aarch64_el2` tipa los registros de control, traducción y salida:
`HCR_EL2`, `VTCR_EL2`, `VTTBR_EL2`, `ESR_EL2`, `FAR_EL2`, `HPFAR_EL2`,
`ELR_EL2`, `SPSR_EL2`, `VBAR_EL2`, `CNTHCTL_EL2`, `CNTVOFF_EL2` y compañía.
Cada entrada conserva op0/op1/CRn/CRm/op2 y acceso permitido.

La capa incluye:

- codificación `MRS`, `MSR`, `HVC`, `ERET` y `TLBI VMALLS12E1IS`;
- granos stage-2 de 4 KiB, 16 KiB y 64 KiB;
- validación de IPA, PA, alineación, niveles y concatenación;
- descriptores block/page con memoria, shareability, AF, XN y permisos;
- `VTTBR_EL2` con VMID y base física;
- clasificación de `ESR_EL2` y snapshot material de salida.

El lowering kernel emite assembly inline con clobber de memoria y las barreras
`DSB ISH`/`ISB` necesarias alrededor de invalidación y cambio de contexto.

## RISC-V Hypervisor extension

`ncc-machine::riscv_h` tipa los CSR `hstatus`, `hedeleg`, `hideleg`, `hie`,
`hcounteren`, `hgeie`, `htval`, `hip`, `hvip`, `htinst`, `hgatp`, `hgeip`,
`vsstatus`, `vsie`, `vstvec`, `vsscratch`, `vsepc`, `vscause`, `vstval`,
`vsip`, `vsatp` y los registros de temporizador relacionados.

También codifica y valida:

- `HFENCE.GVMA` y `HFENCE.VVMA`;
- `HLV.B/BU/H/HU/W/WU/D`;
- `HLVX.HU/WU`;
- `HSV.B/H/W/D`;
- `HGATP` Sv32x4, Sv39x4, Sv48x4 y Sv57x4;
- PTE G-stage y snapshots `scause/sepc/stval/htval/htinst/hstatus`.

Los registros x0..x31 se validan antes de codificar y las raíces G-stage exigen
la alineación x4 de 16 KiB definida por la extensión H.

## Lowering material

El archivo normal `.n` usa intrínsecos compactos sin import hosted:

```n
hcr >> int = el2_read_hcr();
el2_write_vttbr(root);
el2_tlbi_all();

hstatus >> int = riscv_h_read_hstatus();
riscv_h_write_hgatp(root);
riscv_h_hfence_gvma(0, 0);
```

Con perfil kernel, parser, sema e IR resuelven firmas y símbolos internos. El
backend genera objetos ELF reales para `aarch64-unknown-none`,
`riscv64-unknown-elf` y `riscv32imac-unknown-elf`, sin imports finales. En RV32,
las operaciones atómicas de 64 bits del runtime freestanding se serializan con
un lock i32 interno y no arrastran `libatomic`.

Un intrínseco privilegiado solo es ejecutable en la arquitectura y nivel de
privilegio que lo define. KVM/HVF son la vía hosted; un kernel/bare-metal en EL2
o HS es la vía directa.

## Autoridades externas

- [Arm virtualization documentation through Hypervisor.framework](https://developer.apple.com/documentation/hypervisor)
- [RISC-V Hypervisor extension](https://docs.riscv.org/reference/isa/priv/hypervisor)
