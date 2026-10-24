# Manual final de usuario de ghor

Status: closed.

Este manual describe `ghor` como orquestador nC con superficie compatible
con comandos Cargo donde corresponde. Cubre uso diario, registry setup,
publish, install, vendor, seguridad, conformance y diferencias con Cargo.

## Tabla global de comandos tipo Cargo

| ghor | Cargo cercano | Funcion |
| --- | --- | --- |
| `new` | `cargo new` | Crea un paquete nC con estructura inicial. |
| `init` | `cargo init` | Inicializa un paquete en el directorio actual. |
| `check` | `cargo check` | Analiza sin producir artefacto final. |
| `build` | `cargo build` | Construye artefactos de trabajo. |
| `run` | `cargo run` | Ejecuta el binario o unidad seleccionada. |
| `test` | `cargo test` | Ejecuta tests del paquete o workspace. |
| `bench` | `cargo bench` | Ejecuta benchmarks declarados. |
| `clean` | `cargo clean` | Limpia target de ghor. |
| `fmt` | `cargo fmt` | Formatea fuentes nC y superficies soportadas. |
| `lint` | `cargo clippy` | Aplica lint y diagnostics de calidad. |
| `doc` | `cargo doc` | Genera documentacion local. |
| `package` | `cargo package` | Construye .crate compatible. |
| `publish` | `cargo publish` | Publica en registry declarado. |
| `install` | `cargo install` | Instala binario o fuente desde registry. |
| `uninstall` | `cargo uninstall` | Elimina instalacion gestionada. |
| `update` | `cargo update` | Actualiza lockfile mediante resolver. |
| `tree` | `cargo tree` | Muestra grafo de dependencias. |
| `metadata` | `cargo metadata` | Emite metadata JSON compatible. |
| `vendor` | `cargo vendor` | Copia dependencias verificadas localmente. |
| `login` | `cargo login` | Guarda token seguro para registry. |
| `logout` | `cargo logout` | Elimina credencial guardada. |
| `owner` | `cargo owner` | Gestiona propietarios del paquete. |
| `yank` | `cargo yank` | Marca version como yanked o unyanked. |

## Flujo minimo recomendado

```sh
ghor doctor --json
ghor check
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
ghor release-check --json
```


## 1. Que es ghor

Objetivo: entender ghor como orquestador nC compatible con superficies Cargo.

Resumen operativo: ghor no es solo un alias de cargo: administra proyectos nC, integra machine y basal, y conserva una vista compatible para tooling Cargo cuando el proyecto lo necesita.

Comandos principales:

```sh
$ ghor --help
$ ghor doctor --json
$ ghor conformance --json
```

Equivalencia o referencia Cargo: `cargo --help, cargo metadata`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 2. Instalacion, PATH y binarios

Objetivo: instalar y validar ghor, ncc, nclsp, nfmt y nlint.

Resumen operativo: la instalacion industrial exige que el binario activo sea el nuevo, que hash -r no deje un binario viejo en cache y que el smoke compruebe cada ejecutable instalado.

Comandos principales:

```sh
$ ghor install-smoke --json
$ ghor toolchain
$ ghor verify-toolchain
```

Equivalencia o referencia Cargo: `rustup component list, cargo --version`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 3. Ayuda tipo Cargo

Objetivo: usar la ayuda global y por subcomando.

Resumen operativo: la ayuda de ghor se organiza para que el usuario vea opciones globales, comandos, alias y ejemplos sin tener que leer codigo fuente.

Comandos principales:

```sh
$ ghor
$ ghor --help
$ ghor help build
$ ghor build --help
$ ghor --list
```

Equivalencia o referencia Cargo: `cargo help, cargo help build`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 4. Manifiestos ghor.toml y nc.toml

Objetivo: describir paquete, dependencias, workspace, targets y metadata.

Resumen operativo: ghor.toml es el manifiesto baseico; nc.toml queda como compatibilidad historica cuando el proyecto lo necesita.

