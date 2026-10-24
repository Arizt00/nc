# ncc dentro de nC Toolchain

El estado de los modulos 1 y 2 se consolida en
[el dossier v80](../../../docs/NCC_MODULES_V80_DOSSIER.md) y
[el cierre de alineaciones nativas v81](../../../docs/NCC_MODULES_V81_DOSSIER.md).
El [contrato ABI/layout v81](../../../docs/NCC_MODULES_V81_ABI_LAYOUT.md)
define las alineaciones de 32 bytes a 1 MiB y el limite COFF de 8192.
Las revisiones numeradas
de esta pagina son hitos historicos: sus bloqueos no describen por si solos
el estado del candidato actual. Stage0 se conserva hasta superar todas las
puertas de promocion, no solo por obtener una imagen ejecutable.

Este es el proyecto del compilador escrito en nC. El nombre del producto
completo es **nC Toolchain**; el compilador sigue siendo **ncc**.

El frontend se organiza en `src/lexer/`, `src/parser/` y `src/AST/`, con un
contenedor por archivo y dependencias resueltas por `bridge.json`. Los demas
componentes mantienen sus rutas hasta su migracion por fases.

- [Distribucion de componentes](../../../docs/NC_TOOLCHAIN_LAYOUT.md)
- [Contrato y cierre del lexer](../../../docs/NCC_LEXER_MATERIAL_V2.md)
- [Parser y AST material](../../../docs/NCC_PARSER_AST_MATERIAL_V35.md)
- [Modulos y paquetes del parser](../../../docs/NCC_PARSER_MODULES_V36.md)
- [Constantes, WASM ejecutable y verificacion nC](../../../docs/NCC_MATERIAL_VALUES_WASM_V37.md)
- [Verificador parser nativo y gate AST v41](../../../docs/PYTHON_RETIREMENT_V41.md)
- [Contratos frontend y equivalencia v44](../../../docs/NCC_FRONTEND_CONTRACTS_V44.md)
- [Reconstruccion y pruebas reproducibles v44](../../../docs/NCC_FRONTEND_V44_COMMANDS.md)
- [ABI de 64 bits y memoria raw del emisor nC v47](../../../docs/NCC_NATIVE_POINTER_ABI_V47.md)
- [Reproduccion y limite del auto-build v47](../../../docs/NCC_NATIVE_POINTER_ABI_V47_COMMANDS.md)
- [Heap y proveedor nativo v48](../../../docs/NCC_NATIVE_HEAP_ABI_V48.md)
- [Layout material y auto-build v49](../../../docs/NCC_BINARY_LAYOUT_V49.md)
- [Validacion reproducible v49](../../../docs/NCC_BINARY_LAYOUT_V49_COMMANDS.md)
- [Llamadas entre unidades y strings nativos v50](../../../docs/NCC_NATIVE_UNITS_STRINGS_V50.md)
- [Validacion reproducible v50](../../../docs/NCC_NATIVE_UNITS_STRINGS_V50_COMMANDS.md)
- [Proveedores output y render nativos v51](../../../docs/NCC_NATIVE_OUTPUT_ABI_V51.md)
- [Identidad de funciones y comparaciones v52](../../../docs/NCC_GENERIC_COMPARISON_IDENTITY_V52.md)
- [Longitud de strings y unwind PIC v53](../../../docs/NCC_NATIVE_STRING_LENGTH_V53.md)
- [Validacion reproducible v53](../../../docs/NCC_NATIVE_STRING_LENGTH_V53_COMMANDS.md)
- [JSON nativo, cuerpos y ABI v54](../../../docs/NCC_NATIVE_JSON_ABI_V54.md)
- [Validacion reproducible v54](../../../docs/NCC_NATIVE_JSON_ABI_V54_COMMANDS.md)
- [Procesos y pipes nativos v55](../../../docs/NCC_NATIVE_COMMAND_ABI_V55.md)
- [Validacion reproducible v55](../../../docs/NCC_NATIVE_COMMAND_ABI_V55_COMMANDS.md)
- [Resolucion y frontera v55](../../../docs/NCC_NATIVE_COMMAND_ABI_V55_RESOLUTION.md)
- [Liberacion de strings y llamadas PIC v56](../../../docs/NCC_NATIVE_STRING_RELEASE_V56.md)
- [Validacion reproducible v56](../../../docs/NCC_NATIVE_STRING_RELEASE_V56_COMMANDS.md)
- [Resolucion y siguiente puerta v56](../../../docs/NCC_NATIVE_STRING_RELEASE_V56_RESOLUTION.md)
- [Manifiesto del candidato](bootstrap.json)
- [Bridge del proyecto](bridge.json)
- [Puertas locales del modulo 1 v72](../../../docs/NCC_MODULE1_V72_CLOSURE.md)
- [Dossier del modulo 1 y reconstruccion B/C v73](../../../docs/NCC_MODULE1_V73_DOSSIER.md)
- [Reproduccion de las puertas v73](../../../docs/NCC_MODULES_V73_REPRO.md)
- [Orden de modulos y limites v73](../../../docs/NCC_MODULE_ROADMAP_V73.md)
- [Primera puerta del runtime nativo v73](../../../docs/NCC_NATIVE_RUNTIME_MODULE2_V73.md)

