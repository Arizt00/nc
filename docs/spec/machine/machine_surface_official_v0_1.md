# nC v0.1 — [@machine] — superficie oficial

## Estado
Documento operativo de implementación inicial dentro del repo.

## Propósito
Fijar la superficie visible oficial de `[@machine]` como capa ejecutable legible y tipada de bajo nivel.

## Secciones oficiales mínimas de [@machine]
Un contenedor `[@machine]` admite en esta fase únicamente:

- `reg:`
- `flag:`
- `label:`
- `inst:`
- `final:`

## Principios de cierre
1. `[@machine]` no es `[@]`.
2. `[@machine]` no es `primitive[@]`.
3. `[@machine]` es la capa ejecutable legible del núcleo triádico.
4. La ISA listada en este documento es la superficie oficial activa de v0.1; cualquier expansión futura debe versionarse explícitamente.
5. `inst:` y `final:` inválidos fallan en parser (sin fallback `Raw`).

## Instrucciones activas de `inst:` en esta fase

- Datos/comparación: `mov`, `cmp`, `test`, `load`, `store`, `lea`
- Aritmética/bitwise: `add`, `sub`, `mul`, `adc`, `sbb`, `and`, `or`, `xor`, `shl`, `shr`, `sar`, `rol`, `ror`, `xchg`, `not.bin`, `neg.bin`, `bsf`, `bsr`, `bswap`
- Stack/unitarias: `push`, `pop`, `inc`, `dec`
- Flujo: `call`, `jmp`, `jmp.eq`, `jmp.ne`, `jmp.gt`, `jmp.lt`, `jmp.ge`, `jmp.le`, `ret`
- Estado/ejecución: `nop`, `wait`, `halt`

## Frontera `[@machine]` ↔ `[@]` en `call`

`call` en `[@machine]` admite solo dos targets tipados:

- `call -> <label_local>`
- `call -> root.<contrato>`

Contratos root permitidos desde machine (implícitos):

- `root.policy.bind`
- `root.policy.guard`
- `root.memory.resolve`
- `root.binary.attach`

Límites activos:

- rutas cualificadas no-root en `call` se rechazan en parser.
- símbolos físicos root (`root.memory.map`, `root.binary.resolve`) se rechazan en sema para `[@machine]`.
- `jmp` y `jmp.*` siguen siendo targets de label local, no contratos root.
- cuando `call` apunta a label inline, su control-flow debe garantizar `ret` en todos los caminos alcanzables.

## Operando de memoria tipado actual

- Formas soportadas:
  - `mem[<base_reg>]`
  - `mem[<base_reg> + <index_reg>]`
  - `mem[<base_reg> + <index_reg>*<scale>]`, con `scale ∈ {1,2,4,8}`
  - cualquiera de las anteriores con desplazamiento: `+ <disp>` o `- <disp>`
- Uso oficial:
  - `load <dst_reg>, mem[<base_reg>[ + <index_reg>*<scale>][ +/- <disp>]]`
  - `store mem[<base_reg>[ + <index_reg>*<scale>][ +/- <disp>]], <src_reg>`
- `load/store` forman parte del camino tipado parser → AST → sema → IR.
- parser rechaza formas no canónicas de `mem[...]` en `inst:`.
- sema exige que `base` e `index` (si existe) estén declarados en `reg:`.

## Semántica mínima por sección

### `reg:`
Zona de declaración simbólica de registros mínimos del contenedor machine.

### `flag:`
Zona de declaración simbólica de flags mínimos del contenedor machine.

### `label:`
Zona de etiquetas de salto y localización simbólica.

### `inst:`
Zona de instrucciones machine mínimas legibles.
Debe existir exactamente una vez y no puede estar vacía.

### `final:`
Clausura terminal del contenedor machine.
Debe existir exactamente una vez, no puede estar vacía y contiene exactamente una operación terminal.

## Diagnósticos canónicos
Ver [machine_diagnostics_official_v0_1.md](./machine_diagnostics_official_v0_1.md).

## Cierre técnico consolidado
Ver [machine_technical_closure_v0_1.md](./machine_technical_closure_v0_1.md).

## Puente machine-root
Ver [machine_root_bridge_v0_1.md](./machine_root_bridge_v0_1.md).
