# Kernel physical drivers v1

Estado normativo: activo.

## Contrato

Los drivers reciben memoria, mappings MMIO/PIO, colas DMA y limites de espera
del caller. No crean mappings, no reservan DMA y no habilitan interrupciones de
forma oculta.

La superficie material incluye:

- UART 16550 y PL011;
- framebuffer caller-owned BGRA/RGBA/RGB565;
- teclado PS/2 y parser HID existente;
- xAPIC, GICv3 y PLIC;
- HPET, AArch64 CNTV y RISC-V `stimecmp`;
- PCIe ECAM;
- entradas NVMe de 64 bytes y colas admin/doorbells;
- command list AHCI de 1024 bytes y received FIS de 256 bytes;
- TRB xHCI de 16 bytes;
- VirtIO MMIO, red y notificaciones de cola;
- timeout, estado degradado, reset, fallo inyectado y recuperacion.

`VolatileMmio` ejecuta lecturas y escrituras volatiles reales sobre un mapping
entregado explicitamente. Su constructor es `unsafe`: el programador responde
por cacheability, permisos, vida y correspondencia fisica.

## Machine

x86_64 e i386 incorporan `insb`, `insw`, `insd`, `outsb`, `outsw` y `outsd`.
Las formas no tienen operandos explicitos porque consumen DX y RDI/RSI segun la
ISA. Sus bytes son `6c`, `66 6d`, `6d`, `6e`, `66 6f` y `6f`.

## Evidencia

`ncc machine kernel-system io-storage` ejecuta el contrato host, retiene un
informe y genera un certificado SHA-256 verificable. El informe diferencia de
forma obligatoria validacion host y ejecucion en hardware.

Este contrato no afirma acceso fisico desde un proceso de usuario. La
certificacion de placa requiere ejecutar el kernel con su manifest de board y
retener serial, framebuffer, identidad y hashes.