Stage2 es un candidato parcial. Stage0 permanece disponible para compilarlo
y para las capacidades no migradas. La equivalencia de toda la gramatica y
del AST Rust sigue pendiente; no se sustituye el compilador instalado.

La revision 44 preserva los IDs 1..85 y agrega cuatro clases de nodo para
contratos de tipo. `compiler type-contracts archivo.n --json` proyecta esos
nodos a declaraciones completas para compararlas con Stage0. Las clausulas
conservan tokens y orden; esto no implica que todos sus cuerpos tengan ya
consumidores semanticos o lowering propio. La evidencia identifica cada
reconstruccion, sus fuentes, objetos y limites de promocion.

La revision 47 materializa cuerpos x86_64 con punteros y enteros de 64 bits,
cargas/stores raw y argumentos en stack para Win64/SysV64. Sus programas
completos se ejecutan en Windows y Linux/WSL. Esto no extiende automaticamente
esa evidencia a otros targets: su auto-build se detenia en `alloc` de
`ast_arena.open`. Proveedores, agregados y reconstruccion completa permanecen
como puertas pendientes de Stage3.

La revision 49 supera las consultas sizeOf/alignOf/offsetOf de NodeRecord,
con IR v3 y constantes completas de 64 bits. Su auto-build se detenia
con un rechazo controlado en ast_arena.n:251, en la llamada calificada que
devuelve un string; los bloqueos descritos en las revisiones anteriores son
historicos. Las pruebas nativas y el layout no equivalen a reconstruccion
completa del compilador ni habilitan promocion Stage3.

La revision 50 conserva firmas entre unidades en IR v4, emite llamadas
calificadas y strings nativos en Win64/SysV64, y serializa el IR independiente
con offsets. La arena AST real se ejecuta con resultado 103. El auto-build
de esa revision se detenia en ast_report.n:8, en el proveedor runtime print; Stage0
continua retenido y Stage3 no es promovible. El dossier registra 80 programas
nativos y 41 casos historicos por host, no certificacion universal de targets.

La revision 53 supera print, render y strByteLen en los auto-builds limpios
Windows/Linux. Su bloqueo historico era constant_probe.n:46, jsonObjectSet.
Los 92 programas del corpus ejecutan con retorno exacto y sus AST materiales
coinciden con v52; Stage0 serializa 91 y rechaza el caso heredado de funcion
ordinaria emit. Ese fallo permanece en los reportes. Los 85 programas previos
y los 41 casos historicos pasan completos. Stage0 sigue retenido, el candidato
no reemplaza la instalacion y Stage3 no es promovible.

La revision 54 supera jsonObjectSet/jsonNewBool, con cuerpos nC y DLL/SO
enlazados a programas completos en los dos hosts. El proveedor JSON aun
lo compila Stage0; el candidato produce los consumidores nativos. El
auto-build limpio con 67 fuentes se detenia en constant_probe.n:48,
commandPipeRead. El corpus ejecuta 95/95 programas y compara 95/95 AST;
Stage0 serializa 93/95, con rechazos de emit y clone visibles en el reporte.
No hay promocion Stage3 ni sustitucion de la instalacion.

