# SMP fisico multi-arquitectura v1

Fecha: 2026-07-22

## Estado y alcance

Estado de implementacion: `closed_material_pipeline`.

Estado de certificacion de placa: `validated_not_executed`.

Este contrato cubre x86_64, AArch64, RISC-V 64 y las variantes de objeto que
comparten la infraestructura freestanding. La sintaxis fuente sigue siendo `.n`
compacta, sin imports ni `[@kernel]` obligatorios:

```nc
kernelPhysicalSmpAarch64 {
    prepare :: in() -> bool {
        guard bootValid() && smpDiscover() else return false;
        guard smpConfigureFirmware() else return false;
        return smpInterruptInitFirmware(0);
    }

    ref main ->> prepare;
}
```

## Topologia y memoria por CPU

`smpDiscover()` consume el descriptor de firmware canonico y publica hasta 256
procesadores con:

- id logico, id de hardware e id de firmware;
- nodo NUMA, package, core y thread;
- procesador bootstrap, enabled y estado fisico;
- pila independiente alineada y base local por CPU.

x86 obtiene APIC/x2APIC y jerarquia desde MADT, SRAT y CPUID 0x1F/0x0B. AArch64
usa MPIDR y nodos CPU de ACPI/FDT. RISC-V usa hart ids y FDT. La asignacion tipada
de pilas detecta overflow y solapamiento; el modo raw permite que el programador
entregue direcciones fisicas propias mediante `cpuSetStackPhysical`.

## Arranque x86_64 INIT/SIPI

El backend contiene una imagen AP propia de 314 bytes. No invoca ensamblador externo
durante el build. La secuencia es:

1. reclama un buzon global para impedir dos arranques simultaneos;
2. copia la imagen real16/protected32/long64 a `0x7000`;
3. publica en `0x6000` generacion, CPU, pila, entry, contexto, CR3 y callback;
4. emite INIT assert, espera 10 ms, INIT deassert y dos SIPI con vector 7;
5. el AP carga GDT temporal, activa PAE/LME/paging y entra en modo largo;
6. captura entry/contexto antes de liberar el buzon y llama a la funcion `.n`;
7. `cpuMarkOnline(cpu)` publica el estado release; si la funcion retorna, el AP
   pasa a `parked` y ejecuta `cli/hlt`.

El CR3 del contrato de boot propio reside bajo 4 GiB y mapea identidad la ventana
de arranque. El kernel puede instalar despues GDT/TSS/IDT por CPU con
`x86TablesLoad(gdt, idt, tssSelector)`. `ncc-machine` define layouts exactos para
GDT, descriptor TSS de 16 bytes, IDT gates y TSS64 de 104 bytes.

## Arranque AArch64 PSCI

El descriptor de plataforma fija el conduit SMC o HVC. El backend emite
`PSCI_CPU_ON_64` y consulta `PSCI_AFFINITY_INFO_64` hasta `ON`, error terminal o
timeout. La entrada conserva el contexto firmware y cada CPU publica su id en
`TPIDR_EL1` mediante `cpuMarkOnline`.

GICv3 inicia Distributor y Redistributor, espera RWP, despierta el redistributor
correspondiente al MPIDR, configura SGI/PPI y habilita la interfaz de registros de
sistema. `gicEnable`, `gicAck` y `gicEoi` materializan prioridad, trigger, IAR1 y
EOIR1. El temporizador fisico usa CNTFRQ/CNTV y el PPI 27.

## Arranque RISC-V SBI HSM

El backend emite SBI HSM `HART_START` y consulta `HART_GET_STATUS` hasta `STARTED`,
error terminal o timeout. El hart conserva `a0=hartid`, `a1=opaque/context` y publica
el id logico en `tp`.

El descriptor FDT selecciona PLIC, IMSIC, APLIC o combinaciones. PLIC implementa
priority, enable, threshold, claim y complete con limites de source/context. IMSIC
programa EIDELIVERY, EITHRESHOLD, EIE y STOPEI mediante CSR indirectos. Los alias
machine `csrr` y `csrw` se codifican internamente como CSRRS/CSRRW y no requieren
assembler externo. El temporizador e IPI usan SBI TIME/IPI.

## IPI, timers y TLB shootdown

`ipiSendPhysical(cpu, vector)` publica primero el vector con release y despues
activa APIC SGI o SBI IPI. El receptor usa `ipiTakePhysical` con exchange atomico.
RV32 materializa su helper de exchange de 64 bits dentro del kernel, sin `libatomic`.

`timerArmPhysical(deadline, vector)` valida las restricciones de cada arquitectura:
TSC deadline en x86, CNTV PPI 27 en AArch64 y supervisor timer interrupt 5 en
RISC-V.

`tlbShootdown(address, length, asid, vector)`:

- serializa una generacion con lock atomico;
- invalida localmente;
- publica rango/ASID y mascara de CPUs online;
- envia IPI y espera acknowledgement de todos los destinos;
- usa INVLPG, TLBI o SFENCE.VMA segun target;
- falla por timeout y libera siempre el lock.

## Artefactos fisicos

Los builds de referencia generan fuera del IDE y sin ejecutar QEMU:

| Target | Artefactos de consumo fisico |
| --- | --- |
| x86_64 | ELF, BIOS image, Multiboot2 ELF, BOOTX64.EFI, GPT/ESP y AP trampoline |
| AArch64 | ELF, BOOTAA64.EFI y GPT/ESP |
| RISC-V 64 | ELF y payload plano OpenSBI |

Comandos:

```bash
ncc build examples/11_machine/kernel_physical_smp_x86.n --profile kernel --target x86_64-unknown-none
ncc build examples/11_machine/kernel_physical_smp_aarch64.n --profile kernel --target aarch64-unknown-none
ncc build examples/11_machine/kernel_physical_smp_riscv.n --profile kernel --target riscv64-unknown-elf
```

## Verificacion permanente

```bash
cargo test -p ncc-machine smp::tests
cargo test -p ncc-ir kernel_firmware_smp_llvm::tests
cargo test -p ncc-cli --test kernel_firmware_artifact_isolation
cargo test -p ncc-cli --test kernel_physical_smp_artifacts
cargo test -p ncc-cli --test machine_physical_board_certification
```

La prueba `kernel_physical_smp_artifacts` construye los tres ejemplos, inspecciona
LLVM/objeto/ELF, verifica UEFI/Multiboot2/OpenSBI y compara bytes y SHA-256 del
trampoline. No lanza QEMU.

## Limite de verdad fisica

Un ELF, una EFI o un payload correcto no demuestra por si solo una placa concreta.
`ncc machine certify-board` exige vendor, modelo, revision, serial de placa, serial
de probe, hash del artefacto, comandos argv de flash/reset/captura y marcadores UART
ordenados. Rechaza QEMU, TCG y simuladores. `--validate-only` nunca emite una
atestacion fisica.