Comandos principales:

```sh
$ ghor manifest --json
$ ghor check --manifest-path ghor.toml
```

Equivalencia o referencia Cargo: `Cargo.toml`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 5. Crear proyectos

Objetivo: crear paquetes nC y adaptadores de lenguajes externos.

Resumen operativo: ghor crea estructura nC con src, examples, tests, docs y reportes listos para doctor, lock y release-check.

Comandos principales:

```sh
$ ghor new app
$ ghor init
$ ghor lang new riskledge --host nC
```

Equivalencia o referencia Cargo: `cargo new, cargo init`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 6. Check, build, run, test y bench

Objetivo: ejecutar el ciclo de desarrollo principal.

Resumen operativo: cada comando debe producir estado verificable y respetar --locked, --offline y --frozen cuando corresponde.

Comandos principales:

```sh
$ ghor check
$ ghor build
$ ghor run
$ ghor test
$ ghor bench
```

Equivalencia o referencia Cargo: `cargo check, cargo build, cargo run, cargo test, cargo bench`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 7. Formato, lint y documentacion

Objetivo: normalizar codigo y generar documentacion local.

Resumen operativo: ghor separa formato, lint y docs para que una pipeline pueda fallar en el punto exacto y registrar evidencia.

Comandos principales:

```sh
$ ghor fmt
$ ghor lint
$ ghor doc
```

Equivalencia o referencia Cargo: `cargo fmt, cargo clippy, cargo doc`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 8. Lockfile compatible

Objetivo: generar y verificar ghor.lock.

Resumen operativo: ghor.lock es TOML estilo Cargo como vista primaria compatible y ghor.lock.meta.json conserva metadata nC adicional.

Comandos principales:

```sh
$ ghor lock
$ ghor verify-lock
$ ghor build --locked
```

Equivalencia o referencia Cargo: `Cargo.lock, cargo update --locked`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 9. Resolver SemVer y features

Objetivo: resolver versiones, features y dependencias por tipo.

Resumen operativo: el resolver cubre normal, dev, build, target-specific, patch, replace, alternate registries y source replacement.

Comandos principales:

```sh
$ ghor resolve --json
$ ghor tree --json
$ ghor update --dry-run
```

Equivalencia o referencia Cargo: `cargo update, cargo tree`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 10. Workspaces Cargo-grade

Objetivo: operar workspaces reales con miembros, exclude y default-members.

Resumen operativo: un workspace comparte lockfile, target y metadata de paquete, dependencias y lints cuando el root lo declara.

Comandos principales:

```sh
$ ghor workspace --json
$ ghor check --workspace
$ ghor test --workspace
```

Equivalencia o referencia Cargo: `cargo metadata --workspace`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 11. Targets, perfiles y matrices

Objetivo: entender target triples, perfiles y cross build.

Resumen operativo: ghor reporta honestamente si un target requiere toolchain externo y no convierte una ausencia en evidencia falsa.

Comandos principales:

```sh
$ ghor target
$ ghor profile
$ ghor cross
$ ghor artifact
```

Equivalencia o referencia Cargo: `cargo build --target, .cargo/config.toml`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 12. Registry setup local

Objetivo: crear un registry filesystem para desarrollo y pruebas.

Resumen operativo: el registry local escribe config.json, index.json de compatibilidad, indice por paquete y packages/<name>/<version>.

Comandos principales:

```sh
$ ghor publish --registry ./registry
$ ghor search pkg --registry ./registry
$ ghor info pkg --registry ./registry
```

Equivalencia o referencia Cargo: `local registry fixtures`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 13. Registry sparse HTTPS y HTTP cache

Objetivo: usar sparse+https y cache con ETag, Last-Modified y 304.

Resumen operativo: la cache HTTP solo acepta paquetes con checksum valido y conserva headers para evitar descargas innecesarias.

Comandos principales:

```sh
$ ghor fetch --registry sparse+https://host/index
$ ghor registry hardening --registry https://host/index --json
```

