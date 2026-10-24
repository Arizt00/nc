# Runtime real prioridad 1

Este documento fija el contrato implementado para la primera bajada real del
runtime ejecutado por `ncc run archivo.n`.

`ncc run archivo.n` es el modo real para estas APIs. No se requiere
`ncc run archivo.n --real`: las llamadas a `fs*`, `tcp*`, `process.*` y
`thread.*` se resuelven contra símbolos runtime reales y el manifiesto
Backend/MCT las marca como `closed_real_backing_bytes`.

El facade compacto `use <time:sleep>` baja a
`nC.runtime.time.sleep`. La familia `nC.runtime.sleep` sigue reservada para
utilidades explícitas como `sleepMs` y `sleepSec`.

## Alcance cerrado

- `nC.runtime.fs`
  - `readFile` / `fsread`: lee bytes de un archivo real y devuelve `Result<str, str>`.
  - `writeFile` / `fswrite`: crea o reemplaza un archivo real y devuelve `Result<bool, str>`.
  - `appendFile` / `fsappend`: abre en append real y devuelve `Result<bool, str>`.
  - `fsync` / `syncFile`: ejecuta `sync_all` sobre el descriptor real y devuelve `Result<bool, str>`.
  - `exists`: comprueba existencia real y devuelve `bool`.
  - `deleteFile`: borra archivo real y devuelve `Result<bool, str>`.
  - `copyFile`: copia archivo real y devuelve `Result<int, str>` con bytes copiados.

- `nC.runtime.time` / `nC.runtime.sleep`
  - `now` / `timestamp`: devuelve segundos UNIX reales en `int`.
  - `monotonic`: devuelve milisegundos desde un reloj monotónico interno.
  - `sleep` / `sleepMs` / `sleepSec`: bloquea realmente el hilo del runtime.

- `nC.runtime.tcp`
  - `tcpListen(port)`: abre un `TcpListener` real en `127.0.0.1`.
  - `tcpPort(listener)`: devuelve el puerto real asignado, útil con `tcpListen(0)`.
  - `tcpConnect(host, port)`: abre un `TcpStream` real.
  - `tcpAccept(listener)`: acepta una conexión real.
  - `tcpSend(stream, payload)`: escribe bytes reales y devuelve bytes enviados.
  - `tcpRecv(stream, maxBytes, timeoutMs)`: lee bytes reales con timeout.
  - `tcpClose(handle)`: cierra listener o stream.

- `nC.runtime.exit`
  - `processCurrent` / `processOpen` exponen handles nativos sin tabla runtime.
  - `processId`, `processExitCode`, `processTerminate` y `processClose` operan
    directamente sobre esos handles.
  - `exitNow(code)` llama a la terminación inmediata del sistema; no ejecuta
    limpieza, destructores ni estado de salida administrado.
  - `failFast(record, context, flags)` conserva punteros y flags del programador.

- `nC.runtime.process`
  - `processId()`: devuelve el PID real del proceso `ncc run`.
  - `parentProcessId()`: devuelve el PPID real en Linux/WSL mediante `/proc/self/stat`;
    en plataformas sin API estable expuesta por `std` devuelve `0` de forma explícita.
  - `spawn(cmd, args)`: ejecuta `cmd` con `args:list<str>` usando `Command::new`
    sin shell implícito y devuelve `Result<ProcessHandle, ProcessError>`.
  - `wait(handle)`: espera el hijo real, consume el handle y devuelve
    `Result<int, ProcessError>` con el código de salida (`-1` si el OS no entrega código).
  - `kill(handle)`: termina el hijo real, consume el handle, hace `wait` interno de
    limpieza y devuelve `Result<bool, ProcessError>`.
  - `ProcessHandle` y `ProcessError` son valores nominales etiquetados por el runtime.
    El id interno del handle indexa una tabla viva de hijos reales; usar un handle
    desconocido o ya consumido devuelve `Result.err(ProcessError)`.

- `nC.runtime.thread`
  - `threadSpawn(@fn(...))`: crea un hilo real del host, ejecuta la función nC en un
    `ExecState` aislado y devuelve `Result<ThreadHandle, ThreadError>`.
  - `threadJoin(handle)`: consume el handle, espera el hilo real y devuelve
    `Result<any, ThreadError>` con el valor retornado por la función del hilo.
    El checker permite asignar ese resultado dinámico a `Result<T, ThreadError>`
    cuando el programa conoce el tipo de retorno esperado.
  - `threadId()`: devuelve un id entero estable por hilo dentro del proceso `ncc run`.
  - `threadSleep(ms)`: bloquea realmente el hilo actual con milisegundos no negativos.
  - `ThreadHandle` y `ThreadError` son valores nominales etiquetados por el runtime.
    El id interno del handle indexa una tabla viva de `JoinHandle` reales; usar un
    handle desconocido o ya consumido, o pasar algo distinto de `@fn(...)` a
    `threadSpawn`, devuelve `Result.err(ThreadError)` con `E_RUNTIME_THREAD_001`.

