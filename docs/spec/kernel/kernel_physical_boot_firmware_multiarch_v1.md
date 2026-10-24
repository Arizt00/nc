# Boot y firmware fisicos multi-arquitectura v1

Fecha: 2026-07-21

## Estado

La implementacion material de boot y firmware queda cerrada para los perfiles
declarados en este documento. Un mismo archivo `.n` compacto produce cuerpo machine,
objeto, ELF y el medio de arranque propio de cada target. El kernel no necesita
`[@kernel]`, imports hosted ni sintaxis distinta de la superficie kernel compacta.

La certificacion en una placa concreta es una evidencia externa y separada. Sin una
placa, numero de serie, probe y captura UART reales, el estado obligatorio es
`validated_not_executed`; nunca `certified`.

## Ejemplo canonico

El ejemplo ejecutable es:

```text
examples/11_machine/kernel_physical_firmware_boot.n
```

Conserva la forma compacta oficial:

```nc
kernelPhysicalFirmwareBoot {
    inspectBoot :: in() -> bool {
        guard bootValid() else return false;
        guard bootMemoryCount() > 0 else return false;
        acpi = bootPlatform(2);
        uefi = bootPlatform(3);
        fdt = bootPlatform(6);
        return bootMemorySize(0) > 0 && (uefi != null || acpi != null || fdt != null);
    }

    ref main ->> inspectBoot;
}
```

El backend expande esa superficie a `ncc_kernel_boot_info_v1`; no hay descubrimiento
oculto dentro del cuerpo high-level.

## Comandos de build

```bash
ncc build examples/11_machine/kernel_physical_firmware_boot.n --profile kernel --target x86_64-unknown-none --out-dir examples/11_machine/.ncc
ncc build examples/11_machine/kernel_physical_firmware_boot.n --profile kernel --target i386-unknown-none-elf --out-dir examples/11_machine/.ncc
ncc build examples/11_machine/kernel_physical_firmware_boot.n --profile kernel --target aarch64-unknown-none --out-dir examples/11_machine/.ncc
ncc build examples/11_machine/kernel_physical_firmware_boot.n --profile kernel --target riscv64-unknown-elf --out-dir examples/11_machine/.ncc
ncc build examples/11_machine/kernel_physical_firmware_boot.n --profile kernel --target riscv32imac-unknown-elf --out-dir examples/11_machine/.ncc
```

Los artefactos de cada target llevan el triple en el nombre. Compilar otro target no
sobrescribe `.nm`, `.obj`, LLVM, map, manifest ni imagen de uno anterior.

## UEFI x86_64 y AArch64

La ruta UEFI genera aplicaciones PE/COFF nativas:

- `BOOTX64.EFI` para x86_64 con ABI Microsoft x64;
- `BOOTAA64.EFI` para AArch64 con AAPCS64;
- relocaciones `.reloc` conservadas;
- imagen FAT de compatibilidad y disco GPT/FAT32 con ESP;
- MBR protector, cabeceras GPT primaria/secundaria y CRC32;
- ruta removible `EFI/BOOT/BOOTX64.EFI` o `EFI/BOOT/BOOTAA64.EFI`.

El loader valida el ELF, reserva las paginas fisicas de cada `PT_LOAD`, copia
`p_filesz`, limpia `p_memsz - p_filesz` y entra por `e_entry`. Antes de abandonar
Boot Services:

1. valida `EFI_SYSTEM_TABLE` y conserva su longitud de cabecera;
2. busca ACPI 1.0/2.0, SMBIOS 2/3 y FDT por GUID;
3. valida checksum/longitud de RSDP, SMBIOS y FDT;
4. captura GOP solo si existe framebuffer lineal inspeccionable;
5. obtiene el memory map completo y traduce tipos/atributos;
6. reintenta `GetMemoryMap` + `ExitBootServices` cuando cambia la map key;
7. llama a `kmain` con el descriptor canonico ya materializado.

La imagen GPT se puede escribir en un medio fisico y arrancar desde el firmware UEFI
sin IDE ni ncc residente. El dispositivo concreto debe certificarse con el protocolo
de placa descrito al final.

## Multiboot2 x86_64

El ELF Multiboot2 contiene cabecera dentro de la ventana de busqueda, checksum,
information-request, entry de modo protegido, console flags, preferencia framebuffer,
module alignment, entry EFI64 y terminador. Conserva ELF64 con segmentos RX/R/RW y
`e_entry` ejecutable.

El normalizador consume y limita todos los tags estandar 0..21:

- command line y boot-loader name terminados en NUL;
- modulos y basic memory;
- boot device, mmap, VBE, framebuffer, ELF sections y APM;
- EFI32/EFI64 system table, SMBIOS y ACPI old/new;
- network, EFI mmap, estado de Boot Services, image handle y load base.

