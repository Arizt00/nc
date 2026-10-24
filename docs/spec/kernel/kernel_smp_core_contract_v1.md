# Kernel SMP Core Contract v1

Estado: `closed` para el nucleo SMP logico, semantico, nativo y conectado a
los protocolos fisicos definidos en `kernel_physical_smp_multiarch_v1.md`.

## Superficie tipada

CPU y almacenamiento por CPU:

- `cpuRegister(id:int, bootstrap:bool, stack:addr) -> bool`
- `cpuStart(id:int) -> bool`
- `cpuOnline(id:int) -> bool`
- `cpuCurrent() -> int`
- `cpuCount() -> int`
- `cpuOnlineCount() -> int`
- `cpuSetCurrent(id:int) -> bool`
- `cpuLocalSet(id:int, slot:int, value:addr) -> bool`
- `cpuLocalGet(id:int, slot:int) -> addr`
- `cpuStack(id:int) -> addr`

Scheduler multicore:

- `schedulerBind(pid:int, cpu:int) -> bool`
- `schedulerNextOn(cpu:int) -> bool`
- `schedulerCurrentOn(cpu:int) -> int`

IPI:

- `ipiSend(cpu:int, vector:int) -> bool`
- `ipiBroadcast(vector:int) -> int`
- `ipiPending(cpu:int) -> int`
- `ipiPeek(cpu:int) -> int`
- `ipiAck(cpu:int, vector:int) -> bool`

Las formas cualificadas `kernel.cpu.core.*`, `kernel.ipi.core.*` y
`kernel.schedule.core.*` son equivalentes. No requieren imports en perfil kernel.

## Invariantes

- Hasta 64 CPUs registradas y un unico bootstrap processor.
- Un AP registrado queda offline hasta `cpuStart`.
- 32 slots `addr` por CPU.
- Afinidad opcional por tarea; el scheduler por CPU solo elige tareas compatibles.
- Cada CPU tiene una cola IPI FIFO de 256 entradas.
- Vectores IPI validos: 0..255.
- Broadcast entrega a CPUs online salvo la CPU actual.
- `ipiAck` solo consume la cabeza esperada, preservando orden FIFO.

Hosted runtime y LLVM kernel implementan los mismos limites, retornos y estados.
La ruta LLVM usa almacenamiento kernel nativo, llamadas tipadas y relocaciones
resueltas en el objeto final.

## Matriz material

`kernel_operational_smp.n` compila por `.n -> LLVM/MCT -> .nm + .obj -> imagen`
en once targets:

- x86_64 freestanding, Linux, Windows y macOS;
- AArch64 freestanding y Linux;
- RISC-V 64 y RV32IMAC;
- i386 freestanding y Linux;
- WASM32.

Para x86_64 freestanding el build retenido materializa `.nm`, objeto de entrada,
LLVM del cuerpo, objeto nativo del cuerpo, ELF kernel y boot image. Su informe
registra 452 relocaciones aplicadas y cero simbolos sin resolver.

## Extension fisica

La superficie `*Physical` es el contrato actual para hardware real:

- `smpDiscover()` normaliza MADT/SRAT o FDT;
- `smpConfigureFirmware()` enlaza firmware, controlador y temporizador;
- `cpuSetStackPhysical()` publica la pila independiente del procesador;
- `cpuStartPhysicalWait()` usa INIT/SIPI, PSCI CPU_ON o SBI HSM;
- `cpuMarkOnline()` publica la CPU con orden release;
- `ipiSendPhysical()` y `tlbShootdown()` operan sobre CPUs online;
- `smpInterruptInitFirmware()` inicia APIC/x2APIC, GICv3, PLIC o IMSIC.

El estado de implementacion material es `closed`. El estado de una placa concreta
solo puede ser `certified` despues de flash/reset/captura UART con identidad y serie
reales. Sin esa ejecucion, el estado correcto es `validated_not_executed`.

Autoridad detallada:

```text
docs/spec/kernel/kernel_physical_smp_multiarch_v1.md
```
