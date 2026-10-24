# Runtime JSON tipado

Estado: contratos versionados. El perfil nativo del módulo 2 v77 publica 44
símbolos JSON desde nC en Windows x86_64 y Linux/WSL x86_64. Su contrato de
`ft64`, pruebas y reconstrucción B/C están en
[JSON binary64 v77](../../NCC_JSON_BINARY64_V77.md). El contrato amplio v1 de
Stage0 se conserva; no debe confundirse con el inventario distribuido actual.

## Perfil nativo: fallos de asignación v78

El proveedor distribuido conserva los 44 exports y sus firmas. En este perfil,
el error `3` significa fallo runtime, incluidos OOM y números no finitos; no
debe interpretarse usando la tabla histórica de diagnósticos de Stage0.
`jsonClearError` limpia explícitamente el error. No todos los getters nativos
lo limpian al acertar.

Si falla una reserva, el parser conserva el error runtime en vez de ocultarlo
como éxito o sustituirlo por un error de sintaxis. La serialización devuelve
`""`, libera su salida incompleta y no publica JSON parcial. Los getters que
devuelven una copia de texto pueden devolver null por OOM. `jsonEquals`
devuelve false si alguna serialización falla, aunque fallen ambas.

Los tamaños y offsets de texto usan el dominio int32. Las sumas de tamaño y
la indentación se comprueban antes de reservar o escribir el resultado.
Los objetos nativos conservan el orden de inserción; la ordenación de claves
descrita más abajo pertenece al contrato histórico de Stage0.

La tabla mutable de handles requiere sincronización del llamador. Getters
`Get` producen copias poseídas; los `Borrow` y `jsonStringView` son vistas
cuya validez depende del padre. No se añade un recolector, mutex implícito o
motor de ownership. La inyección de fallos vive solo en los tests nC, no en
la biblioteca publicada. Ver [módulos v78](../../NCC_MODULES_V78.md).

## Objetivo

`nC.runtime.json` ofrece un árbol JSON completo sin exponer punteros del host.
Cada valor se representa mediante un `int` opaco distinto de cero. El handle
`0` siempre significa fallo. Los hijos obtenidos durante la navegación son
copias poseídas: liberar o mutar un hijo no invalida ni modifica el padre.

El perfil distribuido actual se implementa en `runtime/nc/json_native_provider.n`
y `runtime/nc/binary64.n`. La implementación histórica de Stage0 permanece en
`ncc-runtime/src/json.rs`; no es un cuerpo nuevo incorporado al proveedor nC.
El contrato amplio describe:

- parseo y validación estrictos de JSON UTF-8;
- serialización compacta y pretty-print determinista;
- recorrido de objetos, arrays y JSON Pointer RFC 6901;
- getters tipados para `str`, `bool`, `int` y `ft`;
- construcción y mutación de árboles;
- carga y guardado de archivos;
- errores estructurados y límites de recursos;
- liberación y clonación explícitas de handles.

## Importación

```nC
use <data:json>;
```

## Tipos y códigos

El tipo se obtiene con `jsonKind(handle)`:

| Código | Tipo |
|---:|---|
| 0 | handle inválido |
| 1 | `null` |
| 2 | `bool` |
| 3 | número |
| 4 | string |
| 5 | array |
| 6 | object |

Los errores son estables y consultables mediante `jsonErrorCode`,
`jsonErrorName`, `jsonErrorMessage`, `jsonErrorLine`, `jsonErrorColumn` y `jsonErrorPath`.
En el contrato histórico de Stage0, una operación correcta limpia el error
anterior. El perfil nativo conserva la regla explícita indicada arriba.

| Código | Diagnóstico |
|---:|---|
| 0 | sin error |
| 1 | `E_RUNTIME_JSON_PARSE_001` |
| 2 | `E_RUNTIME_JSON_IO_001` |
| 3 | `E_RUNTIME_JSON_HANDLE_001` |
| 4 | `E_RUNTIME_JSON_TYPE_001` |
| 5 | `E_RUNTIME_JSON_KEY_001` |
| 6 | `E_RUNTIME_JSON_INDEX_001` |
| 7 | `E_RUNTIME_JSON_POINTER_001` |
| 8 | `E_RUNTIME_JSON_NUMBER_001` |
| 9 | `E_RUNTIME_JSON_LIMIT_001` |
| 10 | `E_RUNTIME_JSON_SERIALIZE_001` |

## API v1

### Parseo, validación y persistencia

```text
jsonParse(text: str) -> int
jsonValidate(text: str) -> bool
jsonParseFile(path: str) -> int
jsonStringify(handle: int) -> str
jsonStringifyPretty(handle: int, indent: int) -> str
jsonSaveFile(handle: int, path: str, indent: int) -> bool
jsonClone(handle: int) -> int
jsonRelease(handle: int) -> bool
```

`jsonSaveFile` escribe primero un temporal en el mismo directorio y lo mueve al
destino. `indent == 0` selecciona salida compacta; el resto se limita al rango
`1..8`.

En el bridge PE64, las rutas relativas se resuelven primero contra el directorio
de trabajo. `jsonParseFile` reintenta junto al `.exe` cuando el archivo de cwd no
existe o contiene JSON inválido. `settings.json` es la excepción de despliegue:
`jsonSaveFile` lo persiste primero junto al ejecutable para que los ajustes
acompañen a la aplicación aunque se lance desde otro directorio.

