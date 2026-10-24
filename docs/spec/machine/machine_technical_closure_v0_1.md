# nC v0.1 — [@machine] — cierre técnico operativo

## Estado
Documento técnico consolidado para el cierre operativo de `[@machine]` en la línea actual del compilador.

Puente `[@machine]` ↔ `[@]` detallado en: [machine_root_bridge_v0_1.md](./machine_root_bridge_v0_1.md).

## Alcance y contrato de pipeline
- `[@machine]` usa representación tipada en parser → AST → sema → IR.
- `inst:` y `final:` bajan a streams IR separados (`MachineInstIR` y `MachineFinalOpIR`).
- `inst:` y `final:` inválidos fallan en parser.
- No existe fallback `Raw` en el camino machine tipado.
- `emit-ir` valida semántica antes de bajar a IR.

## Superficie activa de secciones
| Sección | Estado | Notas |
|---|---|---|
| `reg:` | Activa | Declaración de registros simbólicos |
| `flag:` | Activa | Declaración de flags simbólicos |
| `label:` | Activa | Declaración de labels simbólicas |
| `inst:` | Activa | Instrucciones machine tipadas; sección única y no vacía |
| `final:` | Activa | Solo `ret`, `halt`, `jmp <label>`; sección única con exactamente una operación terminal |

## Tabla oficial de instrucciones soportadas
| Familia | Opcode | Forma canónica | Efecto semántico lateral |
|---|---|---|---|
| Datos | `mov` | `mov <dst>, <src>` | Clobbera estado de comparación |
| Datos/memoria | `load` | `load <dst>, mem[<base>[ + <index>*<scale>][ +/- <disp>]]` | Clobbera estado de comparación |
| Datos/memoria | `store` | `store mem[<base>[ + <index>*<scale>][ +/- <disp>]], <src>` | Clobbera estado de comparación |
| Datos/memoria | `lea` | `lea <dst>, mem[<base>[ + <index>*<scale>][ +/- <disp>]]` | Clobbera estado de comparación |
| Comparación | `cmp` | `cmp <left>, <right>` | Activa estado de comparación |
| Comparación | `test` | `test <left>, <right>` | Activa estado de comparación |
| Aritmética | `add` | `add <dst>, <src>` | Clobbera estado de comparación |
| Aritmética | `sub` | `sub <dst>, <src>` | Clobbera estado de comparación |
| Aritmética | `mul` | `mul <dst>, <src>` | Clobbera estado de comparación |
| Aritmética | `adc` | `adc <dst>, <src>` | Clobbera estado de comparación |
| Aritmética | `sbb` | `sbb <dst>, <src>` | Clobbera estado de comparación |
| Bitwise | `and` | `and <dst>, <src>` | Clobbera estado de comparación |
| Bitwise | `or` | `or <dst>, <src>` | Clobbera estado de comparación |
| Bitwise | `xor` | `xor <dst>, <src>` | Clobbera estado de comparación |
| Shift/rotate | `shl` | `shl <dst>, <src>` | Clobbera estado de comparación |
| Shift/rotate | `shr` | `shr <dst>, <src>` | Clobbera estado de comparación |
| Shift/rotate | `sar` | `sar <dst>, <src>` | Clobbera estado de comparación |
| Shift/rotate | `rol` | `rol <dst>, <src>` | Clobbera estado de comparación |
| Shift/rotate | `ror` | `ror <dst>, <src>` | Clobbera estado de comparación |
| Bitwise | `xchg` | `xchg <dst>, <src>` | Clobbera estado de comparación |
| Bit-scan | `bsf` | `bsf <dst>, <src>` | Clobbera estado de comparación |
| Bit-scan | `bsr` | `bsr <dst>, <src>` | Clobbera estado de comparación |
| Bitwise | `bswap` | `bswap <reg>` | Clobbera estado de comparación |
| Unaria | `not.bin` | `not.bin <reg>` | Clobbera estado de comparación |
| Unaria | `neg.bin` | `neg.bin <reg>` | Clobbera estado de comparación |
| Stack/unaria | `push` | `push <reg>` | Clobbera estado de comparación |
| Stack/unaria | `pop` | `pop <reg>` | Clobbera estado de comparación |
| Stack/unaria | `inc` | `inc <reg>` | Clobbera estado de comparación |
| Stack/unaria | `dec` | `dec <reg>` | Clobbera estado de comparación |
| Flujo | `call` | `call -> <label>` o `call -> root.<contrato>` | Clobbera estado de comparación; en target inline exige retorno garantizado en todos los caminos |
| Flujo | `jmp` | `jmp <label>` o `jmp -> <label>` | Terminador local |
| Flujo | `jmp.eq` | `jmp.eq <label>` o `jmp.eq -> <label>` | Requiere estado de comparación activo |
| Flujo | `jmp.ne` | `jmp.ne <label>` o `jmp.ne -> <label>` | Requiere estado de comparación activo |
| Flujo | `jmp.gt` | `jmp.gt <label>` o `jmp.gt -> <label>` | Requiere estado de comparación activo |
| Flujo | `jmp.lt` | `jmp.lt <label>` o `jmp.lt -> <label>` | Requiere estado de comparación activo |
| Flujo | `jmp.ge` | `jmp.ge <label>` o `jmp.ge -> <label>` | Requiere estado de comparación activo |
| Flujo | `jmp.le` | `jmp.le <label>` o `jmp.le -> <label>` | Requiere estado de comparación activo |
| Flujo | `ret` | `ret` | Terminador local |
| Estado | `nop` | `nop` | No altera estado de comparación |
| Estado | `wait` | `wait` | No altera estado de comparación |
| Estado | `halt` | `halt` | Terminador local |
| Estructura | `label inline` | `<name>:` | Reinicia bloque local y resetea estado de comparación |

