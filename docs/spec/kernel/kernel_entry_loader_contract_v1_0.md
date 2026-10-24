# Kernel entry and loader contract

Fecha: 2026-07-21

## Entrada real

Todo build `--profile kernel` materializa una entrada machine global:

```text
kmain
```

`ref main ->> ...;` sigue indicando la entrada semantica high-level, pero el
simbolo que baja a `.nm`, `.obj` e imagen/link artifact es `kmain`.

## ABI de entrada

El loader entra con stack valido antes de `kmain`:

- x86_64 SysV/freestanding: `boot_info` en `rdi`, stack en `rsp`, alineacion 16.
- Win64: `boot_info` en `rcx`, stack en `rsp`, shadow-space segun Win64.
- AArch64: `boot_info` en `x0`, stack en `sp`, alineacion 16.
- RISC-V 64/32: `boot_info` en `a0`, stack en `sp`, alineacion 16.
- i386: `boot_info` en `stack[0]`, stack en `esp`, alineacion 4.
- wasm32: `boot_info` en `param0`, stack del host wasm.

La entrada `kmain` no recibe argumentos ocultos. El primer dato observable del kernel
es siempre el descriptor `boot_info`; todo lo demas debe venir referenciado desde ese
descriptor. En x86_64 freestanding el stage1 establece `rsp = 0x90000`, reserva la
ventana `0x80000..0x90000` como stack inicial y pasa `rdi = 0x8438`.
En i386 freestanding el stage1 establece `esp = 0x90000`, reserva la misma ventana de
stack inicial y empuja el puntero `boot_info` como primer argumento cdecl antes de
llamar a `e_entry`.
En QEMU `virt`, la entrada AArch64 conserva el FDT inicial de `x0`, establece
`sp = 0x47ff0000` y sustituye `x0` por el `boot_info` materializado. Las entradas
RISC-V conservan hart/FDT de `a0/a1`, establecen `sp = 0x87ff0000` y sustituyen `a0`
por el mismo contrato antes de llamar al cuerpo high-level.

## `ncc_kernel_boot_info_v1`

El puntero `boot_info` apunta a un descriptor estable:

- offset 0: `magic: u64`, bytes ASCII `NCCBOO1\0`
- offset 8: `version: u64`
- offset 16: `flags: u64`
- offset 24: `memory_map_ptr: ptr<ncc_kernel_memory_region_v1>`
- offset 32: `memory_map_len: u64` en entradas
- offset 40: `framebuffer_ptr: ptr<ncc_kernel_framebuffer_v1>|0`
- offset 48: `serial_ptr: ptr<ncc_kernel_serial_v1>|0`
- offset 56: `platform_table_ptr: ptr<ncc_kernel_platform_table_v1>|0`
- offset 64: `command_line_ptr: ptr<u8>|0`
- offset 72: `kernel_payload_ptr: phys_addr|0`
- offset 80: `kernel_payload_size: u64`

Los flags v1 son:

- `0x1`: `memory_map_present`
- `0x2`: `serial_present`
- `0x4`: `platform_table_present`
- `0x8`: `framebuffer_present`
- `0x10`: `cpu_topology_present`

La memoria se describe como array `ncc_kernel_memory_region_v1` con `base`,
`length`, `kind`, `flags`, `source` y `reserved`. Cada entrada ocupa 32 bytes:

- offset 0: `base: u64`
- offset 8: `length: u64`
- offset 16: `kind: u32`
- offset 20: `flags: u32`
- offset 24: `source: u32`
- offset 28: `reserved: u32`

Catalogo inicial de `kind`:

- `0`: reservado
- `1`: loader
- `2`: imagen ELF del kernel cargada como fichero
- `3`: segmento `PT_LOAD` ya materializado
- `4`: stack inicial
- `5`: memoria utilizable reportada por firmware
- `6`: ACPI reclaim reportado por firmware
- `7`: ACPI NVS reportado por firmware
- `8`: memoria defectuosa reportada por firmware
- `9`: servicios runtime de firmware
- `10`: MMIO reservado
- `11`: memoria persistente

