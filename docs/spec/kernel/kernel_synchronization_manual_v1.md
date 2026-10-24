# Sincronizacion kernel manual v1

Estado normativo: activo.

Todas las operaciones son explicitas. Ningun lock duerme, despierta, asigna,
deshabilita interrupciones o ejecuta reclamacion RCU sin una llamada visible.

## Primitivas

- ticket spinlock con adquisicion ordenada, try y espera acotada sin tickets
  abandonados;
- RW lock con contador de lectores y bit de escritor;
- mutex con owner y cola circular entregada por el programador;
- spinlock IRQ-safe con save/disable/restore mediante `InterruptMask`;
- seqlock con reintento observable;
- RCU por CPU con epoch, read-side critical section y grace period explicito;
- atomics acquire/release/acq_rel/seq_cst y barreras de compilador, memoria,
  MMIO e instrucciones.

## Machine

| Target | Atomics | Espera | Barreras |
|---|---|---|---|
| x86_64/i386 | lock cmpxchg/xadd, cmpxchg8b/16b | pause, monitor/mwait, WAITPKG | lfence/sfence/mfence/serialize |
| AArch64 | LDXR/STXR y LSE CAS/SWP/LD* | yield, WFE/SEV/SEVL | DMB ISHLD/ISHST/ISH/OSH, ISB |
| ARMv7/Thumb2 | LDREX/STREX/CLREX | yield, WFE/SEV | DMB/DSB/ISB |
| RV32/RV64 | LR/SC y AMO | pause/WFI | fence y fence.i |

Los bytes se publican en `kernel_sync_plan`; `hidden_scheduler` y
`hidden_allocation` son siempre falsos.

## Uso IRQ

El token conserva el estado anterior de interrupciones. El programador debe
liberar con el mismo token. Si la adquisicion acotada falla, el estado se
restaura antes de devolver el error.

## RCU

Una publicacion incrementa el epoch. La memoria retirada no puede reutilizarse
hasta que `synchronize_bounded` confirme que todos los lectores anteriores han
salido. El dominio no libera objetos ni mantiene una lista de retirados oculta.
