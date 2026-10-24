# Autoridad kernel manual y machine viva v1

## Fuente coordinadora

El archivo coordinador de kernel y system es siempre `.n`. No requiere
`[@kernel]` ni imports hosted:

```text
ncc build kernel.n --profile kernel --target x86_64-unknown-none
```

La sintaxis compacta es superficie fuente, no automatizacion oculta. `at`,
`load*`, `*8/*16/*32/*64/*128/*256/*512`, atomicos, puertos, registros de
control, barreras, interrupciones y operaciones de virtualizacion son actos
explicitos del programador.

## Modo normal

El modo kernel normal es `manual_raw`:

- memoria raw, aliasing y direcciones fuera de rango quedan bajo autoridad del
  programador;
- el compilador no inserta bounds checks, overflow checks, heap, limpieza,
  scheduler ni cambios de interrupciones;
- el programador controla lifetime, clobbers, ABI, secciones y relocaciones;
- `core` permanece freestanding, `alloc` solo aparece si se solicita y `stoud`
  hosted esta excluido.

Esto permite comportamiento indefinido y fallos de maquina deliberados. El
compilador ofrece las operaciones; no convierte una direccion invalida en una
direccion segura.

## Machine como dependencia viva

`ncc-machine` selecciona el perfil material del target y publica:

- registros disponibles;
- instrucciones codificables;
- ABI y ancho de puntero;
- formatos de objeto y ejecutable;
- relocaciones finales;
- capacidades requeridas por cada subsistema.

El manifiesto `machine_plan.manual_authority` retiene este contrato. La lista
`compiler_inserts` debe estar vacia. El lowering repite los campos
`manual_raw_default` y `programmer_controls_registers_abi_and_relocations` para
que una herramienta no pueda interpretar el build de otra forma.

Una forma ajena al perfil elegido se diagnostica: aceptar `vmrun` en una ISA
sin esa codificacion no daria libertad material, sino bytes falsos. Cuando un
driver necesita una forma real ausente, se amplia el perfil machine, encoder,
objeto, linker y pruebas en la misma iteracion.

## Ejemplo

`examples/11_machine/kernel_manual_machine_authority.n` demuestra una fuente
compacta sin imports con MMIO, transferencias anchas, atomicos, CR3, TLB,
puertos, barreras e interrupciones. Su prueba recorre parser, sema, LLVM/MCT,
objeto y ELF.

Las instrucciones privilegiadas no se ejecutan dentro de un proceso usuario.
El ELF se ejecuta mediante boot fisico o un backend de hipervisor autorizado;
el selftest y el arnes de resistencia son las variantes ejecutables fuera de
QEMU que no necesitan privilegio de anillo 0.