Catalogo inicial de `source`:

- `1`: stage1 `ncc-boot-v1`
- `2`: program header ELF64
- `3`: BIOS `int 15h E820`
- `4`: mapa de memoria UEFI
- `5`: tag de mapa de memoria Multiboot2
- `6`: nodo `/memory` de FDT

`ncc_kernel_framebuffer_v1` ocupa 32 bytes y conserva el framebuffer que entrega el
firmware o cargador, sin inferir formatos ocultos:

- offset 0: `base: u64`
- offset 8: `width: u32`
- offset 12: `height: u32`
- offset 16: `pitch: u32`
- offset 20: `format: u32`
- offset 24: `flags: u64`

`ncc_kernel_serial_v1` retiene `magic`, `version`, `kind`, `base`, `irq`, `baud`,
`flags` y `register_shift`. En las rutas BIOS x86_64 e i386 actuales,
`kind = x86_port_io`, `base = 0x3f8` y `baud = 115200`.

`ncc_kernel_platform_table_v1` retiene `magic`, `version`, `entry_count` y entradas de
plataforma. La cabecera ocupa 16 bytes y cada entrada `ncc_kernel_platform_entry_v1`
ocupa 32 bytes:

- offset 0: `kind: u32`
- offset 4: `flags: u32`
- offset 8: `ptr: u64`
- offset 16: `length: u64`
- offset 24: `checksum_or_revision: u64`

Catalogo inicial de `kind`:

- `1`: `ncc_boot_v1_own_contract`
- `2`: `acpi_rsdp`
- `3`: `uefi_system_table`
- `4`: `multiboot2_tags`
- `5`: `bios_e820`
- `6`: `fdt`
- `7`: `opensbi_hart`
- `8`: `wasm_host_imports`
- `9`: `system_loader_process_contract`
- `10`: `cpu_topology`
- `11`: `smbios32_entry`
- `12`: `smbios64_entry`
- `13`: `smbios_tables`
- `14`: `multiboot2_boot_device`
- `15`: `uefi_memory_map`
- `16`: `uefi_image_handle`
- `17`: `multiboot2_load_base`
- `18`: `multiboot2_vbe`
- `19`: `multiboot2_elf_sections`
- `20`: `multiboot2_apm`
- `21`: `multiboot2_network`
- `22`: `uefi_boot_services_active`
- `23`: `multiboot2_framebuffer_raw`
- `24`: `multiboot2_boot_loader_name`

Los flags v1 son:

- `0x1`: entrada presente
- `0x2`: entrada requerida
- `0x4`: contrato propio NCC
- `0x8`: adaptador externo
- `0x10`: puntero cero obligatorio si esta ausente
- `0x20`: probe de runtime hecho por stage1
- `0x40`: longitud dinamica rellenada por stage1

Si no hay UEFI/Multiboot/FDT real, la entrada obligatoria presente es
`ncc_boot_v1_own_contract`; los adaptadores externos conservan su slot tipado con
`ptr = 0`, `length = 0` y `present = false`. En las rutas BIOS x86_64 e i386,
`acpi_rsdp` arranca como `runtime_probe_external`: stage1 busca `RSD PTR ` en EBDA y en
`0xe0000..0xfffff`, valida checksum y rellena el slot si lo encuentra. El slot
`bios_e820` arranca como `runtime_probe_buffer`, con puntero a un buffer real dentro de
stage1, y stage1 lo marca presente si `int 15h E820` devuelve entradas. Un loader
externo rellena esos mismos slots cuando entrega la tabla real.

## Loader y plataforma

El contrato materializa adaptadores por target:

- x86_64 freestanding: BIOS propio, Multiboot2 BIOS/EFI y UEFI PE/COFF con GPT/ESP.
- AArch64 freestanding: UEFI PE/COFF con GPT/ESP y FDT.
- RISC-V 64/32: OpenSBI con hart/FDT y payload plano.
- i386 freestanding: BIOS propio con E820 y ELF32.
- hosted Linux/Windows/macOS: smoke de loader del sistema, no bootloader kernel.
- wasm32: contrato de host wasm.

