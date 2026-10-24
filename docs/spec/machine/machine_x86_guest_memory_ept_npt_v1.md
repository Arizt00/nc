# Machine x86 guest physical memory, EPT and NPT v1

Estado: cerrado para construccion tipada, traduccion, reemplazo, desmontaje,
invalidacion y evidencia CPU-backed. La capa raw de kernel conserva control total
del programador y no introduce allocation ni comprobaciones ocultas.

## Autoridad

`ncc-machine::x86_guest_memory` define el contrato compartido:

- direcciones distintas `GuestPhysicalAddress` y `HostPhysicalAddress`;
- paginas de 4 KiB, 2 MiB y 1 GiB;
- tablas de 512 entradas, exactamente 4096 bytes y alineadas a 4096;
- pool fijo de tablas, parametrizado en compilacion y sin heap;
- `IntelEptPageTables<N>` y `AmdNptPageTables<N>`;
- mapa fisico tipado para RAM, ROM, MMIO, memoria compartida y reservas;
- permisos de lectura, escritura, ejecucion y ejecucion de usuario.

El ancho fisico admitido por esta revision es el maximo arquitectonico x86-64 de
52 bits. Una politica de plataforma puede imponer un ancho menor usando CPUID.

## EPT Intel

Las entradas EPT conservan permisos R/W/X, execute-only de usuario, tipo de
memoria UC/WC/WT/WP/WB, ignore-PAT, hojas grandes y suppress-VE. `EptPointer`
codifica direccion PML4, tipo de memoria, walk length de cuatro niveles y el bit
global de accessed/dirty.

Los bits accessed y dirty de una hoja nueva empiezan a cero. Activar seguimiento
en EPTP permite que el procesador los materialice; el constructor no falsifica una
pagina como ya accedida o modificada.

## NPT AMD

Las entradas NPT conservan present, writable, user, write-through,
cache-disable, global, PAT, hojas grandes y NX. La traduccion acumula permisos de
todos los niveles, no solo los de la hoja final.

## Operaciones

| Operacion | Contrato |
|---|---|
| `map` | Instala una hoja 4K/2M/1G y crea tablas intermedias desde el pool. |
| `map_range` | Elige hojas grandes cuando estan alineadas y revierte todo el rango si falla. |
| `replace` | Sustituye atomicamente una hoja del mismo tamano sin desmontarla primero. |
| `translate` | Devuelve HPA, tamano efectivo y permisos acumulados. |
| `unmap` | Elimina la hoja y recupera tablas intermedias que queden vacias. |

El agotamiento del pool tambien es transaccional: se eliminan las entradas padre
creadas durante el intento y se recuperan todos los slots. Cambiar el tamano de
pagina mediante `replace` se rechaza sin destruir el mapping anterior; para
split/merge el kernel debe ejecutar una secuencia explicita de unmap, invalidacion
y map.

## ABI kernel raw

Estas funciones estan disponibles en `.n` de perfil kernel sin imports:

| Funcion | Firma | Materializacion |
|---|---|---|
| `guest_phys_read64` | `addr -> int` | carga volatile de 64 bits |
| `guest_phys_write64` | `(addr, int) -> bool` | store volatile de 64 bits |
| `guest_phys_zero_page` | `addr -> bool` | zero de 4096 bytes si esta alineada |
| `ept_pointer` | `(addr, int, bool) -> int` | EPTP de cuatro niveles |
| `ept_set_entry` | `(addr, int, int) -> bool` | store raw en una entrada EPT |
| `npt_set_entry` | `(addr, int, int) -> bool` | store raw en una entrada NPT |

Las direcciones raw son virtuales directamente accesibles por el kernel, por
ejemplo mediante identity map o direct map. No hay pinning, ownership, bounds,
aliasing, lifetime, cache policy ni conversion HPA/HVA oculta. Una direccion
incorrecta puede corromper memoria o producir una excepcion, como en C/ASM.

Tras cambiar mappings, el programador ejecuta `invept`, `invvpid` o `invlpga`
segun proveedor, alcance y ASID/VPID. El backend no inserta invalidaciones de forma
implicita.

## Evidencia

- `cargo test -p ncc-machine`: 19 pruebas, incluidas hojas 4K/2M/1G, NX,
  rollback, replace, traduccion y reclamacion.
- `kernel_guest_memory_vmexit_loop.n` baja a LLVM, objeto nativo, objeto machine,
  ELF64 e imagen boot con cero simbolos sin resolver.
- El certificado KVM retenido demuestra RAM escrita, dirty log y accesos a GPA
  no mapeada convertidos en VM-exit de lectura y escritura.

Artefactos: `reports/machine/virtualization/`.
