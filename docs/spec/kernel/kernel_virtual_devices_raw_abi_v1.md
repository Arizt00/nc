# Kernel virtual-device raw ABI v1

Estado: cerrado desde `.n` compacto hasta LLVM, objeto y enlace kernel.

## Superficie sin imports

| Intrinseco | Firma | Materializacion |
|---|---|---|
| `pci_config_read32` | `(int,int,int,int) -> int` | PCI config mechanism #1 en x86 |
| `pci_config_write32` | `(int,int,int,int,int) -> bool` | `0xcf8/0xcfc` en x86 |
| `pci_ecam_read32` | `(addr,int,int,int,int) -> int` | MMIO32 volatile |
| `pci_ecam_write32` | `(addr,int,int,int,int,int) -> bool` | MMIO32 volatile |
| `virtio_notify` | `(addr,int) -> bool` | MMIO16 volatile |
| `debug_break` | `() -> bool` | breakpoint nativo del target |

No requieren `use`, libc ni runtime hosted. El programador aporta ECAM base,
notify address, BDF, offset y queue index. No hay discovery, mapping, bounds,
locks, asignacion ni interrupciones ocultas.

## Arquitecturas

En x86/i386, PCI legacy compone enable, bus, device, function y offset y emite
PIO32 en `0xcf8/0xcfc`. En AArch64 y RISC-V PCI legacy devuelve unsupported de
forma explicita; ECAM sigue siendo el camino portable y material.

`debug_break` baja a `int3`, `.inst 0xd4200000` o
`.word 0x00100073` para x86, AArch64 y RISC-V. `virtio_notify` reutiliza el store
MMIO16 del core kernel en todos esos targets.

## Continuidad

El fixture `examples/11_machine/kernel_virtual_devices_pci_virtio.n` pasa:

```text
ncc check examples/11_machine/kernel_virtual_devices_pci_virtio.n
ncc build examples/11_machine/kernel_virtual_devices_pci_virtio.n \
  --target x86_64-unknown-none --profile kernel
```

El manifest exige cuerpo high-level completo, `kmain` enlazado y cero simbolos
sin resolver. La prueba integrada comprueba que las seis funciones estan
definidas dentro del LLVM retenido y no sobreviven como declaraciones externas.

## Regla raw

La ABI no promete seguridad. Direcciones PCI/ECAM/notify incorrectas, accesos
fuera de dispositivo y breakpoints sin debugger conservan el comportamiento
material del target. Esa libertad no introduce runtime hosted en kernel.