Se validan `total_size`, reserva cero, alineacion de 8 bytes, overflow, limites de cada
tag, RSDP, descriptores EFI y terminador. El raw MBI completo permanece disponible
como plataforma kind 4. La precedencia del mapa canonico es: mmap detallado, EFI mmap,
basic memory. El framebuffer solo se publica cuando es RGB/BGR lineal de 32 bits con
pitch suficiente.

El artefacto pasa `grub-file --is-x86-multiboot2` y fue cargado por GRUB tanto en BIOS
como en UEFI. Las ISOs retenidas en `.ncc/bin` son medios independientes del IDE.

## OpenSBI y FDT RISC-V

RV64 y RV32 generan ELF `ET_EXEC/EM_RISCV` y un payload plano para OpenSBI. El
trampolin de entrada conserva el contrato oficial:

- `a0`: hart id;
- `a1`: direccion FDT alineada;
- siguiente etapa en S-mode;
- RV64 en `0x80200000`, entry real indicada por manifest;
- RV32 en `0x80400000`, ABI ILP32.

El payload sirve como siguiente etapa de `FW_JUMP` pre-cargada o como kernel entregado
por un firmware OpenSBI. El parser FDT valida magic, version, offsets, reserve map,
structure block, strings block y tokens. Extrae:

- `/memory` y reservas;
- `/chosen/bootargs` y `stdout-path`;
- `/cpus/cpu*`, hart ids y frecuencia;
- `simple-framebuffer`;
- controlador de interrupciones y timer cuando estan descritos.

Las direcciones y longitudes se interpretan con `#address-cells` y `#size-cells`; RV32
no trunca silenciosamente los identificadores o punteros de entrada.

## Contratos canonicos

`ncc_kernel_boot_info_v1` es comun a todos los loaders. Expone:

- mapa de memoria tipado;
- framebuffer lineal opcional;
- serial opcional;
- tabla de plataforma;
- command line;
- payload y longitud.

La tabla de plataforma conserva ACPI RSDP, UEFI system table, MBI completo, BIOS E820,
FDT, OpenSBI hart, topologia, SMBIOS 2/3, EFI mmap/image handle/Boot Services y los
tags Multiboot2 extendidos. `bootPlatformSize(kind)` nunca inventa cero para una tabla
UEFI presente: usa el header real cuando es accesible y el minimo contractual de 24
bytes cuando el puntero externo no puede inspeccionarse durante la normalizacion.

## i386 adicional

La ruta i386 comparte el contrato y agrega dos correcciones necesarias para kernels
reales de mayor tamano:

- el wrapper cdecl pasa `boot_info` como `i64` zero-extended mediante dos words;
- stage1 copia segmentos ELF con semantica `memmove`, hacia atras cuando destino y
  fuente se solapan, antes de limpiar BSS.

Esto evita ejecutar bytes del ELF como instrucciones cuando un `PT_LOAD` supera la
ventana entre `0x9000` y `0x10000`.

## Artefactos para uso fisico

| Target | Medio consumible fuera del IDE |
| --- | --- |
| x86_64 BIOS | `*.x86_64-unknown-none.boot.img` |
| x86_64 UEFI | `*.x86_64-unknown-none.uefi-gpt-fat32.img` |
| x86_64 Multiboot2 BIOS/UEFI | `*.multiboot2.elf` o las ISOs retenidas |
| i386 BIOS | `*.i386-unknown-none-elf.boot.img` |
| AArch64 UEFI | `*.aarch64-unknown-none.uefi-gpt-fat32.img` |
| RISC-V 64 | `*.riscv64-unknown-elf.opensbi-payload.bin` |
| RISC-V 32 | `*.riscv32imac-unknown-elf.opensbi-payload.bin` |

La escritura del medio debe usar una herramienta explicita y el dispositivo correcto.
El compilador no selecciona ni sobrescribe discos fisicos implicitamente.

## Certificacion de placa

```bash
ncc machine board-template --target aarch64-unknown-none --out board.json
ncc machine certify-board --manifest board.json --bundle target/board-certification
ncc machine verify-board-certificate target/board-certification
```

El manifest exige identidad de placa y probe, argv estructurado, hash SHA-256,
flash/reset/captura, timeout, marcadores ordenados y rechazo de QEMU/TCG/emuladores.
Puede firmar el certificado con HMAC. `--validate-only` produce
`validated_not_executed`; solo una captura fisica valida puede producir `certified`.

## Autoridades externas

- [UEFI Specification 2.11](https://uefi.org/specs/UEFI/2.11/)
- [GNU Multiboot2 Specification](https://www.gnu.org/software/grub/manual/multiboot2/html_node/Boot-information-format.html)
- [OpenSBI firmware contracts](https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw.md)
- [Devicetree node requirements](https://devicetree-specification.readthedocs.io/en/stable/devicenodes.html)