Framebuffer y serial son opcionales, pero si existen deben llegar por el descriptor
`boot_info`; el kernel no debe descubrirlos por convenciones ocultas.

## Boot image x86_64 stage0 + stage1 + ELF64

Para `x86_64-unknown-none`, `ncc build --profile kernel` retiene tambien:

```text
.ncc/bin/<unidad>.x86_64-unknown-none.boot.img
```

Es una imagen BIOS multietapa con firma `55 AA` en el primer sector. El stage0 lee
stage1 mas el ejecutable kernel ELF64 final desde el sector 2 hacia `0x8000`. Stage1
activa A20, prepara tablas de pagina identity-map, carga GDT, habilita long mode
x86_64, valida el ELF64 `ET_EXEC/EM_X86_64`, recorre program headers, copia cada
segmento `PT_LOAD` desde `p_offset` a `p_paddr`, zero-fill de `p_memsz - p_filesz`,
coloca `boot_info` en `rdi` y llama a `e_entry`. Stage1 valida el contrato de retorno
`rax == 0x4f4b` y emite `nC kernel ELF kmain` por COM1. Antes de entrar en long mode,
stage1 busca ACPI RSDP real, ejecuta BIOS `int 15h E820`, convierte las entradas a
`ncc_kernel_memory_region_v1`, actualiza `boot_info.memory_map_len` y marca los slots
`acpi_rsdp`/`bios_e820` si el firmware responde. El manifest retiene offset del ELF,
direccion de memoria, sectores leidos, hash, program headers, segmentos `PT_LOAD` y
comando QEMU reproducible para comprobar arranque real y salida serial.

El `boot_info` que se pasa al kernel ya no es solo metadata del manifest: esta escrito
en los bytes de stage1. En la ruta x86_64 BIOS contiene:

- direccion real `0x8438`;
- mapa de memoria bootstrap con regiones de stage0, stage1, imagen ELF, segmentos
  `PT_LOAD` y stack inicial;
- buffer runtime para hasta 16 entradas BIOS E820 contiguas al mapa bootstrap;
- descriptor serial `NCCSER1`;
- tabla de plataforma `NCCPLT1` con contrato propio `ncc_boot_v1_own_contract` presente
  y slots tipados ausentes para `uefi_system_table` y `multiboot2_tags`;
- slot `acpi_rsdp` como probe runtime externo, con busqueda EBDA/BIOS y checksum;
- slot `bios_e820` con buffer runtime, longitud dinamica y materializacion por stage1;
- framebuffer ausente con puntero `0`, declarado explicitamente;
- `kernel_payload_ptr = 0x9000` y `kernel_payload_size` parcheado al tamaño real del
  ELF generado.

## Boot image i386 stage0 + stage1 + ELF32

Para `i386-unknown-none-elf`, `ncc build --profile kernel` retiene tambien:

```text
.ncc/bin/<unidad>.i386-unknown-none-elf.boot.img
```

Es una imagen BIOS multietapa con firma `55 AA`. El stage0 lee stage1 mas el ELF32
final desde el sector 2 hacia `0x8000`. Stage1 inicializa COM1, busca ACPI RSDP real,
ejecuta BIOS `int 15h E820`, carga GDT, entra en modo protegido i386, valida el ELF32
`ET_EXEC/EM_386`, recorre program headers, copia cada segmento `PT_LOAD` desde
`p_offset` a `p_paddr`, hace zero-fill de `p_memsz - p_filesz`, empuja `boot_info`
como `stack[0]` y llama a `e_entry`. Stage1 valida `eax == 0x4f4b` y emite
`nC kernel i386 ELF kmain` por COM1. Cuando QEMU/firmware responde, antes emite
`nC kernel i386 ACPI RSDP ok` y `nC kernel i386 E820 ok`.

