# Kernel x86 VM-exit, exceptions and interrupts v1

Estado: cerrado para Intel VMX y AMD SVM en el modelo tipado, lowering x86_64,
bucle reusable y ejecucion CPU-backed mediante KVM.

## Snapshots ABI

`IntelVmExitSnapshot` usa once campos `u64`: reason, instruction length,
qualification, GPA, GLA, interruption information/error, IDT vectoring
information/error, RIP y RFLAGS.

`AmdVmExitSnapshot` usa siete campos `u64`: exit code, EXITINFO1/2, exit
interrupt information, NRIP, RIP y RFLAGS. Los offsets leidos del VMCB siguen el
layout AMD de control area y state-save.

Los decoders preservan el codigo raw y producen `VmExitKind`. Cubren excepcion,
NMI, interrupcion externa, triple fault, CPUID, HLT, IO, CR/DR, MSR, EPT
violation/misconfiguration, nested page fault, hypercall, APIC/AVIC, security y
razones futuras como `Unknown`.

## Cola de eventos

`PendingVmEvents<N>` tiene capacidad fija y no usa heap. Prioriza NMI,
excepciones, interrupciones externas y eventos software, conservando FIFO dentro
de la misma prioridad.

Un evento pasa por estos estados:

1. pendiente;
2. seleccionado para entrega;
3. inyectado al siguiente VM-entry;
4. confirmado, reencolado por entry failure o reconstruido desde IDT vectoring.

Si la ventana esta cerrada, el evento vuelve a la cola y el engine solicita
interrupt-window o NMI-window. No se descarta antes de confirmar VM-entry.

## Excepciones durante entrega

El motor aplica la matriz arquitectonica x86:

- evento benigno seguido de excepcion: se entrega la nueva excepcion;
- contributory seguido de contributory/page fault: `#DF`;
- page fault seguido de contributory/page fault: `#DF`;
- fallo durante entrega de `#DF`: triple fault;
- triple fault: termina el bucle con estado explicito.

La informacion de IDT vectoring y el error code se validan. Una interrupcion ya
consumida por el algoritmo de entrega no se reinyecta por segunda vez cuando el
handler devuelve `Unhandled`.

## Bucle

`run_vm_exit_loop` recibe un `VmExecutionEngine`, un `VmExitHandler`, una cola de
eventos y un presupuesto no nulo. El handler elige:

- `Resume`;
- `Advance`;
- `Inject`;
- `AdvanceAndInject`;
- `Yield`;
- `Halt`;
- `Shutdown`;
- `Unhandled`.

RIP solo avanza por accion explicita. El resultado informa salidas, eventos
inyectados, instrucciones avanzadas, ultima salida y causa de parada.

## ABI kernel raw

| Funcion | Uso |
|---|---|
| `vmexit_intel_snapshot(destination)` | Lee campos VMCS de salida. |
| `vmexit_amd_snapshot(vmcb, destination)` | Copia campos VMCB de salida. |
| `vmexit_intel_inject(vector, kind, hasError, error, length)` | Programa VM-entry interruption fields. |
| `vmexit_amd_inject(vmcb, vector, kind, hasError, error)` | Programa EVENTINJ. |
| `vmexit_intel_advance(length)` | VMREAD/VMWRITE de guest RIP. |
| `vmexit_amd_advance(vmcb)` | Copia NRIP a RIP. |
| `cpu_read_cr0/cr3/cr4`, `cpu_write_cr0/cr3/cr4` | Estado de control raw. |
| `cpu_read_msr`, `cpu_write_msr`, `cpuid_write` | Capabilities y estado CPU. |
| `port_read8/16/32`, `port_write8/16/32` | Emulacion de puertos. |
| `svm_clgi`, `svm_stgi`, `svm_invlpga` | GIF y TLB por ASID en AMD. |

No se importa runtime hosted, no se reserva memoria y no se crea scheduler.

## Ejecucion material

El engine Linux usa `/dev/kvm` directamente y convierte cada `VcpuExit` al mismo
`VmExit` consumido por `run_vm_exit_loop`. Primero copia la salida prestada por el
mmap de KVM a una representacion poseida, por lo que ninguna referencia vive tras
el siguiente ioctl. Un watchdog monotono de 15 segundos interrumpe `KVM_RUN`,
restaura senales y evita procesos colgados. La sesion certificada ejecuto:

- handler real de `#UD` y `iret`;
- inyeccion externa del vector `0x20`;
- seis salidas de puerto;
- dos HLT;
- write/read MMIO por GPA `0x8000` no mapeada;
- store de RAM `0x1234` y tres paginas dirty;
- secuencia final `SEXIMO`, diez VM-exits y cero eventos pendientes.
- finalizacion en 2 ms con deadline material activo.

El certificado declara que KVM posee el VMCS/VMCB host. No afirma que un proceso
userland ejecute instrucciones privilegiadas ni que la sesion sea bare metal.
