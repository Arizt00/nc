# Debug kernel manual v1

Estado normativo: activo.

El depurador no asigna memoria, no crea hilos y no instala un scheduler. El
programador entrega cada direccion, buffer, tabla de simbolos y limite.

## Superficie compacta

Los parametros consecutivos del mismo tipo se agrupan como
`source, output: addr`; un punto y coma separa grupos de tipos. Las variables
locales pueden inferirse con `name = expression;`. Esta forma se normaliza al
AST canonico antes de sema y no introduce asignaciones ni comprobaciones.

| Forma compacta | Intrinseco canonico |
|---|---|
| `dbgRegs` | `debug_register_snapshot` |
| `dbgDump` | `debug_memory_dump` |
| `dbgTrace` | `debug_stack_trace` |
| `dbgSym` | `debug_symbol_lookup` |
| `rspSum` | `debug_rsp_checksum` |
| `rspFrame` | `debug_rsp_frame` |
| `brkSet` / `brkDel` | `debug_breakpoint_install/remove` |
| `crashWrite` | `debug_crash_report` |

Las formas largas siguen siendo codigo fuente valido.

- `debug_register_snapshot(source, output, slots)` copia registros ya
  capturados a palabras normalizadas de 64 bits.
- `debug_memory_dump(source, output, bytes)` realiza una copia volatil byte a
  byte sin comprobar rangos.
- `debug_stack_trace(frame, output, maximum)` recorre frame pointers ABI y
  escribe direcciones de retorno de 64 bits.
- `debug_symbol_lookup(table, count, pc, output)` busca en registros
  `{start, size, nameAddress, nameBytes}` de cuatro palabras de 64 bits.
- `debug_rsp_checksum` y `debug_rsp_frame` materializan paquetes GDB RSP.
- `debug_breakpoint_install/remove` parchean y restauran el opcode de trap del
  target. La pagina debe ser escribible por una decision explicita anterior.
- `debug_crash_report(report, code, pc, sp, fault, flags)` escribe el registro
  fijo `NCCRASH1` de 64 bytes.
- `panic_raw(code, report)` escribe el informe basico e invoca el trap nativo.

No hay validacion de null, ownership, lifetime, bounds o permisos. Una
direccion invalida tiene el mismo tipo de comportamiento indefinido que en C.

```n
debug(trap, regs, mem, saved: addr; slots: int) -> bool {
    copied = dbgRegs(trap, regs, slots);
    dumped = dbgDump(trap, mem, 256);
    width = brkSet(trap, saved);
    return copied == slots && dumped == 256 && width > 0;
}
```

## Frame pointers

| Target | previous frame | return address |
|---|---:|---:|
| x86_64 | `fp + 0` | `fp + 8` |
| i386 | `fp + 0` | `fp + 4` |
| AArch64 | `fp + 0` | `fp + 8` |
| ARMv7/Thumb | `fp - 4` | `fp + 0` |
| RISC-V 32 | `fp - 8` | `fp - 4` |
| RISC-V 64 | `fp - 16` | `fp - 8` |

La compilacion que quiera stack traces estables debe conservar frame pointers.

## Monitor serial

`SerialDebugger` implementa framing y checksum RSP, lectura de registros y
memoria, escritura de memoria, breakpoints software, continue, single-step,
detach, kill y negociacion `vCont`. El transporte UART sigue siendo manual: el
kernel recibe bytes, llama al monitor y transmite la respuesta.

## Evidencia

El ejemplo oficial es
`examples/11_machine/kernel_debug_hardening_manual.n`. La cadena normal genero
LLVM, objetos e imagen ELF para x86_64, i386, AArch64 y RISC-V64. El bundle
`machine kernel-system debug-hardening` contiene crash binario, texto serial,
transcripcion RSP, traps por ISA, imagen relocada, reporte y certificado.
