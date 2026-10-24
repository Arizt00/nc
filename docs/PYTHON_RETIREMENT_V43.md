# Retirada Nativa de Herramientas: V43

## Estado de Integracion

La retirada del inventario de esta tanda esta cerrada en Windows x86_64 y
Ubuntu WSL x86_64. Los 46 originales se retiraron despues de aprobar sus
92 receipts y 184 validaciones nativas cruzadas, verificar la recuperacion
y pasar las puertas conjuntas antes y despues de la retirada. No equivale
a un cierre de Stage3. El inventario contiene 46 scripts:
45 entradas del registro anterior y un router JetBrains versionado que su
regla local `bin/` ocultaba de `rg --files`.

Las implementaciones nuevas son archivos `.n`. Rust permanece como Stage0;
sus cambios de esta tanda corresponden a referencias de distribucion y a
pruebas que deben consumir los nuevos modulos, no a nuevos algoritmos de
sustitucion. Los proveedores binarios del sistema y OpenSSL son dependencias
ABI explicitas, no archivos de implementacion C generados.

El dossier final es `docs/NATIVE_TOOLS_V43_RETIREMENT_CLOSURE.md`.
El estado verificable por original se registra en
`bootstrap/python-retirement-v43.json`. Mientras aparezcan evidencias vacias,
pruebas pendientes o hashes diferentes, la comprobacion nativa debe fallar.

## Checkpoints Historicos

El primer checkpoint cualifica 34 originales y 68 receipts de host.
`docs/evidence/native-tools43/receipt-cross-host-34.json` conserva las 136
comprobaciones reales: cada receipt se verifico con el validador nativo de
Windows y con el de Linux. Todas pasaron; entonces los 46 originales seguian presentes.
En el checkpoint siguiente se incorporan los siete originales de distribucion:
41 cualificados y 82 receipts. `receipt-cross-host-release7.json`, en la misma
carpeta de evidencias, registra sus otras 28 comprobaciones nativas cruzadas.
Los candidatos exactos de distribucion pasaron nueve suites por host con
cobertura explicita de los siete originales. Los cinco de certificacion siguen
pendientes y no reciben permiso de retirada por compartir ejecutable con otra
prueba aprobada. Estos checkpoints preceden a la cualificacion final de los
cinco originales de certificacion y no deben confundirse con el estado actual.

## Validacion Final

Los candidatos actuales de certificacion se reconstruyeron despues de corregir
la resolucion del comando Cargo por defecto. Cada host repitio 875 controles
exact-head, 555 casos portable, 24 casos primitive y 71 controles estrictos del
pipeline final/raw. Estos ultimos incluyen comparaciones completas de informes,
argv, codigos de salida y ocho controles del comando Cargo resuelto mediante
un fixture ejecutable escrito en nC. Ese fixture no certifica Cargo real,
hardware fisico ni exito de las dieciseis arquitecturas: los fallos esperados
de la matriz permanecen como fallos tanto en el original como en el reemplazo.

`pre-retirement-independent-review-late-final.json` contrasta los 46 originales,
las entradas del ZIP, las 92 receipts y sus candidatos exactos.
`receipts-validator-*-late-final.json` conserva las 184 invocaciones nativas.
`retirement-transaction-late-final.json` registra las 46 rutas concretas
retiradas, sin borrado recursivo. Estos informes estan en
`docs/evidence/native-tools43`.

Las puertas `batch-retained-*-late-final.json` y
`batch-retired-*-late-final.json` pasaron en ambos hosts. Los fixtures que
pretenden aprobar sin evidencias fueron rechazados; sus informes negativos
se conservan por separado. Los despachadores pasaron 29/29 por host despues
de la retirada. `post-retirement-stage0-check.json` registra los dos
`cargo check --workspace` reales con salida cero, no una suite global verde.

`bootstrap/python-retirement.json` registra 51 migraciones acumuladas: cinco
anteriores y las 46 de esta tanda, con cero pendientes dentro de este
inventario de herramientas de produccion. No declara libre de Python todo
el repositorio, los archivos historicos ni los programas externos del usuario.

Los informes `dispatch-windows-r2.json` y `dispatch-linux-r2.json`, dentro de
`docs/evidence/native-tools43`, repiten las 29 comprobaciones del despachador
con los candidatos conjuntos. El barrido de los dos proyectos compilados no
encontro fuentes `.c`, `.cpp` ni `.py`. Esto no afirma que todo el repositorio,
su historia o los scripts externos del usuario esten libres de Python.

`docs/evidence/native-tools43/stage0-consumer-final.json` registra 24 pruebas
enfocadas realmente ejecutadas, doce en cada host. Cubren resolucion de rutas,
consumidores de distribucion e instalacion, assets Android nativos y contratos
de rendimiento. No sustituye una certificacion global del compilador, ni retira
Rust Stage0. Los intentos cuyo filtro ejecuto cero pruebas no se contabilizan.