Equivalencia o referencia Cargo: `Cargo sparse registry`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 14. Registry git index

Objetivo: usar indices git estilo Cargo.

Resumen operativo: el indice git permite revision controlada del catalogo de paquetes y resolucion reproducible del estado del index.

Comandos principales:

```sh
$ ghor fetch --registry git+https://host/index.git
$ ghor registry inspect --json
```

Equivalencia o referencia Cargo: `registry git index`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 15. Publicar paquetes

Objetivo: preparar, verificar y publicar .crate compatible.

Resumen operativo: publish verifica metadata, README, licencia, categorias, keywords, repositorio y checksum antes de aceptar bytes.

Comandos principales:

```sh
$ ghor package --list
$ ghor publish --dry-run
$ ghor publish --registry <registry>
```

Equivalencia o referencia Cargo: `cargo package --list, cargo publish --dry-run, cargo publish`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 16. Yank, owner y moderation

Objetivo: administrar versiones, propietarios y estado de moderacion.

Resumen operativo: mutaciones remotas exigen auth, audit log y policy; local se mantiene practico para desarrollo sin relajar release hardening.

Comandos principales:

```sh
$ ghor yank pkg --vers 1.2.3
$ ghor yank pkg --vers 1.2.3 --undo
$ ghor owner add pkg team
$ ghor moderation pkg --action block
```

Equivalencia o referencia Cargo: `cargo yank, cargo owner`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 17. Login, credenciales y seguridad

Objetivo: guardar tokens sin imprimirlos y con permisos privados.

Resumen operativo: tokens nunca aparecen en reportes; credentials.json se guarda privado y los permisos inseguros no desbloquean mutaciones remotas.

Comandos principales:

```sh
$ ghor login --registry <url> --token <token>
$ ghor logout --registry <url>
```

Equivalencia o referencia Cargo: `cargo login, cargo logout`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 18. Instalar y desinstalar

Objetivo: construir, instalar, actualizar y desinstalar ejecutables nativos desde
un proyecto local o un registry.

Resumen operativo: `install` materializa el ejecutable real, `.nm`, objeto,
informe de build y recibo verificable. El recibo separa material propiedad de ghor
de fuentes locales ajenas. `uninstall` elimina solo lo que ese recibo declara y
usa una transaccion recuperable.

Comandos principales:

```sh
$ ghor install pkg --registry <registry>
$ ghor install pkg --version 1.2.3 --registry <registry> --root <root>
$ ghor install --manifest-path ./ghor.toml --root <root>
$ ghor uninstall pkg
$ ghor uninstall pkg --version 1.2.3 --root <root>
$ ghor bin --json
```

El registry debe aportar una version SemVer no retirada, archivo `.crate`,
SHA-256 y manifiesto canonico coherentes. Tras validar y extraer, stage1 invoca la
compilacion nativa y conserva el ejecutable junto con `.nm` y objeto. Una
reinstalacion sustituye el recibo anterior solo despues de que todo el material
nuevo haya sido verificado y movido correctamente.

Equivalencia o referencia Cargo: `cargo install, cargo uninstall`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 19. Vendor, fetch y offline

Objetivo: preparar dependencias locales reproducibles.

Resumen operativo: vendor escribe .cargo-checksum.json compatible y offline falla si falta cache en lugar de salir a red.

Comandos principales:

```sh
$ ghor fetch
$ ghor vendor vendor/
$ ghor check --offline
```

Equivalencia o referencia Cargo: `cargo fetch, cargo vendor, cargo --offline`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 20. Metadata y tree

Objetivo: emitir datos para tooling externo.

Resumen operativo: la salida conserva ids, paquetes, targets, features y nodos de resolucion suficientes para tooling Cargo-compatible declarado.

Comandos principales:

```sh
$ ghor metadata --json
$ ghor tree
$ ghor graph --json
```

Equivalencia o referencia Cargo: `cargo metadata, cargo tree`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 21. External compatibility