La revision 55 supera commandPipeRead y las veinte funciones command,
con tres canales std, firmas y cuerpos nC, DLL/SO y consumidores nativos
en los dos hosts. El corpus ejecuta 96/96 programas y compara 96/96 AST;
Stage0 serializa 94/96, manteniendo visibles emit y clone. El auto-build
limpio de esa revision, con 68 fuentes, generaba seis objetos y se detenia en
constant_probe.n:59, strRelease. El candidato no sustituye Stage0 ni la
instalacion; Stage3 sigue sin ser promovible. Los proveedores nuevos se
compilan desde nC con Stage0, no mediante una reconstruccion ya autonoma.

La revision 56 materializa strRelease como llamada canonica bool(str), con
cuerpo nC emitido por el propio candidato y enlazado en ncruntime.dll y
libncruntime.so.2. Las llamadas externas x86_64 ELF usan PLT32; datos y
unwind conservan sus relocaciones PC-relative. Pasan 99/99 ejecuciones y
99/99 AST materiales en Windows y Linux/WSL; Stage0 serializa 97/99.
El auto-build de esa revision, desde 68 fuentes sin objetos o caches anteriores,
supera constant_probe.n:59 y se detiene en jsonGetBoolOr, linea 91.
Genera seis objetos antes del rechazo controlado. Stage0 permanece retenido,
Stage3 no es promovible y los binarios instalados no se sustituyen.

La revision 57 materializa jsonGetBoolOr como bool(int,bool), con autoridad
JSON ID 25, cuerpo nC emitido por el candidato y DLL/SO reales. Pasan
102/102 ejecuciones y AST materiales por host; Stage0 serializa 100/102.
Los auto-builds limpios de 68 fuentes superan la linea 91 y se detienen en
argsCount, constant_probe.n:96, con seis objetos y rechazo controlado.
La reconstruccion integral sigue pendiente y Stage3 no es promovible.
El dossier conserva tambien un primer smoke PE Windows no satisfactorio
y cinco reintentos exactos del mismo archivo, sin atribuir una causa probada.
Ver [contrato v57](../../../bootstrap/frontend-v57-contracts.json),
[dossier](../../../docs/NCC_NATIVE_JSON_BOOL_DEFAULT_V57.md),
[comandos](../../../docs/NCC_NATIVE_JSON_BOOL_DEFAULT_V57_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_JSON_BOOL_DEFAULT_V57_RESOLUTION.md).

La revision 58 materializa las siete funciones args con ABI canonica,
capturas independientes, UTF-8 real y DLL/SO con 71 exports. Pasan
105/105 ejecuciones y AST materiales por host; Stage0 serializa 103/105.
Los 24 smokes exactos cubren emision, enlace interno, GCC y Clang; tambien
se comparan bytes de argumentos vacios, comillas, Unicode y capturas largas.
El auto-build limpio de 68 fuentes supera argsCount y argsAt y se detiene
en envHas("WINDIR"), constant_probe.n:107, nodo 683.
Los adaptadores foreign estan escritos en nC pero requieren Stage0 para
emitir sus objetos: la prueba directa del candidato sigue rechazada.
No hay promocion Stage3 ni sustitucion de los binarios instalados.
Ver [contrato v58](../../../bootstrap/frontend-v58-contracts.json),
[dossier](../../../docs/NCC_NATIVE_PROCESS_ARGS_V58.md),
[comandos](../../../docs/NCC_NATIVE_PROCESS_ARGS_V58_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_PROCESS_ARGS_V58_RESOLUTION.md).

