# Hardening kernel opt-in v1

Estado normativo: activo.

La politica implicita unica es `HardeningPolicy::RAW`: ASLR, KASLR, W^X, NX,
canarios y sanitizers estan desactivados. El compilador no puede activarlos por
perfil, heuristica o import oculto.

## Capacidades explicitas

La escritura compacta `kaslr`, `canarySet`, `canaryOk`, `sanPoison`,
`sanClear` y `sanOk` se normaliza respectivamente a los intrinsecos largos
enumerados debajo. Ambas superficies producen las mismas llamadas LLVM y el
mismo codigo machine; ninguna habilita una politica por si sola.

- ASLR/KASLR seleccionan un slot alineado usando entropia entregada por el
  llamador y aplican relocaciones absolutas de 32/64 bits y relativas.
- W^X y NX validan transiciones y codifican entradas de pagina x86 PAE/64,
  AArch64, ARMv7 LPAE, RISC-V32 y RISC-V64.
- `--permit-rwx` permite RWX de forma deliberada incluso con W^X activo.
- `stack_canary_write/check` trabajan sobre un slot entregado por el programa.
- `sanitizer_poison/unpoison/check` trabajan sobre una sombra entregada por el
  programa y no reservan memoria.
- `kaslr_relocate(value, slide)` aplica una suma raw visible en LLVM.

`pageProtect` y los gestores de tablas existentes realizan el cambio real de
permisos. Ninguna llamada de depuracion cambia RW/RX de forma escondida.

## CLI

```text
ncc machine kernel-system debug-hardening --target x86_64-unknown-none --bundle raw
ncc machine kernel-system debug-hardening --target x86_64-unknown-none --enable all --bundle hardened
ncc machine kernel-system debug-hardening --enable wx,nx --permit-rwx --bundle explicit-rwx
ncc machine kernel-system verify-debug-hardening hardened
```

El primer comando conserva libertad raw. Los otros habilitan exactamente las
opciones nombradas. El certificado enumera la politica y vuelve a calcular
todos los SHA-256 antes de aceptar el bundle.
