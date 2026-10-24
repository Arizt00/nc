# Subsistemas de kernel y coordinacion machine v1

## Autoridad

`ncc-machine` es la autoridad material del perfil kernel. Un backend no puede
declarar un subsistema si el perfil de destino no publica ABI, ancho de
puntero, registros, instrucciones, relocaciones y formato final compatibles.
El plan se conserva en LLVM, lowering JSON, manifest de build y reporte de
link.

## Grafo cerrado

El grafo completo contiene 35 nodos con dependencias ordenadas. Las familias
principales son:

1. memoria fisica;
2. memoria virtual;
3. ELF loader;
4. procesos y espacio usuario;
5. copy-on-write;
6. syscalls;
7. scheduler multicore y sincronizacion;
8. controladores de interrupciones, timers, serial y teclado;
9. VFS, initramfs, almacenamiento, block cache, FAT y ext2;
10. PCIe, NVMe, AHCI y VirtIO;
11. NIC, Ethernet, ARP, IPv4, IPv6, UDP, TCP, sockets y red integrada;
12. USB e HID;
13. framebuffer;
14. firmware.

`coordinate_target_kernel` selecciona el grafo completo en destinos con MMU,
espacio usuario y ELF. En Cortex-M selecciona memoria fisica, framebuffer y
firmware. No simula una MMU que el perfil no posee.

## Implementacion ejecutable

`kernel_core.rs` materializa:

- allocator de frames y espacios de direcciones;
- loader ELF32/ELF64 ET_EXEC/ET_DYN con relocaciones, TLS y stack SysV;
- procesos, copia entre kernel/usuario y COW en primera escritura;
- tabla de syscalls;
- scheduler multinivel por CPU, prioridades, preemption, timers, afinidad,
  sleep/wakeup, wait queues, locks y balanceo.

`kernel_io.rs` materializa:

- VFS con rutas, directorios, enlaces, handles, rename y sparse write;
- bloque sectorial, colas NVMe y AHCI;
- configuracion y enumeracion PCIe;
- la pila historica Ethernet, IPv4 y UDP con checksum;
- enumeracion USB, informes HID y transiciones de teclado;
- framebuffer con formatos de pixel, rectangulos sucios y checksum.

`kernel_network.rs` materializa NIC fisicas, anillos DMA, Ethernet/VLAN,
ARP, IPv4/IPv6, fragmentacion, NDP, UDP/TCP dual-stack, sockets y timers.
`kernel_sync.rs` materializa locks manuales, atomics, RCU, estado IRQ y
barreras por arquitectura.

Estas implementaciones forman el motor de referencia ejecutable de
`ncc-machine`. Los objetos bare-metal reciben el runtime LLVM freestanding y
las operaciones arquitecturales de su perfil, sin libc ni `stoud` hosted.

## Sintaxis compacta

Un archivo kernel sigue siendo `.n` normal. El perfil se elige en build:

```text
ncc build kernel.n --target x86_64-unknown-none --profile kernel
```

La entrada compacta es `main: funcion;`. No requiere `[@kernel]`, imports
hosted ni un bloque monolitico. MMIO conserva `at`, `load*` y `*32/*64/...`.

## Evidencia fuera de QEMU

```text
ncc machine kernel-system selftest --target x86_64-unknown-none --bundle reports/kernel-system/host
ncc machine kernel-system verify reports/kernel-system/host
```

El selftest ejecuta el grafo en un proceso nativo, mueve bytes reales por COW,
VFS, bloque, red, USB/HID y framebuffer, y genera certificado con SHA-256.
QEMU no participa.

La ejecucion en una placa sigue siendo una certificacion distinta: exige el
firmware, controlador de flash, mapa de memoria y perifericos de esa placa.
La ausencia de placa nunca se registra como ejecucion fisica satisfactoria.
