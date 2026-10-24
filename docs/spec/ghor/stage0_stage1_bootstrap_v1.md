# Bootstrap modular de ghor v1

## Estructura nC

`tools/ghor/nc/main.n` es la madre. El proyecto contiene veintisiete hijos nC
para texto raw, diagnosticos, manifiesto, configuracion Cargo, grafo de
paquetes/workspace, transporte, SHA-256, resolucion, materializacion, edicion
de dependencias, registro, mutaciones, artefactos, compilacion, ejecucion,
instalacion, herramientas de proyecto, assurance de host y catalogo de
comandos y administracion del registro. El hijo `certification.n` compone las garantias de release sin
duplicar sus motores materiales. `security.n` es la autoridad de auditoria,
SBOM, licencias, cadena de suministro y capacidades. `foreign_execution.n`
posee los adaptadores extranjeros. `bridge.json` retiene veintiocho unidades y
ciento veintiuna aristas.

Cada hijo contiene logica material y una prueba propia. El proyecto se
sincroniza con `ncc project sync tools/ghor/nc`. La ruta normal
`ncc build tools/ghor/nc/main.n tools/ghor/nc/.ncc/bin/ghor-stage1.exe`
enlaza el AST completo y genera `.nm`, LLVM/MCT, COFF y PE64. El comando
`ghor bootstrap --json` analiza los veintiocho fuentes, publica sus hashes y nunca
marca el bootstrap como completo mientras falte implementar el gestor completo
en stage1.

La superficie contiene 77 comandos. En la fase nativa inicial, `build`, `run`,
`test` y `bench` recorren nC, IR optimizado, MCT, Machine, objeto y ejecutable.
`run` y `bench` tambien son stage1 materiales. `run` compila una vez, conserva
el ejecutable, `.nm`, `.obj`, informe y hashes, hereda la terminal y propaga el
estado del programa. `bench` compila cada fuente una vez en `release-fast`,
aplica calentamiento, mide muestras con reloj monotono, permite filtrar y
retiene todos los artefactos y hashes. `manifest` y `metadata` ya poseen parser TOML, filesystem, argumentos y salida
JSON en nC stage1. `workspace metadata` es una accion stage1 material. Las
`resolve`, `lock`, `verify-lock`, `fetch`, `vendor`, `update`, `add`, `remove`
y `tree` tambien se ejecutan en stage1. Resuelven SemVer, features,
dependencias transitivas, Git, registros filesystem/HTTP/sparse/Git,
`[source.*]`, `[registries.*]` y paquetes basales publicados por `ncc basal
publish`. Los paquetes basales requieren hashes de archivo y FFI validos,
contrato `basal.package.v1`, compatibilidad Ghor y metadata `NBB0`. El catalogo
ejecutable de `ghor report commands --json` conserva el reparto oficial de
comandos completos, parciales y pendientes.

`search`, `info`, `login`, `logout` y `registry doctor|cargo-config|index|config`
tambien tienen implementacion material nC. El cliente lee registros
filesystem/HTTP/sparse/Git, propaga bearer tokens sin imprimirlos y guarda
`GHOR_HOME/credentials.json` con permisos privados en Unix. La precedencia
nativa es `--token`, `GHOR_REGISTRY_TOKEN` y credencial almacenada.

`publish`, `owner` y `yank` tambien se despachan a stage1. El hijo
`registry_mutation.n` construye el cuerpo binario Cargo publish, llama a Cargo
Web API, actualiza `index.json` y el shard JSON-lines, persiste propietarios y
estado yank, y escribe auditoria. Los reemplazos locales usan lock, temporal,
`fsync`, backup y rename; los registros HTTP exigen bearer para toda mutacion.

La misma fuente genera PE64 en Windows y ELF64 en Linux. El wrapper ELF
interno convierte la pila inicial del loader en `argc`, `argv` y `envp` antes
de llamar a la entrada SysV; por ello el cliente stage1 usa argumentos y
entorno reales tambien fuera de Windows.

## Despacho