Objetivo: comparar ghor contra proyectos Cargo reales.

Resumen operativo: la matriz marca diferencias aceptadas y evita declarar clon social completo de crates.io.

Comandos principales:

```sh
$ ghor external-compat --json
$ ghor cargo-compat --json
```

Equivalencia o referencia Cargo: `cargo metadata, cargo tree, cargo package --list, cargo vendor, cargo update --dry-run`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 22. Build scripts y proc macros

Objetivo: entender frontera de ejecucion Cargo-backed.

Resumen operativo: ghor captura evidencia de cargo check real, hashes stdout/stderr y exit status; no inventa ejecucion si Cargo no corre.

Comandos principales:

```sh
$ ghor build-script
$ ghor proc-macro
$ ghor cargo-exec
```

Equivalencia o referencia Cargo: `build.rs, proc-macro crates`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 23. Release-check, evidence, seal y conformance

Objetivo: cerrar un paquete antes de release.

Resumen operativo: release-check preflighta sin archivar por defecto; package real exige --package para evitar artefactos gigantes accidentales.

Comandos principales:

```sh
$ ghor release-check
$ ghor evidence --json
$ ghor seal --json
$ ghor conformance --json
```

Equivalencia o referencia Cargo: `cargo package, cargo publish dry-run, release scripts`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 24. Integracion con ncc, machine y basal

Objetivo: delegar doctors sin mezclar semanticas.

Resumen operativo: ghor observa machine y basal como dependencias cerradas, pero no cambia su base ni usa imports para saltar modos.

Comandos principales:

```sh
$ ghor machine doctor
$ ghor basal doctor
$ ghor compiler doctor
$ ghor runtime doctor
```

Equivalencia o referencia Cargo: `no direct equivalent`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 25. Diferencias con Cargo

Objetivo: entender que es igual, compatible, distinto o no-goal.

Resumen operativo: ghor busca paridad operativa declarada, no gobernanza social completa de crates.io ni CDN global publico.

Comandos principales:

```sh
$ ghor conformance --json
$ ghor external-compat --json
```

Equivalencia o referencia Cargo: `Cargo and crates.io ecosystem`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 26. Recetas de migracion

Objetivo: migrar desde Cargo o convivir con Cargo.

Resumen operativo: la migracion conserva Cargo.toml cuando corresponde y usa ghor.toml para superficies nC nativas.

Comandos principales:

```sh
$ ghor metadata --manifest-path Cargo.toml --json
$ ghor vendor
$ ghor lock
```

Equivalencia o referencia Cargo: `Cargo project migration`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 27. Troubleshooting

Objetivo: diagnosticar errores comunes.

Resumen operativo: cada error debe tener codigo, causa probable, accion recomendada y forma de reproducirlo.

Comandos principales:

```sh
$ ghor doctor --json
$ ghor registry hardening --json
$ ghor verify-lock
```

Equivalencia o referencia Cargo: `cargo -vv, cargo metadata`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 28. Operacion industrial

Objetivo: usar ghor en CI multi-host y release prolongado.

Resumen operativo: CI debe conservar logs, manifests, checksums y outputs JSON para que el cierre sea revisable.

Comandos principales:

```sh
$ ghor install-smoke --json
$ ghor release-hardening --json
$ ghor cargo-compat --json
```

Equivalencia o referencia Cargo: `CI jobs around Cargo`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 29. Politica de compatibilidad

Objetivo: mantener cambios futuros sin romper usuarios.

Resumen operativo: nuevas capacidades no deben romper lockfiles, manifests ni comandos declarados sin diagnostico y migracion.

Comandos principales:

```sh
$ ghor conformance --json
$ ghor release-check
```

Equivalencia o referencia Cargo: `Cargo stability expectations`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## 30. Guia rapida por tareas

Objetivo: encontrar el comando correcto por intencion.

Resumen operativo: esta guia condensa el manual para uso diario sin reemplazar las secciones profundas.

Comandos principales:

```sh
$ ghor help <command>
$ ghor --list
```

Equivalencia o referencia Cargo: `cargo help`.

Modelo mental:
`ghor` debe leerse como una herramienta de orquestacion, no como un simple
script de conveniencia. Cuando una tarea se parece a Cargo, `ghor` intenta
mantener nombres, flags y salidas familiares. Cuando la tarea entra en nC,
machine, basal, primitive, runtime o compiler, `ghor` delega con claridad y
conserva evidencia. El usuario no debe adivinar si una accion fue local,
remota, de cache, de resolver o de release: el comando debe indicarlo en texto
humano y en JSON.

Uso diario:
Empieza con `ghor doctor --json` para confirmar que la herramienta activa es la
correcta. Si el proyecto tiene lockfile, usa `ghor verify-lock` antes de
compilar. Si vas a tocar dependencias, usa `ghor resolve --json` o `ghor tree`
para ver el grafo antes de ejecutar `ghor update`. Si el proyecto depende de un
registry remoto, comprueba `ghor registry hardening --json` antes de publicar o
consumir en CI. Esta rutina evita que un error de entorno parezca un error de
lenguaje.

Flags que conviene recordar:
`--manifest-path` selecciona un manifiesto concreto. `--workspace` aplica el
comando al workspace. `--json` convierte la salida en evidencia procesable.
`--locked` exige que el lockfile no cambie. `--offline` prohibe red. `--frozen`
combina la intencion de no tocar red ni lock. `--registry` selecciona registry
local, sparse, http, https o git. `--token` desbloquea mutaciones remotas sin
imprimir el secreto.

Practica recomendada:
En desarrollo local puedes usar un registry filesystem para publicar fixtures y
probar resolucion. En release usa registry remoto autenticado, audit log,
governance, moderation y checksums. No mezcles ambos mundos en el mismo reporte.
Un registry local puede ser comodo, pero no demuestra una ruta productiva si la
fase pide auth, cache HTTP, CDN o governance. Una pipeline industrial debe
guardar los JSON de `doctor`, `conformance`, `release-check`, `registry
hardening` y `install-smoke`.

Errores tipicos:
Si un comando falla por lock drift, regenera el lock con `ghor lock` o revisa el
cambio de dependencias. Si falla por checksum, no fuerces la cache: elimina el
paquete corrupto y vuelve a descargar desde un registry confiable. Si falla por
auth, usa `ghor login` o un credential provider. Si falla por offline, revisa
que `ghor fetch` o `ghor vendor` hayan materializado todos los paquetes. Si falla
por toolchain, no lo marques como exito: el reporte debe decir missing,
blocked_by_environment o unavailable con causa.

Ejemplo de flujo:

```sh
ghor doctor --json
ghor check --manifest-path ghor.toml
ghor resolve --json
ghor lock
ghor verify-lock
ghor build --locked
ghor test --locked
```

Lectura de salida:
Un status `ok` indica que el comando concreto paso. Un status `closed` indica
que la superficie documentada esta cerrada para el alcance declarado. Un status
`warning` exige revision, pero puede no bloquear desarrollo local. Un status
`error` bloquea release. Un campo `blocked_by_environment` no es exito ni fallo
del compilador; es una evidencia honesta de que el host no tiene una herramienta
necesaria para esa comprobacion.

Relaciones con Cargo:
Cuando Cargo tiene un comando equivalente, el usuario deberia reconocer la
intencion inmediatamente. La diferencia es que `ghor` adjunta contexto nC:
machine dependency, basal closure, adapters, evidence, seal, conformance y
registry governance. Esa informacion extra no debe romper tooling externo; por
eso se conserva una vista Cargo-compatible donde el ecosistema espera formatos
Cargo.


## Apendice A. Checklists operacionales por capitulo


