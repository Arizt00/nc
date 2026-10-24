# Kernel lowering oficial desde `.n`

Fecha: 2026-07-18

## Regla canónica

El kernel se escribe como `.n` normal. No requiere `[@kernel]` ni imports `kernel.*`.
La activación de build se expresa por perfil:

```bash
ncc build kernel.n --target x86_64-unknown-none --profile kernel
```

`nbuild` comparte la misma superficie:

```bash
nbuild kernel.n --target riscv32imac-unknown-elf --profile kernel
```

## Superficie compacta oficial

Un kernel normal no necesita `[@kernel]`, imports, `fn:`, `public:` ni `fa:` para
declarar una unidad ejecutable. La forma compacta conserva el mismo AST canonico:

```n
kernelBridge {
    verify :: in() -> bool {
        cell >> addr <- 0x70000;
        atomic32_store(cell, 1);
        return atomic32_load(cell) == 1;
    }

    ref main ->> verify;
}
```

`main: verify;` equivale a `ref main ->> verify;`. Las funciones directas
equivalen a las funciones dentro de `fn:` y la sintaxis historica sigue siendo
compatible. `nfmt --compact` realiza esta conversion sin cambiar comentarios ni
proyecciones tipadas.

Las primitivas raw inequivocas de atomic, MMIO, barreras y memoria se resuelven sin
imports ni estado kernel ficticio. La ejecucion nativa normal no inserta validacion
preventiva de limites, generaciones o vida de punteros. `--strict-source` activa ese
analisis de forma explicita. Lexer, parser, tipos, ABI y codificacion siguen
validandose porque son necesarios para producir un objeto materialmente enlazable.

## Artefactos obligatorios

El perfil `kernel` crea la carpeta operativa `.ncc` con:

- `.ncc/lowering/<unidad>_<target>_kernel.nm`
- `.ncc/obj/<unidad>.kernel.obj`
- `.ncc/lowering/<unidad>.kernel.body.ll` cuando el target tiene perfil MCT de
  cuerpo high-level admitido
- `.ncc/obj/<unidad>.kernel.body.o|obj` cuando ese cuerpo se materializa como objeto
  nativo real
- `.ncc/bin/<unidad>.<target>.kernel.*`
- `.ncc/bin/<unidad>.x86_64-unknown-none.boot.img` para smoke BIOS x86_64
- `.ncc/bin/<unidad>.i386-unknown-none-elf.boot.img` para smoke BIOS i386
- `.ncc/bin/<unidad>.aarch64-unknown-none.kernel.elf` para QEMU `virt` AArch64
- `.ncc/bin/<unidad>.riscv64-unknown-elf.kernel.elf` para QEMU `virt` RV64
- `.ncc/bin/<unidad>.riscv32imac-unknown-elf.kernel.elf` para QEMU `virt` RV32
- `.ncc/lowering/<unidad>.kernel.lowering.json`
- `.ncc/run/<unidad>.kernel-build.json`
- `.ncc/run/<unidad>.kernel.map`
- `.ncc/incremental/`
- `.ncc/cache/`
- `.ncc/ide/`

El `.nm` retenido siempre pasa por `machine check`; el `.obj` se emite con bytes reales
de machine y el artefacto final se enlaza por Phase12.

## Entry + loader contract

El perfil kernel materializa un simbolo machine global `kmain`. La entrada semantica
high-level (`ref main ->> ...;`) se conserva en los manifiestos y, para targets
con perfil MCT de cuerpo admitido, baja como modulo LLVM freestanding y objeto nativo
enlazado junto al objeto machine de `kmain`. El handoff real del loader sigue apuntando
a `kmain`; `kmain` llama al `main` high-level materializado cuando existe.

El contrato de loader retenido es `ncc_kernel_boot_info_v1`: stack inicial creado por
el loader, puntero `boot_info` segun ABI de target, memoria recibida como
`ncc_kernel_memory_region_v1`, framebuffer y serial opcionales, y tabla de plataforma
ACPI/UEFI/Multiboot/FDT/E820/host segun target. La especificacion detallada vive en
`docs/spec/kernel/kernel_entry_loader_contract_v1_0.md`.

En `x86_64-unknown-none` se materializa ademas una imagen BIOS multietapa con firma
`55 AA` en el primer sector. El stage0 lee stage1 mas el kernel ELF64 final desde el
disco hacia `0x8000`. Stage1 busca ACPI RSDP en EBDA y `0xe0000..0xfffff`, ejecuta
BIOS `int 15h E820`, prepara page tables/GDT, entra en long mode, valida ELF64
`ET_EXEC/EM_X86_64`, carga segmentos `PT_LOAD` a `p_paddr`, pasa `boot_info` por `rdi`
y llama a `e_entry`. Cuando hay cuerpo high-level materializado, `kmain` llama a
`main`; `main` devuelve `0` si el `ref main` booleano ha terminado correctamente y
`kmain` convierte ese resultado en el contrato boot `rax == 0x4f4b`. El manifest
retiene offset del ELF, direccion de memoria, program headers, segmentos cargados,
sectores, hash, contrato `rax == 0x4f4b`, comando QEMU, ruta de serial log y hash
SHA-256 para smoke real de arranque.

## Lowering completo del cuerpo `.n`

La ruta kernel ya no se limita a un stub machine. En targets con perfil MCT de cuerpo
admitido (`x86_64-unknown-none`, `x86_64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-unknown-none`,
`aarch64-unknown-linux-gnu`, `i386-unknown-none-elf`, `i386-unknown-linux-gnu`,
`riscv64-unknown-elf` y `riscv32imac-unknown-elf`), el build ejecuta:

`.n -> AST/sema -> MCT LLVM body -> clang -ffreestanding -c -> objeto nativo -> Phase12`

El cuerpo real cubierto por esta fase incluye variables `stc/local`, llamadas directas,
`ref main` root, structs y campos, punteros a almacenamiento, arrays, strings y control
flow (`while`, `if/else`, retorno bool/int). El `.nm` retenido declara `extern main`,
emite el wrapper `kmain` y el linker final recibe dos objetos: el objeto machine de la
entrada y el objeto nativo del cuerpo high-level. En x86_64 el wrapper usa `call main`
con el contrato de stack/retorno del target; en AArch64 usa `bl main; ret`; en i386 usa
`call main` con relocacion `R_386_PC32`; y en RISC-V usa `call main` con relocacion
`R_RISCV_CALL`. El objeto high-level se importa como COFF, Mach-O, ELF64 o ELF32 real,
segun el target.

Los manifiestos retienen la evidencia bajo:

- `paths.high_level_body_llvm_ir`
- `paths.high_level_body_object`
- `lowering.high_level_body`
- `truth_boundary.full_high_level_kernel_body_instruction_lowering`
- `kernel_contract.kmain_calls_high_level_main`

Si un target aun no tiene perfil MCT de objeto high-level, como WASM en esta fase, el
manifest usa `status = "not_materialized_for_target"`. Si parser/sema aceptan la fuente
pero MCT no puede emitir `main`, usa `status = "not_materialized_by_mct"` con conteo de
funciones y diagnosticos del modulo. Esa frontera es deliberada: no se presenta como
lowering total cuando no hay bytes de objeto reales.

El stage1 x86_64 escribe un `boot_info` real dentro del payload de arranque. El
manifest se genera leyendo esos bytes y retiene:

- `boot_info.address = 0x8438`
- `boot_info.stack.top = 0x90000`
- mapa `ncc_kernel_memory_region_v1` con entradas de loader, stack, imagen ELF y
  segmentos `PT_LOAD`;
- extension runtime `bios_e820` con buffer para 16 entradas de firmware, actualizacion
  de `boot_info.memory_map_len` y slot dinamico en la tabla de plataforma;
- descriptor `ncc_kernel_serial_v1` para COM1 `0x3f8`;
- tabla `ncc_kernel_platform_table_v1` con entradas binarias de 32 bytes:
  `ncc_boot_v1_own_contract` presente, ACPI RSDP como probe runtime externo, slots
  externos ausentes para UEFI system table y Multiboot2 tags, y BIOS E820 como probe
  runtime real de stage1;
- framebuffer ausente como puntero `0`, sin inventar un modo grafico.

En `i386-unknown-none-elf` se materializa una ruta BIOS equivalente para la entrada:
Phase12 genera un ELF32 `ET_EXEC/EM_386`; stage0 lee stage1 mas el ELF32 final hacia
`0x8000`; stage1 inicializa COM1, busca ACPI RSDP en EBDA y `0xe0000..0xfffff`,
ejecuta BIOS `int 15h E820`, carga GDT, entra en modo protegido i386, valida ELF32,
copia `PT_LOAD` a `p_paddr`, empuja `boot_info` como `stack[0]` cdecl y llama a
`e_entry`. El contrato de retorno es `eax == 0x4f4b` y el smoke serial esperado retiene
`nC kernel i386 ACPI RSDP ok`, `nC kernel i386 E820 ok` y
`nC kernel i386 ELF kmain`. Los slots ACPI RSDP y BIOS E820 son probes runtime reales
de stage1; Multiboot2 queda como adaptador externo tipado ausente hasta que un loader
externo lo entregue.

En `aarch64-unknown-none`, `riscv64-unknown-elf` y
`riscv32imac-unknown-elf`, Phase12 genera directamente ELF `ET_EXEC` con segmentos
RX/R/RW, `e_entry` ejecutable y `ncc_kernel_boot_info_v1` dentro del `PT_LOAD`
escribible. Los wrappers crean stack, conservan FDT/hart, llaman al `main` materializado
y publican el resultado por PL011 o UART 16550. `ncc run` arranca el ELF mediante el
loader de sistema QEMU `virt` y valida el marcador serie de cada arquitectura.

## Targets admitidos

El primer perfil kernel oficial cubre las familias machine admitidas:

- `x86_64-unknown-none`
- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-unknown-none`
- `aarch64-unknown-linux-gnu`
- `riscv64-unknown-elf`
- `riscv32imac-unknown-elf`
- `i386-unknown-linux-gnu`
- `i386-unknown-none-elf`
- `wasm32-unknown-unknown`

Aliases aceptados:

- `x86_64-unknown-none-elf`
- `aarch64-unknown-none-elf`
- `riscv32-unknown-elf`
- `riscv64gc-unknown-none-elf`

## Contrato de verdad

Esta fase cierra la ruta oficial de build:

`.n -> sema -> MCT body object cuando aplica -> .nm -> machine check -> .obj -> kernel image/link artifact`

La carga x86_64 e i386 genera `.kernel.elf`, lo empaqueta en el boot image y stage1 lo
carga como ELF64/ELF32. AArch64, RV64 y RV32 generan ELF ejecutable consumido por el
loader QEMU `virt`; no se presenta ese adaptador como un bootloader de firmware propio.
La frontera restante por target queda explicita en:

- `.ncc/lowering/<unidad>.kernel.lowering.json`
- `.ncc/run/<unidad>.kernel-build.json`
- reporte Phase12 junto al artefacto final

La regla sigue siendo: cuando kernel necesite una instrucción, relocación, formato,
ABI o loader que falte, se cierra primero en `[@machine]` con prueba y luego se consume
desde esta ruta.
