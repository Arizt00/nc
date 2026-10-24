# Machine VM lifecycle, snapshots and debug v1

Estado: cerrado para el formato y ciclo de vida declarados.

## Estados

`VmLifecycle` admite `Created`, `Running`, `Paused`, `Suspended`, `Resetting`,
`Stopped` y `Crashed`. Las transiciones invalidas fallan; start, pause, resume,
suspend, wake, warm/cold reset, stop y crash generan eventos secuenciales con
motivo, boot count y reset count.

## Snapshot NCSNAP02

El formato binario tiene magic `NCSNAP02`, version 2 y endian little. Conserva:

- ID y parent ID de 128 bits, generacion y timestamp;
- estado, target y arquitectura;
- bancos de registros para x86_64, i386, AArch64, RISC-V 64 y RISC-V 32;
- estado halted, interrupciones pendientes y tiempo virtual por vCPU;
- memoria full o incremental por paginas;
- dispositivos versionados, required/optional, payload opaco y CRC32;
- metadata ordenada y checksum global.

La decodificacion comprueba magic, version, endian, longitudes, overflow,
duplicados, page size, indices, CRC por dispositivo, CRC global y ausencia de
bytes finales. `SnapshotLimits` limita bytes totales, RAM, paginas, vCPU,
dispositivos, payload y strings antes de reservar memoria.

La persistencia escribe un temporal en el mismo directorio, sincroniza y hace
rename atomico. Full + incremental reconstruyen exactamente la memoria base y
rechazan parent, longitud o pagina incompatibles.

## Depuracion

`VmDebugger` soporta breakpoints software, breakpoints hardware de ejecucion,
watchpoints read/write/read-write, single-step, excepciones, interrupciones,
shutdown y crash. Conserva byte original, slot hardware, hit count y una cola
acotada de eventos.

Los opcodes software son `int3` en x86, `brk #0` en AArch64 y `ebreak` en
RISC-V. El flujo software restaura el opcode original, permite ejecutar una
instruccion y lo reinserta. Los watchpoints validan longitud, overlap, clase de
acceso y slots disponibles.

## Evidencia material

La prueba KVM retenida produce un exit debug hardware real, pausa el lifecycle,
persiste una snapshot completa y otra incremental, recarga ambas, restaura RAM y
checkpoint de dispositivos, limpia el evento pendiente, reanuda y completa un
reset calido. QMP valida ademas `prelaunch -> running -> paused`, savevm, loadvm,
system_reset y apagado limpio.

## Frontera

El formato es multi-arquitectura; la ejecucion privilegiada exact-head disponible
se realizo en x86_64 Intel. AArch64 y RISC-V tienen opcodes y bancos tipados, pero
su certificacion CPU-backed exige runners de esas arquitecturas.
