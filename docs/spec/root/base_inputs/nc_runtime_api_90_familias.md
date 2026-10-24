# Entrada histórica — API de `runtime` anterior a la sintaxis compacta canónica

> Registro de diseño preservado para trazabilidad. No define sintaxis vigente ni
> debe copiarse como estilo actual. La autoridad de escritura es
> `docs/spec/language/canonical_compact_syntax_v3.md`; los nombres internos
> `nC.runtime.*` continúan existiendo únicamente como símbolos del compilador.

## 0. Principio central

El `runtime` de nC no es una librería decorativa ni una carpeta de utilidades sueltas. Es la **superficie de ejecución** que permite a los programas altos hablar con entrada, salida, archivos, red, memoria controlada, procesos, tareas, errores, capacidades y entorno sin caer directamente en máquina cruda.

Regla madre:

```txt
El runtime ayuda arriba, se revela en la frontera, y desaparece abajo.
```

En código normal, el runtime **no debe escribirse con rutas `runtime.*` dentro del cuerpo**. Las familias incluidas en el prelude material del backend se resuelven sin `use`; las demás se seleccionan con el `use` compacto. `print` y `println` conservan `use <io:output>;` como requisito explícito.

### Forma correcta

```nC
use nC.runtime.input;
use nC.runtime.output;

programa[@runtime] {
    fn:
        ejecutar() -> bool {
            print("Edad: ");
            @edad >> int : readInt()
                !> int : dev(InputParseError @runtime);

            println("Edad recibida.");
            return true;
        }

    ref main ->> ejecutar;

    fa:
        return true;
}
```
### Forma incorrecta en código normal

```nC
@edad >> int : runtime.input.readInt();
runtime.output.println("Hola");
```
### Excepción: rutas explícitas en zonas cercanas al sistema

```nC
use nC.runtime.trace with bridge(kernel, debug);

kernelTrace[@kernel] {
    fn:
        marcar() -> bool {
            runtime.trace.kernelMark("scheduler-enter");
            return true;
        }
}
```

## 1. Relación con modos

| Modo | Relación con `runtime` |
|---|---|
| Código normal | Puede usar runtime mediante `use`; llamadas directas |
| `[@runtime]` | Modo natural del runtime |
| `[@tooling]` | Muy compatible: CLI, compilador, IDE, diagnósticos |
| `[@compiler]` | Compatible para diagnósticos, archivos, logs y procesos controlados |
| `[@inspect]` | Compatible para trazas, introspección y reportes |
| `[@system]` | Compatible con rutas explícitas, capacidades y política |
| `[@network]` | Compatible para sockets, buffers, streams y protocolos |
| `[@security]` | Compatible, pero con redacción, política y restricciones |
| `[@kernel]` | Solo puentes concretos |
| `[@machine]` | Solo puentes extremos |
| `[@]` | No depende de runtime |
| `primitive[@]` | Prohibido |

`[@]` puede tener operaciones basales propias del lenguaje, pero no debe importar `nC.runtime.*`.

`primitive[@]` no usa runtime. Ahí se habla con `anchor`, `slot`, `pattern`, `mask`, `window`, `signal`, `bus`, `state`, `timing`, `emit`, `sample`, `latch`, `assert`, `rupture`, `bridge` y `seal`.


## 2. Sintaxis oficial de importación

```nC
// Importar familia completa
use nC.runtime.input;
use nC.runtime.output;

// Importar miembros concretos
use nC.runtime.input.{readInt, readLine};
use nC.runtime.output.{print, println};

// Alias explícito
use nC.runtime.output as out;

// Capacidad
use nC.runtime.fs with capability("fs.read");

// Política
use nC.runtime.log with policy("security.audit");

// Bridge
use nC.runtime.trace with bridge(kernel, debug);
```

## 3. Errores canónicos del runtime

```txt
RuntimeError
InputReadError
InputParseError
InputEOF
InputTooLong
OutputWriteError
EncodingError
DecodeError
ParseError
ValidationError
ConfigError
EnvAccessError
FsError
PathError
StreamError
BufferError
MemoryError
RegionError
HeapError
ProcessError
TaskError
SyncError
NetworkError
CapabilityError
PolicyError
SandboxViolation
BridgeViolation
PermissionDenied
ResourceUnavailable
UnsupportedRuntimeFeature
```

Uso con `dev()`:

```nC
@linea >> str : readLine()
    !> str : dev(InputReadError @runtime);
```

Uso con `Result<T,E>`:

```nC
@r >> Result<str, InputError> : tryReadLine();

if (isErr(@r)) {
    return false;
}
```


## 4. Índice de las 90 familias


### I. Entrada, salida y entorno

Interacción con usuario, proceso, canales, argumentos, entorno y sesión.

- **01. `nC.runtime.input`** — Entrada externa
- **02. `nC.runtime.output`** — Salida textual base
- **03. `nC.runtime.errorOutput`** — Salida de errores
- **04. `nC.runtime.channel`** — Canales explícitos
- **05. `nC.runtime.flush`** — Vaciado de salida
- **06. `nC.runtime.std`** — Canales estándar
- **07. `nC.runtime.args`** — Argumentos del programa
- **08. `nC.runtime.env`** — Variables de entorno
- **09. `nC.runtime.exit`** — Salida del proceso
- **10. `nC.runtime.session`** — Sesión de ejecución

### II. Texto, parseo y renderizado

Texto, render, formato, concatenación, parseo, ancho visual y Unicode.

- **11. `nC.runtime.text`** — Texto general
- **12. `nC.runtime.string`** — Cadenas `str`
- **13. `nC.runtime.render`** — Renderizado textual
- **14. `nC.runtime.format`** — Formato textual
- **15. `nC.runtime.concat`** — Concatenación explícita
- **16. `nC.runtime.parse`** — Parseo de texto
- **17. `nC.runtime.convert`** — Conversión controlada
- **18. `nC.runtime.validate`** — Validación común
- **19. `nC.runtime.width`** — Ancho visual
- **20. `nC.runtime.align`** — Alineación textual
- **21. `nC.runtime.pad`** — Relleno textual
- **22. `nC.runtime.truncate`** — Corte seguro
- **23. `nC.runtime.wrap`** — Ajuste de líneas
- **24. `nC.runtime.escape`** — Escape y saneamiento
- **25. `nC.runtime.unicode`** — Unicode y UTF

### III. Resultado, errores y diagnóstico

Errores recuperables, diagnósticos, logging, trazas, debug y estados.

- **26. `nC.runtime.result`** — Resultado recuperable
- **27. `nC.runtime.option`** — Valores opcionales
- **28. `nC.runtime.error`** — Errores runtime
- **29. `nC.runtime.diagnostic`** — Diagnósticos estructurados
- **30. `nC.runtime.log`** — Logging
- **31. `nC.runtime.trace`** — Trazas
- **32. `nC.runtime.debug`** — Depuración
- **33. `nC.runtime.assert`** — Aserciones
- **34. `nC.runtime.status`** — Estados operativos

### IV. Memoria, buffers y regiones

Buffers, memoria controlada, heap, stack, regiones, punteros y layout.

- **35. `nC.runtime.buffer`** — Buffers controlados
- **36. `nC.runtime.memory`** — Memoria controlada
- **37. `nC.runtime.heap`** — Heap dinámico
- **38. `nC.runtime.stack`** — Stack runtime
- **39. `nC.runtime.region`** — Regiones de memoria
- **40. `nC.runtime.slice`** — Slices
- **41. `nC.runtime.span`** — Spans
- **42. `nC.runtime.ptr`** — Punteros controlados
- **43. `nC.runtime.addr`** — Direcciones lógicas
- **44. `nC.runtime.bytes`** — Bytes
- **45. `nC.runtime.endian`** — Endianness
- **46. `nC.runtime.layout`** — Layout de tipos
- **47. `nC.runtime.alloc`** — Asignadores

### V. Sistema de archivos y sistema operativo

FS, rutas, archivos, procesos, comandos, OS, usuario, servicios y señales.

- **48. `nC.runtime.fs`** — Sistema de archivos
- **49. `nC.runtime.path`** — Rutas
- **50. `nC.runtime.dir`** — Directorios
- **51. `nC.runtime.file`** — Handles de archivo
- **52. `nC.runtime.stream`** — Streams
- **53. `nC.runtime.process`** — Procesos
- **54. `nC.runtime.command`** — Comandos controlados
- **55. `nC.runtime.os`** — Sistema operativo
- **56. `nC.runtime.user`** — Usuario del sistema
- **57. `nC.runtime.permission`** — Permisos del entorno
- **58. `nC.runtime.temp`** — Temporales
- **59. `nC.runtime.service`** — Servicios del sistema
- **60. `nC.runtime.signal`** — Señales del SO

### VI. Tiempo, reloj y aleatoriedad

Tiempo humano, reloj técnico, timers, pausas, aleatoriedad y UUID.

- **61. `nC.runtime.time`** — Tiempo humano/sistema
- **62. `nC.runtime.clock`** — Reloj monotónico
- **63. `nC.runtime.timer`** — Temporizadores
- **64. `nC.runtime.sleep`** — Suspensión
- **65. `nC.runtime.random`** — Aleatoriedad no criptográfica
- **66. `nC.runtime.entropy`** — Entropía
- **67. `nC.runtime.uuid`** — UUID

### VII. Concurrencia y sincronización

Tareas, hilos, async, sync, atomic, locks, colas, eventos y cancelación.

- **68. `nC.runtime.task`** — Tareas ligeras
- **69. `nC.runtime.thread`** — Hilos del sistema
- **70. `nC.runtime.async`** — Async/Future
- **71. `nC.runtime.sync`** — Sincronización
- **72. `nC.runtime.atomic`** — Atómicos
- **73. `nC.runtime.lock`** — Locks avanzados
- **74. `nC.runtime.queue`** — Colas concurrentes
- **75. `nC.runtime.event`** — Eventos runtime
- **76. `nC.runtime.scheduler`** — Scheduler runtime
- **77. `nC.runtime.cancel`** — Cancelación

### VIII. Red y comunicación

Red general, sockets, TCP, UDP, DNS, HTTP y TLS.

- **78. `nC.runtime.net`** — Red general
- **79. `nC.runtime.socket`** — Sockets
- **80. `nC.runtime.tcp`** — TCP
- **81. `nC.runtime.udp`** — UDP
- **82. `nC.runtime.dns`** — DNS
- **83. `nC.runtime.http`** — HTTP
- **84. `nC.runtime.tls`** — TLS

### IX. Capacidades, políticas y aislamiento

Capacidades, políticas, perfiles, bridges, sandbox y recursos.

- **85. `nC.runtime.capability`** — Capacidades
- **86. `nC.runtime.policy`** — Políticas
- **87. `nC.runtime.profile`** — Perfil activo
- **88. `nC.runtime.bridge`** — Puentes
- **89. `nC.runtime.sandbox`** — Aislamiento
- **90. `nC.runtime.resource`** — Recursos


---

# 5. Familias oficiales, una por una


## I. Entrada, salida y entorno

Interacción con usuario, proceso, canales, argumentos, entorno y sesión.


### 01. `nC.runtime.input` — Entrada externa

**Qué hace.** Leer entrada externa desde usuario, stdin u otro canal textual autorizado. Convierte el mundo exterior en datos del programa.

**Import canónico.**

```nC
use stoud.io { input };
```

**Funciones principales.**

```txt
readLine() -> str
readInt() -> int
readFloat() -> ft64
readBool() -> bool
readChar() -> char
tryReadLine() -> Result<str, InputError>
tryReadInt() -> Result<int, InputError>
tryReadFloat() -> Result<ft64, InputError>
tryReadBool() -> Result<bool, InputError>
clearLine() -> bool
hasInput() -> bool
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@ai] bajo permiso.

**Restringido en.** [@system] solo para consola administrativa; [@security] solo entrada segura/política.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** La entrada humana es una operación alta: depende de stdin, buffer, encoding, parseo y errores de entorno.

**Ejemplo canónico.**

```nC
use nC.runtime.input;
use nC.runtime.output;

print("Edad: ");
@edad >> int : readInt()
    !> int : dev(InputParseError @runtime);
