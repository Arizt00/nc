# ncc clean / ncc doctor run

## Objetivo

`ncc clean` limpia artefactos generados por nC sin tocar fuentes, binarios
instalados ni configuracion IDE.

## Comandos

```text
ncc clean
ncc clean --cache
ncc clean <path> --json
ncc clean <path> --cache --dry-run
ncc clean <workspace> --build
ncc clean <workspace> --all
ncc doctor run
ncc doctor run <archivo.n> --json
```

## Alcance de limpieza

`ncc clean` elimina:

- `.ncc/lowering`
- `.ncc/obj`
- `.ncc/run`
- `.ncc/incremental`
- `.ncc/tmp`

`ncc clean --cache` tambien elimina:

- `.ncc/cache`

`ncc clean --build` exige una raiz Cargo y elimina solamente caches
reproducibles conocidos:

- `target/debug`
- `target/tmp`
- `target/ncup`
- directorios target de arquitectura que contienen `.fingerprint`
- `target/release/{deps,build,.fingerprint,incremental,examples}`

`--all` equivale a `--cache --build`. `--dry-run` no crea locks ni borra
archivos; informa bytes, archivos y directorios recuperables.

Se preserva por diseno:

- `.ncc/code`
- `.ncc/bin`
- `.ncc/ide`
- `.ncc/locks`
- ejecutables situados directamente en `target/release`
- evidencia, informes y artefactos target personalizados

## Concurrencia

Antes de borrar, `ncc clean` toma el mismo lock de cache que `ncc run`,
`ncc check` y `ncc project check`:

```text
.ncc/locks/project-cache.lock
```

Esto evita que el IDE borre `.ncc/lowering` o `.ncc/obj` mientras otra terminal
los esta escribiendo. Si el lock no se puede tomar antes del timeout, el comando
falla con `E_NCC_CLEAN_LOCK_001`.

El modo `--dry-run` es estrictamente de lectura y por ello no crea ni toma el
lock. Su medicion es una instantanea; una compilacion concurrente puede cambiarla.

## Control de crecimiento

Los perfiles `dev` y `test` usan simbolos de linea y optimizacion ligera. Esto
reduce de forma importante el tamano de cada copia de `ncc` y acelera las pruebas
del compilador sin activar incremental. `ncup` usa una cache aislada, fuerza
`CARGO_INCREMENTAL=0` y la retira tras instalar y verificar todos los binarios,
tambien dentro de WSL. Las rutas WSL, Ubuntu y Kali pasan un `--target-dir`
temporal explicito fuera del workspace; un `trap` lo elimina incluso si la
compilacion o la instalacion falla.

## Diagnostico

`ncc doctor run` inspecciona:

- cwd y binario exacto;
- hash del binario activo;
- resolucion universal de archivo si se pasa un `.n`;
- raiz de proyecto detectada;
- `bridge.json`;
- `.ncc/run/last.json`;
- estado del lock de cache;
- artefactos que limpiaria `ncc clean` y `ncc clean --cache`.

El schema JSON es:

```text
ncc.clean.v1
ncc.doctor.run.v1
```