## Ejemplos funcionales

- `examples/runtime/runtime_real_fs_append_fsync.n`
- `examples/runtime/runtime_real_time_sleep.n`
- `examples/runtime/runtime_real_tcp_loopback.n`
- `examples/runtime/runtime_real_exit_code.n`
- `examples/runtime/runtime_real_process_id.n`
- `examples/runtime/runtime_real_process_spawn_wait.n`
- `examples/runtime/runtime_real_process_spawn_kill.n`
- `examples/runtime/runtime_real_thread_spawn_join.n`
- `examples/runtime/runtime_real_thread_sleep_id.n`

## Ejemplos negativos

- `examples/runtime/negative/runtime_real_fs_missing_read_fail.n`
- `examples/runtime/negative/runtime_real_tcp_invalid_connect_result.n`
- `examples/runtime/negative/runtime_real_exit_code_invalid_fail.n`
- `examples/runtime/negative/runtime_real_process_invalid_spawn_result.n`
- `examples/runtime/negative/runtime_real_process_double_wait_result.n`
- `examples/runtime/negative/runtime_real_process_wait_invalid_handle_result.n`
- `examples/runtime/negative/runtime_real_process_kill_missing_process_result.n`
- `examples/runtime/negative/runtime_real_thread_invalid_spawn_result.n`
- `examples/runtime/negative/runtime_real_thread_double_join_result.n`

## Tests de cierre process/thread/WAL

La fase de tests queda cerrada con ejemplos `.n` ejecutables y pruebas de
integracion que invocan `ncc run` real:

- Positivo process: `runtime_real_process_spawn_wait.n` lanza un proceso real y
  espera su exit code.
- Positivo thread: `runtime_real_thread_spawn_join.n` lanza un hilo real, hace
  join y verifica el payload retornado.
- Negativo wait: `runtime_real_process_wait_invalid_handle_result.n` usa un
  `ProcessHandle` nominal que no existe en el registro vivo y exige
  `Result.err(ProcessError)`.
- Negativo kill: `runtime_real_process_kill_missing_process_result.n` intenta
  matar un proceso inexistente en el registro vivo y exige
  `Result.err(ProcessError)`.
- WAL E2E: `walLeader.n` y `walFollower.n` ejecutan un cluster multi-proceso real
  con TCP, fsync, kill/restart y recovery.

Comandos baseicos:

```bash
cargo test -p ncc-cli --test runtime_process_thread_contract -- --nocapture
cargo test -p ncc-cli --test wal_runtime_cluster -- --nocapture
ncc run examples/runtime/runtime_real_process_spawn_wait.n
ncc run examples/runtime/runtime_real_thread_spawn_join.n
ncc run examples/runtime/negative/runtime_real_process_wait_invalid_handle_result.n
ncc run examples/runtime/negative/runtime_real_process_kill_missing_process_result.n
ncc run examples/runtime/walLeader.n
```

## Nota de backend/MCT

Las familias nuevas quedan visibles desde el catálogo de imports y el manifiesto
de lowering de imports. Cada import backed tiene:

- `status = closed_real_backing_bytes`;
- símbolo de activación MCT, por ejemplo
  `__nc_import_activate_auth_system_process`;
- bytes reales de activación para `nC.sysv64`, `nC.win64`, `nC.aapcs64` y
  `nC.portable`;
- `runtime_symbols` con las funciones reales invocadas por `ncc run`, por
  ejemplo `nC.runtime.process.spawn`, `nC.runtime.thread.threadJoin`,
  `nC.runtime.tcp.tcpRecv`, `nC.runtime.fs.fsync` y
  `nC.runtime.time.sleep`;
- `runtime_call_lowering = direct_runtime_symbol_call_without_real_flag`.

Las entradas que solo existen en catálogo siguen visibles para MCT, pero quedan
marcadas como `closed_catalog_only_with_activation_bytes` y mantienen
`E_USE_BACKING_DEFERRED_001` en lugar de fingir soporte runtime.

## Prioridad 2: ABI estable de símbolos

El ABI estable de las llamadas reales queda declarado por:

```bash
ncc runtime abi --json
```

Ese manifiesto usa el schema `nc.runtime.symbol_abi.v1` y documenta, por cada
símbolo real:

- tipos de entrada;
- tipo de retorno;
- layout de `Result<T,E>`, handles y errores nominales;
- códigos de error;
- ownership del recurso y si una llamada consume o solo toma prestado un handle.

La prueba de cierre exige que todo símbolo runnable expuesto por
`ncc imports runtime-table --json` tenga una fila ABI.
