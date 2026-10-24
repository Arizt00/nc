# Reproduccion del modulo 1 v72

## Politica Nativa E Integracion

Usar el B r39 certificado en el dossier v71 y su proveedor distribuible.
Windows necesita ncruntime.dll al lado de B; Linux necesita
LD_LIBRARY_PATH apuntando al directorio de libncruntime.so.2. Seleccionar
Ubuntu explicitamente con wsl -d Ubuntu.

```text
B compiler emit-object bootstrap/nc/stage0_policy.n --target x86_64-pc-windows-msvc --out policy.obj --quiet
B compiler emit-object bootstrap/nc/stage0_policy.n --target x86_64-unknown-linux-gnu --out policy.o --quiet
```

Antes de construir Stage0, definir NC_STAGE0_POLICY_COMPILER con la ruta de B.
El build script ejecuta B y archiva el objeto con llvm-lib en Windows/MSVC o
ar en Linux. NC_STAGE0_POLICY_ARCHIVER permite seleccionar el archivador.
La ausencia de B es un error de bootstrap, no activa una implementacion Rust.
Usar CARGO_BUILD_JOBS=1 y no ejecutar varias reconstrucciones monoliticas a
la vez. Las pruebas Rust existentes requieren la misma variable de entorno.

## Programas

Cada programa es un proyecto separado, con su fuente como main.n. El de
politica necesita src/stage0_policy.n. Generar por separado native.exe o
native, LLVM -O0 y LLVM -O3 para GVN, contextual y promotions.

```text
B project link PROYECTO --target TARGET --out IMAGEN --no-incremental
Stage0 mct emit-llvm FUENTE --out PROGRAMA.ll --cc nC.win64 --opt-level 0 --json
Stage0 mct emit-llvm FUENTE --out PROGRAMA.ll --cc nC.sysv64 --opt-level 0 --json
```

Clang es el ejecutor del backend LLVM existente, no una implementacion de
la politica ni un main C. Conservar .ll, objetos, imagenes y logs. El
verificador de procesos recibe un JSON con cases: name, image, cwd, stdout
y exit. Se ejecuta como verificador MANIFEST.json REPORT.json. Un error,
timeout, stdout parcial, diferencia de bytes o stderr no vacio falla.

## Generacion C

Para r40, copiar y verificar las 71 fuentes del manifiesto v72 a un
directorio nuevo. No mezclar sus objetos con los del baseline r39.
No copiar .ncc ni usar materiales heredados. Construir el driver nC como
proyecto con src/sha256.n del mismo snapshot y su proveedor.

```text
driver B C_ROOT B_BRIDGE TARGET C_IMAGE C_RECEIPT
driver B C_ROOT B_BRIDGE TARGET C_IMAGE C_RECEIPT --resume-verified-materials
```

El driver reinicia despues de commits de unidades verificadas y conserva
solo materiales producidos por esa generacion. El enlace final debe terminar
sin reiniciarse. Comparar los artefactos B/C con normalizacion none mediante
frontend-verify artifacts, ejecutar C self-test y repetir el corpus de
programas con C como candidate y B como baseline. Nunca inferir el cierre
por el numero de objetos parciales o por aceptar declaraciones.

La segunda forma valida y reutiliza materiales de un intento anterior;
su recibo marca clean_initial_input=false, inherited_materials=true y el
numero de unidades previas. No demuestra por si sola un inicio limpio.
En esta iteracion el primer intento comenzo limpio y termino con senal 15
despues de nueve unidades. Conservar ambos logs y las identidades de los
drivers, ademas de las verificaciones finales, para auditar la cadena.

## Candidato R40

El cambio de project.n utiliza jsonStringView para consultas de lectura de
bindings, manteniendo vivo el propietario. Antes del auto-build, ejecutar
borrowed_queries.n: salida borrowed-bindings-ok y codigo 251. El programa
realiza un millon de consultas; no modifica ni libera el string prestado.

```text
A39 project link B40_ROOT --target TARGET --out B40_IMAGE --no-incremental
B40 self-test
B40 project link C40_ROOT --target TARGET --out C40_IMAGE --no-incremental
C40 self-test
```

La raiz de B40 de esta iteracion conserva directorios previos, pero el flag
desactiva la reutilizacion. Comprobar el recibo project-cache.json y sus
71 unidades mas enlace, no inferirlo del nombre del comando. C40 debe empezar
sin .ncc. Si se usa el driver por unidades en vez de la ruta monolitica,
registrar explicitamente el cambio: reutiliza solo material producido en C.

No cambiar configuraciones globales de WSL para reproducirlo. Registrar
boot_id, memoria, swap, tiempos, salida y codigo de proceso. Los reinicios
observados no fueron provocados por el usuario; su causa sigue sin demostrar.

## Proveedor Con Cursor

Preparar la fuente con prepare_json_source72.n, emitir LLVM con Stage0 y
compilar el objeto con Clang. link_json_provider72.n recibe el JSON de
mappings, objetos conservados y opciones de enlace. Conservar el manifest,
la fuente preparada, LLVM, objetos, imports/exports, hashes y logs. No
sobrescribir proveedores dentro de un proceso activo.

```text
prepare ROOT JSON_SOURCE windows
prepare ROOT JSON_SOURCE linux
Stage0 mct emit-llvm JSON_SOURCE --out JSON.ll --cc nC.win64 --opt-level 0 --json
Stage0 mct emit-llvm JSON_SOURCE --out JSON.ll --cc nC.sysv64 --opt-level 0 --json
link LINK_MANIFEST.json
```

Ejecutar json_cursor.n con ambos proveedores, exigiendo codigo 252 y salida
exacta json-cursor-ok. Repetir el corpus v69 de 147 programas antes de usar
el proveedor nuevo para C. La version anterior del verificador v45 no admite
--corpus; los recibos usan v47-r2 en Windows y v46 en Linux. Conservar sus
identidades y fuentes, no asumir compatibilidad por el nombre del binario.

module1_artifact_verify_v72.n recibe ARTIFACTS.json REPORT.json y acepta solo
normalization=none. Comprobar todos los materiales de B/C y la imagen, ademas
de source pins, ausencia inicial de .ncc, cero cache hits y self-test real.