```


### 02. `nC.runtime.output` — Salida textual base

**Qué hace.** Emitir salida textual base a stdout mediante funciones cómodas de escritura. No sustituye la familia canónica `print.*`; sirve como servicio runtime de salida.

**Import canónico.**

```nC
use stoud.io { output };
```

**Funciones principales.**

```txt
print(value) -> void
println(value) -> void
println() -> void
write(text: str) -> Result<void, OutputError>
writeln(text: str) -> Result<void, OutputError>
flush() -> bool
color(text: str, name: str) -> str
colorBold(text: str, name: str) -> str
bgColor(text: str, name: str) -> str
style(text: str, style: str) -> str
emoji(name: str) -> str
progressBar(label: str, current: int, total: int, width: int) -> str
spinnerFrame(label: str, frame: int) -> str
table2(leftHeader: str, rightHeader: str, leftValue: str, rightValue: str) -> str
table3(colA: str, colB: str, colC: str, valueA: str, valueB: str, valueC: str) -> str
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@inspect], [@ai].

**Restringido en.** [@system] controlado; [@security] con redacción; [@kernel] solo bridge debug; [@machine] casi nunca.

**Prohibido en.** [@], primitive[@].

**Por qué.** La salida textual depende de canal, buffer, terminal y política de renderizado.

**Ejemplo canónico.**

```nC
use nC.runtime.output;

println("Hola desde nC");
println(color("estado=ok", "green"));
println("emoji=" + emoji("rocket"));
print("> ");
flush();
```


### 03. `nC.runtime.errorOutput` — Salida de errores

**Qué hace.** Emitir errores, avisos y mensajes operativos por stderr sin mezclar resultado normal con fallo.

**Import canónico.**

```nC
use nC.runtime.errorOutput;
```

**Funciones principales.**

```txt
eprint(value) -> void
eprintln(value) -> void
warn(value) -> void
error(value) -> void
flushErr() -> bool
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@system], [@inspect], [@security].

**Restringido en.** [@kernel] solo diagnóstico; [@machine] solo bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** stderr separa errores y warnings del flujo normal del programa.

**Ejemplo canónico.**

```nC
use nC.runtime.errorOutput;

eprintln("Error: archivo no encontrado.");
warn("Import no usado.");
```


### 04. `nC.runtime.channel` — Canales explícitos

**Qué hace.** Trabajar con stdin, stdout, stderr u otros canales de entrada/salida de forma explícita.

**Import canónico.**

```nC
use nC.runtime.channel;
```

**Funciones principales.**

```txt
stdin() -> InputChannel
stdout() -> OutputChannel
stderr() -> OutputChannel
writeTo(channel: OutputChannel, text: str) -> Result<void, OutputError>
readFrom(channel: InputChannel) -> Result<str, InputError>
flushChannel(channel: OutputChannel) -> bool
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@inspect], [@network] para streams adaptados.

**Restringido en.** [@security] con política; [@kernel]/[@machine] mediante bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Expone la frontera canal/stream y por eso requiere más intención que `println()`.

**Ejemplo canónico.**

```nC
use nC.runtime.channel;

writeTo(stdout(), "OK");
writeTo(stderr(), "ERROR");
```


### 05. `nC.runtime.flush` — Vaciado de salida

**Qué hace.** Controlar cuándo se vacían los buffers de salida. Es clave antes de pedir entrada interactiva.

**Import canónico.**

```nC
use nC.runtime.flush;
```

**Funciones principales.**

```txt
flush() -> bool
flushStdout() -> bool
flushStderr() -> bool
autoFlushOnInput(enabled: bool) -> bool
flushBeforeInput() -> bool
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@system].

**Restringido en.** [@security] controlado; [@kernel] bridge.

**Prohibido en.** [@machine] salvo bridge, [@], primitive[@].

**Por qué.** Evita prompts invisibles y permite control exacto del flujo interactivo.

**Ejemplo canónico.**

```nC
use nC.runtime.output;
use nC.runtime.flush;
use nC.runtime.input;

print("> ");
flush();
@linea >> str : readLine();
```


### 06. `nC.runtime.std` — Canales estándar

**Qué hace.** Acceso a los canales estándar y al modo interactivo del proceso.

**Import canónico.**

```nC
use nC.runtime.std;
```

**Funciones principales.**

```txt
stdIn() -> addr
stdOut() -> addr
stdErr() -> addr
isInteractive() -> bool
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@security], [@kernel] bridge, [@machine] bridge extremo.

**Prohibido en.** [@], primitive[@].

**Por qué.** Expone descriptores base del proceso, no una abstracción humana pura.

**Ejemplo canónico.**

```nC
use nC.runtime.std;

@out >> OutputChannel : stdOut();
```


### 07. `nC.runtime.args` — Argumentos del programa

**Qué hace.** Materializar manualmente los argumentos del proceso como un descriptor raw.

**Import canónico.**

```n
use <process:args>;
```

**Funciones principales.**

```txt
argsOpen() -> addr
argsCount(handle: addr) -> int
argsAt(handle: addr, index: int) -> str
argsData(handle: addr) -> addr
argsBytes(handle: addr) -> int
argsError(handle: addr) -> int
argsRelease(handle: addr) -> bool
```

**Permitido en.** Código normal, [@runtime], [@system], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@security] con validación estricta.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** El descriptor, sus punteros y su vida pertenecen al programador. No hay lista, bounds check ni liberación implícita.

**Ejemplo canónico.**

```n
use <process:args>;

handle >> addr <- argsOpen();
first >> str <- argsAt(handle, 1);
argsRelease(handle);
```


### 08. `nC.runtime.env` — Variables de entorno

**Qué hace.** Consultar o modificar variables del entorno del proceso.

**Import canónico.**

```n
use <process:env>;
```

**Funciones principales.**

```txt
envOpen() -> addr
envCount(handle: addr) -> int
envAt(handle: addr, index: int) -> str
envData(handle: addr) -> addr
envBytes(handle: addr) -> int
envError(handle: addr) -> int
envRelease(handle: addr) -> bool
envGet(name: str, destination: addr, capacity: int) -> int
envHas(name: str) -> bool
envSet(name: str, value: str) -> bool
envRemove(name: str) -> bool
envExpand(source: str, destination: addr, capacity: int) -> int
envLastError() -> int
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler].

**Restringido en.** [@security] solo lectura controlada; [@ai] normalmente no.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** El entorno puede contener tokens, rutas, secretos y configuración sensible.

**Ejemplo canónico.**

```n
use <memory:stack>;
use <process:env>;

mark >> p<byte> <- stackMark();
buffer >> p<byte> <- stackAlloc(256, 8);
written >> int <- envGet("HOME", adr(buffer), 256);
stackRestore(mark);
```


### 09. `nC.runtime.exit` — Salida del proceso

**Qué hace.** Finalizar el programa o establecer código de salida de forma controlada.

**Import canónico.**

```n
use <process:exit>;
```

**Funciones principales.**

```txt
processCurrent() -> addr
processOpen(access: int, inherit: bool, pid: int) -> addr
processId(handle: addr) -> int
processExitCode(handle: addr) -> int
processTerminate(handle: addr, code: int) -> bool
processClose(handle: addr) -> bool
exitNow(code: int) -> bool
failFast(exceptionRecord: addr, contextRecord: addr, flags: int) -> bool
processLastError() -> int
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@system].

**Restringido en.** [@security] con política; [@inspect] no debería cortar ejecución salvo permiso.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Cortar el proceso altera control de flujo global.

**Ejemplo canónico.**

```n
use <process:exit>;

current >> addr <- processCurrent();
pid >> int <- processId(current);
if (pid == 0) { exitNow(1); }
```


### 10. `nC.runtime.session` - sesiones OS manuales

**Que hace.** Expone Remote Desktop Services, tokens, notificaciones y canales
virtuales mediante handles, buffers y flags raw.

**Import canonico.**

```nC
use <process:session>;
```

**Funciones principales.** `sessionCurrentId`, `sessionProcessId`,
`sessionEnumerate`, `sessionQuery`, `sessionQueryUserToken`,
`sessionSendMessage`, `sessionDisconnect`, `sessionLogoff`,
`sessionWaitEvent` y la familia `sessionChannel*`.

**Memoria.** WTS escribe punteros y contadores en slots proporcionados por el
programador. Toda memoria WTS se libera con `sessionFree`; UTF-16 creado por
`sessionWideAlloc` se libera con `sessionWideFree`.

**Backend.** Win64 usa `WTSAPI32.dll`/`KERNEL32.dll` directamente desde
LLVM/MCT y Machine. No genera codigo C ni fallback administrado.


## II. Texto, parseo y renderizado

Texto, render, formato, concatenación, parseo, ancho visual y Unicode.


### 11. `nC.runtime.text` — Texto general

**Qué hace.** Operaciones generales sobre texto: longitud semántica, búsqueda, limpieza, sustitución y división.

**Import canónico.**

```nC
use nC.runtime.text;
```

**Funciones principales.**

```txt
textLength(text: str) -> u64
textIsEmpty(text: str) -> bool
textContains(text: str, part: str) -> bool
textReplace(text: str, from: str, to: str) -> str
textTrim(text: str) -> str
textSplit(text: str, sep: str) -> list<str>
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@ai], [@system].

**Restringido en.** [@security] con sanitización.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Texto de alto nivel requiere encoding y memoria gestionada.

**Ejemplo canónico.**

```nC
@limpio >> str : textTrim(@entrada);
```


### 12. `nC.runtime.string` — Cadenas `str`

**Qué hace.** Operaciones concretas sobre el tipo `str`, más compactas que `text` amplio.

**Import canónico.**

```nC

```

**Funciones principales.**

```txt
strEmpty() -> str
strRelease(owned: str) -> bool
strConcat(a: str, b: str) -> str
strSlice(s: str, start: u64, len: u64) -> str
charFromCode(code: int) -> str
strStartsWith(s: str, prefix: str) -> bool
strEndsWith(s: str, suffix: str) -> bool
strToUpper(s: str) -> str
strToLower(s: str) -> str
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@ai].

**Restringido en.** [@security] con redacción/sanitización.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** `str` vive en capas gestionadas, no en materia basal.

**Ejemplo canónico.**

```nC
@nombreCompleto >> str : strConcat(@nombre, @apellido);
@usado >> bool : procesar(@nombreCompleto);
@liberado >> bool : strRelease(@nombreCompleto); # último uso; nunca literales/borrowed
```


### 13. `nC.runtime.render` — Renderizado textual

**Qué hace.** Convierte valores internos a texto de forma explícita y controlada.

**Import canónico.**

```nC
use stoud.io { render };
```

**Funciones principales.**

```txt
renderInt(value: int) -> str
renderUInt(value: u64) -> str
renderFloat(value: ft64, precision: u8) -> str
renderBool(value: bool) -> str
renderChar(value: char) -> str
renderType(value) -> str
renderDebug(value) -> str
renderStable(value) -> str
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@inspect], [@system].

**Restringido en.** [@security] no renderizar secretos; [@kernel]/[@machine] bridge debug.

**Prohibido en.** [@], primitive[@].

**Por qué.** Evita conversiones mágicas: valor interno -> texto visible.

**Ejemplo canónico.**

```nC
println("Edad: " + renderInt(@edad));
```


### 14. `nC.runtime.format` — Formato textual

**Qué hace.** Plantillas, formato de enteros/float y composición textual de alto nivel.

**Import canónico.**

```nC
use nC.runtime.format;
```

**Funciones principales.**

```txt
format(template: str, values...) -> str
formatNamed(template: str, values...) -> str
formatInt(value: int, spec: FormatSpec) -> str
formatFloat(value: ft64, spec: FormatSpec) -> str
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@inspect], [@ai].

**Restringido en.** [@security] con redacción; [@system] controlado.

**Prohibido en.** [@kernel] salvo bridge debug, [@machine], [@], primitive[@].

**Por qué.** El formato depende de locale, precisión, buffers y render textual.

**Ejemplo canónico.**

```nC
@msg >> str : format("{} tiene {} años", @nombre, renderInt(@edad));
```


### 15. `nC.runtime.concat` — Concatenación explícita

**Qué hace.** Une texto con texto sin ocultar conversiones. Es la bajada explícita del `+` textual estricto.

**Import canónico.**

```nC
use nC.runtime.concat;
```

**Funciones principales.**

