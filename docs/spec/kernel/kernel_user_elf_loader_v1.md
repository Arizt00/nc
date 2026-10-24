# Kernel user ELF loader v1

Estado normativo: activo.

## Formatos y arquitecturas

`ncc-machine::elf_loader` carga ELF32 y ELF64 little-endian, tanto `ET_EXEC`
como `ET_DYN`, para i386, x86_64, AArch64 y RISC-V 64. Valida cabecera,
program headers, maquina, alineacion, limites de archivo, entrada ejecutable y
geometria de cada tabla dinamica antes de transferir control.

Los `PT_LOAD` se fusionan por pagina. El loader reserva paginas temporalmente
escribibles para copiar segmentos y aplicar relocaciones, y finalmente instala
los permisos combinados de cada pagina.

## Linking dinamico basico

La version v1 interpreta `PT_DYNAMIC`, `DT_NEEDED`, `DT_STRTAB`, `DT_SYMTAB`,
`DT_REL`, `DT_RELA` y `DT_JMPREL`. Resuelve simbolos definidos en la imagen o
proporcionados explicitamente por el kernel.

Relocaciones materiales:

- x86_64: `R_X86_64_64`, `PC32`, `GLOB_DAT`, `JUMP_SLOT`, `RELATIVE`, `32` y
  `32S`;
- i386: `R_386_32`, `PC32`, `GLOB_DAT`, `JMP_SLOT` y `RELATIVE`;
- AArch64: `ABS64`, `GLOB_DAT`, `JUMP_SLOT` y `RELATIVE`;
- RISC-V: `32`, `64`, `JUMP_SLOT` y `RELATIVE`.

Un simbolo externo sin resolver o una relocacion desconocida rechazan la
imagen. No se silencian ni se sustituyen por cero.

Este contrato no se presenta como un `ld.so` completo: versionado de simbolos,
namespaces, `LD_AUDIT`, carga recursiva de bibliotecas y descarga dinamica son
una fase posterior.

## TLS y entrada ABI

`PT_TLS` crea un bloque por proceso, copia `.tdata`, pone `.tbss` a cero y
publica el thread pointer. El contexto machine usa FS/GS base en x86_64,
`TPIDR_EL0` en AArch64 y `sscratch` en el perfil RISC-V declarado.

El stack inicial respeta punteros de 32 o 64 bits, alineacion de 16 bytes y el
orden SysV:

```text
argc, argv[], NULL, envp[], NULL, auxv[], AT_NULL
```

El auxv incluye `AT_PHDR`, `AT_PHENT`, `AT_PHNUM`, `AT_PAGESZ`, `AT_BASE`,
`AT_ENTRY`, `AT_RANDOM` y `AT_EXECFN`.

## CLI y stdin

```text
ncc machine kernel-system load-elf --image app.elf \
  --target x86_64-unknown-none --arg app --env MODE=raw \
  --symbol host_write=0x60000000 --bundle target/evidence/user-runtime

cat app.elf | ncc machine kernel-system load-elf --image - \
  --target x86_64-unknown-none --bundle target/evidence/user-runtime

ncc machine kernel-system verify-user-runtime target/evidence/user-runtime
```

El bundle retiene el ELF exacto, informe, SHA-256 y certificado verificable.

