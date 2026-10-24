# Import manual `process:command`

## Superficie

```n
use <process:command>;
```

Esta familia expone procesos y pipes como descriptores crudos. No invoca un
shell, no cita argumentos, no crea colecciones y no cierra recursos de forma
implicita. El programador conecta handles y controla espera, terminacion,
entrada, salida y liberacion.

## Proceso

| Funcion | Contrato |
|---|---|
| `commandOpen` | `(str, str, addr, addr, addr, int) -> addr` |
| `commandStarted` | `(addr) -> bool` |
| `commandWait` | `(addr, int) -> int` |
| `commandExitCode` | `(addr) -> int` |
| `commandTerminate` | `(addr, int) -> bool` |
| `commandProcessHandle` | `(addr) -> addr` |
| `commandThreadHandle` | `(addr) -> addr` |
| `commandId` | `(addr) -> int` |
| `commandThreadId` | `(addr) -> int` |
| `commandError` | `(addr) -> int` |
| `commandRelease` | `(addr) -> bool` |

`commandOpen` recibe linea mutable UTF-8, directorio, handles de stdin/stdout/
stderr y flags nativos. Una cadena vacia como directorio hereda el actual.

## Pipe

| Funcion | Contrato |
|---|---|
| `commandPipeOpen` | `(bool, bool) -> addr` |
| `commandPipeReadHandle` | `(addr) -> addr` |
| `commandPipeWriteHandle` | `(addr) -> addr` |
| `commandPipeRead` | `(addr, addr, int) -> int` |
| `commandPipeWrite` | `(addr, addr, int) -> int` |
| `commandPipeCloseRead` | `(addr) -> bool` |
| `commandPipeCloseWrite` | `(addr) -> bool` |
| `commandPipeError` | `(addr) -> int` |
| `commandPipeRelease` | `(addr) -> bool` |

Los booleanos de `commandPipeOpen` fijan herencia para cada extremo. Los
buffers y capacidades de lectura/escritura pertenecen por completo al caller.

## Backend

Win64 materializa la familia con `CreateProcessW`, `WaitForSingleObject`,
`GetExitCodeProcess`, `TerminateProcess`, `CreatePipe`,
`SetHandleInformation`, `ReadFile`, `WriteFile` y `CloseHandle`.

Linux, Android y Darwin materializan la misma ABI con `fork`, `execvp`,
`waitpid`, `kill`, `pipe`, `dup2`, `fcntl`, `read`, `write`, `close`, `chdir`
y `_exit`. El parser interno convierte la linea en `argv` con soporte de
comillas simples/dobles y escapes; los argumentos llegan directamente a
`execvp`.

No usa `system`, `popen`, CRT, proveedor C ni shell implícito. Para usar CMD o
PowerShell el nombre y sus argumentos deben estar visibles en la linea que el
programador entrega.

## Comportamiento raw

Los handles expuestos pueden cerrarse o corromperse fuera de esta API. Usar un
descriptor liberado, reutilizar handles, truncar flags o leer fuera de un
buffer conserva comportamiento indefinido. La auditoria no modifica bytes.

## Evidencia

El fixture `examples/runtime/families/process_command_raw.n` abre un pipe,
ejecuta CMD explicitamente, hereda stdout/stderr, espera, recibe bytes en stack,
comprueba IDs y codigo de salida y libera los dos descriptores. El fixture
`examples/runtime/families/process_command_portable_raw.n` selecciona un argv
visible por host y verifica la captura byte a byte en Windows y Linux.

La ejecucion retenida certifica Windows x86_64 y Ubuntu x86_64. Windows AArch64
comparte la definicion LLVM de Win64; Android y Darwin comparten la definicion
POSIX. Esos targets quedan pendientes de sus runners fisicos, sin atribuirles
ejecucion no observada.
