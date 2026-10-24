# Reproduccion V81

## Snapshot y generaciones

`bootstrap/frontend-v81-build.json` fija las 74 fuentes del compilador
y las fuentes de validacion. Cada constructor verifica SHA-256 al copiar.
Se conservan los snapshots usados por las generaciones en el directorio
de trabajo de evidencia. El constructor rechaza destinos con .ncc.

Compilar `build_native_compiler81.n` con los hijos sha256, elf_entry79 y
coff_entry79 bajo `src/sha256.n`, `src/elf_entry.n`, `src/coff_entry.n`.
El constructor de runtime usa sha256 y el publisher nC
`link_native_providers73.n`.

```text
compiler-builder REPO SEMILLA DIRECTORIO-B PROVEEDOR-SEMILLA
B/ncc self-test
runtime-builder REPO B/ncc PROYECTO-RUNTIME-B PUBLICACION-B PUBLISHER
compiler-builder REPO B/ncc DIRECTORIO-C PUBLICACION-B
C/ncc self-test
runtime-builder REPO C/ncc PROYECTO-RUNTIME-C PUBLICACION-C PUBLISHER
```

Declarar los proveedores junto a los ejecutables Windows. En Linux,
usar LD_LIBRARY_PATH para los consumidores sin RUNPATH. El compilador
enlazado por unidades usa el RUNPATH $ORIGIN. Las dependencias de sistema
y los linkers son explicitos; no se compila ninguna fuente C auxiliar.

## Programas de alineacion

```text
ncc compiler emit-mct tools/tests/native_overalignment81.n --target TARGET --out caso.ncmct
ncc compiler emit-object tools/tests/native_overalignment81.n --target TARGET --out caso.obj
ncc compiler emit-executable tools/tests/native_overalignment81.n --target TARGET --out caso.exe
```

Windows usa `x86_64-pc-windows-msvc`; Linux usa
`x86_64-unknown-linux-gnu`, objeto .o y permiso de ejecucion del ELF.
Ejecutar y comprobar stdout y exit exactos. Para los tres archivos del
fixture transitivo copiar calls como main.n y api/leaf en src/.

Los fixtures powers_globals y powers_locals recorren las dieciseis
potencias de dos de 32 a 1048576. Los globales mayores a 8192 no caben
en COFF: exigir rechazo del objeto y ejecutar PE directo. Los locales
grandes si caben en COFF; enlazar con reserva de pila suficiente.

Ejemplo de opciones de link.exe para el fixture global COFF de 8192:

```text
/nodefaultlib /entry:nc_start /subsystem:console /machine:x64
/align:8192 /Brepro /incremental:no
```

Para powers_locals y max_local en Windows agregar
`/stack:16777216,65536`. El entry nC llama a main y finaliza mediante
ExitProcess; no usar /entry:main. Linux enlaza con entry _start,
GNU ld, -z noexecstack, -z separate-code y --build-id=none.
La prueba max_abi declara `ulimit -s 32768` en su shell Linux antes de
ejecutar el programa y usa la reserva Windows de 16 MiB.

El verificador `native_overalignment_verify81.n` recibe objeto o imagen
y ruta JSON de informe. Lee COFF/PE/ELF y comprueba secciones, padding,
direcciones y los tres punteros del fixture. Conservar tambien el
desensamblado y la inspeccion de secciones con LLVM/GNU.

## Cache y pruebas adversas

En el proyecto wide ejecutar project link en frio y en caliente, sin
--no-incremental. Guardar `.ncc/incremental/project-cache.json` de cada
pasada: un AST y dos materiales deben restaurarse en caliente.

Los controles MCT copian el cache a proyectos nuevos, alteran solamente
el material derivado y recalculan su digest en el manifiesto. Declarar
alignment=24, alignment=16 con simbolos de 8192, o un offset de global=1.
El emisor debe rechazar los tres sin escribir la imagen final.
No modificar fuentes o binarios positivos para hacerlos coincidir.

## Corpus y runtime

```text
runner verify-programs --root REPO --candidate B --baseline C --stage0 STAGE0
    --out DIRECTORIO-NUEVO --corpus bootstrap/frontend-v69-native-programs.json
```

Repetir C/B y ambos sistemas, serialmente. En Windows preparar el
directorio de salida con ncruntime.dll antes de invocar al runner.
El runner necesita un directorio de captura nuevo, pero no publica
automaticamente ese proveedor.

Los 38 contratos usan runtime_processes_verify76.n. Preparar file.dat
con cinco bytes, contenido bytes, mtime Unix 1700000000 y modo Linux 0644;
names con first.dat/segundo.dat, empty/stress vacios, y create/missing
ausentes. No reutilizar fixtures mutados para ocultar un fallo.

## Equivalencia y custodia

runtime_artifacts_verify76.n, con su hijo sha256, recibe:

```json
{"artifacts":[{"name":"caso","reference":"ruta-B","candidate":"ruta-C","normalization":"none"}]}
```

Comparar archivos distintos, no vacios. Conservar el informe con SHA-256,
bytes comparados y first_difference=-1. Separar los JSON con tiempos/rutas
de los materiales binarios; no normalizar estos ultimos.

El ZIP conserva payloads deduplicados por hash, manifiesto y lectura
completa posterior de cada entrada. El sello exterior contiene los hashes
del ZIP y del manifiesto. La certificacion es local Windows/WSL x86-64,
no de placas fisicas, otros hosts o la promocion global de Stage3.
