# Alineacion nativa V81

## Contrato

`align(N)` conserva la misma sintaxis y el mismo calculo de layout. `N` debe
ser una potencia de dos positiva. El backend nativo x86-64 deja de imponer
un limite artificial de 16 bytes a los agregados.

Este perfil admite alineaciones de hasta 1 MiB. Los valores agregados
temporales y de pila mantienen su limite material de 1 MiB por valor;
el almacenamiento estatico conserva sus limites propios de tamano.
Una alineacion no representable se rechaza, nunca se trunca ni se redondea
hacia abajo.

| Ruta | Alineacion de almacenamiento |
| --- | --- |
| AMD64 COFF | Potencias de dos hasta 8192 bytes, limite del formato |
| ELF64 ET_REL | Potencias de dos hasta 1 MiB en este backend |
| PE64 directo | Potencias de dos hasta 1 MiB en este backend |
| ELF64 directo | Potencias de dos hasta 1 MiB en este backend |
| Locales, argumentos temporales y retornos x86-64 | Hasta el limite material de 1 MiB |

Un global de 64 KiB puede materializarse como PE64 directo, pero no se puede
representar como una seccion COFF de alineacion 64 KiB. Esa ruta debe fallar
sin escribir un objeto defectuoso. Los limites de pila del sistema siguen
siendo responsabilidad del programa; los sondeos no aumentan su reserva.
La prueba local de 1 MiB reserva explicitamente 16 MiB de pila al enlazar
en Windows; no pretende ejecutarse con la reserva PE directa de 1 MiB.
El argumento y retorno por valor de 1 MiB se ejecutan tambien con esa
reserva; en Linux esa prueba declara un limite de pila de 32 MiB.

Al enlazar COFF de 8192 bytes con MSVC, declarar `/ALIGN:8192`. El linker
rechaza una seccion cuya alineacion supere la alineacion solicitada para la
imagen. LLD acepta ese objeto con sus opciones por defecto. Esto no modifica
la ruta PE directa: esta alinea la direccion virtual de datos y BSS sin
depender de un linker externo.

## Pila y ABI

Cada objeto local sobrealineado tiene su propio intervalo reservado, con
espacio para ajustar su direccion. El emisor alinea esa direccion dentro
del intervalo, sin modificar `rbp` ni solapar slots vecinos.

Win64 sigue pasando por direccion las copias de agregados que lo requieren.
Esas copias conservan la alineacion solicitada. SysV alinea el area de
argumentos a la mayor alineacion de los argumentos en memoria, mantiene una
copia del `rsp` anterior y lo restaura despues de la llamada. Los argumentos
se evaluan antes de ajustar esa area; los registros ABI se cargan despues
de realizar las copias.

Las reservas grandes y el ajuste dinamico de llamadas tocan cada pagina
de pila antes de continuar. `rbp` permanece estable para los metadatos
DWARF y Win64. No se introduce un allocator, heap ni scheduler.

## Objetos e imagenes

MCT conserva la alineacion maxima de almacenamiento y la alineacion de cada
simbolo. El lector valida sus potencias de dos, rangos y desplazamientos.
ELF transmite `sh_addralign`; COFF codifica `IMAGE_SCN_ALIGN_*`.
El enlazador interno calcula las direcciones virtuales de datos y BSS
teniendo en cuenta la base de la imagen, no la direccion del buffer host.
Las relocaciones siguen apuntando al simbolo y a su addend reales.

La identidad de cache V81 es distinta de V80. No deben reutilizarse IR,
MCT u objetos emitidos bajo el contrato anterior.

## Referencias del contrato externo

- [Microsoft: PE/COFF](https://learn.microsoft.com/en-gb/windows/win32/debug/pe-format):
  caracteristicas de alineacion de seccion y alineacion de imagen.
- [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/blob/master/x86-64-ABI/low-level-sys-info.tex):
  alineacion del area de argumentos en memoria.

No se ha necesitado una ISA nueva: LEA, AND, TEST, comparaciones, saltos y
las operaciones de pila ya pertenecen al perfil x86-64. La ampliacion esta
en su seleccion, codificacion y contrato de almacenamiento.
