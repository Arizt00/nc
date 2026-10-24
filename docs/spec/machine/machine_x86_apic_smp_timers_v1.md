# Machine x86 APIC, SMP and timers v1

Estado: cerrado para el perfil de plataforma virtual x86/x86_64 declarado.

## Autoridad

La autoridad tipada vive en `crates/ncc-machine/src/x86_apic.rs`. El modelo no
depende de cadenas de ensamblador, heap, libc ni runtime hosted. Todos los campos
arquitectonicos se conservan como enteros de ancho fijo y los errores se devuelven
como `ApicError`.

## Local APIC

`LocalApic` implementa los dos modos arquitectonicos:

- xAPIC por pagina MMIO en `0xfee00000`;
- x2APIC por MSR desde `0x800`;
- registro `IA32_APIC_BASE` y bit BSP;
- ID, version, TPR, PPR, LDR, DFR, SVR y ESR;
- IRR, ISR y TMR de 256 vectores;
- acknowledge por prioridad, EOI y recalculo de prioridad;
- ICR xAPIC de 64 bits dividido en high/low;
- ICR x2APIC de 64 bits atomico.

`ApicInterruptCommand` valida, codifica y decodifica:

- fixed, lowest-priority, SMI, NMI, INIT y SIPI;
- destino fisico o logico;
- edge o level;
- assert o deassert;
- destino explicito, self, all-including-self y all-excluding-self.

## IOAPIC

`IoApic` implementa IOREGSEL/IOWIN, ID, version, arbitration ID y 24 entradas de
redireccion de 64 bits. Cada entrada conserva vector, delivery mode, destination
mode, polaridad, trigger, mask, destination y remote IRR.

El camino level-triggered mantiene `remote_irr` hasta EOI. El camino edge no lo
retiene. Una entrada enmascarada no entrega eventos.

## Multiprocesador

`ApicBus<VCPUS, EVENTS>` usa capacidades fijas y modela:

- topologia package/core/thread y APIC ID;
- BSP y procesadores de aplicacion;
- reset, INIT, wait-for-SIPI, running, halted y shutdown;
- seleccion de destinos fisicos, logicos y shorthand;
- cola independiente y acotada por vCPU;
- IPI, NMI, SMI, INIT, SIPI y rutas IOAPIC;
- entrega de temporizadores a la vCPU propietaria.

No crea vCPU, memoria ni hilos de manera oculta. El VMM o kernel aporta esos
recursos y consume los eventos.

## Temporizadores

La superficie incluye:

| Dispositivo | Contrato |
|---|---|
| LAPIC | one-shot, periodic y TSC-deadline; divisor, mask, initial/current count y expiraciones acumuladas |
| PIT 8254 | tres canales, seis modos, latch, read-back, acceso low/high/lo-hi, gate y output |
| HPET | contador principal de 64 bits, hasta 32 comparadores, one-shot/periodic, rutas IRQ y MSI/FSB |
| Virtual TSC | frecuencia, offset, escala racional y avance comprobado contra overflow |

La constante de TSC deadline es `IA32_TSC_DEADLINE_MSR = 0x6e0`.

## Evidencia material

Las pruebas unitarias comprueban roundtrip ICR xAPIC/x2APIC, INIT/SIPI, prioridad,
IRR/ISR/TMR, EOI, edge/level IOAPIC, remote IRR, todos los modos LAPIC declarados,
PIT, HPET y TSC escalado.

La certificacion CPU-backed usa `/dev/kvm` con irqchip y PIT dentro del kernel.
Retiene los marcadores `LTl`, `PTp`, `Ii` y `Oo` para LAPIC, PIT, MSI e IOAPIC,
respectivamente. La evidencia exact-head esta referenciada por
`RESOLUTION_machine_kernel_apic_smp_guest_boot_2026-07-20.md`.

## Frontera

Este contrato cierra la plataforma APIC virtual x86 declarada. No afirma que un
proceso userland pueda escribir MMIO/MSR privilegiados, ni sustituye la
certificacion bare-metal de un chipset concreto. La ejecucion Intel disponible es
KVM anidado bajo WSL2; AMD SVM y una placa fisica requieren esos hosts reales.
