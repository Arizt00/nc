# Intentos y controles V81

## Baseline

V80 C7 rechaza el programa sobrealineado en los dos sistemas en la
materializacion del primer global. Se conserva el error y se comprueba
que no aparece el objeto solicitado.

## Preparacion de fixtures

El primer preflight del fixture unwind uso `local` como identificador,
que es una palabra reservada. Se cambio a `frameValue` en la fuente de
prueba; no se relajo el parser.

Las funciones del hijo leaf quedaron inicialmente bajo la seccion stc
en vez de fn/public. El resolver rechazo sus tres referencias sin permiso.
Se corrigio la fuente del fixture, sin cambiar reglas de visibilidad.

Una consulta del estado de cache uso una ruta de informe incorrecta.
La compilacion fria ya habia terminado correctamente; se conservo su
informe original desde `.ncc/incremental/project-cache.json` antes de
ejecutar la pasada caliente.

## Reconstrucciones preliminares

El intento Windows B1 perdio su proceso con 70 de 74 objetos y sin imagen
ni recibo de cierre. Su causa no se certifica. No se cuenta como build limpio.
Se obtuvo una semilla provisional enlazando esos objetos con cuatro objetos
nC V80 de fuentes verificadas como identicas y un entry nC retenido.
El recibo declara expresamente esa retencion. No es B2 ni C2.

El intento provisional A2 se cancelo cuando esa semilla permitio comenzar
B2 desde un directorio completamente nuevo. Se conservaron sus materiales
y el registro de cancelacion. B2 y C2 finales no reutilizan sus objetos.

Linux B1 termino como semilla preliminar; B2/C2 usan el snapshot final,
con la identidad de cache y validacion MCT V81.

## Linkers

MSVC rechazo el COFF de 8192 bytes con LNK1164 al no declararse
`/ALIGN:8192`. El enlace explicito con esa opcion produjo una imagen
que se ejecuto correctamente. El warning LNK4108 se conserva; no se
suprime ni se presenta como una certificacion de driver.

Los fixtures globales superiores a 8192 bytes se rechazan para COFF,
pero sus IR/MCT y ejecutables PE directos se conservan. Es un limite del
formato de objeto, no un truncamiento aceptado.

## Runtime y corpus

La primera ejecucion Linux B2 de los 38 contratos carecia de los
directorios names, empty y stress. Fallaron tres contratos de filesystem.
Se conservaron ese informe y sus fixtures; el reintento usa
`fixtures-r2` y `processes-r2.json`, con la preparacion completa.
Windows B2/C2 y Linux C2 recibieron esa preparacion antes de su primera
corrida y pasaron sus 38 contratos.

El corpus Windows B2 inicial no tenia ncruntime.dll junto a los programas
generados. Los 55 casos dependientes del proveedor fueron rechazados por
el loader con 0xc0000135. No fue una correccion de codigo generado.
Se conserva la corrida y se ejecutan B2/C2 en directorios nuevos
`windows-r2`, con el proveedor propio declarado. Las cuatro corridas
finales del corpus pasan.

## Controles negativos intencionales

- align(24) y align(2097152) no producen imagen ni objeto valido.
- COFF no trunca los globales alineados por encima de 8192 bytes.
- Un byte de datos cambiado hace fallar el verificador golden.
- Cabecera MCT no potencia de dos, alineacion declarada insuficiente y
  offset global desalineado se rechazan con digests de cache coincidentes.
- El verificador de equivalencia detecta exactamente el byte 31 alterado.
  Esa imagen no se ejecuta.

Los FAIL de esos controles y los intentos retenidos no se confunden con
los informes finales positivos ni se eliminan para obtener una suite verde.
