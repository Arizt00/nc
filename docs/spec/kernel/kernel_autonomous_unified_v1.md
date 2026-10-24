# Kernel autonomous unified image v1

Estado normativo: activo.

## Imagen

`.nki` contiene seis componentes alineados a 4096 bytes: target, ELF kernel,
filesystem raiz, ELF `init`, manifiesto machine y command line. La cabecera
registra version, offsets, longitudes, CPU, frames, formato raiz y modo
read-only/read-write. Cada componente usa CRC32 y el bundle retiene SHA-256.

El loader rechaza versiones desconocidas, offsets fuera de rango,
solapamientos, desalineacion, duplicados, CRC incorrecto, ELF ausente o un
manifiesto que no corresponda al target.

## Arranque

`AutonomousKernel::boot` consume exclusivamente la imagen:

1. coordina todo el grafo machine del target;
2. valida los ELF y su `e_machine`;
3. crea memoria fisica y scheduler multicore;
4. monta FAT o ext2 como `/root`;
5. inicializa PCIe, NVMe, red, USB/HID y framebuffer;
6. carga `init` en espacio usuario y agenda las CPU;
7. materializa y sincroniza `/root/var/log/boot.log` si la raiz es escribible.

`tick_all`, `sync_root` y `shutdown` son llamadas manuales. `sync_root` ejecuta
journal, escribe todos los sectores al namespace NVMe y emite Flush. No hay
scheduler, heap, cleanup o flush ocultos.

## CLI

```text
ncc machine kernel-system autonomous --kernel kernel.elf --rootfs root.ext2 --root-format ext2 --init init.elf --target x86_64-unknown-none --frames 4096 --cpus 4 --bundle evidence --out kernel.nki
ncc machine kernel-system verify-autonomous evidence
```

`--root-readonly` desactiva persistencia. FAT acepta `fat`, `fat12`, `fat16` o
`fat32`; la geometria real de la imagen determina la variante.

## Frontera material

El comando ejecuta el kernel integrado dentro del motor nativo de referencia,
fuera de QEMU, y verifica una unica imagen. No afirma que la CPU haya entrado a
esa imagen desde UEFI, Multiboot, OpenSBI o una placa. Esa entrada fisica usa
los loaders y certificados de hardware ya separados.
