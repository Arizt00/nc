# Ejemplos SMP fisicos

Los tres ejemplos usan la sintaxis kernel compacta normal. No tienen imports ni
requieren `[@kernel]`:

- `kernel_physical_smp_x86.n`
- `kernel_physical_smp_aarch64.n`
- `kernel_physical_smp_riscv.n`

## Construccion fuera del IDE

```powershell
ncc build kernel_physical_smp_x86.n --profile kernel --target x86_64-unknown-none
ncc build kernel_physical_smp_aarch64.n --profile kernel --target aarch64-unknown-none
ncc build kernel_physical_smp_riscv.n --profile kernel --target riscv64-unknown-elf
```

El primer comando retiene ELF, BIOS image, Multiboot2, BOOTX64.EFI, GPT/ESP y el
trampoline INIT/SIPI. El segundo retiene ELF, BOOTAA64.EFI y GPT/ESP. El tercero
retiene ELF y payload OpenSBI.

`start(cpu, stack, entry, context)` recibe la funcion secundaria real, no una pagina
SIPI. x86 copia automaticamente su trampoline a `0x7000`; AArch64 y RISC-V entregan
entry/context al firmware PSCI/SBI. La funcion secundaria debe llamar
`cpuMarkOnline(cpu)` antes de participar en IPI o shootdowns.

## Prueba automatica sin QEMU

```powershell
cargo test -p ncc-cli --test kernel_physical_smp_artifacts -- --nocapture
```

La prueba construye y verifica los medios fisicos, pero no afirma haber arrancado
una placa. Para ello se usan los manifiestos de `physical_board_smp`.