## Familias

| Familia | Originales | Ejecutable |
| --- | ---: | --- |
| Verificadores y routers existentes | 20 | `ncc-tools-v43` |
| Distribucion, instalador y actualizaciones | 7 | `ncc-release-v43` |
| IDE y router adicional JetBrains | 6 | `ncc-ide-v43` |
| Generadores de cierre | 4 | `ncc-closures-v43` |
| Manual y biblioteca PDF | 2 | `ncc-documents-v43` |
| Certificacion y alias de compatibilidad | 5 | `ncc-certification-v43` |
| Cliente movil y pruebas del protocolo | 2 | `ncc-mobile-v43`, `ncc-mobile-protocol-v43` |

Cada familia se construye como proyecto independiente con su propio bridge,
fuentes, objetos y ejecutable. El despachador comun conserva argv, entrada y
salida interactivas y el codigo de salida del proceso. No selecciona un
interprete Python como fallback de implementacion.

## Construccion

`tools/scripts/build_native_tools43.n` es el constructor nativo. Para su
bootstrap se copia como `main.n` de un proyecto aislado y se ejecutan
`ncc project sync` y `ncc project link` con el Stage0 retenido.

Despues, el constructor recibe:

```text
build-native-tools --manifest bootstrap/native-tools-v43.json --root <raiz-absoluta>
  --out <directorio-absoluto-por-host> --ncc <Stage0> --cc <linker>
  [--family <nombre>] [--target <triple>]
```

El manifiesto selecciona explicitamente los adaptadores de Windows o Linux.
Las fuentes no se buscan por un barrido indiscriminado del repositorio. La
accion `.github/actions/build-native-tools` reproduce esta construccion;
tener un workflow actualizado no significa que GitHub Actions haya ejecutado
la certificacion.

La integracion material actual contiene 147 fuentes canonicas distintas y
191 unidades compiladas por host. Cada uno conserva 211 objetos nativos y
diez ejecutables; los veinte objetos adicionales pertenecen al soporte de
proyecto y plataforma del Stage0 retenido. El registro de artefactos contrasta
los 382 snapshots con las fuentes actuales. Estos conteos no sustituyen las
pruebas funcionales. El adaptador `gcc` y el constructor movil son dos
ejecutables auxiliares; no anaden originales al inventario de 46.

El paquete fuente publico se extrajo y sus 191 unidades se compilaron de
nuevo en cada host, con 211 objetos y diez ejecutables por host. El informe
`public-source-closure-late-final.json` verifica fuentes, cabeceras materiales,
hashes y las dos puertas nativas de layout. Stage0 y las bibliotecas ABI
externas permanecen como dependencias declaradas.

Las familias release y certificacion se reconstruyeron en proyectos nuevos
antes de integrarse en el directorio conjunto. Sus proyectos anteriores y
los artefactos ligados a receipts historicas quedaron conservados por separado.
Los informes `release-family-integration-sealed.json` y
`certification-family-integration-sealed.json` registran los hashes exactos
copiados, sin cambiar los resultados historicos de las primeras pruebas.

## Contrato de Retirada

1. Conservar y hashificar el original antes de compararlo.
2. Compilar las fuentes nC a objetos COFF/ELF y a ejecutables PE/ELF.
3. Comparar resultados, errores y artefactos con el original en ambos hosts.
4. Mantener los fallos reales y las comprobaciones sustantivas del contrato.
5. Registrar por separado las mejoras deliberadas y los limites medidos.
6. Actualizar consumidores activos, paquetes y pruebas de Stage0.
7. Repetir las pruebas sin depender del repositorio madre ni del original.
8. Verificar hashes y formatos materiales antes y despues de la retirada.

`tools/tests/native_tools43_batch_check.n` verifica el inventario completo,
unicidad, hashes, fuentes `.n`, evidencias de ambos hosts y cabeceras de
objetos/ejecutables. Rechaza una promocion implicita de Stage3 y exige el
estado indicado: originales presentes o retirados. No borra archivos.

La puerta contrasta cada fuente y destino del manifiesto con la copia realmente
compilada y su objeto por familia/host. Los objetos ELF deben ser ET_REL; los
ejecutables ELF, ET_EXEC o ET_DYN con cabeceras de programa. Los hashes de los
dos Stage0 de referencia y de la copia recuperable se fijan en el verificador,
no se aceptan unicamente como booleanos o hashes propuestos por el ledger.
Las receipts de equivalencia deben demostrar pruebas satisfactorias; un
informe de producto fallido no se convierte en prueba aprobada por estar
presente o tener un hash correcto.

## Regresiones de Integracion

El constructor tiene pruebas nativas para manifiestos ausentes o invalidos,
familias vacias, fuentes ausentes, rutas que salen del proyecto, nombres
desconocidos, compilacion en rutas con espacios y seleccion por plataforma.
Las pruebas positivas ejecutan tambien el programa compilado.