```txt
concat(a: str, b: str) -> str
concat3(a: str, b: str, c: str) -> str
join(separator: str, values: list<str>) -> str
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@ai].

**Restringido en.** [@security], [@system].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** La concatenación explícita evita magia tipo texto->número.

**Ejemplo canónico.**

```nC
@msg >> str : concat("Edad: ", renderInt(@edad));
```


### 16. `nC.runtime.parse` — Parseo de texto

**Qué hace.** Convierte texto externo en tipos internos con error explícito.

**Import canónico.**

```nC
use nC.runtime.parse;
```

**Funciones principales.**

```txt
parseInt(text: str) -> int
parseUInt(text: str) -> u64
parseFloat(text: str) -> ft64
parseBool(text: str) -> bool
parseChar(text: str) -> char
tryParseInt(text: str, out: p<int>) -> bool
tryParseFloat(text: str, out: p<ft64>) -> bool
tryParseBool(text: str, out: p<bool>) -> bool
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler], [@system], [@ai].

**Restringido en.** [@security] con límites de tamaño y forma.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Contrato manual.** El llamador aporta el puntero de salida. No se crea un
`Result`, no se reserva memoria y el slot no se modifica cuando el parseo falla.

**Ejemplo canónico.**

```nC
edad >> int <- 0;
guard tryParseInt(linea, &edad) else return false;
```


### 17. `nC.runtime.convert` — Conversión controlada

**Qué hace.** Convierte valores con semántica explícita y sin conversiones peligrosas implícitas.

**Import canónico.**

```nC
use nC.runtime.convert;
```

**Funciones principales.**

```txt
toStr(value) -> str
toText(value) -> text
toInt(value) -> Result<int, ConvertError>
toFloat(value) -> Result<ft64, ConvertError>
toBool(value) -> Result<bool, ConvertError>
safeToInt(value) -> Result<int, ConvertError>
forceToInt(value) -> int
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@compiler].

**Restringido en.** [@security], [@system].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Distingue conversión visual segura de coerción numérica peligrosa.

**Ejemplo canónico.**

```nC
@texto >> str : toStr(@edad);
```


### 18. `nC.runtime.validate` — Validación raw

**Qué hace.** Ejecuta predicados explícitos sobre direcciones, longitudes y
enteros sin crear objetos, diagnósticos ni checks persistentes.

**Import canónico.** `use <audit:validate>;`.

```txt
validateNotNull(addr) -> bool
validateRange(value, minimum, maximum) -> bool
validateLength(length, minimum, maximum) -> bool
validateEqual(left, leftBytes, right, rightBytes) -> bool
validateAscii(data, bytes) -> bool
validateUtf8(data, bytes) -> bool
validateAligned(addr, alignment) -> bool
validatePowerOfTwo(value) -> bool
validateZero(data, bytes) -> bool
validateMask(value, required, forbidden) -> bool
```

**Semántica.** Las funciones no asignan memoria ni protegen accesos posteriores.
Una dirección inaccesible conserva comportamiento indefinido raw. La
materialización es LLVM/Machine interna y no depende de C.

**Ejemplo canónico.** `examples/runtime/families/audit_manual_family_raw.n`.


### 19. `nC.runtime.width` — Ancho visual

**Qué hace.** Calcula ancho real en terminal, distinto de bytes o caracteres.

**Import canónico.**

```nC
use nC.runtime.width;
```

**Funciones principales.**

```txt
displayWidth(text: str) -> u64
byteLength(text: str) -> u64
charCount(text: str) -> u64
graphemeCount(text: str) -> u64
ansiVisibleWidth(text: str) -> u64
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@ai].

**Restringido en.** [@system], [@security].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Tablas, paneles y diagnósticos se rompen si se usa `length()` como ancho visual.

**Ejemplo canónico.**

```nC
@w >> u64 : displayWidth("🧠 nC");
```


### 20. `nC.runtime.align` — Alineación textual

**Qué hace.** Alinea texto según ancho visual real.

**Import canónico.**

```nC
use nC.runtime.align;
```

**Funciones principales.**

```txt
alignLeft(text: str, width: u64) -> str
alignRight(text: str, width: u64) -> str
alignCenter(text: str, width: u64) -> str
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@system] controlado.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Layout humano pertenece a capas altas.

**Ejemplo canónico.**

```nC
println(alignRight(renderInt(@edad), 4));
```


### 21. `nC.runtime.pad` — Relleno textual

**Qué hace.** Rellena texto a izquierda, derecha o ambos lados.

**Import canónico.**

```nC
use nC.runtime.pad;
```

**Funciones principales.**

```txt
padLeft(text: str, width: u64, fill: char) -> str
padRight(text: str, width: u64, fill: char) -> str
padBoth(text: str, width: u64, fill: char) -> str
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@security] cuidado con ocultación visual.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Relleno es presentación, no máquina.

**Ejemplo canónico.**

```nC
@codigo >> str : padLeft("7", 3, '0');
```


### 22. `nC.runtime.truncate` — Corte seguro

**Qué hace.** Corta texto sin romper graphemes, ANSI o ancho visual.

**Import canónico.**

```nC
use nC.runtime.truncate;
```

**Funciones principales.**

```txt
truncate(text: str, width: u64) -> str
truncateEnd(text: str, width: u64) -> str
truncateMiddle(text: str, width: u64) -> str
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@security] cuidado con ocultar datos relevantes.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Un corte bruto puede romper Unicode o falsear diagnósticos.

**Ejemplo canónico.**

```nC
@rutaCorta >> str : truncateMiddle(@ruta, 60);
```


### 23. `nC.runtime.wrap` — Ajuste de líneas

**Qué hace.** Envuelve texto por ancho de pantalla o documento.

**Import canónico.**

```nC
use nC.runtime.wrap;
```

**Funciones principales.**

```txt
wrap(text: str, width: u64) -> str
wrapWords(text: str, width: u64) -> str
indent(text: str, spaces: u64) -> str
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@ai].

**Restringido en.** [@system] controlado.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** El wrapping es presentación humana, no ejecución basal.

**Ejemplo canónico.**

```nC
println(wrap(@descripcion, 80));
```


### 24. `nC.runtime.escape` — Escape y saneamiento

**Qué hace.** Escapa ANSI, JSON, caracteres de control y texto peligroso para terminal/log.

**Import canónico.**

```nC
use nC.runtime.escape;
```

**Funciones principales.**

```txt
escapeAnsi(text: str) -> str
escapeJson(text: str) -> str
escapeControl(text: str) -> str
sanitizeTerminal(text: str) -> str
sanitizeLog(text: str) -> str
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@security], [@system].

**Restringido en.** Obligatorio en [@security] con entrada externa.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Previene terminal injection, logs rotos y salida maliciosa.

**Ejemplo canónico.**

```nC
println(sanitizeTerminal(@entradaUsuario));
```


### 25. `nC.runtime.unicode` — Unicode y UTF

**Qué hace.** Normalización, validación y limpieza Unicode/UTF.

**Import canónico.**

```nC
use nC.runtime.unicode;
```

**Funciones principales.**

```txt
isValidUtf8(text: str) -> bool
normalizeUnicode(text: str) -> str
stripInvalidUnicode(text: str) -> str
unicodeMode() -> UnicodeMode
supportsGrapheme() -> bool
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@ai].

**Restringido en.** [@security], [@system].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Texto moderno exige tratar graphemes y UTF explícitamente.

**Ejemplo canónico.**

```nC
@normal >> str : normalizeUnicode(@entrada);
```


## III. Resultado, errores y diagnóstico

Errores recuperables, diagnósticos, logging, trazas, debug y estados.


### 26. `nC.runtime.result` — Resultado recuperable

**Qué hace.** Modelo `Result<T,E>` para éxito/error sin excepción mágica.

**Import canónico.**

```nC
use stoud.collections { result };
```

**Funciones principales.**

```txt
ok(value) -> Result<T, E>
err(error) -> Result<T, E>
isOk(result) -> bool
isErr(result) -> bool
unwrap(result) -> T
unwrapOr(result, fallback) -> T
errorOf(result) -> E
```

**Permitido en.** Todos los modos altos y medios: [@runtime], [@system], [@security], [@inspect], [@tooling], [@compiler].

**Restringido en.** [@kernel] con bridge; [@machine] muy limitado.

**Prohibido en.** [@], primitive[@].

**Por qué.** Permite errores recuperables explícitos, estilo Rust pero con semántica nC.

**Ejemplo canónico.**

```nC
@r >> Result<int, InputError> : tryReadInt(); if (isErr(@r)) { return false; }
```


### 27. `nC.runtime.option` — Valores opcionales

**Qué hace.** Modelo `Option<T>` para presencia/ausencia sin null libre.

**Import canónico.**

```nC
use nC.runtime.option;
```

**Funciones principales.**

```txt
some(value) -> Option<T>
none() -> Option<T>
isSome(option) -> bool
isNone(option) -> bool
unwrapOption(option) -> T
unwrapOptionOr(option, fallback) -> T
```

**Permitido en.** Código normal, [@runtime], [@system], [@tooling], [@compiler], [@inspect], [@security].

**Restringido en.** [@machine] limitado.

**Prohibido en.** [@], primitive[@].

**Por qué.** Evita nullabilidad implícita y errores por ausencia no tratada.

**Ejemplo canónico.**

```n
value >> Option<int> <- some(7);
present >> bool <- isSome(value);
```


### 28. `nC.runtime.error` — Errores runtime

**Qué hace.** Consulta y emite errores de runtime de forma estructurada.

**Import canónico.**

```nC
use nC.runtime.error;
```

**Funciones principales.**

```txt
lastError() -> Option<RuntimeError>
errorCode(error) -> int
errorMessage(error) -> str
errorKind(error) -> str
clearError() -> bool
raiseRuntime(error) -> never
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@inspect], [@security].

**Restringido en.** [@kernel]/[@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** El error runtime necesita contrato y no debe ser excepción invisible.

**Ejemplo canónico.**

```nC
@err >> Option<RuntimeError> : lastError();
```


### 29. `nC.runtime.diagnostic` — Diagnósticos raw

**Qué hace.** Escribe registros binarios de diagnostico en memoria del
programa, sin construir objetos ni emitir salida.

**Import canónico.**

```nC
use <audit:diagnostic>;
```

**Layout.** Cada entrada ocupa 64 bytes: severidad, codigo, mensaje prestado,
longitud, fuente prestada, linea, columna y flags.

**Funciones principales.** `diagStride`, `diagWrite`, `diagSeverity`,
`diagCode`, `diagMessage`, `diagMessageBytes`, `diagSource`, `diagLine`,
`diagColumn`, `diagFlags`, `diagCopy` y `diagClear`.

**Semántica.** El programa controla memoria, capacidad, codificacion, formato y
salida. No hay strings, JSON, spans administrados, bounds ni ownership oculto.

**Ejemplo canónico.** `examples/runtime/families/audit_diagnostic_raw.n`.


### 30. `nC.runtime.log` — Registro raw

**Qué hace.** Escribe registros binarios de 48 bytes en memoria aportada por el
programa.

**Import canónico.** `use <audit:log>;`.

```txt
logStride() -> int
logWrite(table, index, level, category, message, messageBytes, value, flags) -> bool
logLevel/logCategory/logMessage/logMessageBytes/logValue/logFlags(table, index)
logCopy(destination, destinationIndex, source, sourceIndex) -> bool
logClear(table, index) -> bool
```

**Semántica.** No hay persistencia, formato, allocation, strings ni bounds
implícitos. El llamador gobierna memoria, índices y vida del mensaje. Se baja a
LLVM/Machine sin proveedor C.

**Ejemplo canónico.** `examples/runtime/families/audit_manual_family_raw.n`.


### 31. `nC.runtime.trace` — Trazas raw

**Qué hace.** Escribe registros binarios de 56 bytes con kind, id, parent,
tick, payload y valor.

**Import canónico.** `use <audit:trace>;`.

```txt
traceStride() -> int
traceWrite(table, index, kind, id, parent, tick, payload, payloadBytes, value) -> bool
traceKind/traceId/traceParent/traceTick(table, index) -> int
tracePayload(table, index) -> addr
tracePayloadBytes/traceValue(table, index) -> int
traceCopy(destination, destinationIndex, source, sourceIndex) -> bool
traceClear(table, index) -> bool
```

**Semántica.** No hay reloj, sincronización, allocation ni persistencia oculta.
El programa decide todos los datos y el almacenamiento. Backend LLVM/Machine
interno, sin C.

**Ejemplo canónico.** `examples/runtime/families/audit_manual_family_raw.n`.


### 32. `nC.runtime.debug` — Depuración raw

**Qué hace.** Registra y copia bytes de depuracion en memoria aportada por el
programa, calcula checksum y ofrece traps explicitos.

**Import canónico.**

```nC
use <audit:debug>;
```

**Layout.** Cada registro ocupa 32 bytes: `tag:i64`, `location:ptr`,
`value:i64` y `flags:i64`.

**Funciones principales.**

```txt
debugStride() -> int
debugRecord(buffer: addr, index: int, tag: int, location: addr, value: int, flags: int) -> bool
debugTag(buffer: addr, index: int) -> int
debugLocation(buffer: addr, index: int) -> addr
debugValue(buffer: addr, index: int) -> int
debugFlags(buffer: addr, index: int) -> int
debugDump(destination: addr, source: addr, bytes: int) -> int
debugChecksum(source: addr, bytes: int) -> int
debugBreak() -> bool
debugAssert(condition: bool, context: addr, bytes: int) -> bool
```

**Semántica.** Sin allocation, strings, captura de estado, formato, bounds ni
recuperacion oculta. `debugBreak` y un `debugAssert` falso ejecutan trap.

**Ejemplo canónico.** `examples/runtime/families/audit_debug_raw.n`.


### 33. `nC.runtime.assert` — Aserciones raw

**Qué hace.** Comprueba invariantes explicitos y ejecuta trap al fallar.

**Import canónico.**

```nC
use nC.runtime.assert;
```

**Funciones principales.**

```txt
assert(condition: bool) -> bool
assertMsg(condition: bool, message: addr, bytes: int) -> bool
assertEq(left: int, right: int) -> bool
assertEqAddr(left: addr, right: addr) -> bool
assertNotNull(value: addr) -> bool
assertRange(value: int, minimum: int, maximum: int) -> bool
assertFail() -> bool
assertUnreachable() -> bool
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@system].

