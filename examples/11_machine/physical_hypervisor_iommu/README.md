# Ejecucion fisica de hipervisor e IOMMU

Estos ejemplos conservan la sintaxis kernel compacta y no necesitan
`[@kernel]` ni imports hosted:

- `../kernel_x86_virtualization_vmcs_vmcb.n`: VMCS/VMCB y VMX/SVM;
- `../kernel_guest_memory_vmexit_loop.n`: EPT/NPT, eventos y VM-exit;
- `../kernel_apic_smp_guest_boot.n`: APIC, temporizadores y vCPU;
- `../kernel_portable_hypervisor_iommu.n`: EL2, RISC-V H e IOMMU/DMA.

## Construccion de imagen fisica

```text
ncc build ../kernel_x86_virtualization_vmcs_vmcb.n --profile kernel --target x86_64-unknown-none
ncc build ../kernel_portable_hypervisor_iommu.n --profile kernel --target aarch64-unknown-none
ncc build ../kernel_portable_hypervisor_iommu.n --profile kernel --target riscv64-unknown-elf
```

La salida contiene `.nm`, objeto e imagen kernel. x86_64 y AArch64 generan
ademas una aplicacion EFI y discos FAT/GPT; RISC-V genera el payload OpenSBI.
La carga en una placa usa el contrato de boot y el manifiesto de placa
correspondiente. Estos artefactos son ejecutables fuera de QEMU; su arranque
fisico requiere firmware y hardware de la arquitectura indicada.

## Certificacion sin QEMU

```text
ncc machine virtualization probe-host --json
ncc machine virtualization certify-host --out hypervisor.json
ncc machine virtualization verify-certificate hypervisor.json --require-bare-metal --json
```

Este camino abre `/dev/kvm` directamente. No inicia QEMU y el certificado
rechaza TCG, falta de aceleracion, timeout no recuperado o host virtualizado
cuando se exige `--require-bare-metal`.

## Passthrough

El esquema `../../../tools/release/vfio-dma-exercise.schema.json` describe el
protocolo MMIO del dispositivo asignado. Se necesita un BDF que no use el host,
todo su grupo ligado a `vfio-pci` y una secuencia documentada por el fabricante.

```text
ncc machine virtualization probe-iommu --json
ncc machine virtualization certify-vfio --group 12 --device 0000:03:00.0 --exercise device.json --out vfio.json
ncc machine virtualization verify-iommu-certificate vfio.json --json
```

No se incluye un BDF ficticio como ejemplo ejecutable: escribir offsets de un
dispositivo distinto puede bloquearlo o corromper datos. El esquema completo y
el certificador son genericos; la secuencia `device.json` pertenece al hardware
real del runner.
