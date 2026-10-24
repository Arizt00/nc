# Arnes de resistencia kernel v1

## Objetivo

`ncc-machine` incluye un arnes determinista que ejecuta algoritmos reales del
kernel y del hipervisor dentro de un proceso nativo. No sustituye una prueba en
hardware privilegiado; encuentra corrupcion, carreras, errores de prioridad y
regresiones de rendimiento antes de llegar a ese hardware.

## Escenarios

1. `vm-exit-storm`: un millon o mas de exits Intel/AMD, avance de RIP y orden
   NMI, excepcion e interrupcion externa.
2. `smp-contention`: hilos host simultaneos, atomicos `SeqCst` y confirmacion
   de TLB shootdown por cada vCPU remota.
3. `cow-page-faults`: fork COW, primera escritura, aislamiento padre/hijo y
   rechazo de una pagina no mapeada.
4. `invalid-dma`: traducciones IOVA validas, escritura denegada y direccion
   sin mapa mediante el vIOMMU y su cola de fallos.
5. `simultaneous-interrupts`: lotes llenos y entrega estricta por prioridad.
6. `snapshot-restore`: snapshots completos e incrementales con varios vCPU,
   memoria, dispositivos, ancestry y tiempo virtual.

Cada escenario publica operaciones, nanosegundos, operaciones por segundo,
rechazos esperados y checksum. El checksum SMP no depende del orden elegido
por el scheduler host.

## Perfiles

- `smoke`: desarrollo local rapido;
- `ci`: millones de eventos y memoria suficiente para detectar regresiones;
- `endurance`: carga prolongada para runners fisicos dedicados.

Todos los contadores se pueden sobrescribir de forma explicita. El perfil no
crea trabajo oculto ni ajusta automaticamente un test para hacerlo pasar.

```text
ncc machine kernel-system stress \
  --profile ci --bundle reports/kernel-stress

ncc machine kernel-system verify-stress reports/kernel-stress
```

Opciones: `--vm-exits`, `--vcpus`, `--smp-iterations`, `--cow-rounds`,
`--dma-rounds`, `--interrupt-events`, `--snapshot-rounds`,
`--snapshot-memory-bytes`, `--seed` y `--min-ops-per-second`.

## Evidencia

El bundle contiene:

- `kernel-stress-report.json`;
- `kernel-stress-summary.txt`;
- `kernel-stress-certificate.json`.

El certificado usa un dominio hash distinto al selftest funcional. El
verificador vuelve a calcular su payload, todos los archivos, el resumen y el
checksum. Cualquier byte alterado invalida la evidencia.

## Limites

El informe marca `qemu=false` y `physical_hardware_certified=false`. Prueba el
motor machine y la concurrencia nativa del host, pero VMX/SVM/EL2/H, DMA real y
dispositivos fisicos requieren los lanes de hardware del workflow exact-head.
Cada lane WHPX/KVM/HVF ejecuta ademas 256 ciclos reales del proveedor, verifica
y retiene cada certificado, y publica latencia total y media. Si un solo ciclo
no entra, sale o se verifica, el lane fisico falla antes del sellado.