### Inspección y getters

```text
jsonKind(handle: int) -> int
jsonIsNull(handle: int) -> bool
jsonEquals(left: int, right: int) -> bool
jsonGetString(handle: int) -> str
jsonGetBool(handle: int) -> bool
jsonGetInt(handle: int) -> int
jsonGetFloat(handle: int) -> ft64
jsonGetNumberText(handle: int) -> str
jsonGetStringOr(handle: int, fallback: str) -> str
jsonGetBoolOr(handle: int, fallback: bool) -> bool
jsonGetIntOr(handle: int, fallback: int) -> int
jsonGetFloatOr(handle: int, fallback: ft64) -> ft64
```

Los getters sin fallback devuelven el cero de su tipo al cruzar la ABI y dejan
el error estructurado. Los getters `*Or` devuelven el fallback y conservan el
error que explica por qué fue necesario.

En el perfil nativo v77, `jsonGetFloat` devuelve `+0.0` para handles inválidos
o nodos no numéricos, sin fabricar un error nuevo. `jsonGetFloatOr` conserva
los bits del fallback cuando el nodo no es numérico. Un número decimal válido
fuera de rango se convierte a infinito con signo, no al fallback.

### Objetos, arrays y punteros

```text
jsonObjectLen(object: int) -> int
jsonObjectHas(object: int, key: str) -> bool
jsonObjectGet(object: int, key: str) -> int
jsonObjectKeyAt(object: int, index: int) -> str
jsonObjectSet(object: int, key: str, value: int) -> bool
jsonObjectRemove(object: int, key: str) -> int

jsonArrayLen(array: int) -> int
jsonArrayGet(array: int, index: int) -> int
jsonArrayPush(array: int, value: int) -> bool
jsonArrayInsert(array: int, index: int, value: int) -> bool
jsonArrayRemove(array: int, index: int) -> int

jsonPointer(root: int, pointer: str) -> int
```

`jsonPointer` sigue RFC 6901, incluyendo `~0` para `~` y `~1` para `/`.
Índices con cero inicial, `-`, valores negativos o fuera de rango fallan de
forma estructurada.

### Constructores

```text
jsonNewNull() -> int
jsonNewBool(value: bool) -> int
jsonNewInt(value: int) -> int
jsonNewFloat(value: ft64) -> int
jsonNewString(value: str) -> int
jsonNewArray() -> int
jsonNewObject() -> int
```

NaN e infinitos no son números JSON y producen
`E_RUNTIME_JSON_NUMBER_001`.

Ese código corresponde al contrato histórico de Stage0. En la ABI nativa v77,
`jsonNewFloat` devuelve handle `0` y `jsonErrorCode() == 3` para una entrada
no finita. Limpia el error anterior al comenzar. El llamador libera el handle
válido con `jsonRelease` y los textos serializados propios con `free`.

### Error actual

```text
jsonErrorCode() -> int
jsonErrorName() -> str
jsonErrorMessage() -> str
jsonErrorLine() -> int
jsonErrorColumn() -> int
jsonErrorPath() -> str
jsonClearError() -> bool
```

## Límites y seguridad

Los defaults del host son 16 MiB de entrada, 128 niveles, 1 000 000 de nodos
y 65 536 handles activos. Los recorridos recursivos están acotados por el límite
de profundidad antes de descender. La API de archivo no crea directorios ni concede capacidades: el bridge
debe respetar las políticas `fs.read` y `fs.write` del runtime.

La serialización de objetos es determinista y ordena claves por su texto UTF-8
en ambas implementaciones. Los números se conservan como números JSON y no se
convierten a texto binario flotante durante el parseo. El bridge rechaza UTF-8
inválido y secuencias `\u` con sustitutos Unicode incompletos.

## Ejemplo

```nC
root >> int <- jsonParse("{\"element\":\"Cm\",\"number\":96}");
symbol >> int <- jsonObjectGet(root, "element");
name >> str <- jsonGetString(symbol);
ok >> bool <- name == "Cm";
strRelease(name);
jsonRelease(symbol);
jsonRelease(root);
```

## Frontera de verdad

El perfil nativo v77 se publica desde objetos COFF/ELF emitidos de nC. No usa
C intermedio, `strtod` ni `snprintf`. El módulo JSON general y otros targets
no quedan certificados por esas pruebas. El dossier v77 registra el alcance
y la evidencia efectivamente ejecutada, sin promoción global de Stage3.

El siguiente recorrido describe exclusivamente el bridge histórico de Stage0:

El núcleo Rust y el intérprete comparten exactamente la misma arena y los
mismos diagnósticos. Para `nC.win64`, el compilador detecta referencias `json*`,
genera un objeto COFF freestanding con los 46 símbolos y lo entrega como tercer
objeto a Machine Phase 12. El manifiesto registra fuente, compilador y hashes;
una unidad que no usa JSON no compila ni enlaza el bridge. El PE resultante debe
quedar con cero símbolos sin resolver y ejecuta parseo, mutación, serialización,
archivos y errores sin pasar por el intérprete.

Otros backends que aún no materialicen esta ABI deben rechazarla con un
diagnóstico explícito; nunca deben simular un parseo correcto ni convertir JSON
en concatenaciones de strings.