**Restringido en.** [@security], [@kernel]/[@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** No hay logging, allocation, recuperacion ni proveedor C ocultos.

**Ejemplo canónico.**

```nC
assertMsg(edad >= 0, messageAddress, messageBytes);
```


### 34. `nC.runtime.status` — Estado raw

**Qué hace.** Escribe registros binarios de 40 bytes con kind, code, mensaje y
detalle.

**Import canónico.** `use <audit:status>;`.

```txt
statusStride() -> int
statusWrite(table, index, kind, code, message, messageBytes, detail) -> bool
statusKind/statusCode/statusMessage/statusMessageBytes/statusDetail(table, index)
statusCopy(destination, destinationIndex, source, sourceIndex) -> bool
statusClear(table, index) -> bool
```

**Semántica.** Mensaje y almacenamiento son direcciones prestadas y gobernadas
por el programa. No existen Status gestionado, render automático ni bounds
ocultos. Backend LLVM/Machine interno, sin C.

**Ejemplo canónico.** `examples/runtime/families/audit_manual_family_raw.n`.


## IV. Memoria, buffers y regiones

Buffers, memoria controlada, heap, stack, regiones, punteros y layout.


### 35. `nC.runtime.buffer` — Buffers controlados

**Qué hace.** Crea y manipula buffers/zonas contiguas bajo control runtime.

**Import canónico.**

```nC
use nC.runtime.buffer;
```

**Funciones principales.**

```txt
fixedBuffer<T>(size: u64) -> memRegion<T>
bufferLen(buffer) -> u64
bufferRead(buffer, index: u64) -> T
bufferWrite(buffer, index: u64, value: T) -> bool
bufferSlice(buffer, start: u64, len: u64) -> memRegion<T>
bufferClear(buffer) -> bool
```

**Permitido en.** [@runtime], [@system], [@network], [@inspect], [@security], [@tooling].

**Restringido en.** [@kernel] bridge; [@machine] fixed/readonly bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Buffer runtime aún tiene límites y ownership gestionado.

**Ejemplo canónico.**

```nC
@buffer >> memRegion<byte> : fixedBuffer<byte>(64) !> memRegion<byte> : dev(BufferCreateError @runtime);
```


### 36. `nC.runtime.memory` — Memoria controlada

**Qué hace.** Operaciones controladas sobre memoria: tamaño, alineación, copia, cero y comparación.

**Import canónico.**

```nC
use nC.runtime.memory;
```

**Funciones principales.**

```txt
sizeOf(type) -> u64
alignOf(type) -> u64
memoryCopy(dst, src, len) -> bool
memoryZero(region) -> bool
memoryCompare(a, b, len) -> int
```

**Permitido en.** [@runtime], [@system], [@inspect], [@security].

**Restringido en.** [@kernel]/[@machine] bridge.

**Prohibido en.** Código alto normal salvo permisos, [@], primitive[@].

**Por qué.** La memoria controlada sigue siendo peligrosa si se usa sin contrato.

**Ejemplo canónico.**

```nC
@tam >> u64 : sizeOf(int);
```


### 37. `nC.runtime.heap` — Heap dinámico

**Qué hace.** Asignación/liberación dinámica controlada por runtime.

**Import canónico.**

```nC
use stoud.memory { heap };
```

**Funciones principales.**

```txt
heapAlloc<T>(count: u64) -> Result<ptr<T>, HeapError>
heapFree<T>(ptr: ptr<T>) -> Result<void, HeapError>
heapStats() -> HeapStats
heapCheck() -> bool
```

**Permitido en.** [@runtime], [@system], [@inspect], [@tooling] limitado.

**Restringido en.** [@security] política estricta; [@kernel] normalmente no; [@machine] casi nunca.

**Prohibido en.** [@], primitive[@].

**Por qué.** Heap implica vida, liberación, fragmentación y errores de asignación.

**Ejemplo canónico.**

```nC
@ptr >> ptr<byte> : heapAlloc<byte>(512) !> ptr<byte> : dev(HeapAllocError @runtime);
```


### 38. `nC.runtime.stack` — Stack runtime

**Qué hace.** Consulta información de pila gestionada y trazas de stack.

**Import canónico.**

```nC
use nC.runtime.stack;
```

**Funciones principales.**

```txt
stackDepth() -> u64
stackLimit() -> u64
stackRemaining() -> u64
stackTrace() -> StackTrace
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@system].

**Restringido en.** [@security], [@kernel]/[@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** La pila revela flujo y layout de ejecución.

**Ejemplo canónico.**

```nC
@depth >> u64 : stackDepth();
```


### 39. `nC.runtime.region` — Regiones de memoria

**Qué hace.** Crear, leer, escribir e invalidar regiones runtime controladas.

**Import canónico.**

```nC
use stoud.memory { region };
```

**Funciones principales.**

```txt
regionCreate<T>(len: u64) -> memRegion<T>
regionLen(region) -> u64
regionRead(region, index: u64) -> T
regionWrite(region, index: u64, value: T) -> bool
regionReadonly(region) -> memRegion<T>
regionInvalidate(region) -> bool
```

**Permitido en.** [@runtime], [@system], [@inspect], [@security], [@network].

**Restringido en.** [@kernel] bridge; [@machine] readonly/fixed bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Región explicita límites, ownership y validez.

**Ejemplo canónico.**

```nC
@region >> memRegion<byte> : regionCreate<byte>(128);
```


### 40. `nC.runtime.slice` — Slices

**Qué hace.** Vistas no propietarias sobre regiones o buffers.

**Import canónico.**

```nC
use stoud.collections { slice };
```

**Funciones principales.**

```txt
slice(region, start: u64, len: u64) -> Slice<T>
sliceLen(slice) -> u64
sliceRead(slice, index: u64) -> T
sliceIsEmpty(slice) -> bool
```

**Permitido en.** [@runtime], [@system], [@network], [@inspect], [@security].

**Restringido en.** [@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Slice debe respetar bounds y vida de región origen.

**Ejemplo canónico.**

```nC
@parte >> Slice<byte> : slice(@buffer, 0, 16);
```


### 41. `nC.runtime.span` — Spans

**Qué hace.** Rangos semánticos de texto, código o memoria lógica, útil para diagnósticos.

**Import canónico.**

```nC
use stoud.collections { span };
```

**Funciones principales.**

```txt
spanCreate(start: u64, end: u64) -> Span
spanLen(span) -> u64
spanJoin(a: Span, b: Span) -> Span
spanContains(span, pos: u64) -> bool
```

**Permitido en.** [@compiler], [@tooling], [@inspect], [@runtime], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Span representa ubicación semántica, no puntero físico.

**Ejemplo canónico.**

```nC
@span >> Span : spanCreate(10, 25);
```


### 42. `nC.runtime.ptr` — Punteros controlados

**Qué hace.** Punteros bajo contrato runtime: null, cast, read/write controlado.

**Import canónico.**

```nC
use stoud.memory { ptr };
```

**Funciones principales.**

```txt
ptrIsNull(ptr) -> bool
ptrAddr(ptr) -> addr
ptrCast<T>(ptr) -> Result<ptr<T>, PtrError>
ptrRead<T>(ptr) -> Result<T, PtrError>
ptrWrite<T>(ptr, value: T) -> Result<void, PtrError>
```

**Permitido en.** [@system], [@inspect], [@security] controlado.

**Restringido en.** [@runtime] no cotidiano; [@kernel]/[@machine] bridge.

**Prohibido en.** Código alto normal, [@], primitive[@].

**Por qué.** Punteros rompen abstracción si se exponen arriba sin necesidad.

**Ejemplo canónico.**

```nC
if (ptrIsNull(@p)) { dev(NullPtrError @runtime); }
```


### 43. `nC.runtime.addr` — Direcciones lógicas

**Qué hace.** Direcciones lógicas y aritmética controlada de direcciones.

**Import canónico.**

```nC
use stoud.memory { ptr };
```

**Funciones principales.**

```txt
addrOf(value) -> addr
addrAdd(base: addr, offset: u64) -> addr
addrCompare(a: addr, b: addr) -> int
addrIsAligned(address: addr, align: u64) -> bool
```

**Permitido en.** [@system], [@inspect], [@security] restringido.

**Restringido en.** [@kernel]/[@machine] bridge.

**Prohibido en.** Código normal, [@], primitive[@].

**Por qué.** Direcciones exponen layout y pueden filtrar información sensible.

**Ejemplo canónico.**

```nC
@a >> addr : addrOf(@buffer);
```


### 44. `nC.runtime.bytes` — Bytes

**Qué hace.** Conversión, comparación y acceso a regiones de bytes.

**Import canónico.**

```nC
use nC.runtime.bytes;
```

**Funciones principales.**

```txt
byteAt(region, index: u64) -> byte
bytesToHex(region) -> str
hexToBytes(text: str) -> memRegion<byte>
bytesEq(a, b) -> bool
bytesLen(region) -> u64
```

**Permitido en.** [@runtime], [@system], [@network], [@inspect], [@security], [@tooling].

**Restringido en.** [@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Bytes son frontera entre texto, red, memoria y sistema.

**Ejemplo canónico.**

```nC
@hex >> str : bytesToHex(@buffer);
```


### 45. `nC.runtime.endian` — Endianness

**Qué hace.** Conversión little/big endian y consulta de endian nativo.

**Import canónico.**

```nC
use nC.runtime.endian;
```

**Funciones principales.**

```txt
nativeEndian() -> Endian
toLittleEndian(value) -> bytes
toBigEndian(value) -> bytes
fromLittleEndian<T>(bytes) -> T
fromBigEndian<T>(bytes) -> T
```

**Permitido en.** [@runtime], [@system], [@network], [@inspect], [@compiler].

**Restringido en.** [@kernel]/[@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Endianness es frontera binaria y de protocolo.

**Ejemplo canónico.**

```nC
@bytes >> memRegion<byte> : toLittleEndian(@numero);
```


### 46. `nC.runtime.layout` — Layout de tipos

**Qué hace.** Inspección de tamaño, alineación y offsets de tipos/estructuras.

**Import canónico.**

```nC
use nC.runtime.layout;
```

**Funciones principales.**

```txt
layoutOf(type) -> Layout
layoutSize(layout) -> u64
layoutAlign(layout) -> u64
fieldOffset(type, field: str) -> u64
```

**Permitido en.** [@compiler], [@inspect], [@system], [@security] controlado.

**Restringido en.** [@runtime], [@kernel]/[@machine] bridge.

**Prohibido en.** Código alto normal, [@], primitive[@].

**Por qué.** Layout revela representación interna y ABI.

**Ejemplo canónico.**

```nC
@layout >> Layout : layoutOf(Registro);
```


### 47. `nC.runtime.alloc` — Asignadores

**Qué hace.** Selecciona y usa asignadores como default/arena.

**Import canónico.**

```nC
use nC.runtime.alloc;
```

**Funciones principales.**

```txt
defaultAllocator() -> Allocator
arenaAllocator(size: u64) -> Allocator
allocWith<T>(allocator, count: u64) -> Result<ptr<T>, AllocError>
freeWith<T>(allocator, ptr: ptr<T>) -> Result<void, AllocError>
```

**Permitido en.** [@runtime], [@system], [@inspect].

**Restringido en.** [@security], [@kernel] muy restringido, [@machine] casi nunca.

**Prohibido en.** [@], primitive[@].

**Por qué.** Asignador define vida y ownership de memoria dinámica.

**Ejemplo canónico.**

```nC
@arena >> Allocator : arenaAllocator(4096);
```


## V. Sistema de archivos y sistema operativo

FS, rutas, archivos, procesos, comandos, OS, usuario, servicios y señales.


### 48. `nC.runtime.fs` — Sistema de archivos

**Qué hace.** Lectura/escritura básica de archivos por ruta.

**Import canónico.**

```nC
use nC.runtime.fs with capability("fs.read");
```

**Funciones principales.**

```txt
readFile(path: str) -> Result<str, FsError>
writeFile(path: str, content: str) -> Result<void, FsError>
exists(path: str) -> bool
deleteFile(path: str) -> Result<void, FsError>
copyFile(src: str, dst: str) -> Result<void, FsError>
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler].

