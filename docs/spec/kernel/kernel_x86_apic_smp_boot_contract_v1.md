# Kernel x86 APIC, SMP and guest boot contract v1

Estado: cerrado para lowering kernel x86_64 freestanding y enlace material.

## Superficie compacta

El perfil kernel acepta estas operaciones sin `use`, heap o runtime hosted:

| Operacion | Firma | Materializacion x86_64 |
|---|---|---|
| `apic_read32` | `(addr: int) -> int` | load volatile i32 |
| `apic_write32` | `(addr: int, value: int) -> bool` | store volatile i32 |
| `apic_eoi` | `(base: int) -> bool` | store cero en `base + 0xb0` |
| `apic_send_ipi` | `(base, destination, command: int) -> bool` | ICR high `+0x310`, ICR low `+0x300` |
| `x2apic_read` | `(msr: int) -> int` | `rdmsr` material |
| `x2apic_write` | `(msr, value: int) -> bool` | `wrmsr` material |
| `ioapic_read` | `(base, register: int) -> int` | IOREGSEL y load IOWIN `+0x10` |
| `ioapic_write` | `(base, register, value: int) -> bool` | IOREGSEL y store IOWIN `+0x10` |
| `hpet_read64` | `(addr: int) -> int` | load volatile i64 |
| `hpet_write64` | `(addr, value: int) -> bool` | store volatile i64 |
| `tsc_read` | `() -> int` | cycle counter/`rdtsc` |
| `tsc_deadline_write` | `(value: int) -> bool` | `wrmsr 0x6e0` |

Los aliases cualificados `kernel.apic.*`, `kernel.ioapic.*` y `kernel.timer.*`
resuelven al mismo simbolo canonico. En targets no x86, estas primitivas no
inventan una emulacion hosted: devuelven el estado no soportado previsto por el
contrato multi-target.

## ABI y enlace

El ejemplo `examples/11_machine/kernel_apic_smp_guest_boot.n` baja por:

```text
.n -> sema -> IR -> LLVM freestanding -> body.o
   -> machine .nm -> machine .obj -> ELF64 -> boot.img
```

`kmain` usa `ncc-kernel-entry-sysv64-v1`, recibe `boot_info` en `rdi`, exige stack
de 16 bytes y no usa red zone. El cuerpo high-level llama las funciones materiales
del kernel core; no quedan declaraciones APIC sin definicion ni simbolos externos
sin resolver.

La construccion retenida contiene dos objetos de entrada, mapa, report de enlace,
ELF64 ET_EXEC e imagen BIOS stage0/stage1. El stage1 carga los segmentos `PT_LOAD`,
entrega memory map, serial, tabla de plataforma y llama la entrada ELF `kmain`.

## Regla raw

El compilador no habilita APIC, no escoge vectores, no reserva stacks de AP ni
crea vCPU. El programa kernel escribe direcciones y MSR explicitamente. Los
errores de plataforma, carreras, aliasing y accesos invalidos pertenecen al
programador raw y al hardware, igual que en C/ASM freestanding.

## Verificacion

La prueba integrada exige:

- lowering completo del cuerpo high-level;
- `kmain` enlazado con `main`;
- objeto de cuerpo material;
- definiciones LLVM APIC/IOAPIC/TSC deadline;
- cero declaraciones sustitutivas para esas funciones;
- cero simbolos sin resolver en phase12.

La prueba comparte matriz con VMCS/VMCB, EPT/NPT y VM-exit. Sus directorios
temporales usan PID, reloj y secuencia atomica, por lo que los builds paralelos no
se borran entre si en Windows.
