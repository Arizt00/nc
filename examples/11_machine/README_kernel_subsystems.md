# Ejemplo de subsistemas kernel

`kernel_subsystems_compact.n` conserva la sintaxis kernel compacta y pasa por
la ruta normal del compilador.

## Ejecutar todo el grafo fuera de QEMU

```powershell
ncc machine kernel-system selftest --target x86_64-unknown-none --bundle reports/kernel-system/host
ncc machine kernel-system verify reports/kernel-system/host
```

Esto ejecuta procesos, ELF, COW, scheduler, VFS, almacenamiento, PCIe, red,
USB/HID y framebuffer dentro del motor nativo de referencia. El bundle
contiene bytes, imagenes y hashes verificables.

## Construir ARMv7-A

```powershell
ncc build examples/11_machine/kernel_subsystems_compact.n --target armv7a-unknown-none-eabi --profile kernel --json
```

Artefactos principales:

- `.ncc/code/*.nm`;
- `.ncc/obj/*.obj` y `*.body.o`;
- `.ncc/bin/*.kernel.elf`;
- `.ncc/run/*.map` y `*.json`.

## Construir firmware Cortex-M

```powershell
ncc build examples/11_machine/kernel_subsystems_compact.n --target armv7em-none-eabi --profile kernel --json
```

Se generan `.firmware.bin` e Intel HEX. El binario esta enlazado para flash
`0x08000000`; el HEX ya incluye direcciones. La placa debe tener al menos el
mapa de SRAM declarado por el reporte o el build se rechaza.

Los dos builds se verifican sin QEMU. El arranque fisico solo queda certificado
despues de flashear una placa identificada y retener su log.