**Restringido en.** [@security] política dura; [@ai] sandbox; [@inspect] lectura.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** FS toca mundo externo persistente y requiere capacidades.

**Ejemplo canónico.**

```nC
@contenido >> str : readFile("config.n") !> str : dev(FsReadError @runtime);
```


### 49. `nC.runtime.path` — Rutas

**Qué hace.** Manipulación de rutas sin tocar disco necesariamente.

**Import canónico.**

```nC
use nC.runtime.path;
```

**Funciones principales.**

```txt
pathJoin(a: str, b: str) -> str
pathNormalize(path: str) -> str
pathParent(path: str) -> str
pathFileName(path: str) -> str
pathExtension(path: str) -> str
isAbsolute(path: str) -> bool
```

**Permitido en.** Código normal, [@runtime], [@system], [@tooling], [@compiler], [@security].

**Restringido en.** [@ai] según sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Rutas son texto con semántica OS y seguridad.

**Ejemplo canónico.**

```nC
@ruta >> str : pathJoin("src", "main.n");
```


### 50. `nC.runtime.dir` — Directorios

**Qué hace.** Listado, creación, borrado y cambio de directorio.

**Import canónico.**

```nC
use nC.runtime.dir with capability("fs.read");
```

**Funciones principales.**

```txt
listDir(path: str) -> Result<list<str>, FsError>
createDir(path: str) -> Result<bool, FsError>
removeDir(path: str) -> Result<bool, FsError>
currentDir() -> Result<str, FsError>
changeDir(path: str) -> Result<bool, FsError>
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler].

**Restringido en.** [@security], [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Directorios exponen estructura del sistema de archivos.

**Ejemplo canónico.**

```nC
@files >> list<str> : listDir("src") !> list<str> : dev(DirReadError @runtime);
```


### 51. `nC.runtime.file` — Handles de archivo

**Qué hace.** Archivos abiertos como recursos con vida explícita.

**Import canónico.**

```nC
use nC.runtime.file with capability("fs.read");
```

**Funciones principales.**

```txt
fileOpen(path: str, mode: FileMode) -> Result<File, FsError>
fileRead(file: File, len: u64) -> Result<memRegion<byte>, FsError>
fileWrite(file: File, data) -> Result<void, FsError>
fileClose(file: File) -> bool
fileSize(file: File) -> u64
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler].

**Restringido en.** [@security], [@inspect] lectura.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Un handle exige cierre y política de recurso.

**Ejemplo canónico.**

```nC
@f >> File : fileOpen("data.bin", Read) !> File : dev(FileOpenError @runtime);
```


### 52. `nC.runtime.stream` — Streams

**Qué hace.** Lectura/escritura secuencial sobre canales, archivos, sockets u otros recursos.

**Import canónico.**

```nC
use nC.runtime.stream;
```

**Funciones principales.**

```txt
streamRead(stream, buffer, len: u64) -> Result<u64, StreamError>
streamWrite(stream, buffer, len: u64) -> Result<u64, StreamError>
streamFlush(stream) -> bool
streamClose(stream) -> bool
```

**Permitido en.** [@runtime], [@system], [@network], [@tooling], [@compiler].

**Restringido en.** [@security], [@kernel] bridge.

**Prohibido en.** [@machine] normalmente, [@], primitive[@].

**Por qué.** Stream aproxima el mundo byte a byte, pero aún bajo runtime.

**Ejemplo canónico.**

```nC
@bytes >> u64 : streamRead(@input, @buffer, 64) !> u64 : dev(StreamReadError @runtime);
```


### 53. `nC.runtime.process` — Procesos

**Qué hace.** Crear, esperar o terminar procesos del sistema.

**Import canónico.**

```nC
use nC.runtime.process with capability("process.spawn");
```

**Funciones principales.**

```txt
processId() -> u64
parentProcessId() -> u64
spawn(command: str, args: list<str>) -> Result<Process, ProcessError>
wait(process: Process) -> Result<int, ProcessError>
kill(process: Process) -> Result<void, ProcessError>
```

**Permitido en.** [@system], [@tooling], [@compiler].

**Restringido en.** [@runtime] no por defecto; [@security] política estricta; [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Lanzar procesos es una capacidad poderosa y peligrosa.

**Ejemplo canónico.**

```nC
@p >> Process : spawn("ncc", ["check", "main.n"]) !> Process : dev(ProcessSpawnError @runtime);
```


### 54. `nC.runtime.command` — Comandos controlados

**Qué hace.** Ejecutar comandos y capturar salida/estado de forma estructurada.

**Import canónico.**

```nC
use nC.runtime.command with capability("command.run");
```

**Funciones principales.**

```txt
run(command: str) -> Result<CommandResult, CommandError>
runArgs(command: str, args: list<str>) -> Result<CommandResult, CommandError>
commandOutput(result: CommandResult) -> str
commandStatus(result: CommandResult) -> int
```

**Permitido en.** [@tooling], [@compiler], [@system].

**Restringido en.** [@security] muy controlado; [@ai] sandbox fuerte.

**Prohibido en.** Código normal por defecto, [@kernel], [@machine], [@], primitive[@].

**Por qué.** Comandos abren superficie de shell/ejecución externa.

**Ejemplo canónico.**

```nC
@r >> CommandResult : runArgs("git", ["status"]) !> CommandResult : dev(CommandError @runtime);
```


### 55. `nC.runtime.os` — Sistema operativo

**Qué hace.** Información general del sistema operativo huésped.

**Import canónico.**

```nC
use nC.runtime.os;
```

**Funciones principales.**

```txt
osName() -> str
osVersion() -> str
osArch() -> str
osFamily() -> str
isWindows() -> bool
isLinux() -> bool
isMac() -> bool
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@inspect].

**Restringido en.** [@security] evitar fingerprint innecesario.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Identifica entorno y puede condicionar rutas o capacidades.

**Ejemplo canónico.**

```nC
println("OS: " + osName());
```


### 56. `nC.runtime.user` — Usuario del sistema

**Que hace.** Expone identidad, tokens, SID, impersonacion y perfiles como API
raw de buffers y handles.

**Import canónico.**

```n
use <process:user>;
```

**Funciones principales.**

```txt
userBuffer(bytes: int) -> addr
userName(buffer: addr, sizeSlot: addr) -> bool
userOpenProcessToken(process: addr, access: int, outToken: addr) -> bool
userTokenInformation(token: addr, class: int, buffer: addr, cap: int, required: addr) -> bool
userCreateWellKnownSid(kind: int, domain: addr, sid: addr, sizeSlot: addr) -> bool
userImpersonate(token: addr) -> bool
userRevert() -> bool
userLoadProfile(token: addr, profileInfo: addr) -> bool
userClose(handle: addr) -> bool
userFree(memory: addr) -> bool
```

**Materializacion.** LLVM/MCT y Machine llaman directamente a ADVAPI32,
KERNEL32, SECUR32 y USERENV. No hay fuente C ni CRT.

**Permitido en.** Contenedores runtime hosted Win64 con import explicito.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Identidad del usuario es dato sensible y operativo.

**Ejemplo canónico.**

```nC
@home >> str : homeDir() !> str : dev(UserEnvError @runtime);
```


### 57. `nC.runtime.permission` — Permisos del entorno

**Qué hace.** Comprobar permisos de lectura, escritura, ejecución y capacidades de entorno.

**Import canónico.**

```nC
use nC.runtime.permission;
```

**Funciones principales.**

```txt
canRead(path: str) -> bool
canWrite(path: str) -> bool
canExecute(path: str) -> bool
requirePermission(name: str) -> Result<void, PermissionError>
```

**Permitido en.** [@system], [@security], [@tooling], [@compiler], [@runtime].

**Restringido en.** [@inspect] lectura.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Permiso explícito evita fallos tardíos y accesos indebidos.

**Ejemplo canónico.**

```nC
if (!canRead(@path)) { dev(PermissionDenied @runtime); }
```


### 58. `nC.runtime.temp` — Temporales

**Qué hace.** Crear rutas/archivos temporales y limpiar recursos temporales.

**Import canónico.**

```nC
use nC.runtime.temp;
```

**Funciones principales.**

```txt
tempWideAlloc(text: str) -> addr
tempWideBuffer(chars: int) -> addr
tempWideFree(buffer: addr) -> bool
tempDirectory(buffer: addr, capacity: int) -> int
tempFileName(directory: addr, prefix: addr, unique: int, output: addr) -> int
tempOpen(path: addr, access: int, share: int, security: addr, creation: int, flags: int, template: addr) -> addr
tempDeleteFile(path: addr) -> bool
tempCreateDirectory(path: addr, security: addr) -> bool
tempRemoveDirectory(path: addr) -> bool
tempMove(from: addr, to: addr, flags: int) -> bool
tempAttributes(path: addr) -> int
tempSetAttributes(path: addr, attributes: int) -> bool
tempFinalPath(handle: addr, buffer: addr, capacity: int, flags: int) -> int
tempClose(handle: addr) -> bool
tempLastError() -> int
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** La ubicacion, buffers, handles, flags y limpieza pertenecen al
programa. El backend solo materializa la ABI del sistema.

**Ejemplo canónico.**

```n
directory >> addr <- tempWideBuffer(260);
chars >> int <- tempDirectory(directory, 260);
tempWideFree(directory);
```


### 59. `nC.runtime.service` — SCM manual

**Qué hace.** Expone handles, buffers, access masks, callbacks y estructuras
del Service Control Manager sin estado ni políticas administradas.

**Import canónico.**

```nC
use <process:service>;
```

**Superficie.** Son 25 operaciones crudas: conversión UTF-16 explícita;
apertura, creación, consulta, enumeración, control y borrado; lock del SCM;
notificaciones; dispatcher/handler/status del proceso-servicio; cierre y
`GetLastError`.

**Memoria.** El programa asigna todos los buffers y estructuras. Cada handle se
cierra con `serviceClose`; cada nombre de `serviceWideAlloc` se libera con
`serviceWideFree`. No hay bounds, ownership, espera, shell ni elevación oculta.

**Backend.** Win64 llama directamente a `ADVAPI32.dll`/`KERNEL32.dll` desde
LLVM/MCT/Machine. La especificación completa está en
`docs/spec/runtime/imports/service.md`.


### 60. `nC.runtime.signal` — Señales del SO

**Qué hace.** Manejo de señales del sistema operativo.

**Import canónico.**

```nC
use nC.runtime.signal;
```

**Funciones principales.**

```txt
signalCtrlHandler(handler: addr, add: bool) -> bool
signalCtrlEvent(event: int, processGroup: int) -> bool
signalCreateEvent(attributes: addr, manualReset: bool, initialState: bool, wideName: addr) -> addr
signalOpenEvent(access: int, inherit: bool, wideName: addr) -> addr
signalSetEvent(event: addr) -> bool
signalResetEvent(event: addr) -> bool
signalWaitOne(event: addr, timeoutMs: int) -> int
signalWaitMany(count: int, handles: addr, waitAll: bool, timeoutMs: int) -> int
signalQueueApc(callback: addr, thread: addr, data: int) -> int
signalAddVectored(first: int, handler: addr) -> addr
signalRaiseException(code: int, flags: int, count: int, arguments: addr) -> bool
signalClose(handle: addr) -> bool
```

**Permitido en.** [@system], [@tooling], [@compiler], [@runtime].

**Restringido en.** [@security].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** La API expone directamente eventos, esperas, APC y excepciones de
Win64. Handles, callbacks, buffers y vida de recursos son responsabilidad del
programa; no existe un proveedor C ni gestion automatica.

**Ejemplo canónico.**

```n
event >> addr <- signalCreateEvent(null, true, false, null);
signalSetEvent(event);
status >> int <- signalWaitOne(event, 0);
signalClose(event);
```


## VI. Tiempo, reloj y aleatoriedad

Tiempo humano, reloj técnico, timers, pausas, aleatoriedad y UUID.


### 61. `nC.runtime.time` — Tiempo humano/sistema

**Qué hace.** Tiempo Unix, reloj monotónico, espera y formato UTC raw.

**Import canónico.** Ninguno: prelude hosted del backend.

**Funciones principales.**

```txt
now() -> int
nowMs() -> int
timestamp() -> int
monotonicMs() -> int
monotonicNs() -> int
sleep(ms: int) -> void
sleepUntilMs(deadline: int) -> bool
formatTimeInto(seconds: int, out: p<byte>, capacity: u64) -> int
dateInto(out: p<byte>, capacity: u64) -> int
formatTime(seconds: int) -> str
date() -> str
timeRelease(value: str) -> bool
```

**Permitido en.** Código normal, [@runtime], [@system], [@tooling], [@compiler], [@network], [@ai].

**Restringido en.** [@security].

**Prohibido en.** [@kernel] salvo bridge, [@machine] usar machine timing, [@], primitive[@].

**Por qué.** Tiempo humano depende de SO y zona/entorno.

**Ejemplo canónico.**

```nC
buffer >> p<byte> <- stackAlloc(24, 8);
written >> int <- dateInto(buffer, 24);
```


### 62. `nC.runtime.clock` — Reloj monotónico

**Qué hace.** Medición de duración con reloj monotónico/técnico.

**Import canónico.** Ninguno: prelude hosted del backend.

**Funciones principales.**

```txt
monotonic() -> u64
elapsed(start: u64) -> u64
measure(callback: fn() -> bool) -> u64
clockResolution() -> u64
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@inspect], [@system], [@network].

**Restringido en.** [@security], [@kernel]/[@machine] bridge.

**Prohibido en.** [@], primitive[@].

**Por qué.** Medición técnica no debe depender del reloj humano.

**Ejemplo canónico.**

```nC
start >> u64 <- monotonic();
```


### 63. `nC.runtime.timer` — Temporizadores

**Qué hace.** Timers, timeout y medición por objeto temporizador.

**Import canónico.**

```nC
use nC.runtime.timer;
```

**Funciones principales.**

```txt
timerStart() -> Timer
timerStop(timer: Timer) -> Duration
timerReset(timer: Timer) -> bool
timeout(ms: u64) -> Timer
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@network], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@machine] usar timing machine, [@], primitive[@].

