# Machine virtual devices, PCI and VirtIO v1

Estado: cerrado para el perfil de hipervisor tipado declarado.

## Autoridad

La autoridad vive en:

- `crates/ncc-machine/src/virtual_devices.rs` para UART, PS/2, bloque, red y
  framebuffer;
- `crates/ncc-machine/src/pci_virtio.rs` para PCI, MSI/MSI-X, transporte VirtIO,
  split virtqueues y backends VirtIO.

Las operaciones reciben memoria invitada, direcciones y buffers explicitos. El
modelo no crea hilos, sockets, archivos ni scheduler de manera oculta.

## Dispositivos base

`Uart16550` implementa registros DLAB, divisor, LSR, IIR, FCR, FIFO RX/TX,
loopback, scratch, interrupciones y reset. `Ps2Controller` implementa status,
command byte, output port, comandos del controlador, comandos de teclado,
scancode set 2, press/release, ACK y reset.

`VirtualBlockDevice` cubre lectura/escritura sectorial, flush, discard,
write-zeroes, readonly, bounds y contador de generacion. `VirtualNetworkDevice`
mantiene MAC, MTU, link, filtros, colas RX/TX y contadores de bytes, paquetes,
drops y errores. `VirtualFramebuffer` soporta RGBA8888/BGRA8888/RGB565, stride,
pixels, fill, copy, dirty rectangles, checksum y reset.

## PCI

Cada funcion conserva 4096 bytes de configuracion. Se validan segmento, bus,
device, function y accesos alineados de 1/2/4 bytes. La superficie cubre:

- mecanismo PCI configuration #1 por `0xcf8/0xcfc`;
- ECAM mediante la ABI kernel;
- BAR IO, MMIO32 y MMIO64, incluido size probing;
- command/status, capability list y header type;
- MSI y MSI-X con mask, pending bits y tabla tipada;
- attach, detach, lookup y enumeracion por BDF.

## VirtIO moderno

`VirtioPciTransport` aplica el ciclo ACKNOWLEDGE, DRIVER, FEATURES_OK y
DRIVER_OK. Negocia VERSION_1, INDIRECT_DESC, EVENT_IDX y ACCESS_PLATFORM,
mantiene ISR, config generation, queue selection y notifications.

Las split virtqueues conservan descriptor/avail/used, indices wrapping y cadenas
directas o indirectas. Rechazan indices invalidos, ciclos, NEXT+INDIRECT,
direcciones fuera de memoria y escrituras contra descriptores readonly.

| Backend | Operaciones materiales |
|---|---|
| block | IN, OUT, FLUSH, GET_ID, DISCARD, WRITE_ZEROES y status byte |
| network | TX/RX con cabecera VirtIO, MTU, link y contadores |
| console | RX/TX contra UART virtual |
| input | eventos EV_KEY y EV_SYN de 8 bytes |
| GPU | resource 2D, backing, scanout, transfer-to-host y flush |

## Evidencia

Las pruebas unitarias validan configuracion PCI, BAR, MSI/MSI-X, negociacion,
colas directas/indirectas, deteccion de ciclos y movimiento exacto de bytes por
los cinco backends. La certificacion CPU-backed complementaria usa `/dev/kvm` y
QEMU/QMP; su hash y artefactos estan en la resolucion de esta fase.

## Frontera

Este contrato modela dispositivos virtuales y su transporte. Un driver de host
fisico concreto sigue necesitando su propio BAR, DMA/IOMMU, IRQ y certificado de
placa. La red de la prueba no transmite fuera del backend controlado.