La revision 59 materializa las trece funciones env con firmas canonicas,
snapshots independientes, expansion de variables, errores del hilo y
herencia real hacia procesos hijos. Los DLL/SO tienen 84 exports. Pasan
32 smokes exactos y seis negativos en Windows/Linux; el corpus ejecuta
108/108 programas y materializa sus AST por host. Los dos rechazos
historicos de serializacion Stage0 siguen impidiendo un reporte integral
verde. El auto-build limpio supera envHas y se detiene en la concatenacion
de constant_probe.n:109, nodo 698. Los adaptadores foreign continuan
requiriendo objetos compilados desde nC por Stage0/LLVM. No hay promocion
Stage3 ni sustitucion de binarios instalados.
Ver [contrato v59](../../../bootstrap/frontend-v59-contracts.json),
[dossier](../../../docs/NCC_NATIVE_PROCESS_ENV_V59.md),
[comandos](../../../docs/NCC_NATIVE_PROCESS_ENV_V59_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_PROCESS_ENV_V59_RESOLUTION.md).

La revision 60 materializa str + str, str += str y concat/concat3 con
proveedor escrito en nC y objeto emitido por el candidato, sin sembrar
ese nuevo cuerpo con Stage0. Se conservan tipos, efectos IR, ABI y
relocalizaciones. Pasan 48 ejecuciones exactas y seis negativos entre
Windows/Linux; DLL/SO publican 86 exports. El corpus ejecuta 113/113;
112/113 materiales coinciden con v59 y 111/113 serializaciones con Stage0.
La diferencia material nueva es la correccion de cast<str> en GenericApply;
el reporte integral no se declara verde. El auto-build limpio supera la
concatenacion y se detiene en exists, constant_probe.n:110, nodo 701.
Ver [contrato v60](../../../bootstrap/frontend-v60-contracts.json),
[dossier](../../../docs/NCC_NATIVE_STRING_CONCAT_V60.md),
[comandos](../../../docs/NCC_NATIVE_STRING_CONCAT_V60_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_STRING_CONCAT_V60_RESOLUTION.md).

La revision 61 materializa exists y createDir con proveedor escrito en nC,
ABI Result<bool, FsError> de ocho bytes, helpers isOk/isErr y firmas entre
unidades. Pasan 40 ejecuciones exactas y ocho negativos en Windows/Linux;
DLL/SO publican 91 exports. El corpus ejecuta y materializa 116/116 casos
por host; Stage0 serializa 114/116 y el informe integral sigue false.
El auto-build limpio de 69 fuentes supera exists/createDir y se detiene
en jsonGetIntOr, constant_probe.n:114, nodo 754. Los adaptadores OS son
nC, pero sus declaraciones foreign todavia requieren Stage0/LLVM para
emitir objetos. No hay promocion Stage3 ni reemplazo de binarios instalados.
Ver [contrato v61](../../../bootstrap/frontend-v61-contracts.json),
[dossier](../../../docs/NCC_NATIVE_FS_DIRECTORY_V61.md),
[comandos](../../../docs/NCC_NATIVE_FS_DIRECTORY_V61_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_FS_DIRECTORY_V61_RESOLUTION.md).

La revision 62 encadena JSON numerico/pretty/archivo, E/S de bytes y
consultas/ventanas de string: 17 operaciones, dos aliases y lowering de
repeat/until y do/while. Pasan 46 ejecuciones exactas, ocho negativos,
102 llamadas IR y ocho nodos de bucle en Windows/Linux. DLL/SO publican
110 exports desde 22 objetos. El corpus ejecuta y materializa 120/120;
Stage0 serializa 118/120 y el reporte integral sigue false. Cuatro
exports JSON conservan sus cuerpos nC/objetos Stage0 de la base v54;
los wrappers y cuerpos nuevos usan objetos candidatos, y los adaptadores
OS foreign siguen requiriendo la semilla Stage0/LLVM explicita.
El auto-build limpio supera constant_probe y la ventana de content_cache,
pero no completa: ambos hosts rechazan explicitRoot (stc), nodo 430,
content_cache.n:61. Falta almacenamiento global nativo con secciones y
relocalizaciones. No se promueve Stage3 ni se reemplazan binarios instalados.
Ver [contrato v62](../../../bootstrap/frontend-v62-contracts.json),
[dossier](../../../docs/NCC_NATIVE_MULTIGATE_V62.md),
[comandos](../../../docs/NCC_NATIVE_MULTIGATE_V62_COMMANDS.md) y
[resolucion](../../../docs/NCC_NATIVE_MULTIGATE_V62_RESOLUTION.md).

