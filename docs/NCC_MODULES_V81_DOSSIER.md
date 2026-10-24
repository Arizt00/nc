# V81: alineacion nativa superior a 16 bytes

## Resolucion

Cierre del perfil nativo x86-64 en Windows/Win64 y Ubuntu/WSL/SysV.
Se ejecutaron todas las potencias de dos entre 32 bytes y 1 MiB para
almacenamiento global y local. El limite representable de una seccion
COFF es 8192 bytes; PE directo y ELF admiten alineaciones mayores dentro
del limite de este perfil. No se trunca una solicitud no representable.

La implementacion, los constructores y los verificadores nuevos son nC.
No se introdujeron fuentes Rust, C, C++ o Python. Stage0 permanece como
oraculo de AST en el corpus, no como constructor de B o C.

## Cambios

| Autoridad nC | Cambio material |
| --- | --- |
| `native_body.n` | Intervalos de pila alineados, temporales, argumentos, retornos, restauracion de rsp y sondeo por paginas |
| `mct.n` | Alineacion maxima y por simbolo; validacion de rangos, potencias de dos y offsets al leer materiales |
| `mct_object.n` | sh_addralign ELF y caracteristicas COFF calculadas, sin el valor fijo de 16 |
| `mct_link.n` | Alineacion de datos/BSS respecto a la direccion virtual final |
| `mct_image_elf.n` | Alineacion de las secciones finales publicada en ELF |
| `content_cache.n` | Identidad V81 para no restaurar materiales del emisor anterior |

No cambia la sintaxis de `align(N)`. Parser, AST, sema y layout aceptaban
esas solicitudes; el backend ahora consume el contrato material. No se
ha agregado una ISA: se emplean instrucciones existentes del perfil x86-64.

## Reconstruccion

Cuatro reconstrucciones finales: B2 y C2 en cada sistema. Cada una parte
de un directorio nuevo y reconstruye 74 unidades con
`project objects --no-incremental`, sin LTO ni objetos de proyecto retenidos.
Se reconstruyen tambien 36 proveedores propios por generacion en Windows
y 35 en Linux, con sus bibliotecas distribuibles.

| Sistema | B2 | C2 | Identidad del compilador |
| --- | --- | --- | --- |
| Windows | 572485 ms | 540828 ms | SHA-256 identico |
| Linux/WSL | 475769 ms | 474393 ms | SHA-256 identico |

Windows: `4ff2300d30fbe99a4c541481e0fab9df284d77c32d1df082084440b9718854c9`.
Linux: `e3d6fc744f0768d19a3f44ca93eddcb90cc3f89caee536f68f8f5ce486eae1e7`.
Son tiempos de construccion observados, no un benchmark de rendimiento
del codigo frente a C.

## Ejecucion y contratos

- Dieciseis alineaciones: 32, 64, 128, 256, 512, 1024, 2048, 4096,
  8192, 16384, 32768, 65536, 131072, 262144, 524288 y 1048576.
- Datos inicializados, BSS, padding, relocaciones de punteros y direcciones
  observadas en el proceso real.
- Locales, arrays, structs anidados, unions y packed sobrealineados.
- Llamadas directas e indirectas, retornos agregados, presion de registros,
  argumentos escalares anteriores a agregados, nesting y recursion.
- Argumento y retorno agregado de 1 MiB ejecutados con reservas de pila
  explicitas en los dos sistemas.
- Win64 por referencia y SysV con area de argumentos realineada.
- COFF con LLD/MSVC y ELF con GNU ld. MSVC requiere `/ALIGN:8192`
  para el fixture COFF de 8192 bytes; su rechazo sin esa opcion se conserva.
- Reserva de pila explicita de 16 MiB para las pruebas locales grandes
  de Windows. El emisor toca las paginas sin introducir `__chkstk` de CRT.
- APIs reales `RtlLookupFunctionEntry` y `RtlVirtualUnwind` aplicadas a
  metadatos COFF enlazados, con contexto de prueba. No se afirma haber
  provocado una excepcion real en todas las instrucciones del prologo.
- Oraculo independiente de bytes x86-64 escrito en nC y ejecutado mediante
  memoria RW -> RX: argumentos sobrealineados en ambos sentidos.
- 310 checks de layout en diez perfiles por generacion y sistema.
  Es calculo de layout, no ejecucion de diez arquitecturas.
- 147 programas completos y 1472 checks por corrida del corpus; cuatro
  corridas finales. Windows ejecuta 1176 comandos por corrida y Linux 1323,
  incluyendo los comandos chmod del runner.
- 38 contratos de runtime por generacion y sistema, con stdout y exits
  exactos, junto a las regresiones de agregados, firmas e imports V80.

Los materiales validos se restauran en caliente: un AST y dos materiales
en el proyecto sobrealineado. Tres materiales corruptos por host se
rechazan incluso al recalcular su SHA-256 en el manifiesto de cache.
Una copia de objeto con datos alterados es rechazada por el lector golden.

## Evidencia

El manifiesto de `docs/evidence/frontend81-r1` relaciona rutas logicas con
payloads por SHA-256. Los informes nC de equivalencia registran cada byte
comparado y la primera diferencia, sin normalizacion. Hay ocho pares
adicionales de objeto/imagen entre hosts; los ELF producidos en Windows
se ejecutaron en WSL y los PE producidos en Linux se ejecutaron en Windows.
Una imagen alterada en el byte 31 es rechazada y nunca se ejecuta.

La ejecucion final y los reintentos estan identificados por separado.
Consultar `NCC_MODULES_V81_ATTEMPTS.md`; los intentos fallidos no se
reescriben como exitosos.

## Alcance

Este cierre no promueve Stage3 global, no retira Stage0, no sustituye
la toolchain instalada y no certifica ejecucion fisica de otras ISAs.
El arbol Git heredado conserva su indice; la reconstruccion limpia se
refiere a fuentes/objetos/caches de las generaciones, no a un commit limpio.

La reserva de pila y los limites del sistema siguen siendo explicitos.
No se agregan ownership, lifetime tracking ni checks de punteros raw.
Los formatos, ABI y limites completos estan en
`NCC_MODULES_V81_ABI_LAYOUT.md`; los comandos en
`NCC_MODULES_V81_REPRO.md`.
