# Machine guest boot BIOS and UEFI v1

Estado: cerrado para deteccion, validacion, plan de carga y materializacion de los
formatos invitados declarados; BIOS y UEFI tienen ejecucion KVM retenida.

## Autoridad

`crates/ncc-machine/src/x86_guest_boot.rs` contiene una sola autoridad tipada para
firmware, imagen, protocolo, modo CPU, segmentos y errores. El cargador valida
rangos con aritmetica comprobada antes de escribir memoria invitada.

## Firmware

`GuestFirmwareDescriptor` reconoce:

| Firmware | Contrato |
|---|---|
| Legacy BIOS | imagen alineada al final de los primeros 4 GiB y reset vector `0xffff0` |
| UEFI PI | volumen con firma `_FVH`, longitud validada y reset vector en firmware |

La certificacion usa SeaBIOS y OVMF reales con `qemu-system-x86_64 -accel kvm
-cpu host -smp 2`. TCG se rechaza como evidencia fisica.

## Imagenes invitadas

`GuestLoadPlan::parse` detecta y describe:

- sector BIOS de 512 bytes con firma `55 aa` y entrada `0x7c00`;
- aplicacion PE32+ EFI x86_64;
- ELF32 y ELF64 mediante segmentos `PT_LOAD`;
- Linux x86 boot protocol/bzImage;
- Multiboot2 con header valido dentro de la ventana permitida;
- imagen raw directa cuando el llamador aporta el contrato.

Cada `GuestLoadSegment` conserva offset de archivo, bytes presentes, bytes en
memoria, GPA, permisos R/W/X y final comprobado. `materialize` copia `filesz`, pone
a cero `memsz - filesz` y rechaza solapes o rangos fuera de la RAM invitada.

## Linux y Multiboot2

`LinuxBootParameters::build` genera zero-page, command line terminada en cero y
campos initrd con limites del protocolo. La carga usa `0x90000` para setup y
`0x100000` para protected kernel.

`build_multiboot2_information` emite tags alineados a ocho bytes para command
line, boot loader name, basic memory, memory map y terminador. El magic de entrada
es `0x36d76289`.

## Constructores reproducibles

La capa machine puede construir:

- sector y disco BIOS de 1,44 MiB;
- aplicacion PE32+ EFI x86_64;
- disco FAT12 con ruta canonica `/EFI/BOOT/BOOTX64.EFI`;
- ELF64 invitado con segmentos materiales;
- ELF64 con header Multiboot2.

La cadena FAT12 incluye BPB, FAT duplicada, directorios `EFI`/`BOOT`, entrada
`BOOTX64.EFI` y cluster chain completa.

## Ejecucion retenida

El microinvitado BIOS recorre el reset de SeaBIOS, carga el sector, escribe
`NCCBIOS` y termina por el dispositivo de debug con estado 33. El microinvitado
UEFI recorre OVMF y FAT12, carga `BOOTX64.EFI`, escribe `NCCUEFI` y termina con
estado 65.

Los planes ELF, Linux y Multiboot2 se prueban por parse, layout y
materializacion. Una distribucion invitada de terceros no se incluye en el
repositorio: su kernel, initrd y licencia son entradas externas, no bytes
fabricados por el compilador.

## Frontera

La evidencia ejecutada demuestra firmware real sobre KVM y los cargadores
generados. No declara boot completo de una distribucion Linux que no haya sido
aportada ni ejecucion bare-metal. Esa distincion queda codificada en el
certificado y no se sustituye por una simulacion TCG.