La revision 63 materializa stc/local de contenedor como almacenamiento
escalar nativo: .data, .bss, .rodata, direcciones y relocaciones COFF/ELF.
Incluye inicializadores signed/unsigned y addends ABS64 completos, tambien
negativos y mayores de 4 GiB. Pasan 51 ejecuciones exactas entre Windows,
Linux, GCC/Clang y MSVC. El corpus ejecuta y materializa 125/125 por host;
Stage0 serializa 123/125 y el reporte integral conserva los dos rechazos
historicos. Los providers se mantienen en frontend62 con su procedencia.
El auto-build source-only de 69 fuentes supera explicitRoot, pero ambos
hosts se detienen en sleepMs, content_cache.n:90, nodo 548. No hay imagen
self completa ni promocion Stage3, y los binarios instalados no cambian.
Ver [dossier v63](../../../docs/NCC_NATIVE_GLOBAL_STORAGE_V63.md),
[comandos](../../../docs/NCC_NATIVE_GLOBAL_STORAGE_V63_COMMANDS.md),
[validacion](../../../docs/NCC_NATIVE_GLOBAL_STORAGE_V63_VALIDATION.md) y
[resolucion](../../../docs/NCC_NATIVE_GLOBAL_STORAGE_V63_RESOLUTION.md).

La revision 65 continua desde los proveedores v64 de tiempo/filesystem.
Materializa records enteros y anidados, campos, direcciones, argumentos y
retornos Win64/SysV64, con firmas agregadas entre unidades. IR v5 conserva
size/align/offset. Corrige el PE rechazado al terminar .text exactamente en
una pagina y la identidad contextual de label. Pasan 30 ejecuciones exactas
de programas completos y dos verificadores PE nC, mas seis negativos.
El corpus ejecuta 133/133 por host; material contra v64 es 132/133 y
serializacion Stage0 131/133, por lo que el reporte integral sigue false.
El auto-build source-only de 70 fuentes supera contract() -> CacheContract
y ahora rechaza parseInt en driver.n:125. Un consumidor adicional detecta
>>> variable en sha256.n:32. No hay promocion Stage3, nuevos cuerpos en
otros lenguajes ni reemplazo de binarios instalados.
Ver [contrato v65](../../../bootstrap/frontend-v65-contracts.json),
[dossier](../../../docs/NCC_NATIVE_RECORDS_PE_V65.md),
[validacion](../../../docs/NCC_NATIVE_RECORDS_PE_V65_VALIDATION.md),
[reproduccion](../../../docs/NCC_NATIVE_RECORDS_PE_V65_REPRO.md) y
[siguiente puerta](../../../docs/NCC_NATIVE_RECORDS_PE_V65_NEXT.md).

La revision 66 materializa 19 operaciones de parse/string/pad y el shift
logico >>> variable/constante de 32/64 bits. El proveedor nC publica 136
exports comprobados. Corrige la omision de funciones public no llamadas
en emit-object e incorpora arrays fijos locales para ejecutar sha256.n
sin reformular su cuerpo. Pasan 42 ejecuciones por enlace interno,
GCC/Clang/MSVC, el consumidor SHA/cache en ambos hosts y shadowing.
El corpus ejecuta y materializa 139/139 por host; Stage0 serializa 137/139.
El auto-build limpio supera parseInt y se detiene en la llamada calificada
project.nccProject.configureCache, driver.n:587. Stage0 e instalacion
publica permanecen intactos. La compresion de reconstrucciones antiguas
libera 24,35 GB fisicos sin cambiar los 573 archivos; el borrado de caches
esta bloqueado por la herramienta y no se declara realizado.
Ver [dossier v66](../../../docs/NCC_NATIVE_TEXT_V66.md),
[validacion](../../../docs/NCC_NATIVE_TEXT_V66_VALIDATION.md),
[siguiente puerta](../../../docs/NCC_NATIVE_TEXT_V66_NEXT.md) y
[saneamiento](../../../docs/CLEANUP_V66.md).
