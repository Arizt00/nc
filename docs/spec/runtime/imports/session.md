# `use <process:session>`

## Proposito

Control manual de sesiones Windows Remote Desktop Services y canales virtuales.
La familia baja directamente a `WTSAPI32.dll` y `KERNEL32.dll`; no representa
metadatos del compilador ni crea objetos administrados.

## Contrato

```nC
sessionWideAlloc(text: str) -> addr
sessionWideFree(wide: addr) -> bool
sessionCurrentId() -> int
sessionActiveConsoleId() -> int
sessionProcessId(pid: int, outId: addr) -> bool
sessionServerOpen(wideName: addr) -> addr
sessionServerClose(server: addr) -> bool
sessionEnumerate(server: addr, outEntries: addr, outCount: addr) -> bool
sessionQuery(server: addr, id: int, class: int, outBuffer: addr, outBytes: addr) -> bool
sessionFree(buffer: addr) -> bool
sessionQueryUserToken(id: int, outToken: addr) -> bool
sessionSendMessage(server: addr, id: int, title: addr, titleBytes: int,
    message: addr, messageBytes: int, style: int, timeout: int,
    outResponse: addr, wait: bool) -> bool
sessionDisconnect(server: addr, id: int, wait: bool) -> bool
sessionLogoff(server: addr, id: int, wait: bool) -> bool
sessionConnect(id: int, target: int, password: addr, wait: bool) -> bool
sessionRegisterNotification(window: addr, flags: int) -> bool
sessionUnregisterNotification(window: addr) -> bool
sessionWaitEvent(server: addr, mask: int, outFlags: addr) -> bool
sessionChannelOpen(id: int, name: str, flags: int) -> addr
sessionChannelClose(channel: addr) -> bool
sessionChannelRead(channel: addr, timeout: int, buffer: addr,
    capacity: int, outRead: addr) -> bool
sessionChannelWrite(channel: addr, buffer: addr, bytes: int, outWritten: addr) -> bool
sessionChannelQuery(channel: addr, class: int, outBuffer: addr, outBytes: addr) -> bool
sessionChannelPurgeInput(channel: addr) -> bool
sessionChannelPurgeOutput(channel: addr) -> bool
sessionLastError() -> int
```

## Memoria y handles

- `null` representa `WTS_CURRENT_SERVER_HANDLE` donde la API WTS lo admite.
- `sessionServerClose` solo se usa con handles obtenidos por `sessionServerOpen`.
- `sessionQuery`, `sessionEnumerate` y `sessionChannelQuery` devuelven memoria en
  un slot `addr*`; el programador debe llamar `sessionFree`.
- `sessionWideAlloc` convierte UTF-8 a UTF-16 y exige `sessionWideFree`.
- Los tokens, ventanas y canales son handles raw; no hay cierre implicito.
- Las clases, flags, permisos y layouts WTS son enteros y buffers del programa.

## Backend

Win64 x86_64/AArch64 usa ABI WTS directa. Un target no compatible produce
`E_MCT_LLVM_PROCESS_SESSION_TARGET_001`; nunca se simula la operacion ni se
genera C como fallback.

Ejemplo ejecutable:
`examples/runtime/families/process_session_raw.n`.
