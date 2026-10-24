# Kernel virtual memory material v1

Estado normativo: activo.

## Arquitecturas

La autoridad es `ncc-machine::virtual_memory`. Materializa entradas de 64 bits
en paginas de tablas de 4096 bytes para:

- x86_64 PML4 de cuatro niveles;
- AArch64 stage-1 con granulo de 4 KiB;
- RISC-V Sv39 y Sv48.

Cada perfil expone la raiz fisica y el valor exacto para `cr3`, `ttbr0_el1` o
`satp`. La imagen `page-tables.bin` contiene las tablas reales, no un modelo
JSON ni pseudocodigo.

## Operaciones

`HardwarePageTables` permite mapear, desmapear, proteger y traducir paginas de
4 KiB, 2 MiB y 1 GiB. Tambien reserva guard pages, clasifica page faults y crea
un address space hijo COW. Resolver un COW sustituye unicamente la hoja del
hijo y conserva la traduccion del padre.

No se crean allocators ni schedulers ocultos. El llamador proporciona raices,
paginas de tablas, direcciones fisicas, ASID y nuevos frames COW.

## Invalidacion SMP

Las secuencias locales son materiales:

- x86_64: `invlpg` e `invpcid`;
- AArch64: `dsb ishst`, `tlbi vaae1is`, `dsb ish`, `isb`;
- RISC-V: `sfence.vma`.

`VmTlbShootdown` registra generacion, rango, ASID, CPU objetivo y acuse. Una
operacion no esta completa hasta recibir todos los acuses de la misma
generacion.

## Certificado reproducible

```text
ncc machine kernel-system virtual-memory \
  --target x86_64-unknown-none --bundle target/evidence/kernel-vm
ncc machine kernel-system verify-virtual-memory target/evidence/kernel-vm
```

El bundle conserva `page-tables.bin`, informe y certificado. El verificador
recalcula SHA-256 de imagen, informe canonico y payload con dominio separado.

Este certificado demuestra materializacion y coherencia del subsistema. No
afirma que una CPU fisica haya cargado la raiz; esa afirmacion pertenece al
certificado de placa.