El `boot_info` i386 usa el mismo layout binario `ncc_kernel_boot_info_v1`, con
descriptor serial `NCCSER1`, tabla `NCCPLT1`, contrato propio
`ncc_boot_v1_own_contract` presente, slot `acpi_rsdp` como probe runtime externo y
slot `bios_e820` con buffer runtime para 16 entradas `ncc_kernel_memory_region_v1`.
El stage1 actualiza `boot_info.memory_map_len` antes de `kmain` cuando E820 devuelve
entradas. `multiboot2_tags` permanece como adaptador externo tipado ausente hasta que
un loader Multiboot real entregue ese puntero.

## Boot ELF AArch64 y RISC-V

Los targets freestanding siguientes producen el ELF final que consume el firmware o
loader; no son contenedores `NCCLINK12` ni blobs de `.text`:

- `aarch64-unknown-none`: ELF64 `ET_EXEC/EM_AARCH64`, aplicacion `BOOTAA64.EFI`,
  GPT/ESP, stack alineado y FDT capturado desde el firmware.
- `riscv64-unknown-elf`: ELF64 `ET_EXEC/EM_RISCV`, base `0x80200000`, stack
  `0x87ff0000`, UART 16550 en `0x10000000` y hart/FDT capturados desde `a0/a1`.
- `riscv32imac-unknown-elf`: ELF32 `ET_EXEC/EM_RISCV`, base `0x80400000`, el mismo
  stack/UART y slots hart/FDT adaptados a ILP32.

Phase12 separa `PT_LOAD` RX, R y RW, ubica `ncc_kernel_boot_info_v1`, serial, tabla de
plataforma y mapa de RAM en el segmento escribible, y comprueba que `e_entry` pertenece
a un segmento ejecutable. La ruta UEFI AArch64 carga sus segmentos mediante Boot
Services y la ruta RISC-V conserva el contrato OpenSBI `a0 = hartid`, `a1 = fdt`.
Los runners externos usados por las pruebas respetan `e_entry` y los segmentos ELF.
El wrapper llama a `main`, interpreta `0` como exito C, escribe el marcador por UART y
aparca la CPU con `wfi`.

Los objetos del cuerpo usan `R_AARCH64_CALL26` o `R_RISCV_CALL`; RV32/RV64 se compilan
con modelo `medany` para que codigo y datos sean direccionables en las bases altas de
QEMU `virt`. El manifest retiene ELF exacto, SHA-256, entry, program headers, comando,
log serie y texto esperado.

## Frontera honesta

Este contrato queda materializado en:

- `.ncc/lowering/<unidad>_<target>_kernel.nm`
- `.ncc/bin/<unidad>.x86_64-unknown-none.boot.img` cuando el target es x86_64 freestanding
- `.ncc/bin/<unidad>.i386-unknown-none-elf.boot.img` cuando el target es i386 freestanding
- `.ncc/bin/<unidad>.<aarch64|riscv64|riscv32>.kernel.elf` para QEMU `virt`
- `.ncc/lowering/<unidad>.kernel.lowering.json`
- `.ncc/run/<unidad>.kernel-build.json`

Esto valida entrada, ABI, stack, `boot_info`, serial y ejecucion de `e_entry` mediante
BIOS x86_64/i386, UEFI x86_64/AArch64, GRUB Multiboot2 BIOS/EFI y OpenSBI RV64/RV32.
Tambien valida ELF y cuerpo high-level materializado en las cinco arquitecturas. La
especificacion operativa completa, artefactos, hashes y evidencia se encuentran en
[`kernel_physical_boot_firmware_multiarch_v1.md`](kernel_physical_boot_firmware_multiarch_v1.md).

La unica frontera externa es ejecutar esos mismos artefactos sobre una placa fisica
identificada y producir el certificado firmado por `ncc machine certify-board`. Sin
esa ejecucion el estado es `validated_not_executed`, nunca `certified`.
