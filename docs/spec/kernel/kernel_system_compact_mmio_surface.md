# Superficie compacta kernel y system

Fecha: 2026-07-21

## Forma fuente

Los programas kernel y system usan funciones directas, estado local inferido,
bases MMIO mediante `at` y una entrada declarativa. La forma fuente oficial no
lleva `[@kernel]`: parser, herramientas e IDE infieren el dominio desde la
superficie material. El marcador explícito se conserva solo como entrada
compatible para fuentes antiguas.

```n
kernelApicSmpGuestBoot {
    configureXapic :: in() -> bool {
        lapic  at 0xFEE00000;
        ioapic at 0xFEC00000;
        hpet   at 0xFED00000;

        guard load32(lapic + 0x30) >= 0 else return false;
        *32(lapic + 0xF0, 0x1FF);
        route = load32(ioapic + 0x10);
        *32(ioapic + 0x10, route);
        counter = load64(hpet + 0xF0);
        *64(hpet + 0xF0, counter);
        sendIpi(lapic, 1, 0x4042);
        *32(lapic + 0xB0, 0);
        return true;
    }

    ref main ->> configureXapic;
}
```

## Inferencia del dominio

La inferencia se hace antes de analizar el cuerpo y después de enmascarar
comentarios y literales en las integraciones de editor. Una unidad `.n` se
considera kernel cuando contiene al menos una de estas señales:

- una base física `name at address;`;
- una escritura `*8`, `*16`, `*32`, `*64`, `*128`, `*256` o `*512`;
- un contenedor cuyo nombre empieza por `kernel` y que declara `main:`.

La tercera señal permite kernels sin MMIO, por ejemplo un arranque que solo use
atomics, asignador freestanding o control de flujo. Un contexto explícito tiene
precedencia, de modo que una unidad `[@machine]` nunca se reinterpreta como
high-level kernel.

## Normalización tipada

| Fuente | AST/IR canónico |
|---|---|
| `name at address;` | variable `addr` con dirección explícita |
| `load8/16/32/64(address)` | `mmio_read8/16/32/64` |
| `*8/16/32/64(address, value)` | `mmio_write8/16/32/64` |
| `8/16/32/64(address, value)` | `mmio_write8/16/32/64` |
| `load128/256/512(address, destination)` | `mmio_read128/256/512` |
| `*128/256/512(address, source)` | `mmio_write128/256/512` |
| `store128/256/512(address, source)` | `mmio_write128/256/512` |
| `sendIpi(base, cpu, vector)` | `apic_send_ipi` |
| `msrRead/msrWrite` | `x2apic_read/x2apic_write` |
| `tscRead/tscDeadline` | `tsc_read/tsc_deadline_write` |
| `main: function;` | entrada tipada a `function` |

La normalización no inserta heap, libc, scheduler, filesystem ni terminal.
Direcciones, alineación, orden y validez MMIO quedan bajo responsabilidad del
programador y del contrato del target.

## Coloreado semántico

Los plugins asignan una categoría independiente a `at` y a cada familia MMIO.
VS Code y JetBrains aplican la misma inferencia tagless del compilador y
enmascaran comentarios, strings y chars antes de decidir el modo. El color no
se deduce de una aparición textual aislada y no contamina programas hosted.

| Superficie | Token semántico |
|---|---|
| `at` | `kernelAddressKeyword` |
| `*8`, `load8`, `store8` | `kernelMmio8` |
| `*16`, `load16`, `store16` | `kernelMmio16` |
| `*32`, `load32`, `store32` | `kernelMmio32` |
| `*64`, `load64`, `store64` | `kernelMmio64` |
| `*128`, `load128`, `store128` | `kernelMmio128` |
| `*256`, `load256`, `store256` | `kernelMmio256` |
| `*512`, `load512`, `store512` | `kernelMmio512` |

Los anchos 8, 16, 32 y 64 transfieren un valor escalar. Los anchos 128, 256 y
512 son transferencias raw de bloque: `source` y `destination` son direcciones
explícitas de buffers de 16, 32 y 64 bytes. No se crea almacenamiento oculto ni
se exige AVX/SVE; LLVM recibe accesos `volatile i128`, `volatile i256` y
`volatile i512` y legaliza cada operación para la ISA del target.

La transferencia conserva exactamente los bits que ocupan esos bytes y acepta
direcciones sin alinear (`align 1`). No es atómica y no añade barreras de bus:
el programador debe usar `read_barrier` o `write_barrier` cuando el dispositivo
o la arquitectura lo requieran.

## Migración de fuentes activas

Los 14 archivos con nombre kernel de `examples/11_machine` usan ya la forma
compacta. Trece son fuentes high-level tagless con funciones directas y
`main:`; `kernel_boot.n` es una unidad `[@machine]` y conserva correctamente su
gramática de registros, labels, instrucciones y `final:`. Ninguno conserva
`[@kernel]`, `[@system]`, `fn:` o `ref main`.

## Evidencia material

El fixture canónico se valida en parser, sema e IR. La prueba
`compact_kernel_surface_reaches_typed_ir_llvm_machine_and_object_bytes`
comprueba que el cuerpo llega a LLVM, a machine y a una sección `.text` de
objeto con bytes no vacíos.

La matriz activa exige `ncc check` para los 14 archivos. La prueba
`ncc_build_infers_kernel_profile_and_emits_multi_arch_artifacts_from_compact_n`
invoca `ncc build` sin `--profile kernel` y comprueba `.nm`, objeto y artefacto
enlazado para la matriz declarada. El formateador conserva `at`, operaciones
MMIO y `main:` en
`nfmt_compact_preserves_kernel_at_mmio_and_direct_main`.
