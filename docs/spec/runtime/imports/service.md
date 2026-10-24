# Import `use <process:service>;`

## Contrato

Expone el Service Control Manager de Windows como ABI manual. El backend no
busca servicios, no asigna buffers, no espera transiciones y no conserva
handles. El programa prepara nombres UTF-16, access masks, estructuras,
callbacks, memoria y tiempos de espera, y cierra cada recurso.

## Nombres y memoria

`serviceWideAlloc(text)` convierte UTF-8 a un bloque UTF-16 asignado. El bloque
pertenece al programa y se libera con `serviceWideFree`. Las demás operaciones
reciben `addr`; `null` conserva el significado nativo de Win32. No se validan
alineación, capacidad, vida, aliasing ni mutabilidad de los punteros.

## Operaciones

| Operación | Firma | API material |
|---|---|---|
| `serviceWideAlloc` | `(str) -> addr` | `MultiByteToWideChar` + heap raw |
| `serviceWideFree` | `(addr) -> bool` | heap raw |
| `serviceManagerOpen` | `(addr, addr, int) -> addr` | `OpenSCManagerW` |
| `serviceOpen` | `(addr, addr, int) -> addr` | `OpenServiceW` |
| `serviceCreate` | `(addr, addr, addr, int, int, int, int, addr, addr, addr, addr, addr, addr) -> addr` | `CreateServiceW` |
| `serviceDelete` | `(addr) -> bool` | `DeleteService` |
| `serviceStart` | `(addr, int, addr) -> bool` | `StartServiceW` |
| `serviceControl` | `(addr, int, addr) -> bool` | `ControlService` |
| `serviceControlEx` | `(addr, int, int, addr) -> bool` | `ControlServiceExW` |
| `serviceQueryStatus` | `(addr, int, addr, int, addr) -> bool` | `QueryServiceStatusEx` |
| `serviceQueryConfig` | `(addr, addr, int, addr) -> bool` | `QueryServiceConfigW` |
| `serviceChangeConfig` | `(addr, int, int, int, addr, addr, addr, addr, addr, addr, addr) -> bool` | `ChangeServiceConfigW` |
| `serviceEnum` | `(addr, int, int, int, addr, int, addr, addr, addr, addr) -> bool` | `EnumServicesStatusExW` |
| `serviceEnumDependents` | `(addr, int, addr, int, addr, addr) -> bool` | `EnumDependentServicesW` |
| `serviceKeyName` | `(addr, addr, addr, addr) -> bool` | `GetServiceKeyNameW` |
| `serviceDisplayName` | `(addr, addr, addr, addr) -> bool` | `GetServiceDisplayNameW` |
| `serviceManagerLock` | `(addr) -> addr` | `LockServiceDatabase` |
| `serviceManagerUnlock` | `(addr) -> bool` | `UnlockServiceDatabase` |
| `serviceManagerLockStatus` | `(addr, addr, int, addr) -> bool` | `QueryServiceLockStatusW` |
| `serviceNotify` | `(addr, int, addr) -> int` | `NotifyServiceStatusChangeW` |
| `serviceStartDispatcher` | `(addr) -> bool` | `StartServiceCtrlDispatcherW` |
| `serviceRegisterHandler` | `(addr, addr, addr) -> addr` | `RegisterServiceCtrlHandlerExW` |
| `serviceSetStatus` | `(addr, addr) -> bool` | `SetServiceStatus` |
| `serviceClose` | `(addr) -> bool` | `CloseServiceHandle` |
| `serviceLastError` | `() -> int` | `GetLastError` |

Los enteros se truncan a `DWORD` en Win64. Los callbacks y tablas se pasan como
direcciones sin adaptador. Los buffers de salida conservan exactamente el
layout de Win32 y sus campos `DWORD`/puntero según el target.

## Errores y privilegios

Las funciones `BOOL` devuelven `false`, las de handle devuelven `null` y
`serviceNotify` devuelve el código Win32. `serviceLastError()` debe consultarse
inmediatamente cuando la API correspondiente indique fallo. No se solicitan ni
elevan privilegios de forma implícita.

## Backend

Win64 x86_64 y AArch64 bajan directamente a `ADVAPI32.dll` y `KERNEL32.dll`
desde LLVM/MCT/Machine. No se genera C, no se enlaza CRT y no se ejecutan
PowerShell, CMD ni `sc.exe`. Otros targets reciben un diagnóstico de target en
vez de una emulación silenciosa.

## Ejemplo y pruebas

- `examples/runtime/families/process_service_raw.n` abre y consulta `EventLog`
  sin modificarlo.
- `crates/ncc-sema/tests/compact_process_service.rs` verifica tipos positivos y
  rechazo de un `str` usado como handle.
- Las pruebas de `ncc-ir` verifican llamadas MCT y símbolos nativos.
- Las pruebas de `ncc-cli` verifican que el linker PE asigne las APIs SCM a
  `ADVAPI32.dll`.
