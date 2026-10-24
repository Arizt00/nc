# Kernel multicore scheduler v1

Estado normativo: activo.

## Modelo

`MulticoreScheduler` admite entre 1 y 128 CPU y 32 prioridades. Prioridad 0 es
la mas alta. Cada CPU conserva colas FIFO independientes por prioridad,
proceso actual, quantum restante, cambios de contexto y preemptions.

No existe scheduler oculto: el kernel invoca explicitamente `schedule`,
`timer_tick`, `sleep`, `wake`, `block_on`, `wake_one`, `lock`, `unlock` y
`rebalance`.

## Preemption y tiempo

Un timer tick acumula runtime, despierta deadlines vencidos y provoca cambio
cuando termina el quantum o aparece una tarea de mayor prioridad. Sleep usa un
heap de deadlines monotonicamente ordenado y descarta entradas obsoletas tras
un wakeup anticipado.

## Afinidad y balanceo

La afinidad es una mascara de hasta 128 CPU. La insercion elige la CPU
permitida con menor carga. El balanceador roba tareas ready desde la cola de
menor prioridad del CPU mas cargado, pero nunca viola la mascara de afinidad.

## Espera y locks

Las wait queues tienen identidad de 64 bits y operaciones wake-one/wake-all.
Los mutexes son no recursivos, transfieren propiedad al primer waiter y aplican
herencia de prioridad mientras exista contencion. Unlock restaura la prioridad
base del propietario.

## Integracion con procesos

`KernelCore` sincroniza Ready, Running, Blocked, Sleeping y Zombie, ofrece
wrappers de timer, sleep/wakeup, wait queues, afinidad, prioridad y locks, y
elimina la entidad scheduler al terminar un proceso.

El cambio material de contexto usa las instrucciones publicadas por el perfil
machine: XSAVE/FSGSBASE/IRET en x86_64, registros EL1/TPIDR y ERET en AArch64,
y CSR/SRET en RISC-V.

