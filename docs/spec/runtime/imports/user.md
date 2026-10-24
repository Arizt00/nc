# Import `use <process:user>`

## Contrato

`process:user` expone identidad, tokens, SID, impersonacion y perfiles mediante
punteros, buffers, capacidades y handles administrados manualmente. No devuelve
`str`, no asigna objetos de usuario y no libera recursos de forma implicita.

```n
use <process:user>;
```

## API raw Win64

| Funcion | Firma | Efecto |
|---|---|---|
| `userBuffer` | `(bytes: int) -> addr` | reserva bytes RW |
| `userFree` | `(memory: addr) -> bool` | libera una reserva de `userBuffer` |
| `userCurrentProcess` | `() -> addr` | devuelve el pseudo-handle del proceso |
| `userName` | `(buffer: addr, sizeSlot: addr) -> bool` | escribe el nombre UTF-16 |
| `userNameEx` | `(format: int, buffer: addr, sizeSlot: addr) -> bool` | escribe el nombre extendido UTF-16 |
| `userProfileDirectory` | `(token: addr, buffer: addr, sizeSlot: addr) -> bool` | escribe la ruta de perfil UTF-16 |
| `userOpenProcessToken` | `(process: addr, access: int, outToken: addr) -> bool` | abre un token |
| `userTokenInformation` | `(token: addr, class: int, buffer: addr, cap: int, required: addr) -> bool` | copia informacion del token |
| `userCheckMembership` | `(token: addr, sid: addr, outMember: addr) -> bool` | comprueba pertenencia a un SID |
| `userCreateWellKnownSid` | `(kind: int, domain: addr, sid: addr, sizeSlot: addr) -> bool` | materializa un SID conocido |
| `userDuplicateToken` | `(token: addr, level: int, outToken: addr) -> bool` | duplica un token |
| `userSetThreadToken` | `(threadSlot: addr, token: addr) -> bool` | asigna el token de hilo |
| `userImpersonate` | `(token: addr) -> bool` | inicia impersonacion |
| `userRevert` | `() -> bool` | restaura la identidad del proceso |
| `userLoadProfile` | `(token: addr, profileInfo: addr) -> bool` | carga un `PROFILEINFOW` preparado por el programa |
| `userUnloadProfile` | `(token: addr, profile: addr) -> bool` | descarga el perfil |
| `userClose` | `(handle: addr) -> bool` | cierra un handle real |
| `userLastError` | `() -> int` | lee el ultimo error Win32 |

Los slots de salida son direcciones de memoria escritas por la API del sistema.
El programa decide su tipo, capacidad, alineacion, ciclo de vida y orden de
cleanup. Pasar null, buffers pequenos o handles invalidos conserva la semantica
raw del sistema operativo.

## Materializacion

La cadena es `.n -> IR -> LLVM/MCT -> Machine -> COFF -> PE64`. Los wrappers
`__ncc_user_*` pertenecen al modulo LLVM y enlazan directamente:

- `ADVAPI32.dll`: tokens, SID e impersonacion;
- `KERNEL32.dll`: proceso, handles, memoria y errores;
- `SECUR32.dll`: nombres extendidos;
- `USERENV.dll`: directorio y carga/descarga de perfiles.

No se genera fuente C, no se enlaza CRT y no existe proveedor C interno para
esta familia. El ejemplo ejecutable es
`examples/runtime/families/process_user_raw.n`.

## Targets

El cierre actual es Win64 x86_64/AArch64. Otro target recibe
`E_MCT_LLVM_PROCESS_USER_TARGET_001`; no se simula una identidad Unix.