El despachador tiene una matriz nativa de 29 comprobaciones: ayuda de las
familias, orden desconocida y preservacion del manual canonico. La primera
ejecucion encontro que el generador primitive heredado ignoraba `--help` y
comenzaba la generacion. Esa ejecucion fue detenida y se conservo su informe
fallido; la nueva ayuda se resuelve antes de inicializar o escribir archivos.
El generador de documentos recibio la misma regresion de ayuda sin escrituras.

El bloque Bash actual de la accion compartida se extrajo y ejecuto localmente
en dos workspaces aislados, uno Windows y otro Ubuntu WSL. En ambos construyo
el constructor desde `.n`, emitio objetos y enlazo un fixture `.n` que imprimio
`native-builder=43` y termino con codigo cero. La salida del enlace es absoluta.
La variante Windows ARM64 se rechazo con codigo dos antes de compilar, pues
esta tanda no dispone de ese adaptador validado. `action-local-final.json`
registra estos resultados y los hashes; no afirma que GitHub Actions o el paso
remoto de instalacion de dependencias se hayan ejecutado.

Durante esta tanda se corrigieron la liberacion de un JSON inexistente, la
herencia de stdio POSIX y el alias que podia liberar dos veces una fuente
seleccionada por plataforma. Las pruebas de seleccion por plataforma se
incorporaron antes de admitir la compilacion conjunta como evidencia.

Los detalles y limites de cada familia se conservan en sus dossiers V43.
La bateria suplementaria `physical43_material_contract_tests.n` compila 17
unidades y dos objetos runtime por host. Sus 97 comprobaciones contrastan
matrices, diagnosticos de campos ausentes, cabeceras minimas, seleccion de
objetos, artefactos enlazados y manifiestos. La primera ejecucion fallo en las
tres cabeceras COFF de dos bytes; el original las aceptaba y el reemplazo no.
Tras corregir el lector y reconstruir, las 97 pasan en cada host. Los informes
rojos `material-contract-*-r1.json` y los finales se conservan por separado;
`material-contract-builds.json` fija las fuentes, objetos y binarios probados.
Esta bateria es evidencia suplementaria de funciones, no una autorizacion
independiente para borrar originales ni un arranque fisico.
Despues del freeze de los helpers compartidos se reconstruyo y ejecuto de
nuevo en ambos hosts: 97/97 controles. `material-contract-builds-sealed.json`
registra sus 34 bindings de fuentes, 38 objetos y los dos ejecutables actuales;
los informes anteriores permanecen como evidencia de sus propias generaciones.

No se afirma ejecucion fisica, equivalencia ilimitada para entradas arbitrarias,
ausencia universal de fugas ni certificacion de arquitecturas no ejecutadas.
Windows x86_64 y Ubuntu WSL x86_64 son los hosts de validacion de esta tanda.
La accion de construccion rechaza hosts sin adaptador ABI validado; no reutiliza
silenciosamente estructuras glibc en Darwin u otra arquitectura.

Los informes `docs/evidence/native-tools43/builder-*.json` conservan 13 pruebas
del constructor por host; los informes `builder-*-final.json` amplian esa matriz
a 16, verificando seleccion entre varias familias y propagacion del fallo real
del compilador. `dispatch-*.json` conserva 29 del despachador por host.
`stage0-integration.json` registra seis pruebas enfocadas de distribucion,
instalador y proyectos Android, con los hashes de sus ejecutables de prueba.
No representa una ejecucion satisfactoria de toda la suite historica.

## Proteccion del Trabajo Previo

No se revierte, stagea ni limpia el arbol heredado. Los informes historicos no
se reescriben para aparentar que una prueba antigua usaba un reemplazo nuevo.
Los 46 originales se retiraron solo despues de la integracion validada. Las
comparaciones temporales con Python constituyen evidencia anterior a esa
retirada, no una dependencia de los ejecutables distribuidos.

`docs/evidence/native-tools43/originals-v43.zip` conserva los bytes de los 46
originales antes de retirarlos, incluidos los tres no registrados en Git.
Cada entrada fue descomprimida y su SHA-256 comparado con el archivo presente.
`originals-v43.json` registra esas comprobaciones y el hash del archivo ZIP.
Es una copia de recuperacion, no un proveedor ni un fallback ejecutable.
El verificador de retirada exige tambien los hashes de ambos artefactos.
SHA-256 del ZIP: `f8a800ffc088af5c8f09bfbc6f334f402da4f6e8101a13d3681e2e43936ba059`.

Una invocacion inicial de ayuda regenero el DOCX canonico antes de que se
corrigiera ese comportamiento, y otra inicio la generacion de documentos
primitive. No habia un backup exacto confirmado de esos outputs. Se conservaron
los archivos, sin restaurar versiones Git que pudieran eliminar trabajo del
usuario. Las comprobaciones finales de ayuda deben demostrar ausencia de nuevas
escrituras; no se presentan como un rollback de aquella invocacion.
