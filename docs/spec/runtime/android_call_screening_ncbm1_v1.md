# Android Call Screening y NCBM1 v1

## Alcance

`ncc android build` materializa una fuente normal `.n` mediante esta cadena:

```text
.n -> AST/sema -> IR O3 -> LLVM/MCT AArch64 PIC -> .o -> arm64-v8a .so
   -> JNI provider -> Java/DEX -> CallScreeningService -> APK alineado y firmado
```

El target canónico es `aarch64-linux-android<API>`, ABI `arm64-v8a`, ELF64,
PIC y AAPCS64 Android. El mínimo de `CallScreeningService` es API 29.

## Comandos

```powershell
ncc android doctor --json
ncc android build examples/12_apps/blockmaster_android_core.n
ncc android build app.n --native-prefix miContenedor --keystore release.p12 `
  --key-alias release --ks-pass "$env:NCC_ANDROID_KEYSTORE_PASSWORD" `
  --key-pass "$env:NCC_ANDROID_KEY_PASSWORD"
ncc android install .ncc/android/app/bin/BlockMaster-arm64-v8a-signed.apk
```

Sin `--keystore`, el compilador crea una identidad local RSA-3072 para pruebas.
Una publicación debe suministrar y custodiar su propio keystore.

## Contrato JNI

La biblioteca `libncblockmaster.so` exporta JNI 1.6 y conecta
`NcMobileProviderV2` con el runtime. Las decisiones de bloqueo, listas,
patrones, estado, serialización y restauración se ejecutan en funciones nC.
Java coordina el ciclo de vida Android, el rol del sistema, persistencia
atómica y transportes.

El constructor distingue cuatro perfiles materiales por los exports LLVM del
contenedor nC: `blocklist`, `blocklist-contacts`,
`blocklist-contacts-bmui1` y `whitelist-urgency`. El perfil
`blocklist-contacts` exige, además del contrato de blocklist,
`setContactsOnlyMode(bool)`, `shouldBlockContactAt(str, int, bool)` y
`lastDecision() -> str`. Solo en ese perfil se genera el JNI de contactos y
`BuildConfig.NATIVE_CONTACTS=true`.

`blocklist-contacts-bmui1` amplía ese contrato con `addAllowPattern`,
`removeAllowPattern`, `setProtectionActive`, `setBlockHidden`, `activity`,
`uiContract`, `versionInfo` y `updatePolicy`. Activa
`BuildConfig.NATIVE_BMUI1=true` y materializa las superficies Inicio,
Actividad, Reglas y Ajustes. Android presenta estado y ciclo de vida; las
decisiones, prioridades y snapshots siguen siendo autoridad exclusiva de nC.

Antes de sema y lowering, la ruta Android adjunta los proveedores nC requeridos
de texto, parseo y Unicode. Esto cubre tanto imports explicitos como operaciones
implicitas del lenguaje, por ejemplo concatenacion, `.split()` y `.toInt()`.

El servicio de filtrado inicializa y restaura el estado nativo aunque el
servidor remoto no se haya iniciado. Ante un fallo de inicialización permite
la llamada y registra el error, evitando dejar una llamada sin respuesta.

## Contactos del dispositivo

El adaptador Android del compilador declara `READ_CONTACTS` y consulta
`ContactsContract.PhoneLookup.CONTENT_FILTER_URI`, que compara el número según
las reglas de telefonía de Android. La plataforma solo entrega a nC un booleano
`esContacto`; la política y sus prioridades permanecen en la función nC
tipada `shouldBlockContactAt`.

La prioridad canónica de BlockMaster es:

1. número o patrón en la blocklist: bloquear, aunque sea un contacto;
2. contacto Android o whitelist manual: permitir;
3. `MODO_CONTACTOS ON`: bloquear cualquier otro número;
4. `MODO_CONTACTOS OFF`: continuar con la política de urgencia previa.

Un número oculto se clasifica como desconocido cuando el permiso está activo.
Si falta `READ_CONTACTS` o la consulta falla, el servicio no supone que todos
los números sean desconocidos: vuelve a `shouldBlockAt` y conserva el
comportamiento anterior. La aplicación muestra este estado y permite solicitar
el permiso de nuevo. La preferencia `contacts_only` se reaplica durante la
inicialización nativa, incluso si Android despierta `CallScreeningService` sin
haber iniciado antes el servidor remoto.

Después de cada evaluación, nC conserva una decisión tipada como texto de
protocolo: `BLOCKED|PATTERN|<patrón>|<número>`, `ALLOWED|CONTACT|<número>` o
el motivo equivalente. El adaptador la guarda con su hora en preferencias
privadas. La actividad observa esos cambios y muestra número, resultado y causa
sin duplicar la política de decisión fuera de nC.

### Límite de redirección

`CallScreeningService` permite, silencia o rechaza una llamada entrante; no
ofrece una operación para cambiar su destino. `CallRedirectionService` sí puede
cambiar un destino, pero Android lo invoca al realizar llamadas salientes. Por
eso BlockMaster representa el descarte real como `BLOCKED` y no como una falsa
redirección. Un desvío entrante verdadero depende del operador o de una
infraestructura telefónica propia, no del contrato de screening.

## Persistencia

`BlocklistStore` guarda `blockmaster.snapshot` en el almacenamiento privado de
la aplicación. Escribe primero un temporal, hace `fsync` y usa reemplazo
atómico cuando el filesystem lo permite. Las mutaciones remotas solo responden
`OK` después de persistir; un fallo devuelve `ERR|PERSISTENCE|<tipo>`.

## NCBM1

El saludo siempre usa una línea ASCII:

```text
HELLO|NCBM1|serverNonce|LINE,FRAME|SECURE
HELLO|NCBM1|serverNonce|LINE,FRAME|RAW
```

