# Machine kernel debug ISA v1

Estado normativo: activo.

## Codificacion cerrada

| ISA | Forma | Bytes little-endian |
|---|---|---|
| x86/x86_64 | `int3` | `CC` |
| x86/x86_64 | `int1` | `F1` |
| x86/x86_64 | `ud2` | `0F 0B` |
| x86_64 | `mov r64, drN` | `0F 21 /r` |
| x86_64 | `mov drN, r64` | `0F 23 /r` |
| AArch64 | `brk #imm16` | `D4200000 | imm16 << 5` |
| AArch64 | `hlt #imm16` | `D4400000 | imm16 << 5` |
| ARMv7 Thumb | `bkpt #imm8` | `BE00 | imm8` |
| RISC-V | `ebreak` | `00100073` |
| WASM | `unreachable` | opcode `00` del modulo |

Encoder y decoder comparten el catalogo. Las formas privilegiadas y los
registros de depuracion exigen perfil kernel/privileged.

## Entrada directa por shell

PowerShell:

```powershell
Get-Content -Raw examples/machine/kernel_debug_x86_64.asm |
  ncc machine stdin emit-bytes --syntax nasm --profile kernel --out debug.bin
```

CMD:

```bat
type examples\machine\kernel_debug_x86_64.asm | ncc machine stdin emit-bytes --syntax nasm --profile kernel --out debug.bin
```

Bash/WSL:

```bash
cat examples/machine/kernel_debug_x86_64.asm |
  ncc machine stdin emit-bytes --syntax nasm --profile kernel --out debug.bin
```

La ruta no invoca un assembler externo. Tambien acepta `.nm`, dialectos NASM,
MASM, GAS Intel/AT&T, FASM y LLVM-MC. Los modulos WASM binarios entran por
`--syntax wasm` para verificar o desensamblar; una unidad machine wasm32 puede
materializarse con la accion `emit-wasm`.

