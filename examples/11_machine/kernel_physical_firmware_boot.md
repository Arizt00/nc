# Kernel physical firmware boot

Este ejemplo conserva la sintaxis kernel compacta y genera artefactos arrancables
para BIOS, UEFI, Multiboot2 y OpenSBI sin imports hosted ni `[@kernel]`.

## Compilar

```bash
ncc build kernel_physical_firmware_boot.n --profile kernel --target x86_64-unknown-none --out-dir .ncc
ncc build kernel_physical_firmware_boot.n --profile kernel --target i386-unknown-none-elf --out-dir .ncc
ncc build kernel_physical_firmware_boot.n --profile kernel --target aarch64-unknown-none --out-dir .ncc
ncc build kernel_physical_firmware_boot.n --profile kernel --target riscv64-unknown-elf --out-dir .ncc
ncc build kernel_physical_firmware_boot.n --profile kernel --target riscv32imac-unknown-elf --out-dir .ncc
```

## Medios fisicos

- x86_64 UEFI: escribir `*.x86_64-unknown-none.uefi-gpt-fat32.img` en un medio completo.
- AArch64 UEFI: escribir `*.aarch64-unknown-none.uefi-gpt-fat32.img` en un medio completo.
- x86 BIOS: escribir `*.boot.img` como imagen de disco cruda.
- Multiboot2: usar `*.multiboot2-bios.iso` o `*.multiboot2-uefi.iso`.
- RISC-V: cargar `*.opensbi-payload.bin` con el firmware OpenSBI de la placa.

No se debe copiar una imagen GPT como si fuera un archivo dentro de una particion: se
escribe sobre el dispositivo completo elegido deliberadamente por el operador.

## Certificacion de placa

La identidad, serie, probe, flasheo, reset y captura UART se declaran en un manifiesto
de placa. La CLI valida el manifiesto antes de ejecutar y rechaza QEMU, TCG y otros
emuladores como sustituto de hardware:

```bash
ncc machine board-template --target aarch64-unknown-none --out board.json
ncc machine certify-board --manifest board.json --bundle .ncc/run/board-evidence
ncc machine verify-board-certificate .ncc/run/board-evidence
```

Sin una placa conectada el resultado correcto es `validated_not_executed`. El contrato,
los hashes exact-head y la matriz de cargadores externos estan documentados en
`docs/spec/kernel/kernel_physical_boot_firmware_multiarch_v1.md`.