### Checklist ampliado 1: 1. Que es ghor

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor no es solo un alias de cargo: administra proyectos nC, integra machine y basal, y conserva una vista compatible para tooling Cargo cuando el proyecto lo necesita. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 2: 2. Instalacion, PATH y binarios

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la instalacion industrial exige que el binario activo sea el nuevo, que hash -r no deje un binario viejo en cache y que el smoke compruebe cada ejecutable instalado. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 3: 3. Ayuda tipo Cargo

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la ayuda de ghor se organiza para que el usuario vea opciones globales, comandos, alias y ejemplos sin tener que leer codigo fuente. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 4: 4. Manifiestos ghor.toml y nc.toml

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor.toml es el manifiesto baseico; nc.toml queda como compatibilidad historica cuando el proyecto lo necesita. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 5: 5. Crear proyectos

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor crea estructura nC con src, examples, tests, docs y reportes listos para doctor, lock y release-check. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 6: 6. Check, build, run, test y bench

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: cada comando debe producir estado verificable y respetar --locked, --offline y --frozen cuando corresponde. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 7: 7. Formato, lint y documentacion

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor separa formato, lint y docs para que una pipeline pueda fallar en el punto exacto y registrar evidencia. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 8: 8. Lockfile compatible

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor.lock es TOML estilo Cargo como vista primaria compatible y ghor.lock.meta.json conserva metadata nC adicional. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 9: 9. Resolver SemVer y features

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: el resolver cubre normal, dev, build, target-specific, patch, replace, alternate registries y source replacement. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 10: 10. Workspaces Cargo-grade

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: un workspace comparte lockfile, target y metadata de paquete, dependencias y lints cuando el root lo declara. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 11: 11. Targets, perfiles y matrices

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor reporta honestamente si un target requiere toolchain externo y no convierte una ausencia en evidencia falsa. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 12: 12. Registry setup local

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: el registry local escribe config.json, index.json de compatibilidad, indice por paquete y packages/<name>/<version>. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 13: 13. Registry sparse HTTPS y HTTP cache

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la cache HTTP solo acepta paquetes con checksum valido y conserva headers para evitar descargas innecesarias. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 14: 14. Registry git index

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: el indice git permite revision controlada del catalogo de paquetes y resolucion reproducible del estado del index. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 15: 15. Publicar paquetes

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: publish verifica metadata, README, licencia, categorias, keywords, repositorio y checksum antes de aceptar bytes. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 16: 16. Yank, owner y moderation

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: mutaciones remotas exigen auth, audit log y policy; local se mantiene practico para desarrollo sin relajar release hardening. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 17: 17. Login, credenciales y seguridad

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: tokens nunca aparecen en reportes; credentials.json se guarda privado y los permisos inseguros no desbloquean mutaciones remotas. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 18: 18. Instalar y desinstalar

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: install materializa launchers y metadata para que el usuario pueda auditar que binario quedo activo. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 19: 19. Vendor, fetch y offline

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: vendor escribe .cargo-checksum.json compatible y offline falla si falta cache en lugar de salir a red. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 20: 20. Metadata y tree

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la salida conserva ids, paquetes, targets, features y nodos de resolucion suficientes para tooling Cargo-compatible declarado. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 21: 21. External compatibility

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la matriz marca diferencias aceptadas y evita declarar clon social completo de crates.io. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 22: 22. Build scripts y proc macros

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor captura evidencia de cargo check real, hashes stdout/stderr y exit status; no inventa ejecucion si Cargo no corre. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 23: 23. Release-check, evidence, seal y conformance

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: release-check preflighta sin archivar por defecto; package real exige --package para evitar artefactos gigantes accidentales. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 24: 24. Integracion con ncc, machine y basal

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor observa machine y basal como dependencias cerradas, pero no cambia su base ni usa imports para saltar modos. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 25: 25. Diferencias con Cargo

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: ghor busca paridad operativa declarada, no gobernanza social completa de crates.io ni CDN global publico. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 26: 26. Recetas de migracion

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: la migracion conserva Cargo.toml cuando corresponde y usa ghor.toml para superficies nC nativas. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 27: 27. Troubleshooting

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: cada error debe tener codigo, causa probable, accion recomendada y forma de reproducirlo. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 28: 28. Operacion industrial

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: CI debe conservar logs, manifests, checksums y outputs JSON para que el cierre sea revisable. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 29: 29. Politica de compatibilidad

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: nuevas capacidades no deben romper lockfiles, manifests ni comandos declarados sin diagnostico y migracion. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