`LINE` termina cada frame con LF. `FRAME` usa longitud big-endian de 32 bits
seguida de UTF-8. El límite por frame es 1 MiB. Las respuestas de órdenes no
contienen LF interno, por lo que una orden siempre produce un frame.

### Compatible, puerto 8080

TCP simple, una orden por conexión. Esta ruta acepta controladores existentes que
envían bytes UTF-8 sin saludo ni delimitador y esperan una respuesta antes de cerrar.
El final de la petición se detecta mediante cierre o inactividad acotada.

### Seguro, puerto 8443

1. TLS 1.2 o 1.3 con certificado fijable por certificado o SHA-256.
2. `AUTH|framing|clientNonce|HMAC(token, AUTH|framing|serverNonce|clientNonce)`.
3. `AUTH_OK|session|HMAC(token, AUTH_OK|session)`.
4. `REQ|session|sequence|base64url(command)|HMAC(envelope)`.
5. `RES|session|sequence|base64url(response)|HMAC(envelope)`.

La secuencia empieza en uno y debe crecer exactamente de uno en uno. Cada
reconexión negocia nonces y sesión nuevos; un replay o salto cierra la sesión.

### Raw, puerto 8081

Raw está desactivado por defecto y requiere opt-in visible en la aplicación:

```text
SELECT|LINE|RAW
READY|RAW|LINE
```

No cifra, no autentica y no evita replay. Existe para pruebas y para quien
decida asumir explícitamente ese riesgo.

## Órdenes

Todas devuelven una respuesta:

| Orden | Respuesta principal |
| --- | --- |
| `PING` | `OK|PONG` |
| `AYUDA` | catálogo de órdenes |
| `AGREGAR <número>` | `OK|ADDED` o error |
| `ELIMINAR <número>` | `OK|REMOVED` o error |
| `BLOQUEAR_POR_PATRON <texto>` | `OK|PATTERN_ADDED` o error |
| `DESBLOQUEAR_PATRON <texto>` | `OK|PATTERN_REMOVED` o error |
| `AGREGAR_WHITELIST <número>` | `OK|WHITELIST_ADDED` o error |
| `ELIMINAR_WHITELIST <número>` | `OK|WHITELIST_REMOVED` o error |
| `AJUSTAR_TIEMPO <1..86400>` | `OK|WINDOW|<segundos>` o error |
| `MODO_WHITELIST ON\|OFF` | selecciona urgencia o blocklist |
| `MODO_CONTACTOS ON\|OFF` | activa o desactiva el filtro de desconocidos |
| `ULTIMA_DECISION` | resultado y motivo exactos producidos por nC |
| `PERMITIR_POR_PATRON <texto>` | agrega un patrón permitido |
| `QUITAR_PERMISO_PATRON <texto>` | elimina un patrón permitido |
| `MODO_PROTECCION ON\|OFF` | activa o pausa la política nC |
| `BLOQUEAR_OCULTOS ON\|OFF` | configura números sin identidad |
| `ACTIVIDAD` | decisiones recientes normalizadas por nC |
| `CONTRATO_UI` | contrato de capacidades `BMUI1` |
| `VERSION` | versión, esquema y nivel de parche |
| `POLITICA_ACTUALIZACION` | canal, firma y protección anti-rollback |
| `HISTORIAL` | historial de llamadas en un frame |
| `LIMPIAR_HISTORIAL` | `OK|HISTORY_CLEARED` |
| `LISTAR` | números y patrones en un frame |
| `ESTADO` | contadores y tamaños |
| `LIMPIAR` | `OK|CLEARED` |
| `APAGAR_SERVIDOR` | `OK|SERVER_STOPPING` |
| cualquier otra | `ERR|UNKNOWN_COMMAND` |

## Uso desde PC

Copie el token mostrado por la aplicación y use el certificado emitido junto
al APK:

El paquete de desarrollo entrega `bin/pc-tools`, con proyectos nC separados
para el constructor, el enlazador, el cliente y el verificador. Construya primero
el ejecutable de `pc-tools/build` con `ncc project sync` y `ncc project link`,
usando `main.n` como entrada. Después ejecute el constructor:

```powershell
build-mobile-tools.exe --source "RUTA/bin/pc-tools" --ncc "RUTA/ncc.exe" --cc "C:/msys64/mingw64/bin/gcc.exe"
```

En Windows el constructor distribuye las DLL de OpenSSL junto a los ejecutables.
En Linux necesita los proveedores binarios instalados `libssl` y `libcrypto`.
El código de protocolo, HMAC, Base64, reconexión y verificación está escrito en
nC; TLS usa la ABI declarada de esos proveedores, no un intérprete Python.

```powershell
blockmaster.exe 192.168.1.40 `
  --token TOKEN --cert blockmaster-server.crt.pem `
  "AGREGAR +34620123456" "LISTAR" "ESTADO"

blockmaster.exe 192.168.1.40 `
  --token TOKEN --cert blockmaster-server.crt.pem --framing FRAME "ESTADO"

blockmaster.exe 192.168.1.40 `
  --raw --framing LINE "PING"
```

La conexión USB puede tunelizar el puerto seguro:

```powershell
adb forward tcp:8443 tcp:8443
blockmaster.exe 127.0.0.1 --port 8443 `
  --token TOKEN --cert blockmaster-server.crt.pem "ESTADO"
```

## Evidencia

`protocol.exe` (o `protocol` en Linux), construido desde
`protocol_conformance_test.n`, prueba ambos niveles de seguridad, ambos
framings, todas las órdenes y reconexión forzada. `ncc android build` retiene
LLVM, `.nm`, objetos, `.so`, APK, certificado y hashes en
`blockmaster-android-build.json`.