**Por qué.** Timer es abstracción de scheduler/clock runtime.

**Ejemplo canónico.**

```nC
@timer >> Timer : timerStart();
```


### 64. `nC.runtime.sleep` — Suspensión

**Qué hace.** Bloquear explícitamente o ceder una vez al sistema hosted.

**Import canónico.** Ninguno: prelude hosted del backend.

**Funciones principales.**

```txt
sleepMs(ms: int) -> void
sleepSec(sec: int) -> void
yieldRuntime() -> void
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@network], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** La llamada es bloqueante y no crea scheduler, future ni tarea.

**Ejemplo canónico.**

```nC
sleepMs(500);
```


### 65. `nC.runtime.random` — Aleatoriedad no criptográfica

**Qué hace.** Valores aleatorios para simulación, juegos o reparto no seguro.

**Import canónico.**

```nC
use stoud.io { random };
```

**Funciones principales.**

```txt
randomInt() -> int
randomRange(min: int, max: int) -> int
randomFloat() -> ft64
randomBool() -> bool
seed(value: u64) -> bool
```

**Permitido en.** Código normal, [@runtime], [@tooling], [@ai].

**Restringido en.** [@system].

**Prohibido en.** [@security] para claves/tokens, [@kernel], [@machine], [@], primitive[@].

**Por qué.** No es criptográfico; comodidad arriba, no seguridad.

**Ejemplo canónico.**

```nC
@x >> int : randomRange(1, 10);
```


### 66. `nC.runtime.entropy` — Entropía

**Qué hace.** Bytes de entropía del sistema para usos seguros.

**Disponibilidad.** Prelude hosted del backend; no requiere `use`. La operación
sigue siendo manual: el programa entrega dirección y tamaño.

**Funciones principales.**

```txt
entropyFill(out: ptr<byte>, len: u64) -> bool
entropyAvailable() -> bool
entropyBytes(len: u64) -> Result<memRegion<byte>, EntropyError> // compatibilidad
entropyRelease(value: Result<memRegion<byte>, EntropyError>) -> bool
```

**Permitido en.** [@security], [@system], [@runtime] con permiso.

**Restringido en.** [@tooling], [@compiler].

**Prohibido en.** [@machine] salvo bridge autorizado, [@], primitive[@].

**Por qué.** Entropía es recurso del sistema y puede bloquear/fallar.

**Ejemplo canónico.**

```nC
bytes >> p<byte> <- stackAlloc(32, 16);
ok >> bool <- entropyFill(bytes, 32);
```


### 67. `nC.runtime.uuid` — UUID

**Qué hace.** Generación y validación de identificadores únicos.

**Disponibilidad.** Prelude hosted del backend; no requiere `use`. Las formas
`Into` escriben en almacenamiento del llamador.

**Funciones principales.**

```txt
uuidV4Into(out: ptr<byte>, capacity: u64) -> int
uuidFromBytesInto(bytes: ptr<byte>, out: ptr<byte>, capacity: u64) -> int
isUuid(text: str) -> bool
uuidV4() -> str                                      // compatibilidad; liberar
uuidFromBytes(bytes: array<byte,16>) -> str          // compatibilidad; liberar
uuidRelease(value: str) -> bool
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@ai].

**Restringido en.** [@security] política de generación.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** UUID requiere aleatoriedad/entropía o política de generación.

**Ejemplo canónico.**

```nC
out >> p<byte> <- stackAlloc(37, 1);
written >> int <- uuidV4Into(out, 37);
```


## VII. Concurrencia y sincronización

Tareas, hilos, async, sync, atomic, locks, colas, eventos y cancelación.


### 68. `nC.runtime.task` — Tareas ligeras

**Qué hace.** Crear, esperar, cancelar y consultar tareas runtime ligeras.

**Import canónico.**

```nC
use nC.runtime.task;
```

**Funciones principales.**

```txt
taskSpawn(fn) -> Result<Task, TaskError>
taskAwait(task: Task) -> Result<void, TaskError>
taskCancel(task: Task) -> Result<void, TaskError>
taskStatus(task: Task) -> TaskStatus
```

**Permitido en.** [@runtime], [@tooling], [@compiler], [@network], [@ai], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Tareas dependen de scheduler runtime.

**Ejemplo canónico.**

```nC
@t >> Task : taskSpawn(@fn(trabajo)) !> Task : dev(TaskSpawnError @runtime);
```


### 69. `nC.runtime.thread` — Hilos del sistema

**Qué hace.** Hilos reales del sistema con join e identidad.

**Import canónico.**

```nC
use nC.runtime.thread with capability("thread.spawn");
```

**Funciones principales.**

```txt
threadSpawn(fn) -> Result<Thread, ThreadError>
threadJoin(thread: Thread) -> Result<void, ThreadError>
threadId() -> ThreadId
threadSleep(ms: u64) -> void
```

**Permitido en.** [@system], [@runtime], [@tooling], [@compiler].

**Restringido en.** [@security], [@network].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Thread toca scheduler del SO y concurrencia real.

**Ejemplo canónico.**

```nC
@h >> Thread : threadSpawn(@fn(worker)) !> Thread : dev(ThreadSpawnError @runtime);
```


### 70. `nC.runtime.async` — Async/Future

**Qué hace.** Futuros y `await` de runtime.

**Import canónico.**

```nC
use nC.runtime.async;
```

**Funciones principales.**

```txt
asyncRun(fn) -> Future<T>
await(future: Future<T>) -> Result<T, AsyncError>
futureReady(future) -> bool
futureCancel(future) -> bool
```

**Permitido en.** [@runtime], [@network], [@tooling], [@compiler], [@ai].