### Checklist ampliado 30: 30. Guia rapida por tareas

1. Ejecuta el comando con `--json` cuando sea parte de CI o release.
2. Guarda stdout, stderr hash, exit code y version del binario activo.
3. Comprueba que no haya tokens en logs ni reportes.
4. Si el comando toca registry remoto, exige auth, audit y checksum.
5. Si el comando toca lockfile, revisa diff antes de commitear.
6. Si el comando toca workspace, revisa members, exclude y default-members.
7. Si el comando compara con Cargo, documenta diferencias aceptadas.
8. Si el comando genera artefactos, guarda manifest y SHA-256.
9. Si el comando usa cache, verifica invalidacion y modo offline.
10. Si el comando falla, conserva el diagnostic baseico y no lo conviertas en
    exito manual.

Notas de operacion: esta guia condensa el manual para uso diario sin reemplazar las secciones profundas. Esta checklist se repite por
familia para que el manual sirva como libro de trabajo, no solo como texto de
referencia. En equipos grandes la repeticion ayuda a que cada persona pueda leer
un capitulo aislado y aun asi aplicar las mismas reglas de seguridad,
reproducibilidad y compatibilidad.


## Apendice B. Diferencias resumidas con Cargo

`ghor` se parece a Cargo en nombres de comandos, lockfile, metadata, tree,
package, publish, install, vendor, update, registry index y cache. La diferencia
principal es que `ghor` gobierna proyectos nC y conserva evidencia de machine,
basal, compiler, runtime y primitive cuando esas superficies participan.

`ghor` no declara ser el ecosistema social completo de crates.io. No implementa
moderacion publica global, CDN mundial operado como servicio social ni policy de
governance comunitaria externa. Lo que si implementa es la superficie tecnica
necesaria para registry protocol, auth, audit, checksum, package format,
publish, yank, owner, moderation local/productiva y evidencia reproducible.

Si una herramienta externa espera `cargo metadata`, `cargo tree`, `.crate`,
Cargo.lock-style o vendor con `.cargo-checksum.json`, `ghor` ofrece vistas
compatibles dentro del alcance declarado. Si una herramienta externa exige
comportamiento social especifico de crates.io, ese punto queda fuera de alcance
y debe integrarse como servicio externo.


## Apendice C.86. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.87. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.88. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.89. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.90. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.91. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.92. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.93. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.94. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.95. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.96. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.97. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.98. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.99. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.100. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.


## Apendice C.101. Notas adicionales de operacion reproducible

Un cierre industrial de `ghor` exige que cada comando sea repetible, observable
y explicable. Repetible significa que el mismo manifiesto, lockfile, registry y
cache producen el mismo resultado o el mismo diagnostic. Observable significa
que el usuario puede ver rutas, hashes, status, paquetes y decisiones del
resolver. Explicable significa que cada fallo tiene accion concreta: instalar
toolchain, autenticar registry, regenerar lockfile, corregir checksum, actualizar
manifest o marcar una diferencia aceptada frente a Cargo.

La compatibilidad con Cargo no debe convertirse en imitacion ciega. Cuando Cargo
es autoridad de formato, `ghor` respeta el formato. Cuando nC necesita metadata
extra, `ghor` la coloca en campos o sidecars que no rompen la vista compatible.
Cuando un registry esta en modo local, `ghor` favorece velocidad de desarrollo.
Cuando un registry es remoto o productivo, `ghor` exige token, audit, checksum y
policy. Esa separacion es la clave para que el mismo comando sirva en clase,
desarrollo, CI y release.
