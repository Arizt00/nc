# Cierre de Retirada de Herramientas V43

## Resolucion

Se retiraron los 46 scripts restantes del inventario de herramientas de
produccion, despues de comprobar sus reemplazos nC en Windows x86_64 y
Ubuntu WSL x86_64. El numero real superaba los 34 anunciados: el inventario
integrado incluia 45 entradas y un router JetBrains versionado pero ignorado
por la regla local `bin/`. Ninguno se descarto para reducir el alcance.

El registro acumulado `bootstrap/python-retirement.json` contiene 51
migraciones: cinco anteriores y 46 de esta tanda. Hay cero pendientes dentro
de ese inventario. Esto no declara ausentes otros lenguajes de todo el
repositorio: permanecen Rust Stage0, historia archivada, dependencias ABI
externas y el launcher de programas Python proporcionados por el usuario.

## Implementacion Material

| Familia | Originales retirados | Ejecutable principal |
| --- | ---: | --- |
| Verificadores y routers | 20 | `ncc-tools-v43` |
| Distribucion y actualizaciones | 7 | `ncc-release-v43` |
| Integracion IDE | 6 | `ncc-ide-v43` |
| Generadores de cierre | 4 | `ncc-closures-v43` |
| Documentos | 2 | `ncc-documents-v43` |
| Certificacion | 5 | `ncc-certification-v43` |
| Cliente y protocolo movil | 2 | `ncc-mobile-v43`, `ncc-mobile-protocol-v43` |

La implementacion comprende 147 fuentes canonicas distintas y 191 unidades
compiladas por host. Los diez proyectos producen 211 objetos y diez
ejecutables por host: 422 objetos y veinte ejecutables en conjunto. Veinte
objetos por host son soporte de proyecto/plataforma emitido por Stage0.
El adaptador de enlace `gcc` y `ncc-mobile-build-v43` completan los diez
proyectos; no representan otros originales retirados.

El constructor es `tools/scripts/build_native_tools43.n`; el manifiesto
`bootstrap/native-tools-v43.json` selecciona explicitamente las fuentes y
adaptadores. `bootstrap/native-tools43-artifacts.json` contrasta los hashes
de fuentes, snapshots, objetos y ejecutables. Las herramientas no delegan
su implementacion a Python ni generan una fuente C de sustitucion.
El enlace sigue usando las herramientas y bibliotecas ABI declaradas.

Los binarios cualificados estan en `target/tmp/native43-windows/bin` y
`target/tmp/native43-linux/bin`. No se sustituyo el compilador instalado ni
se promovio Stage3. [Comandos de construccion](NATIVE_TOOLS_V43_COMMANDS.md)
documenta salidas absolutas y separadas por host.

## Pruebas de Retirada

Se aprobaron 92 receipts, una por original y host. Cada una se verifico con
ambos validadores nativos: 184 invocaciones reales, no 184 filas propuestas
por un manifiesto. La revision previa comprobo los candidatos exactos,
los informes referenciados y los bytes originales recuperables.

Las puertas conjuntas pasaron antes y despues de retirar los originales.
Los fixtures de falso aprobado fueron rechazados. La retirada uso una
lista de 46 rutas literales dentro del workspace, sin borrado recursivo.
Despues, el despachador paso 29/29 casos en cada host y
`cargo check --workspace` termino con cero en Windows y Linux/WSL.
Ese check no se presenta como una suite global de tests verde.

El ultimo cambio del pipeline de certificacion exigio reconstruirlo y
repetir sus controles, sin reutilizar receipts de un ejecutable anterior:

| Contrato | Controles por host |
| --- | ---: |
| Exact-head | 875, con 98 casos CLI |
| Portable hypervisor/IOMMU | 555 |
| Primitive | 24 |
| Pipeline final/raw | 71 |

Los 71 controles finales incluyen tres comparaciones completas de informes,
32 aserciones CLI, dieciseis comprobaciones argv/salida y ocho del comando
Cargo por defecto. Un fixture nC verifica la resolucion por PATH y propaga
salidas cero y 23. No simula una certificacion de Cargo real. La matriz
multitarget conserva los rechazos y fallos esperados del compilador original;
equivalencia de esos resultados no significa que todos los targets ejecuten.

Los binarios actuales de certificacion tienen estos SHA-256:

- Windows: `6b2d806b38844d1fabe216803f0a7f15bdea02bab695c0e29fc3bd88ca492908`.
- Linux/WSL: `bdf9bf4b6c6b2750a63fd986d815d6c386ad0f394964d14d33a341f884195b44`.

El paquete fuente publico cualificado se extrajo en directorios nuevos.
Sus 191 unidades se compilaron en cada host y las dos puertas nativas de
layout pasaron. Los hashes de fuentes se compararon con los snapshots que
originaron los 422 objetos y veinte ejecutables. El indice final distingue
esa distribucion ejecutada de cualquier paquete posterior que solo actualice
documentacion, fixtures y registros.

## Evidencias

Los siguientes archivos estan en `docs/evidence/native-tools43`:

- `pre-retirement-independent-review-late-final.json`: originales, ZIP,
  receipts, candidatos y 184 llamadas de validacion.
- `receipts-validator-windows-late-final.json` y su pareja Linux:
  92 invocaciones nativas por validador.
- `batch-retained-*-late-final.json` y `batch-retired-*-late-final.json`:
  validacion material antes y despues de la retirada.
- `retirement-transaction-late-final.json`: las 46 rutas retiradas.
- `dispatch-*-after-retirement.json`: 29 casos por host sin originales.
- `post-retirement-stage0-check.json`: checks reales del workspace.
- `public-source-closure-late-final.json`: archivo fuente extraido,
  unidades, objetos, ejecutables y layout de ambos hosts.
- `final-retirement-closure.json`: indice final con hashes verificables,
  alcance y limites explicitos.

Las pruebas historicas permanecen ligadas a su generacion original. Un
informe rojo no se edito para convertirlo en verde. Los reintentos correctos
conservaron los fallos previos de argumentos, entorno y construccion.

## Recuperacion

`docs/evidence/native-tools43/originals-v43.zip` conserva las 46 entradas,
incluidas tres que no estaban registradas en Git. Su SHA-256 es
`f8a800ffc088af5c8f09bfbc6f334f402da4f6e8101a13d3681e2e43936ba059`.
`originals-v43.json` registra tamanos, hashes y rutas. Cada entrada se
descomprimio y comparo con el original antes de autorizar su retirada.
El ZIP es recuperacion historica, nunca un fallback de produccion.

## Limites y Siguiente Fase

No se afirma ejecucion fisica, CI remoto exact-head, macOS, otros hosts,
equivalencia para toda entrada posible, ni retiro de Rust Stage0. Los limites
de recursos y los casos de Unicode estan en los dossiers de cada familia;
las pruebas con rutas largas de Windows no certifican el cierre de ese
limite del backend heredado. Tampoco se ha saneado ni stageado el trabajo
previo del arbol para aparentar un commit limpio.

La siguiente fase vuelve al lexer, parser y AST de ncc: completar sus
contratos nC, comparar corpus y artefactos entre generaciones, y avanzar
Stage2/Stage3 solo cuando cada puerta demuestre equivalencia. Esta tanda
elimina los scripts de produccion enumerados; no elimina antes de tiempo
el compilador que todavia construye y enlaza sus reemplazos.