**Restringido en.** [@system], [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Async exige scheduler y modelo de espera runtime.

**Ejemplo canónico.**

```nC
@f >> Future<int> : asyncRun(@fn(calcular)); @v >> int : await(@f);
```


### 71. `nC.runtime.sync` — Sincronización

**Qué hace.** Mutex, locks y barreras de alto nivel.

**Import canónico.**

```nC
use nC.runtime.sync;
```

**Funciones principales.**

```txt
mutexCreate() -> Mutex
lock(mutex: Mutex) -> bool
unlock(mutex: Mutex) -> bool
barrierCreate(count: u64) -> Barrier
barrierWait(barrier: Barrier) -> bool
```

**Permitido en.** [@runtime], [@system], [@network], [@tooling], [@compiler], [@security].

**Restringido en.** [@kernel] bridge; [@machine] usar machine.sync.

**Prohibido en.** [@], primitive[@].

**Por qué.** Sincronización debe tener ownership y posibilidad de deadlock.

**Ejemplo canónico.**

```nC
@mx >> Mutex : mutexCreate(); lock(@mx); unlock(@mx);
```


### 72. `nC.runtime.atomic` — Atómicos

**Qué hace.** Operaciones atómicas runtime y fence básico.

**Import canónico.**

```nC
use nC.runtime.atomic;
```

**Funciones principales.**

```txt
atomicLoad(value) -> T
atomicStore(target, value) -> bool
atomicAdd(target, value) -> T
compareExchange(target, expected, value) -> bool
atomicFence() -> void
```

**Permitido en.** [@runtime], [@system], [@network], [@security].

**Restringido en.** [@kernel] bridge; [@machine] preferir machine.atomic.

**Prohibido en.** [@], primitive[@].

**Por qué.** Los atómicos dependen del modelo de memoria y ordenamiento.

**Ejemplo canónico.**

```nC
atomicAdd(@contador, 1);
```


### 73. `nC.runtime.lock` — Locks avanzados

**Qué hace.** Locks de lectura/escritura y bloqueo de alto nivel.

**Import canónico.**

```nC
use nC.runtime.lock;
```

**Funciones principales.**

```txt
rwLockCreate() -> RwLock
readLock(lock: RwLock) -> bool
writeLock(lock: RwLock) -> bool
unlock(lock) -> bool
```

**Permitido en.** [@runtime], [@system], [@network], [@tooling].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Lock alto nivel no pertenece a máquina basal.

**Ejemplo canónico.**

```nC
writeLock(@lock); unlock(@lock);
```


### 74. `nC.runtime.queue` — Colas concurrentes

**Qué hace.** Colas para comunicación entre tareas/hilos.

**Import canónico.**

```nC
use stoud.collections { queue };
```

**Funciones principales.**

```txt
queueCreate<T>() -> Queue<T>
queuePush(queue, value) -> bool
queuePop(queue) -> Option<T>
queueLen(queue) -> u64
```

**Permitido en.** [@runtime], [@system], [@network], [@tooling], [@ai].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Cola runtime depende de memoria gestionada y sincronización.

**Ejemplo canónico.**

```nC
@q >> Queue<int> : queueCreate<int>(); queuePush(@q, 10);
```


### 75. `nC.runtime.event` — Eventos runtime

**Que hace.** Eventos del sistema mediante handles, flags y waits manuales.

**Import canónico.**

```n
use <task:event>;
```

**Funciones principales.**

```txt
eventCreate(attributes: addr, manualReset: bool, initial: bool, name: addr) -> addr
eventCreateEx(attributes: addr, name: addr, flags: int, access: int) -> addr
eventSet(event: addr) -> bool
eventWait(event: addr, timeout: int) -> int
eventWaitMany(count: int, handles: addr, all: bool, timeout: int) -> int
eventSignalAndWait(signal: addr, wait: addr, timeout: int, alertable: bool) -> int
eventClose(event: addr) -> bool
```

**Materializacion.** LLVM/MCT y Machine llaman directamente a KERNEL32. No hay
fuente C, CRT, futures ni scheduler implicito.

**Permitido en.** Contenedores runtime hosted Win64 con import explicito.

**Ejemplo canónico.**

```n
ev >> addr <- eventCreate(null, true, false, null);
eventSet(ev);
eventWait(ev, 1000);
eventClose(ev);
```


### 76. `nC.runtime.scheduler` — Scheduler runtime

**Qué hace.** Planificación de tareas y ejecución diferida/recurrente.

**Import canónico.**

```nC
use nC.runtime.scheduler;
```

**Funciones principales.**

```txt
schedule(fn) -> Task
scheduleAfter(ms: u64, fn) -> Task
scheduleEvery(ms: u64, fn) -> Task
schedulerYield() -> void
```

**Permitido en.** [@runtime], [@tooling], [@network], [@ai], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Scheduler runtime no es scheduler de kernel ni timing físico.

**Ejemplo canónico.**

```nC
scheduleAfter(1000, @fn(actualizar));
```


### 77. `nC.runtime.cancel` — Cancelación

**Qué hace.** Tokens de cancelación para tareas y operaciones largas.

**Import canónico.**

```nC
use nC.runtime.cancel;
```

**Funciones principales.**

```txt
cancelToken() -> CancelToken
cancel(token: CancelToken) -> bool
isCancelled(token: CancelToken) -> bool
throwIfCancelled(token: CancelToken) -> Result<void, CancelError>
```

**Permitido en.** [@runtime], [@tooling], [@network], [@ai], [@system].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Cancelación controlada evita matar procesos/tareas a lo bruto.

**Ejemplo canónico.**

```nC
@token >> CancelToken : cancelToken(); if (isCancelled(@token)) { return false; }
```


## VIII. Red y comunicación

Red general, sockets, TCP, UDP, DNS, HTTP y TLS.


### 78. `nC.runtime.net` — Red general

**Qué hace.** Disponibilidad de red, interfaces y direcciones.

**Import canónico.**

```nC
use nC.runtime.net with capability("net.access");
```

**Funciones principales.**

```txt
netAvailable() -> bool
netInterfaces() -> list<NetInterface>
netAddressOf(name: str) -> Option<IpAddress>
```

**Permitido en.** [@network], [@system], [@tooling], [@runtime] con permiso.

**Restringido en.** [@security] política; [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Red abre superficie externa y requiere autorización.

**Ejemplo canónico.**

```nC
if (netAvailable()) { println("Red disponible."); }
```


### 79. `nC.runtime.socket` — Sockets

**Qué hace.** Socket genérico de bajo/medio nivel runtime.

**Import canónico.**

```nC
use nC.runtime.socket with capability("net.socket");
```

**Funciones principales.**

```txt
socketCreate(protocol: Protocol) -> Result<Socket, NetError>
socketConnect(socket, host: str, port: port) -> Result<void, NetError>
socketSend(socket, data) -> Result<u64, NetError>
socketRecv(socket, buffer, len: u64) -> Result<u64, NetError>
socketClose(socket) -> bool
```

**Permitido en.** [@network], [@system], [@tooling].

**Restringido en.** [@security], [@runtime] permiso, [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Socket puede enviar/recibir datos externos.

**Ejemplo canónico.**

```nC
@s >> Socket : socketCreate(TCP) !> Socket : dev(SocketError @network);
```


### 80. `nC.runtime.tcp` — TCP

**Qué hace.** Streams TCP cliente/servidor.

**Import canónico.**

```nC
use nC.runtime.tcp with capability("net.tcp");
```

**Funciones principales.**

```txt
tcpConnect(host: str, port: port) -> Result<TcpStream, NetError>
tcpListen(port: port) -> Result<TcpListener, NetError>
tcpAccept(listener: TcpListener) -> Result<TcpStream, NetError>
tcpSend(stream: TcpStream, data) -> Result<u64, NetError>
tcpRecv(stream: TcpStream, buffer, len: u64) -> Result<u64, NetError>
```

**Permitido en.** [@network], [@system], [@tooling].

**Restringido en.** [@security], [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** TCP implica comunicación externa persistente y estados de conexión.

**Ejemplo canónico.**

```nC
@conn >> TcpStream : tcpConnect("localhost", 8080) !> TcpStream : dev(TcpConnectError @network);
```


### 81. `nC.runtime.udp` — UDP

**Qué hace.** Sockets UDP y datagramas.

**Import canónico.**

```nC
use nC.runtime.udp with capability("net.udp");
```

**Funciones principales.**

```txt
udpBind(port: port) -> Result<UdpSocket, NetError>
udpSendTo(socket, host: str, port: port, data) -> Result<u64, NetError>
udpRecvFrom(socket, buffer, len: u64) -> Result<UdpPacket, NetError>
```

**Permitido en.** [@network], [@system], [@tooling].

**Restringido en.** [@security].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** UDP no garantiza entrega ni orden, requiere contrato explícito.

**Ejemplo canónico.**

```nC
@u >> UdpSocket : udpBind(5353) !> UdpSocket : dev(UdpBindError @network);
```


### 82. `nC.runtime.dns` — DNS

**Qué hace.** Resolución DNS directa e inversa.

**Import canónico.**

```nC
use nC.runtime.dns with capability("net.dns");
```

**Funciones principales.**

```txt
resolve(host: str) -> Result<IpAddress, DnsError>
resolveAll(host: str) -> Result<list<IpAddress>, DnsError>
reverseLookup(ip: IpAddress) -> Result<str, DnsError>
```

**Permitido en.** [@network], [@system], [@tooling], [@runtime] con permiso.

**Restringido en.** [@security], [@ai] sandbox.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** DNS filtra nombres y depende de red/entorno.

**Ejemplo canónico.**

```nC
@ip >> IpAddress : resolve("example.com") !> IpAddress : dev(DnsResolveError @network);
```


### 83. `nC.runtime.http` — HTTP

**Qué hace.** HTTP alto nivel: GET/POST y respuesta básica.

**Import canónico.**

```nC
use nC.runtime.http with capability("net.http");
```

**Funciones principales.**

```txt
httpGet(url: str) -> Result<HttpResponse, HttpError>
httpPost(url: str, body: str) -> Result<HttpResponse, HttpError>
httpStatus(response: HttpResponse) -> int
httpBody(response: HttpResponse) -> str
```

**Permitido en.** [@network], [@tooling], [@system].

**Restringido en.** [@security] política; [@ai] sandbox obligatorio.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** HTTP es red alta, no máquina ni kernel.

**Ejemplo canónico.**

```nC
@res >> HttpResponse : httpGet("https://example.com") !> HttpResponse : dev(HttpError @network);
```


### 84. `nC.runtime.tls` — TLS

**Qué hace.** Canales TLS y certificado de peer.

**Import canónico.**

```nC
use nC.runtime.tls with capability("net.tls");
```

**Funciones principales.**

```txt
tlsConnect(host: str, port: port) -> Result<TlsStream, TlsError>
tlsSend(stream, data) -> Result<u64, TlsError>
tlsRecv(stream, buffer, len: u64) -> Result<u64, TlsError>
tlsPeerCert(stream) -> Result<Certificate, TlsError>
```

**Permitido en.** [@network], [@system], [@security].

**Restringido en.** [@tooling], [@ai] sandbox/policy.

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** TLS depende de criptografía, red y política de certificados.

**Ejemplo canónico.**

```nC
@tls >> TlsStream : tlsConnect("example.com", 443) !> TlsStream : dev(TlsError @network);
```


## IX. Capacidades, políticas y aislamiento

Capacidades, políticas, perfiles, bridges, sandbox y recursos.


### 85. `nC.runtime.capability` — Capacidades

**Qué hace.** Consulta y exige capacidades activas del runtime.

**Import canónico.**

```nC
use nC.runtime.capability;
```

**Funciones principales.**

```txt
hasCapability(name: str) -> bool
requireCapability(name: str) -> Result<void, CapabilityError>
grantCapability(name: str) -> Result<void, CapabilityError>
revokeCapability(name: str) -> Result<void, CapabilityError>
listCapabilities() -> list<str>
```

**Permitido en.** [@runtime], [@system], [@security], [@tooling], [@compiler].

**Restringido en.** [@inspect] lectura.

**Prohibido en.** [@kernel] salvo bridge de política, [@machine], [@], primitive[@].

**Por qué.** Capacidad decide si una acción puede existir.

**Ejemplo canónico.**

```nC
requireCapability("fs.read") !> void : dev(CapabilityError @runtime);
```


### 86. `nC.runtime.policy` — Políticas

**Qué hace.** Políticas activas, niveles y ejecución bajo política.

**Import canónico.**

```nC
use nC.runtime.policy;
```

**Funciones principales.**

```txt
policyActive(name: str) -> bool
requirePolicy(name: str) -> Result<void, PolicyError>
policyLevel(name: str) -> PolicyLevel
withPolicy(name: str, fn) -> Result<void, PolicyError>
```

**Permitido en.** [@security], [@system], [@runtime], [@tooling], [@compiler].

**Restringido en.** [@inspect], [@network].

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Política gobierna operaciones sensibles más allá del tipo.

**Ejemplo canónico.**

```nC
requirePolicy("security.audit") !> void : dev(PolicyError @runtime);
```


### 87. `nC.runtime.profile` — Perfil raw

**Qué hace.** Almacena perfiles explícitos de 48 bytes con mode, flags,
CPU-features, optimization y target.

**Import canónico.** `use <audit:profile>;`.

```txt
profileStride() -> int
profileWrite(table, index, mode, flags, cpuFeatures, optimization, target, targetBytes) -> bool
profileMode/profileFlags/profileCpuFeatures/profileOptimization(table, index) -> int
profileTarget(table, index) -> addr
profileTargetBytes(table, index) -> int
profileHas/profileRequire(table, index, requiredFlags) -> bool
profileCopy(destination, destinationIndex, source, sourceIndex) -> bool
profileClear(table, index) -> bool
```

**Semántica.** El compilador no crea ni descubre el descriptor. `profileRequire`
ejecuta trap si faltan bits. El resto conserva semántica raw y baja directamente
a LLVM/Machine sin C.

**Ejemplo canónico.** `examples/runtime/families/audit_manual_family_raw.n`.


### 88. `nC.runtime.bridge` — Puentes raw

**Qué hace.** Expone una tabla material de bridges creada y gobernada por el
programa.

**Import canónico.**

```nC
use <audit:bridge>;
```

**Layout.** Cada entrada ocupa 32 bytes: `key:i64`, `target:ptr`, `mode:i64` y
`authority:i64`.

**Funciones principales.**

```txt
bridgeStride() -> int
bridgeWrite(table: addr, index: int, key: int, target: addr, mode: int, authority: int) -> bool
bridgeFind(table: addr, count: int, key: int) -> int
bridgeActive(table: addr, count: int, key: int) -> bool
bridgeRequire(table: addr, count: int, key: int) -> int
bridgeTarget(table: addr, index: int) -> addr
bridgeMode(table: addr, index: int) -> int
bridgeAuthority(table: addr, index: int) -> int
bridgeErase(table: addr, index: int) -> bool
```

**Semántica.** El llamador aporta memoria, claves, índices y recuentos. No hay
strings administradas, reserva, ownership, bounds checks ni recuperación
oculta. Un acceso inválido es comportamiento indefinido; una clave ausente en
`bridgeRequire` ejecuta trap.

**Ejemplo canónico.** `examples/runtime/families/audit_bridge_raw.n`.


### 89. `nC.runtime.sandbox` — Aislamiento

**Qué hace.** Sandbox de rutas, capacidades y violaciones.

**Import canónico.**

```nC
use nC.runtime.sandbox;
```

**Funciones principales.**

```txt
sandboxActive() -> bool
sandboxPath(path: str) -> Result<str, SandboxError>
sandboxAllow(capability: str) -> Result<void, SandboxError>
sandboxDeny(capability: str) -> Result<void, SandboxError>
sandboxViolation(reason: str) -> never
```

**Permitido en.** [@security], [@system], [@tooling], [@compiler], [@ai], [@runtime].

**Restringido en.** [@network], [@inspect].

**Prohibido en.** [@kernel], [@machine], [@], primitive[@].

**Por qué.** Aísla efectos externos y limita daño.

**Ejemplo canónico.**

```nC
@safePath >> str : sandboxPath(@path) !> str : dev(SandboxViolation @runtime);
```


### 90. `nC.runtime.resource` — Recursos

**Qué hace.** Apertura, cierre, identificación y límites de recursos runtime.

**Import canónico.**

```nC
use nC.runtime.resource;
```

**Funciones principales.**

```txt
resourceOpen(name: str) -> Result<Resource, ResourceError>
resourceClose(resource: Resource) -> bool
resourceId(resource: Resource) -> str
resourceKind(resource: Resource) -> str
resourceAlive(resource: Resource) -> bool
resourceLimit(kind: str) -> u64
```

**Permitido en.** [@runtime], [@system], [@tooling], [@compiler], [@inspect], [@security], [@network].

**Restringido en.** [@kernel] bridge.

**Prohibido en.** [@machine], [@], primitive[@].

**Por qué.** Todo recurso necesita vida, cierre y límite.

**Ejemplo canónico.**

```nC
@res >> Resource : resourceOpen("session-cache") !> Resource : dev(ResourceOpenError @runtime);
```


---

# 6. Tabla general de disponibilidad

| Familia | Normal | `[@runtime]` | `[@system]` | `[@security]` | `[@inspect]` | `[@tooling]` | `[@compiler]` | `[@network]` | `[@kernel]` | `[@machine]` | `[@]` | `primitive[@]` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| input | Sí | Sí | Limitado | Seguro | No | Sí | Sí | No | No | No | No | No |
| output | Sí | Sí | Sí | Controlado | Sí | Sí | Sí | No | Bridge | No | No | No |
| errorOutput | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| channel | Limitado | Sí | Sí | Controlado | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| args/env | Sí | Sí | Sí | Lectura | Sí | Sí | Sí | No | No | No | No | No |
| text/render/format | Sí | Sí | Sí | Redactado | Sí | Sí | Sí | Limitado | No | No | No | No |
| parse/convert | Sí | Sí | Sí | Controlado | Sí | Sí | Sí | Limitado | No | No | No | No |
| validate | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| result/option | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Limitado | No | No |
| diagnostic/log/trace | Limitado | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| debug/assert | Debug | Debug | Debug | Controlado | Sí | Sí | Sí | Limitado | Bridge | Bridge | No | No |
| buffer/region | Limitado | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| memory/heap/ptr/addr | No | Limitado | Sí | Controlado | Sí | No | Sí | No | Bridge | Bridge | No | No |
| fs/path/file/dir | Limitado | Sí | Sí | Política | Lectura | Sí | Sí | No | No | No | No | No |
| process/command | No | Limitado | Sí | Política | No | Sí | Sí | No | No | No | No | No |
| time/clock/timer | Sí | Sí | Sí | Controlado | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| random | Sí | Sí | Limitado | No seguro | No | Sí | Sí | No | No | No | No | No |
| entropy | No | Permiso | Sí | Sí | No | Limitado | Limitado | No | No | Bridge | No | No |
| task/thread/async | Limitado | Sí | Sí | Controlado | No | Sí | Sí | Sí | No | No | No | No |
| sync/atomic | Limitado | Sí | Sí | Sí | Sí | Sí | Sí | Sí | Bridge | Bridge | No | No |
| net/socket/tcp/udp | No | Permiso | Sí | Política | No | Sí | Limitado | Sí | No | No | No | No |
| http/tls/dns | No | Permiso | Sí | Política | No | Sí | Limitado | Sí | No | No | No | No |
| capability/policy | No | Sí | Sí | Sí | Lectura | Sí | Sí | Limitado | Bridge | No | No | No |
| bridge | No | Limitado | Sí | Sí | Sí | Limitado | Limitado | No | Sí | Sí | No | No |
| sandbox/resource | Limitado | Sí | Sí | Sí | Sí | Sí | Sí | Limitado | No | No | No | No |


# 7. Relación con la familia canónica `print`

La API `nC.runtime.output` no sustituye a la familia doctrinal `print`.

La familia `print` oficial tiene 22 miembros:

```txt
print
println
print.err
print.wm
print.inf
print.ok
print.sta
print.trc
print.dbg
print.inspect
print.mem
print.site
print.type
print.raw
print.stable
print.addr
print.phys
print.dump
print.peek
print.mmio
print.port
print.sensory
```

`runtime.output` es el servicio de salida base de runtime.  
`print.*` es una familia modal de exposición de verdad, con crudeza, canal, autoridad y lowering propio.

En código normal, `print`/`println` pueden venir de `use nC.runtime.output;` o de la superficie `nC.print` si el compilador la separa como familia intrínseca.

En modos bajos, especialmente `[@]`, `[@machine]`, `[@kernel]` y `primitive[@]`, ciertos `print.*` no deben entenderse como simple runtime. Deben tratarse como operaciones semánticas autorizadas por modo.


# 8. Diagnósticos canónicos

## Runtime no importado

```txt
error[E_USE_RUNTIME_001]:
function `readInt` is not available in this scope.

help:
add `use nC.runtime.input;`
```

## Ruta `runtime.*` usada en código normal

```txt
error[E_RUNTIME_PATH_001]:
explicit `runtime.*` paths are not allowed in ordinary code.

help:
import the runtime family with `use nC.runtime.input;`
and call `readInt()` directly.
```

## Runtime en `[@]`

```txt
error[E_RUNTIME_ROOT_001]:
runtime imports are not allowed inside [@].

reason:
[@] is a root semantic layer and cannot depend on runtime services.
```

## Runtime en `primitive[@]`

```txt
error[E_RUNTIME_PRIMITIVE_001]:
runtime is not available in primitive[@].

reason:
primitive[@] operates below runtime and cannot use high-level execution services.
```

## Entrada en `[@machine]`

```txt
error[E_MACHINE_RUNTIME_INPUT_001]:
nC.runtime.input is not available inside [@machine].

help:
machine containers may only use restricted diagnostic/runtime bridge families.
```

## Sistema de archivos sin capacidad

```txt
error[E_CAPABILITY_FS_001]:
filesystem access requires capability `fs.read`.

help:
use `use nC.runtime.fs with capability("fs.read");`
```

## Impresión de secreto

```txt
error[E_SEC_RENDER_001]:
cannot render sensitive value into output.

help:
use redact(value) or an approved security renderer.
```


# 9. Ejemplo completo canónico

```nC
use nC.runtime.input;
use nC.runtime.output;
use nC.runtime.render;
use nC.runtime.validate;
use nC.runtime.result;

edadConInput[@runtime] {
    stc:
        edadMinima: int;
        edadMedia: int;
        edadMaxima: int;
        edadActual: int;
        decision: int;
        opcion: str;
        mensaje: str;

    fn:
        boot() -> bool {
            @edadMinima >> int : 18;
            @edadMedia >> int : 23;
            @edadMaxima >> int : 65;
            return true;
        }

        clasificarEdad(edad: int) -> str {
            if (@edad < @edadMedia && @edad >= @edadMinima) {
                return "Eres joven.";
            } else if (@edad >= @edadMedia && @edad <= @edadMaxima) {
                return "Eres adulto.";
            } else if (@edad < @edadMinima) {
                return "Eres menor de edad.";
            } else {
                return "Eres mayor de la edad máxima considerada.";
            }
        }

        leerEdad() -> int {
            @edadLeida >> int : readInt()
                !> int : dev(InputParseError @runtime);

            if (!range(@edadLeida, 0, 130)) {
                dev(InputRangeError @runtime);
            }

            return @edadLeida;
        }

        pedirAccion() -> int {
            while (true) {
                println();
                println("¿Qué deseas hacer a continuación?");
                println("1. Jugar de nuevo");
                println("2. Salir");
                print("> ");

                @opcion >> str : readLine()
                    !> str : dev(InputReadError @runtime);

                if (@opcion == "1") {
                    return 1;
                } else if (@opcion == "2") {
                    return -1;
                } else {
                    print.wm("Entrada no válida.");
                }
            }
        }

        ejecutar() -> bool {
            boot();

            while (true) {
                print("Por favor, ingresa tu edad: ");

                @edadActual >> int : leerEdad();

                @mensaje >> str : clasificarEdad(@edadActual);

                println("Edad recibida: " + renderInt(@edadActual));
                println(@mensaje);

                @decision >> int : pedirAccion();

                if (@decision == -1) {
                    print.ok("Saliendo del programa. Hasta luego.");
                    break;
                }
            }

            return true;
        }

    ref main ->> ejecutar;

    fa:
        return true;
}
```

## 9.1. Extensiones nativas de interfaz grafica

La interfaz grafica no crea familias paralelas ni duplica semantica. Se integra en
`input`, `render` y `terminal`, y se importa de forma compacta con:

```nC
use stoud.ui { canvas, input, window };
```

`WindowHandle` se representa como `int` de ancho de puntero en la revision ABI
actual. Los colores usan `0xRRGGBB`; la capa Win32 los convierte a `COLORREF`.

```txt
nC.runtime.terminal:
    windowOpen(title: str, width: int, height: int) -> int
    windowShow(handle: int) -> bool
    windowHide(handle: int) -> bool
    windowPoll(handle: int) -> bool
    windowWaitEvent(handle: int, timeoutMs: int) -> bool
    windowClose(handle: int) -> bool
    windowIsOpen(handle: int) -> bool
    windowIsFocused(handle: int) -> bool
    windowSetTitle(handle: int, title: str) -> bool
    windowSetSize(handle: int, width: int, height: int) -> bool
    windowWidth(handle: int) -> int
    windowHeight(handle: int) -> int
    windowFocus(handle: int) -> bool

nC.runtime.render:
    canvasClear(handle: int, rgb: int) -> bool
    canvasRect(handle: int, x: int, y: int, width: int, height: int, rgb: int) -> bool
    canvasText(handle: int, x: int, y: int, text: str, rgb: int) -> bool
    canvasLine(handle: int, x1: int, y1: int, x2: int, y2: int, rgb: int, width: int) -> bool
    canvasEllipse(handle: int, x: int, y: int, width: int, height: int, rgb: int) -> bool
    canvasPresent(handle: int) -> bool

nC.runtime.input:
    mouseX(handle: int) -> int
    mouseY(handle: int) -> int
    mouseDown(button: int) -> bool
    keyDown(virtualKey: int) -> bool
    windowReadKey(handle: int) -> int
    windowReadChar(handle: int) -> int
```

El backend PE64 materializa estas llamadas como thunks Win64 y las enlaza con
`USER32.dll`, `GDI32.dll` y `KERNEL32.dll`. `ncc run` usa el mismo contrato en
un host Windows. En otro host devuelve `E_RUNTIME_WINDOW_PLATFORM_001` en vez
de simular una ventana.

## 9.2. Extensión JSON tipada

`nC.runtime.json` incorpora 46 operaciones tipadas para parseo, validación,
serialización, persistencia, navegación RFC 6901, construcción, mutación y
diagnósticos estructurados. Los árboles se cruzan por handles `int` poseídos;
`0` es siempre inválido y cada hijo expuesto es una copia independiente.

La implementación alojada pertenece a `ncc-runtime`. En PE64, el compilador
añade el objeto COFF del bridge real solo cuando el LLVM referencia un símbolo
`json*`; Phase 12 enlaza ese tercer objeto y exige cero símbolos sin resolver.
El contrato completo, los 46 símbolos y los límites están en
[`runtime_json.md`](../../runtime/runtime_json.md).

# 10. Base final

```txt
El runtime de nC es una superficie modal de servicios de ejecución.

En código normal:
    se importa con `use`;
    se usa sin prefijo `runtime.*`.

En código system/kernel/machine:
    puede aparecer `runtime.*` explícito solo para señalar frontera peligrosa.

En [@]:
    no hay dependencia de runtime.

En primitive[@]:
    no existe runtime.

La API oficial del runtime conserva 90 familias base y 3 extensiones tipadas:
    entrada, salida, texto, parseo, errores, memoria, sistema,
    tiempo, concurrencia, red, capacidades, políticas y aislamiento.

Cada familia tiene:
    utilidad;
    funciones;
    modos permitidos;
    restricciones;
    errores posibles;
    y razón semántica.
```

Frase doctrinal:

```txt
El runtime ayuda arriba, se revela en la frontera, y desaparece abajo.
```
# Historical base input

This file preserves the original source request used to derive runtime contracts.
Its snippets are not the current authoring surface. Use
`docs/spec/language/canonical_compact_syntax_v3.md` for executable code.