Rust conserva temporalmente el ejecutor de compatibilidad. Ghor descubre
automaticamente `ghor-stage1` junto al launcher; `NC_GHOR_STAGE1` permite
seleccionar otro ejecutable. Se delegan `manifest`, `metadata`, `resolve`,
`lock`, `verify-lock`, `fetch`, `vendor`, `update`, `add`, `remove`, `tree`,
`workspace metadata`, `search`, `info`, `login`, `logout`, `publish`, `owner`, `yank`,
`registry doctor|cargo-config|index|config`, `check`, `build`, `run`, `test`,
`bench`, `version`, `--version`, `-V`, `new`, `init`, `clean`, `fmt`, `lint` y
`doc`, `doctor`, `toolchain`, `verify-toolchain` y `sync-toolchain` cuando la
combinacion de opciones esta implementada. El hijo
`project_tools.n` crea scaffolds transaccionales para nC y adaptadores
declarados, recorre fuentes mediante buffers filesystem raw, ejecuta `nfmt` y
`nlint` sin contaminar el protocolo JSON, genera inventarios SHA-256 y limpia
arboles con una barrera explicita contra destinos peligrosos. El launcher
usa `assurance.n` para ejecutar y capturar directamente los doctores machine,
ASM nativo y basal mediante la ABI raw de procesos. La misma autoridad detecta
deriva entre el compilador activo y el instalado, publica diagnosticos
estructurados y construye el plan explicito de sincronizacion de toolchain.
`system_adapters.n` despacha `machine`, `basal`, `runtime`, `compiler` y
`primitive` desde nC stage1. Resuelve el compilador por `NC_GHOR_NCC`, `NCC` o
`ncc`, construye `argv` con almacenamiento explicito, ejecuta sin shell, hereda
la terminal y conserva el codigo de salida material del subsistema.
`registry_administration.n` implementa `cdn`, `governance` y `moderation` con
JSON, filesystem, HTTP, SHA-256, shards Cargo y auditoria nativos. Sus
mutaciones son transaccionales a traves de `registry_mutation.n`, exigen token,
redactan credenciales y rechazan escritura directa sobre registros Git.
`bootstrap_authority.n` emite el contrato activo desde el ejecutable nC,
enumera las 28 unidades enlazadas y calcula el reparto del catalogo a traves de
`project_inspection.n`. `project_tools.n` implementa tambien `lang new`: crea
manifiestos, las nueve unidades del forge, pruebas, ejemplo, documentacion y
reporte de conformidad mediante una transaccion de filesystem nativa.
El hijo `certification.n` es la autoridad de `verify`, `evidence`, `seal`,
`conformance`, `public-beta`/`beta-check`, `release-check`/`self-check`,
`release-hardening`/`hardening` e `install-smoke`/`release-install-smoke`.
Estas rutas reutilizan los modelos nC de doctor, lock, verificacion, preflight,
registro, SHA-256 y captura de procesos. Sus resultados JSON identifican
`nc-stage1`; los secretos se redactan y ningun array material obtiene
almacenamiento implicito en modo manual/raw.

`security.n` recorre explicitamente dependencias normales, dev, build, target,
Rust, C, C++, ASM, basal, native, workspace, patch y replace. `audit` y
`supply-chain verify` rechazan dependencias remotas sin checksum; `sbom`
publica todas las familias; `license` retiene la expresion del paquete y
`capability-audit` incluye machine, basal, primitive, runtime, compiler y las
capacidades declaradas.

`foreign_execution.n` ejecuta `cargo-exec`, `build-script`, `proc-macro`,
`interop` y `external-compat` desde el PE/ELF nC. La llamada tipada
`spawnIn(program, argv, cwd)` conserva argumentos y directorio sin shell
implicito. Los compiladores C/C++ y Cargo son procesos extranjeros solicitados
por el comando; no forman parte de la implementacion ni del runtime de ghor.

El catalogo conserva 77 comandos: cuatro rutas de artefacto machine, 57
comandos stage1 completos, 16 parciales y 0 pendientes. Los alias anteriores
son nombres de compatibilidad de las mismas autoridades y no comandos
independientes en ese reparto.

El launcher reconoce rutas, Git, registros
filesystem/HTTP/sparse/Git, referencias nombradas, reemplazos de fuente y
dependencias basales declaradas en el manifiesto que stage1 puede leer. La
configuracion heredada exclusivamente desde `.cargo/config.toml`, la
politica proxy/CA, credential providers y opciones que stage1 aun no declara
quedan en stage0. Las mutaciones se delegan para referencias materiales y
autenticacion explicita o almacenada. `NC_GHOR_FORCE_STAGE0=1` fuerza recuperacion;
el puente impide delegarse a si mismo.

## Retirada de Rust

Rust deja de ser necesario cuando el ejecutable nC supera todo el corpus,
reconstruye sus artefactos y forma parte de un compilador stage2 reproducible.
No quedan comandos stage1 pendientes. Rust permanece exclusivamente como
launcher stage0 y ruta de recuperacion forzada. Hasta completar stage2, el
estado oficial es `bootstrap in progress`; no se reclama autoalojamiento.

El contrato global de etapas vive en `bootstrap/stages.json`.