## Reglas de flags y estado condicional
| Regla | Estado |
|---|---|
| Productores de estado condicional | `cmp`, `test` |
| Consumidores de estado condicional | `jmp.eq`, `jmp.ne`, `jmp.gt`, `jmp.lt`, `jmp.ge`, `jmp.le` |
| Si no hay estado condicional activo, salto condicional falla | Activo |
| `call` invalida estado condicional | Activo |
| Labels inline reinician estado condicional | Activo |
| `nop` y `wait` preservan estado condicional | Activo |

## Frontera técnica `[@machine]` ↔ `[@]`
La continuidad machine↔root se representa con `MachineCallTarget` tipado:

- `Label(<name>)`
- `RootPath(<segments>)`

Reglas activas de frontera:

- machine puede llamar contratos root implícitos:
  - `root.policy.bind`
  - `root.policy.guard`
  - `root.memory.resolve`
  - `root.binary.attach`
- machine no puede llamar símbolos físicos root:
  - `root.memory.map`
  - `root.binary.resolve`
- rutas cualificadas no-root en `call` fallan en parser.
- `jmp`/`jmp.*`/`final:jmp` siguen estrictamente en dominio de labels locales.

## Tabla de terminadores y reglas de alcanzabilidad
| Elemento | Tipo | Regla |
|---|---|---|
| `ret` | Terminador local | Instrucción posterior en el mismo bloque falla como unreachable |
| `halt` | Terminador local | Instrucción posterior en el mismo bloque falla como unreachable |
| `jmp <label>` | Terminador local | Instrucción posterior en el mismo bloque falla como unreachable |
| `jmp.*` condicional | No terminador local | Permite fallthrough; requiere estado condicional activo |
| `call` | No terminador local | Requiere label válida; si el target es inline, todos los caminos deben cerrar en `ret` |
| `label:` | Reinicio de bloque | Tras terminador, abre nuevo bloque válido |

## Contrato tipado de direccionamiento en `load/store`
- `MachineMemRef` tipado soporta:
  - `MemReg { base }`
  - `MemAddr { base, index?: reg, scale: 1|2|4|8, disp: signed }`
- parser acepta formas canónicas de `mem[...]` y rechaza formas ambiguas/no tipables.
- sema exige declaración previa de `base` e `index` (si existe) en `reg:`.
- IR preserva la estructura (`MachineMemRefIR`) sin degradar a string crudo.

## Tabla canónica de diagnósticos (resumen)
Las tablas detalladas viven en `machine_diagnostics_official_v0_1.md`.
Diagnósticos críticos añadidos para cierre profundo:

| Condición | Diagnóstico |
|---|---|
| Salto condicional sin `cmp`/`test` activo | `conditional jump '...' requires active compare state from 'cmp' or 'test' in current [@machine] inst block` |
| `call` inline sin ruta alcanzable a `ret` | `call target '...' in [@machine] inst block has no reachable 'ret' path` |
| `call` inline con ruta no-retornante | `call target '...' in [@machine] inst block has non-returning control path without guaranteed 'ret'` |
| `call` machine a símbolo root físico | `root physical symbol '...' not allowed in [@machine] context` |
| Base de memoria no declarada | `load memory base '...' not declared in reg section of [@machine] container` / `store memory base '...' not declared in reg section of [@machine] container` |
| Índice de memoria no declarado | `load memory index '...' not declared in reg section of [@machine] container` / `store memory index '...' not declared in reg section of [@machine] container` |
| Registro no declarado en `load/store` | `load destination '...' not declared in reg section of [@machine] container` / `store source '...' not declared in reg section of [@machine] container` |
